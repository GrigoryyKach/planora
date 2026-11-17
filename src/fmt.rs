//! Localised date and time text. Names come from the app's own tables, not from the OS.

use crate::i18n::Lang;
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Weekday};

const MONTHS_EN: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MONTHS_RU: [&str; 12] =
    ["Январь", "Февраль", "Март", "Апрель", "Май", "Июнь", "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь"];
const MONTHS_UK: [&str; 12] =
    ["Січень", "Лютий", "Березень", "Квітень", "Травень", "Червень", "Липень", "Серпень", "Вересень", "Жовтень", "Листопад", "Грудень"];
const MONTHS_RU_GEN: [&str; 12] =
    ["января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября", "декабря"];
const MONTHS_UK_GEN: [&str; 12] =
    ["січня", "лютого", "березня", "квітня", "травня", "червня", "липня", "серпня", "вересня", "жовтня", "листопада", "грудня"];
const MONTHS_SHORT_EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_SHORT_RU: [&str; 12] = ["янв", "фев", "мар", "апр", "мая", "июн", "июл", "авг", "сен", "окт", "ноя", "дек"];
const MONTHS_SHORT_UK: [&str; 12] = ["січ", "лют", "бер", "кві", "тра", "чер", "лип", "сер", "вер", "жов", "лис", "гру"];

const DAYS_EN: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const DAYS_RU: [&str; 7] = ["понедельник", "вторник", "среда", "четверг", "пятница", "суббота", "воскресенье"];
const DAYS_UK: [&str; 7] = ["понеділок", "вівторок", "середа", "четвер", "пʼятниця", "субота", "неділя"];
const DAYS_SHORT_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const DAYS_SHORT_RU: [&str; 7] = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"];
const DAYS_SHORT_UK: [&str; 7] = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"];

fn pick<'a>(lang: Lang, en: &'a [&'a str], ru: &'a [&'a str], uk: &'a [&'a str], i: usize) -> &'a str {
    match lang {
        Lang::En => en[i],
        Lang::Ru => ru[i],
        Lang::Uk => uk[i],
    }
}

/// "November", "Ноябрь", "Листопад".
pub fn month_name(lang: Lang, month: u32) -> &'static str {
    pick(lang, &MONTHS_EN, &MONTHS_RU, &MONTHS_UK, (month as usize + 11) % 12)
}

pub fn month_title(lang: Lang, year: i32, month: u32) -> String {
    format!("{} {}", month_name(lang, month), year)
}

pub fn weekday_short(lang: Lang, w: Weekday) -> &'static str {
    pick(lang, &DAYS_SHORT_EN, &DAYS_SHORT_RU, &DAYS_SHORT_UK, w.num_days_from_monday() as usize)
}

pub fn weekday_long(lang: Lang, w: Weekday) -> &'static str {
    pick(lang, &DAYS_EN, &DAYS_RU, &DAYS_UK, w.num_days_from_monday() as usize)
}

/// "Nov 17" / "17 ноя" / "17 лис".
pub fn date_short(lang: Lang, d: NaiveDate) -> String {
    let i = d.month0() as usize;
    match lang {
        Lang::En => format!("{} {}", MONTHS_SHORT_EN[i], d.day()),
        Lang::Ru => format!("{} {}", d.day(), MONTHS_SHORT_RU[i]),
        Lang::Uk => format!("{} {}", d.day(), MONTHS_SHORT_UK[i]),
    }
}

/// "Nov 17, 2026" / "17 ноя 2026".
pub fn date_full(lang: Lang, d: NaiveDate) -> String {
    format!("{} {}", date_short(lang, d), d.year())
}

/// "Tuesday, November 17" / "вторник, 17 ноября".
pub fn date_long(lang: Lang, d: NaiveDate) -> String {
    let i = d.month0() as usize;
    let wd = weekday_long(lang, d.weekday());
    match lang {
        Lang::En => format!("{wd}, {} {}", MONTHS_EN[i], d.day()),
        Lang::Ru => format!("{wd}, {} {}", d.day(), MONTHS_RU_GEN[i]),
        Lang::Uk => format!("{wd}, {} {}", d.day(), MONTHS_UK_GEN[i]),
    }
}

/// `14:30`, or `2:30 PM` for the 12-hour clock.
pub fn time_text(t: NaiveTime, h24: bool) -> String {
    if h24 {
        format!("{:02}:{:02}", t.hour(), t.minute())
    } else {
        let (h12, suffix) = match t.hour() {
            0 => (12, "AM"),
            h @ 1..=11 => (h, "AM"),
            12 => (12, "PM"),
            h => (h - 12, "PM"),
        };
        format!("{}:{:02} {}", h12, t.minute(), suffix)
    }
}

/// Accepts `14:30`, `9:05`, `930`, `2:30 pm`, `2pm`.
pub fn parse_time(s: &str) -> Option<NaiveTime> {
    let s = s.trim().to_lowercase();
    let (body, pm) = if let Some(b) = s.strip_suffix("pm") {
        (b.trim(), Some(true))
    } else if let Some(b) = s.strip_suffix("am") {
        (b.trim(), Some(false))
    } else {
        (s.as_str(), None)
    };
    let (h, m) = if let Some((h, m)) = body.split_once([':', '.']) {
        (h.trim().parse::<u32>().ok()?, m.trim().parse::<u32>().ok()?)
    } else if body.len() > 2 && body.chars().all(|c| c.is_ascii_digit()) {
        let n: u32 = body.parse().ok()?;
        (n / 100, n % 100)
    } else {
        (body.parse::<u32>().ok()?, 0)
    };
    let h = match pm {
        Some(true) if h < 12 => h + 12,
        Some(false) if h == 12 => 0,
        _ => h,
    };
    NaiveTime::from_hms_opt(h, m, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_format_and_parse() {
        let t = NaiveTime::from_hms_opt(14, 5, 0).unwrap();
        assert_eq!(time_text(t, true), "14:05");
        assert_eq!(time_text(t, false), "2:05 PM");
        assert_eq!(time_text(NaiveTime::from_hms_opt(0, 0, 0).unwrap(), false), "12:00 AM");
        assert_eq!(parse_time("14:05"), Some(t));
        assert_eq!(parse_time("2:05 pm"), Some(t));
        assert_eq!(parse_time("1405"), Some(t));
        assert_eq!(parse_time("12am"), NaiveTime::from_hms_opt(0, 0, 0));
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(parse_time("abc"), None);
    }

    #[test]
    fn names() {
        assert_eq!(month_title(Lang::Uk, 2026, 11), "Листопад 2026");
        let d = NaiveDate::from_ymd_opt(2026, 11, 17).unwrap();
        assert_eq!(date_long(Lang::Ru, d), "вторник, 17 ноября");
        assert_eq!(date_short(Lang::En, d), "Nov 17");
    }
}
