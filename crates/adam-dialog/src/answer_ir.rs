//! Typed answer IR + proof-carrying composer — v5.9.0 (G3.0, the
//! final milestone of the proof-carrying generation arc).
//!
//! Pre-G3.0 the dialog kernel produced answers via template
//! selection: the planner picked one of N hand-authored template
//! strings, the realiser substituted slots, the result was emitted.
//! Variation came from picking a different template string. The
//! `ProofObject` (G2.0) and `verifier::audit` gate (G2.5) sat
//! *alongside* this path, validating it but not driving it.
//!
//! G3.0 inverts the relationship: **the proof drives composition**.
//! A `compose(proof, shape, rng_seed)` function takes a
//! [`ProofObject`] + a target [`AnswerShape`] and produces an
//! [`AnswerIR`] tree. The realiser walks the tree to produce the
//! final string. The verifier audits the result against the same
//! proof object that was its source — closing the loop:
//!
//! ```text
//!   user intent
//!     → proof object (G2.0)
//!     → answer IR     (G3.0 composer — this module)
//!     → answer text   (G3.0 IR realiser — this module)
//!     → verifier::audit(text, proof) (G2.5 gate)
//!     → emit only if Verified
//! ```
//!
//! Codex's seven-step pipeline («facts + rules + constraints →
//! derived proof object → structured answer IR → FST/morphology
//! realiser → final answer → verifier») is now closed end-to-end
//! for the four shapes [`AnswerShape::YesNoConfirm`],
//! [`AnswerShape::YesNoDeny`], [`AnswerShape::YesNoUnknown`],
//! [`AnswerShape::SafetyRefusal`].
//!
//! ## Why this fits the project's identity
//!
//! Per `MISSION.md`: «казахская агглютинативная морфология
//! детерминистически компонует типизированные операторы». G3.0
//! lifts that compositional discipline from the morpheme level to
//! the discourse level: an [`AnswerNode`] tree is composed from
//! typed sub-claims with explicit support — every node traceable
//! to its proof.
//!
//! ## Scope at G3.0
//!
//! **Four shapes wired:** YesNoConfirm / YesNoDeny / YesNoUnknown /
//! SafetyRefusal. These are the shapes the kernel can already prove
//! today (per the v5.8.5 G2.5 wiring). Definition / Comparison /
//! StepByStep shapes will land as separate composer paths in the
//! v5.9.x patch series — G3.0 ships the typed substrate; the
//! per-shape composers grow incrementally on top.
//!
//! **Coexists with template path.** v5.9.0 does NOT replace
//! `Conversation::turn` or its template-driven render loop. The new
//! `Conversation::compose_answer` companion is opt-in for callers
//! that want the proof-carrying pipeline. Templates continue to
//! work; the composer is the alternative path for the four shapes
//! it covers.

use serde::{Deserialize, Serialize};

use crate::proof_object::{
    ClaimPredicate, HedgeMarker, Polarity, ProofObject, SafetyDomain, VerificationOutcome, verifier,
};

/// What kind of answer the composer is producing. Each shape has a
/// canonical IR template — the composer fills it in from the proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerShape {
    /// Confirming yes/no IsA: «Иә, X — Y. Дәлел тізбегі: chain.»
    YesNoConfirm,
    /// Denying yes/no IsA via antonym path: «Жоқ, X — Y емес. Себебі: chain.»
    YesNoDeny,
    /// Honest unknown: «Менің білім қорымда X — Y екеніне дерек жоқ.»
    YesNoUnknown,
    /// High-stakes safety refusal: medical / legal / financial /
    /// current-data.
    SafetyRefusal,
    /// **v5.10.5 — B4.1 of the Codex follow-up review arc.** Proof-
    /// chain mode requested by the user via «дәлелде X (Y)» / «X-тің
    /// Y екенін дәлелде» / «дәлелдеп бер». Distinct from
    /// `YesNoConfirm` (which is a *yes/no answer* whose IsA chain is
    /// supporting evidence) — `IsAProofChain` is a *proof
    /// performance*: «Дәлелдейік: A → B → C → D. Сондықтан A — D.»
    /// The conclusion is the same shape (IsA + Affirmative + non-
    /// empty chain), but the surface emphasises the chain itself
    /// rather than wrapping a yes/no verdict around it. Per the v5.9.0
    /// G3.0 substrate, no new proof-object structure is required —
    /// `from_isa_chain` already carries the derivation trace.
    IsAProofChain,
}

