//! The Trash: deleted tasks are moved out of `Data::tasks` into `Data::trash`, kept for a few days, then purged.
//! Because trashed tasks are not in `tasks`, no view, count, search, export or reminder can see them.

use super::types::{Data, Id, Profile, TrashEntry};
use chrono::{Duration, NaiveDateTime};

/// Choices for "Keep deleted tasks for", in days. The first is the default.
pub const RETENTION_OPTIONS: [u32; 3] = [7, 14, 30];
pub const DEFAULT_RETENTION: u32 = RETENTION_OPTIONS[0];

/// Snaps a stored value to a supported one (an unknown value falls back to the default).
pub fn clamp_retention(days: u32) -> u32 {
    if RETENTION_OPTIONS.contains(&days) { days } else { DEFAULT_RETENTION }
}

/// What a restore did, for the message shown to the user.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Restored {
    /// The restored task (the one that was deleted).
    pub id: Id,
    /// Tasks put back, subtasks included.
    pub count: usize,
    /// Its project no longer exists: it went to the Inbox.
    pub project_missing: bool,
    /// It was a subtask but its parent is gone or in the Trash: it came back as a normal task.
    pub detached: bool,
}

impl Data {
    /// Moves a task, with its subtasks when it is a parent, to the Trash. Deleting a subtask trashes only that
    /// subtask. Returns the number of tasks moved (0 if the id is unknown).
    pub fn trash_task(&mut self, id: &str, now: NaiveDateTime) -> usize {
        let Some(index) = self.tasks.iter().position(|t| t.id == id) else { return 0 };
        let root = self.tasks.remove(index);
        let (subs, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.tasks).into_iter().partition(|t| t.parent_id.as_deref() == Some(id));
        self.tasks = rest;
        let project_name = root.project_id.as_deref().and_then(|p| self.project(p)).map(|p| p.name.clone());
        let parent_title = root.parent_id.as_deref().and_then(|p| self.task(p)).map(|p| p.title.clone());
        let mut tasks = vec![root];
        tasks.extend(subs);
        let count = tasks.len();
        self.trash.push(TrashEntry { id: id.to_string(), deleted_at: now, tasks, index, project_name, parent_title });
        count
    }

    /// Trashes every task of a project (each top-level task with its subtasks). Returns the number of tasks moved.
    pub fn trash_project_tasks(&mut self, project: &str, now: NaiveDateTime) -> usize {
        let ids: Vec<Id> = self.tasks.iter().filter(|t| t.parent_id.is_none() && t.project_id.as_deref() == Some(project)).map(|t| t.id.clone()).collect();
        ids.iter().map(|id| self.trash_task(id, now)).sum()
    }

    /// Puts a trashed task back where it was, with all its fields and its subtasks.
    ///
    /// * A subtask whose parent is still there returns under that parent.
    /// * A subtask whose parent is gone, or itself in the Trash, comes back as a normal task in the project it
    ///   had (`detached`). Restore the parent entry to get the old structure back.
    /// * If the project no longer exists everything goes to the Inbox (`project_missing`).
    pub fn restore_from_trash(&mut self, entry_id: &str) -> Option<Restored> {
        let pos = self.trash.iter().position(|e| e.id == entry_id)?;
        let mut entry = self.trash.remove(pos);
        let mut project_missing = false;
        let mut detached = false;
        let parent = entry.tasks[0].parent_id.clone();
        let mut slot = entry.index.min(self.tasks.len());
        match parent {
            Some(pid) if self.task(&pid).is_some_and(|p| p.parent_id.is_none()) => {
                let (profile, project) = self.task(&pid).map(|p| (p.profile, p.project_id.clone())).unwrap_or((Profile::Personal, None));
                for t in &mut entry.tasks {
                    t.profile = profile;
                    t.project_id = project.clone();
                }
                let last = self.tasks.iter().rposition(|t| t.parent_id.as_deref() == Some(pid.as_str()));
                slot = last.or_else(|| self.tasks.iter().position(|t| t.id == pid)).map_or(self.tasks.len(), |i| i + 1);
            }
            other => {
                if other.is_some() {
                    detached = true;
                    entry.tasks[0].parent_id = None;
                }
                if let Some(pid) = entry.tasks[0].project_id.clone() {
                    if self.project(&pid).is_none() {
                        project_missing = true;
                        for t in &mut entry.tasks {
                            t.project_id = None;
                        }
                    }
                }
            }
        }
        let id = entry.id.clone();
        let count = entry.tasks.len();
        for (i, t) in entry.tasks.into_iter().enumerate() {
            self.tasks.insert((slot + i).min(self.tasks.len()), t);
        }
        Some(Restored { id, count, project_missing, detached })
    }

