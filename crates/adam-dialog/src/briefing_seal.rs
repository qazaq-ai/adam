//! Protocol Seal — turn a graded [`BriefingProtocol`] into a signed,
//! publicly verifiable допуск artifact.
//!
//! The deterministic engine produces the *content* of the protocol; this
//! module wraps that content, plus the caller-side context the engine
//! cannot own (who was briefed, who signed off, when, where), into a
//! **canonical envelope** and seals it with an Ed25519 signature
//! ([`adam_seal`]).  A service inspector or a court expert can then
//! verify the seal with any independent Ed25519 implementation — the
//! artifact does not require trusting our program.
//!
//! ## Canonical form
//!
//! The signed bytes are the **compact JSON of [`SealEnvelope`]**.  Field
//! order is fixed by the struct declaration and every quantity is an
//! integer or string (coverage is stored as `coverage_permille`, never a
//! float), so the serialization is fully deterministic.  Verification
//! re-serializes the parsed envelope, so pretty-printing or reordering
//! the stored file never breaks a valid seal.
//!
//! The engine's fast FNV `content_digest` is carried inside the envelope
//! as `content_digest` — a human-checkable cross-reference — but the
//! legal tamper-evidence is the Ed25519 signature over the whole
//! envelope, not that checksum.
//!
//! ## Deliberately out of scope (until a deployment asks)
//!
//! No key hierarchy, rotation, revocation, or ledger.  `prev_record_hash`
//! is reserved in the format so a future append-only journal can chain
//! records, but this version does not build one.

use adam_seal::{SEAL_ALG, SigningKey, sha256, to_hex, verify_hex};
use serde::{Deserialize, Serialize};

use crate::briefing_session::BriefingProtocol;

/// Format tag stored in every sealed protocol.
pub const SEALED_FORMAT: &str = "adam-sealed-protocol/1";

/// Caller-supplied context the deterministic engine does not own.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SealContext {
    /// Name of the briefed worker.
    pub worker: String,
    /// Name of the ИТҚ / operator who conducted and signs off.
    pub operator: String,
    /// Local wall-clock timestamp, e.g. `2026-07-04 14:32`.
    pub timestamp: String,
    /// Timezone label for `timestamp`, e.g. `UTC+05:00`.
    pub timezone: String,
    /// Site / device identifier; may be empty.
    #[serde(default)]
    pub site: String,
    /// Hash of the previous record — reserved for a future append-only
    /// ledger.  Empty string = no chaining in this version.
    #[serde(default)]
    pub prev_record_hash: String,
}

/// One graded control question, flattened into the sealed envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedAnswer {
    /// 1-based position in the quiz.
    pub index: u32,
    /// What the question tested (`QuestionSource::label_kk`).
    pub kind: String,
    /// Whether this is a safety-critical question.
    pub critical: bool,
    /// Whether the worker's answer passed.
    pub passed: bool,
    /// Coverage as integer permille (`0..=1000`) — no float in signed bytes.
    pub coverage_permille: u32,
    /// The Kazakh prompt as read to the worker.
    pub prompt_kk: String,
    /// The worker's raw answer (trimmed).
    pub answer: String,
}

/// The full signed envelope — every field that the seal commits to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealEnvelope {
    pub format: String,
    pub alg: String,
    pub engine_version: String,
    pub procedure_id: String,
    pub procedure_title_kk: String,
    pub worker: String,
    pub operator: String,
    pub timestamp: String,
    pub timezone: String,
    pub site: String,
    pub prev_record_hash: String,
    pub answers: Vec<SealedAnswer>,
    pub passed_count: u32,
    pub total: u32,
    pub critical_failed: bool,
    pub admitted: bool,
    /// The engine's FNV content digest (`adam1-…`), carried for cross-check.
    pub content_digest: String,
}

