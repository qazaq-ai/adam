// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Live weather readout via Open-Meteo, Kazakh-formatted.
//!
//! Until v6.0 every `Intent::AskWeather` turn returned «менде терезе
//! жоқ». The 2026-05-18 user request asked adam to consult the
//! laptop's actual weather. macOS Weather.app has no public CLI /
//! IPC, but **Open-Meteo** (https://open-meteo.com) exposes a free,
//! no-API-key, no-registration HTTP endpoint. This module shells out
//! to `curl`, parses the JSON reply, and emits a Kazakh-language
//! one-line summary.
//!
//! ## Air-gap discipline
//!
//! The whole feature is **opt-in**. Adam's air-gap-deployable USP
//! requires that the kernel does not phone home unless the operator
//! deliberately enables it. The gate is the presence of a location:
//!
//! 1. `ADAM_WEATHER_LAT` + `ADAM_WEATHER_LON` env vars → exact
//!    coordinates;
//! 2. `ADAM_WEATHER_CITY` env var → looked up in the Kazakh city
//!    table below;
//! 3. `session["city"]` (the user previously said «мен Алматыда
//!    тұрамын» / «Қостанайдан» / etc.) → looked up in the city table;
//! 4. None of the above → return `None`, planner falls through to
//!    the existing «менде терезе жоқ» refusal.
//!
//! No IP-geolocation, no cloud call without explicit configuration.
//!
//! ## Shell-out choice
//!
//! We deliberately avoid adding `ureq` / `reqwest`. Adam already
//! shells out to whisper-cli + macOS `say`; one more `curl` keeps
//! the dependency tree flat and matches the project's pragmatic
//! «pure-Rust kernel + thin external transducers» pattern. macOS
//! and Linux both ship curl pre-installed; Windows would need an
//! adapter when the project supports it (currently it doesn't).
//!
//! ## Network failure modes
//!
//! Hard timeout: 4 seconds (curl `--max-time 4`). On timeout,
//! non-zero exit, or unparseable JSON, [`fetch_now`] returns
//! `Err(WeatherError::…)`. The planner treats any error as
//! «could not fetch» and falls back to the refusal template —
//! never blocks the dialog turn waiting on the network.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Deserialize;

/// Coordinates of a Kazakh city / town. Keys are lower-case Kazakh
/// surface forms. Centroid-ish lat/lon, accurate to ~0.5 km, plenty
/// for hourly weather granularity. Closed-list approach is a feature
/// — only cities adam genuinely knows about resolve to a forecast;
/// unknown places refuse honestly rather than guess.
pub fn kazakh_city_coords() -> HashMap<String, (f64, f64)> {
    let entries: &[(&str, f64, f64)] = &[
        ("алматы", 43.2389, 76.8897),
        ("астана", 51.1801, 71.4460),
        ("нұр-сұлтан", 51.1801, 71.4460),
        ("шымкент", 42.3174, 69.5901),
        ("қарағанды", 49.8019, 73.1024),
        ("қарагандi", 49.8019, 73.1024),
        ("ақтөбе", 50.2839, 57.1670),
        ("тараз", 42.9000, 71.3667),
        ("павлодар", 52.2873, 76.9674),
        ("өскемен", 49.9714, 82.6059),
        ("семей", 50.4111, 80.2275),
        ("атырау", 47.0945, 51.9238),
        ("қостанай", 53.2144, 63.6246),
        ("қызылорда", 44.8479, 65.4823),
        ("петропавл", 54.8754, 69.1639),
        ("түркістан", 43.2978, 68.2517),
        ("талдықорған", 45.0156, 78.3737),
        ("орал", 51.2333, 51.3667),
        ("жезқазған", 47.7833, 67.7000),
        ("көкшетау", 53.2855, 69.3958),
        ("балқаш", 46.8425, 74.9711),
        ("ақтау", 43.6517, 51.1572),
        ("жаңаөзен", 43.3416, 52.8553),
        ("қапшағай", 43.8786, 77.0681),
        ("кентау", 43.5167, 68.5167),
        ("риддер", 50.3489, 83.5097),
        ("сатпаев", 47.9000, 67.5333),
        ("екібастұз", 51.7244, 75.3231),
        ("теміртау", 50.0500, 72.9667),
    ];
    entries
        .iter()
        .map(|(name, lat, lon)| ((*name).to_string(), (*lat, *lon)))
        .collect()
}

