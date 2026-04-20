//! Layer 3.5 — multi-turn session state + dialog state tracking.
//!
//! v0.8.5 introduced a flat slot map. v1.4.0 adds proper Dialog State
//! Tracking (DST): the `Conversation` now carries
//!
//! ```text
//!   { slots: HashMap, active_intent: Option<IntentKind>, intent_history: Vec<IntentKind> }
//! ```
//!
//! - `slots` — accumulated entities across turns (name, age, city, occupation).
//! - `active_intent` — the kind of the LAST recognised intent. Used for
//!   follow-up resolution like "ал сіз?" ("and you?") which re-interpretates
//!   against the previous frame.
//! - `intent_history` — ordered list of every recognised intent kind this
//!   session (bounded to MAX_HISTORY to avoid unbounded growth).
//!
//! Deterministic-by-construction: given (slots, active_intent, input, seed),
//! the next turn's output is fully determined. No probabilistic decisions.

use std::collections::HashMap;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_retrieval::{MorphemeIndex, RankConfig};

use crate::intent::Intent;
use crate::planner::plan_response_with_session;
use crate::realiser::realise;
use crate::semantics::{content_roots, interpret_text_with_lexicon};
use crate::templates::TemplateRepository;

/// Maximum intent-history length retained across turns. Bounded so a
/// long-running session doesn't accumulate an unbounded trace.
const MAX_HISTORY: usize = 32;

/// A running multi-turn dialog.
///
/// v1.6.5 adds an optional [`MorphemeIndex`]: when attached, the
/// `Intent::Unknown` fallback consults the index for the parsed
/// `noun_hint` and cites a concrete Kazakh sentence from the committed
/// corpus in place of a bare "түсінбедім". The index is optional so
/// embedders (CLI, tests) that don't want the retrieval dependency
/// can continue to run the 26-intent pipeline stand-alone.
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// Entity slot values accumulated across turns
    /// (`name`, `age`, `city`, `occupation`).
    pub session: HashMap<String, String>,
    /// Kind of the last recognised intent. `None` before the first turn.
    /// Used by follow-up resolution (v1.4.0).
    pub active_intent: Option<IntentKind>,
    /// Ordered history of recognised intent kinds, oldest-first.
    /// Bounded to [`MAX_HISTORY`] items.
    pub intent_history: Vec<IntentKind>,
    /// Optional retrieval index. When `Some`, `Intent::Unknown` gets an
    /// `example` injected for any `noun_hint` that has postings in the
    /// index. Added v1.6.5.
    pub morpheme_index: Option<MorphemeIndex>,
    /// Tunable weights + pack purity priors for the v1.7.0 ranker.
    /// Defaults to `RankConfig::default()` — only override for tests
    /// or experiments. Uses `Option` rather than a direct value to keep
    /// `Conversation::default()` cheap and avoid threading the config
    /// through every embedder.
    pub rank_config: Option<RankConfig>,
}

/// Lightweight "kind" summary of an `Intent` — the payload (name /
/// years / city / …) is already held in `slots`, so history doesn't
/// need to copy it. Keeping this separate from `intent::Intent` avoids
/// retaining potentially large `String`s in the session log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentKind {
    Greeting,
    Farewell,
    Affirmation,
    Negation,
    Thanks,
    Apology,
    AskHowAreYou,
    StatementOfWellbeing,
    AskName,
    StatementOfName,
    AskAge,
    StatementOfAge,
    AskLocation,
    StatementOfLocation,
    AskOccupation,
    StatementOfOccupation,
    AskFamily,
    StatementOfFamily,
    AskWeather,
    StatementOfWeather,
    AskTime,
    Compliment,
    Request,
    WellWishes,
    Insult,
    Unknown,
}

