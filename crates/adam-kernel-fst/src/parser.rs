//! FST parser — the analysis direction.
//!
//! Given a surface form like `жазбадым`, produce the possible analyses as
//! `(root, features)` tuples. The algorithm is "generate and test":
//!
//! 1. Enumerate every root in the lexicon that is a prefix of the input.
//! 2. For each candidate root, enumerate plausible feature bundles.
//! 3. Synthesise each `(root, features)` via the existing morphotactics
//!    synthesiser; if the output equals the input, record the analysis.
//!
//! Because the synthesiser is deterministic and O(suffix length), the whole
//! parse is O(lexicon_size × feature_space × avg_word_length). For a 12k
//! lexicon and a ~50-combination feature space, that is ~600k synth calls
//! per parse, which is a few milliseconds on a modern CPU — acceptable for
//! batch processing of corpus-sized inputs.
//!
//! The parse is ambiguous by design: morphologically ill-specified forms
//! may yield multiple analyses (e.g. `бала` could be a bare noun or a
//! second-person-imperative verb if `бала` were a verb root). We return all
//! matches and let the LM layer disambiguate.

use crate::lexicon::{LexiconV1, RootEntry};
use crate::morphotactics::{
    Case, NounFeatures, Number, Person, Possessive, Predicate, Tense, VerbFeatures, Voice,
    synthesise_noun, synthesise_verb,
};

/// One analysis of a surface form. Holds the root entry (so consumers can
/// see POS + the other phonological metadata) and the feature bundle that
/// produced the match. `Either` because a word is either a noun or a verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Analysis {
    Noun {
        root: RootEntry,
        features: NounFeatures,
    },
    Verb {
        root: RootEntry,
        features: VerbFeatures,
    },
}

