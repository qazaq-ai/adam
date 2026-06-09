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

/// **2026-06-03 evening** — slot validation guard.
///
/// `looks_like_named_place_candidate` accepts any alphabetic word
/// >1 character so that brand-new cities not yet in the geo
/// registry still parse.  The fallback became dangerous on voice
/// REPL — Whisper-drifted noise tokens like «қайық» (boat) or «ең»
/// (most / particle) entered the city slot and adam acknowledged
/// «Қайық екен, түсіндім» as if a real city were named.
///
/// This guard says: reject a candidate that is a **known common
/// noun** AND is **not** in the geo registry.  Two sources are
/// consulted:
///
///   1. The closed-set «not-a-topic» list (interrogatives,
///      demonstratives, function words) — same list the existing
///      detect_statement_of_location uses to filter `Қай` etc.
///   2. A small but high-recall common-noun blocklist of the
///      tokens we've seen Whisper-drift into the city slot.
///
/// Returns `true` if the candidate should be REJECTED (i.e. it's
/// noise, not a real place name).  Used by location / origin
/// detectors before emitting `StatementOfLocation { city }`.
pub fn looks_like_common_noun_not_a_place(token: &str) -> bool {
    // Genuine geo registry entries are never «common nouns» — short-
    // circuit so user-added cities (e.g. «Жетісай») don't get blocked.
    if canonical_geo_name(token).is_some() {
        return false;
    }
    let lower = token.to_lowercase();
    let lower = lower.trim_end_matches(|c: char| !c.is_alphabetic());
    // Closed-set common nouns we've seen in voice-REPL drift.
    // Listed as bare nouns; the locative / accusative / etc.
    // case-stripping that happens upstream should leave us with
    // the bare stem.
    let common_nouns: &[&str] = &[
        // Drift sources caught in live REPL (2026-06-03):
        "қайық", // boat
        "ең",    // most / particle
        // **Phase 26.B (2026-06-04 — post-rc11 audit)** — more
        // Whisper drifts of «қайда» / «қай жерде» (interrogative
        // "where") that the rc10 blocklist missed.  All map to
        // a recall-query slip the user makes when asking "where
        // do I live", not a statement of location:
        "қажер", // «қай жер» (which place) compressed
        "қаж",   // partial of «қайжер»
        "қажерде",
        // Generic place descriptors (already filtered by
        // generic_place_root, but listed here for defense-in-depth):
        "қала",
        "ауыл",
        "аудан",
        "облыс",
        "өңір",
        "кент",
        "ел",
        // Pronouns / function words that the FST sometimes parses
        // as nouns under drift:
        "қай",
        "қандай",
        "қашан",
        "не",
        "кім",
        "осы",
        "сол",
        "анау",
        "мынау",
        "бұл",
        "соған",
        // Additional question/recall words that should never be
        // a city slot fill:
        "қайда",
        "қайдан",
        "қалай",
        "неге",
    ];
    // Exact match OR exact match + Kazakh case suffix (locative -да/-де,
    // accusative -ды/-ді, etc.).  Refuses naive prefix-match that would
    // false-fire on «суыл» (starts with «су»).
    common_nouns.iter().any(|n| {
        if lower == *n {
            return true;
        }
        if !lower.starts_with(n) {
            return false;
        }
        let suffix = &lower[n.len()..];
        matches!(
            suffix,
            "да" | "де" | "та" | "те"  // locative
                | "ды" | "ді" | "ты" | "ті"  // accusative
                | "дан" | "ден" | "тан" | "тен"  // ablative
                | "ға" | "ге" | "қа" | "ке"  // dative
                | "мын" | "мін" | "сың" | "сің"  // 1/2sg copula
                | "тың" | "тің" | "дың" | "дің" // genitive
        )
    })
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
/// **v4.18.0** — respectful Kazakh address form.
///
/// Builds the diminutive-respectful version of a personal name by
/// taking the first consonant and appending «әке» (etymologically
/// "father / elder"). This is the warm, respectful way for a younger
/// or junior speaker to address an older or honoured person in
/// Kazakh tradition. Since adam is a young system addressing the
/// human user, every post-introduction turn should use this form
/// instead of the literal name.
///
/// **Pattern.** Take the first consonant of the name, append `әке`
/// (preserving the case of the input's first letter):
///
/// - `Дәулет → Дәке` (Д + әке)
/// - `Марат → Мәке` (М + әке)
/// - `Серік → Сәке`
/// - `Нұрлан → Нәке`
/// - `Жанболат → Жәке`
///
/// **Vowel-initial names** (Абай, Алия, Айгүл, Аман) return `None`
/// — the «<consonant>әке» pattern doesn't fit, and the alternative
/// «<vowel>+әке» would collide with adam's own name (`Адам → Әке`,
/// where `әке` literally means "father" — confusing). Callers
/// should fall back to the literal name in those cases.
///
/// **Returns.** `Some("Дәке")` for consonant-initial names, `None`
/// otherwise.
pub fn kazakh_respectful_address(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() < 3 {
        return None;
    }
    let first = chars[0];
    if !first.is_alphabetic() {
        return None;
    }
    let first_uppercase = first.to_uppercase().next().unwrap_or(first);
    if !is_kazakh_vowel(first) {
        // **v4.18.0** — consonant-initial: first consonant + «әке».
        // Дәулет → Дәке, Мұрат → Мәке, Сергей → Сәке.
        return Some(format!("{first_uppercase}әке"));
    }
    // **v4.51.5** — vowel-initial: take first 2 characters of the
    // name + «әке». Per Kazakh address tradition: Арман → Арәке,
    // Алия → Аләке, Абай → Абәке, Айгүл → Айәке (where «й» is
    // treated as part of the first syllable). Simpler than tracking
    // vowel/consonant categories — the first two characters reliably
    // form the diminutive prefix.
    let second = chars[1];
    if !second.is_alphabetic() {
        return None;
    }
    Some(format!(
        "{}{}әке",
        first_uppercase,
        second.to_lowercase().next().unwrap_or(second)
    ))
}

/// **v6.0.0-rc5 MOD voice REPL 2026-05-20** — Kazakh personal-name
/// gender classification. Returns the inferred gender of a given
/// proper noun via a closed-list lookup over the ~120 most common
/// Kazakh male / female names + a heuristic suffix fallback for
/// out-of-list entries.
///
/// Used to choose culturally-appropriate respectful address forms:
/// male names take the bare «-әке» honorific (Дәулет → Дәке);
/// female names take «-жан» suffix (Айгерим → Айгерімжан) and,
/// when the speaker context includes a teacher / elder role,
/// «X апай» (Айгерим апай) instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KazakhNameGender {
    Male,
    Female,
}

