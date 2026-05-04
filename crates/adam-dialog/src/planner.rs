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
    ensure_name_respect_slot(&mut slots);
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
            // **v4.41.0** — surface Kazakh number-words alongside
            // the digit when the renderer produced output. The
            // slot value INCLUDES the leading space and parens
            // when populated (« (жүз елу)») and is empty otherwise,
            // so the same template `"{math_value}{math_words}"`
            // renders «150 (жүз елу)» for word-renderable results
            // and «1234567890» (bare) for out-of-range integers.
            let math_words_slot = extra_slots
                .get("__math_words__")
                .map(|w| format!(" ({w})"))
                .unwrap_or_default();
            slots.insert("math_words".into(), math_words_slot);
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
        // **v4.37.0** — same bypass for inversion-question. When the
        // user asks «X емес пе?», `unknown.with_inversion_question`
        // takes precedence over Conflicted/Tentative overrides — a
        // tentative template would dilute the engagement adam wants
        // to do.
        (
            Intent::Unknown {
                noun_hint: Some(_),
                input_is_inversion_question: true,
                ..
            },
            _,
        ) => None,
        // **v4.34.0** — when the user negates the topic («X емес»),
        // the `unknown.with_negated_topic` family from v4.33.5 takes
        // precedence over Conflicted/Tentative epistemic overrides.
        // Reason: the user is denying X's predicate role, not asking
        // for evidence about X. A "tentative" template that says
        // "Бәлкім, X туралы айтасыз ба" would force the user back
        // into asserting X — exactly what they just denied. Bypass
        // the override block entirely when polarity is Negated; the
        // base_key computed by plan_response_with_session
        // (`unknown.with_negated_topic`) wins.
        (
            Intent::Unknown {
                noun_hint: Some(_),
                noun_hint_polarity: adam_kernel_fst::Polarity::Negated,
                ..
            },
            _,
        ) => None,
        // **v4.34.7 + v4.35.5** — same bypass for modality. When the
        // user makes a periphrastic-modality claim, the modal-aware
        // template families take precedence over Conflicted/Tentative
        // overrides. v4.35.5 dropped the `noun_hint: Some(_)`
        // requirement to support verb-only modal claims like «Жаза
        // аламын» (battery case 21).
        (
            Intent::Unknown {
                input_modality: Some(_),
                ..
            },
            _,
        ) => None,
        // **v4.36.0** — same bypass for hearsay evidentiality.
        // When the user reports hearsay («-{Y}п(ты)» evidential
        // tense), `unknown.with_hearsay_hedge` takes precedence
        // over Conflicted/Tentative overrides — a tentative
        // template would dilute the hedging adam wants to do
        // anyway.
        (
            Intent::Unknown {
                input_evidence: Some(adam_kernel_fst::EvidenceKind::Hearsay),
                ..
            },
            _,
        ) => None,
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
    ensure_name_respect_slot(&mut slots);
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

