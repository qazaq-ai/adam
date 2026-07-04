//! Protocol Seal — turn a graded [`BriefingProtocol`] into a signed,
//! publicly verifiable **work-admission credential** (допуск).
//!
//! The deterministic engine produces the *content* of the protocol; this
//! module wraps that content, plus the caller-side context the engine
//! cannot own (who was briefed, who signed off, by what authority, when,
//! where, on which SOP version), into a **canonical envelope** and seals
//! it with an Ed25519 signature ([`adam_seal`]).  A service inspector or
//! a court expert can then verify the seal with any independent Ed25519
//! implementation — the artifact does not require trusting our program.
//!
//! ## Identity & authority chain (v6.12.0)
//!
//! A signature over the *content* is not enough for a legally load-bearing
//! допуск: an accident review asks *who answered*, *who admitted them and
//! by what authority*, and *which version of the SOP* was briefed.  The
//! envelope therefore binds:
//! - **subject** — the worker (name + id reference + how identity was
//!   established), the `credentialSubject`;
//! - **issuer** — the operator/ИТҚ who admits, their authority `role`, an
//!   explicit `authorityAssertion`, and their public key.  Verification
//!   requires `issuer.publicKey == seal.publicKey`, so the authority
//!   assertion is provably signed by the operator's own key;
//! - **procedure** — the SOP `id`, `sopHash` (content hash) and version
//!   date, proving which procedure version the worker was tested on.
//!
//! ## Canonical form (W3C-VC-shaped)
//!
//! The vocabulary mirrors W3C Verifiable Credentials (`issuer`,
//! `credentialSubject`, `evidence`, `credentialStatus`, `type`, schema
//! version) so the format can converge on that standard later without a
//! breaking change — but this is a self-contained canonical-JSON + Ed25519
//! credential, **not** a JSON-LD VC.  The signed bytes are the compact
//! JSON of [`SealEnvelope`]; field order is fixed by the struct and every
//! quantity is an integer or string (coverage is `coveragePermille`, never
//! a float), so serialization is deterministic.  Verification re-serializes
//! the parsed envelope, so reformatting the stored file never breaks a
//! valid seal; editing any field does.
//!
//! ## Deliberately out of scope (until a deployment asks)
//!
//! `credentialStatus.status` and `prevRecordHash` are reserved for a
//! future revocation/append-only ledger, but this version issues only
//! `active` credentials and builds no ledger.  Full key management —
//! rotation, revocation lists, operator-role registries, HSM custody — is
//! the next layer after the first pilot, not this release.

use adam_seal::{SEAL_ALG, SigningKey, sha256, to_hex, verify_hex};
use serde::{Deserialize, Serialize};

use crate::briefing_session::BriefingProtocol;

/// Schema tag stored in every sealed credential.
pub const SEALED_FORMAT: &str = "adam-dopusk-credential/2";

/// Credential `type` array (W3C-VC-shaped).
pub const CREDENTIAL_TYPE: [&str; 2] = ["VerifiableCredential", "WorkAdmissionCredential"];

/// Default Kazakh authority assertion an operator signs when no explicit
/// statement is supplied.
pub const DEFAULT_AUTHORITY_ASSERTION: &str =
    "Мен жұмысшының жеке басын растадым және оны жұмысқа жіберуге өкілеттімін.";

/// Default method by which the worker's identity was established.
pub const DEFAULT_ID_METHOD: &str = "operator-confirmed";

/// Default operator authority role.
pub const DEFAULT_OPERATOR_ROLE: &str = "ИТҚ";

/// Caller-supplied context the deterministic engine does not own —
/// identity, authority, time, place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealContext {
    /// Briefed worker's name.
    pub worker: String,
    /// Worker identity reference (badge / personnel id); may be empty.
    pub worker_id: String,
    /// How the worker's identity was established
    /// (`operator-confirmed` / `badge` / `biometric` — the last reserved).
    pub worker_id_method: String,
    /// Operator / ИТҚ who conducted the briefing and admits the worker.
    pub operator: String,
    /// Operator's authority role (e.g. `ИТҚ`, `начальник участка`).
    pub operator_role: String,
    /// The statement the operator signs, asserting authority + identity
    /// confirmation.
    pub authority_assertion: String,
    /// Local wall-clock timestamp, e.g. `2026-07-04 14:32`.
    pub timestamp: String,
    /// Timezone label for `timestamp`, e.g. `UTC+05:00`.
    pub timezone: String,
    /// Site / device identifier; may be empty.
    pub site: String,
    /// Hash of the previous record — reserved for a future append-only
    /// ledger.  Empty string = no chaining in this version.
    pub prev_record_hash: String,
}

impl Default for SealContext {
    fn default() -> Self {
        SealContext {
            worker: String::new(),
            worker_id: String::new(),
            worker_id_method: DEFAULT_ID_METHOD.to_string(),
            operator: String::new(),
            operator_role: DEFAULT_OPERATOR_ROLE.to_string(),
            authority_assertion: DEFAULT_AUTHORITY_ASSERTION.to_string(),
            timestamp: String::new(),
            timezone: String::new(),
            site: String::new(),
            prev_record_hash: String::new(),
        }
    }
}

