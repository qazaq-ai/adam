//! Layer 3 — dialog planner. Given a recognised [`Intent`], pick a
//! template. The planner is a pure function of (intent, rng_seed): for
//! any given seed, the chosen template is fully determined. Changing
//! the seed picks a different template from the same enumerable set.
//!
//! The universe of possible responses per intent is finite and
//! enumerable. No free generation.

use crate::intent::{GreetingKind, Intent, TimeOfDay};

/// The planner's output — what the realiser needs to produce text.
///
/// For MVP we hand-code a small set of responses as literals. Once the
/// template repository grows (v0.7.5+) this will carry a
/// `template_id` + `slots` and the realiser will look it up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponsePlan {
    /// The literal surface form to emit. v0.7.0 uses hand-written
    /// responses; v0.7.5 switches to slotted templates.
    pub literal: String,
    /// Trace log entries — what the planner did, for --trace mode.
    pub trace: Vec<String>,
}

/// Pure function: (intent, seed) → plan.
pub fn plan_response(intent: &Intent, rng_seed: u64) -> ResponsePlan {
    let mut trace = Vec::new();
    trace.push(format!("planner: seed={rng_seed}"));
    trace.push(format!("planner: intent={intent:?}"));

    let applicable: Vec<&str> = match intent {
        Intent::Greeting { kind } => greeting_responses(*kind),
        Intent::Farewell => vec!["сау бол", "кездескенше", "аман бол"],
        Intent::Affirmation => vec!["иә", "дұрыс айтасыз", "рас", "мақұл"],
        Intent::Negation => vec!["жоқ", "дұрыс емес"],
        Intent::Unknown { .. } => vec!["түсінбедім", "қайта айтыңызшы"],
    };

    // Deterministic pick: rng_seed modulo applicable.len(). This is our
    // single stochastic component; keeping it pure-modular lets tests
    // reproduce exactly by seeding.
    let idx = (rng_seed as usize) % applicable.len().max(1);
    let chosen = applicable.get(idx).copied().unwrap_or("");
    trace.push(format!(
        "planner: applicable_count={} chosen_index={} text='{}'",
        applicable.len(),
        idx,
        chosen,
    ));

    ResponsePlan {
        literal: chosen.to_string(),
        trace,
    }
}

/// Greeting responses vary by greeting kind to mirror tone.
fn greeting_responses(kind: GreetingKind) -> Vec<&'static str> {
    match kind {
        GreetingKind::Casual => vec!["сәлем", "сәлем достым"],
        GreetingKind::Polite => vec!["сәлеметсіз бе", "армысыз"],
        GreetingKind::TimeOfDay(TimeOfDay::Morning) => {
            vec!["қайырлы таң", "қайырлы таң болсын"]
        }
        GreetingKind::TimeOfDay(TimeOfDay::Day) => vec!["қайырлы күн"],
        GreetingKind::TimeOfDay(TimeOfDay::Evening) => vec!["қайырлы кеш"],
    }
}
