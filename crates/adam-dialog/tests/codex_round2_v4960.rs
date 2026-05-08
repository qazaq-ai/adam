//! **v4.96.0** — Codex 2026-05-07 round-2 audit regression tests.
//!
//! Captures the 5 bugs Codex flagged that are addressable in this
//! release (Bug 2 / 4 / 6 / 7 / 1 — Bug 3 REPL multi-line and Bug 5
//! anaphora deferred to medium-term per agreed roadmap).

use std::collections::HashMap;

use adam_dialog::{Intent, planner::plan_response_with_session, templates::TemplateRepository};

fn repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml")
}

/// **Bug 2 — seed-leak in pedagogical families.** Pre-v4.96.0
/// 40 % of seeds picked the clarification template even when a
/// topic was set. Post-fix: 0 % (sub-key remap on slot presence).
#[test]
fn ask_exercise_with_topic_never_routes_to_clarification() {
    let repo = repo();
    let intent = Intent::AskExercise {
        topic: Some("ownership".into()),
    };
    let session = HashMap::new();
    let mut clarify_hits = 0;
    let mut topic_template_hits = 0;
    for seed in 0..50u64 {
        let plan = plan_response_with_session(&intent, seed, &repo, &session);
        let lit = plan.literal.to_lowercase();
        if lit.contains("қандай тақырып") {
            clarify_hits += 1;
        }
        // Topic-bearing templates contain `{topic}` and `{exercise_body}`
        // placeholders — we check the placeholders are present in the
        // raw template (substitution happens in the realiser stage).
        if lit.contains("{topic}") || lit.contains("{exercise_body}") {
            topic_template_hits += 1;
        }
    }
    assert_eq!(
        clarify_hits, 0,
        "topic-bearing exercise should NEVER pick clarification template"
    );
    assert_eq!(
        topic_template_hits, 50,
        "all 50 seeds should pick a topic-bearing template; got {topic_template_hits}/50"
    );
    // Slot map should carry the topic so the realiser can fill it.
    let plan = plan_response_with_session(&intent, 0, &repo, &session);
    assert_eq!(
        plan.slots.get("topic").map(String::as_str),
        Some("ownership")
    );
    assert!(plan.slots.contains_key("exercise_body"));
}

#[test]
fn code_request_with_topic_never_routes_to_clarification() {
    let repo = repo();
    let intent = Intent::CodeRequest {
        topic: Some("hello world".into()),
    };
    let session = HashMap::new();
    let mut clarify_hits = 0;
    for seed in 0..50u64 {
        let plan = plan_response_with_session(&intent, seed, &repo, &session);
        if plan.literal.to_lowercase().contains("қандай тақырыпта код") {
            clarify_hits += 1;
        }
    }
    assert_eq!(clarify_hits, 0, "topic-bearing CodeRequest never clarify");
}

#[test]
fn ask_purpose_with_topic_never_routes_to_clarification() {
    let repo = repo();
    let intent = Intent::AskPurpose {
        topic: Some("ownership".into()),
    };
    let session = HashMap::new();
    let mut clarify_hits = 0;
    for seed in 0..50u64 {
        let plan = plan_response_with_session(&intent, seed, &repo, &session);
        if plan
            .literal
            .to_lowercase()
            .contains("қандай ұғымның мақсатын")
        {
            clarify_hits += 1;
        }
    }
    assert_eq!(clarify_hits, 0, "topic-bearing AskPurpose never clarify");
}

#[test]
fn explain_compiler_error_with_code_never_clarify() {
    let repo = repo();
    let intent = Intent::ExplainCompilerError {
        error_code: Some("E0382".into()),
        topic: None,
    };
    let session = HashMap::new();
    let mut clarify_hits = 0;
    for seed in 0..50u64 {
        let plan = plan_response_with_session(&intent, seed, &repo, &session);
        if plan
            .literal
            .to_lowercase()
            .contains("қателіктің нөмірін (e0xxx)")
        {
            clarify_hits += 1;
        }
    }
    assert_eq!(
        clarify_hits, 0,
        "known-code ExplainCompilerError never clarify"
    );
}

#[test]
fn topicless_pedagogical_intents_route_to_clarification() {
    // Inverse: when topic is None, clarification is the right route.
    let repo = repo();
    let session = HashMap::new();
    let plan = plan_response_with_session(&Intent::AskExercise { topic: None }, 0, &repo, &session);
    assert!(
        plan.literal.to_lowercase().contains("қандай тақырып"),
        "topicless exercise should clarify; got: {}",
        plan.literal
    );
}

/// **Bug 4 — inline language tag.** Single-line «```rust X ```»
/// should strip the leading `rust ` token so the body is just `X`.
#[test]
fn detect_submit_solution_strips_inline_rust_tag() {
    let input = "Test ```rust let x = 5; println!(\"{x}\"); ```";
    let result = adam_dialog::semantics::detect_submit_solution(input);
    assert!(result.is_some(), "expected SubmitSolution match");
    let (code, _) = result.unwrap();
    assert!(
        !code.starts_with("rust "),
        "leading `rust ` tag should be stripped; got body: {code:?}"
    );
    assert!(
        code.contains("let x = 5"),
        "body should contain the actual code; got: {code:?}"
    );
}

/// **Bug 6 — Russian-only refusal.** «Расскажи про Rust» should
/// route to non-Kazakh refusal, not surface Rust topic answer.
#[test]
fn russian_imperative_routes_to_non_kazakh_refusal() {
    use adam_dialog::discourse::input_is_likely_russian;
    assert!(
        input_is_likely_russian("Расскажи про Rust"),
        "Russian imperative «Расскажи про Rust» should be detected"
    );
    assert!(
        input_is_likely_russian("Объясните мне ownership"),
        "Russian explanation request should be detected"
    );
    assert!(
        input_is_likely_russian("Покажи код"),
        "Russian show-code request should be detected"
    );
    // Negative: pure Kazakh with no Russian markers stays through.
    assert!(
        !input_is_likely_russian("Маған Rust туралы айтыңыз."),
        "Kazakh sentence should NOT trigger Russian refusal"
    );
}

/// **Bug 7 — cross-language contrast.** «Python-да ownership бар
/// ма?» should route to `CrossLanguageContrast`, not surface the
/// Rust ownership definition without contrast.
#[test]
fn cross_language_contrast_detected_python_ownership() {
    use adam_dialog::semantics::detect_cross_language_contrast;
    let r = detect_cross_language_contrast("Python-да ownership бар ма?");
    assert!(r.is_some(), "expected CrossLanguageContrast match");
    let (lang, concept) = r.unwrap();
    assert_eq!(lang, "python");
    assert_eq!(concept, "ownership");
}

#[test]
fn cross_language_contrast_detected_java_lifetime() {
    use adam_dialog::semantics::detect_cross_language_contrast;
    let r = detect_cross_language_contrast("Java-да lifetime бар ма?");
    assert!(
        r.is_some(),
        "expected CrossLanguageContrast for Java/lifetime"
    );
    let (lang, concept) = r.unwrap();
    assert_eq!(lang, "java");
    assert_eq!(concept, "lifetime");
}

#[test]
fn cross_language_contrast_negative_pure_rust_query() {
    use adam_dialog::semantics::detect_cross_language_contrast;
    // Pure Rust query without another language mention shouldn't fire.
    let r = detect_cross_language_contrast("Rust-та ownership деген не?");
    assert!(
        r.is_none(),
        "pure-Rust query should NOT fire CrossLanguageContrast; got {r:?}"
    );
}