/// One node in the answer-IR tree. Composition is recursive — every
/// `AnswerNode::Sequence` can hold further nodes — so multi-clause
/// answers compose naturally from typed sub-units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "node")]
pub enum AnswerNode {
    /// Confirming verdict surface form: «Иә,» / «Дұрыс,» (rng-picked).
    ConfirmVerdict { surface: String },
    /// Denying verdict: «Жоқ,» / «Терісі рас:».
    DenyVerdict { surface: String },
    /// Subject of an IsA claim, capitalised for sentence-initial use.
    Subject { surface: String },
    /// Predicate of an IsA claim (or the asked-about property).
    Predicate { surface: String },
    /// Particle / connector punctuation between clauses («— », «.»,
    /// « мен »).
    Punctuation { surface: String },
    /// Citation of the derivation chain: «Дәлел тізбегі: жер
    /// сілкінісі → табиғи апат».
    ChainCitation { rendered_chain: String },
    /// Honest hedge surface — pulled from the proof's
    /// [`HedgeMarker`] list with a per-marker template.
    Hedge {
        marker: HedgeMarker,
        surface: String,
    },
    /// Refusal body for safety / no-data / system-self answers. The
    /// body is curated text bound to the proof's domain or kind.
    RefusalBody { surface: String },
    /// **v5.10.5.** Proof-prologue used by `IsAProofChain`: opens the
    /// performance with «Дәлелдейік:» / «Қарап шығайық:» (rng-picked).
    ProofPrologue { surface: String },
    /// **v5.10.5.** Stepwise rendering of the IsA chain — «A → B → C
    /// → D» — distinct from [`AnswerNode::ChainCitation`] which
    /// embeds chain text in a parenthetical. The proof composer for
    /// `IsAProofChain` uses this node to make the chain the
    /// structural focus of the answer.
    ProofChainSteps { rendered_steps: String },
    /// **v5.10.5.** Concluding statement bound to the chain — «Сондықтан
    /// X — Y.» — distinct from a bare `Predicate` node so the realiser
    /// can emit it on its own clause.
    ProofConclusion { surface: String },
    /// Concatenation of child nodes. The renderer walks them
    /// left-to-right and joins their surface forms with single
    /// spaces, then lets the post-processor fix punctuation
    /// adjacency.
    Sequence { nodes: Vec<AnswerNode> },
}

/// Top-level answer-IR. The composer produces one of these per
/// turn; the realiser walks it to produce the final string. Every
/// IR is bound to its source [`ProofObject`] so the verifier can
/// audit the realised text against the same proof that drove
/// composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerIR {
    pub shape: AnswerShape,
    pub root: AnswerNode,
    pub source_proof: ProofObject,
}

/// Compose a typed answer IR from a proof object + shape.
///
/// **Determinism.** `rng_seed` selects between equivalent surface
/// forms (e.g. «Иә,» vs «Дұрыс,» for `ConfirmVerdict`); the same
/// seed + same proof → byte-identical IR.
///
/// **Shape mismatches** (proof claims `SafetyRefusal` but caller
/// asks for `YesNoConfirm`) are not silently coerced — the function
/// returns `None`. Caller must select a shape consistent with the
/// proof's predicate.
pub fn compose(proof: &ProofObject, shape: AnswerShape, rng_seed: u64) -> Option<AnswerIR> {
    if !proof_matches_shape(proof, shape) {
        return None;
    }
    let root = match shape {
        AnswerShape::YesNoConfirm => compose_yes_no_confirm(proof, rng_seed),
        AnswerShape::YesNoDeny => compose_yes_no_deny(proof, rng_seed),
        AnswerShape::YesNoUnknown => compose_yes_no_unknown(proof, rng_seed),
        AnswerShape::SafetyRefusal => compose_safety_refusal(proof, rng_seed),
        AnswerShape::IsAProofChain => compose_isa_proof_chain(proof, rng_seed)?,
    };
    Some(AnswerIR {
        shape,
        root,
        source_proof: proof.clone(),
    })
}