impl SealEnvelope {
    /// Deterministic canonical bytes that the signature is computed over.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Compact JSON: struct field order is fixed and there are no
        // floats or maps, so this is stable and reproducible.
        serde_json::to_vec(self).expect("SealEnvelope serializes")
    }

    /// SHA-256 of the canonical bytes, hex-encoded.
    pub fn content_sha256(&self) -> String {
        to_hex(&sha256(&self.canonical_bytes()))
    }
}

/// The detached seal over a [`SealEnvelope`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seal {
    /// Signature algorithm tag (`adam-ed25519-sha256-v1`).
    pub alg: String,
    /// SHA-256 of the canonical envelope bytes (hex).
    pub content_sha256: String,
    /// Signer's Ed25519 public key (hex).
    pub public_key: String,
    /// Ed25519 signature over the canonical envelope bytes (hex).
    pub signature: String,
}

/// A sealed protocol ready to persist: envelope + its detached seal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedProtocol {
    pub envelope: SealEnvelope,
    pub seal: Seal,
}

/// Outcome of verifying a [`SealedProtocol`].
#[derive(Debug, Clone)]
pub struct SealVerification {
    /// Ed25519 signature verifies against the stated public key.
    pub signature_valid: bool,
    /// Stored `content_sha256` matches a freshly computed digest.
    pub digest_matches: bool,
    /// Algorithm tags are the ones this build understands.
    pub alg_known: bool,
}

impl SealVerification {
    /// A seal is trustworthy only if all checks pass.
    pub fn is_valid(&self) -> bool {
        self.signature_valid && self.digest_matches && self.alg_known
    }
}

impl BriefingProtocol {
    /// Build the canonical [`SealEnvelope`] for this protocol under the
    /// given caller context and engine version.
    pub fn to_envelope(&self, ctx: &SealContext, engine_version: &str) -> SealEnvelope {
        let answers = self
            .answers
            .iter()
            .enumerate()
            .map(|(i, a)| SealedAnswer {
                index: (i + 1) as u32,
                kind: a.source.label_kk().to_string(),
                critical: a.source.is_safety_critical(),
                passed: a.passed,
                coverage_permille: (a.coverage.clamp(0.0, 1.0) * 1000.0).round() as u32,
                prompt_kk: a.prompt_kk.clone(),
                answer: a.user_answer.trim().to_string(),
            })
            .collect();

        SealEnvelope {
            format: SEALED_FORMAT.to_string(),
            alg: SEAL_ALG.to_string(),
            engine_version: engine_version.to_string(),
            procedure_id: self.procedure_id.clone(),
            procedure_title_kk: self.title_kk.clone(),
            worker: ctx.worker.clone(),
            operator: ctx.operator.clone(),
            timestamp: ctx.timestamp.clone(),
            timezone: ctx.timezone.clone(),
            site: ctx.site.clone(),
            prev_record_hash: ctx.prev_record_hash.clone(),
            answers,
            passed_count: self.passed_count as u32,
            total: self.total as u32,
            critical_failed: self.critical_failed,
            admitted: self.admitted,
            content_digest: self.content_digest(),
        }
    }

    /// Seal this protocol with `key`, producing a persistable artifact.
    pub fn seal_with(
        &self,
        ctx: &SealContext,
        key: &SigningKey,
        engine_version: &str,
    ) -> SealedProtocol {
        let envelope = self.to_envelope(ctx, engine_version);
        let bytes = envelope.canonical_bytes();
        let signature = key.sign(&bytes);
        let seal = Seal {
            alg: SEAL_ALG.to_string(),
            content_sha256: to_hex(&sha256(&bytes)),
            public_key: key.public_key_hex(),
            signature: to_hex(&signature),
        };
        SealedProtocol { envelope, seal }
    }
}

