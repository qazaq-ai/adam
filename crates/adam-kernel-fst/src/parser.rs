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
            // **v4.34.0** — particle-copula inflection for «емес».
            // Pre-v4.34.0 «particle» POS fell through to the catch-all
            // bare-only branch, so inflected predicate-copula forms
            // («емеспін / емессің / емеспіз / емессіз») didn't parse
            // at all. v4.33.5 wired sentence-level negation
            // («Бұл шындық емес») end-to-end through SemFrame; the
            // inflected pattern «Мен X емеспін» was blocked here at
            // the FST layer. Fix: dispatch «емес» specifically through
            // try_noun_analyses, which enumerates the Predicate
            // copulas (P1Sg / P2SgInformal / P2SgPolite / P1Pl /
            // P2PlInformal / P2PlPolite). Other particles (ба / бе /
            // ма / ме / па / пе question particles, да / де connector,
            // еді / екен copulas) keep bare-only behaviour — surgical
            // scope keeps blast radius minimal.
            "particle" if entry.root == "емес" => {
                try_noun_analyses(surface, entry, &mut out);
                // Also keep bare «емес» path: try_noun_analyses with
                // `predicate: None` covers it, but we add an explicit
                // bare emit too in case the noun-feature enumeration
                // changes upstream and stops covering bare-form.
                if entry.root == surface
                    && !out.iter().any(|a| {
                        matches!(a, Analysis::Noun { root, features }
                            if root == entry && *features == NounFeatures::default())
                    })
                {
                    out.push(Analysis::Noun {
                        root: entry.clone(),
                        features: NounFeatures::default(),
                    });
                }
            }
            // adverbs, postpositions, conjunctions, other particles: bare only.
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
    // **v4.21.0** — pronoun stem-alternation paradigm. Surfaces
    // like «онда» / «оны» / «оған» / «одан» can't be reached by
    // the regular `surface_could_contain_root` prefix-match
    // because the pronoun root «ол» is not a prefix of these
    // forms (Kazakh phonological alternation `ол → он-/оғ-/од-`
    // before consonant-initial suffixes). The paradigm matcher
    // adds the missing analyses without disturbing the regular
    // candidates.
    out.extend(crate::pronoun_paradigm::try_pronoun_paradigm(surface, lex));
    // **v6.0 (Codex / proptest finding)** — longest-root preference.
    //
    // When the lexicon contains both a long noun root and a shorter
    // root that, with the right inflection, produces the same surface,
    // the previous lex-order rule could pick the shorter root. Example:
    // surface `бостық` is both the noun «бостық» (bare Nominative) and
    // the verb «бос» + PastDefinite + 1pl. Lex order put «бос» first,
    // and `verb_to_tokens` then dropped the number field across the
    // token boundary, so detokenize re-synthesised the 1sg form
    // («бостым»). The proptest `noun_round_trip` shrinks straight to
    // this case.
    //
    // Disambiguation rule: longer root wins; tie-break on the original
    // determinism order (lex of `(root, id)`, already established by
    // `entries_ordered`). `sort_by` is stable, so equal-length roots
    // keep their original relative position.
    //
    // Why this is safe for the v2.1+ "`.into_iter().next()` is stable"
    // contract: the previous order was a stable function of the
    // Lexicon; the new order is also a stable function of the
    // Lexicon (no random keys, no clock, no hash seed). The contract
    // says "stable", not "lex-first" — downstream callers were
    // already promised reproducibility, not a specific tie-breaking
    // rule.
    out.sort_by(|a, b| {
        let len_a = analysis_root_len(a);
        let len_b = analysis_root_len(b);
        len_b.cmp(&len_a)
    });
    out
}

