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
    // v1 -> v2: deadlines may carry a time. Old deadlines are date-only, so every task gets an explicit
    // `deadline_time: null` and keeps behaving exactly as before.
    if version < 2 {
        if let Some(tasks) = obj.get_mut("tasks").and_then(|t| t.as_array_mut()) {
            for t in tasks.iter_mut().filter_map(|t| t.as_object_mut()) {
                t.entry("deadline_time").or_insert(serde_json::Value::Null);
            }
        }
    }
    // v2 -> v3: subtasks and the Trash. Every task gets an explicit `parent_id: null` (a normal top-level task)
    // and the file gets an empty `trash`. Nothing else changes.
    if version < 3 {
        if let Some(tasks) = obj.get_mut("tasks").and_then(|t| t.as_array_mut()) {
            for t in tasks.iter_mut().filter_map(|t| t.as_object_mut()) {
                t.entry("parent_id").or_insert(serde_json::Value::Null);
            }
        }
        obj.entry("trash").or_insert_with(|| serde_json::Value::Array(Vec::new()));
    }
    obj.insert("version".into(), SCHEMA_VERSION.into());
    let mut data: Data = serde_json::from_value(value).map_err(|e| e.to_string())?;
    data.version = SCHEMA_VERSION;
    data.normalize();
    Ok(data)
}

/// The schema version a data text was written with (0 when it has none or cannot be read).
pub fn schema_of(text: &str) -> u32 {
    serde_json::from_str::<serde_json::Value>(text.trim_start_matches('\u{feff}'))
        .ok()
        .and_then(|v| v.get("version").and_then(|v| v.as_u64()))
        .unwrap_or(0) as u32
}

