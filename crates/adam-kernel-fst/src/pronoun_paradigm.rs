//! Irregular pronoun paradigms — hardcoded surface → analysis table
//! for the closed class of Kazakh pronouns whose oblique forms
//! involve stem alternation that the regular `synthesise_noun`
//! pipeline cannot generate.
//!
//! **v4.21.0 — pronoun stem-alternation foundation.** The
//! v4.20.5 eval surfaced that the FST `analyse()` returned no
//! parse with `root=ол` for «онда»: the surface starts with «он»,
//! not «ол», so the prefix-match in `surface_could_contain_root`
//! never considers `ол` as a candidate. Kazakh's core pronouns
//! exhibit a regular phonological alternation — `ол → он-` (with
//! lateralisation `л → н` before consonant-initial dental
//! suffixes), `ол → оғ-` (before the dative `-ан`), `ол → од-`
//! (before the ablative `-ан`) — that the otherwise-uniform
//! morphotactics doesn't capture. Adding allomorph rules to the
//! general phonology engine would over-trigger on common
//! consonant-final nouns (ел → *ен-, бел → *бен-); the linguistic
//! reality is that this is a **closed-class irregularity** of the
//! demonstrative-pronoun paradigm.
//!
//! v4.21.0 ships the irregularity as exactly that: a small
//! hardcoded table of `(surface, bare-root-id, NounFeatures)`
//! triples, consulted by `analyse()` after the regular noun /
//! verb passes. When a pronoun's bare root is in the lexicon
//! (e.g. `pron_ol → "ол"`), the corresponding `RootEntry` is
//! cloned from the lexicon and emitted with the documented
//! features.
//!
//! **Scope.**
//! - **v4.21.0** — `ол` (he/she/it/that) — the empirical target.
//! - **v4.21.5** — extends to the rest of the demonstrative /
//!   personal closed class:
//!   - `бұл` (this) — 6 oblique cases (бұны / бұның / бұған /
//!     бұнда / бұдан / бұнымен).
//!   - `сол` (that) — 6 oblique cases (соны / соның / соған /
//!     сонда / содан / сонымен).
//!   - `мен` (I) — only Dative is irregular (маған, due to
//!     `мен → ма-` stem alternation). The regular cases (мені /
//!     менің / менде / менен) round-trip through the standard
//!     synth pipeline.
//!   - `сен` (you-informal) — only Dative irregular (саған, same
//!     pattern as мен).
//! - **v4.22.0+** — broader FST irregularity catalog (мынау /
//!   анау demonstratives, irregular noun stems, possessive-stem
//!   alternations, voicing edge cases).

use crate::lexicon::LexiconV1;
use crate::morphotactics::{Case, NounFeatures, Number};
use crate::parser::Analysis;

/// One entry in the pronoun-paradigm table: a fully-inflected
/// surface form and the underlying bare-root + feature bundle.
struct PronounForm {
    /// Surface form, lowercase, no leading whitespace.
    surface: &'static str,
    /// Bare-root surface as stored in the lexicon (e.g. `"ол"`).
    /// Looked up at runtime via `LexiconV1::find_by_root`; the
    /// matching `RootEntry` is cloned into the emitted `Analysis`
    /// so downstream consumers see a canonical pronoun root.
    root: &'static str,
    /// Case the surface form encodes.
    case: Case,
    /// Number — for pronouns this is always `Some(Number::Singular)`
    /// in v4.21.0 (plurals like `олар + Loc → оларда` already work
    /// through the regular path because `олар` is a separate
    /// lexicon entry without alternation).
    number: Option<Number>,
}

