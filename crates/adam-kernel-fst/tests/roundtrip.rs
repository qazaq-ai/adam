//! Integration test — synthesise → analyse roundtrip against the real
//! 12 k-entry v1.0.0 lexicon.
//!
//! We sample a feature bundle, synthesise every lexicon entry of the
//! matching POS into that form, then analyse the resulting string and
//! check that the original `(root, features)` appears in the analyses.
//!
//! Success criterion: ≥ 90 % roundtrip rate. Failures below that threshold
//! point at an undocumented stem irregularity or a missing rule in the
//! phonology module; printing them gives a concrete TODO list for week 2.

use adam_kernel_fst::lexicon::{LexiconV1, RootEntry};
use adam_kernel_fst::morphotactics::{
    Case, NounFeatures, Number, Person, Possessive, Tense, VerbFeatures, synthesise_noun,
    synthesise_verb,
};
use adam_kernel_fst::parser::{Analysis, analyse};

fn load_lexicon() -> Option<LexiconV1> {
    // Paths relative to the crate directory (where `cargo test` runs).
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !std::path::Path::new(curated).exists() || !std::path::Path::new(apertium).exists() {
        eprintln!("lexicon files not present — skipping integration test");
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

/// Run a roundtrip check: for every lexicon entry matching `pos`, synthesise
/// the specified features, analyse the result, and look for the original
/// entry in the analyses. Returns `(successes, total, failing_examples)`.
fn roundtrip_nouns(
    lex: &LexiconV1,
    features: NounFeatures,
) -> (usize, usize, Vec<(String, String)>) {
    let mut successes = 0usize;
    let mut total = 0usize;
    let mut fails: Vec<(String, String)> = Vec::new();
    for entry in lex.by_surface.values() {
        if !matches!(
            entry.part_of_speech.as_str(),
            "noun" | "adjective" | "pronoun" | "numeral"
        ) {
            continue;
        }
        total += 1;
        let synth = synthesise_noun(&entry.root, features);
        let analyses = analyse(&synth, lex);
        let ok = analyses.iter().any(|a| matches_noun_analysis(a, entry, features));
        if ok {
            successes += 1;
        } else if fails.len() < 20 {
            fails.push((entry.root.clone(), synth));
        }
    }
    (successes, total, fails)
}

fn matches_noun_analysis(a: &Analysis, expected_root: &RootEntry, expected: NounFeatures) -> bool {
    matches!(a,
        Analysis::Noun { root, features }
            if root.root == expected_root.root
                && features.number == expected.number
                && features.possessive == expected.possessive
                && features.case == expected.case)
}

fn roundtrip_verbs(
    lex: &LexiconV1,
    features: VerbFeatures,
) -> (usize, usize, Vec<(String, String)>) {
    let mut successes = 0usize;
    let mut total = 0usize;
    let mut fails: Vec<(String, String)> = Vec::new();
    for entry in lex.by_surface.values() {
        if entry.part_of_speech != "verb" {
            continue;
        }
        total += 1;
        let synth = synthesise_verb(&entry.root, features);
        let analyses = analyse(&synth, lex);
        let ok = analyses.iter().any(|a| matches_verb_analysis(a, entry, features));
        if ok {
            successes += 1;
        } else if fails.len() < 20 {
            fails.push((entry.root.clone(), synth));
        }
    }
    (successes, total, fails)
}

fn matches_verb_analysis(a: &Analysis, expected_root: &RootEntry, expected: VerbFeatures) -> bool {
    matches!(a,
        Analysis::Verb { root, features }
            if root.root == expected_root.root
                && features.tense == expected.tense
                && features.person == expected.person
                && features.number == expected.number
                && features.negation == expected.negation
                && features.polite == expected.polite
                && features.voice == expected.voice)
}

/// Fast smoke test — 100 lexicon entries × plural roundtrip. Runs in CI.
#[test]
fn roundtrip_noun_plural_smoke_100() {
    let Some(lex) = load_lexicon() else {
        return;
    };
    let features = NounFeatures {
        number: Some(Number::Plural),
        ..Default::default()
    };
    let mut successes = 0usize;
    let mut total = 0usize;
    for entry in lex
        .by_surface
        .values()
        .filter(|e| {
            matches!(
                e.part_of_speech.as_str(),
                "noun" | "adjective" | "pronoun" | "numeral"
            )
        })
        .take(100)
    {
        total += 1;
        let synth = synthesise_noun(&entry.root, features);
        let analyses = analyse(&synth, &lex);
        if analyses.iter().any(|a| matches_noun_analysis(a, entry, features)) {
            successes += 1;
        }
    }
    let rate = successes as f64 / total.max(1) as f64;
    eprintln!("smoke100 noun+PL: {successes}/{total} = {:.1}%", rate * 100.0);
    assert!(rate >= 0.95, "smoke roundtrip < 95%: {:.1}%", rate * 100.0);
}

#[test]
#[ignore = "slow (~30s); run manually with `cargo test --test roundtrip -- --ignored`"]
fn roundtrip_noun_plural() {
    let Some(lex) = load_lexicon() else {
        return;
    };
    let features = NounFeatures {
        number: Some(Number::Plural),
        ..Default::default()
    };
    let (ok, total, fails) = roundtrip_nouns(&lex, features);
    let rate = ok as f64 / total.max(1) as f64;
    eprintln!("noun+PL roundtrip: {ok}/{total} = {:.1}%", rate * 100.0);
    for (root, synth) in fails.iter().take(5) {
        eprintln!("  fail: {root} → {synth}");
    }
    assert!(
        rate >= 0.90,
        "noun plural roundtrip < 90% ({:.1}%), would need investigation",
        rate * 100.0
    );
}

#[test]
#[ignore = "slow (~30s); run manually with `--ignored`"]
fn roundtrip_noun_dative() {
    let Some(lex) = load_lexicon() else {
        return;
    };
    let features = NounFeatures {
        case: Some(Case::Dative),
        ..Default::default()
    };
    let (ok, total, fails) = roundtrip_nouns(&lex, features);
    let rate = ok as f64 / total.max(1) as f64;
    eprintln!("noun+DAT roundtrip: {ok}/{total} = {:.1}%", rate * 100.0);
    for (root, synth) in fails.iter().take(5) {
        eprintln!("  fail: {root} → {synth}");
    }
    assert!(
        rate >= 0.90,
        "noun dative roundtrip < 90% ({:.1}%)",
        rate * 100.0
    );
}

#[test]
#[ignore = "slow (~30s); run manually with `--ignored`"]
fn roundtrip_verb_past_1sg() {
    let Some(lex) = load_lexicon() else {
        return;
    };
    let features = VerbFeatures {
        tense: Some(Tense::PastDefinite),
        person: Some(Person::First),
        number: Some(Number::Singular),
        ..Default::default()
    };
    let (ok, total, fails) = roundtrip_verbs(&lex, features);
    let rate = ok as f64 / total.max(1) as f64;
    eprintln!("verb+PAST+1SG roundtrip: {ok}/{total} = {:.1}%", rate * 100.0);
    for (root, synth) in fails.iter().take(5) {
        eprintln!("  fail: {root} → {synth}");
    }
    assert!(
        rate >= 0.85,
        "verb past 1sg roundtrip < 85% ({:.1}%)",
        rate * 100.0
    );
}

#[test]
#[ignore = "slow (~30s); run manually with `--ignored`"]
fn roundtrip_noun_possessive_3() {
    let Some(lex) = load_lexicon() else {
        return;
    };
    let features = NounFeatures {
        possessive: Some(Possessive::P3),
        ..Default::default()
    };
    let (ok, total, fails) = roundtrip_nouns(&lex, features);
    let rate = ok as f64 / total.max(1) as f64;
    eprintln!("noun+POSS.3 roundtrip: {ok}/{total} = {:.1}%", rate * 100.0);
    for (root, synth) in fails.iter().take(5) {
        eprintln!("  fail: {root} → {synth}");
    }
    assert!(
        rate >= 0.85,
        "noun P3 roundtrip < 85% ({:.1}%)",
        rate * 100.0
    );
}
