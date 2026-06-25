// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `lang_bridge` — **v6.8.25 bounded peripheral pilot.**
//!
//! Implements the «peripheral semantic adapter» pattern from
//! the 2026-06-25 strategic review: Russian / English queries
//! about facts adam knows are translated, looked up, and
//! mirrored back in the source language WITHOUT duplicating
//! the canonical Kazakh fact graph.
//!
//! ## What this module covers
//!
//! v6.8.25 ships a single bounded slice: **capital-of-country**
//! queries.  Detects RU / EN / KZ query shapes, looks up the
//! country in a curated trilingual table, formats the answer in
//! the same language the query came in.
//!
//! Scope is deliberately small per Codex's «bounded pilot»
//! direction — not a generic translation layer, not all world
//! capitals.  Adds the user-visible value the
//! `language_switch_mid_dialog` probe asks for; validates the
//! peripheral pattern before scaling.
//!
//! ## What this module does NOT do
//!
//! - Generic Russian / English understanding.  Every shape is
//!   an explicit regex / substring match.
//! - Open-domain country trivia.  Only capital lookups.
//! - Translation of adam's safety / clarification / wellness
//!   templates.  Those stay Kazakh-first — the peripheral
//!   adapter is for FACTUAL queries.

/// Capital-of-country lookup entry.  `kz` / `ru` / `en` carry
/// the country name in each language as **stem prefixes**
/// (lowercased, used as substring matches against query
/// input); `cap_ru` / `cap_en` carry the capital name for
/// response formatting.  No `cap_kz` — KZ queries are handled
/// by the existing v6.1 Kazakhstan handler with its richer
/// template.
#[derive(Debug, Clone, Copy)]
struct CapitalEntry {
    kz: &'static str,
    ru: &'static str,
    en: &'static str,
    cap_ru: &'static str,
    cap_en: &'static str,
}

/// Curated neighbour / strategic-country capitals.  Coverage:
/// Central-Asian neighbours, major economic partners
/// (Russia / China / Turkey), and high-traffic code-switch
/// targets.  The `ru` / `en` fields are **stem prefixes** so
/// RU genitive («России» ← «Россия») and other inflected
/// forms still match.  Kazakhstan is intentionally absent —
/// KZ-about-KZ queries are richer in the existing v6.1
/// handler; RU/EN-about-Kazakhstan is a special case in
/// `lookup_capital`.  Expanding this list is data work, not
/// architecture work.
const CAPITALS: &[CapitalEntry] = &[
    CapitalEntry {
        kz: "ресей",
        ru: "росси", // россия / России / Россию / Россией
        en: "russia",
        cap_ru: "Москва",
        cap_en: "Moscow",
    },
    CapitalEntry {
        kz: "қырғызстан",
        ru: "кыргызстан",
        en: "kyrgyzstan",
        cap_ru: "Бишкек",
        cap_en: "Bishkek",
    },
    CapitalEntry {
        kz: "өзбекстан",
        ru: "узбекистан",
        en: "uzbekistan",
        cap_ru: "Ташкент",
        cap_en: "Tashkent",
    },
    CapitalEntry {
        kz: "түрікменстан",
        ru: "туркменистан",
        en: "turkmenistan",
        cap_ru: "Ашхабад",
        cap_en: "Ashgabat",
    },
    CapitalEntry {
        kz: "қытай",
        ru: "кита", // китай / Китая / Китаю
        en: "china",
        cap_ru: "Пекин",
        cap_en: "Beijing",
    },
    CapitalEntry {
        kz: "түркия",
        ru: "турци", // турция / Турции / Турцию
        en: "turkey",
        cap_ru: "Анкара",
        cap_en: "Ankara",
    },
    CapitalEntry {
        kz: "иран",
        ru: "иран",
        en: "iran",
        cap_ru: "Тегеран",
        cap_en: "Tehran",
    },
    CapitalEntry {
        kz: "беларусь",
        ru: "беларус", // беларусь / Беларуси
        en: "belarus",
        cap_ru: "Минск",
        cap_en: "Minsk",
    },
    CapitalEntry {
        kz: "әзірбайжан",
        ru: "азербайджан",
        en: "azerbaijan",
        cap_ru: "Баку",
        cap_en: "Baku",
    },
];