/// **v4.21.0 / v4.21.5** — hand-curated table for irregular
/// pronoun forms. Forms with regular synthesis (e.g. nominative
/// bare `ол` / `бұл` / `сол` / `мен` / `сен`, or regular oblique
/// cases of мен / сен like мені / менің / менде / менен / сені /
/// сенің / сенде / сенен) are NOT listed here — they're handled
/// by the standard `try_noun_analyses` path. Only the irregular-
/// stem oblique cases need a dedicated entry.
///
/// Forms are ordered: pronoun group (ол → бұл → сол → мен →
/// сен), then within each group by case (Acc / Gen / Dat / Loc /
/// Abl / Inst). Order is purely cosmetic — the matcher walks the
/// full table per call and emits matching entries; the aggregate
/// `analyse()` re-sorts deterministically.
const PRONOUN_FORMS: &[PronounForm] = &[
    // ───── ол (he/she/it/that) — v4.21.0 ─────────────────────
    PronounForm {
        surface: "оны",
        root: "ол",
        case: Case::Accusative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "оның",
        root: "ол",
        case: Case::Genitive,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "оған",
        root: "ол",
        case: Case::Dative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "онда",
        root: "ол",
        case: Case::Locative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "одан",
        root: "ол",
        case: Case::Ablative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "онымен",
        root: "ол",
        case: Case::Instrumental,
        number: Some(Number::Singular),
    },
    // ───── бұл (this) — v4.21.5 ──────────────────────────────
    PronounForm {
        surface: "бұны",
        root: "бұл",
        case: Case::Accusative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "бұның",
        root: "бұл",
        case: Case::Genitive,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "бұған",
        root: "бұл",
        case: Case::Dative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "бұнда",
        root: "бұл",
        case: Case::Locative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "бұдан",
        root: "бұл",
        case: Case::Ablative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "бұнымен",
        root: "бұл",
        case: Case::Instrumental,
        number: Some(Number::Singular),
    },
    // ───── сол (that) — v4.21.5 ──────────────────────────────
    PronounForm {
        surface: "соны",
        root: "сол",
        case: Case::Accusative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "соның",
        root: "сол",
        case: Case::Genitive,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "соған",
        root: "сол",
        case: Case::Dative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "сонда",
        root: "сол",
        case: Case::Locative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "содан",
        root: "сол",
        case: Case::Ablative,
        number: Some(Number::Singular),
    },
    PronounForm {
        surface: "сонымен",
        root: "сол",
        case: Case::Instrumental,
        number: Some(Number::Singular),
    },
    // ───── мен (I) — v4.21.5 — only Dative irregular ────────
    PronounForm {
        surface: "маған",
        root: "мен",
        case: Case::Dative,
        number: Some(Number::Singular),
    },
    // ───── сен (you-informal) — v4.21.5 — only Dative ───────
    PronounForm {
        surface: "саған",
        root: "сен",
        case: Case::Dative,
        number: Some(Number::Singular),
    },
];

fn all_forms() -> &'static [PronounForm] {
    PRONOUN_FORMS
}

/// **2026-05-17 / v6.0 hook** — public read-only view of the
/// hardcoded irregular-pronoun surfaces. Used by
/// `adam-corpus::mine_lexicon_gaps` to mark these surfaces as
/// "covered" without re-running the FST analyser on every token
/// (Pass-1 of the gap miner is prefix-match; the irregulars are
/// the textbook case it can't catch). Keep this in sync with
/// `PRONOUN_FORMS` above — only the `.surface` field is exposed.
pub fn irregular_pronoun_surfaces() -> impl Iterator<Item = &'static str> {
    PRONOUN_FORMS.iter().map(|f| f.surface)
}

