//! End-to-end dialog tests. These verify the full Layer-1..5 pipeline
//! on real Kazakh inputs, against the v1.0.0 lexicon (14 k entries).
//!
//! Each test is a (input, {acceptable_outputs}) pair. The dialog
//! planner has a controlled stochastic pick, so tests allow any of the
//! enumerable outputs for the recognised intent. A failure here means
//! either semantics mis-classified the input or the planner's template
//! pool for that intent is wrong.

use adam_dialog::intent::{GreetingKind, Intent, TimeOfDay};
use adam_dialog::{
    Conversation, TemplateRepository, interpret_text, plan_response, realise, respond,
    respond_with_repo,
};
use adam_kernel_fst::lexicon::LexiconV1;

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_lexicon() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !std::path::Path::new(curated).exists() || !std::path::Path::new(apertium).exists() {
        eprintln!("lexicon files not present, skipping");
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

/// Assert that the pipeline recognises `input` as the given `expected_intent`
/// (structural match — greeting kind etc. matter).
fn assert_intent(input: &str, expected: Intent) {
    let got = interpret_text(input, &[]);
    assert_eq!(got, expected, "input={input:?}");
}

/// Assert the final response text is one of `allowed` (accounting for
/// the planner's stochastic pick across different seeds).
fn assert_response_in_set(input: &str, allowed: &[&str]) {
    let Some(lex) = load_lexicon() else {
        return;
    };
    // Cycle seeds 0..8 to sample the planner's output space.
    for seed in 0..8u64 {
        let out = respond(input, &lex, seed);
        assert!(
            allowed.contains(&out.as_str()),
            "input={input:?} seed={seed} output={out:?} not in allowed={allowed:?}",
        );
    }
}

/// Like `assert_response_in_set` but uses the full TOML template
/// repository so all v0.7.5 templates (including new intents) are
/// reachable.
fn assert_response_with_toml(input: &str, allowed: &[&str]) {
    let Some(lex) = load_lexicon() else {
        return;
    };
    let repo = load_repo();
    for seed in 0..16u64 {
        let out = respond_with_repo(input, &lex, &repo, seed);
        assert!(
            allowed.contains(&out.as_str()),
            "input={input:?} seed={seed} output={out:?} not in allowed={allowed:?}",
        );
    }
}

// --- Intent recognition tests ----------------------------------------------

#[test]
fn greeting_casual_сәлем() {
    assert_intent(
        "сәлем",
        Intent::Greeting {
            kind: GreetingKind::Casual,
        },
    );
}

#[test]
fn greeting_polite_сәлеметсіз_бе() {
    assert_intent(
        "сәлеметсіз бе",
        Intent::Greeting {
            kind: GreetingKind::Polite,
        },
    );
}

#[test]
fn greeting_morning_қайырлы_таң() {
    assert_intent(
        "қайырлы таң",
        Intent::Greeting {
            kind: GreetingKind::TimeOfDay(TimeOfDay::Morning),
        },
    );
}

#[test]
fn farewell_сау_бол() {
    assert_intent("сау бол", Intent::Farewell);
}

#[test]
fn affirmation_иә() {
    assert_intent("иә", Intent::Affirmation);
}

#[test]
fn affirmation_дұрыс() {
    assert_intent("дұрыс", Intent::Affirmation);
}

#[test]
fn negation_жоқ() {
    assert_intent("жоқ", Intent::Negation);
}

#[test]
fn unknown_gibberish() {
    let got = matches!(interpret_text("xyz", &[]), Intent::Unknown { .. });
    assert!(got, "unknown token should produce Unknown intent");
}

// --- Full-pipeline response tests ------------------------------------------

#[test]
fn response_greeting_casual() {
    assert_response_in_set("сәлем", &["сәлем", "сәлем достым"]);
}

#[test]
fn response_greeting_polite() {
    assert_response_in_set("сәлеметсіз бе", &["сәлеметсіз бе", "армысыз"]);
}

#[test]
fn response_farewell() {
    assert_response_in_set("сау бол", &["сау бол", "кездескенше", "аман бол"]);
}

#[test]
fn response_affirmation() {
    assert_response_in_set("иә", &["иә", "дұрыс айтасыз", "рас", "мақұл"]);
}

#[test]
fn response_negation() {
    assert_response_in_set("жоқ", &["жоқ", "дұрыс емес"]);
}

// --- v0.7.5 new intents (require TOML repo) --------------------------------

#[test]
fn response_thanks() {
    assert_response_with_toml("рахмет", &["оқасы жоқ", "ештеңе емес", "ризамын"]);
}

#[test]
fn response_thanks_emphatic() {
    assert_response_with_toml("көп рахмет", &["оқасы жоқ", "ештеңе емес", "ризамын"]);
}

#[test]
fn response_apology() {
    assert_response_with_toml("кешіріңіз", &["ештеңе емес", "мейлі", "түк етпейді"]);
}

#[test]
fn response_ask_how_are_you_polite() {
    assert_response_with_toml(
        "қалайсыз",
        &[
            "жақсымын, рахмет",
            "жаман емеспін",
            "жақсы, ал сіз қалайсыз",
        ],
    );
}

#[test]
fn response_ask_how_are_you_casual() {
    assert_response_with_toml(
        "қалайсың",
        &[
            "жақсымын, рахмет",
            "жаман емеспін",
            "жақсы, ал сіз қалайсыз",
        ],
    );
}

#[test]
fn response_statement_of_wellbeing() {
    assert_response_with_toml("жақсымын", &["жақсы екен", "қуанамын", "ал сіз қалайсыз"]);
}

#[test]
fn response_ask_name() {
    assert_response_with_toml("атың кім", &["менің атым адам", "мені адам деп атайды"]);
}

#[test]
fn response_ask_name_polite() {
    assert_response_with_toml("атыңыз кім", &["менің атым адам", "мені адам деп атайды"]);
}

// --- v0.8.0 social-topic intents + PersonName extraction -------------------

#[test]
fn intent_statement_of_name_with_атым() {
    let got = interpret_text("менің атым Дәулет", &[]);
    assert_eq!(
        got,
        Intent::StatementOfName {
            name: "Дәулет".into()
        }
    );
}

#[test]
fn intent_statement_of_name_with_мені_деп_атайды() {
    let got = interpret_text("мені Дәулет деп атайды", &[]);
    assert_eq!(
        got,
        Intent::StatementOfName {
            name: "Дәулет".into()
        }
    );
}

#[test]
fn intent_statement_of_name_lowercase_is_capitalised() {
    let got = interpret_text("атым нұрсұлтан", &[]);
    assert_eq!(
        got,
        Intent::StatementOfName {
            name: "Нұрсұлтан".into()
        }
    );
}

#[test]
fn response_statement_of_name_substitutes_slot() {
    // Plain and FST-backed instrumental variants.
    assert_response_with_toml(
        "менің атым Дәулет",
        &[
            "қош келдіңіз Дәулет",
            "сәлем Дәулет",
            "Дәулетпен танысқаныма қуаныштымын",
            "Дәулетпен сөйлесу — құрмет",
        ],
    );
}

#[test]
fn response_ask_age() {
    assert_response_with_toml(
        "жасың неше",
        &[
            "менің жасым адамзат жасындай",
            "мен әлі жаспын",
            "жасымды айта алмаймын",
        ],
    );
}

#[test]
fn response_statement_of_age() {
    assert_response_with_toml(
        "менің жасым отыз",
        &[
            "жақсы жас",
            "қуатты кезеңіңіз",
            "30 жас — тамаша кезең",
            "жасыңыз 30 екен",
        ],
    );
}

#[test]
fn response_ask_location() {
    assert_response_with_toml(
        "қайда тұрасыз",
        &[
            "мен сандық әлемде тұрамын",
            "менің мекенім жоқ",
            "қазақстан елімде",
        ],
    );
}

#[test]
fn response_statement_of_location() {
    assert_response_with_toml(
        "мен Алматыданмын",
        &[
            "жақсы жер",
            "әдемі аймақ",
            "тамаша өлке",
            "Алматы — әдемі қала",
            "Алматы туралы көп естідім",
            "Алматыда тұрасыз ба",
            "Алматыдан хабар жақсы ма",
            "Алматыға сапар шегу қызық",
        ],
    );
}

#[test]
fn response_ask_occupation() {
    assert_response_with_toml(
        "немен айналысасың",
        &[
            "мен сөйлесуге жаралғанмын",
            "менің жұмысым — сізге көмектесу",
            "мен тілдерді үйренемін",
        ],
    );
}

#[test]
fn response_statement_of_occupation() {
    assert_response_with_toml(
        "мен мұғаліммін",
        &[
            "жақсы кәсіп",
            "сәттілік тілеймін",
            "мақтанатын жұмыс",
            "мұғалім — құрметті кәсіп",
            "сіз мұғалім екенсіз",
            "мұғалімдер — қажетті мамандық",
            "мұғалімге сәттілік тілеймін",
        ],
    );
}

#[test]
fn response_ask_family() {
    assert_response_with_toml(
        "балаларың бар ма",
        &["менің отбасым жоқ", "мен жалғызбын", "сұрағыңыз керемет"],
    );
}

#[test]
fn response_statement_of_family() {
    assert_response_with_toml(
        "менің балам бар",
        &[
            "отбасыңыз аман болсын",
            "жақсы отбасы жарасымды",
            "қуаныштымын",
        ],
    );
}

#[test]
fn response_ask_weather() {
    assert_response_with_toml(
        "ауа райы қалай",
        &[
            "менде терезе жоқ",
            "ауа райын білмеймін",
            "сыртта қалай екенін айтыңызшы",
        ],
    );
}

#[test]
fn response_statement_of_weather() {
    assert_response_with_toml(
        "бүгін суық",
        &[
            "ауа райы мейірімді болсын",
            "жақсы күн болсын",
            "табиғат мезгіліне лайық",
        ],
    );
}

#[test]
fn response_ask_time() {
    assert_response_with_toml(
        "сағат неше",
        &[
            "уақытты білмеймін",
            "менде сағат жоқ",
            "уақыт — асыл қазына",
        ],
    );
}

#[test]
fn response_compliment() {
    assert_response_with_toml(
        "жарайсың",
        &["рахмет", "сіз де өте жақсысыз", "қуаныштымын"],
    );
}

#[test]
fn response_request() {
    assert_response_with_toml(
        "көмектесіңізші",
        &["әрине, айтыңыз", "қалай көмектесе аламын", "тыңдап тұрмын"],
    );
}

#[test]
fn response_well_wishes() {
    assert_response_with_toml(
        "сәттілік",
        &["сізге де", "сәттілік сізге де", "тілегіңіз қабыл болсын"],
    );
}

// --- v1.1.0 Kazakh-only surface (reverted v0.9.6 multilingual) -------------

#[test]
fn kazakh_only_rejects_latin_input_as_unknown() {
    // v1.1.0 dropped all RU/EN triggers. Foreign-script input falls
    // through the 25 Kazakh intents and lands on Unknown.
    for input in [
        "hello",
        "hi",
        "bye",
        "thanks",
        "my name is John",
        "как дела",
    ] {
        let got = interpret_text(input, &[]);
        assert!(
            matches!(got, Intent::Unknown { .. }),
            "input={input:?} should be Unknown after Kazakh-only revert, got {got:?}"
        );
    }
}

// --- v1.1.0 Insult intent ---------------------------------------------------

#[test]
fn intent_insult_detects_rude_forms() {
    for input in ["сен ақымақсың", "ақымақ", "түкке тұрмайсың"] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::Insult, "input={input:?}");
    }
}

