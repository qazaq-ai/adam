//! v5.8.0 — end-to-end proof-object construction tests.
//!
//! Exercises the four primary paths that produce a `ProofObject` in
//! this release:
//!
//!   1. Chain query (`find_isa_proof`) over the live extracted +
//!      derived facts in `data/retrieval/`. Confirms a real chain
//!      («қасқыр → ... → тірі») produces a verifier-passing proof.
//!   2. `ProofObject::from_curated_fact` over a single curated entry
//!      from `data/world_core/`. Confirms `FactSource` provenance
//!      survives round-trip.
//!   3. `ProofObject::safety_refusal` over a `SafetyCategory` from
//!      the discourse-layer detector. Confirms the `From` bridge
//!      preserves semantics.
//!   4. `ProofObject::no_data_refusal` for the negative case (chain
//!      query miss + no antonym path) — confirms the kernel can
//!      honestly produce a refusal proof.
//!
//! Skips silently when runtime artefacts are absent (trimmed CI
//! checkouts).

use std::path::Path;

use adam_dialog::discourse::{SafetyCategory, detect_safety_topic};
use adam_dialog::proof_object::{
    ClaimPredicate, HedgeMarker, Polarity, ProofObject, SafetyDomain, SupportKind, verifier,
};
use adam_reasoning::{Fact as ReasFact, FactSource, reasoner::DerivedFact};
use serde::Deserialize;

const FACTS_PATH: &str = "../../data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "../../data/retrieval/derived_facts.json";

fn load_runtime_facts() -> Option<(Vec<ReasFact>, Vec<DerivedFact>)> {
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

#[test]
fn safety_category_bridges_to_safety_domain_v580() {
    // Discourse-layer detection → proof-layer typed enum, via the
    // v5.8.0 `From` bridge. Each category must map cleanly.
    let medical = detect_safety_topic("Басым ауырып тұр, қандай дәрі ішейін?")
        .expect("medical query must trigger safety");
    let domain: SafetyDomain = medical.into();
    assert!(matches!(domain, SafetyDomain::Medical));

    let financial = detect_safety_topic("Маған банк несиесін алуға кеңес бер.")
        .expect("financial query must trigger safety");
    let domain: SafetyDomain = financial.into();
    assert!(matches!(domain, SafetyDomain::Financial));

    let current = detect_safety_topic("Bitcoin бағасы қандай?")
        .expect("current-data query must trigger safety");
    let domain: SafetyDomain = current.into();
    assert!(matches!(domain, SafetyDomain::CurrentData));
}

#[test]
fn safety_refusal_proof_is_emittable_v580() {
    // Detected category → ProofObject::safety_refusal → verifier OK.
    let cat = detect_safety_topic("Басым ауырып тұр, қандай дәрі ішейін?")
        .expect("medical detection must fire");
    let domain: SafetyDomain = cat.into();
    let proof = ProofObject::safety_refusal("басым".into(), "дәрі".into(), domain);
    let answer = "Денсаулыққа қатысты нақты кеңес бере алмаймын — мен дәрігер емеспін.";
    let outcome = verifier::audit(answer, &proof);
    assert!(
        outcome.is_verified(),
        "safety refusal proof must verify; got {outcome:?}"
    );
    assert!(matches!(proof.support[0].kind, SupportKind::PolicyRefusal));
    assert_eq!(proof.hedges, vec![HedgeMarker::SafetyRefusal]);
}

#[test]
fn isa_chain_proof_over_live_facts_v580() {
    // The full G2.0 path: real extracted + derived facts → chain
    // query → typed ProofObject → verifier accepts. Skip silently
    // when runtime artefacts aren't shipped in this checkout.
    let Some((extracted, derived)) = load_runtime_facts() else {
        eprintln!("proof_object_e2e: runtime artefacts missing — skipping");
        return;
    };
    let proof = adam_dialog::conversation::find_isa_proof(&extracted, &derived, "қасқыр", "тірі")
        .expect("қасқыр → тірі chain must resolve via v5.4.0+v5.4.5 bridges");

    assert_eq!(proof.conclusion.subject, "қасқыр");
    assert_eq!(proof.conclusion.object, "тірі");
    assert!(matches!(proof.conclusion.predicate, ClaimPredicate::IsA));
    assert_eq!(proof.conclusion.polarity, Polarity::Affirmative);
    assert!(!proof.support.is_empty());
    assert!(proof.derivation.is_some());
    assert!(proof.unsupported_claims.is_empty());

    // Verifier accepts the bundle — answer is safe to emit.
    let outcome = verifier::audit(
        "Қасқыр — тірі. Бұл қасқыр → тірі тізбегі арқылы расталады.",
        &proof,
    );
    assert!(outcome.is_verified(), "chain proof must verify");
}

#[test]
fn from_curated_fact_round_trips_provenance_v580() {
    // ProofObject::from_curated_fact must preserve the FactSource
    // pack + sample_id so the audit trail can cite the exact
    // world_core entry.
    let source = FactSource {
        pack: "world_core/animals.jsonl".into(),
        sample_id: "anm_034".into(),
    };
    let proof = ProofObject::from_curated_fact(
        "ит".into(),
        "is_a".into(),
        "үй жануары".into(),
        source.clone(),
        "Ит — үй жануары.".into(),
    );
    assert_eq!(proof.support[0].source, Some(source));
    assert!(matches!(proof.support[0].kind, SupportKind::CuratedFact));
    let outcome = verifier::audit("Ит — үй жануары.", &proof);
    assert!(outcome.is_verified());
}

#[test]
fn no_data_refusal_proves_absence_v580() {
    // Honest "у меня нет данных" produces a verifier-passing proof
    // because the absence is observed structurally (chain query
    // failed, no antonym hub reachable). Confirms the kernel can
    // refuse with a proper proof — not just by silence.
    let proof = ProofObject::no_data_refusal("кітап".into(), "тағам".into());
    assert!(matches!(proof.conclusion.predicate, ClaimPredicate::NoData));
    assert_eq!(proof.hedges, vec![HedgeMarker::NoData]);
    let outcome = verifier::audit(
        "Менің білім қорымда Кітап — тағам екеніне дерек жоқ.",
        &proof,
    );
    assert!(outcome.is_verified());
}

#[test]
fn antonym_denial_carries_negated_polarity_v580() {
    // The v5.4.8 antonym path produces a proof with Negated polarity
    // — the answer denies an IsA claim because the subject reaches
    // the antonym hub.
    let proof = ProofObject::from_antonym_denial(
        "тас".into(),
        "тірі".into(),
        vec!["тас".into(), "жансыз нәрсе".into(), "жансыз".into()],
    );
    assert_eq!(proof.conclusion.polarity, Polarity::Negated);
    assert!(matches!(proof.support[0].kind, SupportKind::AntonymDenial));
    let outcome = verifier::audit("Жоқ, Тас — тірі емес. Себебі: тас → жансыз.", &proof);
    assert!(outcome.is_verified());
}

#[test]
fn detector_skips_factual_definitions_v580() {
    // Regression guard: factual «X деген не?» queries must NOT
    // trigger the safety detector; the proof-object path for them
    // is `from_curated_fact`, not `safety_refusal`.
    assert!(detect_safety_topic("Дәрі деген не?").is_none());
    assert!(detect_safety_topic("Заң деген не?").is_none());
    assert!(detect_safety_topic("Несие деген не?").is_none());
    let _ = SafetyCategory::Medical; // keep import live
}