    /// Removes the items that have been in the Trash for `days` days or longer. Returns how many were removed.
    pub fn purge_expired(&mut self, now: NaiveDateTime, days: u32) -> usize {
        let before = self.trash.len();
        self.trash.retain(|e| now - e.deleted_at < Duration::days(days as i64));
        before - self.trash.len()
    }

    /// Deletes one item for good.
    pub fn delete_forever(&mut self, entry_id: &str) -> bool {
        let before = self.trash.len();
        self.trash.retain(|e| e.id != entry_id);
        self.trash.len() != before
    }

    /// Empties the Trash. Returns the number of items removed.
    pub fn empty_trash(&mut self) -> usize {
        std::mem::take(&mut self.trash).len()
    }

    /// Whole days left before an item is purged (0 = at the next cleanup).
    pub fn days_left(entry: &TrashEntry, now: NaiveDateTime, days: u32) -> i64 {
        let left = entry.deleted_at + Duration::days(days as i64) - now;
        if left <= Duration::zero() { 0 } else { (left.num_seconds() + 86_399) / 86_400 }
    }

    /// The Trash, newest first.
    pub fn trash_newest_first(&self) -> Vec<&TrashEntry> {
        let mut v: Vec<&TrashEntry> = self.trash.iter().rev().collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.deleted_at));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::calendar::{items_in_range, overdue_items};
    use crate::model::reminders::{missed_reminders, next_reminder, reminders_between};
    use crate::model::types::{Profile, ProfileFilter, Project, Task};
    use chrono::{NaiveDate, NaiveTime};

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }
    fn at(d: u32, h: u32) -> NaiveDateTime {
        day(d).and_hms_opt(h, 0, 0).unwrap()
    }
    fn project(d: &mut Data, id: &str, name: &str) {
        d.projects.push(Project { id: id.into(), profile: Profile::Work, name: name.into(), description: String::new(), color: 0, archived: false, created: at(1, 0) });
    }
    fn task(d: &mut Data, title: &str, project: Option<&str>) -> Id {
        let mut t = Task::new(Profile::Work, title, at(1, 0));
        t.project_id = project.map(Into::into);
        let id = t.id.clone();
        d.tasks.push(t);
        id
    }
    fn titles(d: &Data) -> Vec<&str> {
        d.tasks.iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn trashing_a_parent_takes_its_subtasks_as_one_group() {
        let mut d = Data::default();
        let a = task(&mut d, "A", None);
        d.add_subtask(&a, "a1", at(2, 0));
        d.add_subtask(&a, "a2", at(2, 0));
        let b = task(&mut d, "B", None);
        assert_eq!(d.trash_task(&a, at(3, 0)), 3);
        assert_eq!(titles(&d), ["B"]);
        assert_eq!(d.trash.len(), 1);
        assert_eq!(d.trash[0].subtask_count(), 2);
        assert_eq!(d.trash[0].root().id, a);
        assert!(d.task(&b).is_some());
        assert_eq!(d.trash_task("missing", at(3, 0)), 0);
    }

    #[test]
    fn trashing_a_subtask_takes_only_that_subtask() {
        let mut d = Data::default();
        let a = task(&mut d, "A", None);
        let s1 = d.add_subtask(&a, "a1", at(2, 0)).unwrap();
        d.add_subtask(&a, "a2", at(2, 0));
        assert_eq!(d.trash_task(&s1, at(3, 0)), 1);
        assert_eq!(titles(&d), ["A", "a2"]);
        assert_eq!(d.trash[0].parent_title.as_deref(), Some("A"));
    }

    #[test]
    fn restore_puts_everything_back_in_place() {
        let mut d = Data::default();
        project(&mut d, "p", "Video");
        let a = task(&mut d, "A", Some("p"));
        let b = task(&mut d, "B", Some("p"));
        let s1 = d.add_subtask(&b, "b1", at(2, 0)).unwrap();
        let c = task(&mut d, "C", Some("p"));
        {
            let t = d.task_mut(&b).unwrap();
            t.description = "keep me".into();
            t.links.push(crate::model::types::Link { url: "https://example.com".into(), title: "t".into() });
            t.checklist.push(crate::model::types::CheckItem { text: "x".into(), done: true });
            t.scheduled = Some(day(25));
            t.time = NaiveTime::from_hms_opt(10, 0, 0);
        }
        d.task_mut(&s1).unwrap().deadline = Some(day(26));
        let snapshot = d.clone();
        d.trash_task(&b, at(3, 0));
        assert_eq!(titles(&d), ["A", "C"]);
        assert_eq!(d.trash[0].project_name.as_deref(), Some("Video"));
        let r = d.restore_from_trash(&b).unwrap();
        assert_eq!(r, Restored { id: b.clone(), count: 2, project_missing: false, detached: false });
        assert!(d.trash.is_empty());
        // Same order, same content, same ids: nothing was lost.
        assert_eq!(d.tasks, snapshot.tasks);
        let _ = (a, c);
    }

    #[test]
    fn restore_when_the_project_is_gone_goes_to_the_inbox() {
        let mut d = Data::default();
        project(&mut d, "p", "Video");
        let a = task(&mut d, "A", Some("p"));
        let s = d.add_subtask(&a, "a1", at(2, 0)).unwrap();
        d.trash_task(&a, at(3, 0));
        d.projects.clear();
        let r = d.restore_from_trash(&a).unwrap();
        assert!(r.project_missing && !r.detached);
        assert_eq!(d.task(&a).unwrap().project_id, None);
        assert_eq!(d.task(&s).unwrap().project_id, None);
        assert_eq!(d.task(&s).unwrap().parent_id.as_deref(), Some(a.as_str()));
    }

    #[test]
    fn restoring_a_subtask_under_a_live_parent_rejoins_it() {
        let mut d = Data::default();
        project(&mut d, "p", "Video");
        let a = task(&mut d, "A", Some("p"));
        let s1 = d.add_subtask(&a, "a1", at(2, 0)).unwrap();
        d.add_subtask(&a, "a2", at(2, 0));
        d.trash_task(&s1, at(3, 0));
        let r = d.restore_from_trash(&s1).unwrap();
        assert!(!r.detached && !r.project_missing);
        assert_eq!(d.task(&s1).unwrap().parent_id.as_deref(), Some(a.as_str()));
        assert_eq!(d.subtasks_of(&a).len(), 2);
    }

    #[test]
    fn restoring_an_orphaned_subtask_makes_it_a_normal_task() {
        let mut d = Data::default();
        project(&mut d, "p", "Video");
        let a = task(&mut d, "A", Some("p"));
        let s1 = d.add_subtask(&a, "a1", at(2, 0)).unwrap();
        d.add_subtask(&a, "a2", at(2, 0));
        d.trash_task(&s1, at(3, 0));
        // The parent goes to the Trash after the subtask: the subtask's own entry is now an orphan.
        d.trash_task(&a, at(4, 0));
        let r = d.restore_from_trash(&s1).unwrap();
        assert!(r.detached && !r.project_missing);
        let t = d.task(&s1).unwrap();
        assert!(t.parent_id.is_none());
        assert_eq!(t.project_id.as_deref(), Some("p"));
        // Restoring the parent later still works and keeps its remaining subtask.
        let r = d.restore_from_trash(&a).unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(d.subtasks_of(&a).len(), 1);
    }

    #[test]
    fn purge_by_age_for_7_14_and_30_days() {
        for days in RETENTION_OPTIONS {
            let mut d = Data::default();
            let ids: Vec<Id> = (0..3).map(|i| task(&mut d, &format!("t{i}"), None)).collect();
            let now = at(24, 12);
            // Deleted just under the limit, exactly at it, and well past it.
            d.trash_task(&ids[0], now - Duration::days(days as i64) + Duration::hours(1));
            d.trash_task(&ids[1], now - Duration::days(days as i64));
            d.trash_task(&ids[2], now - Duration::days(days as i64 + 5));
            assert_eq!(d.purge_expired(now, days), 2, "{days} days");
            assert_eq!(d.trash.len(), 1);
            assert_eq!(d.trash[0].id, ids[0]);
        }
    }

    #[test]
    fn shortening_the_retention_removes_older_items_at_the_next_purge() {
        let mut d = Data::default();
        let a = task(&mut d, "old", None);
        let b = task(&mut d, "new", None);
        let now = at(24, 12);
        d.trash_task(&a, now - Duration::days(10));
        d.trash_task(&b, now - Duration::days(2));
        // With 14 days both stay; nothing happens just because the setting changed ...
        assert_eq!(d.purge_expired(now, 14), 0);
        // ... but the next purge with 7 days removes the 10-day-old one only.
        assert_eq!(d.purge_expired(now, 7), 1);
        assert_eq!(d.trash.len(), 1);
        assert_eq!(d.trash[0].id, b);
        // Lengthening never brings anything back.
        assert_eq!(d.purge_expired(now, 30), 0);
    }

    #[test]
    fn days_left_counts_down_and_never_goes_negative() {
        let mut d = Data::default();
        let a = task(&mut d, "a", None);
        let now = at(24, 12);
        d.trash_task(&a, now);
        let e = &d.trash[0];
        assert_eq!(Data::days_left(e, now, 7), 7);
        assert_eq!(Data::days_left(e, now + Duration::days(3), 7), 4);
        assert_eq!(Data::days_left(e, now + Duration::days(3) + Duration::hours(1), 7), 4);
        assert_eq!(Data::days_left(e, now + Duration::days(7), 7), 0);
        assert_eq!(Data::days_left(e, now + Duration::days(20), 7), 0);
    }

    #[test]
    fn delete_forever_and_empty() {
        let mut d = Data::default();
        let a = task(&mut d, "a", None);
        let b = task(&mut d, "b", None);
        d.trash_task(&a, at(2, 0));
        d.trash_task(&b, at(3, 0));
        assert_eq!(d.trash_newest_first()[0].id, b);
        assert!(d.delete_forever(&b));
        assert!(!d.delete_forever(&b));
        assert_eq!(d.empty_trash(), 1);
        assert!(d.trash.is_empty());
    }

    #[test]
    fn trashing_a_project_moves_its_tasks_and_subtasks() {
        let mut d = Data::default();
        project(&mut d, "p", "Video");
        let a = task(&mut d, "A", Some("p"));
        d.add_subtask(&a, "a1", at(2, 0));
        task(&mut d, "B", Some("p"));
        let other = task(&mut d, "Other", None);
        assert_eq!(d.trash_project_tasks("p", at(3, 0)), 3);
        assert_eq!(titles(&d), ["Other"]);
        assert_eq!(d.trash.len(), 2);
        assert!(d.task(&other).is_some());
    }

    #[test]
    fn trashed_tasks_are_invisible_to_every_query_and_to_reminders() {
        let mut d = Data::default();
        let mut t = Task::new(Profile::Work, "Standup notes", at(1, 0));
        t.scheduled = Some(day(25));
        t.time = NaiveTime::from_hms_opt(10, 0, 0);
        t.deadline = Some(day(20));
        t.deadline_time = NaiveTime::from_hms_opt(9, 0, 0);
        let id = t.id.clone();
        d.tasks.push(t);
        let sub = d.add_subtask(&id, "Notes subtask", at(1, 0)).unwrap();
        {
            let s = d.task_mut(&sub).unwrap();
            s.scheduled = Some(day(25));
            s.time = NaiveTime::from_hms_opt(11, 0, 0);
        }
        // Visible before ...
        let now = at(24, 8);
        let q = |d: &Data| items_in_range(&d.tasks, day(1), day(30), ProfileFilter::All, false, now).values().map(Vec::len).sum::<usize>();
        assert!(q(&d) >= 3);
        assert_eq!(overdue_items(&d.tasks, now, ProfileFilter::All).len(), 1);
        assert!(next_reminder(&d.tasks, 10, now).is_some());
        assert_eq!(d.search("notes").len(), 2);
        // ... and gone after the delete, for every query.
        d.trash_task(&id, now);
        assert_eq!(q(&d), 0);
        assert!(overdue_items(&d.tasks, now, ProfileFilter::All).is_empty());
        assert!(next_reminder(&d.tasks, 10, now).is_none());
        assert!(reminders_between(&d.tasks, 10, at(1, 0), at(30, 0)).is_empty());
        assert!(missed_reminders(&d.tasks, 10, at(20, 0), now, 14).is_empty());
        assert!(d.search("notes").is_empty());
        assert!(d.task(&id).is_none() && d.task(&sub).is_none());
        // And back again after a restore.
        d.restore_from_trash(&id).unwrap();
        assert!(next_reminder(&d.tasks, 10, now).is_some());
        assert_eq!(d.search("notes").len(), 2);
    }

    #[test]
    fn retention_values_are_snapped_to_the_supported_ones() {
        assert_eq!(clamp_retention(7), 7);
        assert_eq!(clamp_retention(14), 14);
        assert_eq!(clamp_retention(30), 30);
        assert_eq!(clamp_retention(0), 7);
        assert_eq!(clamp_retention(9999), 7);
    }
}
