//! Layer 4 — response realiser.
//!
//! v0.8.0: templates may contain `{slot}` placeholders (e.g., `{name}`);
//! realiser substitutes them with the slot values the planner
//! extracted from the Intent. Unknown slots stay as-is — better to
//! emit a visible `{unfilled}` than silently drop information.
//!
//! v0.9.0 will extend this with `{root|features}` atoms, expanded
//! through `adam_kernel_fst::morphotactics::synthesise_*` so agreement
//! becomes FST-guaranteed rather than hand-written.

use crate::planner::ResponsePlan;

/// Render a response plan into the final output string.
pub fn realise(plan: &ResponsePlan) -> String {
    let mut out = plan.literal.clone();
    for (key, value) in &plan.slots {
        let needle = format!("{{{key}}}");
        out = out.replace(&needle, value);
    }
    out
}
