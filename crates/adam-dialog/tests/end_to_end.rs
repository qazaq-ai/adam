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
            "түсіндім",
            "жасыңыз келісті",
            "жақсы жас",
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
            "түсіндім",
            "жақсы жер",
            "әдемі аймақ",
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
            "мақтанышпен",
            "сәттілік тілеймін",
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
        &["түсіндім", "ауа райы мейірімді болсын", "жақсы күн болсын"],
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

// --- v0.9.6 multilingual recogniser surface --------------------------------

#[test]
fn greeting_casual_triggers_from_russian_and_english() {
    for input in ["hi", "hello", "hey", "привет"] {
        let got = interpret_text(input, &[]);
        assert_eq!(
            got,
            Intent::Greeting {
                kind: GreetingKind::Casual
            },
            "input={input:?}"
        );
    }
}

#[test]
fn greeting_polite_from_russian() {
    let got = interpret_text("здравствуйте", &[]);
    assert_eq!(
        got,
        Intent::Greeting {
            kind: GreetingKind::Polite
        }
    );
}

#[test]
fn greeting_time_of_day_cross_language() {
    for (input, expected_tod) in [
        ("доброе утро", TimeOfDay::Morning),
        ("good morning", TimeOfDay::Morning),
        ("добрый день", TimeOfDay::Day),
        ("good afternoon", TimeOfDay::Day),
        ("добрый вечер", TimeOfDay::Evening),
        ("good evening", TimeOfDay::Evening),
    ] {
        let got = interpret_text(input, &[]);
        assert_eq!(
            got,
            Intent::Greeting {
                kind: GreetingKind::TimeOfDay(expected_tod)
            },
            "input={input:?}"
        );
    }
}

#[test]
fn farewell_triggers_ru_en() {
    for input in ["bye", "goodbye", "до свидания", "пока"] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::Farewell, "input={input:?}");
    }
}

#[test]
fn affirmation_triggers_ru_en() {
    for input in ["yes", "yeah", "ok", "да", "конечно"] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::Affirmation, "input={input:?}");
    }
}

#[test]
fn negation_triggers_ru_en() {
    for input in ["no", "nope", "нет"] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::Negation, "input={input:?}");
    }
}

#[test]
fn thanks_triggers_ru_en() {
    for input in ["thanks", "thank you", "спасибо", "большое спасибо"] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::Thanks, "input={input:?}");
    }
}

#[test]
fn apology_triggers_ru_en() {
    for input in ["sorry", "извини", "извините", "excuse me"] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::Apology, "input={input:?}");
    }
}

#[test]
fn ask_how_are_you_triggers_ru_en() {
    for input in ["how are you", "как дела", "как ты"] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::AskHowAreYou, "input={input:?}");
    }
}

#[test]
fn ask_name_triggers_ru_en() {
    for input in [
        "what's your name",
        "what is your name",
        "как тебя зовут",
        "как вас зовут",
    ] {
        let got = interpret_text(input, &[]);
        assert_eq!(got, Intent::AskName, "input={input:?}");
    }
}

#[test]
fn statement_of_name_from_russian_zovut() {
    let got = interpret_text("меня зовут Дәулет", &[]);
    assert_eq!(
        got,
        Intent::StatementOfName {
            name: "Дәулет".into()
        }
    );
}

#[test]
fn statement_of_name_from_english_my_name_is() {
    let got = interpret_text("my name is John", &[]);
    assert_eq!(
        got,
        Intent::StatementOfName {
            name: "John".into()
        }
    );
}

#[test]
fn statement_of_name_from_english_call_me() {
    let got = interpret_text("call me Anna", &[]);
    assert_eq!(
        got,
        Intent::StatementOfName {
            name: "Anna".into()
        }
    );
}

#[test]
fn statement_of_name_from_english_hi_i_am() {
    let got = interpret_text("hi i am John", &[]);
    assert_eq!(
        got,
        Intent::StatementOfName {
            name: "John".into()
        }
    );
}

#[test]
fn response_to_russian_input_is_kazakh() {
    // "Привет" in → Kazakh greeting out. Output never contains Latin
    // or Russian-only tokens.
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    for seed in 0..8u64 {
        let out = respond_with_repo("привет", &lex, &repo, seed);
        assert!(
            !out.chars().any(|c| c.is_ascii_alphabetic()),
            "response should be Kazakh (no ASCII letters), got {out:?}"
        );
    }
}

#[test]
fn latin_name_is_transliterated_for_fst_synthesis() {
    // v0.9.8 policy: plain `{name}` substitution stays verbatim
    // (Latin in → Latin out — user's spelling is preserved), but
    // morphology-requesting `{name|features}` transliterates to
    // Cyrillic first so `synthesise_noun` can work. No template
    // should ever produce a garbled "Johnм…"/"Johnп…"/"Johnб…".
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut saw_transliterated_synth = false;
    for seed in 0..32u64 {
        let out = respond_with_repo("my name is John", &lex, &repo, seed);
        assert!(
            !out.contains("Johnм") && !out.contains("Johnп") && !out.contains("Johnб"),
            "FST suffix attached to un-transliterated Latin root: {out:?}"
        );
        // Transliteration maps `John` → `Джохн`; the Instrumental
        // variant should therefore start with "Джохн" at least once.
        if out.contains("Джохн") {
            saw_transliterated_synth = true;
        }
    }
    assert!(
        saw_transliterated_synth,
        "expected at least one seed in 0..32 to produce a transliterated FST-synthesised form"
    );
}

#[test]
fn multilingual_conversation_flow_answers_in_kazakh() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    // English intro + Kazakh response with absorbed name.
    let intro = conv.turn("my name is John", &lex, &repo, 0);
    assert!(
        intro.contains("John"),
        "expected introduction response to reference John, got {intro:?}"
    );
    assert_eq!(conv.session.get("name"), Some(&"John".to_string()));

    // Russian greeting next — response should be Kazakh and may include John.
    let greet = conv.turn("привет", &lex, &repo, 2);
    assert!(!greet.contains("{"), "unfilled slot leaked: {greet:?}");
    // Either plain or personalised — both are valid paths.
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