/// Top-120 Kazakh male personal names (lowercased). Closed list
/// for explicit gender lookup; out-of-list inputs fall through to
/// the heuristic suffix detector.
const KAZAKH_MALE_NAMES: &[&str] = &[
    "абай",
    "абдулла",
    "абзал",
    "абылай",
    "абзал",
    "айбар",
    "айбек",
    "айдар",
    "айдос",
    "айдын",
    "айнур",
    "айтуар",
    "алдияр",
    "алибек",
    "алимжан",
    "алмат",
    "алтай",
    "альмас",
    "ансар",
    "анвар",
    "арман",
    "армат",
    "арнур",
    "арсен",
    "аршат",
    "аскар",
    "аслан",
    "асылбек",
    "ахмет",
    "аян",
    "ауэлбек",
    "әбіш",
    "әділ",
    "әділет",
    "әзіз",
    "әнуар",
    "ержан",
    "ерлан",
    "ермек",
    "ернар",
    "ерсин",
    "есен",
    "жаміл",
    "жаңабай",
    "жанболат",
    "жанболды",
    "жанибек",
    "жасулан",
    "кадыр",
    "кайрат",
    "канат",
    "касым",
    "кенесары",
    "куаныш",
    "қазыбек",
    "қанат",
    "қасым",
    "мадияр",
    "максат",
    "манап",
    "мансур",
    "марат",
    "мейір",
    "мирас",
    "мұрат",
    "мухтар",
    "нурболат",
    "нурлан",
    "нурсултан",
    "олжас",
    "омар",
    "райымбек",
    "ринат",
    "руслан",
    "рустам",
    "сабит",
    "сабыр",
    "санжар",
    "сапар",
    "саят",
    "сейтжан",
    "серік",
    "сұлтан",
    "султанбек",
    "тауман",
    "тимур",
    "темир",
    "темирлан",
    "темирхан",
    "тоқтар",
    "торехан",
    "ұлан",
    "ұлықбек",
    "фархат",
    "хамза",
    "чингиз",
    "шерхан",
    "шынғыс",
    "ыдырыс",
    "юсуф",
    "ясын",
    // common Russian-loan male names that also appear in Kazakh-
    // speaker dialogs
    "александр",
    "андрей",
    "артем",
    "виктор",
    "виталий",
    "владимир",
    "дмитрий",
    "евгений",
    "иван",
    "максим",
    "михаил",
    "николай",
    "олег",
    "павел",
    "роман",
    "сергей",
    "юрий",
    "ярослав",
    "дәулет",
    "бауыржан",
    "бекжан",
    "болат",
    "берік",
    "досжан",
    // **v6.4.0-rc9 (2026-06-08) additions** — names surfaced by
    // the end-to-end test suite that were missing from the
    // earlier closed list.  Includes «Нұрсұлтан» (first president)
    // and common Кazakh variants.
    "нұрсұлтан",
    "мәулет",
    "дауыл",
    "дауылбек",
    "құрманбек",
    "бекзат",
    "арман",
    "ержан",
    "ерлан",
    "айдос",
    "адлет",
    "айбек",
];