/// Parsed Open-Meteo `current` block, narrowed to what we surface.
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherReading {
    pub temperature_c: f32,
    pub feels_like_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub wind_kmh: Option<f32>,
    pub weather_code: u32,
    /// Optional human-friendly label of the place the forecast is
    /// for, used to phrase the answer («Алматыда қазір …»).
    pub city_label: Option<String>,
}

#[derive(Debug)]
pub enum WeatherError {
    NoLocation,
    CurlMissing,
    CurlFailed(String),
    JsonParse(String),
}

impl std::fmt::Display for WeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLocation => write!(f, "weather: no location resolved"),
            Self::CurlMissing => write!(f, "weather: curl binary not on PATH"),
            Self::CurlFailed(msg) => write!(f, "weather: curl failed: {msg}"),
            Self::JsonParse(msg) => write!(f, "weather: json parse: {msg}"),
        }
    }
}

impl std::error::Error for WeatherError {}

/// Resolve a location to query, walking the gate cascade. Returns
/// `None` when none of the configured sources surface a location,
/// signalling the planner to keep the refusal template.
pub fn resolve_location(session: &HashMap<String, String>) -> Option<(f64, f64, Option<String>)> {
    // 1. Explicit lat/lon env override (highest precedence — power user
    //    pinning to a specific spot, e.g. a research station).
    if let (Ok(lat_s), Ok(lon_s)) = (
        std::env::var("ADAM_WEATHER_LAT"),
        std::env::var("ADAM_WEATHER_LON"),
    ) && let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>())
    {
        let label = std::env::var("ADAM_WEATHER_CITY").ok();
        return Some((lat, lon, label));
    }
    // 2. City env var (lookup in static table).
    if let Ok(city) = std::env::var("ADAM_WEATHER_CITY")
        && let Some((lat, lon)) = lookup_city(&city)
    {
        return Some((lat, lon, Some(city)));
    }
    // 3. Session-belief city (the user previously said «мен
    //    Қостанайдан / Алматыда тұрамын», and `extract_secondary_
    //    profile_facts` / `detect_statement_of_location` recorded
    //    the city slot).
    if let Some(city) = session.get("city")
        && let Some((lat, lon)) = lookup_city(city)
    {
        return Some((lat, lon, Some(city.clone())));
    }
    None
}

/// Look up a Kazakh-language city name (any case, accusative/locative
/// suffixes tolerated by stripping the trailing inflection) against
/// the [`kazakh_city_coords`] table.
fn lookup_city(name: &str) -> Option<(f64, f64)> {
    let table = kazakh_city_coords();
    let normalised = strip_locative_suffix(&name.trim().to_lowercase());
    table.get(&normalised).copied()
}

