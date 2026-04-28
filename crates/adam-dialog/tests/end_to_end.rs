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
    Conversation, GraphAdmissibilityIssue, TemplateRepository, TraceFaithfulnessIssue,
    audit_graph_admissibility, audit_response, audit_trace_faithfulness, audit_typed_faithfulness,
    interpret_text, plan_response, realise, respond, respond_with_repo,
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

fn support_facts_for_derived(
    derived: &[adam_reasoning::reasoner::DerivedFact],
) -> Vec<adam_reasoning::Fact> {
    derived
        .iter()
        .flat_map(support_facts_for_single_derived)
        .collect()
}

fn support_facts_for_single_derived(
    derived: &adam_reasoning::reasoner::DerivedFact,
) -> Vec<adam_reasoning::Fact> {
    let src = &derived.source_chain;
    if src.is_empty() {
        return Vec::new();
    }
    if src.len() == 1 {
        return vec![make_support_fact(
            &src[0],
            "қолдау",
            adam_reasoning::Predicate::IsA,
            "ұғым",
        )];
    }
    let mid_kind = "аралық";
    let place_mid = "қала";
    let time_mid = "түс";
    match derived.rule_id.as_str() {
        "R1_is_a_transitivity" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::IsA,
                mid_kind,
            ),
            make_support_fact(
                &src[1],
                mid_kind,
                adam_reasoning::Predicate::IsA,
                &derived.object.root,
            ),
        ],
        "R2_has_inheritance" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::IsA,
                mid_kind,
            ),
            make_support_fact(
                &src[1],
                mid_kind,
                adam_reasoning::Predicate::Has,
                &derived.object.root,
            ),
        ],
        "R3_has_inheritance_via_part_of" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::Has,
                mid_kind,
            ),
            make_support_fact(
                &src[1],
                mid_kind,
                adam_reasoning::Predicate::PartOf,
                &derived.object.root,
            ),
        ],
        "R5_shared_is_a_target" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::IsA,
                mid_kind,
            ),
            make_support_fact(
                &src[1],
                &derived.object.root,
                adam_reasoning::Predicate::IsA,
                mid_kind,
            ),
        ],
        "R6_lives_in_via_part_of" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::LivesIn,
                place_mid,
            ),
            make_support_fact(
                &src[1],
                place_mid,
                adam_reasoning::Predicate::PartOf,
                &derived.object.root,
            ),
        ],
        "R7_goes_to_via_part_of" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::GoesTo,
                place_mid,
            ),
            make_support_fact(
                &src[1],
                place_mid,
                adam_reasoning::Predicate::PartOf,
                &derived.object.root,
            ),
        ],
        "R8_after_transitivity" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::After,
                time_mid,
            ),
            make_support_fact(
                &src[1],
                time_mid,
                adam_reasoning::Predicate::After,
                &derived.object.root,
            ),
        ],
        "R9_part_of_transitivity" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::PartOf,
                mid_kind,
            ),
            make_support_fact(
                &src[1],
                mid_kind,
                adam_reasoning::Predicate::PartOf,
                &derived.object.root,
            ),
        ],
        "R10_in_domain_inheritance" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::IsA,
                mid_kind,
            ),
            make_support_fact(
                &src[1],
                mid_kind,
                adam_reasoning::Predicate::InDomain,
                &derived.object.root,
            ),
        ],
        "R11_in_domain_shared_target" => vec![
            make_support_fact(
                &src[0],
                &derived.subject.root,
                adam_reasoning::Predicate::InDomain,
                mid_kind,
            ),
            make_support_fact(
                &src[1],
                &derived.object.root,
                adam_reasoning::Predicate::InDomain,
                mid_kind,
            ),
        ],
        _ => vec![make_support_fact(
            &src[0],
            "қолдау",
            adam_reasoning::Predicate::IsA,
            "ұғым",
        )],
    }
}

fn make_support_fact(
    source: &adam_reasoning::FactSource,
    subject: &str,
    predicate: adam_reasoning::Predicate,
    object: &str,
) -> adam_reasoning::Fact {
    adam_reasoning::Fact {
        subject: adam_reasoning::SlotRef {
            surface: subject.into(),
            root: subject.into(),
            pos: "noun".into(),
        },
        predicate,
        object: adam_reasoning::SlotRef {
            surface: object.into(),
            root: object.into(),
            pos: "noun".into(),
        },
        pattern: "test_support".into(),
        source: source.clone(),
        confidence: adam_reasoning::ConfidenceKind::Grammar,
        raw_text: format!("{subject} {} {object}", predicate.as_str()),
    }
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
fn greeting_plus_how_are_you_routes_to_how_are_you() {
    assert_response_with_toml(
        "Сәлем қалайсың.",
        &[
            "жақсымын, рахмет",
            "жаман емеспін",
            "жақсы, ал сіз қалайсыз",
        ],
    );
}

#[test]
fn response_ask_how_are_you_missing_h_variant() {
    assert_response_with_toml(
        "Қалдарыңыз қалай?",
        &[
            "жақсымын, рахмет",
            "жаман емеспін",
            "жақсы, ал сіз қалайсыз",
        ],
    );
}

#[test]
fn response_statement_of_wellbeing() {
    assert_response_with_toml(
        "жақсымын",
        &["жақсы екен", "оны естігеніме қуаныштымын", "қуанып қалдым"],
    );
}

#[test]
fn response_ask_name() {
    assert_response_with_toml("атың кім", &["менің атым адам", "мені адам деп атайды"]);
}

#[test]
fn response_ask_name_polite() {
    assert_response_with_toml("атыңыз кім", &["менің атым адам", "мені адам деп атайды"]);
}

#[test]
fn response_ask_name_with_known_user_profile() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);

    for seed in 0..16u64 {
        let out = conv.turn("атыңыз кім", &lex, &repo, seed);
        assert!(
            [
                "сіздің атыңыз Дәулет",
                "мен сізді Дәулет деп білемін",
                "Дәулет деп танысқан едіңіз",
            ]
            .contains(&out.as_str()),
            "seed={seed} unexpected known-user AskName output: {out:?}"
        );
    }
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
            "сәлем, Дәулет",
            "Дәулет, танысқаныма қуаныштымын",
            "Дәулетпен танысқаныма қуаныштымын",
            "Дәулет деген атыңызды есте сақтаймын",
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
    // v4.4.9 — every variant in `statement_of_age` interpolates
    // `{age}`. Pre-v4.4.9 «жақсы жас» / «қуатты кезеңіңіз» were
    // bare; the v4.4.9 rewrite prepended the slot to make the
    // family seed-uniform on slot-echo. Promoted aspirational
    // dialog `age_statement_acknowledged` to canonical at v4.4.9.
    assert_response_with_toml(
        "менің жасым отыз",
        &[
            "30 — жақсы жас",
            "30 — қуатты кезеңіңіз",
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
fn response_ask_location_with_known_user_profile() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен Алматыданмын", &lex, &repo, 0);

    for seed in 0..16u64 {
        let out = conv.turn("қайда тұрасыз", &lex, &repo, seed);
        assert!(
            [
                "сіз Алматыда тұрасыз",
                "менің білуімше, мекеніңіз Алматы",
                "сіз Алматы жақтан екенсіз",
            ]
            .contains(&out.as_str()),
            "seed={seed} unexpected known-user AskLocation output: {out:?}"
        );
    }
}

#[test]
fn response_ask_location_with_known_geo_feature_profile() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен каспий жақтанмын", &lex, &repo, 0);

    for seed in 0..16u64 {
        let out = conv.turn("қайда тұрасыз", &lex, &repo, seed);
        assert!(
            [
                "менің білуімше, сіз Каспий жақтансыз",
                "сіз Каспий маңынан екенсіз",
                "Каспий — теңіз; соған жақын жақтансыз",
            ]
            .contains(&out.as_str()),
            "seed={seed} unexpected known-geo AskLocation output: {out:?}"
        );
    }
}

#[test]
fn response_statement_of_location() {
    // v4.4.9 — every variant in `statement_of_location`
    // interpolates `{city}`. Pre-v4.4.9 the bare «түсіндім» at
    // index 0 broke the seed-0 read of the v4.4.6 REPL replay
    // battery; rewritten as «{city} екен, түсіндім».
    assert_response_with_toml(
        "мен Алматыданмын",
        &[
            "Алматы екен, түсіндім",
            "Алматы жақсы мекен екен",
            "Алматы туралы естігенім бар",
            "Алматыда тұратыныңызды түсіндім",
            "мекеніңіз Алматы екенін ұқтым",
            "Алматыда екеніңізді есте сақтаймын",
            "Алматы жақтан екеніңізді білдім",
        ],
    );
}

