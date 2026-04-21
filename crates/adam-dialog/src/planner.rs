//! Layer 3 — dialog planner. Given a recognised [`Intent`], pick a
//! template. The planner is a pure function of (intent, template
//! repository, rng_seed): for any given seed the chosen template is
//! fully determined.
//!
//! v0.7.5 change: template content now comes from the
//! [`TemplateRepository`] loaded from `data/dialog/templates/v1.toml`.
//! Only the INTENT → TEMPLATE-KEY mapping lives in code; the actual
//! response strings are external data that can be extended without
//! recompiling.

use std::collections::HashMap;

use crate::intent::{GreetingKind, Intent, TimeOfDay};
use crate::templates::TemplateRepository;

/// Output of Layer 3 — what the realiser needs to produce text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponsePlan {
    /// The chosen template, possibly containing `{slot}` placeholders.
    pub literal: String,
    /// Slot values extracted from the intent (e.g., `{"name": "Дәулет"}`).
    /// The realiser substitutes `{slot}` in `literal` with the mapped
    /// value; unknown slots stay as-is.
    pub slots: HashMap<String, String>,
    /// Trace log entries — for --trace mode.
    pub trace: Vec<String>,
}

/// Pure function over (intent, seed). Uses a hardcoded-fallback
/// repository so callers that don't load the TOML still get a
/// reasonable response for the common intents.
pub fn plan_response(intent: &Intent, rng_seed: u64) -> ResponsePlan {
    plan_response_with_repo(intent, rng_seed, &TemplateRepository::hardcoded_fallback())
}

/// Full-information variant. Supply the loaded TemplateRepository to
/// use the configured Kazakh template set. Equivalent to calling
/// [`plan_response_with_session`] with an empty session map.
pub fn plan_response_with_repo(
    intent: &Intent,
    rng_seed: u64,
    repo: &TemplateRepository,
) -> ResponsePlan {
    plan_response_with_session(intent, rng_seed, repo, &HashMap::new())
}

/// Session-aware variant. `session` holds entities extracted from past
/// turns (see `Conversation` in `conversation.rs`). The planner:
///
/// 1. Computes the full slot map = per-intent slots ∪ session slots.
///    Per-intent slots win on collision (a freshly stated name
///    overrides an older remembered one).
/// 2. Filters the candidate template pool down to templates whose
///    every `{slot}` reference can be satisfied from that slot map.
///    If filtering empties the pool, falls back to the unfiltered
///    pool — templates with unfilled `{slot}` are visibly ugly which
///    is better than a silent crash.
/// 3. Seed-mod picks among the filtered templates.
pub fn plan_response_with_session(
    intent: &Intent,
    rng_seed: u64,
    repo: &TemplateRepository,
    session: &HashMap<String, String>,
) -> ResponsePlan {
    let mut trace = Vec::new();
    trace.push(format!("planner: seed={rng_seed}"));
    trace.push(format!("planner: intent={intent:?}"));

    let key = intent_key(intent);
    trace.push(format!("planner: template_key={key}"));

    // Merge per-turn slots with persistent session entities. Per-turn
    // wins on collision.
    let mut slots = session.clone();
    for (k, v) in extract_slots(intent) {
        slots.insert(k, v);
    }
    if !slots.is_empty() {
        trace.push(format!("planner: slots={slots:?}"));
    }

    let applicable_all = repo.get(key);
    let fillable: Vec<&String> = applicable_all
        .iter()
        .filter(|t| template_is_fillable(t, &slots))
        .collect();
    let effective: Vec<&String> = if fillable.is_empty() {
        applicable_all.iter().collect()
    } else {
        fillable
    };

    let idx = (rng_seed as usize) % effective.len().max(1);
    let chosen = effective.get(idx).map(|s| (*s).clone()).unwrap_or_default();
    trace.push(format!(
        "planner: applicable_total={} fillable={} chosen_index={} text='{}'",
        applicable_all.len(),
        effective.len(),
        idx,
        chosen,
    ));

    ResponsePlan {
        literal: chosen,
        slots,
        trace,
    }
}

/// True iff every `{placeholder}` appearing in `template` has a
/// corresponding key in `slots`. Understands the `{slot|features}`
/// syntax introduced in v0.9.5 — features don't affect fillability,
/// only the slot name is checked. Literal-only templates are always
/// fillable.
fn template_is_fillable(template: &str, slots: &HashMap<String, String>) -> bool {
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end_rel) = template[i + 1..].find('}') {
                let inner = &template[i + 1..i + 1 + end_rel];
                let (slot_name, _) = crate::slot_syntax::parse_placeholder(inner);
                if !slots.contains_key(slot_name) {
                    return false;
                }
                i += 1 + end_rel + 1;
                continue;
            }
        }
        i += 1;
    }
    true
}

