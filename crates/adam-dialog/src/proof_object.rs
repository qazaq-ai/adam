//! Typed proof object — v5.8.0 (G2.0 of the proof-carrying generation
//! arc).
//!
//! Pre-v5.8.0 the dialog kernel produced text answers without an
//! explicit "what supports this claim?" object. The chain query
//! (`find_isa_chain`) returned a `Vec<String>` path; the safety-
//! refusal layer wrote a sentinel into `extra_slots`; verification
//! happened (when at all) through the post-hoc `audit_*` family in
//! `quality.rs`. The trust contract — every fact-bearing answer must
//! cite a curated source or grounded chain — was enforced by
//! discipline, not by the type system.
//!
//! G2.0 ships the [`ProofObject`] formalism: a typed bundle that
//! describes what the answer claims, what supports it, what
//! reasoning derived it, and what hedges qualify it. The
//! [`verifier::audit`] function gates emission: if `unsupported_claims`
//! is non-empty the response must not leave the system.
//!
//! ## Why this matters for the project's identity
//!
//! Codex's review framing: «генератор не "придумывает текст" — он
//! собирает допустимое высказывание из доказанных typed propositions».
//! G2.0 is the precondition for G3.0 (proof-carrying composition):
//! you can't compose over proof objects until they exist.
//!
//! ## Trajectory
//!
//! - **G1.0 (v5.7.0)** — typed slot inventory shipped
//! - **G1.5 (v5.7.5)** — realiser variation engine shipped
//! - **G2.0 (this milestone)** — `ProofObject` + `verifier::audit`
//!   shipped. Two retrofits: chain-query produces proof objects,
//!   safety-refusal layer produces proof objects. **Verification
//!   is opt-in** at this milestone — Conversation::turn doesn't
//!   yet gate every response. That's G2.5.
//! - **G2.5** — Conversation gates emission on verifier outcome.
//!   Templates that produce proof objects continue; those that
//!   don't fall back to existing path with a warning.
//! - **G3.0** — typed composer over proof objects → answer IR →
//!   realiser. Generation as proof-carrying composition.
//!
//! ## Why this fits the project's existing identity
//!
//! Per `MISSION.md` and the kernel-level discussion of v5.5.0 →
//! v5.6.x: the morphological algebra is itself a proof system at
//! the word level (FST roundtrip 100 %). G2.0 extends the same
//! discipline to the discourse layer — every claim composed at the
//! sentence level traces back to typed support, just as every
//! morpheme traces back to a typed operator.

use serde::{Deserialize, Serialize};

use adam_reasoning::FactSource;

/// Polarity of a claim. `Negated` is for explicit denials (the
/// v5.4.8 antonym path: «Тас — тірі емес, потому что тас → жансыз»);
/// `Affirmative` is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Polarity {
    Affirmative,
    Negated,
}

/// Categorical predicate of a claim. Different predicates warrant
/// different verification rules (e.g. `SafetyRefusal` doesn't need
/// curated support — the refusal *is* the proof).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimPredicate {
    /// Type membership: «X — это Y» / «X IsA Y».
    IsA,
    /// Property assertion: «X is round / hard / alive».
    HasProperty,
    /// Honest "I don't have data" — claim degenerates to the
    /// refusal itself.
    NoData,
    /// Safety refusal in a high-stakes domain (medical / legal /
    /// financial / current-data / political).
    SafetyRefusal { domain: SafetyDomain },
    /// Definitional answer (e.g. "X is the noun that means Y" for
    /// «X деген не?»).
    Definition,
    /// System-identity assertion (adam describing itself).
    SystemSelf,
    /// Catch-all for predicates not yet enumerated. The string is
    /// a stable slug for diagnostic / future-extension use.
    Other(String),
}

/// High-stakes domain for safety refusals. Mirrors the v5.6.5
/// `SafetyCategory` enum (kept independent here so the proof-object
/// module doesn't depend on the discourse layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyDomain {
    Medical,
    Legal,
    Financial,
    CurrentData,
    Political,
}

/// The single proposition this answer asserts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub subject: String,
    pub predicate: ClaimPredicate,
    pub object: String,
    pub polarity: Polarity,
}