/// Strip a trailing locative / dative / ablative / genitive suffix
/// from a Kazakh place name so «Алматыда» / «Қостанайдан» /
/// «Астанаға» / «Шымкенттің» all resolve to the bare-stem table key.
/// Conservative: only the suffixes that produce non-ambiguous strips;
/// other inflections fall back to a bare-key lookup.
fn strip_locative_suffix(name: &str) -> String {
    const SUFFIXES: &[&str] = &[
        "тағы", "тегі", "дағы", "дегі", "нағы", "негі", // locative+adjectival
        "дан", "ден", "тан", "тен", "нан", "нен", // ablative
        "ға", "ге", "қа", "ке", "на", "не", // dative
        "дың", "дің", "тың", "тің", "ның", "нің", // genitive
        "ды", "ді", "ты", "ті", "ны", "ні", // accusative
        "да", "де", "та", "те", // locative
    ];
    for suffix in SUFFIXES {
        if let Some(stripped) = name.strip_suffix(suffix)
            && stripped.chars().count() >= 3
        {
            return stripped.to_string();
        }
    }
    name.to_string()
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    #[serde(rename = "temperature_2m")]
    temperature_c: f32,
    #[serde(rename = "apparent_temperature")]
    apparent_c: Option<f32>,
    #[serde(rename = "relative_humidity_2m")]
    humidity: Option<f32>,
    #[serde(rename = "wind_speed_10m")]
    wind_kmh: Option<f32>,
    #[serde(rename = "weather_code")]
    code: u32,
}

/// Fetch the current weather at `lat, lon` via Open-Meteo. Network
/// timeout is 4 seconds; on any failure returns a typed error rather
/// than panicking — the planner is expected to fall back to the
/// refusal template if this returns `Err`.
pub fn fetch_now(
    lat: f64,
    lon: f64,
    city_label: Option<String>,
) -> Result<WeatherReading, WeatherError> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?\
         latitude={lat:.4}&longitude={lon:.4}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,\
         weather_code,wind_speed_10m\
         &timezone=auto",
    );
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--max-time")
        .arg("4")
        .arg("--fail")
        .arg(&url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WeatherError::CurlMissing
            } else {
                WeatherError::CurlFailed(e.to_string())
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(WeatherError::CurlFailed(stderr));
    }
    let parsed: OpenMeteoResponse = serde_json::from_slice(&output.stdout)
        .map_err(|e| WeatherError::JsonParse(e.to_string()))?;
    Ok(WeatherReading {
        temperature_c: parsed.current.temperature_c,
        feels_like_c: parsed.current.apparent_c,
        humidity_pct: parsed.current.humidity,
        wind_kmh: parsed.current.wind_kmh,
        weather_code: parsed.current.code,
        city_label,
    })
}

/// Render a WMO weather code as a short Kazakh phrase. The official
/// Open-Meteo codes ([reference](https://open-meteo.com/en/docs))
/// map directly to a handful of conditions; we collapse them into
/// the broad categories adam can talk about meaningfully.
fn weather_code_kk(code: u32) -> &'static str {
    match code {
        0 => "ашық аспан",
        1 => "негізінен ашық",
        2 => "ішінара бұлтты",
        3 => "бұлтты",
        45 | 48 => "тұман",
        51 | 53 | 55 => "сіркіреп жаңбыр",
        56 | 57 => "мұзды жаңбыр",
        61 => "жеңіл жаңбыр",
        63 => "жаңбыр",
        65 => "қатты жаңбыр",
        66 | 67 => "мұзды жауын",
        71 => "жеңіл қар",
        73 => "қар жауып тұр",
        75 => "қатты қар",
        77 => "мұз дәндері",
        80 | 81 => "жаңбырлы жауын",
        82 => "қатты жаңбырлы жауын",
        85 | 86 => "қарлы жауын",
        95 => "найзағай",
        96 | 99 => "найзағаймен бұршақ",
        _ => "ауа райы белгілі",
    }
}

/// Format a WeatherReading as a single Kazakh sentence.
pub fn format_kk(r: &WeatherReading) -> String {
    let mut s = if let Some(city) = r.city_label.as_deref() {
        format!("{city}да", city = city)
    } else {
        "Сіздің жеріңізде".to_string()
    };
    s.push_str(&format!(
        " қазір {} — ауа температурасы {:.0} °C",
        weather_code_kk(r.weather_code),
        r.temperature_c
    ));
    if let Some(feels) = r.feels_like_c {
        if (feels - r.temperature_c).abs() >= 2.0 {
            s.push_str(&format!(" (сезімтал температура {:.0} °C)", feels));
        }
    }
    if let Some(wind) = r.wind_kmh {
        if wind >= 1.0 {
            s.push_str(&format!(", жел {:.0} км/сағ", wind));
        }
    }
    if let Some(rh) = r.humidity_pct
        && rh >= 1.0
    {
        s.push_str(&format!(", ылғалдылық {:.0} %", rh));
    }
    s.push('.');
    s
}