#[test]
fn response_insult_is_polite_non_engagement() {
    assert_response_with_toml(
        "сен ақымақсың",
        &[
            "сізге ренжімеймін",
            "мен адамды құрметтеймін",
            "мейлі, сіз өз пікіріңізді айттыңыз",
            "мен жауап бермеймін",
        ],
    );
}

// --- v1.1.0 Smarter Unknown handler (context-aware fallback) ----------------

#[test]
fn unknown_acknowledges_topic_when_noun_in_parse() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    // "кітап оқимын" (I read a book) — "оқимын" isn't a StatementOf*
    // pattern we recognise, but the FST parser should surface "кітап"
    // as a noun, which routes us to unknown.with_noun.
    let mut saw_topic_acknowledged = false;
    for seed in 0..32u64 {
        let out = respond_with_repo("кітап оқимын", &lex, &repo, seed);
        if out.contains("кітап") {
            saw_topic_acknowledged = true;
        }
    }
    assert!(
        saw_topic_acknowledged,
        "expected at least one unknown.with_noun template to mention 'кітап'"
    );
}

// --- v1.1.0 Modern Lexicon coverage ----------------------------------------

#[test]
fn lexicon_extracts_modern_occupations() {
    let Some(lex) = load_lexicon() else { return };
    // New curated roots added in v1.1.0 (бағдарламашы, әзірлеуші,
    // зерттеуші, аудармашы, жазушы) must all route to
    // StatementOfOccupation via the lexicon-backed 1sg-copula stripper.
    for (input, expected_root) in [
        ("мен бағдарламашымын", "бағдарламашы"),
        ("мен әзірлеушімін", "әзірлеуші"),
        ("мен зерттеушімін", "зерттеуші"),
        ("мен аудармашымын", "аудармашы"),
        ("мен жазушымын", "жазушы"),
    ] {
        let got = adam_dialog::interpret_text_with_lexicon(input, &[], Some(&lex));
        assert_eq!(
            got,
            Intent::StatementOfOccupation {
                occupation: Some(expected_root.to_string())
            },
            "input={input:?}"
        );
    }
}

