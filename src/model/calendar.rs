//! Month grid, which tasks appear on which day, overdue logic and quick task operations.

use super::recurrence::{next_occurrence, occurrences};
use super::types::{Id, Profile, ProfileFilter, Status, Task};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use std::collections::BTreeMap;

/// The 42 dates (6 weeks) shown for a month, starting on `week_start`.
pub fn month_grid(year: i32, month: u32, week_start: Weekday) -> Vec<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
    let off = (7 + first.weekday().num_days_from_monday() as i64 - week_start.num_days_from_monday() as i64) % 7;
    let start = first - Duration::days(off);
    (0..42).map(|i| start + Duration::days(i)).collect()
}

/// The month `delta` months away from `(year, month)`.
pub fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let idx = year * 12 + (month as i32 - 1) + delta;
    (idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u32)
}

/// Weekday column headers for a week starting on `week_start`.
pub fn weekday_order(week_start: Weekday) -> [Weekday; 7] {
    let mut w = week_start;
    let mut out = [week_start; 7];
    for slot in out.iter_mut() {
        *slot = w;
        w = w.succ();
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    Scheduled,
    Deadline,
}

/// One chip in the calendar or a row in the Today view.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Item {
    pub task_id: Id,
    pub kind: ItemKind,
    /// The day the chip is shown on.
    pub date: NaiveDate,
    /// The occurrence date of the series (equals `date` for scheduled chips and plain tasks).
    pub occ: NaiveDate,
    pub time: Option<NaiveTime>,
    pub done: bool,
    pub overdue: bool,
    pub profile: Profile,
}

/// Is the (occurrence of the) task finished?
pub fn is_done_on(task: &Task, occ: NaiveDate) -> bool {
    task.is_closed() || (task.recurrence.is_some() && task.done_dates.contains(&occ))
}

/// An open, non-recurring task with a deadline in the past.
pub fn is_overdue(task: &Task, today: NaiveDate) -> bool {
    task.recurrence.is_none() && !task.is_closed() && task.deadline.is_some_and(|d| d < today)
}

/// Items of all tasks that fall within `from..=to`.
pub fn items_in_range(
    tasks: &[Task],
    from: NaiveDate,
    to: NaiveDate,
    filter: ProfileFilter,
    hide_done: bool,
    today: NaiveDate,
) -> BTreeMap<NaiveDate, Vec<Item>> {
    let mut map: BTreeMap<NaiveDate, Vec<Item>> = BTreeMap::new();
    for t in tasks.iter().filter(|t| filter.allows(t.profile)) {
        let mut push = |kind: ItemKind, date: NaiveDate, occ: NaiveDate| {
            let done = is_done_on(t, occ);
            if hide_done && done {
                return;
            }
            let time = if kind == ItemKind::Scheduled { t.time } else { None };
            let overdue = t.recurrence.is_none() && !done && date < today;
            map.entry(date).or_default().push(Item {
                task_id: t.id.clone(),
                kind,
                date,
                occ,
                time,
                done,
                overdue,
                profile: t.profile,
            });
        };
        match (&t.recurrence, t.anchor()) {
            (Some(rec), Some(anchor)) => {
                if let Some(s) = t.scheduled {
                    for o in occurrences(s, rec, from, to) {
                        push(ItemKind::Scheduled, o, o);
                    }
                }
                if let Some(dl) = t.deadline {
                    let off = dl - anchor;
                    for o in occurrences(anchor, rec, from - off, to - off) {
                        push(ItemKind::Deadline, o + off, o);
                    }
                }
            }
            _ => {
                if let Some(s) = t.scheduled.filter(|s| (from..=to).contains(s)) {
                    push(ItemKind::Scheduled, s, s);
                }
                if let Some(dl) = t.deadline.filter(|d| (from..=to).contains(d)) {
                    push(ItemKind::Deadline, dl, dl);
                }
            }
        }
    }
    for items in map.values_mut() {
        items.sort_by_key(|i| (i.kind == ItemKind::Deadline, i.time.is_none(), i.time));
    }
    map
}