fn proof_matches_shape(proof: &ProofObject, shape: AnswerShape) -> bool {
    match shape {
        AnswerShape::YesNoConfirm => {
            matches!(
                proof.conclusion.predicate,
                ClaimPredicate::IsA | ClaimPredicate::HasProperty
            ) && proof.conclusion.polarity == Polarity::Affirmative
                && proof.conclusion.predicate != ClaimPredicate::NoData
        }
        AnswerShape::YesNoDeny => proof.conclusion.polarity == Polarity::Negated,
        AnswerShape::YesNoUnknown => {
            matches!(proof.conclusion.predicate, ClaimPredicate::NoData)
        }
        AnswerShape::SafetyRefusal => {
            matches!(
                proof.conclusion.predicate,
                ClaimPredicate::SafetyRefusal { .. }
            )
        }
        // **v5.10.5.** Proof-chain mode requires affirmative IsA AND
        // a non-empty derivation chain — without the chain there is
        // nothing to perform. `compose_isa_proof_chain` itself returns
        // `None` if the chain is empty (which `proof_matches_shape`
        // can't see at this layer); the caller selects this shape only
        // when the user explicitly asked for a proof, so the empty
        // chain case is the user's signal that the kernel can't
        // perform one — handled by routing back through standard
        // template path.
        AnswerShape::IsAProofChain => {
            matches!(proof.conclusion.predicate, ClaimPredicate::IsA)
                && proof.conclusion.polarity == Polarity::Affirmative
        }
    }
}

fn compose_yes_no_confirm(proof: &ProofObject, rng_seed: u64) -> AnswerNode {
    let subject = capitalise_first(&proof.conclusion.subject);
    let predicate = proof.conclusion.object.clone();
    let verdict_surface = pick_confirm_verdict(rng_seed);
    let chain_text = render_chain(proof);

    let mut nodes = vec![
        AnswerNode::ConfirmVerdict {
            surface: verdict_surface,
        },
        AnswerNode::Subject { surface: subject },
        AnswerNode::Punctuation {
            surface: " — ".to_string(),
        },
        AnswerNode::Predicate { surface: predicate },
        AnswerNode::Punctuation {
            surface: ".".to_string(),
        },
    ];
    if let Some(chain) = chain_text {
        nodes.push(AnswerNode::Punctuation {
            surface: " ".to_string(),
        });
        nodes.push(AnswerNode::ChainCitation {
            rendered_chain: format!("Дәлел тізбегі: {chain}."),
        });
    }
    AnswerNode::Sequence { nodes }
}

fn compose_yes_no_deny(proof: &ProofObject, _rng_seed: u64) -> AnswerNode {
    let subject = capitalise_first(&proof.conclusion.subject);
    let predicate = proof.conclusion.object.clone();
    let chain_text = render_chain(proof).unwrap_or_default();

    let mut nodes = vec![
        AnswerNode::DenyVerdict {
            surface: "Жоқ,".to_string(),
        },
        AnswerNode::Punctuation {
            surface: " ".to_string(),
        },
        AnswerNode::Subject { surface: subject },
        AnswerNode::Punctuation {
            surface: " — ".to_string(),
        },
        AnswerNode::Predicate { surface: predicate },
        AnswerNode::Punctuation {
            surface: " емес.".to_string(),
        },
    ];
    if !chain_text.is_empty() {
        nodes.push(AnswerNode::Punctuation {
            surface: " ".to_string(),
        });
        nodes.push(AnswerNode::ChainCitation {
            rendered_chain: format!("Себебі: {chain_text}."),
        });
    }
    AnswerNode::Sequence { nodes }
}

fn compose_yes_no_unknown(proof: &ProofObject, _rng_seed: u64) -> AnswerNode {
    let subject = capitalise_first(&proof.conclusion.subject);
    let predicate = proof.conclusion.object.clone();
    AnswerNode::Sequence {
        nodes: vec![
            AnswerNode::RefusalBody {
                surface: format!("Менің білім қорымда {subject} — {predicate} екеніне дерек жоқ."),
            },
            AnswerNode::Punctuation {
                surface: " ".to_string(),
            },
            AnswerNode::Hedge {
                marker: HedgeMarker::NoData,
                surface: "Сұрағыңызды басқаша қойсаңыз болады.".to_string(),
            },
        ],
    }
}

fn compose_safety_refusal(proof: &ProofObject, _rng_seed: u64) -> AnswerNode {
    let domain = match proof.conclusion.predicate {
        ClaimPredicate::SafetyRefusal { domain } => domain,
        _ => unreachable!(
            "proof_matches_shape gated this branch; SafetyRefusal predicate guaranteed"
        ),
    };
    let body = canonical_safety_body(domain);
    AnswerNode::Sequence {
        nodes: vec![
            AnswerNode::RefusalBody {
                surface: body.to_string(),
            },
            AnswerNode::Punctuation {
                surface: " ".to_string(),
            },
            AnswerNode::Hedge {
                marker: HedgeMarker::SafetyRefusal,
                surface: hedge_surface_for_domain(domain).to_string(),
            },
        ],
    }
}