// --- v1.4.0 FST-NER + DST + follow-up resolution ---------------------------

#[test]
fn fst_ner_recognises_lexicon_place_name_as_city() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен Алматыданмын", &lex, &repo, 0);
    // Lexicon has 'алматы' (lowercased, v1.4.0). FST-NER primary path
    // returns the Lexicon root directly.
    assert_eq!(conv.session.get("city"), Some(&"Алматы".to_string()));
}

#[test]
fn fst_ner_recognises_lexicon_occupation_via_predicate_p1sg() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    // "мен бағдарламашымын" — FST parses as Noun(бағдарламашы)+P1Sg.
    let _ = conv.turn("мен бағдарламашымын", &lex, &repo, 0);
    assert_eq!(
        conv.session.get("occupation"),
        Some(&"бағдарламашы".to_string())
    );
}

#[test]
fn fst_ner_rejects_adjective_predicate_as_occupation() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    // "жақсымын" = adj жақсы + P1Sg. Parser emits Analysis::Noun
    // (nominal slot) with root marked adjective — occupation NER must
    // POS-filter and reject, letting wellbeing fire.
    let _ = conv.turn("жақсымын", &lex, &repo, 0);
    assert!(conv.session.get("occupation").is_none());
}

#[test]
fn dst_tracks_active_intent_across_turns() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("қалайсыз", &lex, &repo, 0);
    assert_eq!(
        conv.active_intent,
        Some(adam_dialog::conversation::IntentKind::AskHowAreYou)
    );
    let _ = conv.turn("жақсымын", &lex, &repo, 0);
    assert_eq!(
        conv.active_intent,
        Some(adam_dialog::conversation::IntentKind::StatementOfWellbeing)
    );
    assert_eq!(conv.intent_history.len(), 2);
}