/// The issuer of the credential — the operator/ИТҚ who admits the worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issuer {
    pub name: String,
    pub role: String,
    /// The operator's Ed25519 public key.  Verification requires this to
    /// equal the seal's public key, binding the authority assertion to
    /// the signer.
    pub public_key: String,
    pub authority_assertion: String,
}

/// The credential subject — the briefed worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSubject {
    pub name: String,
    pub id_ref: String,
    pub id_method: String,
}

/// The SOP the worker was briefed and tested on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcedureRef {
    pub id: String,
    pub title_kk: String,
    /// `sha256:<hex>` content hash — proves which SOP *version* ran.
    pub sop_hash: String,
    pub sop_version_date: String,
}

/// One graded control question, flattened into the credential evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// The graded-briefing evidence backing the admission decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub answers: Vec<SealedAnswer>,
    pub passed_count: u32,
    pub total: u32,
    pub critical_failed: bool,
    /// The engine's FNV content digest (`adam1-…`), carried for cross-check.
    pub content_digest: String,
}

/// Revocation / ledger status — reserved; every credential is `active`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub status: String,
    pub prev_record_hash: String,
}

/// The full signed envelope — every field the seal commits to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealEnvelope {
    pub schema: String,
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    pub alg: String,
    pub engine_version: String,
    pub issuance_date: String,
    pub timezone: String,
    pub site: String,
    pub issuer: Issuer,
    pub credential_subject: CredentialSubject,
    pub procedure: ProcedureRef,
    pub evidence: Evidence,
    pub admitted: bool,
    pub credential_status: CredentialStatus,
}

impl SealEnvelope {
    /// Deterministic canonical bytes the signature is computed over.
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
#[serde(rename_all = "camelCase")]
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

/// A sealed credential ready to persist: envelope + its detached seal.
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
    /// Schema + algorithm tags are the ones this build understands.
    pub alg_known: bool,
    /// The issuer's public key equals the signing key — the authority
    /// assertion is signed by the operator's own key.
    pub issuer_bound: bool,
}

impl SealVerification {
    /// A seal is trustworthy only if all checks pass.
    pub fn is_valid(&self) -> bool {
        self.signature_valid && self.digest_matches && self.alg_known && self.issuer_bound
    }
}