/// **G3.0 realiser.** Walk the IR depth-first and concatenate
/// surface forms. Performs minimal post-processing: collapses
/// repeated whitespace and trims trailing space before final
/// punctuation. The result is what the verifier audits against the
/// source proof.
pub fn realise_answer_ir(ir: &AnswerIR) -> String {
    let mut out = String::new();
    walk(&ir.root, &mut out);
    normalise_whitespace(&out)
}

fn walk(node: &AnswerNode, out: &mut String) {
    match node {
        AnswerNode::ConfirmVerdict { surface }
        | AnswerNode::DenyVerdict { surface }
        | AnswerNode::Subject { surface }
        | AnswerNode::Predicate { surface }
        | AnswerNode::Punctuation { surface }
        | AnswerNode::RefusalBody { surface }
        | AnswerNode::ProofPrologue { surface }
        | AnswerNode::ProofConclusion { surface } => out.push_str(surface),
        AnswerNode::ChainCitation { rendered_chain } => out.push_str(rendered_chain),
        AnswerNode::ProofChainSteps { rendered_steps } => out.push_str(rendered_steps),
        AnswerNode::Hedge { surface, .. } => out.push_str(surface),
        AnswerNode::Sequence { nodes } => {
            for child in nodes {
                walk(child, out);
            }
        }
    }
}

fn normalise_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// **G3.0 end-to-end gate.** Realise the IR + audit the result
/// against the source proof. Returns `Ok(text)` when the verifier
/// confirms; `Err(outcome)` otherwise. This is the proof-carrying
/// emit gate: callers may release the answer to the user only on
/// `Ok`. Failures land back in the planner / template path.
pub fn realise_and_verify(ir: &AnswerIR) -> Result<String, VerificationOutcome> {
    let text = realise_answer_ir(ir);
    match verifier::audit(&text, &ir.source_proof) {
        VerificationOutcome::Verified => Ok(text),
        other => Err(other),
    }
}

fn render_chain(proof: &ProofObject) -> Option<String> {
    proof.derivation.as_ref().and_then(|trace| {
        if trace.chain_path.is_empty() {
            None
        } else {
            Some(trace.chain_path.join(" → "))
        }
    })
}

fn pick_confirm_verdict(rng_seed: u64) -> String {
    const VERDICTS: &[&str] = &["Иә,", "Дұрыс,"];
    VERDICTS[(rng_seed as usize) % VERDICTS.len()].to_string()
}

fn pick_proof_prologue(rng_seed: u64) -> String {
    const PROLOGUES: &[&str] = &["Дәлелдейік:", "Қарап шығайық:", "Тізбек бойынша:"];
    PROLOGUES[(rng_seed as usize) % PROLOGUES.len()].to_string()
}

/// **v5.10.5 — B4.1 composer.** Build a proof-performance IR over an
/// IsA chain. Pre-conditions enforced by `proof_matches_shape` +
/// non-empty chain check here. Surface shape:
///
/// > Дәлелдейік: <subject> — <terminal>. Тізбек: A → B → C → D.
/// > Сондықтан <subject> — <terminal>.
///
/// Returns `None` when the proof has no derivation chain — the
/// caller (Conversation::turn) falls back to the regular YesNoConfirm
/// path or the template family in that case.
fn compose_isa_proof_chain(proof: &ProofObject, rng_seed: u64) -> Option<AnswerNode> {
    let chain_text = render_chain(proof)?;
    let subject = capitalise_first(&proof.conclusion.subject);
    let predicate = proof.conclusion.object.clone();
    let prologue = pick_proof_prologue(rng_seed);
    let nodes = vec![
        AnswerNode::ProofPrologue { surface: prologue },
        AnswerNode::Punctuation {
            surface: " ".to_string(),
        },
        AnswerNode::Subject {
            surface: subject.clone(),
        },
        AnswerNode::Punctuation {
            surface: " — ".to_string(),
        },
        AnswerNode::Predicate {
            surface: predicate.clone(),
        },
        AnswerNode::Punctuation {
            surface: ". ".to_string(),
        },
        AnswerNode::ProofChainSteps {
            rendered_steps: format!("Тізбек: {chain_text}."),
        },
        AnswerNode::Punctuation {
            surface: " ".to_string(),
        },
        AnswerNode::ProofConclusion {
            surface: format!("Сондықтан {subject} — {predicate}."),
        },
    ];
    Some(AnswerNode::Sequence { nodes })
}

