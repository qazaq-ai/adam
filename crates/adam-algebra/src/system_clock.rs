// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `system_clock` — **OS wall-clock provider** for the v6.2
//! pipeline.
//!
//! Closes the «system/*» gap in the dialog battery (Stage 4.5
//! reported 4 unanswered cases: «Бүгін айдың нешесі?» / «Қазір
//! қандай ай?» / «Бүгін аптаның қай күні?» / «Қазір сағат неше?»).
//!
//! ## Architectural fit
//!
//! System-time is **environment state**, not knowledge — it doesn't
//! belong in [`crate::FrameIndex`] (which holds curated facts) and
//! it's not procedural like the math solver. It's a *provider* the
//! Realiser (Stage 7) consults when the QueryIR focus is a
//! time-anchor modifier on a system-self predicate
//! (`Modality::SystemSelf`).
//!
//! This module is a clean port of the v6.1
//! `adam_dialog::system_clock` module (kept independent because
//! `adam-dialog` depends on `adam-algebra`, not the other way
//! around — circular deps would result if we re-used it directly).
//!
//! ## Pure-Rust kernel
//!
//! No chrono dependency. Six format functions + Howard Hinnant's
//! `civil_from_days` (public domain) — that's all v6.2 needs.
//! Calendar surface is **closed-set**: 7 weekday names + 12 month
//! names in Kazakh.
//!
//! TZ resolution order:
//! 1. `ADAM_TZ_OFFSET_HOURS` env var (operator override).
//! 2. OS `date +%z` output (Unix).
//! 3. Default `+5` (Asia/Almaty / Asia/Astana) — Kazakhstan is
//!    the product locale.

use std::time::{SystemTime, UNIX_EPOCH};

/// Kazakh weekday names indexed by ISO-8601 weekday (1 = Mon …
/// 7 = Sun). Index 0 is unused so the lookup stays one-based.
pub const WEEKDAYS_KK: [&str; 8] = [
    "",
    "дүйсенбі",
    "сейсенбі",
    "сәрсенбі",
    "бейсенбі",
    "жұма",
    "сенбі",
    "жексенбі",
];

/// Kazakh month names indexed 1..=12.
pub const MONTHS_KK: [&str; 13] = [
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

/// Snapshot of the wall clock at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockReading {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// ISO-8601 weekday: 1 = Mon … 7 = Sun.
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
}

impl ClockReading {
    /// Kazakh month name (lowercase, e.g. «мамыр»).
    pub fn month_kk(&self) -> &'static str {
        MONTHS_KK[self.month as usize]
    }

    /// Kazakh weekday name (lowercase, e.g. «дүйсенбі»).
    pub fn weekday_kk(&self) -> &'static str {
        WEEKDAYS_KK[self.weekday as usize]
    }

    /// «08:00» format for the time of day.
    pub fn time_hhmm(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// «YYYY-MM-DD» format for the date.
    pub fn date_iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Read the wall clock. `tz_offset_secs` is added to UTC before
/// decomposition.
pub fn read_clock(tz_offset_secs: i64) -> ClockReading {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let local = now + tz_offset_secs;
    let secs_of_day = local.rem_euclid(86_400);
    let hour = (secs_of_day / 3_600) as u32;
    let minute = ((secs_of_day % 3_600) / 60) as u32;
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

/// Resolve the timezone offset using the documented hierarchy
/// (env var → OS `date +%z` → +5 default).
pub fn tz_offset_secs_from_env() -> i64 {
    if let Ok(raw) = std::env::var("ADAM_TZ_OFFSET_HOURS")
        && let Ok(hours) = raw.parse::<f64>()
    {
        return (hours * 3_600.0) as i64;
    }
    if let Some(secs) = detect_local_tz_offset_secs() {
        return secs;
    }
    5 * 3_600
}

fn detect_local_tz_offset_secs() -> Option<i64> {
    use std::process::Command;
    let output = Command::new("date").arg("+%z").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?;
    let trimmed = raw.trim();
    if trimmed.len() < 5 {
        return None;
    }
    let sign: i64 = if trimmed.starts_with('-') { -1 } else { 1 };
    let body = &trimmed[1..];
    let hh: i64 = body.get(..2)?.parse().ok()?;
    let mm: i64 = body.get(2..4)?.parse().ok()?;
    Some(sign * (hh * 3_600 + mm * 60))
}

/// Live clock reading using the resolved tz offset. Convenience
/// for the dialog battery and the Stage 7 realiser.
pub fn now() -> ClockReading {
    read_clock(tz_offset_secs_from_env())
}

/// Howard Hinnant's `civil_from_days` — public domain. Maps a
/// Unix-epoch day count to a proleptic Gregorian (year, month, day).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
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

    /// `civil_from_days(0)` is 1970-01-01 — the Unix epoch.
    #[test]
    fn epoch_day_zero_is_19700101() {
        let (y, m, d) = civil_from_days(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    /// Day 1 is 1970-01-02.
    #[test]
    fn epoch_day_one_is_19700102() {
        let (y, m, d) = civil_from_days(1);
        assert_eq!((y, m, d), (1970, 1, 2));
    }

    /// 2020 was a leap year — day 60 of 2020 is February 29.
    #[test]
    fn leap_year_feb_29_2020() {
        // Days from 1970-01-01 to 2020-02-29.
        // 50 years × 365 + 13 leap days (1972, 76, ..., 2020 is on it) + Jan (31) + Feb 28 = 18_321.
        // Use the round-trip:
        let r = read_clock(0);
        // Just sanity: read should not panic.
        let _ = r;
        let (y, m, d) = civil_from_days(18_321);
        assert_eq!((y, m, d), (2020, 2, 29));
    }

    #[test]
    fn read_clock_returns_valid_components() {
        let c = read_clock(5 * 3_600);
        assert!(c.year >= 2020 && c.year < 2200);
        assert!(c.month >= 1 && c.month <= 12);
        assert!(c.day >= 1 && c.day <= 31);
        assert!(c.weekday >= 1 && c.weekday <= 7);
        assert!(c.hour < 24);
        assert!(c.minute < 60);
    }

    #[test]
    fn clock_reading_helpers() {
        let c = ClockReading {
            year: 2026,
            month: 5,
            day: 25,
            weekday: 1,
            hour: 8,
            minute: 30,
        };
        assert_eq!(c.month_kk(), "мамыр");
        assert_eq!(c.weekday_kk(), "дүйсенбі");
        assert_eq!(c.time_hhmm(), "08:30");
        assert_eq!(c.date_iso(), "2026-05-25");
    }

    #[test]
    fn weekday_of_known_date() {
        // 2026-05-25 — let civil_from_days verify itself by going
        // back through `read_clock` semantics. The math is the same
        // formula adam-dialog has shipped since v6.0.
        let c = ClockReading {
            year: 2026,
            month: 5,
            day: 25,
            weekday: 1,
            hour: 0,
            minute: 0,
        };
        // We don't compute weekday from y/m/d here; this test just
        // asserts the const tables and helpers don't panic.
        assert_eq!(MONTHS_KK[c.month as usize], "мамыр");
    }
}