/// **v4.18.0** — auto-derive `name_respect` from `name` when the
/// session has the latter but not the former. Direct
/// `session.insert("name", ...)` callers (tests, replay harnesses)
/// don't go through the `StatementOfName` path that also writes
/// `name_respect`, so we paper over that here. No-op when
/// `name_respect` is already present.
fn ensure_name_respect_slot(slots: &mut HashMap<String, String>) {
    if !slots.contains_key("name_respect") {
        if let Some(name) = slots.get("name").cloned() {
            let respect = crate::language_core::kazakh_respectful_address(&name)
                .unwrap_or_else(|| name.clone());
            slots.insert("name_respect".into(), respect);
        }
    }
    // **v4.18.5** — distinct slot present ONLY when the respect
    // form genuinely differs from the literal name. Templates
    // that warmly introduce both forms («Сізді {name_respect_distinct}
    // деп атаймын — қазақ дәстүрі бойынша») gate on this slot
    // so they're auto-filtered for vowel-initial names where
    // respect == literal (Абай → name_respect = Абай → no
    // distinct form, no awkward «Сізді Абай деп атаймын»
    // rendering).
    if !slots.contains_key("name_respect_distinct") {
        if let (Some(name), Some(respect)) = (slots.get("name"), slots.get("name_respect")) {
            if name != respect {
                slots.insert("name_respect_distinct".into(), respect.clone());
            }
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
            // **v4.18.0** — respectful Kazakh address form. Adam
            // is a young system; per Kazakh tradition it addresses
            // older / honoured speakers as «<first-consonant>әке»
            // (Дәулет → Дәке, Марат → Мәке) instead of by full
            // name. Vowel-initial names (Абай, Алия) fall back to
            // the literal name. Templates use `{name_respect}`
            // when set, otherwise `{name}` — see
            // `kazakh_respectful_address` for the rule.
            if let Some(respect) = crate::language_core::kazakh_respectful_address(name) {
                slots.insert("name_respect".into(), respect.clone());
                // **v4.18.5** — distinct slot only set when respect
                // form differs from literal. Used by warmth templates
                // that introduce both forms.
                slots.insert("name_respect_distinct".into(), respect);
            } else {
                slots.insert("name_respect".into(), name.clone());
                // No distinct slot for vowel-initial names — warmth
                // templates won't fire (template_is_fillable filters).
            }
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
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            input_modality: None,
            input_evidence: None,
            input_is_inversion_question: false,
            noun_hint_confidence: crate::topic_extraction::TopicConfidence::High,
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
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            input_modality: None,
            input_evidence: None,
            input_is_inversion_question: false,
            noun_hint_confidence: crate::topic_extraction::TopicConfidence::High,
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
        Intent::UserDisagrees => "disagreement_ack",
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
            // v4.18.5 — composite identity + capabilities aspect.
            crate::system_identity::SystemAspect::IntroAndCapabilities => {
                "ask_about_system.intro_and_capabilities"
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
        Intent::AskWillingness => "ask_willingness",
        Intent::Unknown {
            raw_tokens,
            noun_hint,
            example,
            grounded_fact,
            example_adapted,
            reasoning_chain,
            question_shape,
            temporal_scope,
            compositional_function,
            noun_hint_polarity,
            input_modality,
            input_evidence,
            input_is_inversion_question,
            noun_hint_confidence,
            ..
        } => {
            // **v4.37.0** — inversion-question routing. «X емес пе?»
            // is confirmation-seeking ("isn't X the case?"), NOT a
            // denial. Routing through `unknown.with_negated_topic`
            // (the v4.33.5 denial-acknowledgment family) would
            // misread the speaker's intent. Highest priority among
            // Unknown-routing because the question shape needs
            // engagement, not refusal. Fires only when both `емес`
            // AND a tag-question particle co-occur in the stream
            // (set by `Conversation::turn`).
            if *input_is_inversion_question && noun_hint.is_some() {
                return "unknown.with_inversion_question";
            }
            // **v4.33.5** — sentence-level negation routing. When
            // the user said «X емес» («X is not the case»),
            // `Conversation::turn` copies `Polarity::Negated` from
            // the matching SemFrame onto the Intent. Asserting a
            // definition of X («X — Y») would contradict the user's
            // claim, so we route to a respectful acknowledgement
            // family that doesn't assert anything new. Highest
            // priority among Unknown-routing because it overrides
            // every other "I have evidence about X" path — the
            // user is denying X's predicate role, not asking about
            // it. Default Affirmative path is unchanged: pre-v4.33.5
            // routing preserved bit-for-bit when polarity is not
            // Negated.
            if *noun_hint_polarity == adam_kernel_fst::Polarity::Negated && noun_hint.is_some() {
                return "unknown.with_negated_topic";
            }
            // **v4.34.7** — modality routing. When the user makes a
            // periphrastic-modality claim («X V керек / тиіс /
            // мүмкін» or «-а ал-» ability), `Conversation::turn`
            // copies the Modality from the lexical-verb SemFrame
            // onto the Intent. Asserting a generic fact about
            // noun_hint doesn't engage with the user's modal claim.
            // Route to per-modality template families so the response
            // can acknowledge the specific modal kind appropriately:
            // Necessity → "иә, V-у пайдалы"; Possibility → "мүмкін
            // екен"; Ability → "жақсы екен, сіздің мүмкіндігіңізді
            // түсіндім". Runs AFTER polarity check (negation has
            // higher priority — when both fire on rare edge case
            // «X V керек емес», negation is the more salient signal).
            // **v4.35.5** — relaxed `noun_hint.is_some()` requirement.
            // Pre-v4.35.5 the routing required BOTH modality AND
            // noun_hint, so verb-only modal claims like «Жаза аламын»
            // (battery case 21) fell to "Түсінбедім" because no
            // content noun was extracted. With the requirement
            // dropped, modality alone is enough to route. Templates
            // in each family include both noun-bearing AND no-noun
            // variants; `template_is_fillable` filters to the
            // applicable subset based on whether `{noun}` slot has
            // a value.
            if let Some(modality) = input_modality {
                // **v4.41.7** — explain/teach skip. When the user
                // asks «can you teach/explain X?» («Маған Rust-ты
                // үйрете аласыз ба?» / «Оның жұмысын түсіндіріп
                // бере аласыз ба?»), the Ability modality fires on
                // the auxiliary («аласыз») but the question shape
                // is asking for *content*, not asking adam whether
                // it has a capability in the abstract. The
                // GenericCapability path already exempts these
                // verbs (semantics.rs aux_capability gate); modal-
                // ability template would hedge with «иә, мүмкіндік
                // бар екен» (treating the user as having the
                // ability), which is the wrong direction. Skip the
                // override when an explain/teach verb is in the
                // raw input AND a noun_hint with grounded fact
                // exists — let the response surface the curated
                // fact about the topic instead.
                let has_explain_teach = raw_tokens.iter().any(|t| {
                    let lower = t.to_lowercase();
                    lower.starts_with("түсіндір")
                        || lower.starts_with("үйрет")
                        || lower.starts_with("баянда")
                });
                let has_grounded_fact = matches!(
                    intent,
                    Intent::Unknown {
                        grounded_fact: Some(_),
                        ..
                    }
                );
                if !(has_explain_teach && has_grounded_fact) {
                    return match modality {
                        adam_kernel_fst::Modality::Necessity => "unknown.with_modal_necessity",
                        adam_kernel_fst::Modality::Possibility => "unknown.with_modal_possibility",
                        adam_kernel_fst::Modality::Ability => "unknown.with_modal_ability",
                    };
                }
            }
            // **v4.36.0** — evidentiality routing. When the user
            // uses past-evidential tense («-{Y}п(ты)» / болыпты /
            // деген екен), `populate_*` chain marks the verb frame
            // with `evidence: Some(EvidenceKind::Hearsay)`. The
            // user is reporting hearsay, not asserting first-hand
            // knowledge — adam should hedge in response, marking
            // its uncertainty about whether the reported claim is
            // grounded. Routes BEFORE evidence-shaped families
            // (with_grounded_fact, with_evidence) because Hearsay
            // marking dominates: even if adam has a grounded fact
            // about the topic, surfacing it as a flat assertion
            // misses the user's evidential framing. Runs AFTER
            // polarity + modality checks (those have higher
            // priority — when both fire, the more salient signal
            // wins).
            if matches!(input_evidence, Some(adam_kernel_fst::EvidenceKind::Hearsay)) {
                return "unknown.with_hearsay_hedge";
            }
            // **v4.23.0** — `temporal_scope: true` short-circuits to
            // `unknown.temporal_no_data`. Pattern: temporal adverb
            // (кеше / бүгін / ертең / қазір / бұрын / былтыр /
            // келесі) co-occurring with a question marker — adam
            // has no time-series data for state-at-a-time queries.
            // Honest fallback says so explicitly instead of letting
            // topic extraction fall through to a tangential general
            // fact about the non-temporal subject. Routes BEFORE
            // the v4.12.0 causal short-circuit so a temporal-causal
            // composite («Неліктен кеше...») still routes here, on
            // the principle that "no time data" is a stronger
            // negative than "no causal data".
            if *temporal_scope {
                return "unknown.temporal_no_data";
            }
            // **v4.23.5** — `compositional_function: true` short-
            // circuits to `unknown.compositional_function.*`.
            // Pattern: `X-Genitive Y-Possessive + function-asking
            // phrase`. world_core typically has only structural
            // (PartOf / IsA) facts about Y; the user is asking
            // about FUNCTION. Honest hedge: surface the structural
            // fact when available, but explicitly state we don't
            // have functional data. Same precedence policy as
            // temporal_scope — runs BEFORE the v4.12.0 causal
            // short-circuit.
            if *compositional_function {
                if grounded_fact.is_some() {
                    return "unknown.compositional_function.with_fact";
                }
                if noun_hint.is_some() {
                    return "unknown.compositional_function.bare";
                }
                // Fall through to bare unknown when no topic at all.
            }
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

            // **v4.37.5** — human-like clarification fork. Runs AFTER
            // every user-intent special route (negation / modality /
            // evidence / temporal / compositional / causal) but
            // BEFORE the standard fact-asserting paths.
            //
            // Routing rules:
            //   1. `Low` confidence → ALWAYS clarify, even when the
            //      retrieval/curated-graph paths produced "evidence".
            //      A corpus citation about an adjective or deverbal
            //      participle the user clearly meant as a *modifier*
            //      (e.g. «атақты» / «шыққан») is noise, not signal —
            //      surfacing it confidently misleads. Better to echo
            //      the candidate interpretation and ASK the user.
            //   2. `noun_hint == None` AND no evidence → clarify
            //      entirely (replaces bare «Түсінбедім»).
            //   3. Otherwise — fall through to the standard path
            //      (every pre-v4.37.5 route preserved bit-for-bit
            //      because confidence defaults to High when no field
            //      is set).
            //
            // Surfaced by the 2026-05-03 live REPL transcript:
            //   «Қазақстанның **атақты** жазушыларын атаңыз»
            //     pre-v4.37.5:  unknown.with_evidence (corpus quote
            //                   about Astana — tangential)
            //     post-v4.37.5: confidence=Low → clarify_low_confidence,
            //                   echoes the candidate, asks the user.
            //   «Қазақстаннан **шыққан** танымал тұлғалар…»
            //     pre-v4.37.5:  noun_hint=шыққан, confident citation
            //     post-v4.37.5: deverbal-participle demotion → Low →
            //                   clarify, echoes «тұлға» (deeper noun
            //                   from the parse stream as fallback) or
            //                   «шыққан» if тұлға wasn't reached.
            if matches!(
                noun_hint_confidence,
                crate::topic_extraction::TopicConfidence::Low
            ) {
                return "unknown.clarify_low_confidence";
            }
            let no_evidence =
                example.is_none() && grounded_fact.is_none() && reasoning_chain.is_none();
            if no_evidence && noun_hint.is_none() {
                return "unknown.clarify_no_topic";
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