impl From<&Intent> for IntentKind {
    fn from(intent: &Intent) -> Self {
        match intent {
            Intent::Greeting { .. } => Self::Greeting,
            Intent::Farewell => Self::Farewell,
            Intent::Affirmation => Self::Affirmation,
            Intent::Negation => Self::Negation,
            Intent::Thanks => Self::Thanks,
            Intent::Apology => Self::Apology,
            Intent::AskHowAreYou => Self::AskHowAreYou,
            Intent::StatementOfWellbeing => Self::StatementOfWellbeing,
            Intent::AskName => Self::AskName,
            Intent::StatementOfName { .. } => Self::StatementOfName,
            Intent::AskAge => Self::AskAge,
            Intent::StatementOfAge { .. } => Self::StatementOfAge,
            Intent::AskLocation => Self::AskLocation,
            Intent::StatementOfLocation { .. } => Self::StatementOfLocation,
            Intent::AskOccupation => Self::AskOccupation,
            Intent::StatementOfOccupation { .. } => Self::StatementOfOccupation,
            Intent::AskFamily => Self::AskFamily,
            Intent::StatementOfFamily => Self::StatementOfFamily,
            Intent::AskWeather => Self::AskWeather,
            Intent::StatementOfWeather => Self::StatementOfWeather,
            Intent::AskTime => Self::AskTime,
            Intent::Compliment => Self::Compliment,
            Intent::Request => Self::Request,
            Intent::WellWishes => Self::WellWishes,
            Intent::Insult => Self::Insult,
            Intent::Unknown { .. } => Self::Unknown,
        }
    }
}

impl Conversation {
    /// Start a fresh session — no remembered entities and no history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a retrieval index so `Intent::Unknown` can quote a
    /// concrete Kazakh sentence for any recognised noun. Without this,
    /// the fallback behaviour is the v1.1.0 noun-echo path.
    pub fn with_morpheme_index(mut self, index: MorphemeIndex) -> Self {
        self.morpheme_index = Some(index);
        self
    }

    /// Run one conversational turn. Parses the input, recognises the
    /// intent, folds any new entities into [`session`](Self::session),
    /// updates [`active_intent`](Self::active_intent) and
    /// [`intent_history`](Self::intent_history), then plans + realises
    /// a response using the merged slot map.
    ///
    /// Deterministic given (current state, input, seed). The state
    /// mutation is the ONLY side-effect.
    pub fn turn(
        &mut self,
        input: &str,
        lexicon: &LexiconV1,
        repo: &TemplateRepository,
        rng_seed: u64,
    ) -> String {
        let parses = crate::parse_input_public(input, lexicon);
        let raw_intent = interpret_text_with_lexicon(input, &parses, Some(lexicon));

        // v1.4.0: follow-up resolution. "ал сіз?" after AskHowAreYou
        // becomes a re-interpretation: "user is asking me the same
        // thing back", which is still AskHowAreYou for planning
        // purposes (planner picks a response without asking back).
        let mut intent = resolve_follow_up(raw_intent, input, self.active_intent);

        // v1.6.5 / v1.7.0: inject a retrieval example into Unknown.
        // Deterministic — ranker ties break by (pack, sample_id), so
        // reruns produce identical evidence. rng_seed is NOT consulted.
        self.inject_retrieval_example(&mut intent, &parses);

        self.absorb_entities(&intent);
        self.record_intent(&intent);
        let plan = plan_response_with_session(&intent, rng_seed, repo, &self.session);
        realise(&plan)
    }