/// **v4.21.0** — try matching the surface form against the
/// hardcoded irregular pronoun paradigms. Returns every matching
/// `Analysis::Noun` whose surface equals `surface` (typically
/// 0 or 1). Designed to be called after `try_noun_analyses` /
/// `try_verb_analyses` so it strictly *adds* candidates that the
/// regular pipeline cannot generate.
///
/// Output ordering is the table order; callers don't depend on
/// inter-table ordering since matches are typically a single
/// entry. The aggregate `analyse()` then re-sorts by lexicon
/// `(root, id)` so the final candidate list is deterministic.
pub fn try_pronoun_paradigm(surface: &str, lex: &LexiconV1) -> Vec<Analysis> {
    let mut out = Vec::new();
    for form in all_forms() {
        if form.surface != surface {
            continue;
        }
        let Some(root_entry) = lex.get(form.root) else {
            // Lexicon is missing the bare pronoun — defensive,
            // shouldn't happen in production with the curated
            // pure_kazakh_roots.json.
            continue;
        };
        let features = NounFeatures {
            derivation: None,
            number: form.number,
            possessive: None,
            case: Some(form.case),
            predicate: None,
        };
        out.push(Analysis::Noun {
            root: root_entry.clone(),
            features,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::{LexiconV1, RootEntry};
    use std::collections::HashMap;

    fn mini_lex() -> LexiconV1 {
        // Hand-rolled lexicon containing the closed-class
        // pronouns covered by v4.21.0 / v4.21.5 so the tests
        // don't depend on the full curated file.
        let entries = [
            ("pron_ol", "ол", "back"),
            ("pron_bul", "бұл", "back"),
            ("pron_sol", "сол", "back"),
            ("pron_men", "мен", "front"),
            ("pron_sen", "сен", "front"),
        ];
        let mut by_surface = HashMap::new();
        let mut entries_ordered = Vec::new();
        for (id, root, harmony) in entries {
            let e = RootEntry {
                id: id.to_string(),
                root: root.to_string(),
                part_of_speech: "pronoun".to_string(),
                vowel_harmony: harmony.to_string(),
                final_sound_class: "voiced_consonant".to_string(),
            };
            by_surface.insert(e.root.clone(), e.clone());
            entries_ordered.push(e);
        }
        LexiconV1 {
            by_surface,
            entries_ordered,
            curated_count: 5,
            apertium_count: 0,
        }
    }

    fn empty_lex() -> LexiconV1 {
        LexiconV1 {
            by_surface: HashMap::new(),
            entries_ordered: Vec::new(),
            curated_count: 0,
            apertium_count: 0,
        }
    }

    #[test]
    fn onda_resolves_to_ol_locative() {
        let lex = mini_lex();
        let analyses = try_pronoun_paradigm("онда", &lex);
        assert_eq!(analyses.len(), 1, "expected exactly one paradigm match");
        match &analyses[0] {
            Analysis::Noun { root, features } => {
                assert_eq!(root.root, "ол");
                assert_eq!(features.case, Some(Case::Locative));
                assert_eq!(features.number, Some(Number::Singular));
            }
            other => panic!("expected Noun analysis, got {other:?}"),
        }
    }

    #[test]
    fn ony_resolves_to_ol_accusative() {
        let lex = mini_lex();
        let analyses = try_pronoun_paradigm("оны", &lex);
        assert_eq!(analyses.len(), 1);
        match &analyses[0] {
            Analysis::Noun { root, features } => {
                assert_eq!(root.root, "ол");
                assert_eq!(features.case, Some(Case::Accusative));
            }
            other => panic!("expected Noun analysis, got {other:?}"),
        }
    }

    #[test]
    fn ogan_resolves_to_ol_dative() {
        let lex = mini_lex();
        let analyses = try_pronoun_paradigm("оған", &lex);
        assert_eq!(analyses.len(), 1);
        match &analyses[0] {
            Analysis::Noun { features, .. } => {
                assert_eq!(features.case, Some(Case::Dative));
            }
            other => panic!("expected Noun analysis, got {other:?}"),
        }
    }

    #[test]
    fn unrelated_surface_returns_empty() {
        let lex = mini_lex();
        assert!(try_pronoun_paradigm("кітап", &lex).is_empty());
        assert!(try_pronoun_paradigm("он", &lex).is_empty());
    }

    #[test]
    fn missing_lexicon_root_returns_empty_gracefully() {
        // Empty lexicon — the bare pronoun isn't found, so the
        // paradigm match silently degrades instead of panicking.
        let lex = empty_lex();
        assert!(try_pronoun_paradigm("онда", &lex).is_empty());
    }

    // ───── v4.21.5 — extended paradigm coverage ────────────────

    #[test]
    fn bunda_resolves_to_bul_locative() {
        let lex = mini_lex();
        let analyses = try_pronoun_paradigm("бұнда", &lex);
        assert_eq!(analyses.len(), 1);
        match &analyses[0] {
            Analysis::Noun { root, features } => {
                assert_eq!(root.root, "бұл");
                assert_eq!(features.case, Some(Case::Locative));
            }
            other => panic!("expected Noun analysis, got {other:?}"),
        }
    }

    #[test]
    fn sogan_resolves_to_sol_dative() {
        let lex = mini_lex();
        let analyses = try_pronoun_paradigm("соған", &lex);
        assert_eq!(analyses.len(), 1);
        match &analyses[0] {
            Analysis::Noun { root, features } => {
                assert_eq!(root.root, "сол");
                assert_eq!(features.case, Some(Case::Dative));
            }
            other => panic!("expected Noun analysis, got {other:?}"),
        }
    }

    #[test]
    fn magan_resolves_to_men_dative() {
        let lex = mini_lex();
        let analyses = try_pronoun_paradigm("маған", &lex);
        assert_eq!(analyses.len(), 1);
        match &analyses[0] {
            Analysis::Noun { root, features } => {
                assert_eq!(root.root, "мен");
                assert_eq!(features.case, Some(Case::Dative));
            }
            other => panic!("expected Noun analysis, got {other:?}"),
        }
    }

    #[test]
    fn sagan_resolves_to_sen_dative() {
        let lex = mini_lex();
        let analyses = try_pronoun_paradigm("саған", &lex);
        assert_eq!(analyses.len(), 1);
        match &analyses[0] {
            Analysis::Noun { root, features } => {
                assert_eq!(root.root, "сен");
                assert_eq!(features.case, Some(Case::Dative));
            }
            other => panic!("expected Noun analysis, got {other:?}"),
        }
    }
}
