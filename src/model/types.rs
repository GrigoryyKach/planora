//! Plain data types of the planner. No UI code lives here.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Current on-disk schema version of `data.json`.
pub const SCHEMA_VERSION: u32 = 1;

pub type Id = String;

/// Generates a unique, URL-safe id (16 hex digits). Random per process, unique per call.
pub fn new_id() -> Id {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut h = RandomState::new().build_hasher();
    h.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    format!("{:016x}", h.finish())
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Personal,
    Work,
}

impl Profile {
    pub const ALL: [Profile; 2] = [Profile::Personal, Profile::Work];
}

/// Which profiles the calendar and Today show.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProfileFilter {
    #[default]
    All,
    Personal,
    Work,
}

impl ProfileFilter {
    pub fn allows(self, p: Profile) -> bool {
        match self {
            ProfileFilter::All => true,
            ProfileFilter::Personal => p == Profile::Personal,
            ProfileFilter::Work => p == Profile::Work,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Todo,
    InProgress,
    Done,
    Cancelled,
}

impl Status {
    pub const ALL: [Status; 4] = [Status::Todo, Status::InProgress, Status::Done, Status::Cancelled];

    /// Click-to-cycle order: none -> todo -> in progress -> done -> none.
    pub fn cycle(s: Option<Status>) -> Option<Status> {
        match s {
            None | Some(Status::Cancelled) => Some(Status::Todo),
            Some(Status::Todo) => Some(Status::InProgress),
            Some(Status::InProgress) => Some(Status::Done),
            Some(Status::Done) => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

impl Priority {
    pub const ALL: [Priority; 4] = [Priority::Low, Priority::Medium, Priority::High, Priority::Urgent];
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RecKind {
    Daily,
    Weekdays,
    Weekly,
    Monthly,
    /// Every `interval` days.
    Days,
}

impl RecKind {
    pub const ALL: [RecKind; 5] = [RecKind::Daily, RecKind::Weekdays, RecKind::Weekly, RecKind::Monthly, RecKind::Days];
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Recurrence {
    pub kind: RecKind,
    /// Used by `RecKind::Days` only (at least 1).
    #[serde(default = "one")]
    pub interval: u32,
    #[serde(default)]
    pub until: Option<NaiveDate>,
}

fn one() -> u32 {
    1
}

/// Per-task reminder choice.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Remind {
    /// Use the default from the settings.
    #[default]
    Default,
    Off,
    /// This many minutes before the task time (0 = at the time).
    Min(u32),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct Link {
    pub url: String,
    #[serde(default)]
    pub title: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct CheckItem {
    pub text: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Project {
    pub id: Id,
    pub profile: Profile,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// `0xRRGGBB`.
    pub color: u32,
    #[serde(default)]
    pub archived: bool,
    pub created: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Task {
    pub id: Id,
    pub profile: Profile,
    #[serde(default)]
    pub project_id: Option<Id>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub deadline: Option<NaiveDate>,
    /// Day the task is planned on.
    #[serde(default)]
    pub scheduled: Option<NaiveDate>,
    /// Exact time on the scheduled day; only meaningful together with `scheduled`.
    #[serde(default)]
    pub time: Option<NaiveTime>,
    #[serde(default)]
    pub remind: Remind,
    #[serde(default)]
    pub recurrence: Option<Recurrence>,
    /// Completed occurrences of a recurring task (by occurrence date).
    #[serde(default)]
    pub done_dates: Vec<NaiveDate>,
    #[serde(default)]
    pub checklist: Vec<CheckItem>,
    pub created: NaiveDateTime,
    pub updated: NaiveDateTime,
    #[serde(default)]
    pub completed_at: Option<NaiveDateTime>,
}

impl Task {
    pub fn new(profile: Profile, title: &str, now: NaiveDateTime) -> Self {
        Task {
            id: new_id(),
            profile,
            project_id: None,
            title: title.trim().to_string(),
            description: String::new(),
            links: Vec::new(),
            status: None,
            priority: None,
            deadline: None,
            scheduled: None,
            time: None,
            remind: Remind::Default,
            recurrence: None,
            done_dates: Vec::new(),
            checklist: Vec::new(),
            created: now,
            updated: now,
            completed_at: None,
        }
    }

    /// Done or cancelled (a non-recurring task counts as closed).
    pub fn is_closed(&self) -> bool {
        matches!(self.status, Some(Status::Done | Status::Cancelled))
    }

    /// Anchor of the recurrence: the scheduled day, else the deadline.
    pub fn anchor(&self) -> Option<NaiveDate> {
        self.scheduled.or(self.deadline)
    }

    /// `(done, total)` of the checklist.
    pub fn progress(&self) -> (usize, usize) {
        (self.checklist.iter().filter(|c| c.done).count(), self.checklist.len())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Data {
    pub version: u32,
    #[serde(default)]
    pub projects: Vec<Project>,
    /// The vector order is the "manual" sort order.
    #[serde(default)]
    pub tasks: Vec<Task>,
}

impl Default for Data {
    fn default() -> Self {
        Data { version: SCHEMA_VERSION, projects: Vec::new(), tasks: Vec::new() }
    }
}

impl Data {
    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }
    pub fn task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }
}