/// Name of the copy kept of a file written by an older schema, made before the first save in the new one.
pub fn pre_migration_name(old_version: u32) -> String {
    format!("before-schema-{SCHEMA_VERSION}-from-v{old_version}.json")
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
        Ok(d) => {
            // An older file is upgraded in memory only. Keep its exact bytes in `backups/` before anything is
            // written in the new schema, so going back to an older Planora loses nothing.
            if let Ok(text) = std::str::from_utf8(&bytes) {
                let old = schema_of(text);
                let copy = dir.join(BACKUP_DIR).join(pre_migration_name(old));
                if old < SCHEMA_VERSION && !copy.exists() {
                    let _ = write_atomic(&copy, &bytes);
                }
            }
            (d, LoadOutcome::Loaded)
        }
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
    fn migrates_v1_date_only_deadlines_to_v2() {
        let v1 = r#"{"version":1,"projects":[],"tasks":[{"id":"a","profile":"work","title":"Report","deadline":"2026-11-20","created":"2026-11-01T10:00:00","updated":"2026-11-01T10:00:00"}]}"#;
        let d = parse(v1).unwrap();
        assert_eq!(d.version, SCHEMA_VERSION);
        assert_eq!(d.tasks[0].deadline, NaiveDate::from_ymd_opt(2026, 11, 20));
        assert_eq!(d.tasks[0].deadline_time, None);
        // A migrated file round-trips as schema 2.
        let again = parse(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(again, d);
    }

    const V2: &str = r#"{
  "version": 2,
  "projects": [{"id":"p1","profile":"work","name":"Site","description":"d","color":4286,"archived":false,"created":"2026-11-01T10:00:00"}],
  "tasks": [
    {"id":"a","profile":"work","project_id":"p1","title":"Write copy","description":"notes","links":[{"url":"https://example.com","title":"x"}],
     "status":"in_progress","priority":"high","deadline":"2026-11-20","deadline_time":"17:30:00","scheduled":"2026-11-19","time":"09:00:00",
     "remind":{"min":15},"recurrence":null,"done_dates":[],"checklist":[{"text":"one","done":true}],
     "created":"2026-11-01T10:00:00","updated":"2026-11-02T10:00:00","completed_at":null},
    {"id":"b","profile":"personal","title":"Gym","recurrence":{"kind":"weekly","interval":1,"until":null},"scheduled":"2026-11-02",
     "done_dates":["2026-11-09"],"created":"2026-11-01T10:00:00","updated":"2026-11-01T10:00:00"}
  ]
}"#;

    #[test]
    fn migrates_v2_to_v3_without_losing_anything() {
        let d = parse(V2).unwrap();
        assert_eq!(d.version, SCHEMA_VERSION);
        assert_eq!(d.tasks.len(), 2);
        assert!(d.trash.is_empty());
        let a = &d.tasks[0];
        assert_eq!(a.parent_id, None);
        assert_eq!(a.project_id.as_deref(), Some("p1"));
        assert_eq!(a.title, "Write copy");
        assert_eq!(a.description, "notes");
        assert_eq!(a.links.len(), 1);
        assert_eq!(a.deadline_time, chrono::NaiveTime::from_hms_opt(17, 30, 0));
        assert_eq!(a.time, chrono::NaiveTime::from_hms_opt(9, 0, 0));
        assert_eq!(a.remind, crate::model::types::Remind::Min(15));
        assert_eq!(a.progress(), (1, 1));
        let b = &d.tasks[1];
        assert!(b.recurrence.is_some());
        assert_eq!(b.done_dates.len(), 1);
        assert_eq!(d.projects[0].name, "Site");
        // The migrated data round-trips unchanged.
        assert_eq!(parse(&serde_json::to_string(&d).unwrap()).unwrap(), d);
    }

    #[test]
    fn upgrade_keeps_the_old_file_in_backups_before_the_first_save() {
        let dir = tmp("premigrate");
        fs::write(dir.join(DATA_FILE), V2).unwrap();
        let (d, o) = load(&dir, "x");
        assert_eq!(o, LoadOutcome::Loaded);
        assert_eq!(d.version, SCHEMA_VERSION);
        // data.json is still the old text, and an exact copy of it is in backups/.
        assert_eq!(fs::read_to_string(dir.join(DATA_FILE)).unwrap(), V2);
        let copy = dir.join(BACKUP_DIR).join(pre_migration_name(2));
        assert_eq!(fs::read_to_string(&copy).unwrap(), V2);
        // The first save writes schema 3, and the copy is not overwritten or rotated away.
        save(&dir, &d, day(11, 24)).unwrap();
        assert_eq!(schema_of(&fs::read_to_string(dir.join(DATA_FILE)).unwrap()), SCHEMA_VERSION);
        assert_eq!(fs::read_to_string(&copy).unwrap(), V2);
        // A second start with a current file makes no new copy.
        let (_, _) = load(&dir, "y");
        assert_eq!(fs::read_dir(dir.join(BACKUP_DIR)).unwrap().count(), 2); // the copy and the daily backup
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn trash_and_subtasks_round_trip() {
        let dir = tmp("trashrt");
        let mut d = sample();
        let parent = d.tasks[0].id.clone();
        d.add_subtask(&parent, "Sub", day(11, 17).and_hms_opt(9, 0, 0).unwrap()).unwrap();
        let sub = d.subtasks_of(&parent)[0].id.clone();
        let mut other = Task::new(Profile::Work, "Gone", day(11, 17).and_hms_opt(9, 0, 0).unwrap());
        other.id = "gone".into();
        d.tasks.push(other);
        d.trash_task("gone", day(11, 18).and_hms_opt(8, 0, 0).unwrap());
        save(&dir, &d, day(11, 18)).unwrap();
        let (loaded, _) = load(&dir, "x");
        assert_eq!(loaded, d);
        assert_eq!(loaded.task(&sub).unwrap().parent_id.as_deref(), Some(parent.as_str()));
        assert_eq!(loaded.trash.len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn deadline_time_round_trips() {
        let dir = tmp("dltime");
        let mut d = sample();
        d.tasks[0].deadline_time = chrono::NaiveTime::from_hms_opt(17, 30, 0);
        save(&dir, &d, day(11, 17)).unwrap();
        assert_eq!(load(&dir, "x").0.tasks[0].deadline_time, chrono::NaiveTime::from_hms_opt(17, 30, 0));
        let _ = fs::remove_dir_all(dir);
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