#[test]
fn follow_up_ал_сіз_resolves_against_active_intent() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("қалайсыз", &lex, &repo, 0); // AskHowAreYou
    // "ал сіз" alone would be Unknown; DST should re-tag as AskHowAreYou.
    let out = conv.turn("ал сіз", &lex, &repo, 0);
    // Response must be a how-are-you answer template, not бл Unknown.
    assert!(
        out.contains("жақсы") || out.contains("жаман") || out.contains("рахмет"),
        "expected wellbeing-answer after follow-up, got {out:?}"
    );
}

#[test]
fn reset_clears_all_state_including_history() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);
    let _ = conv.turn("қалайсыз", &lex, &repo, 0);
    assert!(!conv.session.is_empty());
    assert!(conv.active_intent.is_some());
    assert!(!conv.intent_history.is_empty());
    conv.reset();
    assert!(conv.session.is_empty());
    assert!(conv.active_intent.is_none());
    assert!(conv.intent_history.is_empty());
}

// --- v0.8.5 session state (Conversation) -----------------------------------

#[test]
fn conversation_remembers_name_across_turns() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // Turn 1: user introduces. Any of the statement_of_name templates
    // is acceptable — we only assert the name got absorbed.
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);
    assert_eq!(conv.session.get("name"), Some(&"Дәулет".to_string()));

    // Turn 2: plain casual greeting. Because the session has a name,
    // "сәлем {name}" is now eligible and should fire for at least one
    // seed in 0..16. We also ensure every fired response is plausible
    // (no unfilled `{name}` leaks).
    let mut saw_personalised = false;
    for seed in 0..16u64 {
        let out = conv.turn("сәлем", &lex, &repo, seed);
        assert!(!out.contains("{name}"), "unfilled slot leaked: {out:?}");
        if out == "сәлем Дәулет" {
            saw_personalised = true;
        }
    }
    assert!(
        saw_personalised,
        "expected at least one seed in 0..16 to pick \"сәлем Дәулет\""
    );
}

