//! Which reminders are due: the next one to wait for, and the ones missed while the app was off.

use super::calendar::is_done_on;
use super::recurrence::occurrences;
use super::types::{Id, Remind, Task};
use chrono::{Duration, NaiveDate, NaiveDateTime};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reminder {
    pub task_id: Id,
    pub title: String,
    /// When the task happens (its plan time or its deadline time).
    pub event: NaiveDateTime,
    /// The reminder is for the deadline, not for the plan time.
    pub deadline: bool,
    /// When the reminder fires (`event` minus `before` minutes).
    pub at: NaiveDateTime,
    pub before: u32,
}

/// Longest "remind before" we honour, in minutes (one day).
pub const MAX_BEFORE: u32 = 24 * 60;

/// Minutes before the task time, or `None` when reminders are off for the task.
pub fn before_minutes(task: &Task, default_before: u32) -> Option<u32> {
    match task.remind {
        Remind::Default => Some(default_before),
        Remind::Off => None,
        Remind::Min(m) => Some(m),
    }
    .map(|m| m.min(MAX_BEFORE))
}

/// Exact moments of the open occurrences whose day lies in `from..=to`: plan times and deadline times.
/// The flag is `true` for a deadline. One reminder setting (`Task::remind`) applies to both kinds.
fn events(task: &Task, from: NaiveDate, to: NaiveDate) -> Vec<(NaiveDateTime, bool)> {
    let mut out = Vec::new();
    if let (Some(day), Some(time)) = (task.scheduled, task.time) {
        let days = match &task.recurrence {
            Some(rec) => occurrences(day, rec, from, to),
            None if (from..=to).contains(&day) => vec![day],
            None => Vec::new(),
        };
        out.extend(days.into_iter().filter(|d| !is_done_on(task, *d)).map(|d| (d.and_time(time), false)));
    }
    if let (Some(dl), Some(time)) = (task.deadline, task.deadline_time) {
        match (&task.recurrence, task.anchor()) {
            (Some(rec), Some(anchor)) => {
                let off = dl - anchor;
                for o in occurrences(anchor, rec, from - off, to - off) {
                    if !is_done_on(task, o) {
                        out.push(((o + off).and_time(time), true));
                    }
                }
            }
            _ if (from..=to).contains(&dl) && !is_done_on(task, dl) => out.push((dl.and_time(time), true)),
            _ => {}
        }
    }
    out
}

/// Reminders that fire in `(after, upto]`, ordered by time.
pub fn reminders_between(tasks: &[Task], default_before: u32, after: NaiveDateTime, upto: NaiveDateTime) -> Vec<Reminder> {
    let mut out = Vec::new();
    if upto <= after {
        return out;
    }
    let from = after.date();
    let to = (upto + Duration::minutes(MAX_BEFORE as i64)).date();
    for t in tasks {
        let Some(before) = before_minutes(t, default_before) else { continue };
        for (event, deadline) in events(t, from, to) {
            let at = event - Duration::minutes(before as i64);
            if at > after && at <= upto {
                out.push(Reminder { task_id: t.id.clone(), title: t.title.clone(), event, deadline, at, before });
            }
        }
    }
    out.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| a.task_id.cmp(&b.task_id)).then_with(|| a.deadline.cmp(&b.deadline)));
    out
}

/// The earliest reminder strictly after `after` (looks about a year ahead).
pub fn next_reminder(tasks: &[Task], default_before: u32, after: NaiveDateTime) -> Option<Reminder> {
    let mut start = after;
    for _ in 0..13 {
        let end = start + Duration::days(30);
        if let Some(r) = reminders_between(tasks, default_before, start, end).into_iter().next() {
            return Some(r);
        }
        start = end;
    }
    None
}