#[test]
fn response_statement_of_geo_feature_location() {
    assert_response_with_toml(
        "мен каспий жақтанмын",
        &[
            "Каспий жақтан екеніңізді ұқтым",
            "Каспий маңынан екеніңізді түсіндім",
            "Каспий — теңіз; соған жақын жақтан екеніңізді ұқтым",
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
fn response_ask_occupation_with_known_user_profile() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен мұғаліммін", &lex, &repo, 0);

    for seed in 0..16u64 {
        let out = conv.turn("немен айналысасың", &lex, &repo, seed);
        assert!(
            [
                "сіз мұғалім болып еңбек етіп жүрсіз",
                "менің білуімше, мамандығыңыз мұғалім",
                "мұғалім — сіздің кәсібіңіз",
            ]
            .contains(&out.as_str()),
            "seed={seed} unexpected known-user AskOccupation output: {out:?}"
        );
    }
}

#[test]
fn response_statement_of_occupation() {
    // v4.4.9 — every variant in `statement_of_occupation`
    // interpolates `{occupation}`. Pre-v4.4.9 «түсіндім» and
    // «еңбегіңізге сәттілік» were bare; rewritten to prepend the
    // occupation so the family is seed-uniform on slot-echo.
    assert_response_with_toml(
        "мен мұғаліммін",
        &[
            "мұғалім екен, түсіндім",
            "мұғалім еңбегіңізге сәттілік",
            "мұғалім екеніңізді түсіндім",
            "мұғалім болу жауапты іс",
            "сіз мұғалім болып еңбек етіп жүр екенсіз",
            "мұғалімдер еңбегі маңызды",
            "мұғалімдер қоғамға қажет",
            "мұғалімге табыс тілеймін",
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
        &["рахмет", "жылы сөзіңізге рақмет", "сіз де жақсы жансыз"],
    );
}

#[test]
fn response_request() {
    assert_response_with_toml(
        "көмектесіңізші",
        &["әрине, айтыңыз", "қалай көмектесе аламын", "тыңдап отырмын"],
    );
}

#[test]
fn response_well_wishes() {
    assert_response_with_toml(
        "сәттілік",
        &["сізге де", "сәттілік сізге де", "ізгі тілегіңізге рақмет"],
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
            "ренжімеймін, бірақ сыпайы сөйлесейік",
            "мен адамды құрметтеймін",
            "қаласаңыз, әңгімені сабырмен жалғастырайық",
            "сөйлесуді сыпайы жалғастырсақ дұрыс болар еді",
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
    assert_eq!(conv.session.get("city_id"), Some(&"geo_kz_004".to_string()));
    assert_eq!(conv.session.get("geo_kind"), Some(&"қала".to_string()));
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
        if out == "сәлем, Дәулет" {
            saw_personalised = true;
        }
    }
    assert!(
        saw_personalised,
        "expected at least one seed in 0..16 to pick \"сәлем, Дәулет\""
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
    assert_eq!(conv.session.get("city_id"), Some(&"geo_kz_004".to_string()));
    assert_eq!(conv.session.get("geo_kind"), Some(&"қала".to_string()));

    let _ = conv.turn("мен мұғаліммін", &lex, &repo, 0);
    assert_eq!(conv.session.get("occupation"), Some(&"мұғалім".to_string()));
}

#[test]
fn conversation_stores_places_by_canonical_geo_id_in_belief_memory() {
    use adam_dialog::USER_SELF_KEY;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("мен Алматыданмын", &lex, &repo, 0);

    let place = conv
        .belief
        .entities
        .get("geo_kz_004")
        .expect("canonical geo entity should be remembered by id");
    assert_eq!(place.canonical_id.as_deref(), Some("geo_kz_004"));
    assert_eq!(place.root, "Алматы");
    assert_eq!(place.kind, adam_dialog::EntityKind::Place);
    assert!(
        !conv.belief.entities.contains_key("Алматы"),
        "place memory should use canonical geo id as the entity key"
    );
    assert_eq!(
        conv.belief
            .active_fact(USER_SELF_KEY, "city")
            .map(|f| f.object.as_str()),
        Some("Алматы")
    );
}

#[test]
fn trace_faithfulness_passes_for_direct_answer_output() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);
    let (out, trace) = conv.turn_with_trace("атың кім", &lex, &repo, 1);

    let quality = audit_response(&out);
    assert!(quality.is_clean(), "quality issues: {:?}", quality.issues);

    let faithfulness = audit_trace_faithfulness(&out, &trace);
    assert!(
        faithfulness.is_clean(),
        "faithfulness issues: {:?}",
        faithfulness.issues
    );
}

#[test]
fn typed_faithfulness_passes_for_grounded_graph_answer() {
    use adam_reasoning::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    conv.extracted_facts = vec![Fact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::PartOf,
        object: SlotRef {
            surface: "күн жүйесі".into(),
            root: "күн жүйесі".into(),
            pos: "noun".into(),
        },
        pattern: "world_core".into(),
        source: FactSource {
            pack: "world_core/astronomy.jsonl".into(),
            sample_id: "earth_001".into(),
        },
        confidence: ConfidenceKind::HumanApproved,
        raw_text: "Жер Күн жүйесінің құрамына кіреді".into(),
    }];

    let (out, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 0);

    let quality = audit_response(&out);
    assert!(quality.is_clean(), "quality issues: {:?}", quality.issues);

    let faithfulness = audit_trace_faithfulness(&out, &trace);
    assert!(
        faithfulness.is_clean(),
        "faithfulness issues: {:?}",
        faithfulness.issues
    );

    let typed = audit_typed_faithfulness(&trace);
    assert!(typed.is_clean(), "typed issues: {:?}", typed.issues);

    let graph = audit_graph_admissibility(&trace);
    assert!(graph.is_clean(), "graph issues: {:?}", graph.issues);
}

#[test]
fn typed_faithfulness_flags_missing_reasoner_support() {
    use adam_dialog::TypedFaithfulnessIssue;
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let (_out, mut trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 0);
    trace.tool_calls.clear();

    let typed = audit_typed_faithfulness(&trace);
    assert!(
        typed
            .issues
            .contains(&TypedFaithfulnessIssue::ReasoningChainMissingDerivedSupport),
        "expected missing derived support issue, got {:?}",
        typed.issues
    );
}

#[test]
fn graph_admissibility_flags_rule_predicate_mismatch() {
    use adam_reasoning::Predicate;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let derived = vec![adam_reasoning::reasoner::DerivedFact {
        subject: adam_reasoning::SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: adam_reasoning::SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![adam_reasoning::FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: adam_reasoning::ConfidenceKind::RuleInferred,
    }];
    let (_out, mut trace) = Conversation::new()
        .with_reasoning_chains(support_facts_for_derived(&derived), derived)
        .turn_with_trace("жер туралы айтшы", &lex, &repo, 0);

    for result in &mut trace.tool_calls {
        for evidence in &mut result.evidence {
            if let adam_dialog::ToolEvidence::DerivedFact { predicate, .. } = evidence {
                *predicate = Predicate::LivesIn;
            }
        }
    }

    let graph = audit_graph_admissibility(&trace);
    assert!(
        graph
            .issues
            .contains(&GraphAdmissibilityIssue::DerivedFactRulePredicateMismatch),
        "expected rule/predicate mismatch, got {:?}",
        graph.issues
    );
}

#[test]
fn graph_admissibility_flags_support_pattern_mismatch() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let derived = vec![adam_reasoning::reasoner::DerivedFact {
        subject: adam_reasoning::SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: adam_reasoning::Predicate::IsA,
        object: adam_reasoning::SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![
            adam_reasoning::FactSource {
                pack: "world_core/celestial.jsonl".into(),
                sample_id: "sky_01".into(),
            },
            adam_reasoning::FactSource {
                pack: "world_core/celestial.jsonl".into(),
                sample_id: "sky_02".into(),
            },
        ],
        confidence: adam_reasoning::ConfidenceKind::RuleInferred,
    }];
    let (_out, mut trace) = Conversation::new()
        .with_reasoning_chains(support_facts_for_derived(&derived), derived)
        .turn_with_trace("жер туралы айтшы", &lex, &repo, 0);

    for result in &mut trace.tool_calls {
        for evidence in &mut result.evidence {
            if let adam_dialog::ToolEvidence::DerivedFact { support_chain, .. } = evidence {
                if support_chain.len() >= 2 {
                    support_chain[1].subject = "басқа".into();
                }
            }
        }
    }

    let graph = audit_graph_admissibility(&trace);
    assert!(
        graph
            .issues
            .contains(&GraphAdmissibilityIssue::DerivedFactSupportPatternMismatch),
        "expected support-pattern mismatch, got {:?}",
        graph.issues
    );
}

#[test]
fn trace_faithfulness_flags_reasoning_leak_after_verification_failure() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let _ = conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    let _ = conv.turn("мен астанада тұрамын", &lex, &repo, 1);
    let (_out, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 2);

    let leaked = match &trace.intent_after_injection {
        Intent::Unknown {
            reasoning_chain: Some(chain),
            ..
        } => chain.clone(),
        other => panic!("expected injected reasoning chain, got {other:?}"),
    };
    let faithfulness = audit_trace_faithfulness(&leaked, &trace);
    assert!(
        faithfulness
            .issues
            .contains(&TraceFaithfulnessIssue::EvidenceLeakAfterVerificationFailure),
        "expected evidence leak issue, got {:?}",
        faithfulness.issues
    );
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
    // {city|locative} template, producing a locative acknowledgement.
    let mut saw_locative = false;
    for seed in 0..32u64 {
        let out = conv.turn("мен Алматыданмын", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out == "Алматыда тұратыныңызды түсіндім" || out == "Алматыда екеніңізді есте сақтаймын"
        {
            saw_locative = true;
        }
    }
    assert!(
        saw_locative,
        "expected at least one seed in 0..32 to synthesise a locative acknowledgement for Алматы"
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
        if out == "мұғалімдер еңбегі маңызды" {
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
        if out == "сәлем, Дәулет, Алматыдан бәрі жақсы ма" {
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

#[test]
fn unknown_with_grounded_fact_uses_nonquoted_verbalizer() {
    use adam_reasoning::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let extracted = vec![Fact {
        subject: SlotRef {
            surface: "Қазақстан".into(),
            root: "қазақстан".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "ел".into(),
            root: "ел".into(),
            pos: "noun".into(),
        },
        pattern: "world_core".into(),
        source: FactSource {
            pack: "world_core/geography_kz.jsonl".into(),
            sample_id: "geo_001".into(),
        },
        confidence: ConfidenceKind::HumanApproved,
        raw_text: "Қазақстан — Орталық Азиядағы ел.".into(),
    }];

    let mut conv = Conversation::new().with_reasoning_chains(extracted, vec![]);
    let out = conv.turn("Қазақстан туралы айтшы", &lex, &repo, 0);
    assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
    assert!(
        !out.contains("«") && !out.contains("»"),
        "grounded-fact verbalizer should not quote direct facts, got: {out:?}"
    );
    assert!(
        out.contains("Қазақстан — Орталық Азиядағы ел."),
        "grounded fact should surface directly, got: {out:?}"
    );
}

#[test]
fn quantity_question_prefers_has_quantity_graph_fact() {
    use adam_reasoning::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let extracted = vec![
        Fact {
            subject: SlotRef {
                surface: "Күн жүйесі".into(),
                root: "күн жүйесі".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::PartOf,
            object: SlotRef {
                surface: "Құс жолы".into(),
                root: "құс жолы".into(),
                pos: "noun".into(),
            },
            pattern: "world_core".into(),
            source: FactSource {
                pack: "world_core/astronomy.jsonl".into(),
                sample_id: "astro_022".into(),
            },
            confidence: ConfidenceKind::HumanApproved,
            raw_text: "Күн жүйесі — Құс жолының бөлігі.".into(),
        },
        Fact {
            subject: SlotRef {
                surface: "Күн жүйесі".into(),
                root: "күн жүйесі".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::HasQuantity,
            object: SlotRef {
                surface: "ғаламшар".into(),
                root: "ғаламшар".into(),
                pos: "noun".into(),
            },
            pattern: "world_core".into(),
            source: FactSource {
                pack: "world_core/astronomy.jsonl".into(),
                sample_id: "astro_026".into(),
            },
            confidence: ConfidenceKind::HumanApproved,
            raw_text: "Күн жүйесінде сегіз ғаламшар бар.".into(),
        },
    ];

    let mut conv = Conversation::new().with_reasoning_chains(extracted, vec![]);
    let out = conv.turn("Күн жүйесінде қанша планета бар?", &lex, &repo, 0);
    assert!(
        out.contains("Күн жүйесінде сегіз ғаламшар бар."),
        "quantity question should surface the quantity fact, got: {out:?}"
    );
    assert!(
        !out.contains("Құс жолының бөлігі"),
        "quantity question should not fall back to a generic part_of fact, got: {out:?}"
    );
}

#[test]
fn border_question_prefers_border_graph_fact() {
    use adam_reasoning::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let extracted = vec![
        Fact {
            subject: SlotRef {
                surface: "Қазақстан".into(),
                root: "қазақстан".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: SlotRef {
                surface: "ел".into(),
                root: "ел".into(),
                pos: "noun".into(),
            },
            pattern: "world_core".into(),
            source: FactSource {
                pack: "world_core/geography_kz.jsonl".into(),
                sample_id: "geo_kz_002".into(),
            },
            confidence: ConfidenceKind::HumanApproved,
            raw_text: "Қазақстан — Орталық Азиядағы ел.".into(),
        },
        Fact {
            subject: SlotRef {
                surface: "Қазақстан".into(),
                root: "қазақстан".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::RelatedTo,
            object: SlotRef {
                surface: "көрші елдер".into(),
                root: "көрші елдер".into(),
                pos: "noun".into(),
            },
            pattern: "world_core".into(),
            source: FactSource {
                pack: "world_core/geography_kz.jsonl".into(),
                sample_id: "geo_kz_027b".into(),
            },
            confidence: ConfidenceKind::HumanApproved,
            raw_text:
                "Қазақстан Ресеймен, Қытаймен, Қырғызстанмен, Өзбекстанмен және Түрікменстанмен шектеседі."
                    .into(),
        },
    ];

    let mut conv = Conversation::new().with_reasoning_chains(extracted, vec![]);
    let out = conv.turn("Қазақстан қандай елдермен шектеседі?", &lex, &repo, 0);
    assert!(
        out.contains("Қазақстан Ресеймен, Қытаймен, Қырғызстанмен, Өзбекстанмен және Түрікменстанмен шектеседі."),
        "border question should surface the curated border fact, got: {out:?}"
    );
    assert!(
        !out.contains("Орталық Азиядағы ел"),
        "border question should not fall back to a generic is_a fact, got: {out:?}"
    );
}

/// v2.7: when derived facts are attached, `Intent::Unknown` whose
/// noun_hint appears in a derivation should route to the
/// `unknown.with_derived_chain` family. Trust invariant — every
/// template in that family contains the marker stem «байланыс-»
/// so users can distinguish reasoning citations from corpus quotes
/// at the textual level alone.
#[test]
fn unknown_with_reasoning_chain_cites_derivation() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    // Synthetic derivation: кітап RelatedTo ілім, as if R5 had fired.
    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::RelatedTo,
        object: SlotRef {
            surface: "ілім".into(),
            root: "ілім".into(),
            pos: "noun".into(),
        },
        rule_id: "R5_shared_is_a_target".into(),
        source_chain: vec![
            FactSource {
                pack: "proverbs".into(),
                sample_id: "p_003".into(),
            },
            FactSource {
                pack: "common_voice".into(),
                sample_id: "cv_047".into(),
            },
        ],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let out = conv.turn("кітап туралы бірдеңе айт", &lex, &repo, 0);
    assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
    // Trust invariant: the marker stem MUST appear when a chain fires.
    assert!(
        out.contains("байланыс"),
        "unknown.with_derived_chain family must include «байланыс-» marker, got: {out:?}"
    );
    // The derived pair (кітап, ілім) should appear.
    assert!(
        out.contains("кітап") && out.contains("ілім"),
        "rendered chain must name both roots, got: {out:?}"
    );
}

/// v3.8.5: the reasoning-chain renderer must produce morphologically
/// valid Kazakh case forms — not dash-concatenated strings like
/// `атау-ға`. Pre-v3.8.5 the `Has` / `PartOf` / `Causes` / `After` /
/// `HasQuantity` / `InDomain` arms appended `"-ға"` / `"-дың"` /
/// `"-нен"` / `"-мен"` etc. verbatim, violating vowel harmony. v3.8.5
/// routes every case suffix through `synthesise_noun`.
#[test]
fn reasoning_chain_uses_fst_synthesis_not_dash_concatenation() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    // Has predicate — the renderer inflects the object root as dative.
    // Root "атау" + dative = "атауға". Pre-v3.8.5 would emit "атау-ға".
    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::Has,
        object: SlotRef {
            surface: "атау".into(),
            root: "атау".into(),
            pos: "noun".into(),
        },
        rule_id: "R2_has_inheritance".into(),
        source_chain: vec![FactSource {
            pack: "proverbs".into(),
            sample_id: "p_003".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let out = conv.turn("кітап туралы бірдеңе айт", &lex, &repo, 0);

    // Trust invariant: marker still fires.
    assert!(out.contains("байланыс"), "marker missing, got: {out:?}");
    // Positive: properly synthesised dative form of "атау" is "атауға".
    // (Back-vowel harmony: атау's last vowel /a/ → dative suffix /ғa/.)
    assert!(
        out.contains("атауға") || out.contains("атау "),
        "expected FST-synthesised dative «атауға», got: {out:?}"
    );
    // Negative: no dash-concatenated form.
    assert!(
        !out.contains("атау-ға"),
        "dash-concatenation leaked (атау-ға found in: {out:?})"
    );
    assert!(
        !out.contains("атау-дың") && !out.contains("атау-ды"),
        "stale dash-concatenation pattern, got: {out:?}"
    );
}

/// v2.7 negative invariant: without any derived facts attached, the
/// reasoning-chain path must NEVER fire. Guards against false-positive
/// "this is inferred" claims — a trust-critical corollary of v1.9.5's
/// verbatim_mode_never_claims_adaptation.
#[test]
fn unknown_without_derived_facts_never_claims_chain() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    for seed in 0..32u64 {
        let out = conv.turn("кітап туралы бірдеңе айт", &lex, &repo, seed);
        assert!(
            !out.contains("байланыс"),
            "no derived facts → chain marker MUST NOT appear, got: {out:?}"
        );
    }
}

/// v4.0.22 — Codex-reranker picks fully-curated chain over text-only
/// when both are available for the same noun. Pre-v4.0.22 was "first
/// match wins" which could surface noisy text-extracted chains.
#[test]
fn reranker_prefers_curated_over_text_only() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let text_only = DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::RelatedTo,
        object: SlotRef {
            surface: "өзен".into(),
            root: "өзен".into(),
            pos: "noun".into(),
        },
        rule_id: "R5_shared_is_a_target".into(),
        source_chain: vec![
            FactSource {
                pack: "wikipedia_kz_pack.json".into(),
                sample_id: "wiki_1".into(),
            },
            FactSource {
                pack: "wikipedia_kz_pack.json".into(),
                sample_id: "wiki_2".into(),
            },
        ],
        confidence: ConfidenceKind::RuleInferred,
    };
    let curated = DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "құрал".into(),
            root: "құрал".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/tools_household.jsonl".into(),
            sample_id: "tool_014".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    };
    // Feed text-only FIRST so "first match wins" would pick it.
    let derived = vec![text_only, curated];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let out = conv.turn("кітап туралы бірдеңе айт", &lex, &repo, 0);
    assert!(
        out.contains("құрал"),
        "reranker should pick curated chain (construct/tool), got: {out:?}"
    );
    assert!(
        !out.contains("өзен"),
        "reranker should NOT pick text-only river chain, got: {out:?}"
    );
}

/// v4.0.24 — Codex v4.0.23 re-review case #1: when the reranker sees
/// multiple tied curated R1/R10 candidates for the same noun, the IsA
/// predicate boost (+2) must prevent InDomain / Has / RelatedTo
/// picks from winning via canonical-triple tie-break. Pre-v4.0.24
/// for «немере туралы айтшы» the winner was InDomain(немере, зоология)
/// instead of IsA(немере, адам). This test asserts the predicate-level
/// preference.
#[test]
fn reranker_prefers_is_a_over_other_predicates_on_tied_score() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let in_domain = DerivedFact {
        subject: SlotRef {
            surface: "немере".into(),
            root: "немере".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::InDomain,
        object: SlotRef {
            surface: "зоология".into(),
            root: "зоология".into(),
            pos: "noun".into(),
        },
        rule_id: "R10_in_domain_inheritance".into(),
        source_chain: vec![
            FactSource {
                pack: "world_core/kinship_extended.jsonl".into(),
                sample_id: "kin_011".into(),
            },
            FactSource {
                pack: "world_core/animals.jsonl".into(),
                sample_id: "anm_022".into(),
            },
        ],
        confidence: ConfidenceKind::RuleInferred,
    };
    let is_a = DerivedFact {
        subject: SlotRef {
            surface: "немере".into(),
            root: "немере".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "адам".into(),
            root: "адам".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![
            FactSource {
                pack: "world_core/kinship_extended.jsonl".into(),
                sample_id: "kin_011".into(),
            },
            FactSource {
                pack: "world_core/kinship_extended.jsonl".into(),
                sample_id: "kin_001".into(),
            },
        ],
        confidence: ConfidenceKind::RuleInferred,
    };
    // Feed InDomain first — before the IsA predicate bonus, the
    // canonical-triple reverse tie-break would have picked InDomain
    // (InDomain < IsA lexicographically → lower triple wins).
    let derived = vec![in_domain, is_a];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let out = conv.turn("немере туралы айтшы", &lex, &repo, 0);
    assert!(
        out.contains("адам"),
        "reranker should pick IsA(немере→адам), got: {out:?}"
    );
    assert!(
        !out.contains("зоология"),
        "reranker should NOT pick InDomain(немере→зоология), got: {out:?}"
    );
}

/// v4.0.27 — `Conversation::turn` writes user-supplied entities to
/// the new structured `belief` state alongside the legacy flat
/// `session` map. Codex v4.0.26 roadmap Phase 1.
#[test]
fn user_statement_populates_belief_alongside_session() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("менің атым Дәулет", &lex, &repo, 0);

    // Legacy session map still works.
    assert_eq!(conv.session.get("name").map(String::as_str), Some("Дәулет"));
    // New belief state captured the same fact with provenance.
    use adam_dialog::{ConfidenceBand, FactStatus, Provenance, USER_SELF_KEY};
    let fact = conv
        .belief
        .active_fact(USER_SELF_KEY, "name")
        .expect("belief must record name fact");
    assert_eq!(fact.object, "Дәулет");
    assert_eq!(fact.status, FactStatus::Active);
    assert_eq!(fact.confidence, ConfidenceBand::Confirmed);
    assert!(matches!(fact.provenance, Provenance::UserStatement { .. }));
    assert!(
        conv.belief.contradictions.is_empty(),
        "single user statement must not create contradictions"
    );
}

/// v4.0.27 — two contradictory location statements flag a conflict
/// in belief WITHOUT breaking the legacy session-slot overwrite path
/// (session still holds the latest value; the belief layer preserves
/// both and adds a PendingQuestion).
#[test]
fn contradictory_city_statements_produce_belief_conflict() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    conv.turn("мен астанада тұрамын", &lex, &repo, 1);

    use adam_dialog::{FactStatus, QuestionNature, USER_SELF_KEY};
    // Legacy: session has the latest value (no conflict awareness).
    // Exact capitalisation is whatever the detector emits; we only
    // care that the overwrite happened and matches the belief path.
    let session_city = conv
        .session
        .get("city")
        .cloned()
        .expect("session must carry a city after statement");
    assert!(
        session_city.to_lowercase() == "астана",
        "session city should match latest statement, got {session_city:?}"
    );

    // Belief: both facts survived, both Contested, with a conflict
    // log + pending question.
    let city_facts = conv
        .belief
        .facts
        .iter()
        .filter(|f| f.predicate == "city")
        .collect::<Vec<_>>();
    assert_eq!(city_facts.len(), 2, "both city statements must persist");
    assert!(city_facts.iter().all(|f| f.status == FactStatus::Contested));
    assert_eq!(conv.belief.contradictions.len(), 1);
    let c = &conv.belief.contradictions[0];
    assert_eq!(c.subject, USER_SELF_KEY);
    assert_eq!(c.predicate, "city");

    let pending = &conv.belief.pending_questions[0];
    match &pending.nature {
        QuestionNature::ContradictionToResolve {
            old_value,
            new_value,
            ..
        } => {
            assert!(
                old_value.to_lowercase() == "алматы",
                "first city must be алматы, got {old_value:?}"
            );
            assert!(
                new_value.to_lowercase() == "астана",
                "second city must be астана, got {new_value:?}"
            );
        }
        other => panic!("expected ContradictionToResolve, got {other:?}"),
    }
}

/// v4.0.29 — Codex roadmap Phase 2: asking about a topic installs a
/// `LearnAboutTopic` goal on the task state. Subsequent same-topic
/// turns keep the goal (continuity); a new topic switches it.
#[test]
fn turn_installs_learn_about_topic_goal_and_preserves_continuity() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("жер туралы айтшы", &lex, &repo, 0);
    use adam_dialog::{Goal, TaskStatus};
    match &conv.task.active_goal {
        Some(Goal::LearnAboutTopic { topic }) => assert_eq!(topic, "жер"),
        other => panic!("expected LearnAboutTopic/жер, got {other:?}"),
    }
    let first_turn = conv.task.goal_set_at_turn;
    // No reasoning facts attached to this Conversation → evidence
    // injection can't fire → status stays in GatheringEvidence.
    // The dedicated `ready_to_answer_reachable_with_reasoning_chain`
    // test covers the ReadyToAnswer path with facts attached.
    assert_eq!(conv.task.status, TaskStatus::GatheringEvidence);

    // Same topic asked again — goal must persist, set-turn unchanged.
    conv.turn("жер туралы айтшы", &lex, &repo, 1);
    assert_eq!(conv.task.goal_set_at_turn, first_turn);

    // Different topic — goal switches, set-turn advances.
    conv.turn("күн туралы айтшы", &lex, &repo, 2);
    match &conv.task.active_goal {
        Some(Goal::LearnAboutTopic { topic }) => assert_eq!(topic, "күн"),
        other => panic!("expected LearnAboutTopic/күн after switch, got {other:?}"),
    }
    assert_ne!(conv.task.goal_set_at_turn, first_turn);
}

/// v4.0.29 — belief contradiction routes task status to `Blocked`
/// (Codex v4.0.28 invariant: `active_fact() == None` after a
/// contradiction is legitimate state; Phase 2 reflects that in
/// `TaskStatus::Blocked` rather than pretending we have an answer).
#[test]
fn belief_contradiction_blocks_task() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    conv.turn("мен астанада тұрамын", &lex, &repo, 1);

    use adam_dialog::{TaskStatus, USER_SELF_KEY};
    assert!(conv.belief.active_fact(USER_SELF_KEY, "city").is_none());
    assert_eq!(conv.task.status, TaskStatus::Blocked);
}

/// v4.0.29 — social intents don't clobber an existing goal. «Thanks»
/// mid-topic keeps the `LearnAboutTopic` goal so the next turn can
/// continue it.
#[test]
fn social_intent_does_not_clobber_active_goal() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("жер туралы айтшы", &lex, &repo, 0);
    let goal_before = conv.task.active_goal.clone();
    conv.turn("рахмет", &lex, &repo, 1);
    assert_eq!(
        conv.task.active_goal, goal_before,
        "social turn must not erase an active goal"
    );
}

/// v4.0.29 — `TurnTrace` exposes the task digest so `adam_chat
/// --trace` can show goal + status without dumping the full state.
#[test]
fn turn_with_trace_surfaces_task_digest() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    let (_, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 0);
    assert!(trace.task_digest.has_goal);
    assert_eq!(trace.task_digest.goal_variant, Some("LearnAboutTopic"));
    use adam_dialog::TaskStatus;
    // No reasoning facts attached → GatheringEvidence (evidence
    // injection passes don't fire). ReadyToAnswer is covered
    // separately in `ready_to_answer_reachable_with_reasoning_chain`.
    assert_eq!(trace.task_digest.status, TaskStatus::GatheringEvidence);
}