    /// For `Intent::Unknown { noun_hint: Some(n), .. }`, if an index is
    /// attached, call `MorphemeIndex::rank` with every content root
    /// parsed from the input (v1.7.0) and fill the `example` slot with
    /// the top-1 hit's text. Falls back to `search(noun_hint)[0]` when
    /// ranking returns nothing. No-op for every non-Unknown intent, for
    /// unknown-without-noun, and when the index is absent.
    fn inject_retrieval_example(
        &self,
        intent: &mut Intent,
        parses: &[adam_kernel_fst::parser::Analysis],
    ) {
        let Some(index) = self.morpheme_index.as_ref() else {
            return;
        };
        if let Intent::Unknown {
            noun_hint: Some(noun),
            example,
            ..
        } = intent
        {
            if example.is_some() {
                return;
            }
            // v1.7.0: rank over all content roots, not just the first.
            let roots = content_roots(parses);
            let root_refs: Vec<&str> = if roots.is_empty() {
                vec![noun.as_str()]
            } else {
                roots.iter().map(|s| s.as_str()).collect()
            };
            let default_cfg;
            let config = match self.rank_config.as_ref() {
                Some(c) => c,
                None => {
                    default_cfg = RankConfig::default();
                    &default_cfg
                }
            };
            let ranked = index.rank(&root_refs, config);
            if let Some(hit) = ranked.first() {
                if let Some(text) = index.sample_text(&hit.sref) {
                    *example = Some(text.to_string());
                    return;
                }
            }
            // Fallback: single-morpheme first-posting (the v1.6.5 path)
            // — kicks in when the ranker found nothing or the top hit
            // lacks a stored text.
            let refs = index.search(noun);
            if let Some(first) = refs.first() {
                if let Some(text) = index.sample_text(first) {
                    *example = Some(text.to_string());
                }
            }
        }
    }

    /// Extract persistent entities from an intent and push them into
    /// the running session.
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

    /// Update `active_intent` + push to `intent_history` with a bounded
    /// capacity. Called on every turn by [`turn`](Self::turn).
    fn record_intent(&mut self, intent: &Intent) {
        let kind = IntentKind::from(intent);
        self.active_intent = Some(kind);
        if self.intent_history.len() >= MAX_HISTORY {
            self.intent_history.remove(0);
        }
        self.intent_history.push(kind);
    }

    /// Clear all conversation state — slots, active intent, history.
    pub fn reset(&mut self) {
        self.session.clear();
        self.active_intent = None;
        self.intent_history.clear();
    }
}

/// v1.4.0 follow-up resolution. Some Kazakh utterances are meaningless
/// out of context but carry a pointer back to the previous turn:
///
///   "ал сіз?"     — "and you?"       (flip subject of last Ask-)
///   "ал сен?"     — informal version
///
/// When we detect a bare reflective-query as `Unknown` (or any
/// weak-intent), and the PREVIOUS turn was an Ask- intent, we re-tag
/// the current turn as the same Ask kind, so the planner picks a
/// matching response template.
fn resolve_follow_up(raw: Intent, input: &str, active: Option<IntentKind>) -> Intent {
    let normalised: String = input.to_lowercase();
    let is_reflective = normalised.trim() == "ал сіз"
        || normalised.trim() == "ал сіз?"
        || normalised.trim() == "ал сен"
        || normalised.trim() == "ал сен?"
        || normalised.trim() == "сіз ше"
        || normalised.trim() == "сен ше";
    if !is_reflective {
        return raw;
    }
    // Only reroute when the raw intent is weak (Unknown / Affirmation)
    // — never override a clearly-recognised strong intent.
    if !matches!(
        raw,
        Intent::Unknown { .. } | Intent::Affirmation | Intent::Negation
    ) {
        return raw;
    }
    match active {
        Some(IntentKind::AskHowAreYou) | Some(IntentKind::StatementOfWellbeing) => {
            Intent::AskHowAreYou
        }
        Some(IntentKind::AskName) | Some(IntentKind::StatementOfName) => Intent::AskName,
        Some(IntentKind::AskAge) | Some(IntentKind::StatementOfAge) => Intent::AskAge,
        Some(IntentKind::AskLocation) | Some(IntentKind::StatementOfLocation) => {
            Intent::AskLocation
        }
        Some(IntentKind::AskOccupation) | Some(IntentKind::StatementOfOccupation) => {
            Intent::AskOccupation
        }
        _ => raw,
    }
}
