//! adam-dialog — predictable, auditable Kazakh dialog layer.
//!
//! See `docs/kazakh_grammar/07_dialog_architecture.md` for the architectural
//! commitment. Five-layer pipeline:
//!
//! 1. Morphological parser (`adam_kernel_fst::parser`)
//! 2. Semantic interpreter ([`semantics`])
//! 3. Dialog planner ([`planner`])
//! 4. Response realiser ([`realiser`])
//! 5. Morphological synthesiser (`adam_kernel_fst::morphotactics::synthesise_*`)
//!
//! The whole chain is pure-function except for [`planner::choose_template`],
//! which picks uniformly from ≤ 5 applicable templates for the recognised
//! intent. That is the ONLY source of randomness in the system.

pub mod entities;
pub mod intent;
pub mod planner;
pub mod realiser;
pub mod semantics;
pub mod templates;

pub use intent::{GreetingKind, Intent, SubjectPerson};
pub use planner::{ResponsePlan, plan_response};
pub use realiser::realise;
pub use semantics::{interpret, interpret_text};

/// End-to-end entry point: Kazakh text in, Kazakh text out.
///
/// Runs Layer 1 → 5. Returns a response string. Uses `interpret_text`
/// which combines raw-token keyword matching with morphological parses,
/// so tokens that aren't in the lexicon still get classified.
pub fn respond(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
    rng_seed: u64,
) -> String {
    let parses = parse_input(input, lexicon);
    let intent = interpret_text(input, &parses);
    let plan = plan_response(&intent, rng_seed);
    realise(&plan)
}

/// Layer 1 wrapper: parse each whitespace-separated token, keep only the
/// first (highest-confidence) analysis of each. Full disambiguation is
/// a future refinement; for MVP we proceed with the single-best parse.
fn parse_input(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
) -> Vec<adam_kernel_fst::parser::Analysis> {
    let mut out = Vec::new();
    for token in input.split_whitespace() {
        let cleaned: String = token
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        if cleaned.is_empty() {
            continue;
        }
        let analyses = adam_kernel_fst::parser::analyse(&cleaned, lexicon);
        if let Some(a) = analyses.into_iter().next() {
            out.push(a);
        }
    }
    out
}
