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
    TemplateRepository, interpret_text, plan_response, realise, respond, respond_with_repo,
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