/// Reminders missed while the app was not running: fired in `(last_run, now]`.
/// Looks back at most `max_days` so an old state file does not flood the list.
pub fn missed_reminders(tasks: &[Task], default_before: u32, last_run: NaiveDateTime, now: NaiveDateTime, max_days: i64) -> Vec<Reminder> {
    let earliest = now - Duration::days(max_days);
    reminders_between(tasks, default_before, last_run.max(earliest), now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::{Profile, RecKind, Recurrence, Status};
    use chrono::NaiveTime;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, 0).unwrap()
    }
    fn timed(title: &str, day: NaiveDate, h: u32, mi: u32) -> Task {
        let mut t = Task::new(Profile::Personal, title, dt(2026, 11, 1, 0, 0));
        t.scheduled = Some(day);
        t.time = NaiveTime::from_hms_opt(h, mi, 0);
        t
    }
    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn next_reminder_uses_offset_and_order() {
        let a = timed("call", day(2026, 11, 18), 10, 0);
        let mut b = timed("later", day(2026, 11, 18), 12, 0);
        b.remind = Remind::Min(30);
        let now = dt(2026, 11, 17, 9, 0);
        let r = next_reminder(&[b.clone(), a.clone()], 10, now).unwrap();
        assert_eq!(r.task_id, a.id);
        assert_eq!(r.at, dt(2026, 11, 18, 9, 50));
        let r2 = next_reminder(&[a, b.clone()], 10, dt(2026, 11, 18, 9, 50)).unwrap();
        assert_eq!(r2.task_id, b.id);
        assert_eq!(r2.at, dt(2026, 11, 18, 11, 30));
    }

    #[test]
    fn off_done_and_untimed_are_skipped() {
        let mut off = timed("off", day(2026, 11, 18), 10, 0);
        off.remind = Remind::Off;
        let mut done = timed("done", day(2026, 11, 18), 11, 0);
        done.status = Some(Status::Done);
        let mut untimed = timed("untimed", day(2026, 11, 18), 11, 0);
        untimed.time = None;
        assert!(next_reminder(&[off, done, untimed], 10, dt(2026, 11, 17, 0, 0)).is_none());
    }

    #[test]
    fn reminder_at_zero_minutes_fires_at_event() {
        let mut t = timed("x", day(2026, 11, 18), 10, 0);
        t.remind = Remind::Min(0);
        let r = reminders_between(&[t], 5, dt(2026, 11, 18, 9, 59), dt(2026, 11, 18, 10, 0));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].at, dt(2026, 11, 18, 10, 0));
    }

    #[test]
    fn range_is_after_exclusive_upto_inclusive() {
        let t = timed("x", day(2026, 11, 18), 10, 0); // default 10 -> 09:50
        assert!(reminders_between(&[t.clone()], 10, dt(2026, 11, 18, 9, 50), dt(2026, 11, 18, 10, 0)).is_empty());
        assert_eq!(reminders_between(&[t], 10, dt(2026, 11, 18, 9, 49), dt(2026, 11, 18, 9, 50)).len(), 1);
    }

    #[test]
    fn missed_after_downtime() {
        let a = timed("a", day(2026, 11, 15), 9, 0);
        let b = timed("b", day(2026, 11, 16), 9, 0);
        let c = timed("c", day(2026, 11, 20), 9, 0);
        let m = missed_reminders(&[a.clone(), b.clone(), c], 10, dt(2026, 11, 15, 12, 0), dt(2026, 11, 17, 8, 0), 14);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].task_id, b.id);
        // lookback limit
        let m = missed_reminders(&[a], 10, dt(2026, 1, 1, 0, 0), dt(2026, 11, 17, 8, 0), 1);
        assert!(m.is_empty());
    }

    fn with_deadline(title: &str, day: NaiveDate, h: u32, mi: u32) -> Task {
        let mut t = Task::new(Profile::Work, title, dt(2026, 11, 1, 0, 0));
        t.deadline = Some(day);
        t.deadline_time = NaiveTime::from_hms_opt(h, mi, 0);
        t
    }

    #[test]
    fn deadline_time_gets_a_reminder_with_the_shared_setting() {
        let mut t = with_deadline("report due", day(2026, 11, 20), 17, 0);
        t.remind = Remind::Min(30);
        let r = next_reminder(&[t.clone()], 10, dt(2026, 11, 19, 0, 0)).unwrap();
        assert!(r.deadline);
        assert_eq!(r.event, dt(2026, 11, 20, 17, 0));
        assert_eq!(r.at, dt(2026, 11, 20, 16, 30));
        // A date-only deadline has no reminder.
        t.deadline_time = None;
        assert!(next_reminder(&[t], 10, dt(2026, 11, 19, 0, 0)).is_none());
    }

    #[test]
    fn plan_and_deadline_each_remind_and_done_silences_both() {
        let mut t = timed("both", day(2026, 11, 18), 9, 0);
        t.deadline = Some(day(2026, 11, 20));
        t.deadline_time = NaiveTime::from_hms_opt(17, 0, 0);
        let all = reminders_between(&[t.clone()], 10, dt(2026, 11, 17, 0, 0), dt(2026, 11, 21, 0, 0));
        assert_eq!(all.len(), 2);
        assert!(!all[0].deadline && all[1].deadline);
        t.status = Some(Status::Done);
        assert!(reminders_between(&[t], 10, dt(2026, 11, 17, 0, 0), dt(2026, 11, 21, 0, 0)).is_empty());
    }

    #[test]
    fn missed_deadline_reminders_are_listed() {
        let t = with_deadline("due while off", day(2026, 11, 16), 12, 0);
        let m = missed_reminders(&[t], 10, dt(2026, 11, 15, 12, 0), dt(2026, 11, 17, 8, 0), 14);
        assert_eq!(m.len(), 1);
        assert!(m[0].deadline);
    }

    #[test]
    fn recurring_reminders_skip_done_occurrences() {
        let mut t = timed("daily", day(2026, 11, 1), 8, 0);
        t.recurrence = Some(Recurrence { kind: RecKind::Daily, interval: 1, until: None });
        t.done_dates.push(day(2026, 11, 18));
        let r = reminders_between(&[t], 0, dt(2026, 11, 17, 12, 0), dt(2026, 11, 19, 12, 0));
        let ats: Vec<_> = r.iter().map(|r| r.at).collect();
        assert_eq!(ats, vec![dt(2026, 11, 19, 8, 0)]);
    }
}