/// Kazakhstan special case: only the RU and EN query paths.
/// KZ-about-KZ goes through the v6.1 handler unchanged.
const KAZAKHSTAN_RU_STEMS: &[&str] = &["казахстан"]; // казахстан / Казахстана / Казахстану
const KAZAKHSTAN_EN_STEMS: &[&str] = &["kazakhstan"];

/// Language the query was asked in.  Determines response
/// language so adam mirrors back to the user's input
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryLang {
    Ru,
    En,
}

/// Detect a RU / EN capital-of-country query shape.  KZ shapes
/// are intentionally NOT handled here — they go through the
/// existing v6.1 Kazakh capital handler which has richer
/// templates (dates, alias chain).  Returns `None` for
/// anything else; the caller falls through to the cascade.
fn detect_capital_query_lang(lower: &str) -> Option<QueryLang> {
    if lower.contains("столица") || lower.contains("столиц") {
        return Some(QueryLang::Ru);
    }
    if lower.contains("capital of") || lower.contains("what's the capital") {
        return Some(QueryLang::En);
    }
    None
}

/// Find the country `lower` is asking about.  Substring
/// match against all three language fields of each entry.
fn find_capital_entry(lower: &str) -> Option<&'static CapitalEntry> {
    CAPITALS
        .iter()
        .find(|e| lower.contains(e.kz) || lower.contains(e.ru) || lower.contains(e.en))
}

/// Format the answer in the requested language.  Only RU and
/// EN paths exist — KZ queries are not handled here.
fn format_answer(entry: &CapitalEntry, lang: QueryLang) -> String {
    match lang {
        QueryLang::Ru => format!("Столица — {}.", entry.cap_ru),
        QueryLang::En => format!("The capital is {}.", entry.cap_en),
    }
}

/// **Public entry point.**  Returns `Some(answer_text)` when
/// the input matches the capital-of-country shape AND the
/// asked-about country is in the curated table.  Returns
/// `None` otherwise so the cascade handles the input via the
/// existing Kazakh handlers (or falls to clarification).
///
/// **Kazakhstan special case:** RU / EN queries about
/// Kazakhstan return the RU / EN-mirrored answer here;
/// Kazakh queries about Kazakhstan return `None` so the v6.1
/// handler (with its richer dated template) wins.
pub fn lookup_capital(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    let lang = detect_capital_query_lang(&lower)?;
    let asks_kz = match lang {
        QueryLang::Ru => KAZAKHSTAN_RU_STEMS.iter().any(|s| lower.contains(s)),
        QueryLang::En => KAZAKHSTAN_EN_STEMS.iter().any(|s| lower.contains(s)),
    };
    if asks_kz {
        return Some(match lang {
            QueryLang::Ru => "Столица — Астана.".into(),
            QueryLang::En => "The capital is Astana.".into(),
        });
    }
    let entry = find_capital_entry(&lower)?;
    Some(format_answer(entry, lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ru_capital_query_returns_ru_answer() {
        assert_eq!(
            lookup_capital("А столица России?"),
            Some("Столица — Москва.".into()),
        );
        assert_eq!(
            lookup_capital("Столица Узбекистана?"),
            Some("Столица — Ташкент.".into()),
        );
    }

    #[test]
    fn en_capital_query_returns_en_answer() {
        assert_eq!(
            lookup_capital("What's the capital of China?"),
            Some("The capital is Beijing.".into()),
        );
        assert_eq!(
            lookup_capital("the capital of Turkey?"),
            Some("The capital is Ankara.".into()),
        );
    }

    #[test]
    fn kz_queries_about_foreign_countries_fall_through() {
        // KZ queries are intentionally NOT handled by the
        // bridge — they go through the existing v6.1 Kazakh
        // handler.  v6.8.25 ships only RU/EN entry points.
        assert!(lookup_capital("Ресейдің астанасы қай қала?").is_none());
    }

    #[test]
    fn unrecognised_country_returns_none() {
        // Country not in the curated table — pilot is bounded.
        assert!(lookup_capital("столица Бразилии?").is_none());
    }

    #[test]
    fn non_capital_query_returns_none() {
        assert!(lookup_capital("Сәлем!").is_none());
        assert!(lookup_capital("How are you?").is_none());
        assert!(lookup_capital("Россия — большая страна.").is_none());
    }

    #[test]
    fn kazakhstan_via_ru_query_returns_ru_astana() {
        assert_eq!(
            lookup_capital("А столица Казахстана?"),
            Some("Столица — Астана.".into()),
        );
    }
}
