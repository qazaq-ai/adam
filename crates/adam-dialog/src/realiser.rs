//! Layer 4 — response realiser.
//!
//! v0.8.0: plain `{slot}` placeholders, text substitution only.
//! v0.9.5: `{slot|features}` FST-backed expansion. `features` is a
//! `+`-separated spec (see `slot_syntax`) that describes the noun
//! case + number to synthesise via `adam_kernel_fst::morphotactics`.
//!
//! Examples:
//! ```text
//!   "сәлем {name}"                    → "сәлем Дәулет"
//!   "{city|locative} тұрасыз ба?"     → "Алматыда тұрасыз ба?"
//!   "{occupation|plural} қажет"       → "мұғалімдер қажет"
//! ```
//!
//! Unfilled placeholders stay visible as literal `{name}` / `{city|loc}`
//! — deliberately ugly so missing-slot bugs surface during QA rather
//! than silently dropping information.

use adam_kernel_fst::morphotactics::synthesise_noun;

use crate::planner::ResponsePlan;
use crate::slot_syntax::{parse_noun_features, parse_placeholder};

/// Render a response plan into the final output string. Scans the
/// template left-to-right, expanding `{...}` placeholders as they
/// appear; everything else is emitted verbatim.
pub fn realise(plan: &ResponsePlan) -> String {
    expand_template(&plan.literal, &plan.slots)
}

fn expand_template(template: &str, slots: &std::collections::HashMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    let bytes = template.as_bytes();
    while i < bytes.len() {
        // `{` is a single byte; safe to compare at a byte offset since
        // the preceding iterations are char-aligned.
        if bytes[i] == b'{' {
            if let Some(end_rel) = template[i + 1..].find('}') {
                let inner = &template[i + 1..i + 1 + end_rel];
                out.push_str(&expand_placeholder(inner, slots));
                i += 1 + end_rel + 1;
                continue;
            }
        }
        let ch = template[i..].chars().next().expect("valid utf-8 boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn expand_placeholder(inner: &str, slots: &std::collections::HashMap<String, String>) -> String {
    let (slot_name, feature_spec) = parse_placeholder(inner);
    let Some(root) = slots.get(slot_name) else {
        // Unfilled — leave the raw placeholder visible.
        return format!("{{{inner}}}");
    };
    match feature_spec {
        None => root.clone(),
        Some(spec) => {
            let features = parse_noun_features(spec);
            synthesise_noun(root, features)
        }
    }
}
