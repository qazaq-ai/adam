//! Lightweight Kazakh language-core helpers for `adam-dialog`.
//!
//! This module is intentionally narrow:
//! - orthographic cleanup for user-provided proper nouns
//! - mixed-script normalization for Cyrillic Kazakh inputs
//! - conservative candidate checks for named-place extraction
//!
//! It does NOT duplicate `adam-kernel-fst` morphology. Parsing and
//! synthesis remain in the FST crate; this layer only prepares cleaner
//! string inputs for those deterministic components.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// Conservative proper-noun normaliser for user-provided person/place
/// names.
///
/// Keeps the pipeline deterministic and low-risk:
/// - fixes title casing (`дәулет` -> `Дәулет`)
/// - normalises per hyphen/apostrophe segment (`әли-хан` -> `Әли-Хан`)
/// - rewrites common Latin homoglyphs into Cyrillic when the token is
///   otherwise Cyrillic (`Aлматы` -> `Алматы`)
///
/// It intentionally does NOT attempt free-form spelling correction or
/// lexicon lookup: the goal is stable orthographic cleanup, not
/// probabilistic guessing.
pub fn normalize_proper_noun(s: &str) -> String {
    let mapped = if contains_cyrillic(s) {
        map_latin_homoglyphs_to_cyrillic(s)
    } else {
        s.to_string()
    };

    let mut out = String::with_capacity(mapped.len());
    let mut at_segment_start = true;
    for ch in mapped.chars() {
        if matches!(ch, '-' | '\'' | '’' | ' ') {
            out.push(ch);
            at_segment_start = true;
            continue;
        }
        if at_segment_start {
            out.extend(ch.to_uppercase());
            at_segment_start = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

pub fn looks_like_named_place_candidate(token: &str) -> bool {
    canonical_geo_name(token).is_some()
        || token.chars().count() > 1
            && token
                .chars()
                .all(|c| c.is_alphabetic() || matches!(c, '-' | '\'' | '’'))
}

pub fn normalize_place_name(token: &str) -> String {
    canonical_geo_name(token).unwrap_or_else(|| normalize_proper_noun(token))
}

pub fn canonical_geo_entity(token: &str) -> Option<GeoEntity> {
    let key = normalize_lookup_key(token);
    geo_catalog().get(&key).cloned()
}

pub fn canonical_geo_name(token: &str) -> Option<String> {
    canonical_geo_entity(token).map(|entry| entry.canonical)
}

pub fn canonical_geo_id(token: &str) -> Option<String> {
    canonical_geo_entity(token).map(|entry| entry.id)
}

pub fn geo_entity_kind(token: &str) -> Option<String> {
    canonical_geo_entity(token).map(|entry| entry.kind)
}

/// **v4.3.1** — person canonical entity resolver, symmetric to
/// [`canonical_geo_entity`].
///
/// Persons differ from geography in two important ways:
/// - There is no curated registry — adam can't ship a list of "all
///   Kazakh person names". The canonical form is therefore *the
///   normalized form itself*: the deterministic title-cased,
///   homoglyph-cleaned proper-noun spelling.
/// - We only collapse surfaces within the same script. A pure-Latin
///   input like `Daulet` stays Latin (it might mean a different
///   person than Cyrillic `Дәулет`); a mixed-script input like
///   `дӘУЛEТ` is mapped to Cyrillic via [`normalize_proper_noun`]
///   and then collapses to `Дәулет`.
///
/// The id format is `person:<canonical>` — distinct from the
/// geography `geo_kz_NNN` namespace so a single belief store can
/// hold both kinds of entities without key collisions.
///
/// Returns `None` for empty / single-char / non-alphabetic input,
/// and for input that is already a known geography entity (we never
/// want a place name to be silently re-classified as a person).
pub fn canonical_person_entity(token: &str) -> Option<PersonEntity> {
    if !looks_like_person_name(token) {
        return None;
    }
    let canonical = normalize_proper_noun(token.trim());
    if canonical.is_empty() {
        return None;
    }
    Some(PersonEntity {
        id: format!("person:{}", canonical),
        canonical,
    })
}

/// Lean accessor — id only. Symmetric with [`canonical_geo_id`].
pub fn canonical_person_id(token: &str) -> Option<String> {
    canonical_person_entity(token).map(|entry| entry.id)
}

/// Conservative shape guard for inputs that *may* be a person name.
/// Rejects:
/// - empty / single-character input,
/// - input containing digits or symbols other than `-` / `'` / `’`,
/// - input that already resolves to a known geography entity.
///
/// Does not look up any registry — it just checks orthographic
/// shape. The actual canonical resolution happens in
/// [`canonical_person_entity`].
pub fn looks_like_person_name(token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.chars().count() < 2 {
        return false;
    }
    if canonical_geo_entity(trimmed).is_some() {
        return false;
    }
    trimmed
        .chars()
        .all(|c| c.is_alphabetic() || matches!(c, '-' | '\'' | '’'))
}

fn contains_cyrillic(s: &str) -> bool {
    s.chars().any(is_cyrillic)
}

fn is_cyrillic(ch: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&ch) || ('\u{0500}'..='\u{052F}').contains(&ch)
}

fn map_latin_homoglyphs_to_cyrillic(s: &str) -> String {
    s.chars()
        .map(|ch| match ch {
            'A' => 'А',
            'a' => 'а',
            'B' => 'В',
            'C' => 'С',
            'c' => 'с',
            'E' => 'Е',
            'e' => 'е',
            'H' => 'Н',
            'h' => 'һ',
            'K' => 'К',
            'k' => 'к',
            'M' => 'М',
            'O' => 'О',
            'o' => 'о',
            'P' => 'Р',
            'p' => 'р',
            'T' => 'Т',
            'X' => 'Х',
            'x' => 'х',
            'Y' => 'У',
            'y' => 'у',
            _ => ch,
        })
        .collect()
}

fn normalize_lookup_key(s: &str) -> String {
    normalize_geo_phrase(s).to_lowercase()
}

fn normalize_geo_phrase(s: &str) -> String {
    let normalized = normalize_proper_noun(&s.replace('_', " "));
    let words: Vec<&str> = normalized
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return normalized;
    }
    let trimmed = trim_geo_descriptors(&words);
    trimmed.join(" ")
}

fn trim_geo_descriptors<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    let mut start = 0;
    let mut end = words.len();

    while start < end && is_leading_geo_descriptor(words[start]) {
        start += 1;
    }
    while end > start && is_trailing_geo_descriptor(words[end - 1]) {
        end -= 1;
    }

    if start == end {
        words.to_vec()
    } else {
        words[start..end].to_vec()
    }
}