/// Top-120 Kazakh female personal names (lowercased). Same
/// closed-list approach as the male table.
const KAZAKH_FEMALE_NAMES: &[&str] = &[
    "айгерим",
    "айгерім",
    "айгүл",
    "айгуль",
    "айдана",
    "айжан",
    "айзада",
    "айман",
    "айнагүл",
    "айнагул",
    "айнур",
    "айсулу",
    "айша",
    "акмарал",
    "ақмарал",
    "алия",
    "алуа",
    "анар",
    "анель",
    "арайлым",
    "арайым",
    "аруна",
    "аружан",
    "асем",
    "асель",
    "асемгүл",
    "атиля",
    "аяулым",
    "балжан",
    "балнур",
    "балсулу",
    "бану",
    "ботагөз",
    "гаухар",
    "гулмира",
    "гүлбану",
    "гүлден",
    "гүлжан",
    "гүлзада",
    "гүлзат",
    "гүлмира",
    "гүлназ",
    "гүлнар",
    "гүлсім",
    "гүлшат",
    "дана",
    "дария",
    "дилда",
    "динара",
    "елнура",
    "ерке",
    "жадыра",
    "жанар",
    "жанна",
    "жулдыз",
    "жұлдыз",
    "жайна",
    "зада",
    "зарина",
    "зухра",
    "індіра",
    "камилла",
    "карима",
    "карлыгаш",
    "құралай",
    "лаззат",
    "лаура",
    "ләззат",
    "лейла",
    "мадина",
    "майра",
    "макпал",
    "мәдина",
    "маржан",
    "мерей",
    "мейрам",
    "молдір",
    "набат",
    "назгүл",
    "назым",
    "нұргүл",
    "нұрсая",
    "нурсулу",
    "перизат",
    "райхан",
    "раушан",
    "рауза",
    "рахима",
    "роза",
    "сабина",
    "сабира",
    "сайран",
    "салима",
    "салтанат",
    "сандугаш",
    "сауле",
    "сая",
    "тоғжан",
    "толқын",
    "ұлжан",
    "үміт",
    "фариза",
    "фатима",
    "шынар",
    "ырысты",
    "элина",
    "эльмира",
    "әсем",
    "әсия",
    "әйгерім",
    "айбала",
    "айбике",
    "аяжан",
    "аяна",
    "бағлан",
    "балауса",
    "гүлназ",
    "діна",
    "інжу",
    "карина",
    "сабиля",
    "анастасия",
    "екатерина",
    "елена",
    "ирина",
    "ольга",
    "татьяна",
];