#[test]
fn conversation_without_name_never_emits_unfilled_greeting() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    // No introduction: no name in session. Every seed must produce a
    // literal-only greeting.
    for seed in 0..16u64 {
        let out = conv.turn("сәлем", &lex, &repo, seed);
        assert!(!out.contains("{"), "slot placeholder leaked: {out:?}");
        assert!(
            out == "сәлем" || out == "сәлем достым",
            "unexpected greeting w/o name: {out:?}"
        );
    }
}

#[test]
fn intent_statement_of_age_parses_kazakh_numeral() {
    let got = interpret_text("менің жасым отыз", &[]);
    assert_eq!(got, Intent::StatementOfAge { years: Some(30) });
}

#[test]
fn intent_statement_of_age_parses_compound_numeral() {
    let got = interpret_text("менің жасым отыз бес", &[]);
    assert_eq!(got, Intent::StatementOfAge { years: Some(35) });
}

#[test]
fn intent_statement_of_age_without_numeral_still_fires() {
    let got = interpret_text("жасым жасырын", &[]);
    assert_eq!(got, Intent::StatementOfAge { years: None });
}

#[test]
fn intent_statement_of_location_extracts_city_from_ablative() {
    let got = interpret_text("мен Алматыданмын", &[]);
    assert_eq!(
        got,
        Intent::StatementOfLocation {
            city: Some("Алматы".into())
        }
    );
}

#[test]
fn intent_statement_of_location_extracts_city_from_locative() {
    let got = interpret_text("астанада тұрамын", &[]);
    assert_eq!(
        got,
        Intent::StatementOfLocation {
            city: Some("астана".into())
        }
    );
}

#[test]
fn intent_statement_of_occupation_extracts_root() {
    let got = interpret_text("мен мұғаліммін", &[]);
    assert_eq!(
        got,
        Intent::StatementOfOccupation {
            occupation: Some("мұғалім".into())
        }
    );
}

// --- v0.9.7 lexicon-backed generic 1sg-copula stripping --------------------

#[test]
fn lexicon_path_extracts_occupation_outside_fixed_table() {
    let Some(lex) = load_lexicon() else { return };
    // `ақын` (poet) is in the curated lexicon as a noun but NOT in
    // the 6-form fixed table. Without lexicon, v0.9.6 returned None
    // for this utterance — it fell through to StatementOfWellbeing.
    let got = adam_dialog::interpret_text_with_lexicon("мен ақынмын", &[], Some(&lex));
    assert_eq!(
        got,
        Intent::StatementOfOccupation {
            occupation: Some("ақын".into())
        }
    );
}