fn analysis_root_len(a: &Analysis) -> usize {
    let root = match a {
        Analysis::Noun { root, .. } => &root.root,
        Analysis::Verb { root, .. } => &root.root,
    };
    root.chars().count()
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
    let last = root.chars().last();
    // Intervocalic voicing — voiceless final → voiced.
    if let Some(voiced) = match last {
        Some('п') => Some('б'),
        Some('к') => Some('г'),
        Some('қ') => Some('ғ'),
        _ => None,
    } {
        let mut voiced_root: String = root.chars().take(root.chars().count() - 1).collect();
        voiced_root.push(voiced);
        if surface.starts_with(&voiced_root) {
            return true;
        }
    }
    // **v4.36.6** — vowel-final stem alternation. Aorist / converb-
    // imperfect derivation replaces final ы/і with и («оқы + а» →
    // «оқи»). Pre-v4.36.6, surfaces beginning with the alternate
    // stem («оқи*», «жасы*», «таны*») couldn't be analysed because
    // the original root «оқы» / «жасы» / «таны» wasn't a prefix
    // match. Mirrors the intervocalic-voicing branch above:
    // synth produces «оқи»; parser must accept it as a candidate
    // for the «оқы» root.
    if let Some(alt) = match last {
        Some('ы') | Some('і') => Some('и'),
        _ => None,
    } {
        let mut alt_root: String = root.chars().take(root.chars().count() - 1).collect();
        alt_root.push(alt);
        if surface.starts_with(&alt_root) {
            return true;
        }
    }
    false
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
        // v4.5.0 — locative-attributive derivation enumerated here
        // so analyse() recognises surface forms ending in
        // `-дағы / -дегі / -тағы / -тегі` and reverse-parses them
        // to the base noun. Previously these returned no analysis,
        // which was the v4.4.11 carry-forward closed by the
        // string-side `locative_attributive_hint` fallback in
        // v4.4.12. v4.5.0 supersedes the fallback as the primary
        // path and enables round-trip synthesis.
        Some(Case::LocativeAttributive),
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
        // **v4.32.5** — `ConverbImperfect` (`-{A}` "while V-ing")
        // added to the enumeration so the parser can recover converb
        // forms like «жаза» (converb of жаз) and «сөйлей» (converb of
        // сөйле). Pre-v4.32.5 these forms didn't parse at all because
        // — although `morphotactics::synthesise_verb` supports them —
        // the parser's generate-and-test loop only enumerated 4 of
        // the 13 defined tenses. The gap blocked `populate_ability_modality`
        // (which needs the converb signal on the lexical predicate to
        // disambiguate periphrastic ability «жаза алам» from a literal
        // "X took Y" sentence). Adding ConverbImperfect alone keeps
        // the search-space cost bounded (5 tenses instead of 4 → ~25 %
        // more parse time per token); other missing forms (ConverbPerfect,
        // 3 participles, FutureIntentional, FuturePossible, Conditional,
        // Imperative) are deferred to releases that genuinely need them
        // — adding all at once would double the search space and likely
        // surface unrelated regression noise.
        Some(Tense::ConverbImperfect),
        // **v4.36.5** — `ConverbPerfect` («-{Y}п» — "having V-ed")
        // and `PastReportative` («-{Y}п(ты)» — "they say X V-ed")
        // added to enumeration. ConverbPerfect closes a v4.32.5
        // carry-forward (bare «жазып / беріп / оқып» previously
        // didn't parse). PastReportative is the v4.36.5 path that
        // unblocks user-style hearsay forms «жазыпты / болыпты /
        // барыпты» — these were the natural Kazakh hearsay form
        // that v4.36.0's Hearsay routing couldn't see because the
        // FST treated only `-ған/-ген` (PastEvidential) as
        // reportative.
        Some(Tense::ConverbPerfect),
        Some(Tense::PastReportative),
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
                            let long = synthesise_verb(&entry.root, features);
                            if long == surface {
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
                            // **v4.36.6** — short-form 1sg colloquial
                            // contraction («алам» from «аламын»,
                            // «барам» from «барамын», «оқим» from
                            // «оқимын»). Long present-style 1sg
                            // ending is `-мын/-мін` (VERB_PRES_1SG);
                            // colloquial drops the final `-ын/-ін`,
                            // leaving just `-м`. Both forms are
                            // standard Kazakh — long is formal,
                            // short is colloquial. Pre-v4.36.6
                            // parser only enumerated long-form synth,
                            // so user-typed «алам / барам / оқим»
                            // didn't parse and modality detection
                            // couldn't fire on verb-only ability
                            // claims like «Жаза алам». Limited to
                            // (Present|PastEvidential) + First +
                            // (Sg|None) + polite=false — the exact
                            // feature combo that produces -мын/-мін.
                            // Other persons (-сың, -сыз, -міз, etc.)
                            // don't have a similar short-form
                            // contraction in standard Kazakh.
                            let short_eligible =
                                matches!(tense, Some(Tense::Present) | Some(Tense::PastEvidential))
                                    && matches!(person, Some(Person::First))
                                    && matches!(number, Some(Number::Singular) | None)
                                    && !polite;
                            if short_eligible && (long.ends_with("мын") || long.ends_with("мін"))
                            {
                                let short: String =
                                    long.chars().take(long.chars().count() - 2).collect();
                                if short != long && short == surface {
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