fn capitalise_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn canonical_safety_body(domain: SafetyDomain) -> &'static str {
    match domain {
        SafetyDomain::Medical => {
            "Денсаулыққа қатысты нақты кеңес бере алмаймын — мен дәрігер емеспін."
        }
        SafetyDomain::Legal => "Заңдық кеңес бере алмаймын — мен заңгер емеспін.",
        SafetyDomain::Financial => {
            "Қаржылық кеңес бере алмаймын — нарық деректерім жоқ және кәсіби кеңесші емеспін."
        }
        SafetyDomain::CurrentData => {
            "Қазіргі немесе бүгінгі мәліметтерге кіруім жоқ — менде уақытқа байланысты өзгеретін деректер сақталмайды."
        }
        SafetyDomain::Political => {
            "Саяси кеңес немесе партиялық таңдау бойынша ұсыныс бере алмаймын."
        }
    }
}

fn hedge_surface_for_domain(domain: SafetyDomain) -> &'static str {
    match domain {
        SafetyDomain::Medical => "Терапевке немесе білікті дәрігерге жүгінуіңізді ұсынамын.",
        SafetyDomain::Legal => "Білікті заңгерге жүгінуіңізді ұсынамын.",
        SafetyDomain::Financial => {
            "Білікті қаржылық кеңесшіге немесе банк менеджеріне жүгінуіңізді ұсынамын."
        }
        SafetyDomain::CurrentData => "Ресми дереккөзден тексеруіңізді ұсынамын.",
        SafetyDomain::Political => "Партиялық жүйе туралы фактілер қажет болса — сұраңыз.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_object::SafetyDomain;

    #[test]
    fn compose_yes_no_confirm_produces_iaa_ir_v590() {
        let proof = ProofObject::from_isa_chain(
            "қасқыр".into(),
            "тірі".into(),
            vec!["қасқыр".into(), "тірі".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        let ir = compose(&proof, AnswerShape::YesNoConfirm, 0).expect("composer fires");
        assert_eq!(ir.shape, AnswerShape::YesNoConfirm);
        let text = realise_answer_ir(&ir);
        assert!(text.contains("Иә,"), "verdict surface present: {text}");
        assert!(text.contains("Қасқыр"), "subject capitalised: {text}");
        assert!(text.contains("тірі"), "predicate present: {text}");
        assert!(text.contains("Дәлел тізбегі"), "chain cited: {text}");
    }

    #[test]
    fn compose_yes_no_deny_carries_negation_v590() {
        let proof = ProofObject::from_antonym_denial(
            "тас".into(),
            "тірі".into(),
            vec!["тас".into(), "жансыз".into()],
        );
        let ir = compose(&proof, AnswerShape::YesNoDeny, 0).expect("composer fires");
        let text = realise_answer_ir(&ir);
        assert!(text.starts_with("Жоқ,"), "starts with denial: {text}");
        assert!(text.contains("емес"), "negation surface present: {text}");
        assert!(text.contains("Себебі"), "antonym chain cited: {text}");
    }

    #[test]
    fn compose_yes_no_unknown_is_honest_v590() {
        let proof = ProofObject::no_data_refusal("кітап".into(), "тағам".into());
        let ir = compose(&proof, AnswerShape::YesNoUnknown, 0).expect("composer fires");
        let text = realise_answer_ir(&ir);
        assert!(
            text.contains("білім қорымда"),
            "honest hedge present: {text}"
        );
        assert!(text.contains("Кітап"), "subject in answer: {text}");
        assert!(text.contains("тағам"), "predicate in answer: {text}");
    }

    #[test]
    fn compose_isa_proof_chain_renders_steps_v5105() {
        let proof = ProofObject::from_isa_chain(
            "қасқыр".into(),
            "тірі".into(),
            vec![
                "қасқыр".into(),
                "жыртқыш".into(),
                "жануар".into(),
                "тірі".into(),
            ],
            vec!["R1_is_a_transitivity".into()],
        );
        let ir = compose(&proof, AnswerShape::IsAProofChain, 0).expect("composer fires");
        assert_eq!(ir.shape, AnswerShape::IsAProofChain);
        let text = realise_answer_ir(&ir);
        // Prologue + chain + conclusion all present.
        assert!(
            text.contains("Дәлелдейік")
                || text.contains("Қарап шығайық")
                || text.contains("Тізбек бойынша"),
            "prologue surface: {text}"
        );
        assert!(
            text.contains("қасқыр → жыртқыш → жануар → тірі"),
            "chain stepwise: {text}"
        );
        assert!(text.contains("Сондықтан"), "conclusion surface: {text}");
    }

    #[test]
    fn compose_isa_proof_chain_returns_none_on_empty_chain_v5105() {
        // Direct fact — chain has only the subject (single-element
        // chain). The proof matches IsA + Affirmative shape gate but
        // the composer enforces non-empty derivation as a content
        // check; an empty chain has no proof to perform.
        let proof = ProofObject::from_curated_fact(
            "ит".into(),
            "is_a".into(),
            "жануар".into(),
            adam_reasoning::FactSource {
                pack: "world_core".into(),
                sample_id: "wc_anm_001".into(),
            },
            "Ит — жануар.".into(),
        );
        // `from_curated_fact` produces a proof with no derivation
        // chain — composer must return None.
        let ir = compose(&proof, AnswerShape::IsAProofChain, 0);
        assert!(ir.is_none(), "expected None on no-derivation proof");
    }

    #[test]
    fn compose_safety_refusal_routes_by_domain_v590() {
        let proof =
            ProofObject::safety_refusal("user".into(), "medical".into(), SafetyDomain::Medical);
        let ir = compose(&proof, AnswerShape::SafetyRefusal, 0).expect("composer fires");
        let text = realise_answer_ir(&ir);
        assert!(text.contains("дәрігер емеспін"), "medical body: {text}");
        assert!(text.contains("Терапевке"), "medical hedge: {text}");
    }

    #[test]
    fn compose_rejects_shape_mismatch_v590() {
        let proof = ProofObject::no_data_refusal("X".into(), "Y".into());
        // No-data proof can't drive a Confirm shape — return None.
        assert!(compose(&proof, AnswerShape::YesNoConfirm, 0).is_none());
    }

    #[test]
    fn compose_is_deterministic_per_seed_v590() {
        let proof = ProofObject::from_isa_chain(
            "ит".into(),
            "жануар".into(),
            vec!["ит".into(), "жануар".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        let a = compose(&proof, AnswerShape::YesNoConfirm, 0).unwrap();
        let b = compose(&proof, AnswerShape::YesNoConfirm, 0).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn compose_picks_different_verdict_per_seed_v590() {
        let proof = ProofObject::from_isa_chain(
            "ит".into(),
            "жануар".into(),
            vec!["ит".into(), "жануар".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        let s0 = realise_answer_ir(&compose(&proof, AnswerShape::YesNoConfirm, 0).unwrap());
        let s1 = realise_answer_ir(&compose(&proof, AnswerShape::YesNoConfirm, 1).unwrap());
        // With two verdict surfaces in pool, seeds 0 and 1 must
        // pick different ones.
        assert_ne!(s0, s1, "seed-driven variation: {s0} vs {s1}");
    }

    #[test]
    fn realise_and_verify_returns_ok_for_well_formed_ir_v590() {
        let proof = ProofObject::from_isa_chain(
            "қасқыр".into(),
            "тірі".into(),
            vec!["қасқыр".into(), "тірі".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        let ir = compose(&proof, AnswerShape::YesNoConfirm, 0).unwrap();
        let result = realise_and_verify(&ir);
        assert!(result.is_ok(), "well-formed IR must verify: {result:?}");
    }

    #[test]
    fn ir_round_trips_through_serde_v590() {
        let proof = ProofObject::from_isa_chain(
            "ит".into(),
            "жануар".into(),
            vec!["ит".into(), "жануар".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        let ir = compose(&proof, AnswerShape::YesNoConfirm, 0).unwrap();
        let json = serde_json::to_string(&ir).unwrap();
        let back: AnswerIR = serde_json::from_str(&json).unwrap();
        assert_eq!(ir, back);
    }

    #[test]
    fn proof_polarity_disagrees_with_shape_rejected_v590() {
        // Affirmative proof asked for in Deny shape: rejected.
        let proof = ProofObject::from_isa_chain(
            "ит".into(),
            "жануар".into(),
            vec!["ит".into(), "жануар".into()],
            vec!["R1_is_a_transitivity".into()],
        );
        assert!(compose(&proof, AnswerShape::YesNoDeny, 0).is_none());
    }
}