/// What kind of support the [`ProofObject`] cites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportKind {
    /// Curated `world_core/*.jsonl` entry — strongest trust signal.
    CuratedFact,
    /// Derived via forward-chaining over curated + extracted facts.
    DerivedFact,
    /// Honest refusal template: "I don't have data". The refusal is
    /// itself the proof — no external support needed.
    TemplateRefusal,
    /// Antonym chain (v5.4.8 negation reasoning): subject reaches
    /// the antonym hub of the predicate.
    AntonymDenial,
    /// adam's own self-knowledge (system identity).
    SystemSelf,
    /// Domain-policy refusal (medical / legal / financial /
    /// current-data / political).
    PolicyRefusal,
}

/// One link in the support chain — points to provenance for a
/// claim. Multiple links can chain back to the same `FactSource`
/// when several extracted statements derive from one curated entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportLink {
    pub kind: SupportKind,
    /// Provenance to a `data/world_core/*.jsonl` or corpus pack
    /// entry. `None` for `TemplateRefusal` / `PolicyRefusal` /
    /// `SystemSelf` kinds where the support is the kernel itself.
    #[serde(default)]
    pub source: Option<FactSource>,
    /// Rule id for `DerivedFact`-kind support (e.g.
    /// "R1_is_a_transitivity").
    #[serde(default)]
    pub rule_id: Option<String>,
    /// Human-readable description of what this support contributes.
    /// Used by the verifier and `--trace` output.
    pub note: String,
}

/// Reasoning trace — the chain of intermediate steps that derived
/// the conclusion. For chain queries this is the path through the
/// IsA graph (`["қасқыр", "жыртқыш", "жануар", "тіршілік иесі",
/// "тірі"]`). `rule_ids` lists the forward-chaining rules that fired
/// along the path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationTrace {
    pub chain_path: Vec<String>,
    pub rule_ids: Vec<String>,
}

/// Honest qualifier on the answer. Hedges flag the limitations of
/// the proof to the consumer: a chain-derived answer is weaker than
/// a directly-curated one; an antonym denial is structural, not
/// observational; a no-data refusal is honest about absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HedgeMarker {
    /// "У меня нет данных по этому вопросу".
    NoData,
    /// "Не отвечаю — высокая ставка, обратитесь к специалисту".
    SafetyRefusal,
    /// "Знаю часть запроса, но не всё".
    PartialMatch,
    /// "Вывод по правилу, не первичный факт" — для R1-derived
    /// chains.
    DerivedNotCurated,
    /// "X не Y, потому что X — антоним Y" — для closed-class
    /// negation.
    AntonymDenial,
    /// "Я — тілдік модель, отвечаю за себя" — для system-identity
    /// turns.
    SystemSelf,
}

/// The typed proof bundle. An answer is **safe to emit** only when
/// the verifier confirms `unsupported_claims` is empty.
///
/// Construction patterns:
/// - [`ProofObject::from_isa_chain`] for chain-query results
/// - [`ProofObject::from_curated_fact`] for direct world_core hits
/// - [`ProofObject::from_antonym_denial`] for v5.4.8 negation
/// - [`ProofObject::safety_refusal`] for medical / legal / financial
///   / current-data refusals
/// - [`ProofObject::no_data_refusal`] for honest "не знаю"
/// - [`ProofObject::system_self`] for system-identity turns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofObject {
    pub conclusion: Claim,
    #[serde(default)]
    pub support: Vec<SupportLink>,
    #[serde(default)]
    pub derivation: Option<DerivationTrace>,
    #[serde(default)]
    pub hedges: Vec<HedgeMarker>,
    /// Sub-claims in the answer text that the proof can NOT support.
    /// **Empty is required for emission**. The verifier rejects any
    /// proof object with non-empty `unsupported_claims`; the response
    /// stays unrendered and the planner falls back to a refusal.
    #[serde(default)]
    pub unsupported_claims: Vec<String>,
}