#[test]
fn lexicon_path_extracts_multiple_new_occupations() {
    let Some(lex) = load_lexicon() else { return };
    // Each of these nouns lives in the curated lexicon and should
    // round-trip through the copula-strip + POS=noun lookup.
    for (input, expected_root) in [
        ("мен әншімін", "әнші"),
        ("мен ғалыммын", "ғалым"),
        ("мен суретшімін", "суретші"),
    ] {
        let got = adam_dialog::interpret_text_with_lexicon(input, &[], Some(&lex));
        assert_eq!(
            got,
            Intent::StatementOfOccupation {
                occupation: Some(expected_root.to_string())
            },
            "input={input:?}"
        );
    }
}

#[test]
fn lexicon_path_rejects_adjectives_from_occupation_extraction() {
    let Some(lex) = load_lexicon() else { return };
    // "жақсымын" = "I'm good" — "жақсы" is an adjective in the lexicon.
    // The POS filter must reject it from occupation extraction and let
    // wellbeing fire instead.
    let got = adam_dialog::interpret_text_with_lexicon("жақсымын", &[], Some(&lex));
    assert_eq!(got, Intent::StatementOfWellbeing);
}

#[test]
fn lexicon_path_rejects_unknown_roots() {
    let Some(lex) = load_lexicon() else { return };
    // `xyzzyмын` — copula suffix stripping succeeds but "xyzzy" is not
    // in the lexicon, so occupation-extraction should decline and the
    // utterance falls through to the generic pipeline.
    let got = adam_dialog::interpret_text_with_lexicon("xyzzyмын", &[], Some(&lex));
    assert!(
        !matches!(got, Intent::StatementOfOccupation { .. }),
        "unknown root should not become an occupation, got {got:?}"
    );
}

#[test]
fn conversation_absorbs_lexicon_derived_occupation() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен әншімін", &lex, &repo, 0);
    assert_eq!(conv.session.get("occupation"), Some(&"әнші".to_string()));
}

#[test]
fn conversation_absorbs_age_city_occupation() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("менің жасым отыз", &lex, &repo, 0);
    assert_eq!(conv.session.get("age"), Some(&"30".to_string()));

    let _ = conv.turn("мен Алматыданмын", &lex, &repo, 0);
    assert_eq!(conv.session.get("city"), Some(&"Алматы".to_string()));

    let _ = conv.turn("мен мұғаліммін", &lex, &repo, 0);
    assert_eq!(conv.session.get("occupation"), Some(&"мұғалім".to_string()));
}

#[test]
fn conversation_age_slot_appears_in_personalised_template() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // Explore the template space — one seed should pick a
    // {age}-referencing variant.
    let mut saw_personalised = false;
    for seed in 0..32u64 {
        let out = conv.turn("менің жасым отыз", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out.contains("30") {
            saw_personalised = true;
        }
    }
    assert!(
        saw_personalised,
        "expected at least one seed in 0..32 to pick a template containing 30"
    );
}

// --- v0.9.5 FST-backed slot expansion ---------------------------------------

#[test]
fn realiser_synthesises_locative_for_city_slot() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен Алматыданмын", &lex, &repo, 0);

    // Explore the seed space; at least one seed should pick a
    // {city|locative} template, producing "Алматыда тұрасыз ба".
    let mut saw_locative = false;
    for seed in 0..32u64 {
        let out = conv.turn("мен Алматыданмын", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out == "Алматыда тұрасыз ба" {
            saw_locative = true;
        }
    }
    assert!(
        saw_locative,
        "expected at least one seed in 0..32 to synthesise Locative \"Алматыда тұрасыз ба\""
    );
}

#[test]
fn realiser_synthesises_instrumental_for_name_slot() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let mut saw_instrumental = false;
    for seed in 0..32u64 {
        let out = conv.turn("менің атым Дәулет", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out.contains("Дәулетпен") {
            saw_instrumental = true;
        }
    }
    assert!(
        saw_instrumental,
        "expected at least one seed in 0..32 to pick an Instrumental \"Дәулетпен\" template"
    );
}