fn is_leading_geo_descriptor(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "қала"
            | "ауыл"
            | "кент"
            | "аудан"
            | "облыс"
            | "өңір"
            | "өзен"
            | "көл"
            | "теңіз"
            | "тау"
            | "жота"
            | "мемлекет"
            | "ел"
            | "город"
            | "река"
            | "озеро"
            | "море"
            | "гора"
            | "страна"
    )
}

fn is_trailing_geo_descriptor(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "қала"
            | "қаласы"
            | "ауыл"
            | "ауылы"
            | "кент"
            | "кенті"
            | "аудан"
            | "ауданы"
            | "облыс"
            | "облысы"
            | "өңір"
            | "өңірі"
            | "өзен"
            | "өзені"
            | "көл"
            | "көлі"
            | "теңіз"
            | "теңізі"
            | "тау"
            | "тауы"
            | "жота"
            | "жотасы"
            | "мемлекет"
            | "елі"
            | "ел"
            | "город"
            | "городе"
            | "река"
            | "озеро"
            | "море"
            | "гора"
            | "страна"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoEntity {
    pub id: String,
    pub canonical: String,
    pub kind: String,
}

/// Person canonical entity, returned by [`canonical_person_entity`].
///
/// Unlike [`GeoEntity`], persons have no `kind` field — every person
/// is a person; the kind axis would only become meaningful with a
/// future role layer (e.g., `kind: "user" | "third_party"`), and that
/// belongs in `BeliefState`'s `EntityKind` rather than the
/// language-core resolver.
///
/// **v4.3.1**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonEntity {
    pub id: String,
    pub canonical: String,
}

type GeoCatalogEntry = GeoEntity;

#[derive(Debug, Deserialize)]
struct WorldCoreGeoLine {
    id: String,
    facts: Vec<WorldCoreGeoFact>,
    #[serde(default)]
    review_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorldCoreGeoFact {
    subject: String,
    predicate: String,
    object: String,
}

fn geo_catalog() -> &'static HashMap<String, GeoCatalogEntry> {
    static GEO_CATALOG: OnceLock<HashMap<String, GeoCatalogEntry>> = OnceLock::new();
    GEO_CATALOG.get_or_init(build_geo_catalog)
}