/// v4.0.31 Phase 3 — `Conversation::turn_with_trace` classifies each
/// turn into an `ActionPlan`. With a reasoning chain injected the
/// action must be `RunReasoner`; with only a retrieval example it
/// must be `RetrieveEvidence`. Reply text is still byte-identical to
/// v4.0.30 — the planner is a classifier in Phase 3.
#[test]
fn action_planner_classifies_reasoning_chain_intent_as_run_reasoner() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let (_, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 0);
    use adam_dialog::{Action, OutputKind};
    assert_eq!(trace.action_digest.action, Action::RunReasoner);
    assert_eq!(
        trace.action_digest.expected_output,
        OutputKind::DerivedAnswer
    );
    assert!(
        !trace.action_plan.rationale.is_empty(),
        "action plan must carry rationale"
    );
    assert_eq!(
        conv.task.last_action.as_ref().map(|p| p.action),
        Some(Action::RunReasoner),
        "last_action must be persisted on task state"
    );
}

/// v4.0.31 — belief contradiction routes action to
/// `CheckContradiction` even when the current turn has evidence
/// attached. Contradictions dominate everything else.
#[test]
fn action_planner_surfaces_contradiction_over_evidence() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    conv.turn("мен астанада тұрамын", &lex, &repo, 1);
    let (_, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 2);

    use adam_dialog::{Action, OutputKind};
    assert_eq!(trace.action_digest.action, Action::CheckContradiction);
    assert_eq!(
        trace.action_digest.expected_output,
        OutputKind::ClarifyingQuestion
    );
}

/// v4.0.31 — social intents (greeting, thanks, …) route to
/// `Action::Social` and bypass the cognitive stack.
#[test]
fn action_planner_social_intent() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let (_, trace) = conv.turn_with_trace("рахмет", &lex, &repo, 0);
    use adam_dialog::{Action, OutputKind};
    assert_eq!(trace.action_digest.action, Action::Social);
    assert_eq!(
        trace.action_digest.expected_output,
        OutputKind::SocialPleasantry
    );
}