impl SealedProtocol {
    /// Pretty JSON for storage / transmission.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("SealedProtocol serializes")
    }

    /// Parse a sealed protocol from JSON.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Verify the seal end-to-end: recompute the canonical digest, check
    /// it against the stored one, and verify the Ed25519 signature over
    /// the freshly re-serialized envelope.
    pub fn verify(&self) -> SealVerification {
        let bytes = self.envelope.canonical_bytes();
        let fresh_digest = to_hex(&sha256(&bytes));
        let digest_matches = fresh_digest == self.seal.content_sha256;
        let alg_known = self.seal.alg == SEAL_ALG && self.envelope.alg == SEAL_ALG;
        let signature_valid = verify_hex(&self.seal.public_key, &bytes, &self.seal.signature);
        SealVerification {
            signature_valid,
            digest_matches,
            alg_known,
        }
    }

    /// Signer's public key (hex).
    pub fn public_key(&self) -> &str {
        &self.seal.public_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::briefing_session::BriefingSession;

    // Drive a full session to a protocol using a fixed answer per turn.
    fn run_protocol(answers: &[&str]) -> BriefingProtocol {
        let mut s =
            BriefingSession::from_id("kk_metallurgy_loto_003").expect("loto procedure exists");
        let mut idx = 0;
        loop {
            let ans = answers.get(idx).copied().unwrap_or("түсінікті");
            let reply = s.advance(ans);
            idx += 1;
            if reply.done {
                break;
            }
        }
        s.protocol().expect("protocol produced")
    }

    fn ctx() -> SealContext {
        SealContext {
            worker: "Асан Асанов".into(),
            operator: "ИТҚ Досжан".into(),
            timestamp: "2026-07-04 14:32".into(),
            timezone: "UTC+05:00".into(),
            site: "SSGPO-LOTO-01".into(),
            prev_record_hash: String::new(),
        }
    }

    #[test]
    fn seal_then_verify_roundtrips() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([7u8; 32]);
        let sealed = p.seal_with(&ctx(), &key, "6.11.0");
        let v = sealed.verify();
        assert!(v.is_valid(), "freshly sealed protocol must verify: {v:?}");
        assert_eq!(sealed.public_key(), key.public_key_hex());
    }

    #[test]
    fn serialized_roundtrip_still_verifies() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([9u8; 32]);
        let sealed = p.seal_with(&ctx(), &key, "6.11.0");
        let json = sealed.to_json();
        let reparsed = SealedProtocol::from_json(&json).expect("parse back");
        assert!(
            reparsed.verify().is_valid(),
            "seal must survive JSON round-trip"
        );
    }

    #[test]
    fn tampering_with_a_verdict_breaks_the_seal() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([3u8; 32]);
        let mut sealed = p.seal_with(&ctx(), &key, "6.11.0");
        // Flip the admission verdict without re-signing.
        sealed.envelope.admitted = !sealed.envelope.admitted;
        let v = sealed.verify();
        assert!(
            !v.signature_valid,
            "signature must reject a flipped verdict"
        );
        assert!(!v.digest_matches, "digest must reject a flipped verdict");
    }

    #[test]
    fn tampering_with_an_answer_breaks_the_seal() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([5u8; 32]);
        let mut sealed = p.seal_with(&ctx(), &key, "6.11.0");
        if let Some(first) = sealed.envelope.answers.first_mut() {
            first.answer.push_str(" (өзгертілген)");
        }
        assert!(
            !sealed.verify().is_valid(),
            "editing an answer must break the seal"
        );
    }

    #[test]
    fn wrong_key_does_not_verify() {
        let p = run_protocol(&["түсінікті"; 40]);
        let real = SigningKey::from_seed([1u8; 32]);
        let mut sealed = p.seal_with(&ctx(), &real, "6.11.0");
        // Substitute an impostor public key.
        let impostor = SigningKey::from_seed([2u8; 32]);
        sealed.seal.public_key = impostor.public_key_hex();
        assert!(
            !sealed.verify().signature_valid,
            "impostor key must not verify"
        );
    }

    #[test]
    fn envelope_carries_engine_digest_and_permille() {
        let p = run_protocol(&["түсінікті"; 40]);
        let env = p.to_envelope(&ctx(), "6.11.0");
        assert_eq!(env.content_digest, p.content_digest());
        assert!(env.answers.iter().all(|a| a.coverage_permille <= 1000));
    }
}