/// Open non-recurring items from before `today` (overdue deadlines and missed scheduled days).
pub fn overdue_items(tasks: &[Task], today: NaiveDate, filter: ProfileFilter) -> Vec<Item> {
    let mut out = Vec::new();
    for t in tasks.iter().filter(|t| filter.allows(t.profile) && t.recurrence.is_none() && !t.is_closed()) {
        // A task with a deadline is overdue by the deadline; without one, by its planned day.
        let (kind, date) = match (t.deadline, t.scheduled) {
            (Some(d), _) => (ItemKind::Deadline, d),
            (None, Some(s)) => (ItemKind::Scheduled, s),
            _ => continue,
        };
        if date < today {
            out.push(Item {
                task_id: t.id.clone(),
                kind,
                date,
                occ: date,
                time: t.time.filter(|_| kind == ItemKind::Scheduled),
                done: false,
                overdue: true,
                profile: t.profile,
            });
        }
    }
    out.sort_by_key(|i| (i.date, i.time));
    out
}

/// For a recurring task: the occurrence a list row acts on (the next one not done, from today).
pub fn current_occurrence(task: &Task, today: NaiveDate) -> Option<NaiveDate> {
    let rec = task.recurrence.as_ref()?;
    let anchor = task.anchor()?;
    let mut from = today;
    for _ in 0..400 {
        let d = next_occurrence(anchor, rec, from)?;
        if !task.done_dates.contains(&d) {
            return Some(d);
        }
        from = d + Duration::days(1);
    }
    None
}

/// Toggles "done". For a recurring task only the given occurrence is toggled; the series stays open.
pub fn toggle_done(task: &mut Task, occ: Option<NaiveDate>, now: NaiveDateTime) {
    if let (Some(_), Some(o)) = (&task.recurrence, occ) {
        if let Some(pos) = task.done_dates.iter().position(|d| *d == o) {
            task.done_dates.remove(pos);
        } else {
            task.done_dates.push(o);
        }
    } else if task.status == Some(Status::Done) {
        task.status = None;
        task.completed_at = None;
    } else {
        task.status = Some(Status::Done);
        task.completed_at = Some(now);
    }
    task.updated = now;
}

/// Sets the status keeping `completed_at` consistent.
pub fn set_status(task: &mut Task, status: Option<Status>, now: NaiveDateTime) {
    task.status = status;
    task.completed_at = if status == Some(Status::Done) { Some(now) } else { None };
    task.updated = now;
}

/// Drag and drop in the calendar: moves the scheduled day or the deadline by the distance between days.
pub fn move_item(task: &mut Task, kind: ItemKind, from_day: NaiveDate, to_day: NaiveDate, now: NaiveDateTime) {
    let delta = to_day - from_day;
    match kind {
        ItemKind::Scheduled => task.scheduled = task.scheduled.map(|d| d + delta),
        ItemKind::Deadline => task.deadline = task.deadline.map(|d| d + delta),
    }
    task.updated = now;
}

/// Monday-based week start for a language/setting.
pub fn week_start_from(sunday: bool) -> Weekday {
    if sunday {
        Weekday::Sun
    } else {
        Weekday::Mon
    }
}