impl ProofObject {
    /// Construct a proof for an IsA chain query (the v5.4.0 path).
    /// `chain_path` is the BFS path from subject to predicate;
    /// `rule_ids` is typically `["R1_is_a_transitivity"]` because
    /// chain query consumes derived facts from R1.
    pub fn from_isa_chain(
        subject: String,
        predicate: String,
        chain_path: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Self {
        let direct = chain_path.len() <= 2;
        let support_kind = if direct {
            SupportKind::CuratedFact
        } else {
            SupportKind::DerivedFact
        };
        let mut hedges = Vec::new();
        if !direct {
            hedges.push(HedgeMarker::DerivedNotCurated);
        }
        Self {
            conclusion: Claim {
                subject,
                predicate: ClaimPredicate::IsA,
                object: predicate,
                polarity: Polarity::Affirmative,
            },
            support: vec![SupportLink {
                kind: support_kind,
                source: None,
                rule_id: rule_ids.first().cloned(),
                note: format!(
                    "IsA chain depth {} via {} rule(s)",
                    chain_path.len(),
                    rule_ids.len()
                ),
            }],
            derivation: Some(DerivationTrace {
                chain_path,
                rule_ids,
            }),
            hedges,
            unsupported_claims: Vec::new(),
        }
    }

    /// Construct a proof for a v5.4.8-style antonym denial: subject
    /// reaches the antonym hub of the predicate, so the answer
    /// honestly denies the IsA relationship.
    pub fn from_antonym_denial(
        subject: String,
        predicate: String,
        antonym_chain: Vec<String>,
    ) -> Self {
        Self {
            conclusion: Claim {
                subject,
                predicate: ClaimPredicate::IsA,
                object: predicate,
                polarity: Polarity::Negated,
            },
            support: vec![SupportLink {
                kind: SupportKind::AntonymDenial,
                source: None,
                rule_id: None,
                note: "antonym chain reachable; positive chain absent".to_string(),
            }],
            derivation: Some(DerivationTrace {
                chain_path: antonym_chain,
                rule_ids: vec!["antonym_denial".to_string()],
            }),
            hedges: vec![HedgeMarker::AntonymDenial],
            unsupported_claims: Vec::new(),
        }
    }

    /// Construct a proof for a curated `world_core/*.jsonl` fact.
    /// Used by the v4.2.0 SearchGraph path. The `predicate_slug`
    /// captures the original predicate string ("is_a" / "has" /
    /// "part_of" / …) for diagnostic round-trip but doesn't
    /// influence the typed `ClaimPredicate::IsA` (curated facts go
    /// through the IsA conclusion path; non-IsA predicates need a
    /// dedicated builder added in a future patch).
    pub fn from_curated_fact(
        subject: String,
        predicate_slug: String,
        object: String,
        source: FactSource,
        raw_text: String,
    ) -> Self {
        let _ = predicate_slug; // kept for future non-IsA builders

        Self {
            conclusion: Claim {
                subject,
                predicate: ClaimPredicate::IsA,
                object,
                polarity: Polarity::Affirmative,
            },
            support: vec![SupportLink {
                kind: SupportKind::CuratedFact,
                source: Some(source),
                rule_id: None,
                note: raw_text,
            }],
            derivation: None,
            hedges: Vec::new(),
            unsupported_claims: Vec::new(),
        }
    }

    /// Construct a proof for a v5.6.5 safety refusal. The refusal
    /// itself is the proof — no curated support is required because
    /// the kernel is honestly declining to give advice in a high-
    /// stakes domain.
    pub fn safety_refusal(subject: String, predicate: String, domain: SafetyDomain) -> Self {
        Self {
            conclusion: Claim {
                subject,
                predicate: ClaimPredicate::SafetyRefusal { domain },
                object: predicate,
                polarity: Polarity::Affirmative,
            },
            support: vec![SupportLink {
                kind: SupportKind::PolicyRefusal,
                source: None,
                rule_id: None,
                note: format!("{domain:?} domain — outside kernel competence"),
            }],
            derivation: None,
            hedges: vec![HedgeMarker::SafetyRefusal],
            unsupported_claims: Vec::new(),
        }
    }

    /// Construct a proof for an honest "у меня нет данных" — the
    /// chain query failed AND no antonym path exists AND the topic
    /// isn't a high-stakes safety domain.
    pub fn no_data_refusal(subject: String, predicate: String) -> Self {
        Self {
            conclusion: Claim {
                subject,
                predicate: ClaimPredicate::NoData,
                object: predicate,
                polarity: Polarity::Affirmative,
            },
            support: vec![SupportLink {
                kind: SupportKind::TemplateRefusal,
                source: None,
                rule_id: None,
                note: "no chain found in extracted + derived facts".to_string(),
            }],
            derivation: None,
            hedges: vec![HedgeMarker::NoData],
            unsupported_claims: Vec::new(),
        }
    }

    /// Construct a proof for a system-identity turn (adam describing
    /// itself). The support is the kernel — the system knows what it
    /// is by construction.
    pub fn system_self(aspect: String, body_summary: String) -> Self {
        Self {
            conclusion: Claim {
                subject: "адам".to_string(),
                predicate: ClaimPredicate::SystemSelf,
                object: aspect,
                polarity: Polarity::Affirmative,
            },
            support: vec![SupportLink {
                kind: SupportKind::SystemSelf,
                source: None,
                rule_id: None,
                note: body_summary,
            }],
            derivation: None,
            hedges: vec![HedgeMarker::SystemSelf],
            unsupported_claims: Vec::new(),
        }
    }

    /// Mark a claim as unsupported. The verifier will reject any
    /// proof object with non-empty `unsupported_claims`.
    pub fn add_unsupported_claim(&mut self, claim: impl Into<String>) {
        self.unsupported_claims.push(claim.into());
    }

    /// True when the proof has no unsupported claims and at least
    /// one support link.
    pub fn is_emittable(&self) -> bool {
        self.unsupported_claims.is_empty() && !self.support.is_empty()
    }
}

/// Verifier — gates emission of an answer text against its proof
/// object. The contract: an answer is safe to emit ONLY when
/// `audit(answer, proof)` returns [`VerificationOutcome::Verified`].
pub mod verifier {
    use super::{ProofObject, VerificationOutcome};

