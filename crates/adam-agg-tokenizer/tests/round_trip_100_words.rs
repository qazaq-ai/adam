//! Integration test: 100-word round-trip battery against the real
//! committed Lexicon. Target: ≥ 80% pass rate on Phase 0; raised to
//! ≥ 95% in Phase 1 after OOV-handling lands.
//!
//! Words are sampled in two categories:
//! 1. Bare roots — should round-trip 100% (these are the easy case).
//! 2. Inflected forms covered by the FST analyser/synthesiser pair —
//!    should round-trip whenever the FST itself round-trips. Failures
//!    here usually indicate edge cases (ambiguous parse where analyse
//!    returns a different decomposition than synth produces).

use std::path::Path;

use adam_agg_tokenizer::AggTokenizer;
use adam_kernel_fst::lexicon::LexiconV1;

fn load_lexicon() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !Path::new(curated).exists() || !Path::new(apertium).exists() {
        eprintln!("agg-tokenizer test: skipping — lexicon files absent");
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

/// 60 bare-root words: these all exist in the Lexicon and should
/// round-trip 100% (parse → root + empty features → synth = same word).
const BARE_ROOTS: &[&str] = &[
    "бала",
    "ата",
    "ана",
    "адам",
    "ел",
    "тіл",
    "сөз",
    "ой",
    "жан",
    "көз",
    "құлақ",
    "қол",
    "аяқ",
    "бас",
    "жүрек",
    "су",
    "тас",
    "тау",
    "көл",
    "өзен",
    "теңіз",
    "аспан",
    "жұлдыз",
    "ай",
    "күн",
    "от",
    "жел",
    "жаңбыр",
    "қар",
    "бұлт",
    "үй",
    "есік",
    "терезе",
    "стол",
    "орындық",
    "кітап",
    "қалам",
    "дәптер",
    "мектеп",
    "сабақ",
    "мұғалім",
    "оқушы",
    "дос",
    "құрбы",
    "ағаш",
    "гүл",
    "жапырақ",
    "тамыр",
    "жеміс",
    "дән",
    "нан",
    "сүт",
    "май",
    "тұз",
    "қант",
    "шай",
    "су",
    "ас",
    "тағам",
    "дәрі",
];

/// 40 inflected forms — round-trip depends on FST's determinism.
const INFLECTED: &[&str] = &[
    "балалар",      // бала + plural
    "балаға",       // бала + dative
    "баланың",      // бала + genitive
    "балам",        // бала + P1Sg poss
    "баламыз",      // бала + P1Pl poss
    "кітабым",      // кітап + P1Sg
    "кітаптар",     // кітап + plural
    "кітаптарым",   // кітап + plural + P1Sg
    "кітапта",      // кітап + locative
    "кітаптан",     // кітап + ablative
    "көзіме",       // көз + P1Sg + dative
    "қолымен",      // қол + P1Sg + instrumental
    "досым",        // дос + P1Sg
    "достарымыз",   // дос + plural + P1Pl
    "достарымызға", // дос + plural + P1Pl + dative
    "елімізде",     // ел + P1Pl + locative
    "тілімізді",    // тіл + P1Pl + accusative
    "ағаштар",      // ағаш + plural
    "тауларда",     // тау + plural + locative
    "өзендер",      // өзен + plural
    "көлдер",       // көл + plural
    "көлдерден",    // көл + plural + ablative
    "аспандағы",    // аспан + locative-attributive
    "айдан",        // ай + ablative
    "күнді",        // күн + accusative
    "оттың",        // от + genitive
    "желмен",       // жел + instrumental
    "үйде",         // үй + locative
    "мектепке",     // мектеп + dative
    "сабақта",      // сабақ + locative
    "мұғалімге",    // мұғалім + dative
    "оқушылар",     // оқушы + plural
    "оқушыларды",   // оқушы + plural + accusative
    "тастар",       // тас + plural
    "сулар",        // су + plural
    "жұлдыздар",    // жұлдыз + plural
    "терезеде",     // терезе + locative
    "столдар",      // стол + plural
    "қаламды",      // қалам + accusative
    "дәптерлер",    // дәптер + plural
];

#[test]
fn bare_roots_round_trip_100_percent() {
    let Some(lex) = load_lexicon() else { return };
    let tok = AggTokenizer::build(lex);
    let mut failures = Vec::new();
    for word in BARE_ROOTS {
        let trip = match tok.round_trip(word) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{word}: error {e:?}"));
                continue;
            }
        };
        if !trip {
            let tokens = tok.tokenize_word(word);
            let reconstructed = tok.detokenize_word(&tokens).unwrap_or_default();
            failures.push(format!(
                "{word} → tokens={:?} → reconstructed={reconstructed:?}",
                tokens
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "bare-root round-trip failures ({}/{}):\n{}",
        failures.len(),
        BARE_ROOTS.len(),
        failures.join("\n")
    );
}

#[test]
fn inflected_forms_round_trip_at_least_80_percent() {
    let Some(lex) = load_lexicon() else { return };
    let tok = AggTokenizer::build(lex);
    let mut pass = 0;
    let mut fail_details = Vec::new();
    for word in INFLECTED {
        let trip = tok.round_trip(word).unwrap_or_default();
        if trip {
            pass += 1;
        } else {
            let tokens = tok.tokenize_word(word);
            let reconstructed = tok.detokenize_word(&tokens).unwrap_or_default();
            fail_details.push(format!("{word} → {reconstructed:?}"));
        }
    }
    let pass_rate = (pass as f64) / (INFLECTED.len() as f64);
    assert!(
        pass_rate >= 0.80,
        "inflected round-trip < 80% (got {:.1}% = {}/{}):\n{}",
        pass_rate * 100.0,
        pass,
        INFLECTED.len(),
        fail_details.join("\n")
    );
}

#[test]
fn total_round_trip_at_least_90_percent() {
    let Some(lex) = load_lexicon() else { return };
    let tok = AggTokenizer::build(lex);
    let all: Vec<&&str> = BARE_ROOTS.iter().chain(INFLECTED.iter()).collect();
    let total = all.len();
    let mut pass = 0;
    for word in &all {
        if matches!(tok.round_trip(word), Ok(true)) {
            pass += 1;
        }
    }
    let pass_rate = (pass as f64) / (total as f64);
    assert!(
        pass_rate >= 0.90,
        "total round-trip < 90% (got {:.1}% = {}/{})",
        pass_rate * 100.0,
        pass,
        total
    );
}

#[test]
fn vocab_size_matches_lexicon_after_dedup() {
    let Some(lex) = load_lexicon() else { return };
    let tok = AggTokenizer::build(lex);
    let vocab = tok.vocab();
    // LexiconV1 dedups entries by (id, part_of_speech), so the unique
    // count is lower than the raw 25.5k from the source JSON files.
    // Phase 0 reality: ~16-17k unique entries after dedup. Setting a
    // floor of 15k catches accidental loss of large fractions of the
    // lexicon; the upper bound is just sanity (the lexicon should fit
    // in a u32-indexed vocab comfortably).
    let size = vocab.size();
    assert!(
        (15_000..u32::MAX).contains(&size),
        "vocab size unexpected: {size} (expected 15 000 ≤ size < u32::MAX)"
    );
}

#[test]
fn sentence_with_punctuation_tokenizes_and_detokenizes() {
    let Some(lex) = load_lexicon() else { return };
    let tok = AggTokenizer::build(lex);
    let input = "Балалар, мектепке барады.";
    let tokens = tok.tokenize_sentence(input);
    // Should have BOS at start, EOS at end, at least one Punct.
    assert!(matches!(
        tokens.first(),
        Some(adam_agg_tokenizer::MorphToken::Bos)
    ));
    assert!(matches!(
        tokens.last(),
        Some(adam_agg_tokenizer::MorphToken::Eos)
    ));
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, adam_agg_tokenizer::MorphToken::Punct(_)))
    );
    // Detokenize should produce something readable.
    let out = tok.detokenize_sentence(&tokens);
    assert!(out.is_ok(), "detokenize_sentence failed: {:?}", out);
}
