//! Settings (`config.json`), the data folder and the small runtime state file.

use crate::model::storage::write_atomic;
use crate::model::{Id, Profile, ProfileFilter};
use crate::theme::ThemeMode;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `%APPDATA%\Planora`, or the folder from `PLANORA_DATA_DIR` (used for tests and screenshots).
pub fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("PLANORA_DATA_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("Planora")
}

/// What the main area shows.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Nav {
    #[default]
    Today,
    Calendar,
    Inbox(Profile),
    Project(Id),
    Settings,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortMode {
    #[default]
    Manual,
    Deadline,
    Priority,
    Created,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Window {
    pub w: f32,
    pub h: f32,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    /// Interface language code (`en`, `ru`, `uk`); `None` until first resolved.
    pub language: Option<String>,
    pub theme: ThemeMode,
    /// First day of the week is Sunday (English only; Russian and Ukrainian always start on Monday).
    pub week_starts_sunday: bool,
    /// 24-hour clock (English may switch to 12-hour).
    pub time_24h: bool,
    pub close_to_tray: bool,
    /// Sidebar shows icons only.
    pub sidebar_collapsed: bool,
    /// Default "remind before", in minutes.
    pub default_remind: u32,
    pub notifications: bool,
    pub personal_color: u32,
    pub work_color: u32,
    pub check_updates: bool,
    pub nav: Nav,
    pub filter: ProfileFilter,
    pub hide_done: bool,
    pub sort: SortMode,
    pub window: Option<Window>,
    #[serde(skip)]
    pub fresh: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: None,
            theme: ThemeMode::System,
            week_starts_sunday: false,
            time_24h: true,
            close_to_tray: true,
            sidebar_collapsed: false,
            default_remind: 10,
            notifications: true,
            personal_color: 0x4f6bff,
            work_color: 0xf59e0b,
            check_updates: true,
            nav: Nav::Today,
            filter: ProfileFilter::All,
            hide_done: false,
            sort: SortMode::Manual,
            window: None,
            fresh: false,
        }
    }
}

impl Config {
    pub fn profile_color(&self, p: Profile) -> u32 {
        match p {
            Profile::Personal => self.personal_color,
            Profile::Work => self.work_color,
        }
    }

    pub fn load_from(dir: &std::path::Path) -> Self {
        let text = std::fs::read_to_string(dir.join("config.json")).ok();
        let mut cfg: Config = text.as_deref().and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok()).unwrap_or_default();
        cfg.fresh = text.is_none();
        cfg.default_remind = cfg.default_remind.min(24 * 60);
        cfg
    }

    pub fn save_to(&self, dir: &std::path::Path) {
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = write_atomic(&dir.join("config.json"), &json);
        }
    }

}

/// Runtime state shared between the app and the reminder thread (`state.json`).
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct State {
    /// Reminders up to this moment were already shown (or seen in the missed list).
    pub last_tick: Option<NaiveDateTime>,
}

impl State {
    pub fn load_from(dir: &std::path::Path) -> Self {
        std::fs::read_to_string(dir.join("state.json"))
            .ok()
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn save_to(&self, dir: &std::path::Path) {
        if let Ok(json) = serde_json::to_vec(self) {
            let _ = write_atomic(&dir.join("state.json"), &json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{self, Lang};

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("planora-cfg-{name}-{}", crate::model::new_id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn missing_config_is_fresh_with_defaults() {
        let dir = tmp("fresh");
        let c = Config::load_from(&dir);
        assert!(c.fresh);
        assert_eq!(c.theme, ThemeMode::System);
        assert!(c.close_to_tray && c.notifications);
        assert_eq!(c.default_remind, 10);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tmp("rt");
        let mut c = Config::default();
        c.language = Some("uk".into());
        c.theme = ThemeMode::Dark;
        c.nav = Nav::Project("abc".into());
        c.filter = ProfileFilter::Work;
        c.week_starts_sunday = true;
        c.save_to(&dir);
        let l = Config::load_from(&dir);
        assert!(!l.fresh);
        assert_eq!(l.language.as_deref(), Some("uk"));
        assert_eq!(l.theme, ThemeMode::Dark);
        assert_eq!(l.nav, Nav::Project("abc".into()));
        assert_eq!(l.filter, ProfileFilter::Work);
        assert!(l.week_starts_sunday);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn bom_and_unknown_fields_are_tolerated() {
        let dir = tmp("bom");
        std::fs::write(dir.join("config.json"), "\u{feff}{\"theme\":\"light\",\"future_key\":1}").unwrap();
        assert_eq!(Config::load_from(&dir).theme, ThemeMode::Light);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn language_order_config_then_marker_then_english() {
        // 1. The config wins over everything.
        assert_eq!(i18n::resolve(Some("ru")), Lang::Ru);
        assert_eq!(i18n::resolve(Some("uk")), Lang::Uk);
        // 2. Without a config value the marker next to the exe is used; there is none next to the
        //    test binary, so 3. English.
        if i18n::marker_language().is_none() {
            assert_eq!(i18n::resolve(None), Lang::En);
            assert_eq!(i18n::resolve(Some("klingon")), Lang::En);
        }
    }

    #[test]
    fn state_round_trip() {
        let dir = tmp("state");
        assert!(State::load_from(&dir).last_tick.is_none());
        let t = chrono::NaiveDate::from_ymd_opt(2026, 11, 17).unwrap().and_hms_opt(10, 0, 0).unwrap();
        State { last_tick: Some(t) }.save_to(&dir);
        assert_eq!(State::load_from(&dir).last_tick, Some(t));
        let _ = std::fs::remove_dir_all(dir);
    }
}