    /// Audit an answer text against its proof object. Returns
    /// [`VerificationOutcome::Verified`] when the proof has at least
    /// one support link AND `unsupported_claims` is empty AND the
    /// answer text is non-empty. Returns specific failure variants
    /// otherwise.
    ///
    /// **What this does NOT yet do (v5.8.0 / G2.0).** It does not
    /// parse the answer text and check that every NL claim maps to
    /// a support entry. That cross-validation lands in G2.5 once the
    /// claim-extraction surface is built. For now the verifier
    /// trusts the producer (each `from_*` constructor) to fill the
    /// proof correctly; this gate catches structural bugs (empty
    /// support / non-empty unsupported_claims) early.
    pub fn audit(answer_text: &str, proof: &ProofObject) -> VerificationOutcome {
        if answer_text.trim().is_empty() {
            return VerificationOutcome::EmptyAnswer;
        }
        if !proof.unsupported_claims.is_empty() {
            return VerificationOutcome::UnsupportedClaims(proof.unsupported_claims.clone());
        }
        if proof.support.is_empty() {
            return VerificationOutcome::MissingSupport;
        }
        VerificationOutcome::Verified
    }
}

/// Result of [`verifier::audit`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationOutcome {
    /// Proof is structurally valid; safe to emit the answer.
    Verified,
    /// One or more sub-claims in the answer are not supported by the
    /// proof. Each entry is a description of the unsupported claim.
    /// **Caller must NOT emit** the answer; fall back to a refusal.
    UnsupportedClaims(Vec<String>),
    /// The proof has no support links at all. Possible in malformed
    /// builders; caller must NOT emit.
    MissingSupport,
    /// The answer text itself is empty. Caller must NOT emit (this
    /// usually indicates a bug upstream).
    EmptyAnswer,
}