impl BriefingProtocol {
    /// Build the canonical [`SealEnvelope`] for this protocol under the
    /// given caller context and engine version.
    ///
    /// `issuer.publicKey` is left empty here and filled by
    /// [`Self::seal_with`], which knows the signing key — the two must
    /// match for verification to pass.
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
            schema: SEALED_FORMAT.to_string(),
            credential_type: CREDENTIAL_TYPE.iter().map(|s| s.to_string()).collect(),
            alg: SEAL_ALG.to_string(),
            engine_version: engine_version.to_string(),
            issuance_date: ctx.timestamp.clone(),
            timezone: ctx.timezone.clone(),
            site: ctx.site.clone(),
            issuer: Issuer {
                name: ctx.operator.clone(),
                role: ctx.operator_role.clone(),
                public_key: String::new(), // filled by seal_with
                authority_assertion: ctx.authority_assertion.clone(),
            },
            credential_subject: CredentialSubject {
                name: ctx.worker.clone(),
                id_ref: ctx.worker_id.clone(),
                id_method: ctx.worker_id_method.clone(),
            },
            procedure: ProcedureRef {
                id: self.procedure_id.clone(),
                title_kk: self.title_kk.clone(),
                sop_hash: self.sop_hash.clone(),
                sop_version_date: self.sop_version_date.clone(),
            },
            evidence: Evidence {
                answers,
                passed_count: self.passed_count as u32,
                total: self.total as u32,
                critical_failed: self.critical_failed,
                content_digest: self.content_digest(),
            },
            admitted: self.admitted,
            credential_status: CredentialStatus {
                status: "active".to_string(),
                prev_record_hash: ctx.prev_record_hash.clone(),
            },
        }
    }

    /// Seal this protocol with `key`, producing a persistable credential.
    /// The signer's public key is bound into `issuer.publicKey` before
    /// signing, so the operator's authority assertion is provably theirs.
    pub fn seal_with(
        &self,
        ctx: &SealContext,
        key: &SigningKey,
        engine_version: &str,
    ) -> SealedProtocol {
        let mut envelope = self.to_envelope(ctx, engine_version);
        envelope.issuer.public_key = key.public_key_hex();
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

    /// Parse a sealed credential from JSON.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Verify the seal end-to-end: recompute the canonical digest, check
    /// it against the stored one, verify the Ed25519 signature over the
    /// re-serialized envelope, and confirm the issuer key is the signer.
    pub fn verify(&self) -> SealVerification {
        let bytes = self.envelope.canonical_bytes();
        let fresh_digest = to_hex(&sha256(&bytes));
        let digest_matches = fresh_digest == self.seal.content_sha256;
        let alg_known = self.seal.alg == SEAL_ALG
            && self.envelope.alg == SEAL_ALG
            && self.envelope.schema == SEALED_FORMAT;
        let signature_valid = verify_hex(&self.seal.public_key, &bytes, &self.seal.signature);
        let issuer_bound = !self.seal.public_key.is_empty()
            && self.envelope.issuer.public_key == self.seal.public_key;
        SealVerification {
            signature_valid,
            digest_matches,
            alg_known,
            issuer_bound,
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
            worker_id: "SSGPO-EMP-12345".into(),
            operator: "Досжан Оператор".into(),
            timestamp: "2026-07-04 14:32".into(),
            timezone: "UTC+05:00".into(),
            site: "SSGPO-LOTO-01".into(),
            ..SealContext::default()
        }
    }

    #[test]
    fn seal_then_verify_roundtrips() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([7u8; 32]);
        let sealed = p.seal_with(&ctx(), &key, "6.12.0");
        let v = sealed.verify();
        assert!(v.is_valid(), "freshly sealed protocol must verify: {v:?}");
        assert_eq!(sealed.public_key(), key.public_key_hex());
    }

    #[test]
    fn serialized_roundtrip_still_verifies() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([9u8; 32]);
        let sealed = p.seal_with(&ctx(), &key, "6.12.0");
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
        let mut sealed = p.seal_with(&ctx(), &key, "6.12.0");
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
        let mut sealed = p.seal_with(&ctx(), &key, "6.12.0");
        if let Some(first) = sealed.envelope.evidence.answers.first_mut() {
            first.answer.push_str(" (өзгертілген)");
        }
        assert!(
            !sealed.verify().is_valid(),
            "editing an answer must break the seal"
        );
    }

    #[test]
    fn tampering_with_the_authority_assertion_breaks_the_seal() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([11u8; 32]);
        let mut sealed = p.seal_with(&ctx(), &key, "6.12.0");
        sealed.envelope.issuer.authority_assertion = "Басқа мәлімдеме".into();
        assert!(
            !sealed.verify().signature_valid,
            "editing the operator's authority assertion must break the signature"
        );
    }

    #[test]
    fn tampering_with_the_sop_hash_breaks_the_seal() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([13u8; 32]);
        let mut sealed = p.seal_with(&ctx(), &key, "6.12.0");
        sealed.envelope.procedure.sop_hash = "sha256:deadbeef".into();
        assert!(
            !sealed.verify().is_valid(),
            "swapping the SOP version hash must break the seal"
        );
    }

    #[test]
    fn substituted_issuer_key_is_not_bound() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([1u8; 32]);
        let mut sealed = p.seal_with(&ctx(), &key, "6.12.0");
        // Point the issuer at a different key than the one that signed.
        let other = SigningKey::from_seed([2u8; 32]);
        sealed.envelope.issuer.public_key = other.public_key_hex();
        let v = sealed.verify();
        assert!(!v.issuer_bound, "issuer key must match the signer");
        assert!(!v.is_valid(), "an unbound issuer must fail verification");
    }

    #[test]
    fn wrong_signer_key_does_not_verify() {
        let p = run_protocol(&["түсінікті"; 40]);
        let real = SigningKey::from_seed([1u8; 32]);
        let mut sealed = p.seal_with(&ctx(), &real, "6.12.0");
        let impostor = SigningKey::from_seed([2u8; 32]);
        sealed.seal.public_key = impostor.public_key_hex();
        assert!(
            !sealed.verify().signature_valid,
            "impostor key must not verify"
        );
    }

    #[test]
    fn envelope_carries_identity_authority_and_sop() {
        let p = run_protocol(&["түсінікті"; 40]);
        let key = SigningKey::from_seed([21u8; 32]);
        let sealed = p.seal_with(&ctx(), &key, "6.12.0");
        let e = &sealed.envelope;
        assert_eq!(e.schema, SEALED_FORMAT);
        assert_eq!(e.credential_subject.name, "Асан Асанов");
        assert_eq!(e.credential_subject.id_ref, "SSGPO-EMP-12345");
        assert_eq!(e.credential_subject.id_method, DEFAULT_ID_METHOD);
        assert_eq!(e.issuer.role, DEFAULT_OPERATOR_ROLE);
        assert_eq!(e.issuer.authority_assertion, DEFAULT_AUTHORITY_ASSERTION);
        assert_eq!(e.issuer.public_key, key.public_key_hex());
        assert!(e.procedure.sop_hash.starts_with("sha256:"));
        assert_eq!(e.evidence.content_digest, p.content_digest());
        assert!(
            e.evidence
                .answers
                .iter()
                .all(|a| a.coverage_permille <= 1000)
        );
    }
}
