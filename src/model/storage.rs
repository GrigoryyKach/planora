//! Reading and writing `data.json`: atomic writes, schema migration, corrupted-file handling, daily backups.

use super::types::{Data, SCHEMA_VERSION};
use chrono::NaiveDate;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const DATA_FILE: &str = "data.json";
pub const BACKUP_DIR: &str = "backups";
/// Automatic daily backups kept.
pub const KEEP_BACKUPS: usize = 14;

/// What happened while loading.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadOutcome {
    /// No data file yet.
    Fresh,
    Loaded,
    /// The file was unreadable or from a newer version: it was renamed and the app starts empty.
    Recovered { moved_to: PathBuf },
}

/// Writes through a temp file in the same folder, then renames over the target.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Parses data text (BOM tolerant) and migrates older schemas. `Err` for broken or too-new data.
pub fn parse(text: &str) -> Result<Data, String> {
    let value: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    migrate(value)
}

/// Upgrades the JSON value to the current schema, then deserialises it.
pub fn migrate(mut value: serde_json::Value) -> Result<Data, String> {
    let obj = value.as_object_mut().ok_or("data is not an object")?;
    let version = obj.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    if version > SCHEMA_VERSION {
        return Err(format!("data schema {version} is newer than supported {SCHEMA_VERSION}"));
    }
    // v0 -> v1: files written before the schema field existed only need it stamped;
    // every other field has a serde default.
    if version < 1 {
        obj.insert("version".into(), SCHEMA_VERSION.into());
    }
    let mut data: Data = serde_json::from_value(value).map_err(|e| e.to_string())?;
    data.version = SCHEMA_VERSION;
    Ok(data)
}

/// Loads `data.json` from `dir`. A broken file is renamed to `data.corrupt-<timestamp>.json`, never overwritten.
pub fn load(dir: &Path, stamp: &str) -> (Data, LoadOutcome) {
    let path = dir.join(DATA_FILE);
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Data::default(), LoadOutcome::Fresh),
        Err(_) => return (Data::default(), quarantine(&path, dir, stamp)),
    };
    match std::str::from_utf8(&bytes).map_err(|e| e.to_string()).and_then(parse) {
        Ok(d) => (d, LoadOutcome::Loaded),
        Err(_) => (Data::default(), quarantine(&path, dir, stamp)),
    }
}

fn quarantine(path: &Path, dir: &Path, stamp: &str) -> LoadOutcome {
    let target = dir.join(format!("data.corrupt-{stamp}.json"));
    match fs::rename(path, &target) {
        Ok(()) => LoadOutcome::Recovered { moved_to: target },
        // If even renaming fails, keep the file where it is and still report it.
        Err(_) => LoadOutcome::Recovered { moved_to: path.to_path_buf() },
    }
}

/// Saves the data atomically. The first save of each day also writes `backups/data-YYYY-MM-DD.json`.
pub fn save(dir: &Path, data: &Data, today: NaiveDate) -> io::Result<()> {
    let json = serde_json::to_vec_pretty(data).map_err(io::Error::other)?;
    write_atomic(&dir.join(DATA_FILE), &json)?;
    let backups = dir.join(BACKUP_DIR);
    let todays = backups.join(backup_name(today));
    if !todays.exists() {
        write_atomic(&todays, &json)?;
        rotate(&backups, KEEP_BACKUPS);
    }
    Ok(())
}

pub fn backup_name(day: NaiveDate) -> String {
    format!("data-{}.json", day.format("%Y-%m-%d"))
}

/// Automatic backups, newest first.
pub fn list_backups(dir: &Path) -> Vec<(NaiveDate, PathBuf)> {
    let mut out: Vec<(NaiveDate, PathBuf)> = fs::read_dir(dir.join(BACKUP_DIR))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let day = name.strip_prefix("data-")?.strip_suffix(".json")?;
            Some((NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?, e.path()))
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out
}

/// Deletes the oldest automatic backups beyond `keep`.
pub fn rotate(backups_dir: &Path, keep: usize) {
    let dir = backups_dir.parent().unwrap_or(backups_dir);
    for (_, path) in list_backups(dir).into_iter().skip(keep) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::{Profile, Task};

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("planora-test-{name}-{}", super::super::types::new_id()));
        fs::create_dir_all(&p).unwrap();
        p
    }
    fn day(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }
    fn sample() -> Data {
        let now = day(11, 17).and_hms_opt(9, 0, 0).unwrap();
        let mut d = Data::default();
        let mut t = Task::new(Profile::Work, "Write report", now);
        t.deadline = Some(day(11, 20));
        t.description = "multi\nline".into();
        d.tasks.push(t);
        d
    }

    #[test]
    fn round_trip() {
        let dir = tmp("rt");
        let data = sample();
        save(&dir, &data, day(11, 17)).unwrap();
        let (loaded, outcome) = load(&dir, "x");
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(loaded, data);
        assert!(!dir.join("data.json.tmp").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_file_is_fresh() {
        let dir = tmp("fresh");
        let (d, o) = load(&dir, "x");
        assert_eq!(o, LoadOutcome::Fresh);
        assert!(d.tasks.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bom_is_tolerated() {
        let dir = tmp("bom");
        let json = serde_json::to_string(&sample()).unwrap();
        fs::write(dir.join(DATA_FILE), format!("\u{feff}{json}")).unwrap();
        assert_eq!(load(&dir, "x").1, LoadOutcome::Loaded);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn migrates_unversioned_data() {
        let v0 = r#"{"projects":[],"tasks":[{"id":"a","profile":"personal","title":"Old","created":"2026-01-01T10:00:00","updated":"2026-01-01T10:00:00"}]}"#;
        let d = parse(v0).unwrap();
        assert_eq!(d.version, SCHEMA_VERSION);
        assert_eq!(d.tasks[0].title, "Old");
        assert!(d.tasks[0].links.is_empty());
    }

    #[test]
    fn newer_schema_is_rejected() {
        assert!(parse(r#"{"version":99,"tasks":[]}"#).is_err());
    }

    #[test]
    fn corrupt_file_is_moved_not_overwritten() {
        let dir = tmp("corrupt");
        fs::write(dir.join(DATA_FILE), "{ this is not json").unwrap();
        let (d, o) = load(&dir, "20261117-100000");
        assert!(d.tasks.is_empty());
        let LoadOutcome::Recovered { moved_to } = o else { panic!("expected Recovered") };
        assert!(moved_to.ends_with("data.corrupt-20261117-100000.json"));
        assert_eq!(fs::read_to_string(&moved_to).unwrap(), "{ this is not json");
        assert!(!dir.join(DATA_FILE).exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn daily_backup_once_per_day_and_rotation_keeps_14() {
        let dir = tmp("backup");
        let data = sample();
        for i in 1..=20u32 {
            save(&dir, &data, day(10, i)).unwrap();
            save(&dir, &data, day(10, i)).unwrap(); // second save the same day adds nothing
        }
        let list = list_backups(&dir);
        assert_eq!(list.len(), KEEP_BACKUPS);
        assert_eq!(list[0].0, day(10, 20));
        assert_eq!(list.last().unwrap().0, day(10, 7));
        let _ = fs::remove_dir_all(dir);
    }
}