/// v4.0.34 Phase 5 part 2 — conflict-surfacing template family
/// actually fires when belief has a contradiction + Unknown intent
/// with noun_hint. Pre-v4.0.34 the gate stripped evidence and the
/// reply was a generic noun-echo; post-v4.0.34 the user sees the
/// two conflicting claims + a clarifying question.
#[test]
fn conflict_surfaces_explicit_clarification_template() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    conv.turn("мен астанада тұрамын", &lex, &repo, 1);
    let out = conv.turn("жер туралы айтшы", &lex, &repo, 2);

    // Must contain both conflicting values — the whole point is
    // making the user aware of the disagreement.
    assert!(
        out.to_lowercase().contains("алматы"),
        "conflict reply must cite old value, got: {out:?}"
    );
    assert!(
        out.to_lowercase().contains("астана"),
        "conflict reply must cite new value, got: {out:?}"
    );
    // Must route to clarification — some question marker / kazakh
    // clarification cue. All three `unknown.conflicted` templates
    // end with a question; any of «?» / «дұрыс» / «ма» / «нақтылай»
    // suffices.
    let markers = ["?", "дұрыс", "нақтылай"];
    assert!(
        markers.iter().any(|m| out.contains(m)),
        "conflict reply must look like a clarifying question, got: {out:?}"
    );
    // Sanity — must NOT still be the reasoning-chain marker.
    assert!(
        !out.contains("байланыс"),
        "conflict path must not render a reasoning chain, got: {out:?}"
    );
}

/// v4.0.34 — Kazakh predicate mapping in conflict slots. Raw
/// `c.predicate` would be the English slot key («city»), which would
/// read awkwardly in a Kazakh sentence. The turn loop maps it to
/// «қалаңыз» before rendering.
#[test]
fn conflict_predicate_renders_in_kazakh() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    conv.turn("мен астанада тұрамын", &lex, &repo, 1);
    let out = conv.turn("жер туралы айтшы", &lex, &repo, 2);
    // At least one of the three templates mentions the predicate.
    // When it does, it must be Kazakh, not the raw English slot key.
    if out.contains("туралы") || out.contains("екі жауап") {
        assert!(
            !out.to_lowercase().contains("city"),
            "conflict reply must not leak raw English predicate `city`, got: {out:?}"
        );
    }
}

/// v4.0.33 Phase 5 part 1 — `EpistemicStatus` maps each kind of turn
/// to the right confidence band end-to-end. Reply text unchanged
/// from v4.0.32 (the policy is a classifier in v4.0.33); v4.0.34
/// will consume the status at rendering time.
#[test]
fn epistemic_status_classifies_kinds_of_turn() {
    use adam_dialog::EpistemicStatus;
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    // Clean reasoning-chain turn → Derived.
    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let (_, t1) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 0);
    assert_eq!(t1.epistemic_status, EpistemicStatus::Derived);

    // Social turn → Certain.
    let (_, t2) = conv.turn_with_trace("рахмет", &lex, &repo, 1);
    assert_eq!(t2.epistemic_status, EpistemicStatus::Certain);

    // Contradiction → Conflicted.
    conv.turn("мен алматыда тұрамын", &lex, &repo, 2);
    conv.turn("мен астанада тұрамын", &lex, &repo, 3);
    let (_, t4) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 4);
    assert_eq!(t4.epistemic_status, EpistemicStatus::Conflicted);
}

/// v4.0.32 Phase 4 — Verifier gates evidence rendering when a belief
/// contradiction exists. Pre-v4.0.32 the dialog would happily surface
/// a reasoning chain about «жер» even while the user's own city was
/// contested. Post-v4.0.32 the gate strips the chain before template
/// rendering, so the reply falls back to the safe noun-echo.
#[test]
fn verifier_gates_reasoning_chain_under_belief_contradiction() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    conv.turn("мен астанада тұрамын", &lex, &repo, 1);
    let (out, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 2);

    assert!(
        !trace.verification.supported,
        "verifier must reject under unresolved contradiction, got {:?}",
        trace.verification
    );
    assert!(
        !out.contains("байланыс"),
        "gated output must not cite the reasoning chain, got: {out:?}"
    );
    if let adam_dialog::Intent::Unknown {
        reasoning_chain,
        example,
        ..
    } = &trace.intent_after_verification
    {
        assert!(reasoning_chain.is_none());
        assert!(example.is_none());
    } else {
        panic!(
            "expected Intent::Unknown, got {:?}",
            trace.intent_after_verification
        );
    }
}

/// v4.0.32 — Clean path: no contradictions → verifier passes,
/// reasoning chain renders as normal. Reply text byte-identical to
/// v4.0.31.
#[test]
fn verifier_passes_through_clean_reasoning_chain() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let (out, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 0);

    assert!(
        trace.verification.supported,
        "clean scenario must verify, got {:?}",
        trace.verification
    );
    assert!(
        out.contains("байланыс"),
        "clean path must render the reasoning-chain marker, got: {out:?}"
    );
}

/// Codex v4.0.31 review residual — integration coverage for
/// `Action::AnswerDirect`. Pre-v4.0.32 only unit coverage.
#[test]
fn action_planner_classifies_known_profile_question_as_answer_direct() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("менің атым Дәулет", &lex, &repo, 0);
    let (_, trace) = conv.turn_with_trace("менің атым кім", &lex, &repo, 1);
    if matches!(trace.intent_after_injection, adam_dialog::Intent::AskName) {
        use adam_dialog::{Action, OutputKind};
        assert_eq!(
            trace.action_digest.action,
            Action::AnswerDirect,
            "AskName with belief active_fact must route to AnswerDirect"
        );
        assert_eq!(
            trace.action_digest.expected_output,
            OutputKind::DirectAnswer
        );
    }
}

/// v4.0.30 — Codex v4.0.29 review #1 regression. Turn counter must
/// be a real monotone index, not `intent_history.len()` which caps at
/// `MAX_HISTORY = 32`. After >32 turns `goal_set_at_turn` must still
/// reflect the actual turn number.
#[test]
fn goal_set_at_turn_survives_intent_history_cap() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // Feed 35 social turns (no goal installed — Thanks/Farewell
    // etc. aren't profile/topic intents).
    for _ in 0..35 {
        conv.turn("рахмет", &lex, &repo, 0);
    }
    // Now install a goal on turn 35.
    conv.turn("жер туралы айтшы", &lex, &repo, 0);

    assert_eq!(
        conv.task.goal_set_at_turn,
        Some(35),
        "goal_set_at_turn must be the real turn index (35), got {:?}",
        conv.task.goal_set_at_turn
    );
    assert_eq!(
        conv.turn_counter, 36,
        "turn_counter must advance past MAX_HISTORY, got {}",
        conv.turn_counter
    );
}

/// v4.0.30 — `TaskStatus::ReadyToAnswer` actually reachable now.
/// Regression on Codex v4.0.29 review #2: the pre-v4.0.30 status
/// derivation never produced this variant.
#[test]
fn ready_to_answer_reachable_with_reasoning_chain() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    conv.turn("жер туралы айтшы", &lex, &repo, 0);
    use adam_dialog::TaskStatus;
    assert_eq!(
        conv.task.status,
        TaskStatus::ReadyToAnswer,
        "with injected reasoning chain the status must be ReadyToAnswer"
    );
}

/// v4.0.28 — Codex v4.0.27 review #1 regression at the integration
/// layer. Through `Conversation::turn`, the user sequence
/// `city=X → city=X → city=Y` must end with `active_fact() == None`
/// (contradiction detected, all prior actives retired). Pre-v4.0.28
/// the first copy remained `Active`, so any downstream consumer
/// reading `active_fact()` saw a stale winner that disagreed with
/// the conflict log.
#[test]
fn same_same_different_city_leaves_no_active_fact_via_conversation() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    conv.turn("мен алматыда тұрамын", &lex, &repo, 0);
    conv.turn("мен алматыда тұрамын", &lex, &repo, 1);
    conv.turn("мен астанада тұрамын", &lex, &repo, 2);

    use adam_dialog::{FactStatus, USER_SELF_KEY};
    let active_after = conv.belief.active_fact(USER_SELF_KEY, "city");
    assert!(
        active_after.is_none(),
        "after same→same→different, active_fact must be None, got {:?}",
        active_after
    );
    let city_statuses: Vec<FactStatus> = conv
        .belief
        .facts
        .iter()
        .filter(|f| f.predicate == "city")
        .map(|f| f.status)
        .collect();
    assert_eq!(
        city_statuses
            .iter()
            .filter(|s| **s == FactStatus::Active)
            .count(),
        0,
        "zero Active city facts expected; got statuses {:?}",
        city_statuses
    );
    assert_eq!(conv.belief.contradictions.len(), 1);
}

/// v4.0.27 — `TurnTrace` now carries a belief snapshot + digest so
/// `adam_chat --trace` can audit what the belief layer learned on
/// each turn.
#[test]
fn turn_with_trace_surfaces_belief_digest() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // First turn: state name. Trace should show belief picking up
    // one entity + one active fact.
    let (_, trace1) = conv.turn_with_trace("менің атым Дәулет", &lex, &repo, 0);
    assert!(trace1.belief_digest.entities >= 1);
    assert_eq!(trace1.belief_digest.facts_total, 1);
    assert_eq!(trace1.belief_digest.facts_active, 1);
    assert_eq!(trace1.belief_digest.contradictions, 0);

    // Second turn: introduce a conflict. Digest must reflect both
    // facts contested + 1 conflict logged.
    conv.turn_with_trace("мен алматыда тұрамын", &lex, &repo, 1);
    let (_, trace3) = conv.turn_with_trace("мен астанада тұрамын", &lex, &repo, 2);
    assert_eq!(trace3.belief_digest.contradictions, 1);
    assert!(trace3.belief_digest.facts_contested >= 2);
}

/// v4.0.25 — `Conversation::turn_with_trace` must expose the **post-
/// injection** intent (with `reasoning_chain` and/or `example`
/// populated by v4.0.20+ features). Codex v4.0.23 re-review #2 flagged
/// that the pre-v4.0.25 `adam_chat --trace` mode manually duplicated
/// `turn()` and stopped before the injection calls, so trace output
/// was materially false.
#[test]
fn turn_with_trace_returns_post_injection_intent() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "құрал".into(),
            root: "құрал".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/tools_household.jsonl".into(),
            sample_id: "tool_014".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let (out, trace) = conv.turn_with_trace("кітап туралы бірдеңе айт", &lex, &repo, 0);

    // Baseline: output and trace must agree — no divergence between
    // real pipeline and what trace says happened.
    assert!(
        !out.is_empty(),
        "turn_with_trace must produce a non-empty output"
    );

    // Key assertion: trace.intent_after_injection has `reasoning_chain`
    // populated because inject_reasoning_chain ran. Pre-v4.0.25 trace
    // would have shown `reasoning_chain: None` here.
    if let adam_dialog::Intent::Unknown {
        reasoning_chain, ..
    } = &trace.intent_after_injection
    {
        assert!(
            reasoning_chain.is_some(),
            "trace.intent_after_injection must carry the injected reasoning_chain, got {:?}",
            reasoning_chain
        );
    } else {
        panic!(
            "expected Intent::Unknown in trace, got {:?}",
            trace.intent_after_injection
        );
    }
}

/// v4.0.22 — reranker prefers shorter curated chain when both are
/// fully curated.
#[test]
fn reranker_prefers_shorter_chain() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let long_chain = DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::RelatedTo,
        object: SlotRef {
            surface: "сана".into(),
            root: "сана".into(),
            pos: "noun".into(),
        },
        rule_id: "R5_shared_is_a_target".into(),
        source_chain: vec![
            FactSource {
                pack: "world_core/society.jsonl".into(),
                sample_id: "soc_a".into(),
            },
            FactSource {
                pack: "world_core/kz_literature.jsonl".into(),
                sample_id: "lit_b".into(),
            },
            FactSource {
                pack: "world_core/proverbs.jsonl".into(),
                sample_id: "prov_c".into(),
            },
        ],
        confidence: ConfidenceKind::RuleInferred,
    };
    let short_chain = DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "құрал".into(),
            root: "құрал".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/tools_household.jsonl".into(),
            sample_id: "tool_014".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    };
    let derived = vec![long_chain, short_chain];

    let mut conv =
        Conversation::new().with_reasoning_chains(support_facts_for_derived(&derived), derived);
    let out = conv.turn("кітап туралы бірдеңе айт", &lex, &repo, 0);
    assert!(
        out.contains("құрал"),
        "reranker should pick shorter R1 curated chain, got: {out:?}"
    );
}