#[test]
fn realiser_synthesises_plural_for_occupation_slot() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let mut saw_plural = false;
    for seed in 0..32u64 {
        let out = conv.turn("мен мұғаліммін", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out == "мұғалімдер — қажетті мамандық" {
            saw_plural = true;
        }
    }
    assert!(
        saw_plural,
        "expected at least one seed in 0..32 to pick the {{occupation|plural}} template"
    );
}

#[test]
fn cross_slot_how_are_you_uses_remembered_name() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);

    let mut saw_personalised = false;
    for seed in 0..32u64 {
        let out = conv.turn("қалайсыз", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out.contains("Дәулет") {
            saw_personalised = true;
        }
    }
    assert!(
        saw_personalised,
        "expected at least one seed to personalise ask-how-are-you response with the name"
    );
}

#[test]
fn cross_slot_age_mentions_remembered_name() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);

    let mut saw_cross = false;
    for seed in 0..32u64 {
        let out = conv.turn("менің жасым отыз", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out.contains("Дәулет") && out.contains("30") {
            saw_cross = true;
        }
    }
    assert!(
        saw_cross,
        "expected at least one seed in 0..32 to pick a template mentioning BOTH name and age"
    );
}

#[test]
fn cross_slot_occupation_in_city_fires_with_all_three() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);
    let _ = conv.turn("мен Алматыданмын", &lex, &repo, 0);

    let mut saw_triple = false;
    for seed in 0..32u64 {
        let out = conv.turn("мен мұғаліммін", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        // Triple-slot: Дәулет + Алматыда (locative) + мұғалім.
        if out.contains("Дәулет") && out.contains("Алматыда") && out.contains("мұғалім")
        {
            saw_triple = true;
        }
    }
    assert!(
        saw_triple,
        "expected at least one seed to pick the {{name}}+{{city|loc}}+{{occupation}} triple-slot template"
    );
}

#[test]
fn cross_slot_greeting_fires_when_both_name_and_city_known() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    // Seed the session with both entities.
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);
    let _ = conv.turn("мен Алматыданмын", &lex, &repo, 0);
    // Now a plain "сәлем" should have the cross-slot template in its
    // eligible pool.
    let mut saw_cross = false;
    for seed in 0..32u64 {
        let out = conv.turn("сәлем", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out == "сәлем Дәулет, Алматыдан хабар жақсы ма" {
            saw_cross = true;
        }
    }
    assert!(
        saw_cross,
        "expected at least one seed in 0..32 to pick the {{name}}+{{city|abl}} cross-slot greeting"
    );
}

#[test]
fn conversation_reset_clears_name() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("атым Дәулет", &lex, &repo, 0);
    assert!(conv.session.contains_key("name"));
    conv.reset();
    assert!(conv.session.is_empty());
}

// --- Traceability ----------------------------------------------------------

#[test]
fn plan_is_deterministic_for_fixed_seed() {
    let plan1 = plan_response(
        &Intent::Greeting {
            kind: GreetingKind::Casual,
        },
        42,
    );
    let plan2 = plan_response(
        &Intent::Greeting {
            kind: GreetingKind::Casual,
        },
        42,
    );
    assert_eq!(plan1, plan2, "same seed must produce same plan");
}

#[test]
fn trace_contains_intent_and_choice() {
    let plan = plan_response(
        &Intent::Greeting {
            kind: GreetingKind::Casual,
        },
        0,
    );
    let joined = plan.trace.join(" ");
    assert!(
        joined.contains("intent=") && joined.contains("chosen_index="),
        "trace should expose intent + decision"
    );
    let out = realise(&plan);
    assert!(
        !out.is_empty(),
        "realiser should emit non-empty for greeting"
    );
}

