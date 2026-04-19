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
/// use the configured Kazakh template set.
pub fn plan_response_with_repo(
    intent: &Intent,
    rng_seed: u64,
    repo: &TemplateRepository,
) -> ResponsePlan {
    let mut trace = Vec::new();
    trace.push(format!("planner: seed={rng_seed}"));
    trace.push(format!("planner: intent={intent:?}"));

    let key = intent_key(intent);
    trace.push(format!("planner: template_key={key}"));

    let applicable = repo.get(key);
    // Deterministic pick: rng_seed modulo applicable.len().
    let idx = (rng_seed as usize) % applicable.len().max(1);
    let chosen = applicable.get(idx).cloned().unwrap_or_default();
    trace.push(format!(
        "planner: applicable_count={} chosen_index={} text='{}'",
        applicable.len(),
        idx,
        chosen,
    ));

    let slots = extract_slots(intent);
    if !slots.is_empty() {
        trace.push(format!("planner: slots={slots:?}"));
    }

    ResponsePlan {
        literal: chosen,
        slots,
        trace,
    }
}

/// Pull entity values out of the Intent into a slot map the realiser
/// can substitute. v0.8.0 covers `{name}` only.
fn extract_slots(intent: &Intent) -> HashMap<String, String> {
    let mut slots = HashMap::new();
    if let Intent::StatementOfName { name } = intent {
        slots.insert("name".into(), name.clone());
    }
    slots
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
        Intent::StatementOfAge => "statement_of_age",
        Intent::AskLocation => "ask_location",
        Intent::StatementOfLocation => "statement_of_location",
        Intent::AskOccupation => "ask_occupation",
        Intent::StatementOfOccupation => "statement_of_occupation",
        Intent::AskFamily => "ask_family",
        Intent::StatementOfFamily => "statement_of_family",
        Intent::AskWeather => "ask_weather",
        Intent::StatementOfWeather => "statement_of_weather",
        Intent::AskTime => "ask_time",
        Intent::Compliment => "compliment",
        Intent::Request => "request",
        Intent::WellWishes => "well_wishes",
        Intent::Unknown { .. } => "unknown",
    }
}