/// v4.0.24 — Codex v4.0.23 re-review case #2: when multiple equally-
/// scored R1 IsA derivations share the same subject but have different
/// objects, prefer the object with the SHORTER IsA-chain distance in
/// the base-fact graph. Pre-v4.0.24 the reranker canonical-triple tie-
/// break picked байлық (a metaphor target 3 hops from математика via
/// «білім IsA байлық» in proverbs.jsonl) over білім (1 hop direct
/// «ғылым IsA білім»).
///
/// This test wires up a minimal base-fact graph + two tied R1
/// derivations and asserts that the shorter-path object wins.
#[test]
fn reranker_prefers_shorter_is_a_path_on_tied_curated() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let slot = |root: &str| SlotRef {
        surface: root.into(),
        root: root.into(),
        pos: "noun".into(),
    };
    let src = |pack: &str, id: &str| FactSource {
        pack: pack.into(),
        sample_id: id.into(),
    };

    // Base IsA edges: A→B→C (depth 2), A→B→D (depth 2), A→B→C→E (depth 3).
    let base = |s: &str, o: &str, pack: &str, id: &str| Fact {
        subject: slot(s),
        predicate: Predicate::IsA,
        object: slot(o),
        pattern: "test".into(),
        source: src(pack, id),
        confidence: ConfidenceKind::Grammar,
        raw_text: "test".into(),
    };

    let extracted = vec![
        base("a", "b", "world_core/domA.jsonl", "a1"),
        base("b", "c", "world_core/domA.jsonl", "a2"),
        base("b", "d", "world_core/domA.jsonl", "a3"),
        base("c", "e", "world_core/domA.jsonl", "a4"),
    ];

    let derived_chain = |obj: &str| DerivedFact {
        subject: slot("a"),
        predicate: Predicate::IsA,
        object: slot(obj),
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![
            src("world_core/domA.jsonl", "a1"),
            src("world_core/domA.jsonl", "a2"),
        ],
        confidence: ConfidenceKind::RuleInferred,
    };

    // Three tied candidates: a IsA c (depth 2), a IsA d (depth 2),
    // a IsA e (depth 3). The depth-2 candidates should win over e;
    // among c/d, canonical-triple tie-break picks c (alphabetic).
    let derived = vec![derived_chain("e"), derived_chain("d"), derived_chain("c")];

    let mut conv = Conversation::new().with_reasoning_chains(extracted, derived);
    let out = conv.turn("a туралы айтшы", &lex, &repo, 0);
    // The shortest-path winner must be c or d (both depth 2), NOT e.
    assert!(
        !out.contains('e'),
        "reranker should reject depth-3 candidate `e`, got: {out:?}"
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

/// v1.9.0 option-B: with `InSampleCitySwap` mode and a recognised
/// city in the session, a retrieved quote that mentions a different
/// city should be rewritten to the user's city — preserving case
/// morphology via FST. Biographical quotes (containing a 4-digit year)
/// must NOT be swapped.
#[test]
fn compose_mode_swaps_cities_in_retrieval_samples() {
    use adam_dialog::ComposeMode;
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    // Fresh empty index with only a synthetic sample — avoids
    // dependence on the committed corpus ranker picking our sample
    // over, say, an Abai quote that scores higher.
    let mut index = MorphemeIndex::new();
    let sref = adam_retrieval::SampleRef {
        pack: "abai_wikisource_pack.json".into(), // use a high-purity pack
        sample_id: "test_001".into(),
    };
    index.insert("бала", sref.clone());
    index.remember_text(&sref, "Бала Алматыда жақсы өмір сүреді");
    index.refresh_stats();

    let mut conv = Conversation::new()
        .with_morpheme_index(index)
        .with_compose_mode(ComposeMode::InSampleCitySwap);
    conv.session.insert("city".into(), "Шымкент".into());

    let mut saw_swap = false;
    for seed in 0..32u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if let Some(quoted) = quoted_portion(&out) {
            if quoted.contains("Шымкентте") && !quoted.contains("Алматыда") {
                saw_swap = true;
            }
        }
    }
    assert!(
        saw_swap,
        "InSampleCitySwap should rewrite Алматыда → Шымкентте inside the quote for at least one seed"
    );
}

/// Return the string between the first `«` and the last `»` in `s`,
/// or `None` if the quote markers aren't both present. UTF-8-safe:
/// slices on codepoint boundaries, never mid-`»`.
fn quoted_portion(s: &str) -> Option<&str> {
    let start = s.find('«')? + '«'.len_utf8();
    let tail = &s[start..];
    let end_in_tail = tail.rfind('»')?;
    Some(&tail[..end_in_tail])
}

/// v1.9.5: when a swap actually happens, the planner must route to
/// the `unknown.with_adapted_evidence` family — which explicitly marks
/// the quote as adapted so the user can distinguish it from a verbatim
/// corpus citation. "бейімделген" is the common stem across all the
/// adapted-evidence templates; its presence in the output proves the
/// new routing fired.
#[test]
fn adapted_evidence_templates_announce_the_adaptation() {
    use adam_dialog::ComposeMode;
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let mut index = MorphemeIndex::new();
    let sref = adam_retrieval::SampleRef {
        pack: "abai_wikisource_pack.json".into(),
        sample_id: "test_001".into(),
    };
    index.insert("бала", sref.clone());
    index.remember_text(&sref, "Бала Алматыда жақсы өмір сүреді");
    index.refresh_stats();

    let mut conv = Conversation::new()
        .with_morpheme_index(index)
        .with_compose_mode(ComposeMode::InSampleCitySwap);
    conv.session.insert("city".into(), "Шымкент".into());

    let mut saw_adapted_frame = false;
    for seed in 0..32u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        assert!(!out.contains("{"), "unfilled slot leaked: {out:?}");
        if out.contains("бейімд") {
            saw_adapted_frame = true;
        }
    }
    assert!(
        saw_adapted_frame,
        "adapted-evidence family must fire at least once under a successful swap — \
         every template in that family contains the «бейімд-» stem"
    );
}

/// v1.9.5 negative case: without a swap (either Verbatim mode or the
/// city already matches), the planner must NEVER route to the adapted
/// family. Guards against false-positive "this quote was adapted"
/// claims — a trust-critical invariant.
#[test]
fn verbatim_mode_never_claims_adaptation() {
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let mut index = MorphemeIndex::new();
    let sref = adam_retrieval::SampleRef {
        pack: "abai_wikisource_pack.json".into(),
        sample_id: "test_001".into(),
    };
    index.insert("бала", sref.clone());
    index.remember_text(&sref, "Бала Алматыда жақсы өмір сүреді");
    index.refresh_stats();

    let mut conv = Conversation::new().with_morpheme_index(index);
    // Default ComposeMode::Verbatim. Even with a different session
    // city, the adapted-evidence family must never fire.
    conv.session.insert("city".into(), "Шымкент".into());

    for seed in 0..32u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        assert!(
            !out.contains("бейімд"),
            "Verbatim mode must NEVER produce an adapted-evidence template, got: {out:?}"
        );
    }
}

/// v1.9.0 regression: default ComposeMode::Verbatim must NOT rewrite
/// the retrieved sample. The quote stays byte-identical to the corpus.
#[test]
fn compose_mode_verbatim_preserves_retrieved_quote() {
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let mut index = MorphemeIndex::new();
    let sref = adam_retrieval::SampleRef {
        pack: "abai_wikisource_pack.json".into(),
        sample_id: "test_001".into(),
    };
    index.insert("бала", sref.clone());
    index.remember_text(&sref, "Бала Алматыда жақсы өмір сүреді");
    index.refresh_stats();

    // Default compose_mode is Verbatim — no with_compose_mode call.
    let mut conv = Conversation::new().with_morpheme_index(index);
    conv.session.insert("city".into(), "Шымкент".into());

    let mut saw_verbatim_quote = false;
    for seed in 0..32u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        // v1.8.5 frame templates can put "Шымкентте" in the FRAME
        // (outside «…»), but the QUOTED portion must stay verbatim.
        if let Some(quoted) = quoted_portion(&out) {
            assert!(
                !quoted.contains("Шымкентте"),
                "Verbatim mode must NOT swap cities inside the quote: {quoted:?}"
            );
            if quoted.contains("Алматыда") {
                saw_verbatim_quote = true;
            }
        }
    }
    assert!(
        saw_verbatim_quote,
        "at least one seed should cite the synthetic sample verbatim (with Алматыда preserved)"
    );
}

/// v1.9.0 safety: InSampleCitySwap must refuse to rewrite a quote
/// that contains a 4-digit year (biographical / historical guard).
/// "Абай 1845 жылы Қарқаралыда туған" + user.city=Алматы must stay
/// put — swapping would produce a false biographical fact.
#[test]
fn compose_mode_respects_biographical_year_guard() {
    use adam_dialog::ComposeMode;
    use adam_retrieval::MorphemeIndex;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let mut index = MorphemeIndex::new();
    let sref = adam_retrieval::SampleRef {
        pack: "abai_wikisource_pack.json".into(),
        sample_id: "abai_bio".into(),
    };
    index.insert("бала", sref.clone());
    index.remember_text(&sref, "Абай 1845 жылы Қарқаралыда туған бала еді");
    index.refresh_stats();

    let mut conv = Conversation::new()
        .with_morpheme_index(index)
        .with_compose_mode(ComposeMode::InSampleCitySwap);
    conv.session.insert("city".into(), "Алматы".into());

    for seed in 0..8u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        if let Some(quoted) = quoted_portion(&out) {
            if quoted.contains("Қарқаралыда") {
                // In-sample swap must respect the biographical-year
                // guard — Алматы must NOT leak into this quote.
                assert!(
                    !quoted.contains("Алматыда"),
                    "biographical-year guard failed; quote rewritten: {quoted:?}"
                );
            }
        }
    }
}

/// v1.8.5 regression: "мен Алматыдамын" must resolve to
/// `StatementOfLocation { city: Алматы }`, not `StatementOfOccupation`.
/// Before the fix, the occupation recogniser accepted any noun+P1Sg
/// without checking case, so «Алматыдамын» (Алматы + locative + P1Sg)
/// was miscategorised and `session.occupation` ended up as "алматы".
#[test]
fn locative_with_copula_is_location_not_occupation() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен Алматыдамын", &lex, &repo, 0);
    assert_eq!(
        conv.session.get("city").map(|s| s.as_str()),
        Some("Алматы"),
        "locative+P1Sg should populate city, got session={:?}",
        conv.session
    );
    assert!(
        conv.session.get("occupation").is_none(),
        "locative+P1Sg should NOT populate occupation, got session={:?}",
        conv.session
    );
}

/// v1.8.5: FST-aware `{city|locative}` in a session-aware template
/// should render the city with correct vowel-harmonic locative suffix
/// (Алматы → Алматыда, Астана → Астанада, Өскемен → Өскеменде). The
/// test fixes session to Алматы and asserts the rendered form appears
/// when the planner picks an FST-slot template.
#[test]
fn session_aware_city_template_uses_fst_locative() {
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
    conv.session.insert("city".into(), "Алматы".into());

    let mut saw_fst_inflection = false;
    for seed in 0..32u64 {
        let out = conv.turn("бала туралы бірдеңе айт", &lex, &repo, seed);
        assert!(
            !out.contains("{"),
            "unfilled slot leaked at seed={seed}: {out:?}"
        );
        // FST-rendered locative of Алматы is "Алматыда". If we see that
        // exact form (without a dangling hyphen from a literal `{city}-да`),
        // the FST-slot template fired.
        if out.contains("Алматыда") && !out.contains("Алматы-да") {
            saw_fst_inflection = true;
        }
    }
    assert!(
        saw_fst_inflection,
        "expected at least one seed to render {{city|locative}} via the FST"
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

/// v4.0.1 regression: «Неліктен?» («why?» — interrogative) must NOT be
/// absorbed as a city by `StatementOfLocation`. Codex v4.0.0 review
/// discovered that the v3.9.5 `NOT_A_TOPIC` sync only covered
/// `first_noun_root` / `content_roots` paths. The FST analyses
/// "неліктен" as `нелік + Ablative` — a valid noun stem + case combo —
/// so `detect_statement_of_location`'s ablative scan picked up `нелік`
/// as a city root and routed the turn through
/// `StatementOfLocation { city: Some("Нелік") }`. REPL observable effect:
/// reply "Нелікте тұрасыз ба" ("Do you live in Нелік?") to "Неліктен?".
///
/// Fix: `NOT_A_TOPIC` gains `нелік` (the FST-stripped stem), and
/// `detect_statement_of_location` now skips any noun whose root is in
/// `NOT_A_TOPIC` before accepting it as a city. This test verifies the
/// **end-to-end** Conversation path that Codex's unit-level
/// `not_a_topic_covers_v3_9_5_additions` missed.
#[test]
fn nelikten_is_not_absorbed_as_city() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    // Pre-condition: no city in session.
    assert!(!conv.session.contains_key("city"));

    // Turn 1: give a legitimate city first, to confirm the detector
    // still works for real city inputs.
    let _ = conv.turn("мен Қостанайдамын", &lex, &repo, 0);
    assert_eq!(conv.session.get("city"), Some(&"Қостанай".to_string()));

    // Turn 2: «Неліктен?» — this MUST NOT replace the city.
    // Pre-v4.0.1: session.city would be overwritten with "Нелік".
    // Post-v4.0.1: intent is Unknown, session.city is untouched.
    let _ = conv.turn("Неліктен?", &lex, &repo, 0);
    assert_eq!(
        conv.session.get("city"),
        Some(&"Қостанай".to_string()),
        "«Неліктен?» must not be absorbed as a city by StatementOfLocation"
    );

    // Turn 3: fresh conversation, «Неліктен?» as the ONLY input.
    // city must stay absent.
    let mut fresh = Conversation::new();
    let _ = fresh.turn("Неліктен?", &lex, &repo, 0);
    assert!(
        !fresh.session.contains_key("city"),
        "bare «Неліктен?» must not set session.city on any seed"
    );
}

#[test]
fn qaldarynyz_qalai_does_not_poison_city_memory() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("Қалдарыңыз қалай?", &lex, &repo, 0);
    assert!(
        !conv.session.contains_key("city"),
        "wellbeing-style question must not be absorbed as session.city"
    );

    let _ = conv.turn("Мен Қашар ауылынанмын.", &lex, &repo, 0);
    assert_eq!(conv.session.get("city"), Some(&"Қашар".to_string()));
}

