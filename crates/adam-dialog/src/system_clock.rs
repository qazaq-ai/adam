// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Live OS-clock readout, Kazakh-formatted.
//!
//! Until v6.0 every `Intent::AskTime` turn returned «менде сағат жоқ»
//! regardless of what the user asked. The 2026-05-18 live REPL flagged
//! that adam should instead read the laptop's wall clock and answer
//! literally. This module is the one place we touch [`std::time`].
//!
//! ## Why we hand-roll Kazakh date / weekday math instead of pulling
//! `chrono`
//!
//! The whole calendar surface adam needs is **six format functions**
//! over a single `SystemTime`. `chrono` is a ~3 MB crate with a tz
//! database and locale machinery we wouldn't use; the Kazakh
//! day-name / month-name lists below are 7 + 12 = 19 short strings.
//! Pure-Rust kernel directive ⇒ keep dependencies surgical.
//!
//! All calculations are in **UTC** read from the OS via
//! `SystemTime::now()`. Local-zone offset is read from the
//! `ADAM_TZ_OFFSET_HOURS` env var (default 0). On a laptop in
//! Almaty / Astana the user would set `export ADAM_TZ_OFFSET_HOURS=5`
//! once and the dialog turns then report local time. Future work
//! (v6.x): read the OS time-zone properly via `libc::localtime_r`
//! on Unix; for the first cut, an env var keeps the kernel
//! deterministic and the patch surface small.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::intent::TimeAspect;

/// Kazakh weekday names indexed by ISO-8601 weekday (1 = Mon … 7 = Sun).
/// Index 0 is unused so the lookup stays one-based and trivial.
const WEEKDAYS_KK: [&str; 8] = [
    "",
    "Дүйсенбі",
    "Сейсенбі",
    "Сәрсенбі",
    "Бейсенбі",
    "Жұма",
    "Сенбі",
    "Жексенбі",
];

/// Kazakh month names indexed 1..=12. Index 0 unused.
const MONTHS_KK: [&str; 13] = [
    "",
    "қаңтар",
    "ақпан",
    "наурыз",
    "сәуір",
    "мамыр",
    "маусым",
    "шілде",
    "тамыз",
    "қыркүйек",
    "қазан",
    "қараша",
    "желтоқсан",
];

/// Snapshot of the laptop wall clock at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockReading {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub weekday: u32, // 1 = Mon … 7 = Sun (ISO-8601)
    pub hour: u32,
    pub minute: u32,
}

/// Read the wall clock once. `tz_offset_secs` is added to UTC before
/// breaking down into year/month/day/hour/minute, so callers in a
/// non-UTC zone get the local-zone date / hour.
pub fn read_clock(tz_offset_secs: i64) -> ClockReading {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let local = now + tz_offset_secs;
    // Time-of-day components.
    let secs_of_day = local.rem_euclid(86_400);
    let hour = (secs_of_day / 3_600) as u32;
    let minute = ((secs_of_day % 3_600) / 60) as u32;
    // Calendar components via Howard Hinnant's `civil_from_days` —
    // proleptic Gregorian, valid for all sensible epoch offsets.
    let days = local.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    // ISO-8601 weekday: 1970-01-01 (epoch day 0) was a Thursday → 4.
    let weekday = ((days.rem_euclid(7) + 3).rem_euclid(7) + 1) as u32;
    ClockReading {
        year,
        month,
        day,
        weekday,
        hour,
        minute,
    }
}

/// Resolve the timezone-offset env var into seconds. Accepts integer
/// or decimal hours, e.g. `5`, `5.5`, `-3`. Defaults to 0 (UTC) when
/// unset or malformed — the answer stays consistent rather than
/// silently producing garbage on a typo.
pub fn tz_offset_secs_from_env() -> i64 {
    let raw = std::env::var("ADAM_TZ_OFFSET_HOURS").unwrap_or_default();
    let hours: f64 = raw.parse().unwrap_or(0.0);
    (hours * 3_600.0) as i64
}

/// Render a Kazakh-language answer for the requested clock aspect.
/// Caller supplies the snapshot; this function is pure so it
/// round-trips in tests without touching the system clock.
pub fn format_kk(c: ClockReading, aspect: TimeAspect) -> String {
    match aspect {
        TimeAspect::Time => format!("Қазір сағат {:02}:{:02}.", c.hour, c.minute),
        TimeAspect::Date => format!(
            "Бүгін — {day} {month} {year} жыл, {weekday}.",
            day = c.day,
            month = MONTHS_KK[c.month as usize],
            year = c.year,
            weekday = WEEKDAYS_KK[c.weekday as usize],
        ),
        TimeAspect::Weekday => format!("Бүгін — {}.", WEEKDAYS_KK[c.weekday as usize]),
        TimeAspect::Month => format!("Қазір {} айы.", MONTHS_KK[c.month as usize]),
        TimeAspect::Year => format!("Қазір {} жыл.", c.year),
        TimeAspect::DateTime => format!(
            "Бүгін {day} {month} {year} жыл, {weekday}; қазір сағат {h:02}:{m:02}.",
            day = c.day,
            month = MONTHS_KK[c.month as usize],
            year = c.year,
            weekday = WEEKDAYS_KK[c.weekday as usize],
            h = c.hour,
            m = c.minute,
        ),
    }
}

