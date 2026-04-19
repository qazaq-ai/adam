//! Layer 3.5 — multi-turn session state.
//!
//! v0.8.5 adds the first piece of persistent memory to the dialog
//! layer: a [`Conversation`] accumulates entities extracted from past
//! turns (the user's name, later: age, location, occupation) and
//! makes them available to the planner as slot values on every
//! subsequent turn.
//!
//! The design is deliberately small:
//!
//! ```text
//!  turn N:  input --parse--> intent --extract--> new-entities
//!                                                     |
//!                       session (from turn N-1) <-----+
//!                                |
//!                                v
//!                       planner: pick template that
//!                                is fillable from session
//!                                |
//!                                v
//!                       realiser: substitute {slot}s
//! ```
//!
//! No free generation — the only thing session state changes is which
//! TEMPLATE is eligible, and how its `{slot}` placeholders are filled.
//! The MVP predictability property holds: given (session, intent,
//! seed) the output is fully determined.

use std::collections::HashMap;

use adam_kernel_fst::lexicon::LexiconV1;

use crate::intent::Intent;
use crate::planner::plan_response_with_session;
use crate::realiser::realise;
use crate::semantics::interpret_text;
use crate::templates::TemplateRepository;

/// A running multi-turn dialog. Holds accumulated session entities
/// (name, age, location, …) and exposes a single [`turn`](Self::turn)
/// method for "input string → response string".
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// All entities extracted from past turns, keyed by slot name.
    /// Current supported slots: `name`.
    pub session: HashMap<String, String>,
}

impl Conversation {
    /// Start a fresh session — no remembered entities.
    pub fn new() -> Self {
        Self::default()
    }

    /// Run one conversational turn. Parses the input, recognises the
    /// intent, folds any new entities into [`session`](Self::session),
    /// then plans + realises a response using the merged slot map.
    ///
    /// Deterministic given (current session, input, seed). The session
    /// mutation is the ONLY side-effect.
    pub fn turn(
        &mut self,
        input: &str,
        lexicon: &LexiconV1,
        repo: &TemplateRepository,
        rng_seed: u64,
    ) -> String {
        let parses = crate::parse_input_public(input, lexicon);
        let intent = interpret_text(input, &parses);
        self.absorb_entities(&intent);
        let plan = plan_response_with_session(&intent, rng_seed, repo, &self.session);
        realise(&plan)
    }

    /// Extract persistent entities from an intent and push them into
    /// the running session.
    ///
    /// v0.8.5: `{name}` only.
    /// v0.9.0: `{name}`, `{age}`, `{city}`, `{occupation}` — every
    /// statement intent carrying an extractable entity contributes it.
    pub(crate) fn absorb_entities(&mut self, intent: &Intent) {
        match intent {
            Intent::StatementOfName { name } => {
                self.session.insert("name".into(), name.clone());
            }
            Intent::StatementOfAge { years: Some(years) } => {
                self.session.insert("age".into(), years.to_string());
            }
            Intent::StatementOfLocation { city: Some(city) } => {
                self.session.insert("city".into(), city.clone());
            }
            Intent::StatementOfOccupation {
                occupation: Some(occupation),
            } => {
                self.session.insert("occupation".into(), occupation.clone());
            }
            _ => {}
        }
    }

    /// Clear all session state — used when the user wants a fresh
    /// conversation without dropping the Conversation instance.
    pub fn reset(&mut self) {
        self.session.clear();
    }
}