impl VerificationOutcome {
    /// True when the answer is safe to emit.
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isa_chain_proof_is_emittable_v580() {
        let proof = ProofObject::from_isa_chain(
            "қасқыр".into(),
            "тірі".into(),
            vec!["қасқыр".into(), "тіршілік иесі".into(), "тірі".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        assert!(proof.is_emittable());
        assert_eq!(proof.conclusion.polarity, Polarity::Affirmative);
        assert_eq!(proof.hedges, vec![HedgeMarker::DerivedNotCurated]);
        assert!(matches!(proof.support[0].kind, SupportKind::DerivedFact));
    }

    #[test]
    fn direct_isa_chain_marks_curated_fact_v580() {
        let proof = ProofObject::from_isa_chain(
            "ит".into(),
            "жануар".into(),
            vec!["ит".into(), "жануар".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        assert!(matches!(proof.support[0].kind, SupportKind::CuratedFact));
        // Depth 2 — direct curated edge — no DerivedNotCurated hedge.
        assert!(proof.hedges.is_empty());
    }

    #[test]
    fn antonym_denial_is_negated_v580() {
        let proof = ProofObject::from_antonym_denial(
            "тас".into(),
            "тірі".into(),
            vec!["тас".into(), "жансыз нәрсе".into(), "жансыз".into()],
        );
        assert_eq!(proof.conclusion.polarity, Polarity::Negated);
        assert!(matches!(proof.support[0].kind, SupportKind::AntonymDenial));
    }

    #[test]
    fn safety_refusal_carries_domain_v580() {
        let proof =
            ProofObject::safety_refusal("басым".into(), "дәрі".into(), SafetyDomain::Medical);
        assert!(matches!(
            proof.conclusion.predicate,
            ClaimPredicate::SafetyRefusal {
                domain: SafetyDomain::Medical
            }
        ));
        assert!(matches!(proof.support[0].kind, SupportKind::PolicyRefusal));
    }

    #[test]
    fn verifier_accepts_well_formed_proof_v580() {
        let proof = ProofObject::from_isa_chain(
            "қасқыр".into(),
            "тірі".into(),
            vec!["қасқыр".into(), "тірі".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        let result = verifier::audit("Қасқыр — тірі.", &proof);
        assert_eq!(result, VerificationOutcome::Verified);
        assert!(result.is_verified());
    }

    #[test]
    fn verifier_rejects_unsupported_claims_v580() {
        let mut proof = ProofObject::from_isa_chain(
            "қасқыр".into(),
            "тірі".into(),
            vec!["қасқыр".into(), "тірі".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        proof.add_unsupported_claim("Қасқыр Алматыда тұрады");
        let result = verifier::audit("text", &proof);
        assert!(!result.is_verified());
        match result {
            VerificationOutcome::UnsupportedClaims(claims) => {
                assert_eq!(claims, vec!["Қасқыр Алматыда тұрады".to_string()]);
            }
            _ => panic!("expected UnsupportedClaims, got {result:?}"),
        }
    }

    #[test]
    fn verifier_rejects_empty_support_v580() {
        let proof = ProofObject {
            conclusion: Claim {
                subject: "X".into(),
                predicate: ClaimPredicate::IsA,
                object: "Y".into(),
                polarity: Polarity::Affirmative,
            },
            support: vec![],
            derivation: None,
            hedges: vec![],
            unsupported_claims: vec![],
        };
        let result = verifier::audit("X — Y", &proof);
        assert_eq!(result, VerificationOutcome::MissingSupport);
    }

    #[test]
    fn verifier_rejects_empty_answer_v580() {
        let proof = ProofObject::no_data_refusal("X".into(), "Y".into());
        let result = verifier::audit("", &proof);
        assert_eq!(result, VerificationOutcome::EmptyAnswer);
    }

    #[test]
    fn no_data_refusal_is_emittable_v580() {
        // No-data refusal IS itself the proof — it claims absence,
        // and the absence is observed structurally (chain query
        // failed). The verifier accepts this.
        let proof = ProofObject::no_data_refusal("X".into(), "Y".into());
        let result = verifier::audit("у меня нет данных", &proof);
        assert_eq!(result, VerificationOutcome::Verified);
        assert!(matches!(proof.conclusion.predicate, ClaimPredicate::NoData));
    }

    #[test]
    fn proof_object_round_trips_through_serde_v580() {
        // ProofObjects need serde for future trace-export use cases
        // (logging proofs alongside answers for audit).
        let proof = ProofObject::from_isa_chain(
            "ит".into(),
            "жануар".into(),
            vec!["ит".into(), "жануар".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        let json = serde_json::to_string(&proof).expect("serialize");
        let back: ProofObject = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(proof, back);
    }

    #[test]
    fn add_unsupported_claim_blocks_emission_v580() {
        let mut proof = ProofObject::from_isa_chain(
            "ит".into(),
            "жануар".into(),
            vec!["ит".into(), "жануар".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        assert!(proof.is_emittable());
        proof.add_unsupported_claim("phantom claim");
        assert!(!proof.is_emittable());
    }
}
