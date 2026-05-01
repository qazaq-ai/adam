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

use crate::intent::{
    GreetingKind, Intent, TimeOfDay, UnknownAnswerMode, unknown_answer_mode,
    unknown_prefers_quoted_example,
};
use crate::language_core::geo_entity_kind;
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
    ensure_geo_kind_slot(&mut slots);
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

/// v4.0.34 — epistemic-aware planner entry (Codex roadmap Phase 5
/// part 2). Runs the same template-selection algorithm as
/// [`plan_response_with_session`] but additionally looks at the
/// `EpistemicStatus` derived by [`crate::uncertainty::UncertaintyPolicy`]
/// and — for `Intent::Unknown { noun_hint: Some(_), .. }` — overrides
/// the template key to route conflict / tentative responses to their
/// own template families.
///
/// Override rules:
///
/// - `EpistemicStatus::Conflicted` → `unknown.conflicted` (uses
///   `{predicate}`, `{old_value}`, `{new_value}` slots supplied by
///   the caller via `extra_slots`).
/// - `EpistemicStatus::Tentative` → `unknown.tentative` (uses the
///   `{noun}` slot, softer wording, invites clarification).
/// - Any other status, or intent without `noun_hint`, or the
///   overridden family not being present in the repo → falls back
///   to the base `intent_key(intent)` key and the delegated
///   [`plan_response_with_session`] path.
///
/// Non-`Unknown` intents (greetings, profile statements, etc.) are
/// untouched — epistemic overrides only apply where the dialog
/// currently routes `Unknown.with_noun` / `unknown.with_derived_chain` /
/// `unknown`. This keeps the reply text byte-identical to v4.0.33 for
/// every non-Unknown intent.
pub fn plan_response_with_epistemic(
    intent: &Intent,
    rng_seed: u64,
    repo: &TemplateRepository,
    session: &HashMap<String, String>,
    epistemic: crate::uncertainty::EpistemicStatus,
    extra_slots: &HashMap<String, String>,
) -> ResponsePlan {
    let mut trace = Vec::new();
    trace.push(format!("planner: seed={rng_seed}"));
    trace.push(format!("planner: intent={intent:?}"));
    trace.push(format!("planner: epistemic={epistemic:?}"));

    let base_key = intent_key(intent);
    // **v4.4.0** — when `extra_slots` carries the
    // `__dismiss_contradiction__` marker, route through the
    // `dismiss_contradiction` template family unconditionally.
    // The marker is set by `Conversation::turn_with_trace` when
    // the user opted out of resolving a pending contradiction
    // ("екеуі де жоқ" / "білмеймін" / etc.). Bypasses epistemic
    // overrides because dismissal is a deliberate state-transition
    // ack, not an evidence-shaped reply.
    if extra_slots.contains_key("__dismiss_contradiction__") {
        let key = "dismiss_contradiction";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: dismiss_contradiction override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all
                .get(idx)
                .map(|s| s.clone())
                .unwrap_or_default();
            trace.push(format!(
                "planner: applicable_total={} chosen_index={} text='{}'",
                applicable_all.len(),
                idx,
                chosen,
            ));
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    // **v4.4.5** — when `extra_slots` carries the
    // `__check_contradiction__` marker, route through the
    // `check_contradiction` template family unconditionally. The
    // marker is set by `Conversation::turn_with_trace` whenever
    // the action plan is `Action::CheckContradiction`. Pre-v4.4.5
    // the action layer correctly chose CheckContradiction but the
    // renderer fell through to `intent_key(intent)` (e.g.
    // `statement_of_location`) and emitted a normal confirmation,
    // committing to one of the contested values — the bug Codex
    // flagged from a live REPL trace on 2026-04-27. Slots
    // `{old_value}` / `{new_value}` are already populated by
    // `Conversation` from the latest `BeliefConflict`.
    if extra_slots.contains_key("__check_contradiction__") {
        let key = "check_contradiction";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: check_contradiction override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all
                .get(idx)
                .map(|s| s.clone())
                .unwrap_or_default();
            trace.push(format!(
                "planner: applicable_total={} chosen_index={} text='{}'",
                applicable_all.len(),
                idx,
                chosen,
            ));
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    // **v4.6.12** — Russian-input marker. When the user types
    // Russian / non-Kazakh Cyrillic, route to the dedicated
    // `unknown.non_kazakh` template family that explains adam's
    // Kazakh-only policy. Set by `Conversation::turn_with_trace`
    // via `discourse::input_is_likely_russian`. Bypasses the rest
    // of the planner because the input wasn't Kazakh in the first
    // place — no FST analysis or topic recovery would be
    // meaningful.
    // **v4.6.15** — Math-answer marker. Set when
    // `discourse::try_evaluate_arithmetic` parsed the input as
    // pure integer arithmetic and computed a result. Routes to
    // `math_answer` template family with the `{math_value}`
    // slot pre-filled. Pre-v4.6.15 even computable inputs like
    // «5+5» refused via `math_refusal` — adam now ANSWERS the
    // arithmetic deterministically (no novel-text generation;
    // `try_evaluate_arithmetic` is a pure function).
    if let Some(value) = extra_slots.get("__math_answer__") {
        let key = "math_answer";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: math_answer override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all
                .get(idx)
                .map(|s| s.clone())
                .unwrap_or_default();
            trace.push(format!(
                "planner: applicable_total={} chosen_index={} text='{}'",
                applicable_all.len(),
                idx,
                chosen,
            ));
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
            slots.insert("math_value".into(), value.clone());
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    // **v4.6.12** — Math-input marker. Routes to the dedicated
    // `math_refusal` template family explaining adam doesn't
    // compute arithmetic. Mirrors the non-Kazakh override below.
    if extra_slots.contains_key("__math_input__") {
        let key = "math_refusal";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: math_refusal override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all
                .get(idx)
                .map(|s| s.clone())
                .unwrap_or_default();
            trace.push(format!(
                "planner: applicable_total={} chosen_index={} text='{}'",
                applicable_all.len(),
                idx,
                chosen,
            ));
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    if extra_slots.contains_key("__non_kazakh__") {
        let key = "unknown.non_kazakh";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: non_kazakh override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all
                .get(idx)
                .map(|s| s.clone())
                .unwrap_or_default();
            trace.push(format!(
                "planner: applicable_total={} chosen_index={} text='{}'",
                applicable_all.len(),
                idx,
                chosen,
            ));
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    let override_key = match (intent, epistemic) {
        (
            Intent::Unknown {
                noun_hint: Some(_), ..
            },
            crate::uncertainty::EpistemicStatus::Conflicted,
        ) => Some("unknown.conflicted"),
        (
            Intent::Unknown {
                noun_hint: Some(_), ..
            },
            crate::uncertainty::EpistemicStatus::Tentative,
        ) => Some("unknown.tentative"),
        // **v4.2.5** — `Action::AnswerDirect` rendering: when the user
        // asks about a profile slot AND the session has it, prefer
        // the `*.with_known_user` family that cites the stored value.
        // Mirrors the v4.0.34 epistemic-override pattern: the override
        // key only takes effect if the repo actually carries
        // templates under it (`!repo.get(k).is_empty()` below), so a
        // missing template family silently falls back to the bare
        // `ask_*` self-introduction templates. Closes the v4.2.1
        // aspirational gap (5 `direct_answer_*` scenarios).
        (Intent::AskName, _) if session.contains_key("name") => Some("ask_name.with_known_user"),
        (Intent::AskAge, _) if session.contains_key("age") => Some("ask_age.with_known_user"),
        (Intent::AskLocation, _) if session.contains_key("city") => {
            if session_geo_kind(session)
                .is_some_and(|kind| uses_geo_feature_location_family(kind.as_str()))
            {
                Some("ask_location.with_known_user.geo_feature")
            } else {
                Some("ask_location.with_known_user")
            }
        }
        (Intent::AskOccupation, _) if session.contains_key("occupation") => {
            Some("ask_occupation.with_known_user")
        }
        _ => None,
    };

    // Only apply the override if the repo actually has templates
    // registered under the overridden key. Protects against
    // template-pack regressions — if the v4.0.34 families were
    // dropped from v1.toml, we fall back to v4.0.33 behaviour
    // automatically.
    let key = match override_key {
        Some(k) if !repo.get(k).is_empty() => {
            trace.push(format!("planner: epistemic override → {k}"));
            k
        }
        _ => base_key,
    };
    trace.push(format!("planner: template_key={key}"));

    let mut slots = session.clone();
    for (k, v) in extract_slots(intent) {
        slots.insert(k, v);
    }
    // Per-turn extras (e.g. conflict predicate / old_value /
    // new_value) take precedence over session + intent-extracted
    // slots since they describe this specific turn's state.
    for (k, v) in extra_slots {
        slots.insert(k.clone(), v.clone());
    }
    ensure_geo_kind_slot(&mut slots);
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

fn ensure_geo_kind_slot(slots: &mut HashMap<String, String>) {
    if slots.contains_key("geo_kind") {
        return;
    }
    if let Some(city) = slots.get("city").cloned() {
        if let Some(kind) = geo_entity_kind(&city) {
            slots.insert("geo_kind".into(), kind);
        }
    }
}

fn session_geo_kind(session: &HashMap<String, String>) -> Option<String> {
    session
        .get("geo_kind")
        .cloned()
        .or_else(|| session.get("city").and_then(|city| geo_entity_kind(city)))
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
            if let Some(kind) = geo_entity_kind(city) {
                slots.insert("geo_kind".into(), kind);
            }
        }
        Intent::StatementOfOccupation {
            occupation: Some(occupation),
        } => {
            slots.insert("occupation".into(), occupation.clone());
        }
        Intent::Unknown {
            noun_hint,
            example,
            grounded_fact,
            reasoning_chain,
            ..
        } => {
            if let Some(noun) = noun_hint {
                slots.insert("noun".into(), noun.clone());
            }
            if let Some(ex) = example {
                slots.insert("example".into(), ex.clone());
            }
            if let Some(fact) = grounded_fact {
                slots.insert("fact".into(), fact.clone());
            }
            if let Some(chain) = reasoning_chain {
                slots.insert("chain".into(), chain.clone());
            }
        }
        _ => {}
    }
    slots
}

fn uses_geo_feature_location_family(kind: &str) -> bool {
    let kind = kind.to_lowercase();
    kind.contains("теңіз") || kind.contains("өзен") || kind.contains("көл") || kind.contains("тау")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::Intent;

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

    #[test]
    fn reasoning_mode_routes_unknown_to_derived_chain_family() {
        let intent = Intent::Unknown {
            raw_tokens: vec!["абай".into(), "неге".into(), "байланысты".into()],
            noun_hint: Some("абай".into()),
            example: Some("Абайдың өлеңдері көп.".into()),
            grounded_fact: Some("Абай Құнанбайұлы — қазақ ақыны.".into()),
            example_adapted: false,
            reasoning_chain: Some("байланыс бойынша, абай әдебиетке жатады.".into()),
            question_shape: None,
        };
        assert_eq!(intent_key(&intent), "unknown.with_derived_chain");
    }

    #[test]
    fn general_mode_keeps_grounded_fact_ahead_of_reasoning_chain() {
        let intent = Intent::Unknown {
            raw_tokens: vec![
                "абай".into(),
                "туралы".into(),
                "не".into(),
                "білесіз".into(),
            ],
            noun_hint: Some("абай".into()),
            example: Some("Абайдың өлеңдері көп.".into()),
            grounded_fact: Some("Абай Құнанбайұлы — қазақ ақыны.".into()),
            example_adapted: false,
            reasoning_chain: Some("байланыс бойынша, абай әдебиетке жатады.".into()),
            question_shape: None,
        };
        assert_eq!(intent_key(&intent), "unknown.with_grounded_fact");
    }

    #[test]
    fn geo_feature_location_routes_to_special_family() {
        let intent = Intent::StatementOfLocation {
            city: Some("Каспий".into()),
        };
        assert_eq!(intent_key(&intent), "statement_of_location.geo_feature");
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
            GreetingKind::IntroProposal => "greeting.intro_proposal",
        },
        Intent::Farewell => "farewell",
        Intent::Affirmation => "affirmation",
        Intent::Negation => "negation",
        Intent::Thanks => "thanks",
        Intent::Apology => "apology",
        Intent::AskHowAreYou => "ask_how_are_you",
        Intent::StatementOfWellbeing => "statement_of_wellbeing",
        Intent::AskName => "ask_name",
        Intent::AskAboutSystem { aspect } => match aspect {
            crate::system_identity::SystemAspect::General => "ask_about_system",
            crate::system_identity::SystemAspect::Creator => "ask_about_system.creator",
            crate::system_identity::SystemAspect::Birthdate => "ask_about_system.birthdate",
            crate::system_identity::SystemAspect::Architecture => "ask_about_system.architecture",
            // v4.6.0 — three new self-awareness aspects.
            crate::system_identity::SystemAspect::Capabilities => "ask_about_system.capabilities",
            crate::system_identity::SystemAspect::Knowledge => "ask_about_system.knowledge",
            crate::system_identity::SystemAspect::Limitations => "ask_about_system.limitations",
            // v4.6.5 — operational principles aspect.
            crate::system_identity::SystemAspect::Principles => "ask_about_system.principles",
            // v4.6.20 — self-comparison aspect.
            crate::system_identity::SystemAspect::SelfComparison => {
                "ask_about_system.self_comparison"
            }
            // v4.12.0 — implementation aspect (programming language /
            // stack adam is built with).
            crate::system_identity::SystemAspect::Implementation => {
                "ask_about_system.implementation"
            }
            // v4.13.5 — generic verb-capability + multi-topic
            // capability honest-fallback aspects.
            crate::system_identity::SystemAspect::GenericCapability => {
                "ask_about_system.generic_capability"
            }
            crate::system_identity::SystemAspect::MultiTopicCapability => {
                "ask_about_system.multi_topic_capability"
            }
        },
        Intent::StatementOfName { .. } => "statement_of_name",
        Intent::AskAge => "ask_age",
        Intent::StatementOfAge { .. } => "statement_of_age",
        Intent::AskLocation => "ask_location",
        Intent::StatementOfLocation { city: Some(city) } => {
            if geo_entity_kind(city)
                .as_deref()
                .is_some_and(uses_geo_feature_location_family)
            {
                "statement_of_location.geo_feature"
            } else {
                "statement_of_location"
            }
        }
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
        Intent::UserAcknowledgement => "user_acknowledgement",
        // v4.14.0 — curriculum-content honest fallback.
        Intent::AskCurriculumContent => "ask_curriculum_content",
        Intent::Unknown {
            raw_tokens,
            noun_hint,
            example,
            grounded_fact,
            example_adapted,
            reasoning_chain,
            question_shape,
            ..
        } => {
            // **v4.12.0** — `QuestionShape::Causal` short-circuits the
            // standard unknown routing. Pre-v4.12.0 «Неліктен жасуша
            // өледі?» surfaced a generic IsA fact about жасуша, which
            // is logically wrong (adam asserted "жасуша IsA X" when
            // the user asked "why does жасуша die?"). The causal
            // template family hedges honestly: offers the IsA / Has
            // context if available, then explicitly states adam
            // cannot pinpoint the cause from its dataset.
            if matches!(
                question_shape,
                Some(crate::question_shape::QuestionShape::Causal)
            ) {
                if grounded_fact.is_some() {
                    return "unknown.causal.with_fact";
                }
                if noun_hint.is_some() {
                    return "unknown.causal.bare";
                }
                // Fall through to bare unknown when no topic at all.
            }

            // User-facing chat prefers grounded evidence over a
            // reasoning-chain when both exist. This keeps the
            // deterministic kernel's derivations available, but
            // surfaces them only when we have no direct fact or safe
            // retrieval quote to say first.
            //
            // v1.9.5: adapted-evidence routing (retrieval was rewritten
            // by compose_with_city); the «бейімд-» marker fires.
            // v1.6.5+: direct fact / retrieval evidence.
            // v2.7: reasoning-chain fallback, marked with
            // «байланыс-» for auditability.
            // v1.1.0: noun-echo acknowledgement when no retrieval hit.
            // v1.0.0: bare "түсінбедім" fallback.
            match unknown_answer_mode(raw_tokens) {
                UnknownAnswerMode::Example => {
                    if example.is_some() && *example_adapted {
                        "unknown.with_adapted_evidence"
                    } else if example.is_some() {
                        "unknown.with_evidence"
                    } else if grounded_fact.is_some() {
                        "unknown.with_grounded_fact"
                    } else if reasoning_chain.is_some() {
                        "unknown.with_derived_chain"
                    } else if noun_hint.is_some() {
                        "unknown.with_noun"
                    } else {
                        "unknown"
                    }
                }
                UnknownAnswerMode::Reasoning => {
                    if reasoning_chain.is_some() {
                        "unknown.with_derived_chain"
                    } else if grounded_fact.is_some() {
                        "unknown.with_grounded_fact"
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
                UnknownAnswerMode::General => {
                    if example.is_some()
                        && unknown_prefers_quoted_example(raw_tokens)
                        && *example_adapted
                    {
                        "unknown.with_adapted_evidence"
                    } else if example.is_some() && unknown_prefers_quoted_example(raw_tokens) {
                        "unknown.with_evidence"
                    } else if grounded_fact.is_some() {
                        "unknown.with_grounded_fact"
                    } else if example.is_some() && *example_adapted {
                        "unknown.with_adapted_evidence"
                    } else if example.is_some() {
                        "unknown.with_evidence"
                    } else if reasoning_chain.is_some() {
                        "unknown.with_derived_chain"
                    } else if noun_hint.is_some() {
                        "unknown.with_noun"
                    } else {
                        "unknown"
                    }
                }
            }
        }
    }
}