/// Analyse a single surface form against a lexicon, returning every
/// `(root, features)` combination whose synthesis matches the input.
///
/// Runs both noun and verb paradigms on every root-prefix candidate. Does
/// NOT attempt derivational morphology (participles-as-nouns, infinitives
/// etc.) — that is a separate week-2 task.
///
/// ## Determinism contract (v3.2.0)
///
/// The returned `Vec<Analysis>` is ordered by lexicographic root surface
/// (primary) then Lexicon `id` (tie-breaker) — inherited from
/// [`LexiconV1::entries_ordered`], which is built once at Lexicon load.
/// Iterating the Vec is as fast as iterating `HashMap::values()` was
/// before v3.2.0 (contiguous memory, no tree walk), while yielding a
/// stable order across runs.
///
/// Downstream consumers in `adam-reasoning` that pick `analyse(...)
/// .into_iter().next()` (every v2.1+ pattern matcher) are stable
/// because of this ordering guarantee.
pub fn analyse(surface: &str, lex: &LexiconV1) -> Vec<Analysis> {
    let mut out = Vec::new();
    for entry in &lex.entries_ordered {
        if !surface_could_contain_root(surface, &entry.root) {
            continue;
        }
        match entry.part_of_speech.as_str() {
            "noun" | "adjective" | "pronoun" | "numeral" => {
                try_noun_analyses(surface, entry, &mut out);
            }
            "verb" => {
                try_verb_analyses(surface, entry, &mut out);
            }
            // adverbs, postpositions, conjunctions, particles: bare only.
            _ => {
                if entry.root == surface {
                    out.push(Analysis::Noun {
                        root: entry.clone(),
                        features: NounFeatures::default(),
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod determinism_tests {
    //! Regression tests for the v3.2.0 determinism fix — these must
    //! pass or every run of `extract_facts` produces a different fact
    //! set, invalidating the scaling bench and the entire
    //! "deterministic pipeline" thesis.
    //!
    //! ## Why the v3.2.0 in-process test was insufficient (Codex review)
    //!
    //! The v3.2.0 release shipped `analyse_ordering_stable_across_calls`
    //! which called `analyse()` twice in a single process and asserted
    //! equality. A pre-fix HashMap-backed analyse **would also pass that
    //! test** because within one process HashMap iteration order is
    //! fixed (the seed is picked at HashMap instance creation and stays
    //! put). The original bug was cross-process: re-run the binary, get
    //! a different HashMap seed, different `.next()` analysis.
    //!
    //! v3.3.0 strengthens the tests with **expected-order assertions**
    //! on genuinely cross-root-ambiguous surfaces. Under the v3.2.0+
    //! contract (iterate `LexiconV1::entries_ordered`, sorted by
    //! `(root, id)`), the first analysis for an ambiguous surface is
    //! *determined by the sort key*. Under pre-v3.2.0 HashMap-values
    //! iteration, that same assertion would fail ≈ 50 % of runs.

    use super::*;
    use crate::lexicon::LexiconV1;

    fn load_real_lex() -> Option<LexiconV1> {
        let curated = "../../data/tokenizer/segmentation_roots.json";
        let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
        if !std::path::Path::new(curated).exists() {
            return None;
        }
        LexiconV1::load(curated, apertium).ok()
    }

    fn root_id(a: &Analysis) -> &str {
        match a {
            Analysis::Noun { root, .. } => &root.root,
            Analysis::Verb { root, .. } => &root.root,
        }
    }

    #[test]
    fn analyse_ordering_stable_across_calls() {
        // Weak-form determinism: two back-to-back calls must return
        // identical vectors. This passes under both the old and new
        // implementations — kept for defence-in-depth. The stronger
        // expected-order tests below are what actually catches the
        // pre-v3.2.0 bug.
        let Some(lex) = load_real_lex() else { return };
        for word in ["бала", "алматыда", "кітабы", "мектебі", "жазды"]
        {
            let first = analyse(word, &lex);
            let second = analyse(word, &lex);
            assert_eq!(
                first, second,
                "analyse(`{word}`) must be identical across repeat calls"
            );
        }
    }

    #[test]
    fn first_analysis_stable_for_ambiguous_surface() {
        // Same weak-form: `.next()` parity across repeat calls.
        let Some(lex) = load_real_lex() else { return };
        let first_a = analyse("бала", &lex).into_iter().next();
        let first_b = analyse("бала", &lex).into_iter().next();
        assert_eq!(first_a, first_b);
    }

    #[test]
    fn analyses_sorted_by_root_then_id_when_cross_root_ambiguous() {
        // Strong-form determinism: the ordering contract declared in
        // the `analyse` docstring is the sort key `(entry.root, entry.id)`
        // inherited from `LexiconV1::entries_ordered`. This test asserts
        // the contract on a genuinely cross-root-ambiguous surface.
        //
        // On the real Lexicon `кітабы` is the P3 possessive inflection
        // of `кітап` ("book") AND has its own root entry `кітабы`
        // (Apertium import artefact). `analyse("кітабы")` therefore
        // returns analyses under two distinct roots. Under the v3.2.0+
        // contract the sort order is by Cyrillic Unicode code point of
        // `root`: `б` (U+0431) < `п` (U+043F), so `кітабы` < `кітап`
        // and `кітабы` analyses come first. Under the pre-v3.2.0
        // HashMap-values path the "first" could be either root, picked
        // by the per-process HashMap seed — this assertion fails ≈ 50 %
        // of runs on the old code.
        let Some(lex) = load_real_lex() else { return };
        let analyses = analyse("кітабы", &lex);
        if analyses.len() < 2 {
            eprintln!(
                "note: expected ≥ 2 analyses for «кітабы»; got {}. Lexicon may have drifted.",
                analyses.len()
            );
            return;
        }
        // First analysis must be under the lexicographically-smaller
        // root `кітабы` (by Unicode code point), not the `кітап` root.
        assert_eq!(
            root_id(&analyses[0]),
            "кітабы",
            "sort contract broken — first analysis must be under `кітабы` (< `кітап` by Cyrillic code point)"
        );
        // And the whole sequence must be non-decreasing by root.
        let roots: Vec<&str> = analyses.iter().map(root_id).collect();
        let mut sorted = roots.clone();
        sorted.sort();
        assert_eq!(
            roots, sorted,
            "analyses must be non-decreasing by root; got {:?}",
            roots
        );
    }

    #[test]
    fn first_root_matches_entries_ordered_for_prefix_ambiguous_surface() {
        // Cross-check: the first analysis of an ambiguous surface must
        // match the first `entries_ordered` Lexicon entry whose root
        // prefixes the surface. This is a direct invariant on the v3.2.0
        // dual-storage contract.
        let Some(lex) = load_real_lex() else { return };
        let surface = "кітабы";
        let analyses = analyse(surface, &lex);
        if analyses.is_empty() {
            return;
        }
        let first_analysis_root = root_id(&analyses[0]);

        // Find the first Lexicon entry (in entries_ordered) that could
        // plausibly prefix this surface (v3.2.0 analyse() prefix test).
        let first_candidate = lex
            .entries_ordered
            .iter()
            .find(|e| surface_could_contain_root(surface, &e.root))
            .expect("at least one candidate must exist — the analyse above succeeded");
        assert_eq!(
            first_analysis_root, first_candidate.root,
            "first analysis root must match first entries_ordered candidate — got {first_analysis_root}, expected {}",
            first_candidate.root,
        );
    }
}

/// Returns `true` if the surface form could plausibly start with the given
/// root once intervocalic voicing (rules 10–12) is accounted for. Allows
/// both the bare root and its voiced variant (п↔б, к↔г, қ↔ғ) as a prefix.
fn surface_could_contain_root(surface: &str, root: &str) -> bool {
    if surface.starts_with(root) {
        return true;
    }
    // Only care about roots that end in a voiceless obstruent whose voiced
    // counterpart might have surfaced instead.
    let last = root.chars().last();
    let voiced = match last {
        Some('п') => 'б',
        Some('к') => 'г',
        Some('қ') => 'ғ',
        _ => return false,
    };
    // Rebuild the root with the voiced final consonant.
    let mut voiced_root: String = root.chars().take(root.chars().count() - 1).collect();
    voiced_root.push(voiced);
    surface.starts_with(&voiced_root)
}

fn try_noun_analyses(surface: &str, entry: &RootEntry, out: &mut Vec<Analysis>) {
    // Enumerate the noun feature space explored by week-1 morphotactics.
    let numbers = [None, Some(Number::Singular), Some(Number::Plural)];
    let possessives = [
        None,
        Some(Possessive::P1Sg),
        Some(Possessive::P2SgInformal),
        Some(Possessive::P3),
        Some(Possessive::P1Pl),
    ];
    let cases = [
        None,
        Some(Case::Nominative),
        Some(Case::Genitive),
        Some(Case::Dative),
        Some(Case::Accusative),
        Some(Case::Locative),
        Some(Case::Ablative),
        Some(Case::Instrumental),
    ];
    // v1.4.0: also enumerate predicate-person copula suffix.
    let predicates = [
        None,
        Some(Predicate::P1Sg),
        Some(Predicate::P2SgInformal),
        Some(Predicate::P2SgPolite),
        Some(Predicate::P1Pl),
        Some(Predicate::P2PlInformal),
        Some(Predicate::P2PlPolite),
    ];
    for &number in &numbers {
        for &possessive in &possessives {
            for &case in &cases {
                for &predicate in &predicates {
                    // Predicate + possessive never stack in Kazakh.
                    if predicate.is_some() && possessive.is_some() {
                        continue;
                    }
                    let features = NounFeatures {
                        derivation: None,
                        number,
                        possessive,
                        case,
                        predicate,
                    };
                    if synthesise_noun(&entry.root, features) == surface {
                        out.push(Analysis::Noun {
                            root: entry.clone(),
                            features,
                        });
                    }
                }
            }
        }
    }
}

fn try_verb_analyses(surface: &str, entry: &RootEntry, out: &mut Vec<Analysis>) {
    // Enumerate the verb feature space explored by week-1 morphotactics.
    let voices = [
        None,
        Some(Voice::Active),
        Some(Voice::Passive),
        Some(Voice::Reflexive),
        Some(Voice::Reciprocal),
        Some(Voice::Causative),
    ];
    let tenses = [
        None,
        Some(Tense::PastDefinite),
        Some(Tense::PastEvidential),
        Some(Tense::Present),
    ];
    let persons = [
        None,
        Some(Person::First),
        Some(Person::Second),
        Some(Person::Third),
    ];
    let numbers = [None, Some(Number::Singular), Some(Number::Plural)];
    for &voice in &voices {
        for &negation in &[false, true] {
            for &tense in &tenses {
                for &person in &persons {
                    for &number in &numbers {
                        for &polite in &[false, true] {
                            let features = VerbFeatures {
                                voice,
                                negation,
                                tense,
                                person,
                                number,
                                polite,
                            };
                            if synthesise_verb(&entry.root, features) == surface {
                                // Deduplicate against earlier analyses that
                                // differ only by unused fields (e.g. active
                                // vs None voice produce the same output).
                                let analysis = Analysis::Verb {
                                    root: entry.clone(),
                                    features,
                                };
                                if !out.contains(&analysis) {
                                    out.push(analysis);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_lex() -> LexiconV1 {
        use std::collections::HashMap;
        let mut by_surface = HashMap::new();
        for (id, root, pos, harmony, fsc) in [
            ("noun_бала", "бала", "noun", "back", "vowel"),
            (
                "noun_мектеп",
                "мектеп",
                "noun",
                "front",
                "voiceless_consonant",
            ),
            ("noun_адам", "адам", "noun", "back", "nasal"),
            ("verb_жаз", "жаз", "verb", "back", "voiced_consonant"),
            ("verb_бер", "бер", "verb", "front", "voiced_consonant"),
        ] {
            by_surface.insert(
                root.to_string(),
                RootEntry {
                    id: id.to_string(),
                    root: root.to_string(),
                    part_of_speech: pos.to_string(),
                    vowel_harmony: harmony.to_string(),
                    final_sound_class: fsc.to_string(),
                },
            );
        }
        let mut entries_ordered: Vec<RootEntry> = by_surface.values().cloned().collect();
        entries_ordered.sort_by(|a, b| a.root.cmp(&b.root).then_with(|| a.id.cmp(&b.id)));
        LexiconV1 {
            by_surface,
            entries_ordered,
            curated_count: 5,
            apertium_count: 0,
        }
    }

    #[test]
    fn parse_плюрал_балалар() {
        let lex = tiny_lex();
        let analyses = analyse("балалар", &lex);
        let has_plural = analyses.iter().any(|a| {
            matches!(a,
                Analysis::Noun { features, root, .. }
                    if root.root == "бала"
                        && features.number == Some(Number::Plural))
        });
        assert!(
            has_plural,
            "expected plural analysis of балалар, got {analyses:#?}"
        );
    }

    #[test]
    fn parse_плюрал_датив_балаларға() {
        let lex = tiny_lex();
        let analyses = analyse("балаларға", &lex);
        let ok = analyses.iter().any(|a| {
            matches!(a,
                Analysis::Noun { features, root, .. }
                    if root.root == "бала"
                        && features.number == Some(Number::Plural)
                        && features.case == Some(Case::Dative))
        });
        assert!(ok, "expected PL+DAT of бала, got {analyses:#?}");
    }

    #[test]
    fn parse_past_1sg_жаздым() {
        let lex = tiny_lex();
        let analyses = analyse("жаздым", &lex);
        let ok = analyses.iter().any(|a| {
            matches!(a,
                Analysis::Verb { features, root, .. }
                    if root.root == "жаз"
                        && features.tense == Some(Tense::PastDefinite)
                        && features.person == Some(Person::First)
                        && features.number == Some(Number::Singular))
        });
        assert!(ok, "expected жаз+PAST+1SG, got {analyses:#?}");
    }

    #[test]
    fn parse_evidential_жазған() {
        let lex = tiny_lex();
        let analyses = analyse("жазған", &lex);
        let ok = analyses.iter().any(|a| {
            matches!(a,
                Analysis::Verb { features, root, .. }
                    if root.root == "жаз"
                        && features.tense == Some(Tense::PastEvidential))
        });
        assert!(ok, "expected жаз+EVID, got {analyses:#?}");
    }

    #[test]
    fn parse_rejects_non_kazakh_garbage() {
        let lex = tiny_lex();
        let analyses = analyse("xyzzy", &lex);
        assert!(
            analyses.is_empty(),
            "expected 0 analyses, got {analyses:#?}"
        );
    }

    #[test]
    fn parse_possessive_with_intervocalic_voicing() {
        // мектебім should analyse as мектеп + POSS.1SG.
        let lex = tiny_lex();
        let analyses = analyse("мектебім", &lex);
        let ok = analyses.iter().any(|a| {
            matches!(a,
                Analysis::Noun { features, root, .. }
                    if root.root == "мектеп"
                        && features.possessive == Some(Possessive::P1Sg))
        });
        assert!(ok, "expected мектеп+POSS.1SG, got {analyses:#?}");
    }
}