/// Convenience wrapper for the planner. Returns `Some(rendered)` when
/// a location resolves AND the network call succeeds; `None` in every
/// other case (so the caller keeps the refusal template).
pub fn render_live(session: &HashMap<String, String>) -> Option<String> {
    let (lat, lon, label) = resolve_location(session)?;
    let normalized_label = label.map(|c| capitalise_first(&c));
    match fetch_now(lat, lon, normalized_label) {
        Ok(reading) => Some(format_kk(&reading)),
        Err(_) => None,
    }
}

fn capitalise_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Honest refusal template surface for callers that want to explain
/// WHY the live answer isn't available (no location configured vs
/// fetch failed).
pub fn explain_no_location() -> &'static str {
    "Сіздің тұратын жеріңізді білмеймін. Қалаңызды айтсаңыз — ауа райын тексеремін. \
     (Немесе `ADAM_WEATHER_CITY=Алматы` орнатыңыз.)"
}

/// **Why this function is duplicated in shape only — not deleted —**:
/// keeps the `weather::Duration` import meaningful for future
/// `ureq`-based callers. Curl shell-out is the v6.0 transport;
/// dropping the import now would tear up the file later.
#[allow(dead_code)]
fn _retain_duration_import() -> Duration {
    Duration::from_secs(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_table_covers_all_oblast_centres() {
        let t = kazakh_city_coords();
        for city in [
            "алматы",
            "астана",
            "шымкент",
            "қарағанды",
            "ақтөбе",
            "тараз",
            "павлодар",
            "өскемен",
            "семей",
            "атырау",
            "қостанай",
            "қызылорда",
            "петропавл",
            "түркістан",
            "талдықорған",
            "орал",
            "көкшетау",
            "ақтау",
        ] {
            assert!(
                t.contains_key(city),
                "city table missing required entry: {city}"
            );
        }
    }

    #[test]
    fn locative_suffix_strip_resolves_almatyda() {
        // «Алматыда» (in Almaty) and «Алматыдан» (from Almaty) both
        // strip back to «алматы».
        assert_eq!(strip_locative_suffix("алматыда"), "алматы");
        assert_eq!(strip_locative_suffix("алматыдан"), "алматы");
        assert_eq!(strip_locative_suffix("қостанайдан"), "қостанай");
        assert_eq!(strip_locative_suffix("астанаға"), "астана");
    }

    #[test]
    fn lookup_city_handles_inflections() {
        assert_eq!(lookup_city("Қостанайдан"), Some((53.2144, 63.6246)));
        assert_eq!(lookup_city("алматыда"), Some((43.2389, 76.8897)));
    }

    #[test]
    fn resolve_location_prefers_env_lat_lon() {
        // SAFETY: env mutation in tests; restore at end.
        let prev_lat = std::env::var("ADAM_WEATHER_LAT").ok();
        let prev_lon = std::env::var("ADAM_WEATHER_LON").ok();
        unsafe {
            std::env::set_var("ADAM_WEATHER_LAT", "53.21");
            std::env::set_var("ADAM_WEATHER_LON", "63.62");
        }
        let session = HashMap::new();
        let resolved = resolve_location(&session);
        assert!(
            matches!(resolved, Some((lat, lon, _)) if (lat - 53.21).abs() < 0.01 && (lon - 63.62).abs() < 0.01)
        );
        // restore
        match prev_lat {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_LAT", v) },
            None => unsafe { std::env::remove_var("ADAM_WEATHER_LAT") },
        }
        match prev_lon {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_LON", v) },
            None => unsafe { std::env::remove_var("ADAM_WEATHER_LON") },
        }
    }

    #[test]
    fn resolve_location_falls_back_to_session_city() {
        // Make sure no env coords are interfering.
        let prev_lat = std::env::var("ADAM_WEATHER_LAT").ok();
        let prev_lon = std::env::var("ADAM_WEATHER_LON").ok();
        let prev_city = std::env::var("ADAM_WEATHER_CITY").ok();
        unsafe {
            std::env::remove_var("ADAM_WEATHER_LAT");
            std::env::remove_var("ADAM_WEATHER_LON");
            std::env::remove_var("ADAM_WEATHER_CITY");
        }
        let mut session = HashMap::new();
        session.insert("city".into(), "Қостанай".into());
        let resolved = resolve_location(&session);
        assert!(
            matches!(resolved, Some((lat, lon, label)) if (lat - 53.2144).abs() < 0.01 && (lon - 63.6246).abs() < 0.01 && label.as_deref() == Some("Қостанай"))
        );
        // restore
        match prev_lat {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_LAT", v) },
            None => {}
        }
        match prev_lon {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_LON", v) },
            None => {}
        }
        match prev_city {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_CITY", v) },
            None => {}
        }
    }

    #[test]
    fn resolve_location_returns_none_without_signals() {
        let prev_lat = std::env::var("ADAM_WEATHER_LAT").ok();
        let prev_lon = std::env::var("ADAM_WEATHER_LON").ok();
        let prev_city = std::env::var("ADAM_WEATHER_CITY").ok();
        unsafe {
            std::env::remove_var("ADAM_WEATHER_LAT");
            std::env::remove_var("ADAM_WEATHER_LON");
            std::env::remove_var("ADAM_WEATHER_CITY");
        }
        let session = HashMap::new();
        assert!(resolve_location(&session).is_none());
        match prev_lat {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_LAT", v) },
            None => {}
        }
        match prev_lon {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_LON", v) },
            None => {}
        }
        match prev_city {
            Some(v) => unsafe { std::env::set_var("ADAM_WEATHER_CITY", v) },
            None => {}
        }
    }

    #[test]
    fn format_kk_renders_full_sentence_with_city() {
        let r = WeatherReading {
            temperature_c: 15.4,
            feels_like_c: Some(13.0),
            humidity_pct: Some(55.0),
            wind_kmh: Some(8.0),
            weather_code: 2,
            city_label: Some("Қостанай".into()),
        };
        let s = format_kk(&r);
        assert!(s.contains("Қостанай"));
        assert!(s.contains("ішінара бұлтты"));
        assert!(s.contains("15 °C"));
        assert!(s.contains("жел"));
        assert!(s.contains("ылғалдылық"));
    }

    #[test]
    fn format_kk_skips_feels_like_when_close_to_air() {
        let r = WeatherReading {
            temperature_c: 20.0,
            feels_like_c: Some(20.5),
            humidity_pct: None,
            wind_kmh: None,
            weather_code: 0,
            city_label: Some("Алматы".into()),
        };
        let s = format_kk(&r);
        assert!(s.contains("ашық аспан"));
        assert!(s.contains("20 °C"));
        assert!(!s.contains("сезімтал"));
    }

    #[test]
    fn weather_code_table_covers_common_conditions() {
        assert_eq!(weather_code_kk(0), "ашық аспан");
        assert_eq!(weather_code_kk(3), "бұлтты");
        assert_eq!(weather_code_kk(63), "жаңбыр");
        assert_eq!(weather_code_kk(73), "қар жауып тұр");
        assert_eq!(weather_code_kk(95), "найзағай");
        assert_eq!(weather_code_kk(9999), "ауа райы белгілі");
    }
}