/// Days from `today` to the end of this week (Sunday for a Monday start, Saturday for a Sunday start).
pub fn end_of_week(today: NaiveDate, week_start: Weekday) -> NaiveDate {
    let off = (7 + today.weekday().num_days_from_monday() as i64 - week_start.num_days_from_monday() as i64) % 7;
    today + Duration::days(6 - off)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::{RecKind, Recurrence};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn now() -> NaiveDateTime {
        d(2026, 11, 17).and_hms_opt(9, 0, 0).unwrap()
    }
    fn task(profile: Profile, title: &str) -> Task {
        Task::new(profile, title, now())
    }

    #[test]
    fn grid_monday_start_november_2026() {
        let g = month_grid(2026, 11, Weekday::Mon);
        assert_eq!(g.len(), 42);
        assert_eq!(g[0], d(2026, 10, 26)); // 1 Nov 2026 is a Sunday
        assert_eq!(g[6], d(2026, 11, 1));
        assert_eq!(g[41], d(2026, 12, 6));
    }

    #[test]
    fn grid_sunday_start() {
        let g = month_grid(2026, 11, Weekday::Sun);
        assert_eq!(g[0], d(2026, 11, 1));
        assert_eq!(g[0].weekday(), Weekday::Sun);
    }

    #[test]
    fn grid_across_year_boundary() {
        let g = month_grid(2027, 1, Weekday::Mon); // 1 Jan 2027 is a Friday
        assert_eq!(g[0], d(2026, 12, 28));
        assert_eq!(g[4], d(2027, 1, 1));
        assert_eq!(shift_month(2026, 12, 1), (2027, 1));
        assert_eq!(shift_month(2027, 1, -1), (2026, 12));
        assert_eq!(shift_month(2026, 1, -13), (2024, 12));
    }

    #[test]
    fn grid_leap_february() {
        let g = month_grid(2028, 2, Weekday::Mon);
        assert!(g.contains(&d(2028, 2, 29)));
        assert!(g.contains(&d(2028, 3, 1)));
        assert_eq!(g[0], d(2028, 1, 31)); // 1 Feb 2028 is a Tuesday
        let g = month_grid(2026, 2, Weekday::Mon);
        assert!(!g.iter().any(|x| x.month() == 2 && x.day() == 29));
    }

    #[test]
    fn items_scheduled_and_deadline() {
        let mut a = task(Profile::Personal, "a");
        a.scheduled = Some(d(2026, 11, 20));
        a.time = NaiveTime::from_hms_opt(14, 0, 0);
        a.deadline = Some(d(2026, 11, 25));
        let m = items_in_range(&[a.clone()], d(2026, 11, 1), d(2026, 11, 30), ProfileFilter::All, false, d(2026, 11, 17));
        assert_eq!(m[&d(2026, 11, 20)][0].kind, ItemKind::Scheduled);
        assert_eq!(m[&d(2026, 11, 25)][0].kind, ItemKind::Deadline);
        assert_eq!(m.len(), 2);
        let m = items_in_range(&[a], d(2026, 12, 1), d(2026, 12, 31), ProfileFilter::All, false, d(2026, 11, 17));
        assert!(m.is_empty());
    }

    #[test]
    fn items_profile_filter_and_hide_done() {
        let mut a = task(Profile::Personal, "a");
        a.scheduled = Some(d(2026, 11, 20));
        let mut b = task(Profile::Work, "b");
        b.scheduled = Some(d(2026, 11, 20));
        b.status = Some(Status::Done);
        let all = [a, b];
        let q = |f, hide| items_in_range(&all, d(2026, 11, 1), d(2026, 11, 30), f, hide, d(2026, 11, 17));
        assert_eq!(q(ProfileFilter::All, false)[&d(2026, 11, 20)].len(), 2);
        assert_eq!(q(ProfileFilter::Personal, false)[&d(2026, 11, 20)].len(), 1);
        assert_eq!(q(ProfileFilter::Work, false)[&d(2026, 11, 20)].len(), 1);
        assert_eq!(q(ProfileFilter::All, true)[&d(2026, 11, 20)].len(), 1);
        assert!(q(ProfileFilter::Work, true).is_empty());
    }

    #[test]
    fn items_sorted_timed_first_deadlines_last() {
        let mut a = task(Profile::Work, "deadline");
        a.deadline = Some(d(2026, 11, 20));
        let mut b = task(Profile::Work, "untimed");
        b.scheduled = Some(d(2026, 11, 20));
        let mut c = task(Profile::Work, "timed");
        c.scheduled = Some(d(2026, 11, 20));
        c.time = NaiveTime::from_hms_opt(8, 0, 0);
        let m = items_in_range(&[a.clone(), b.clone(), c.clone()], d(2026, 11, 20), d(2026, 11, 20), ProfileFilter::All, false, d(2026, 11, 1));
        let ids: Vec<_> = m[&d(2026, 11, 20)].iter().map(|i| i.task_id.clone()).collect();
        assert_eq!(ids, vec![c.id, b.id, a.id]);
    }

    #[test]
    fn recurring_items_and_per_occurrence_done() {
        let mut t = task(Profile::Personal, "gym");
        t.scheduled = Some(d(2026, 11, 2));
        t.recurrence = Some(Recurrence { kind: RecKind::Weekly, interval: 1, until: None });
        t.done_dates.push(d(2026, 11, 9));
        let m = items_in_range(&[t.clone()], d(2026, 11, 1), d(2026, 11, 30), ProfileFilter::All, false, d(2026, 11, 17));
        assert_eq!(m.len(), 5);
        assert!(m[&d(2026, 11, 9)][0].done);
        assert!(!m[&d(2026, 11, 16)][0].done);
        // past occurrences of a series are never "overdue"
        assert!(m.values().flatten().all(|i| !i.overdue));
        // toggling one occurrence does not close the series
        let mut t2 = t.clone();
        toggle_done(&mut t2, Some(d(2026, 11, 16)), now());
        assert!(t2.status.is_none());
        assert!(t2.done_dates.contains(&d(2026, 11, 16)));
        assert_eq!(current_occurrence(&t2, d(2026, 11, 16)), Some(d(2026, 11, 23)));
    }

    #[test]
    fn recurring_deadline_keeps_offset() {
        let mut t = task(Profile::Work, "report");
        t.scheduled = Some(d(2026, 11, 2));
        t.deadline = Some(d(2026, 11, 4));
        t.recurrence = Some(Recurrence { kind: RecKind::Weekly, interval: 1, until: None });
        let m = items_in_range(&[t], d(2026, 11, 8), d(2026, 11, 14), ProfileFilter::All, false, d(2026, 11, 1));
        assert_eq!(m[&d(2026, 11, 9)][0].kind, ItemKind::Scheduled);
        assert_eq!(m[&d(2026, 11, 11)][0].kind, ItemKind::Deadline);
    }

    #[test]
    fn overdue_and_today_logic() {
        let today = d(2026, 11, 17);
        let mut a = task(Profile::Personal, "late");
        a.deadline = Some(d(2026, 11, 16));
        assert!(is_overdue(&a, today));
        a.deadline = Some(today);
        assert!(!is_overdue(&a, today)); // due today is not overdue
        a.deadline = Some(d(2026, 11, 16));
        a.status = Some(Status::Done);
        assert!(!is_overdue(&a, today));

        let mut b = task(Profile::Work, "planned yesterday");
        b.scheduled = Some(d(2026, 11, 16));
        let mut c = task(Profile::Work, "future");
        c.deadline = Some(d(2026, 11, 20));
        let list = [a, b.clone(), c];
        let od = overdue_items(&list, today, ProfileFilter::All);
        assert_eq!(od.len(), 1);
        assert_eq!(od[0].task_id, b.id);
        assert!(overdue_items(&list, today, ProfileFilter::Personal).is_empty());
    }

    #[test]
    fn drag_moves_scheduled_or_deadline() {
        let mut t = task(Profile::Work, "x");
        t.scheduled = Some(d(2026, 11, 10));
        t.deadline = Some(d(2026, 11, 12));
        move_item(&mut t, ItemKind::Deadline, d(2026, 11, 12), d(2026, 11, 15), now());
        assert_eq!(t.deadline, Some(d(2026, 11, 15)));
        assert_eq!(t.scheduled, Some(d(2026, 11, 10)));
        move_item(&mut t, ItemKind::Scheduled, d(2026, 11, 10), d(2026, 11, 8), now());
        assert_eq!(t.scheduled, Some(d(2026, 11, 8)));
    }

    #[test]
    fn week_end() {
        assert_eq!(end_of_week(d(2026, 11, 17), Weekday::Mon), d(2026, 11, 22));
        assert_eq!(end_of_week(d(2026, 11, 22), Weekday::Mon), d(2026, 11, 22));
        assert_eq!(end_of_week(d(2026, 11, 17), Weekday::Sun), d(2026, 11, 21));
    }
}