/// v1.6.5: when a committed morpheme index is attached, `Intent::Unknown`
/// with a recognised noun should cite a concrete Kazakh sentence from the
/// corpus instead of the bare noun-echo template.
#[test]
fn unknown_with_retrieval_cites_corpus_example() {
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let index_path = "../../data/retrieval/morpheme_index.json";
    if !std::path::Path::new(index_path).exists() {
        eprintln!("morpheme index not present, skipping");
        return;
    }
    let raw = std::fs::read_to_string(index_path).expect("read index");
    let mut index: MorphemeIndex = serde_json::from_str(&raw).expect("parse index");
    index.refresh_stats();

    let mut conv = Conversation::new().with_morpheme_index(index);
    // "бала" is known to have postings in the committed snapshot with
    // Abai's lines stored as sample_texts.
    let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, 0);
    assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
    // The evidence templates all wrap the example in «…», so assert
    // the guillemets are present as a robust signal that the evidence
    // path fired (not the v1.1.0 noun-echo fallback).
    assert!(
        out.contains("«") && out.contains("»"),
        "expected retrieval-evidence template to quote an example, got: {out:?}"
    );
}

/// Without an attached index, the Unknown fallback keeps the v1.1.0
/// noun-echo behaviour — no retrieval side-effects, no crashes.
#[test]
fn unknown_without_index_falls_back_to_noun_echo() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, 0);
    assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
    assert!(
        !out.contains("«"),
        "no index attached → no evidence quote, got: {out:?}"
    );
}

/// v1.8.0: when the user has told us their name AND retrieval has a
/// sample for the topic, at least ONE seed should pick a session-aware
/// template that personalises the citation (mentions the name).
/// The evidence itself stays verbatim — only the frame composes.
#[test]
fn unknown_with_session_and_evidence_personalises_frame() {
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let index_path = "../../data/retrieval/morpheme_index.json";
    if !std::path::Path::new(index_path).exists() {
        eprintln!("morpheme index not present, skipping");
        return;
    }
    let raw = std::fs::read_to_string(index_path).expect("read index");
    let mut index: MorphemeIndex = serde_json::from_str(&raw).expect("parse index");
    index.refresh_stats();

    let mut conv = Conversation::new().with_morpheme_index(index);
    // Establish session: name → Дәулет.
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);

    // Across several seeds, at least one Unknown response should
    // personalise with the name.
    let mut saw_personalised = false;
    for seed in 0..32u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        assert!(
            !out.contains("{"),
            "unfilled slot leaked at seed={seed}: {out:?}"
        );
        // Guillemets confirm the evidence path fired (not the noun-echo
        // path); name presence confirms the frame was personalised.
        if out.contains("Дәулет") && out.contains("«") {
            saw_personalised = true;
        }
    }
    assert!(
        saw_personalised,
        "expected at least one seed to produce a session-aware evidence response mentioning the name"
    );
}

/// v1.8.0: with BOTH name and city in the session, at least one
/// template combining both should activate for an Unknown + evidence
/// turn. Verifies the {name} + {city} + {example} combination is reachable.
#[test]
fn unknown_with_session_name_and_city_can_use_combined_frame() {
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let index_path = "../../data/retrieval/morpheme_index.json";
    if !std::path::Path::new(index_path).exists() {
        eprintln!("morpheme index not present, skipping");
        return;
    }
    let raw = std::fs::read_to_string(index_path).expect("read index");
    let mut index: MorphemeIndex = serde_json::from_str(&raw).expect("parse index");
    index.refresh_stats();

    let mut conv = Conversation::new().with_morpheme_index(index);
    // Populate session directly — the "мен Алматыдамын" path has a
    // pre-existing semantic bug (the occupation recogniser greedy-strips
    // -мын), unrelated to v1.8.0's template layer.
    conv.session.insert("name".into(), "Дәулет".into());
    conv.session.insert("city".into(), "Алматы".into());

    let mut saw_name_and_city = false;
    for seed in 0..32u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        assert!(
            !out.contains("{"),
            "unfilled slot leaked at seed={seed}: {out:?}"
        );
        if out.contains("Дәулет") && out.contains("Алматы") && out.contains("«") {
            saw_name_and_city = true;
        }
    }
    assert!(
        saw_name_and_city,
        "expected at least one seed to produce a response combining name + city + evidence"
    );
}
