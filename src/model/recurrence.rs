//! Expansion of recurring tasks into concrete dates.

use super::types::{RecKind, Recurrence};
use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Last day of the month `y-m`.
pub fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1).and_then(|d| d.pred_opt()).map(|d| d.day()).unwrap_or(28)
}

fn is_weekday(d: NaiveDate) -> bool {
    !matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

/// All occurrence dates of a series that starts at `anchor`, within `from..=to` (inclusive).
/// Monthly series on the 29th-31st are clamped to the last day of shorter months
/// (and return to the original day in longer months).
pub fn occurrences(anchor: NaiveDate, rec: &Recurrence, from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let to = rec.until.map_or(to, |u| u.min(to));
    let from = from.max(anchor);
    if from > to {
        return out;
    }
    match rec.kind {
        RecKind::Daily => {
            let mut d = from;
            while d <= to {
                out.push(d);
                d += Duration::days(1);
            }
        }
        RecKind::Weekdays => {
            let mut d = from;
            while d <= to {
                if is_weekday(d) {
                    out.push(d);
                }
                d += Duration::days(1);
            }
        }
        RecKind::Weekly => {
            let mut d = from;
            while d <= to {
                if d.weekday() == anchor.weekday() {
                    out.push(d);
                }
                d += Duration::days(1);
            }
        }
        RecKind::Days => {
            let step = rec.interval.max(1) as i64;
            let gap = (from - anchor).num_days();
            let first = anchor + Duration::days((gap + step - 1) / step * step);
            let mut d = first;
            while d <= to {
                out.push(d);
                d += Duration::days(step);
            }
        }
        RecKind::Monthly => {
            let (mut y, mut m) = (from.year(), from.month());
            loop {
                let day = anchor.day().min(days_in_month(y, m));
                if let Some(d) = NaiveDate::from_ymd_opt(y, m, day) {
                    if d > to {
                        break;
                    }
                    if d >= from {
                        out.push(d);
                    }
                }
                if m == 12 {
                    y += 1;
                    m = 1;
                } else {
                    m += 1;
                }
                if y > to.year() + 1 {
                    break;
                }
            }
        }
    }
    out
}

/// Does `d` belong to the series?
pub fn occurs_on(anchor: NaiveDate, rec: &Recurrence, d: NaiveDate) -> bool {
    !occurrences(anchor, rec, d, d).is_empty()
}

/// First occurrence on or after `from` (searches two years ahead).
pub fn next_occurrence(anchor: NaiveDate, rec: &Recurrence, from: NaiveDate) -> Option<NaiveDate> {
    occurrences(anchor, rec, from, from + Duration::days(800)).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn rec(kind: RecKind, interval: u32, until: Option<NaiveDate>) -> Recurrence {
        Recurrence { kind, interval, until }
    }

    #[test]
    fn daily_in_range() {
        let r = occurrences(d(2026, 11, 10), &rec(RecKind::Daily, 1, None), d(2026, 11, 8), d(2026, 11, 13));
        assert_eq!(r, vec![d(2026, 11, 10), d(2026, 11, 11), d(2026, 11, 12), d(2026, 11, 13)]);
    }

    #[test]
    fn weekdays_skip_weekend() {
        // 2026-11-13 is a Friday.
        let r = occurrences(d(2026, 11, 13), &rec(RecKind::Weekdays, 1, None), d(2026, 11, 13), d(2026, 11, 18));
        assert_eq!(r, vec![d(2026, 11, 13), d(2026, 11, 16), d(2026, 11, 17), d(2026, 11, 18)]);
    }

    #[test]
    fn weekly_same_weekday() {
        let r = occurrences(d(2026, 11, 2), &rec(RecKind::Weekly, 1, None), d(2026, 11, 1), d(2026, 11, 30));
        assert_eq!(r, vec![d(2026, 11, 2), d(2026, 11, 9), d(2026, 11, 16), d(2026, 11, 23), d(2026, 11, 30)]);
    }

    #[test]
    fn monthly_on_31st_clamps() {
        let r = occurrences(d(2026, 1, 31), &rec(RecKind::Monthly, 1, None), d(2026, 1, 1), d(2026, 5, 31));
        assert_eq!(r, vec![d(2026, 1, 31), d(2026, 2, 28), d(2026, 3, 31), d(2026, 4, 30), d(2026, 5, 31)]);
        // Leap year February.
        let r = occurrences(d(2028, 1, 31), &rec(RecKind::Monthly, 1, None), d(2028, 2, 1), d(2028, 2, 29));
        assert_eq!(r, vec![d(2028, 2, 29)]);
    }

    #[test]
    fn every_n_days_with_end() {
        let r = occurrences(d(2026, 11, 1), &rec(RecKind::Days, 3, Some(d(2026, 11, 12))), d(2026, 11, 5), d(2026, 12, 31));
        assert_eq!(r, vec![d(2026, 11, 7), d(2026, 11, 10)]);
    }

    #[test]
    fn nothing_before_anchor_or_after_until() {
        let rc = rec(RecKind::Daily, 1, Some(d(2026, 11, 3)));
        assert!(occurrences(d(2026, 11, 5), &rc, d(2026, 11, 1), d(2026, 11, 30)).is_empty());
        assert!(occurrences(d(2026, 11, 1), &rc, d(2026, 11, 4), d(2026, 11, 30)).is_empty());
    }

    #[test]
    fn next_occurrence_found() {
        let rc = rec(RecKind::Weekly, 1, None);
        assert_eq!(next_occurrence(d(2026, 11, 2), &rc, d(2026, 11, 3)), Some(d(2026, 11, 9)));
    }
}