/// Convenience wrapper: read clock + format in one call. Used by the
/// planner when it has just an aspect to render.
pub fn render_live(aspect: TimeAspect) -> String {
    let c = read_clock(tz_offset_secs_from_env());
    format_kk(c, aspect)
}

/// Convert a Unix-epoch day count into a (year, month, day) tuple
/// using the proleptic Gregorian calendar. Adapted from Howard
/// Hinnant's `civil_from_days` — public domain.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468; // shift so 0000-03-01 is day 0
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 {
        (mp + 3) as u32
    } else {
        (mp - 9) as u32
    };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading_at(
        year: i32,
        month: u32,
        day: u32,
        weekday: u32,
        hour: u32,
        minute: u32,
    ) -> ClockReading {
        ClockReading {
            year,
            month,
            day,
            weekday,
            hour,
            minute,
        }
    }

    #[test]
    fn format_time_at_5_minutes_past_midnight() {
        let c = reading_at(2026, 5, 18, 1, 0, 5);
        assert_eq!(format_kk(c, TimeAspect::Time), "Қазір сағат 00:05.");
    }

    #[test]
    fn format_date_renders_kazakh_month_and_weekday() {
        // 2026-05-18 is a Monday → weekday=1 → Дүйсенбі.
        let c = reading_at(2026, 5, 18, 1, 14, 30);
        assert_eq!(
            format_kk(c, TimeAspect::Date),
            "Бүгін — 18 мамыр 2026 жыл, Дүйсенбі."
        );
    }

    #[test]
    fn format_weekday_alone() {
        let c = reading_at(2026, 5, 18, 1, 0, 0);
        assert_eq!(format_kk(c, TimeAspect::Weekday), "Бүгін — Дүйсенбі.");
    }

    #[test]
    fn format_month_uses_kazakh_name() {
        let c = reading_at(2026, 1, 1, 4, 0, 0);
        assert_eq!(format_kk(c, TimeAspect::Month), "Қазір қаңтар айы.");
    }

    #[test]
    fn format_year_is_bare_integer() {
        let c = reading_at(2026, 5, 18, 1, 0, 0);
        assert_eq!(format_kk(c, TimeAspect::Year), "Қазір 2026 жыл.");
    }

    #[test]
    fn format_datetime_combines_all_components() {
        let c = reading_at(2026, 5, 18, 1, 9, 7);
        assert_eq!(
            format_kk(c, TimeAspect::DateTime),
            "Бүгін 18 мамыр 2026 жыл, Дүйсенбі; қазір сағат 09:07."
        );
    }

    #[test]
    fn civil_from_days_round_trips_known_dates() {
        // 2026-05-18 = epoch day 20591.
        assert_eq!(civil_from_days(20591), (2026, 5, 18));
        // 1970-01-01 = epoch day 0.
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 2000-02-29 (leap year) = epoch day 11016.
        assert_eq!(civil_from_days(11016), (2000, 2, 29));
    }

    #[test]
    fn read_clock_honours_positive_tz_offset() {
        // Two readings 5h apart should differ by exactly 5 hours in
        // wall-clock fields. We can't pin a specific time without
        // freezing the OS clock, so we assert the *difference*.
        let utc = read_clock(0);
        let almaty = read_clock(5 * 3600);
        // Hour difference is +5 modulo 24 — calendar may roll
        // forward by 1 day at the boundary.
        let hours_diff = (almaty.hour as i32 - utc.hour as i32).rem_euclid(24);
        assert_eq!(hours_diff, 5);
    }

    #[test]
    fn tz_offset_env_parses_integer_and_decimal() {
        // SAFETY: env mutation in tests is process-wide; this test
        // restores the previous value to avoid leaking across tests.
        let prev = std::env::var("ADAM_TZ_OFFSET_HOURS").ok();
        unsafe {
            std::env::set_var("ADAM_TZ_OFFSET_HOURS", "5");
        }
        assert_eq!(tz_offset_secs_from_env(), 5 * 3600);
        unsafe {
            std::env::set_var("ADAM_TZ_OFFSET_HOURS", "5.5");
        }
        assert_eq!(tz_offset_secs_from_env(), (5.5 * 3600.0) as i64);
        unsafe {
            std::env::set_var("ADAM_TZ_OFFSET_HOURS", "garbage");
        }
        assert_eq!(tz_offset_secs_from_env(), 0);
        match prev {
            Some(v) => unsafe { std::env::set_var("ADAM_TZ_OFFSET_HOURS", v) },
            None => unsafe { std::env::remove_var("ADAM_TZ_OFFSET_HOURS") },
        }
    }
}