/// v4.0.3: `Conversation::with_curated_only_reasoning(true)` — the
/// investor-safe REPL mode behind `adam_chat --safe` — must refuse
/// every derivation whose `source_chain` pulls from a text-extracted
/// pack, while default mode still cites them. Covers the promise made
/// in v4.0.2 (demo-only filter) to the chat path.
#[test]
fn safe_mode_rejects_text_source_chain_derivations() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    // A derivation whose source_chain is text-extracted (not world_core).
    // This is the class of chain Codex flagged — wrongly confident
    // "абай is_a халық"-style. In safe mode, MUST be ignored.
    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "кітап".into(),
            root: "кітап".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::RelatedTo,
        object: SlotRef {
            surface: "ілім".into(),
            root: "ілім".into(),
            pos: "noun".into(),
        },
        rule_id: "R5_shared_is_a_target".into(),
        source_chain: vec![
            FactSource {
                pack: "kazakh_proverbs_pack.json".into(),
                sample_id: "p_003".into(),
            },
            FactSource {
                pack: "wikipedia_kz_pack.json".into(),
                sample_id: "wiki_kz_0088327".into(),
            },
        ],
        confidence: ConfidenceKind::RuleInferred,
    }];

    // Default mode — chain fires (v4.0.2 baseline).
    let mut default_conv = Conversation::new()
        .with_reasoning_chains(support_facts_for_derived(&derived), derived.clone());
    let out_default = default_conv.turn("кітап туралы бірдеңе айт", &lex, &repo, 0);
    assert!(
        out_default.contains("байланыс"),
        "default mode must still cite text-source derivations (got: {out_default:?})"
    );

    // Safe mode — chain refused, falls through to Unknown noun-echo.
    let mut safe_conv = Conversation::new()
        .with_reasoning_chains(support_facts_for_derived(&derived), derived.clone())
        .with_curated_only_reasoning(true);
    let out_safe = safe_conv.turn("кітап туралы бірдеңе айт", &lex, &repo, 0);
    assert!(
        !out_safe.contains("байланыс"),
        "safe mode must refuse text-source derivations (got: {out_safe:?})"
    );
}

/// Symmetric test: safe mode must CONTINUE firing on fully-curated
/// chains. Closes the regression window where the filter might
/// overreach and block legitimate world_core derivations.
#[test]
fn safe_mode_still_cites_fully_curated_derivations() {
    use adam_reasoning::reasoner::DerivedFact;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    // Fully-curated derivation: both source packs under world_core/.
    let derived = vec![DerivedFact {
        subject: SlotRef {
            surface: "бөлу".into(),
            root: "бөлу".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::RelatedTo,
        object: SlotRef {
            surface: "көбейту".into(),
            root: "көбейту".into(),
            pos: "noun".into(),
        },
        rule_id: "R5_shared_is_a_target".into(),
        source_chain: vec![
            FactSource {
                pack: "world_core/numbers.jsonl".into(),
                sample_id: "num_026".into(),
            },
            FactSource {
                pack: "world_core/numbers.jsonl".into(),
                sample_id: "num_027".into(),
            },
        ],
        confidence: ConfidenceKind::RuleInferred,
    }];

    let mut safe_conv = Conversation::new()
        .with_reasoning_chains(support_facts_for_derived(&derived), derived)
        .with_curated_only_reasoning(true);
    let out = safe_conv.turn("бөлу туралы бірдеңе айт", &lex, &repo, 0);
    assert!(
        out.contains("байланыс"),
        "safe mode must still cite world_core-only derivations (got: {out:?})"
    );
    assert!(
        out.contains("бөлу") && out.contains("көбейту"),
        "rendered chain must name both roots (got: {out:?})"
    );
}

/// **v4.3.1** — Person canonical entity in dialog memory.
///
/// `Conversation` routes `Intent::StatementOfName` through
/// `language_core::canonical_person_entity`. After three statements
/// of the same name in three different surface forms (`Дәулет`,
/// `дәулет`, `дӘУЛEТ`):
/// - `session["name"]` should be the canonical form `Дәулет`,
/// - `session["name_id"]` should be the stable id `person:Дәулет`,
/// - `belief.entities[USER_SELF_KEY].canonical_id` should be
///   `person:Дәулет` from the first turn onwards (immutable across
///   surface variants),
/// - the active belief fact `(USER, name, …)` should carry the
///   canonical form, not the raw surface, on every restatement.
#[test]
fn conversation_collapses_person_name_surface_variants() {
    use adam_dialog::USER_SELF_KEY;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // First statement: canonical Cyrillic.
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);
    assert_eq!(conv.session.get("name"), Some(&"Дәулет".to_string()));
    assert_eq!(
        conv.session.get("name_id"),
        Some(&"person:Дәулет".to_string()),
        "session must carry the stable canonical id"
    );
    let user = conv
        .belief
        .entities
        .get(USER_SELF_KEY)
        .expect("user entity recorded on first StatementOfName");
    assert_eq!(
        user.canonical_id.as_deref(),
        Some("person:Дәулет"),
        "EntityMemory.canonical_id must capture the resolved person id"
    );
    assert_eq!(
        conv.belief
            .active_fact(USER_SELF_KEY, "name")
            .map(|f| f.object.as_str()),
        Some("Дәулет"),
        "active belief fact must hold the canonical form"
    );

    // Second statement: lowercase surface — must not create a
    // separate entity, must not clear the canonical id, must update
    // session string slot to the canonical form (not the raw input).
    let _ = conv.turn("менің атым дәулет", &lex, &repo, 1);
    assert_eq!(
        conv.session.get("name"),
        Some(&"Дәулет".to_string()),
        "lowercase restatement still produces canonical session value"
    );
    let user = conv.belief.entities.get(USER_SELF_KEY).expect("user");
    assert_eq!(
        user.canonical_id.as_deref(),
        Some("person:Дәулет"),
        "canonical_id must persist through restatement"
    );

    // Third statement: mixed-script — homoglyph cleanup collapses
    // it to the same Cyrillic canonical, so still no new entity,
    // still no contradiction (idempotent on canonical value).
    let _ = conv.turn("менің атым дӘУЛEТ", &lex, &repo, 2);
    assert_eq!(conv.session.get("name"), Some(&"Дәулет".to_string()));
    assert_eq!(
        conv.belief.contradictions.len(),
        0,
        "surface-variant restatements of the same canonical name \
         must not register a contradiction"
    );
}

/// **v4.3.2 — regression**: «жасанды интеллект» dialog block.
///
/// Real test dialog (2026-04-26) revealed that stating a profession
/// containing «жасанды интеллект» locked the dialog into a permanent
/// `CheckContradiction` for every subsequent topic question. Root
/// cause was in `semantics::token_mentions_generic_place`: it used
/// `token.contains(stem)`, and the 2-char stem `ел` (country) is a
/// substring of `интеллект` (`-ЕЛ-` at position 3-4). The
/// false-positive made `recover_named_place_before_generic_location`
/// promote the *previous* token `жасанды` to a city, the belief
/// layer logged `(USER, city, Жасанды)` against the genuine
/// `(USER, city, Атырау)`, and the planner thereafter routed every
/// turn into `CheckContradiction`.
///
/// **v4.3.2 fix**: switch substring match → prefix match. This
/// regression locks the bug closed: an occupation statement that
/// happens to contain `интеллект` MUST classify as
/// `StatementOfOccupation`, not `StatementOfLocation`, and the
/// belief layer MUST end with one Active city fact (Атырау), no
/// contradiction, and a regular Active occupation fact.
///
/// Mirrors the user-reported dialog turn-for-turn.
#[test]
fn jasandi_intellekt_does_not_break_dialog_with_false_city() {
    use adam_dialog::USER_SELF_KEY;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // Establish a real city.
    let _ = conv.turn("Мен Атырауданмын", &lex, &repo, 0);
    assert_eq!(conv.session.get("city"), Some(&"Атырау".to_string()));
    assert_eq!(conv.belief.contradictions.len(), 0);

    // Profession statement that previously broke everything.
    let _ = conv.turn(
        "Мен жаңа жасанды интеллект моделін әзірлейтін бағдарламашымын",
        &lex,
        &repo,
        1,
    );

    // Belief must record the OCCUPATION, not a bogus city.
    assert_eq!(
        conv.belief
            .active_fact(USER_SELF_KEY, "occupation")
            .map(|f| f.object.clone()),
        Some("бағдарламашы".to_string()),
        "occupation must be recorded from the бағдарламашы copula form"
    );
    assert_eq!(
        conv.belief
            .active_fact(USER_SELF_KEY, "city")
            .map(|f| f.object.clone()),
        Some("Атырау".to_string()),
        "city must remain Атырау — `интеллект` containing the 2-char `ел` substring \
         must NOT recover `жасанды` as a city"
    );
    assert_eq!(
        conv.belief.contradictions.len(),
        0,
        "no contradiction may be logged on a clean profession statement"
    );

    // A topic question on the next turn must NOT be hijacked by a
    // CheckContradiction. With the v4.3.2 fix, contradictions stays
    // empty, so the planner routes Unknown+noun_hint normally.
    let out = conv.turn("Қазақстан туралы не білесіз", &lex, &repo, 2);
    assert!(
        !out.to_lowercase().contains("жасанды"),
        "topic reply must not surface the bogus city in a contradiction prompt (got: {out:?})"
    );
}

/// **v4.3.3** — `Intent::AskAboutSystem` (self/other distinction,
/// Track B of `docs/intelligence_roadmap.md`).
///
/// The user-shared 2026-04-26 dialog test had:
///
/// ```text
/// > А, сен кімсің және атың кім?
/// сіздің атыңыз Мәулет
/// ```
///
/// Pre-v4.3.3 the question routed through `Intent::AskName` (the
/// `атың кім` substring matched `detect_ask_name`) and the v4.2.5
/// slot-aware override emitted the user's stored name. Wrong:
/// `сен кімсің` is unambiguously about adam.
///
/// v4.3.3 introduces `Intent::AskAboutSystem` and orders its
/// detector before `detect_ask_name`, so any pronoun-led identity
/// question routes to the system-introduction template family.
/// This regression locks the behaviour: even after the user has
/// stated their own name, asking `сен кімсің` returns adam's
/// self-introduction (containing «адам» and a model-kind word),
/// NOT the user's name.
#[test]
fn ask_about_system_returns_adam_identity_not_user_name() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // User introduces themselves.
    let _ = conv.turn("менің атым Мәулет", &lex, &repo, 0);
    assert_eq!(conv.session.get("name"), Some(&"Мәулет".to_string()));

    // Asking adam about adam — must surface adam's identity, not
    // the stored user name.
    let out = conv.turn("сен кімсің", &lex, &repo, 1);
    let lower = out.to_lowercase();
    assert!(
        lower.contains("адам"),
        "AskAboutSystem reply must mention adam's name (got: {out:?})"
    );
    assert!(
        !lower.contains("мәулет"),
        "AskAboutSystem reply must NOT echo back the user's stored name (got: {out:?})"
    );
}

/// **v4.3.3** — pronoun-led identity question with the formal pronoun.
/// `сіз кімсіз` resolves the same way as `сен кімсің`.
#[test]
fn ask_about_system_handles_formal_pronoun() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let out = conv.turn("сіз кімсіз", &lex, &repo, 0);
    let lower = out.to_lowercase();
    assert!(
        lower.contains("адам"),
        "formal-pronoun AskAboutSystem reply must mention `адам` (got: {out:?})"
    );
}

/// **v4.3.3** — guard against false-positive routing. `менің атым
/// Мәулет` (statement of name) must still classify as
/// `StatementOfName`, not `AskAboutSystem` (no pronoun + no
/// identity question). The detector's pronoun gate keeps the two
/// cleanly separated.
#[test]
fn ask_about_system_does_not_swallow_statement_of_name() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің атым Мәулет", &lex, &repo, 0);
    assert_eq!(
        conv.session.get("name"),
        Some(&"Мәулет".to_string()),
        "StatementOfName must still record the name; AskAboutSystem detector must not preempt it"
    );
}

/// **v4.3.4** — `AskAboutSystem { aspect: General }` mentions the
/// canonical name and the technical full name. Verifies the
/// `system_name` and `system_full_name` slots are wired through
/// the `ask_about_system` template family.
#[test]
fn ask_about_system_general_includes_name_and_full_name() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let out = conv.turn("сен кімсің", &lex, &repo, 0);
    assert!(
        out.to_lowercase().contains("адам"),
        "general AskAboutSystem reply must mention `адам` (got: {out:?})"
    );
    assert!(
        out.contains("Nano Language Model") || out.contains("NLM"),
        "general AskAboutSystem reply must mention either the full name or the abbreviation \
         (got: {out:?})"
    );
}