/// Lookup `name` in the male/female closed lists; falls back to
/// suffix heuristics for out-of-list inputs. Returns `None` when
/// neither path commits — caller then defaults to the generic
/// «-әке» honorific.
pub fn kazakh_name_gender(name: &str) -> Option<KazakhNameGender> {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    if KAZAKH_MALE_NAMES.iter().any(|n| *n == lower) {
        return Some(KazakhNameGender::Male);
    }
    if KAZAKH_FEMALE_NAMES.iter().any(|n| *n == lower) {
        return Some(KazakhNameGender::Female);
    }
    // **v6.4.0-rc11 (2026-06-08 audit).**  Fuzzy DB lookup.
    // Whisper routinely substitutes Kazakh-specific characters
    // for their nearest Russian-keyboard neighbours (ә→а, ң→н,
    // қ→к, ғ→г, ө→о, ұ→у, ү→у, і→и, һ→х).  Exact-match against
    // the DB then misses real names: «даулет» (Whisper) → DB
    // has «дәулет» → rc9 strict check rejected the capture →
    // cascade fell through to topic retrieval over «ат» (horse)
    // and surfaced an Abai proverb instead of acknowledging
    // the name.
    //
    // rc11 — if exact lookup misses, try a fuzzy match against
    // the canonical DB entries using `kazakh_edit_distance`'s
    // built-in cost-0.5 confusion pairs for the Kazakh-specific
    // character set.  Threshold 0.83 = at most ~one character
    // confusion in a 6-letter name.
    if let Some(g) = fuzzy_match_kazakh_name_gender(&lower) {
        return Some(g);
    }
    // Suffix heuristics for unknown names. Strong female endings
    // first (гүл / гул / сім / сем / назым / айым are nearly
    // exclusively female in Kazakh onomastics).
    const FEMALE_SUFFIXES: &[&str] = &[
        "гүл", "гул", "сім", "сем", "айым", "айым", "сұлу", "сулу", "жан",
    ];
    for suf in FEMALE_SUFFIXES {
        if lower.ends_with(suf) && lower.chars().count() > suf.chars().count() + 1 {
            // «жан» is also a male suffix (Жаңабай / Ержан); only commit
            // when paired with a clearly female stem — bail and let the
            // caller default. The other suffixes are unambiguous.
            if *suf == "жан" {
                continue;
            }
            return Some(KazakhNameGender::Female);
        }
    }
    None
}

/// rc11 — fuzzy DB lookup with Kazakh-character tolerance.
/// Returns the gender of the canonical DB entry closest to the
/// candidate token under [`crate::kazakh_fuzzy::kazakh_similarity`]
/// (which uses 0.5-cost confusion pairs for ә↔а / ң↔н / қ↔к /
/// ғ↔г / ө↔о / ұ↔у / ү↔у / і↔и / һ↔х).  Threshold 0.83 catches
/// at most one Whisper-confusion-character in a 6-letter name.
fn fuzzy_match_kazakh_name_gender(candidate: &str) -> Option<KazakhNameGender> {
    use crate::kazakh_fuzzy::kazakh_similarity;
    const THRESHOLD: f32 = 0.83;
    let mut best_score: f32 = 0.0;
    let mut best_gender: Option<KazakhNameGender> = None;
    for &m in KAZAKH_MALE_NAMES {
        let s = kazakh_similarity(candidate, m);
        if s > best_score && s >= THRESHOLD {
            best_score = s;
            best_gender = Some(KazakhNameGender::Male);
        }
    }
    for &f in KAZAKH_FEMALE_NAMES {
        let s = kazakh_similarity(candidate, f);
        if s > best_score && s >= THRESHOLD {
            best_score = s;
            best_gender = Some(KazakhNameGender::Female);
        }
    }
    best_gender
}

