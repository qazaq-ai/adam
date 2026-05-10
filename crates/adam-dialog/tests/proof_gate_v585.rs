//! v5.8.5 — G2.5 verifier-gate integration tests.
//!
//! Confirms that `Conversation::turn_with_trace` constructs a typed
//! `ProofObject` for the four paths wired in G2.5:
//!
//!   1. YesNoCheck → confirm (chain found): IsA chain proof
//!   2. YesNoCheck → deny (antonym hub reachable): negated polarity
//!      proof via `from_antonym_denial`
//!   3. YesNoCheck → unknown (no chain, no antonym): `no_data_refusal`
//!   4. Safety topic detected: `safety_refusal` with the matching
//!      `SafetyDomain`
//!
//! For every case the verifier outcome is `Verified` — the builders
//! ship structurally well-formed proofs by construction. The gate's
//! value at G2.5 is **catching future regressions** if someone wires
//! in a producer that emits malformed proofs.

use std::path::Path;

use adam_dialog::proof_object::{
    ClaimPredicate, Polarity, SafetyDomain, SupportKind, VerificationOutcome,
};
use adam_dialog::{Conversation, DomainIndex, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_reasoning::{Fact as ReasFact, reasoner::DerivedFact};
use serde::Deserialize;

const FACTS_PATH: &str = "../../data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "../../data/retrieval/derived_facts.json";

fn load_runtime() -> Option<(Vec<ReasFact>, Vec<DerivedFact>)> {
    if !Path::new(FACTS_PATH).exists() || !Path::new(DERIVED_FACTS_PATH).exists() {
        return None;
    }
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
    Some((extracted, derived))
}

fn build_conversation() -> Option<(Conversation, LexiconV1, TemplateRepository)> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !Path::new(curated).exists() {
        return None;
    }
    let lex = LexiconV1::load(curated, apertium).ok()?;
    let repo = TemplateRepository::load_default().ok()?;
    let (extracted, derived) = load_runtime()?;
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
fn yes_no_confirm_path_produces_verified_proof_v585() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        eprintln!("proof_gate_v585: runtime artefacts missing — skipping");
        return;
    };
    let (_text, trace) = conv.turn_with_trace("Қасқыр — тірі ме?", &lex, &repo, 0);
    let proof = trace
        .proof_object
        .as_ref()
        .expect("YesNoCheck confirm path must attach a ProofObject");
    assert!(matches!(proof.conclusion.predicate, ClaimPredicate::IsA));
    assert_eq!(proof.conclusion.polarity, Polarity::Affirmative);
    let outcome = trace
        .verification_outcome
        .clone()
        .expect("verifier must run when proof is attached");
    assert!(
        outcome.is_verified(),
        "G2.5 contract: chain-confirm proofs must verify; got {outcome:?}"
    );
}

#[test]
fn yes_no_deny_path_produces_negated_proof_v585() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        return;
    };
    let (_text, trace) = conv.turn_with_trace("Тас — тірі ме?", &lex, &repo, 0);
    let proof = trace
        .proof_object
        .as_ref()
        .expect("antonym-denial path must attach a ProofObject");
    assert_eq!(proof.conclusion.polarity, Polarity::Negated);
    assert!(matches!(proof.support[0].kind, SupportKind::AntonymDenial));
    assert!(
        trace
            .verification_outcome
            .as_ref()
            .map(VerificationOutcome::is_verified)
            .unwrap_or(false),
        "antonym-denial proof must verify"
    );
}

#[test]
fn yes_no_no_data_path_produces_refusal_proof_v585() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        return;
    };
    // «Қызанақ — машина ма?» — neither positive chain nor antonym hub
    // reachable; v5.8.5 attaches a no_data_refusal proof.
    let (_text, trace) = conv.turn_with_trace("Қызанақ — машина ма?", &lex, &repo, 0);
    let proof = trace
        .proof_object
        .as_ref()
        .expect("no-data refusal path must attach a ProofObject");
    assert!(matches!(proof.conclusion.predicate, ClaimPredicate::NoData));
    assert!(matches!(
        proof.support[0].kind,
        SupportKind::TemplateRefusal
    ));
    assert!(
        trace
            .verification_outcome
            .as_ref()
            .map(VerificationOutcome::is_verified)
            .unwrap_or(false),
        "no-data refusal proof must verify"
    );
}

#[test]
fn safety_topic_path_produces_safety_refusal_proof_v585() {
    let Some((mut conv, lex, repo)) = build_conversation() else {
        return;
    };
    let (_text, trace) =
        conv.turn_with_trace("Басым ауырып тұр, қандай дәрі ішейін?", &lex, &repo, 0);
    let proof = trace
        .proof_object
        .as_ref()
        .expect("safety topic path must attach a ProofObject");
    assert!(matches!(
        proof.conclusion.predicate,
        ClaimPredicate::SafetyRefusal {
            domain: SafetyDomain::Medical
        }
    ));
    assert!(matches!(proof.support[0].kind, SupportKind::PolicyRefusal));
    assert!(
        trace
            .verification_outcome
            .as_ref()
            .map(VerificationOutcome::is_verified)
            .unwrap_or(false),
        "safety refusal proof must verify"
    );
}

#[test]
fn non_proof_paths_have_no_proof_object_v585() {
    // Regression guard: paths that don't produce a typed proof at
    // G2.5 (greetings, statements, generic Definition shape) leave
    // `proof_object` and `verification_outcome` both `None`. v5.8.5
    // is opt-in path-by-path; this test fences the current scope.
    let Some((mut conv, lex, repo)) = build_conversation() else {
        return;
    };
    let (_text, trace) = conv.turn_with_trace("Сәлем", &lex, &repo, 0);
    assert!(
        trace.proof_object.is_none(),
        "greeting must not attach a proof object yet"
    );
    assert!(trace.verification_outcome.is_none());
}
