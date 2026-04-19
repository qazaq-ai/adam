//! Layer 4 — response realiser.
//!
//! v0.7.0 realiser is trivial: the planner already emitted a literal
//! Kazakh string, we just hand it through. No FST synthesis needed
//! because MVP templates are hardcoded grammatical forms.
//!
//! v0.7.5 will switch to slotted templates: `planner` will emit
//! `(template_id, slots)`, and `realiser` will expand the template's
//! `(root, features)` atoms through `adam_kernel_fst::morphotactics`.

use crate::planner::ResponsePlan;

/// Render a response plan into the final output string.
///
/// In v0.7.0 this is just the planner's literal. Exists as a separate
/// function so the v0.7.5 template expansion can drop in without
/// changing the top-level `respond()` signature.
pub fn realise(plan: &ResponsePlan) -> String {
    plan.literal.clone()
}