fn build_geo_catalog() -> HashMap<String, GeoCatalogEntry> {
    let raw = include_str!("../../../data/world_core/geography_kz.jsonl");
    let mut out = HashMap::new();
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(entry) = serde_json::from_str::<WorldCoreGeoLine>(line) else {
            continue;
        };
        if entry.review_status.as_deref() == Some("rejected") {
            continue;
        }
        for fact in entry.facts {
            if fact.predicate != "is_a" {
                continue;
            }
            let key = normalize_lookup_key(&fact.subject);
            out.entry(key).or_insert_with(|| GeoCatalogEntry {
                id: entry.id.clone(),
                canonical: normalize_proper_noun(&fact.subject),
                kind: fact.object,
            });
        }
    }
    add_geo_aliases(&mut out);
    out
}

fn add_geo_aliases(out: &mut HashMap<String, GeoCatalogEntry>) {
    let canonical_entries: Vec<GeoCatalogEntry> = out.values().cloned().collect();
    for entry in &canonical_entries {
        for alias in auto_geo_aliases(&entry.canonical, &entry.kind) {
            out.entry(normalize_lookup_key(&alias))
                .or_insert_with(|| entry.clone());
        }
    }

    for (alias, canonical) in curated_geo_aliases() {
        let canonical_key = normalize_lookup_key(canonical);
        let Some(entry) = out.get(&canonical_key).cloned() else {
            continue;
        };
        out.entry(normalize_lookup_key(alias)).or_insert(entry);
    }
}

fn auto_geo_aliases(canonical: &str, kind: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    match kind {
        "қала" => {
            aliases.push(format!("{canonical} қаласы"));
            aliases.push(format!("қала {canonical}"));
            aliases.push(format!("город {canonical}"));
        }
        "өзен" => {
            aliases.push(format!("{canonical} өзені"));
            aliases.push(format!("өзен {canonical}"));
            aliases.push(format!("река {canonical}"));
        }
        "теңіз" => {
            aliases.push(format!("{canonical} теңізі"));
            aliases.push(format!("теңіз {canonical}"));
            aliases.push(format!("море {canonical}"));
        }
        "көл" => {
            aliases.push(format!("{canonical} көлі"));
            aliases.push(format!("көл {canonical}"));
            aliases.push(format!("озеро {canonical}"));
        }
        "тау" => {
            aliases.push(format!("{canonical} тауы"));
            aliases.push(format!("{canonical} жотасы"));
            aliases.push(format!("тау {canonical}"));
            aliases.push(format!("гора {canonical}"));
        }
        "ел" | "мемлекет" => {
            aliases.push(format!("ел {canonical}"));
            aliases.push(format!("страна {canonical}"));
        }
        _ => {}
    }
    aliases
}