/// **v4.3.4** — `AskAboutSystem { aspect: Creator }` surfaces the
/// creator field from `SystemIdentity`. Verifies the creator name
/// reaches the user-visible reply.
#[test]
fn ask_about_system_creator_aspect_mentions_creator() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let out = conv.turn("сені кім жасады", &lex, &repo, 0);
    assert!(
        out.contains("Баймурзин"),
        "Creator aspect reply must mention the creator surname (got: {out:?})"
    );
    assert!(
        out.contains("Даулет"),
        "Creator aspect reply must mention the creator's first name (got: {out:?})"
    );
}

/// **v4.3.4** — `AskAboutSystem { aspect: Birthdate }` surfaces the
/// birthdate (repository creation date 2026-04-07).
#[test]
fn ask_about_system_birthdate_aspect_mentions_date() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let out = conv.turn("қашан пайда болдың", &lex, &repo, 0);
    assert!(
        out.contains("2026-04-07"),
        "Birthdate aspect reply must mention the canonical birthdate (got: {out:?})"
    );
}

/// **v4.3.4** — `AskAboutSystem { aspect: Architecture }` surfaces
/// the architecture summary that distinguishes adam from
/// mainstream LLMs.
#[test]
fn ask_about_system_architecture_aspect_mentions_difference() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let out = conv.turn("сенің ерекшелігің не", &lex, &repo, 0);
    assert!(
        out.contains("ереже") || out.contains("ережелер"),
        "Architecture aspect reply must mention rule-based nature (got: {out:?})"
    );
    assert!(
        out.contains("архитектур"),
        "Architecture aspect reply must mention the word `архитектура` (got: {out:?})"
    );
}

/// **v4.4.0** — `Action::DismissContradiction` end-to-end.
///
/// User states two cities, dialog logs a contradiction (per
/// v4.0.27), system asks `қалаңыз X пе, әлде Y ма`, user replies
/// `екеуі де жоқ` (neither). Post-v4.4.0 this should drop both
/// city facts to `Superseded` and acknowledge with a
/// `dismiss_contradiction`-family reply. Pre-v4.4.0 the user had
/// no clean exit — every subsequent turn would reroute back to
/// `CheckContradiction` until the user picked one.
#[test]
fn dismiss_contradiction_clears_both_cities_on_neither_reply() {
    use adam_dialog::USER_SELF_KEY;

    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("мен Алматыда тұрамын", &lex, &repo, 0);
    let _ = conv.turn("мен Астанада тұрамын", &lex, &repo, 1);
    assert_eq!(
        conv.belief.contradictions.len(),
        1,
        "two distinct city statements must register a conflict"
    );

    let out = conv.turn("екеуі де жоқ", &lex, &repo, 2);
    assert_eq!(
        conv.belief.contradictions.len(),
        0,
        "dismissal must clear the BeliefConflict"
    );
    assert!(
        conv.belief.active_fact(USER_SELF_KEY, "city").is_none(),
        "both city facts must end Superseded after dismissal"
    );
    let lower = out.to_lowercase();
    assert!(
        lower.contains("ұқтым")
            || lower.contains("түсіндім")
            || lower.contains("ұмыт")
            || lower.contains("тарт")
            || lower.contains("есеп"),
        "dismissal reply must signal acknowledgement, got: {out:?}"
    );
}

/// **v4.4.0** — alternate dismissal phrasing (`білмеймін`,
/// "I don't know") routes through the same path.
#[test]
fn dismiss_contradiction_handles_dont_know_phrasing() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен Алматыда тұрамын", &lex, &repo, 0);
    let _ = conv.turn("мен Астанада тұрамын", &lex, &repo, 1);
    assert_eq!(conv.belief.contradictions.len(), 1);

    let _ = conv.turn("білмеймін", &lex, &repo, 2);
    assert_eq!(
        conv.belief.contradictions.len(),
        0,
        "білмеймін must dismiss the pending contradiction"
    );
}

/// **v4.4.0** — contradiction-priority cap. Pre-v4.4.0 a pending
/// contradiction blocked every subsequent turn forever. With the
/// cap (K = 3 turns since `detected_at_turn`), the user can move
/// on; the conflict stays in audit but stops dominating.
///
/// Setup: log conflict on turn 1. Turns 2–3 still route through
/// CheckContradiction (within cap). Turns 4+ fall through —
/// e.g. a `сәлем` greeting on turn 4 routes to `Action::Social`
/// instead of `CheckContradiction`.
#[test]
fn contradiction_priority_cap_lets_user_move_on() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    // `Conversation::turn` takes `rng_seed` as the 4th arg; the
    // *real* turn id is the internal `turn_counter`, which starts
    // at 0 and increments on every call. So the 1st call uses
    // turn_id 0, the 2nd uses 1, etc. — the seed values below are
    // unrelated to the cap arithmetic.
    let _ = conv.turn("мен Алматыда тұрамын", &lex, &repo, 0);
    let _ = conv.turn("мен Астанада тұрамын", &lex, &repo, 1);
    // Conflict detected on turn 1 (the second statement).
    assert_eq!(conv.belief.contradictions.len(), 1);
    assert_eq!(conv.belief.contradictions[0].detected_at_turn, 1);

    // Turn id 2 (3rd call): within cap (2 - 1 == 1 < 3),
    // a generic greeting still routes to CheckContradiction
    // because contradiction priority dominates.
    let (_, t2) = conv.turn_with_trace("сәлем", &lex, &repo, 0);
    assert_eq!(
        format!("{:?}", t2.action_digest.action),
        "CheckContradiction",
        "within the cap, contradiction priority must dominate even a greeting"
    );

    // Turn id 3 (4th call): still within cap (3 - 1 == 2 < 3).
    let (_, t3) = conv.turn_with_trace("сәлем", &lex, &repo, 0);
    assert_eq!(
        format!("{:?}", t3.action_digest.action),
        "CheckContradiction",
        "turn 3 is the last one inside the cap"
    );

    // Turn id 4 (5th call): cap exceeded (4 - 1 == 3, condition is
    // `<`, so the contradiction no longer dominates). A greeting
    // must route to Action::Social, NOT CheckContradiction.
    let (out, t4) = conv.turn_with_trace("сәлем", &lex, &repo, 0);
    assert_eq!(
        format!("{:?}", t4.action_digest.action),
        "Social",
        "after the priority cap, normal action paths must work — got action = {:?}, output = {out:?}",
        t4.action_digest.action
    );
    // Conflict still in belief for audit — only priority changed.
    assert_eq!(conv.belief.contradictions.len(), 1);
}

/// **v4.4.5** — `Action::CheckContradiction` must render the
/// clarifying question, not a confirmation. Codex flagged this
/// from a 2026-04-27 live REPL trace: action layer correctly
/// chose CheckContradiction after Астана/Алматы, but the planner
/// fell through to `intent_key(StatementOfLocation) =
/// "statement_of_location"` and emitted
/// «Алматыда екеніңізді есте сақтаймын» — committing to one of
/// the contested values. Fix routes the renderer through the new
/// `check_contradiction` template family via the
/// `__check_contradiction__` marker slot.
#[test]
fn check_contradiction_action_renders_clarifying_question() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("мен Астанада тұрамын", &lex, &repo, 0);
    let (out, trace) = conv.turn_with_trace("мен Алматыда тұрамын", &lex, &repo, 1);
    assert_eq!(
        format!("{:?}", trace.action_digest.action),
        "CheckContradiction",
        "second contradicting statement must trigger CheckContradiction action"
    );
    let lower = out.to_lowercase();
    // The reply must mention BOTH contested values and ask the
    // user to choose. The question marker `ма` (interrogative
    // particle) is the linguistic anchor — every variant of the
    // `check_contradiction` family carries it.
    assert!(
        lower.contains("астана") && lower.contains("алматы"),
        "clarifying reply must surface both candidates — got {out:?}"
    );
    assert!(
        lower.contains(" ма") || lower.contains("қайсысы"),
        "clarifying reply must be a question, not a confirmation — got {out:?}"
    );
    // The pre-v4.4.5 confirmation phrasings must NOT leak
    // through. These are the exact verbalizers the
    // `statement_of_location` family used to emit before the
    // override landed.
    assert!(
        !lower.contains("есте сақтаймын"),
        "must not emit confirmation phrasing under conflict — got {out:?}"
    );
}

/// **v4.4.5** — `менің жасым қанша?` after `менің жасым 40` must
/// route to `Intent::AskAge` and answer from session storage,
/// NOT to `StatementOfAge { years: None }`. Pre-v4.4.5
/// `detect_statement_of_age` matched on the `жасым` substring
/// before `detect_ask_age` ran; ask-age only checked the 2nd-
/// person `жасың`/`жасыңыз` forms, so the 1sg-self-recall form
/// was misclassified. The reply happened to read correctly only
/// because the `statement_of_age` family interpolates
/// `session.age`, but the underlying intent was wrong.
#[test]
fn ask_age_self_recall_returns_stored_value() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің жасым 40", &lex, &repo, 0);
    let (out, trace) = conv.turn_with_trace("менің жасым қанша?", &lex, &repo, 1);
    assert!(
        matches!(trace.intent_after_verification, adam_dialog::Intent::AskAge),
        "1sg-self-recall must classify as AskAge, got {:?}",
        trace.intent_after_verification
    );
    assert!(
        out.contains("40"),
        "self-recall must surface the stored value — got {out:?}"
    );
}

/// **v4.4.9** — `менің атым кім?` after `менің атым Дәулет` must
/// route to `Intent::AskName` and answer from session storage. The
/// REPL replay battery surfaced this on first run in v4.4.6 and
/// v4.4.8 deferred it; the bug was symmetric to the v4.4.5 AskAge
/// fix and v4.4.6 AskOccupation fix but **worse**: pre-v4.4.9 the
/// 1sg-possessive `атым` matched
/// `detect_statement_of_name`'s "атым X" pattern and grabbed the
/// literal `Кім` as the user's name, then logged a phantom
/// `BeliefConflict` (Дәулет vs Кім) followed by a clarifying
/// question that asked the user to pick between their actual name
/// and the question word. Two complementary fixes:
/// 1. `detect_statement_of_name` refuses interrogative pronouns
///    (`кім` / `не` / `қандай` / `қайсысы`) as the candidate
///    name across all three patterns.
/// 2. `detect_ask_name` extended to match the 1sg form
///    (`атым / есімім + кім / не`) so the question reaches the
///    `ask_name.with_known_user` template family.
#[test]
fn ask_name_self_recall_returns_stored_value_no_phantom_conflict() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("менің атым Дәулет", &lex, &repo, 0);
    let (out, trace) = conv.turn_with_trace("менің атым кім?", &lex, &repo, 1);
    assert!(
        matches!(
            trace.intent_after_verification,
            adam_dialog::Intent::AskName
        ),
        "1sg-self-recall must classify as AskName, got {:?}",
        trace.intent_after_verification
    );
    let lower = out.to_lowercase();
    assert!(
        lower.contains("дәулет"),
        "self-recall must surface the stored name — got {out:?}"
    );
    assert!(
        !lower.contains("кім"),
        "reply must not echo the question word as if it were a name — got {out:?}"
    );
    // No phantom contradiction was logged: belief still has one
    // active name fact, no conflicts.
    assert_eq!(
        conv.belief.contradictions.len(),
        0,
        "self-recall question must not trigger a phantom BeliefConflict"
    );
}

/// **v4.4.9** — bare interrogative pronoun guard. Even without a
/// session-stored name, `менің атым кім?` must NOT pre-commit
/// `Кім` as a name; it should refuse / answer-tentatively rather
/// than misclassify. Belongs in the same regression family as the
/// stored-name test above.
#[test]
fn ask_name_self_recall_with_empty_session_does_not_capture_kim() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let (_out, trace) = conv.turn_with_trace("менің атым кім?", &lex, &repo, 0);
    // The intent must NOT be StatementOfName{Кім}. AskName is
    // the correct classification (the templates will land on
    // `ask_name`, the bare-self-introduction family, since no
    // name is stored yet).
    assert!(
        !matches!(
            trace.intent_after_verification,
            adam_dialog::Intent::StatementOfName { .. }
        ),
        "interrogative pronoun must never be captured as a name; got {:?}",
        trace.intent_after_verification
    );
    // Session must remain empty — no `name` slot was filled.
    assert!(
        conv.session.get("name").is_none(),
        "session must not capture `Кім` as a name"
    );
}