/// Pull entity values out of the Intent into a slot map the realiser
/// can substitute.
///
/// v0.8.0: `{name}` only.
/// v0.9.0: `{name}`, `{age}`, `{city}`, `{occupation}` — every intent
/// that carries an optional entity contributes its slot when the
/// entity is present.
fn extract_slots(intent: &Intent) -> HashMap<String, String> {
    let mut slots = HashMap::new();
    match intent {
        Intent::StatementOfName { name } => {
            slots.insert("name".into(), name.clone());
        }
        Intent::StatementOfAge { years: Some(years) } => {
            slots.insert("age".into(), years.to_string());
        }
        Intent::StatementOfLocation { city: Some(city) } => {
            slots.insert("city".into(), city.clone());
        }
        Intent::StatementOfOccupation {
            occupation: Some(occupation),
        } => {
            slots.insert("occupation".into(), occupation.clone());
        }
        Intent::Unknown {
            noun_hint,
            example,
            reasoning_chain,
            ..
        } => {
            if let Some(noun) = noun_hint {
                slots.insert("noun".into(), noun.clone());
            }
            if let Some(ex) = example {
                slots.insert("example".into(), ex.clone());
            }
            if let Some(chain) = reasoning_chain {
                slots.insert("chain".into(), chain.clone());
            }
        }
        _ => {}
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_without_placeholder_is_always_fillable() {
        assert!(template_is_fillable("сәлем", &HashMap::new()));
    }

    #[test]
    fn template_with_placeholder_requires_slot() {
        let mut slots = HashMap::new();
        assert!(!template_is_fillable("сәлем {name}", &slots));
        slots.insert("name".into(), "Дәулет".into());
        assert!(template_is_fillable("сәлем {name}", &slots));
    }

    #[test]
    fn template_with_multiple_placeholders_needs_all_slots() {
        let mut slots = HashMap::new();
        slots.insert("name".into(), "Дәулет".into());
        assert!(!template_is_fillable(
            "сәлем {name}, сіз {city}-дансыз ба",
            &slots,
        ));
        slots.insert("city".into(), "Алматы".into());
        assert!(template_is_fillable(
            "сәлем {name}, сіз {city}-дансыз ба",
            &slots,
        ));
    }

    #[test]
    fn placeholder_with_features_uses_slot_name_for_fillability() {
        let mut slots = HashMap::new();
        assert!(!template_is_fillable("{city|locative} тұрамын", &slots));
        slots.insert("city".into(), "Алматы".into());
        assert!(template_is_fillable("{city|locative} тұрамын", &slots));
    }
}

/// Map an [`Intent`] to the template-repository key that holds its
/// responses. This is the ONLY place the mapping lives — both the
/// default planner and test harnesses reuse it.
pub fn intent_key(intent: &Intent) -> &'static str {
    match intent {
        Intent::Greeting { kind } => match kind {
            GreetingKind::Casual => "greeting.casual",
            GreetingKind::Polite => "greeting.polite",
            GreetingKind::TimeOfDay(TimeOfDay::Morning) => "greeting.morning",
            GreetingKind::TimeOfDay(TimeOfDay::Day) => "greeting.day",
            GreetingKind::TimeOfDay(TimeOfDay::Evening) => "greeting.evening",
        },
        Intent::Farewell => "farewell",
        Intent::Affirmation => "affirmation",
        Intent::Negation => "negation",
        Intent::Thanks => "thanks",
        Intent::Apology => "apology",
        Intent::AskHowAreYou => "ask_how_are_you",
        Intent::StatementOfWellbeing => "statement_of_wellbeing",
        Intent::AskName => "ask_name",
        Intent::StatementOfName { .. } => "statement_of_name",
        Intent::AskAge => "ask_age",
        Intent::StatementOfAge { .. } => "statement_of_age",
        Intent::AskLocation => "ask_location",
        Intent::StatementOfLocation { .. } => "statement_of_location",
        Intent::AskOccupation => "ask_occupation",
        Intent::StatementOfOccupation { .. } => "statement_of_occupation",
        Intent::AskFamily => "ask_family",
        Intent::StatementOfFamily => "statement_of_family",
        Intent::AskWeather => "ask_weather",
        Intent::StatementOfWeather => "statement_of_weather",
        Intent::AskTime => "ask_time",
        Intent::Compliment => "compliment",
        Intent::Request => "request",
        Intent::WellWishes => "well_wishes",
        Intent::Insult => "insult",
        Intent::Unknown {
            noun_hint,
            example,
            example_adapted,
            reasoning_chain,
            ..
        } => {
            // v2.7: reasoning-chain takes priority over retrieval when
            // both are available, because a derived chain shows how the
            // system CONNECTED facts rather than just cited one. The
            // `unknown.with_derived_chain` family always includes the
            // «байланыс-» marker for user-side auditability (mirrors
            // v1.9.5's «бейімд-» marker for adapted evidence).
            //
            // v1.9.5: adapted-evidence routing (retrieval was rewritten
            // by compose_with_city); the «бейімд-» marker fires.
            //
            // v1.6.5: verbatim retrieval evidence.
            // v1.1.0: noun-echo acknowledgement when no retrieval hit.
            // v1.0.0: bare "түсінбедім" fallback.
            if reasoning_chain.is_some() {
                "unknown.with_derived_chain"
            } else if example.is_some() && *example_adapted {
                "unknown.with_adapted_evidence"
            } else if example.is_some() {
                "unknown.with_evidence"
            } else if noun_hint.is_some() {
                "unknown.with_noun"
            } else {
                "unknown"
            }
        }
    }
}
