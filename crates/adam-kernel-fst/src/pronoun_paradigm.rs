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
//! **Scope (v4.21.0).** Only `ол` (he/she/it/that) — the
//! empirical target. Documented but-not-implemented allomorphs
//! for `бұл`, `сол`, `мен`, `сен`, `мынау`, `анау` defer to
//! v4.21.5+.

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

/// **v4.21.0** — hand-curated table for the pronoun `ол`. Forms
/// with regular synthesis (e.g. nominative bare `ол`) are not
/// listed here — they're already handled by the standard
/// `try_noun_analyses` path. Only the irregular-stem oblique
/// cases need a dedicated entry.
const PRONOUN_OL_FORMS: &[PronounForm] = &[
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
];

/// Concatenation of all paradigm tables. v4.21.0 = `ол` only;
/// future patches extend with `бұл`, `сол`, `мен`, `сен`,
/// `мынау`, `анау`.
fn all_forms() -> &'static [PronounForm] {
    PRONOUN_OL_FORMS
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
        // Hand-rolled lexicon containing just `ол` so the tests
        // don't depend on the full curated file.
        let entry = RootEntry {
            id: "pron_ol".to_string(),
            root: "ол".to_string(),
            part_of_speech: "pronoun".to_string(),
            vowel_harmony: "back".to_string(),
            final_sound_class: "voiced_consonant".to_string(),
        };
        let mut by_surface = HashMap::new();
        by_surface.insert(entry.root.clone(), entry.clone());
        LexiconV1 {
            by_surface,
            entries_ordered: vec![entry],
            curated_count: 1,
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
}