/// **v4.4.10** — `Танысайық` («let's get acquainted») must route
/// to `Intent::Greeting { kind: IntroProposal }` and produce a
/// reply that surfaces adam's own name AND asks for the user's.
/// Pre-v4.4.10 the surface form fell through every greeting
/// branch (no `қайырлы`, no `сәлеметсіз`, first token isn't
/// `сәлем`) and landed on the generic refusal `қайта айтыңызшы`
/// — surfaced by a 2026-04-28 real-REPL transcript.
#[test]
fn intro_proposal_routes_to_greeting_intro_proposal_family() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let (out, trace) = conv.turn_with_trace("Танысайық.", &lex, &repo, 0);
    assert!(
        matches!(
            trace.intent_after_verification,
            adam_dialog::Intent::Greeting {
                kind: adam_dialog::GreetingKind::IntroProposal
            }
        ),
        "Танысайық must classify as Greeting{{IntroProposal}}, got {:?}",
        trace.intent_after_verification
    );
    let lower = out.to_lowercase();
    assert!(
        lower.contains("адам"),
        "intro-proposal reply must surface adam's name — got {out:?}"
    );
    assert!(
        !lower.contains("қайта айтыңызшы"),
        "intro-proposal must not fall through to the safe-fallback refusal — got {out:?}"
    );
}

/// **v4.4.10** — `Танысалық` and `танысып алайық` are alternative
/// imperative forms of the same exhortative; both must reach the
/// IntroProposal branch.
#[test]
fn intro_proposal_variants_route_to_intro_proposal_family() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    for variant in ["Танысалық", "Танысып алайық", "Танысып алыңыз"]
    {
        let mut conv = Conversation::new();
        let (_out, trace) = conv.turn_with_trace(variant, &lex, &repo, 0);
        assert!(
            matches!(
                trace.intent_after_verification,
                adam_dialog::Intent::Greeting {
                    kind: adam_dialog::GreetingKind::IntroProposal
                }
            ),
            "variant {variant:?} must classify as Greeting{{IntroProposal}}, got {:?}",
            trace.intent_after_verification
        );
    }
}

/// **v4.4.10** — `Қысқасы` («briefly / in short») is a discourse
/// adverbial particle, not a topic noun. Pre-v4.4.10 the FST
/// returned root `қысқа` (= "short") and the topic extractor
/// surfaced it, leading to a tangential proverb keyed on `қысқа`
/// in a 2026-04-28 real-REPL transcript. Post-v4.4.10 NOT_A_TOPIC
/// includes `қысқа`, so `Қысқасы` no longer mispoarses as a topic.
#[test]
fn qysqasy_does_not_get_extracted_as_topic() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let (_out, trace) = conv.turn_with_trace("Қысқасы, сен ештеңе білмейсің.", &lex, &repo, 0);
    // The intent will land somewhere — most likely Unknown or a
    // refusal path. The invariant we lock here is that
    // `noun_hint` is NOT `қысқа` and not `ештеңе`.
    if let adam_dialog::Intent::Unknown { noun_hint, .. } = &trace.intent_after_verification {
        let hint_lower = noun_hint.as_deref().map(str::to_lowercase);
        let hint_str = hint_lower.as_deref();
        assert_ne!(
            hint_str,
            Some("қысқа"),
            "discourse particle Қысқасы must not be extracted as topic noun"
        );
        assert_ne!(
            hint_str,
            Some("ештеңе"),
            "indefinite-quantifier ештеңе must not be extracted as topic noun"
        );
    }
}

/// **v4.4.10** — the v4.4.10 geography expansion authored 17
/// oblast entries + a `has_quantity` fact about the count. The
/// dialog-side surface assertion lives in
/// `data/eval/repl_dialogs.json`'s `kazakhstan_oblast_count_v4_4_10`
/// dialog, which exercises the full retrieval + reasoning path.
/// This data-layer test verifies the world_core file carries every
/// 17 oblast entry — the retrieval / reasoning chains all derive
/// from that ground truth, so a missing entry here breaks every
/// downstream surface.
#[test]
fn kazakhstan_world_core_carries_all_17_oblasts() {
    let path = "../../data/world_core/geography_kz.jsonl";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let raw = std::fs::read_to_string(path).expect("read geography_kz");
    let oblasts = [
        "абай облысы",
        "ақмола облысы",
        "ақтөбе облысы",
        "алматы облысы",
        "атырау облысы",
        "батыс қазақстан облысы",
        "жамбыл облысы",
        "жетісу облысы",
        "қарағанды облысы",
        "қостанай облысы",
        "қызылорда облысы",
        "маңғыстау облысы",
        "павлодар облысы",
        "солтүстік қазақстан облысы",
        "түркістан облысы",
        "ұлытау облысы",
        "шығыс қазақстан облысы",
    ];
    for oblast in oblasts {
        assert!(
            raw.to_lowercase().contains(oblast),
            "geography_kz.jsonl must contain world_core entry for {oblast:?}"
        );
    }
    // The has_quantity fact for the country-wide count must also
    // be present so retrieval has a fact to surface for
    // «Қазақстанда қанша облыс бар?».
    assert!(
        raw.contains("Қазақстанда 17 облыс бар"),
        "world_core must carry the country-wide oblast count fact"
    );
}

/// **v4.4.12** — locative-attributive suffix recovery. The
/// Kazakh `-дағы / -дегі / -тағы / -тегі` derivation (locative +
/// attributive `-ғы`) is not yet modelled in the FST
/// morphotactics, so `қазақстандағы` returns no analysis and the
/// pre-v4.4.12 `best_noun_hint` chain fell through to None.
/// v4.4.12 added a string-level fallback `locative_attributive_hint`
/// that strips the 4-char suffix tail and recovers the base noun.
/// Closes the v4.4.11 carry-forward where `Қазақстандағы таулар
/// қандай?` answered with the generic refusal.
#[test]
fn locative_attributive_suffix_recovers_topic_noun_for_kazakhstan() {
    let Some(lex) = load_lexicon() else { return };
    // The fallback works string-side on the raw input, so we
    // don't need to thread parses through. The FST returns no
    // analysis for `қазақстандағы` (the unmodelled `-дағы`
    // suffix) — that's exactly what triggers the fallback.
    let intent =
        adam_dialog::interpret_text_with_lexicon("Қазақстандағы таулар қандай?", &[], Some(&lex));
    if let adam_dialog::Intent::Unknown { noun_hint, .. } = intent {
        assert_eq!(
            noun_hint.as_deref(),
            Some("қазақстан"),
            "locative-attributive `қазақстандағы` must recover topic noun `қазақстан`"
        );
    } else {
        panic!("unexpected intent: not Unknown");
    }
}

/// **v4.4.12** — same fix verified on a different input shape:
/// `Алматыдағы` (city + locative-attributive). The string-level
/// fallback handles both back-vowel (`-дағы`) and front-vowel
/// (`-дегі`) allomorphs because we strip a fixed suffix string,
/// not a phonological pattern.
#[test]
fn locative_attributive_suffix_recovers_topic_noun_for_almaty() {
    let Some(lex) = load_lexicon() else { return };
    let intent =
        adam_dialog::interpret_text_with_lexicon("Алматыдағы қалалар қандай?", &[], Some(&lex));
    if let adam_dialog::Intent::Unknown { noun_hint, .. } = intent {
        assert_eq!(
            noun_hint.as_deref(),
            Some("алматы"),
            "locative-attributive `алматыдағы` must recover topic noun `алматы`"
        );
    } else {
        panic!("unexpected intent: not Unknown");
    }
}

/// **v4.4.11** — list-summary world_core anchors. v4.4.11 added the
/// input-morpheme-overlap reranker (`query_input` on
/// `ToolContext`) and a `raw_text`-preserving renderer for
/// `RelatedTo` facts whose object root contains «тізім». Combined,
/// the system now surfaces the literal list when the user asks a
/// listing-style question. This test locks the data-layer floor:
/// every list-summary fact must contain its category name + a
/// representative member, so the reranker has something concrete
/// to surface.
#[test]
fn world_core_list_summary_facts_present_for_kazakhstan() {
    let path = "../../data/world_core/geography_kz.jsonl";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let raw = std::fs::read_to_string(path).expect("read geography_kz");
    let summaries: &[(&str, &[&str])] = &[
        ("аймақтары", &["17 облыс", "республикалық"]),
        ("көлдер", &["Балқаш", "Каспий", "Зайсан"]),
        ("өзендер", &["Ертіс", "Жайық", "Сырдария"]),
        ("тау жоталары", &["Алтай", "Тянь-Шань", "Хан Тәңірі"]),
        ("шөлдер", &["Бетпақдала", "Қызылқұм", "Үстірт"]),
    ];
    for (category, members) in summaries {
        assert!(
            raw.contains(category),
            "world_core list-summary must mention category {category:?}"
        );
        for member in *members {
            assert!(
                raw.contains(member),
                "world_core list-summary for {category} must mention {member:?}"
            );
        }
    }
}

/// **v4.3.5** — `topic_marker_hint` regression battery, mirroring
/// the user-shared 2026-04-26 dialog turns that exposed three
/// distinct extraction failures.
///
/// Pre-v4.3.5:
/// - `Жазушы Мүсірепов туралы не білесіз?` → `noun_hint = жазушы`
///   (common noun in lexicon won over proper noun out-of-lexicon).
/// - `Мен әйгілі жазушы Мүсірепов туралы сұрап отырмын` →
///   `noun_hint = әйгіл` (adjective root mistaken for noun).
/// - `Онда маған X туралы` → `noun_hint = он` (`Онда` parsed as
///   `он + Locative`; closed-class discourse particle leaked).
///
/// Post-v4.3.5: all three extract the **proper noun preceding the
/// `туралы` marker**, regardless of FST coverage. The marker is a
/// strong context signal — the word in front of it is what the
/// user is asking about.
#[test]
fn topic_marker_hint_picks_proper_noun_over_common_noun() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Жазушы Мүсірепов туралы не білесіз?", &lex, &repo, 0);
    let trace = conv.intent_history.last().copied();
    // Last recognised intent should be Unknown (no evidence path),
    // but the `noun_hint` should be the proper noun, not the
    // generic "жазушы".
    let _ = trace; // intent_history records IntentKind only; check via re-parse below
    let intent = adam_dialog::interpret_text_with_lexicon(
        "Жазушы Мүсірепов туралы не білесіз?",
        &[],
        Some(&lex),
    );
    if let adam_dialog::Intent::Unknown { noun_hint, .. } = intent {
        assert_eq!(
            noun_hint.as_deref().map(str::to_lowercase).as_deref(),
            Some("мүсірепов"),
            "topic must be the proper noun before `туралы`, not the generic `жазушы`"
        );
    } else {
        panic!("expected Intent::Unknown for the topic question");
    }
}

#[test]
fn topic_marker_hint_skips_adjective_root_jana_aigil() {
    let Some(lex) = load_lexicon() else { return };
    let intent = adam_dialog::interpret_text_with_lexicon(
        "Мен әйгілі жазушы Мүсірепов туралы сұрап отырмын",
        &[],
        Some(&lex),
    );
    if let adam_dialog::Intent::Unknown { noun_hint, .. } = intent {
        assert_eq!(
            noun_hint.as_deref().map(str::to_lowercase).as_deref(),
            Some("мүсірепов"),
            "the adjective `әйгіл` must NOT win over the proper noun before `туралы`"
        );
    } else {
        panic!("expected Intent::Unknown");
    }
}

#[test]
fn topic_marker_hint_ignores_onda_discourse_particle() {
    let Some(lex) = load_lexicon() else { return };
    let intent = adam_dialog::interpret_text_with_lexicon(
        "Онда маған ақын Омарбай Малқаров туралы айтып беріңізші",
        &[],
        Some(&lex),
    );
    if let adam_dialog::Intent::Unknown { noun_hint, .. } = intent {
        let lower = noun_hint.as_deref().map(str::to_lowercase);
        assert_eq!(
            lower.as_deref(),
            Some("малқаров"),
            "topic must be the surname before `туралы`, NOT `он` from `онда → он+Locative`"
        );
    } else {
        panic!("expected Intent::Unknown");
    }
}

/// **v4.3.5** — bare-noun topics (`жер туралы`) MUST stay
/// lowercase, mirroring how `first_noun_root` normalizes content
/// nouns. The goal_continuity scenarios depend on the lemma being
/// `жер`, not `Жер`. Locks the regression closed.
///
/// Routes through `Conversation::turn_with_trace` rather than
/// `interpret_text_with_lexicon` directly so the FST parses are
/// available — `topic_marker_hint`'s lowercase branch fires when
/// the cleaned word matches an FST-recognized noun lemma in the
/// parse list.
#[test]
fn topic_marker_hint_keeps_known_lemmas_lowercase() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let (_out, trace) = conv.turn_with_trace("жер туралы айтшы", &lex, &repo, 0);
    if let adam_dialog::Intent::Unknown { noun_hint, .. } = &trace.intent_after_injection {
        assert_eq!(
            noun_hint.as_deref(),
            Some("жер"),
            "known content-noun lemmas must come back lowercase"
        );
    } else {
        panic!(
            "expected Intent::Unknown, got {:?}",
            trace.intent_after_injection
        );
    }
}

/// **v4.3.4** — alternate creator-question phrasing (`авторың кім`)
/// also routes to the Creator aspect.
#[test]
fn ask_about_system_creator_aspect_alternate_phrasings() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let out = conv.turn("сенің авторың кім", &lex, &repo, 0);
    assert!(
        out.contains("Баймурзин"),
        "alternate creator-question phrasing must still surface the creator name (got: {out:?})"
    );
}