/// rc11 — canonical-form lookup.  Returns the DB entry closest to
/// the candidate (title-cased) when the fuzzy similarity is at
/// least `threshold`.  Lets the caller replace a Whisper-noise
/// spelling («даулет») with the canonical form («Дәулет»).
pub fn kazakh_name_canonical(candidate: &str, threshold: f32) -> Option<String> {
    use crate::kazakh_fuzzy::kazakh_similarity;
    let lower = candidate.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    let mut best_score: f32 = 0.0;
    let mut best_match: Option<&str> = None;
    for &n in KAZAKH_MALE_NAMES.iter().chain(KAZAKH_FEMALE_NAMES.iter()) {
        let s = kazakh_similarity(&lower, n);
        if s > best_score && s >= threshold {
            best_score = s;
            best_match = Some(n);
        }
    }
    best_match.map(|m| {
        // Title-case first character (m is already lowercase).
        let mut chars = m.chars();
        match chars.next() {
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    })
}

/// **v6.0.0-rc5 MOD voice REPL 2026-05-20** — Gender-aware
/// respectful-form generator. Wraps the legacy `kazakh_respectful_
/// address` so existing callers can opt into culturally-correct
/// female forms incrementally.
///
/// Male / unknown → existing «-әке» suffix (Дәулет → Дәке);
/// Female → literal name + «жан» (Айгерім → Айгерімжан) which
/// is the standard Kazakh-female endearing form. When the
/// dialog context carries a teacher / elder role (caller can
/// signal via the `role` argument), female addressing falls
/// back to «{Name} апай» instead.
pub fn kazakh_respectful_address_gendered(
    name: &str,
    gender: Option<KazakhNameGender>,
    role: Option<&str>,
) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let cap = capitalize_first(trimmed);
    match gender {
        Some(KazakhNameGender::Female) => {
            if matches!(
                role,
                Some("ұстаз")
                    | Some("мұғалім")
                    | Some("педагог")
                    | Some("учитель")
                    | Some("teacher")
            ) {
                return Some(format!("{cap} апай"));
            }
            Some(format!("{cap}жан"))
        }
        Some(KazakhNameGender::Male) | None => kazakh_respectful_address(trimmed),
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// **v4.18.0** — Kazakh vowel set (both Cyrillic native vowels and
/// vowel-mark variants). Used by `kazakh_respectful_address` to
/// decide whether a name is consonant- or vowel-initial.
fn is_kazakh_vowel(c: char) -> bool {
    let lower = c.to_lowercase().next().unwrap_or(c);
    matches!(
        lower,
        'а' | 'ә'
            | 'е'
            | 'ё'
            | 'и'
            | 'й'
            | 'о'
            | 'ө'
            | 'у'
            | 'ұ'
            | 'ү'
            | 'ы'
            | 'і'
            | 'э'
            | 'ю'
            | 'я'
    )
}

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
    fn kazakh_name_gender_detects_top_male() {
        for n in ["Дәулет", "Бауыржан", "Марат", "Ерлан", "Нурлан", "Серік"]
        {
            assert_eq!(
                kazakh_name_gender(n),
                Some(KazakhNameGender::Male),
                "name={n}"
            );
        }
    }

    #[test]
    fn kazakh_name_gender_detects_top_female() {
        for n in [
            "Айгерім",
            "Айгүл",
            "Айдана",
            "Асем",
            "Гүлмира",
            "Назым",
            "Сабина",
            "Шынар",
        ] {
            assert_eq!(
                kazakh_name_gender(n),
                Some(KazakhNameGender::Female),
                "name={n}"
            );
        }
    }

    #[test]
    fn kazakh_name_gender_suffix_heuristic_female() {
        // Out-of-list female names with «-гүл» / «-сім» endings
        // resolve via the suffix heuristic.
        assert_eq!(kazakh_name_gender("Нұргүл"), Some(KazakhNameGender::Female));
        assert_eq!(kazakh_name_gender("Айсулу"), Some(KazakhNameGender::Female));
    }

    #[test]
    fn kazakh_respectful_address_gendered_female() {
        // Female default: «{Name}жан».
        assert_eq!(
            kazakh_respectful_address_gendered("Айгерім", Some(KazakhNameGender::Female), None)
                .as_deref(),
            Some("Айгерімжан")
        );
        // Teacher role: «{Name} апай».
        assert_eq!(
            kazakh_respectful_address_gendered(
                "Айгерім",
                Some(KazakhNameGender::Female),
                Some("ұстаз")
            )
            .as_deref(),
            Some("Айгерім апай")
        );
        assert_eq!(
            kazakh_respectful_address_gendered(
                "Сабина",
                Some(KazakhNameGender::Female),
                Some("мұғалім")
            )
            .as_deref(),
            Some("Сабина апай")
        );
    }

    #[test]
    fn kazakh_respectful_address_gendered_male_unchanged() {
        // Male falls through to the legacy «-әке» path.
        assert_eq!(
            kazakh_respectful_address_gendered("Дәулет", Some(KazakhNameGender::Male), None)
                .as_deref(),
            Some("Дәке")
        );
    }

    #[test]
    fn respectful_address_consonant_initial_names() {
        // Canonical pattern: first consonant + әке, preserving case.
        assert_eq!(kazakh_respectful_address("Дәулет").as_deref(), Some("Дәке"));
        assert_eq!(kazakh_respectful_address("Марат").as_deref(), Some("Мәке"));
        assert_eq!(kazakh_respectful_address("Серік").as_deref(), Some("Сәке"));
        assert_eq!(kazakh_respectful_address("Нұрлан").as_deref(), Some("Нәке"));
        assert_eq!(
            kazakh_respectful_address("Жанболат").as_deref(),
            Some("Жәке")
        );
    }

    #[test]
    fn respectful_address_lowercase_input_uppercased() {
        // Even if the input is lowercase, the rendered respectful
        // form uses the title-cased first letter so it reads well
        // in templates («Дәке, ...» not «дәке, ...»).
        assert_eq!(kazakh_respectful_address("дәулет").as_deref(), Some("Дәке"));
    }

    #[test]
    fn respectful_address_vowel_initial_uses_two_letter_prefix() {
        // **v4.51.5** — vowel-initial: first vowel + first consonant
        // after the vowel + «әке». Per Kazakh tradition: Арман →
        // Арәке, Алия → Аләке, Абай → Абәке.
        assert_eq!(kazakh_respectful_address("Абай").as_deref(), Some("Абәке"));
        assert_eq!(kazakh_respectful_address("Алия").as_deref(), Some("Аләке"));
        assert_eq!(kazakh_respectful_address("Айгүл").as_deref(), Some("Айәке"));
        assert_eq!(kazakh_respectful_address("Аман").as_deref(), Some("Амәке"));
        assert_eq!(kazakh_respectful_address("Әлем").as_deref(), Some("Әләке"));
        assert_eq!(kazakh_respectful_address("Ермек").as_deref(), Some("Ерәке"));
        assert_eq!(kazakh_respectful_address("Ысқақ").as_deref(), Some("Ысәке"));
        assert_eq!(kazakh_respectful_address("Олжас").as_deref(), Some("Оләке"));
        assert_eq!(kazakh_respectful_address("Үсен").as_deref(), Some("Үсәке"));
    }

    #[test]
    fn respectful_address_empty_or_invalid_returns_none() {
        assert!(kazakh_respectful_address("").is_none());
        assert!(kazakh_respectful_address("   ").is_none());
        // Non-alphabetic first character.
        assert!(kazakh_respectful_address("123").is_none());
    }

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

    /// **2026-06-03 evening** — slot validation guard.
    ///
    /// Live REPL: «Мен қайық қалада тұрамын» (Whisper drift) yielded
    /// «Қайық екен, түсіндім».  The detector must reject «қайық»
    /// (boat) as a city candidate.
    #[test]
    fn common_noun_guard_rejects_qaiyq() {
        assert!(looks_like_common_noun_not_a_place("қайық"));
        assert!(looks_like_common_noun_not_a_place("қайықта"));
        assert!(looks_like_common_noun_not_a_place("қайыққа"));
    }

    #[test]
    fn common_noun_guard_rejects_yenq() {
        // «ең» = "most" (particle).  Live REPL: «Ең екен, түсіндім».
        assert!(looks_like_common_noun_not_a_place("ең"));
        assert!(looks_like_common_noun_not_a_place("еңде"));
    }

    #[test]
    fn common_noun_guard_rejects_pronouns() {
        // «қай» / «қашан» / «не» / «кім» — these are interrogatives
        // / pronouns that the FST sometimes parses as nouns under
        // Whisper drift.
        assert!(looks_like_common_noun_not_a_place("қай"));
        assert!(looks_like_common_noun_not_a_place("қандай"));
        assert!(looks_like_common_noun_not_a_place("не"));
        assert!(looks_like_common_noun_not_a_place("кім"));
    }

    #[test]
    fn common_noun_guard_accepts_real_cities() {
        // Real cities in the geo registry must pass through
        // (short-circuit at top of the function).
        assert!(!looks_like_common_noun_not_a_place("Алматы"));
        assert!(!looks_like_common_noun_not_a_place("Қостанай"));
        assert!(!looks_like_common_noun_not_a_place("Астана"));
        assert!(!looks_like_common_noun_not_a_place("қостанай"));
    }

    #[test]
    fn common_noun_guard_does_not_false_positive_on_prefix_match() {
        // «суыл» starts with «су» but is NOT the common noun «су»
        // (water).  Without the suffix gate, the prefix-match would
        // incorrectly flag it.  The accepted suffixes are case
        // suffixes only.
        assert!(!looks_like_common_noun_not_a_place("суыл"));
        assert!(!looks_like_common_noun_not_a_place("ауыр"));
    }

    /// **Phase 26.B (2026-06-04 evening)** — Whisper-drift compressions
    /// of «қайда» / «қай жерде» (interrogative "where") that the rc10
    /// blocklist missed.  Live REPL: «Мен қажерде тұрамын» →
    /// adam said «Қажер екен, түсіндім» — exactly the kind of phantom
    /// city acknowledgement that slot validation is meant to prevent.
    #[test]
    fn common_noun_guard_rejects_qazher_drift() {
        assert!(looks_like_common_noun_not_a_place("қажер"));
        assert!(looks_like_common_noun_not_a_place("қажерде"));
        // Same family — «қайда» itself in case-inflected form:
        assert!(looks_like_common_noun_not_a_place("қайда"));
        assert!(looks_like_common_noun_not_a_place("қайдан"));
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