fn curated_geo_aliases() -> &'static [(&'static str, &'static str)] {
    &[
        ("алма-ата", "Алматы"),
        ("алмаата", "Алматы"),
        ("нұр-сұлтан", "Астана"),
        ("нурсултан", "Астана"),
        ("ақмола", "Астана"),
        ("целиноград", "Астана"),
        ("усть-каменогорск", "Өскемен"),
        ("семипалатинск", "Семей"),
        ("уральск", "Орал"),
        ("кустанай", "Қостанай"),
        ("актобе", "Ақтөбе"),
        ("кокшетау", "Көкшетау"),
        ("гурьев", "Атырау"),
        ("каспийское море", "Каспий"),
        ("аральское море", "Арал"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_proper_noun_fixes_case_and_script() {
        assert_eq!(normalize_proper_noun("дӘУЛEТ"), "Дәулет");
        assert_eq!(normalize_proper_noun("Aлматы"), "Алматы");
        assert_eq!(normalize_proper_noun("әли-хан"), "Әли-Хан");
    }

    #[test]
    fn named_place_candidate_accepts_lowercase_tokens() {
        assert!(looks_like_named_place_candidate("қашар"));
        assert!(looks_like_named_place_candidate("сарыағаш"));
        assert!(!looks_like_named_place_candidate("1"));
        assert!(!looks_like_named_place_candidate("a1"));
    }

    #[test]
    fn geo_catalog_reuses_world_core_geography_names() {
        assert_eq!(canonical_geo_name("алматы").as_deref(), Some("Алматы"));
        assert_eq!(canonical_geo_name("Aлматы").as_deref(), Some("Алматы"));
        assert_eq!(geo_entity_kind("каспий").as_deref(), Some("теңіз"));
        assert_eq!(canonical_geo_id("алматы").as_deref(), Some("geo_kz_004"));
    }

    #[test]
    fn geo_catalog_resolves_curated_aliases() {
        assert_eq!(canonical_geo_name("Алма-Ата").as_deref(), Some("Алматы"));
        assert_eq!(
            canonical_geo_name("Усть-Каменогорск").as_deref(),
            Some("Өскемен")
        );
        assert_eq!(geo_entity_kind("Кустанай").as_deref(), Some("қала"));
    }

    #[test]
    fn geo_catalog_returns_full_entity_record() {
        let entity = canonical_geo_entity("Каспий теңізі").expect("geo entity");
        assert_eq!(entity.id, "geo_kz_023");
        assert_eq!(entity.canonical, "Каспий");
        assert_eq!(entity.kind, "теңіз");
    }

    #[test]
    fn geo_catalog_trims_descriptor_phrases() {
        assert_eq!(
            canonical_geo_name("Алматы қаласы").as_deref(),
            Some("Алматы")
        );
        assert_eq!(
            canonical_geo_name("Каспий теңізі").as_deref(),
            Some("Каспий")
        );
        assert_eq!(
            canonical_geo_name("город Алматы").as_deref(),
            Some("Алматы")
        );
    }

    /// **v4.3.1** — surface variants of a person's name collapse to
    /// the same canonical entity. Verifies case fix, mixed-script
    /// homoglyph cleanup, and trim handling. The `id` namespace is
    /// `person:<canonical>`, never colliding with `geo_kz_NNN`.
    #[test]
    fn canonical_person_collapses_surface_variants() {
        let cyr = canonical_person_entity("Дәулет").expect("cyr name");
        assert_eq!(cyr.canonical, "Дәулет");
        assert_eq!(cyr.id, "person:Дәулет");

        let lower = canonical_person_entity("дәулет").expect("lowercase");
        assert_eq!(lower, cyr, "case fix must collapse to the same entity");

        let mixed = canonical_person_entity("дӘУЛEТ").expect("mixed-script");
        assert_eq!(
            mixed, cyr,
            "Latin homoglyph cleanup must collapse to the same Cyrillic entity"
        );

        let padded = canonical_person_entity("  Дәулет  ").expect("padded");
        assert_eq!(padded, cyr, "leading/trailing whitespace must not split");
    }

    /// **v4.3.1** — hyphenated names get per-segment title casing
    /// (matches the `normalize_proper_noun` contract).
    #[test]
    fn canonical_person_handles_hyphenated_names() {
        let entity = canonical_person_entity("әли-хан").expect("hyphenated");
        assert_eq!(entity.canonical, "Әли-Хан");
        assert_eq!(entity.id, "person:Әли-Хан");
    }

    /// **v4.3.1** — Latin-only inputs stay Latin (we don't have a
    /// transliteration table; conflating `Daulet` with `Дәулет` would
    /// be unsafe and is explicitly out of v4.3.1 scope).
    #[test]
    fn canonical_person_keeps_latin_inputs_separate() {
        let cyr = canonical_person_entity("Дәулет").expect("cyr");
        let lat = canonical_person_entity("Daulet").expect("lat");
        assert_ne!(
            cyr, lat,
            "Latin and Cyrillic surfaces must produce distinct ids"
        );
        assert_eq!(lat.canonical, "Daulet");
        assert_eq!(lat.id, "person:Daulet");
    }

    /// **v4.3.1** — known geography entities never get reclassified
    /// as persons. The guard rejects them up-front.
    #[test]
    fn canonical_person_rejects_known_geography() {
        assert_eq!(canonical_person_entity("Алматы"), None);
        assert_eq!(canonical_person_entity("алматы"), None);
        assert_eq!(canonical_person_entity("Каспий"), None);
    }

    /// **v4.3.1** — empty / single-char / digit-bearing / whitespace-
    /// only input is rejected. Avoids producing `person:` (empty
    /// canonical) or `person:1` (digit) ids.
    #[test]
    fn canonical_person_rejects_invalid_shape() {
        assert_eq!(canonical_person_entity(""), None);
        assert_eq!(canonical_person_entity("   "), None);
        assert_eq!(canonical_person_entity("Д"), None);
        assert_eq!(canonical_person_entity("Daulet99"), None);
        assert_eq!(canonical_person_entity("123"), None);
    }

    /// **v4.3.1** — lean `canonical_person_id` accessor returns the
    /// id only and tracks `canonical_person_entity` exactly.
    #[test]
    fn canonical_person_id_lean_accessor() {
        assert_eq!(
            canonical_person_id("дәулет").as_deref(),
            Some("person:Дәулет")
        );
        assert_eq!(canonical_person_id("Алматы"), None);
        assert_eq!(canonical_person_id(""), None);
    }
}
