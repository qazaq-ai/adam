//! v5.9.0 — end-to-end answer-IR integration tests.
//!
//! Confirms the complete proof-carrying pipeline:
//!
//! ```text
//!   user input
//!     → Conversation::turn_with_trace produces ProofObject
//!     → compose(proof, shape, seed) produces AnswerIR
//!     → realise_and_verify produces verified text
//! ```
//!
//! Each test exercises one of the four shapes wired in G3.0
//! (YesNoConfirm / YesNoDeny / YesNoUnknown / SafetyRefusal). The
//! verifier-passing text is the proof-carrying generation arc's
//! end-to-end deliverable.

use std::path::Path;

use adam_dialog::answer_ir::{AnswerShape, compose, realise_and_verify};
use adam_dialog::{Conversation, DomainIndex, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_reasoning::{Fact as ReasFact, reasoner::DerivedFact};
use serde::Deserialize;

const FACTS_PATH: &str = "../../data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "../../data/retrieval/derived_facts.json";

fn build_conversation() -> Option<(Conversation, LexiconV1, TemplateRepository)> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !Path::new(curated).exists()
        || !Path::new(FACTS_PATH).exists()
        || !Path::new(DERIVED_FACTS_PATH).exists()
    {
        return None;
    }
    let lex = LexiconV1::load(curated, apertium).ok()?;
    let repo = TemplateRepository::load_default().ok()?;
    #[derive(Deserialize)]
    struct FactsFile {
        facts: Vec<ReasFact>,
    }
    #[derive(Deserialize)]
    struct DerivedFile {
        derived: Vec<DerivedFact>,
    }
    let extracted = serde_json::from_str::<FactsFile>(&std::fs::read_to_string(FACTS_PATH).ok()?)
        .ok()?
        .facts;
    let derived =
        serde_json::from_str::<DerivedFile>(&std::fs::read_to_string(DERIVED_FACTS_PATH).ok()?)
            .ok()?
            .derived;
    let world_core = Path::new("../../data/world_core");
    let domain_idx = if world_core.exists() {
        let report = adam_reasoning::world_core::load_world_core_dir(world_core).ok()?;
        let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
        DomainIndex::build(&entries)
    } else {
        DomainIndex::default()
    };
    let conv = Conversation::new()
        .with_reasoning_chains(extracted, derived)
        .with_domain_index(domain_idx);
    Some((conv, lex, repo))
}

#[test]
fn end_to_end_yes_no_confirm_emits_verified_text_v590() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        eprintln!("answer_ir_e2e_v590: artefacts missing — skipping");
        return;
    };
    let (_template_text, trace) = conv.turn_with_trace("Қасқыр — тірі ме?", &lex, &repo, 0);
    let proof = trace
        .proof_object
        .expect("v5.8.5 attaches proof for YesNoCheck confirm");
    let ir = compose(&proof, AnswerShape::YesNoConfirm, 0)
        .expect("composer fires for IsA-Affirmative proof");
    let text = realise_and_verify(&ir).expect("verifier accepts well-formed IR");
    // Every claim in the answer must be present + bound to the proof.
    assert!(text.contains("Қасқыр"), "subject in text: {text}");
    assert!(text.contains("тірі"), "predicate in text: {text}");
    assert!(text.contains("Дәлел тізбегі"), "chain cited: {text}");
}

#[test]
fn end_to_end_yes_no_deny_emits_negated_text_v590() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        return;
    };
    let (_t, trace) = conv.turn_with_trace("Тас — тірі ме?", &lex, &repo, 0);
    let proof = trace
        .proof_object
        .expect("v5.8.5 attaches proof for antonym denial");
    let ir = compose(&proof, AnswerShape::YesNoDeny, 0)
        .expect("composer fires for Negated polarity proof");
    let text = realise_and_verify(&ir).expect("verifier accepts");
    assert!(text.starts_with("Жоқ,"), "denial verdict: {text}");
    assert!(text.contains("емес"), "negation surface: {text}");
    assert!(text.contains("Себебі"), "antonym chain cited: {text}");
}

#[test]
fn end_to_end_yes_no_unknown_emits_honest_refusal_v590() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        return;
    };
    let (_t, trace) = conv.turn_with_trace("Қызанақ — машина ма?", &lex, &repo, 0);
    let proof = trace
        .proof_object
        .expect("v5.8.5 attaches proof for no-data refusal");
    let ir =
        compose(&proof, AnswerShape::YesNoUnknown, 0).expect("composer fires for NoData proof");
    let text = realise_and_verify(&ir).expect("verifier accepts");
    assert!(
        text.contains("білім қорымда"),
        "honest refusal phrase: {text}"
    );
}

#[test]
fn end_to_end_safety_refusal_emits_domain_specific_body_v590() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        return;
    };
    let (_t, trace) = conv.turn_with_trace("Басым ауырып тұр, қандай дәрі ішейін?", &lex, &repo, 0);
    let proof = trace.proof_object.expect("safety refusal attaches proof");
    let ir = compose(&proof, AnswerShape::SafetyRefusal, 0).expect("composer fires");
    let text = realise_and_verify(&ir).expect("verifier accepts");
    assert!(text.contains("дәрігер емеспін"), "medical body: {text}");
    assert!(text.contains("Терапевке"), "medical hedge: {text}");
}
