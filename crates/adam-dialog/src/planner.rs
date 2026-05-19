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

    let mut key = intent_key(intent);
    trace.push(format!("planner: template_key={key}"));

    // Merge per-turn slots with persistent session entities. Per-turn
    // wins on collision.
    let mut slots = session.clone();
    for (k, v) in extract_slots_with_session(intent, session) {
        slots.insert(k, v);
    }
    ensure_geo_kind_slot(&mut slots);
    ensure_name_respect_slot(&mut slots);
    // **v5.6.6 — Codex follow-up review.** SubmitSolution lesson-
    // context inheritance. The v4.95.5 fallback in `extract_slots`
    // tried to read `last_exercise_topic` from its local slot map,
    // but that map is built fresh from the Intent — the session
    // wasn't visible. Do the resolution HERE instead, where `slots`
    // is the session-merged map, so a passing snippet after an
    // `AskExercise { topic: ownership }` turn inherits "ownership"
    // as the lesson topic instead of "println" (filtered out at
    // detection time per the v5.6.6 detect_submit_solution change)
    // or empty (which falls to a topic-less template variant).
    if matches!(intent, Intent::SubmitSolution { topic: None, .. }) && !slots.contains_key("topic")
    {
        if let Some(t) = slots.get("last_exercise_topic").cloned() {
            if !t.is_empty() {
                slots.insert("topic".into(), t);
            }
        }
    }

    // **v4.95.0** — SubmitSolution sub-routing. After extract_slots
    // ran cargo_verify and populated cargo_status, switch the
    // template key to the matching sub-family. Done here (not in
    // intent_key) because intent_key runs before extract_slots and
    // doesn't see the verifier outcome.
    if matches!(intent, Intent::SubmitSolution { .. }) {
        key = match slots.get("cargo_status").map(String::as_str) {
            // **v4.98.5** — auto-advance routing (mirror of
            // plan_response_with_epistemic). Fires only when the
            // caller pre-stuffed curriculum-closure context.
            Some("passed") if slots.contains_key("__stage_closes__") => {
                if slots.contains_key("__curriculum_complete__") {
                    "submit_solution.passed_curriculum_complete"
                } else {
                    "submit_solution.passed_stage_closed"
                }
            }
            Some("passed") => "submit_solution.passed",
            Some("failed") if slots.contains_key("error_explanation") => {
                "submit_solution.failed_known"
            }
            Some("failed") => "submit_solution.failed_unknown",
            Some("env_error") => "submit_solution.env_error",
            _ => "submit_solution.env_error",
        };
        trace.push(format!("planner: SubmitSolution sub-key → {key}"));
    }
    // **v6.0** — AskWeather sub-routing. Same pattern as
    // AskExercise / CodeRequest below: when the sentinel
    // `__live_weather_set__` is present, the weather provider
    // returned a fresh reading and the dedicated `.live` family
    // (single-template) must fire. Without it the seed-mod picker
    // rolls between the live answer and the honest-refusal variant
    // ~50 % of turns.
    if matches!(intent, Intent::AskWeather) && slots.contains_key("__live_weather_set__") {
        key = "ask_weather.live";
        trace.push(format!("planner: AskWeather sub-key → {key}"));
    }
    // **v4.96.0** — Codex round-2 audit Bug 2 fix. Pedagogical
    // sub-routing (mirror of plan_response_with_epistemic block).
    // Pre-fix `template_is_fillable` accepted both topic-bearing
    // and clarification variants in one family, so 40 % of seeds
    // routed wrong. Now route by slot presence.
    if matches!(intent, Intent::AskExercise { .. }) {
        key = if slots.contains_key("topic") && slots.contains_key("exercise_body") {
            "ask_exercise.with_topic"
        } else {
            "ask_exercise.no_topic"
        };
        trace.push(format!("planner: AskExercise sub-key → {key}"));
    } else if matches!(intent, Intent::CodeRequest { .. }) {
        key = if slots.contains_key("topic") && slots.contains_key("code_snippet") {
            "code_request.with_topic"
        } else {
            "code_request.no_topic"
        };
        trace.push(format!("planner: CodeRequest sub-key → {key}"));
    } else if matches!(intent, Intent::ExplainCompilerError { .. }) {
        key = if slots.contains_key("error_explanation") {
            "explain_compiler_error.with_explanation"
        } else {
            "explain_compiler_error.no_explanation"
        };
        trace.push(format!("planner: ExplainCompilerError sub-key → {key}"));
    } else if matches!(intent, Intent::AskPurpose { .. }) {
        key = if slots.contains_key("topic") && slots.contains_key("purpose_body") {
            "ask_purpose.with_topic"
        } else {
            "ask_purpose.no_topic"
        };
        trace.push(format!("planner: AskPurpose sub-key → {key}"));
    } else if matches!(intent, Intent::CrossLanguageContrast { .. }) {
        key = if slots.contains_key("contrast_body") {
            "cross_language_contrast.with_body"
        } else {
            "cross_language_contrast.no_body"
        };
        trace.push(format!("planner: CrossLanguageContrast sub-key → {key}"));
    } else if matches!(intent, Intent::AskNextTopic) {
        key = if slots.contains_key("__curriculum_complete__") {
            "next_topic.complete"
        } else {
            "next_topic.suggestion"
        };
        trace.push(format!("planner: AskNextTopic sub-key → {key}"));
    } else if matches!(intent, Intent::AskCurrentProgress) {
        key = if slots.contains_key("__progress_empty__") {
            "current_progress.empty"
        } else {
            "current_progress.recap"
        };
        trace.push(format!("planner: AskCurrentProgress sub-key → {key}"));
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

    apply_introducer_migration(key, rng_seed, &mut slots, &mut trace);

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
    // **v5.4.0** — bare yes/no IsA short-circuit. When the input parses
    // as «<X> — <Y> Q?» the conversation layer walks the IsA chain
    // (`find_isa_chain` over extracted + derived facts) and writes the
    // outcome to `__yes_no_isa__` / `__yes_no_outcome__`. Routing here
    // makes the chain query the primary response shape — pre-v5.4.0
    // these questions fell through to standard `Unknown` routing and
    // surfaced the most-central IsA fact about <X>, ignoring <Y>. The
    // bridge facts shipped in v5.4.0 (`data/world_core/{life,concept}_
    // bridges.jsonl`) made transitive paths reachable for these
    // queries; the wiring here is what surfaces them.
    // **v5.11.5 — Codex follow-up review (B5.1).** Yield to political-
    // safety / safety-refusal overrides when both sentinels are set.
    // Pre-v5.11.5 «Министр тиімді ме?» (1) installed `__yes_no_isa__`
    // because the question matches the bare-IsA shape AND (2) installed
    // `__political_safety__` via the new evaluative-question detector —
    // but yes_no_isa was checked first and won, surfacing «no chain
    // found» honest unknown instead of the political refusal Codex
    // expects. Yielding lets the political route claim the turn.
    if extra_slots.contains_key("__yes_no_isa__")
        && !extra_slots.contains_key("__political_safety__")
        && !extra_slots.contains_key("__safety_refusal__")
    {
        let outcome = extra_slots
            .get("__yes_no_outcome__")
            .map(String::as_str)
            .unwrap_or("unknown");
        let key = match outcome {
            "confirm" => "unknown.yes_no_check.confirm",
            "deny" => "unknown.yes_no_check.deny",
            _ => "unknown.yes_no_check.unknown",
        };
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: yes_no_isa override → {key}"));
            let applicable_all = repo.get(key);
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
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
                "planner: applicable_total={} chosen_index={} text='{}'",
                effective.len(),
                idx,
                chosen,
            ));
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    if extra_slots.contains_key("__dismiss_contradiction__") {
        let key = "dismiss_contradiction";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: dismiss_contradiction override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v5.3.0** — sister sentinel for the "user picked one of the
    // contested values" path (e.g. «Жоқ, Алматы дұрыс»). The belief
    // layer already resolved (active=1=Алматы); we route through a
    // resolution-acceptance template with the chosen profile slot.
    if extra_slots.contains_key("__resolve_contradiction__") {
        let key = "resolve_contradiction";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: resolve_contradiction override → {key}"));
            let applicable_all = repo.get(key);
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
            // Prefer templates whose all-slot placeholders are filled.
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
                "planner: applicable_total={} chosen_index={} text='{}'",
                effective.len(),
                idx,
                chosen,
            ));
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
    // **v5.18.1 — adversarial D2a ma_14 closure.** Explicit
    // division-by-zero in input → dedicated math-education
    // message «нөлге бөлуге болмайды». Highest priority among
    // math-related overrides because it's a concrete teachable
    // moment, not a generic «I don't process» hedge.
    if extra_slots.contains_key("__div_by_zero__") {
        let key = "math_refusal.div_by_zero";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: div_by_zero override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v5.18.1 — adversarial D2a word-problem routing.** If
    // `Conversation::turn` set `__word_problem__` via
    // `discourse::is_kazakh_word_problem`, route to the dedicated
    // honest-refusal family BEFORE topic-extraction (which lives
    // inside the `Unknown` branch deep in `intent_key`-driven
    // fallback) surfaces a tangential proverb or definition of a
    // random noun in the question. Closes wp_01/03/05/06/07/09/10
    // routing misses from v5.18.0 benchmark (wp_08 hallucination
    // closed separately by the binary-operator guard in
    // `discourse::try_evaluate_arithmetic`).
    if extra_slots.contains_key("__word_problem__") {
        let key = "unknown.word_problem";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: word_problem override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    if extra_slots.contains_key("__check_contradiction__") {
        let key = "check_contradiction";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: check_contradiction override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v4.76.5 / v4.77.0** — compare_topics routing. Conversation::turn
    // populates `__compare_x__` + `__compare_y__` slots when the input
    // matched «X пен Y айырмашылығы». **v4.77.0** — dual-retrieval
    // also populates `__compare_x_def__` / `__compare_y_def__` when
    // both definitions found in extracted_facts. Picks
    // `compare_topics.dual` (full side-by-side) when both defs
    // present, else falls back to `compare_topics.hedge` (honest
    // refusal naming both topics). Mutually exclusive with
    // math/check_answer/explain_steps (gated upstream).
    if let (Some(x), Some(y)) = (
        extra_slots.get("__compare_x__"),
        extra_slots.get("__compare_y__"),
    ) {
        let x_def = extra_slots.get("__compare_x_def__");
        let y_def = extra_slots.get("__compare_y_def__");
        let key = if x_def.is_some() && y_def.is_some() {
            "compare_topics.dual"
        } else {
            "compare_topics.hedge"
        };
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: compare_topics override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
            slots.insert("compare_x".into(), x.clone());
            slots.insert("compare_y".into(), y.clone());
            if let Some(d) = x_def {
                slots.insert("compare_x_def".into(), d.clone());
            }
            if let Some(d) = y_def {
                slots.insert("compare_y_def".into(), d.clone());
            }
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    // **v4.76.0** — explain_steps routing. Surfaces stored step
    // narrative when user asked «Қалай шештің?» / «Процесін көрсет»
    // / «Қадам-қадаммен» after a prior solve. Routes BEFORE
    // check_answer because explain_steps doesn't carry a `var=N`
    // token; the two slots are mutually exclusive in practice.
    if let Some(steps) = extra_slots.get("__explain_steps__") {
        let key = "explain_steps";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: explain_steps override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
            slots.insert("explain_steps".into(), steps.clone());
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    // **v4.75.5** — check_answer routing. Conversation::turn populates
    // `__check_answer_correct__` (1/0) when user submitted an answer
    // for verification («Жауабымды тексер: x=4»). Routes to
    // `check_answer.correct` or `check_answer.incorrect` template
    // family BEFORE the math_answer path, since the check_answer
    // input may also have a `var=N` token that the linear-equation
    // solver could mis-handle as a fresh equation.
    if let Some(correct_flag) = extra_slots.get("__check_answer_correct__") {
        let key = if correct_flag == "1" {
            "check_answer.correct"
        } else {
            "check_answer.incorrect"
        };
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: check_answer override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
            if let Some(uv) = extra_slots.get("__check_answer_user_value__") {
                slots.insert("check_user_value".into(), uv.clone());
            }
            if let Some(cv) = extra_slots.get("__check_answer_correct_value__") {
                slots.insert("check_correct_value".into(), cv.clone());
            }
            if let Some(uk) = extra_slots.get("__check_answer_unknown__") {
                slots.insert("check_unknown".into(), uk.clone());
            }
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    if let Some(value) = extra_slots.get("__math_answer__") {
        let key = "math_answer";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: math_answer override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v5.6.6 — Codex follow-up review.** AskPreviousError recall.
    // Pre-v5.6.6 «Ал алдыңғы қате неде болды?» fell to retrieval on
    // «болд». New override consults session state populated by a
    // prior failed SubmitSolution turn and routes to a dedicated
    // template family.
    if let Some(mode) = extra_slots.get("__ask_previous_error__") {
        let key = format!("ask_previous_error.{mode}");
        if !repo.get(&key).is_empty() {
            trace.push(format!("planner: ask_previous_error override → {key}"));
            let applicable_all = repo.get(&key);
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
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
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    // **v5.10.0 — Codex follow-up review (B3).** AskFixPreviousError
    // override. Routes to `ask_fix_previous_error.<error_code>` if a
    // per-code specialised family exists in the repo (E0382 / E0277 /
    // E0308 / E0596 / etc.); otherwise falls through to
    // `ask_fix_previous_error.with_data` (generic carry of cached
    // explanation + Rust error-index pointer); on `empty` mode
    // routes to `ask_fix_previous_error.empty` for the no-context
    // honest refusal.
    // **v5.11.0 — Codex follow-up review (B4.2).** Epistemic refusal.
    // Routes BEFORE every factual path so the user's explicit
    // request to fabricate an unsupported claim is met with an
    // honest "I cannot do this" rather than a retrieval-flavoured
    // surface.
    if extra_slots.contains_key("__epistemic_refusal__") {
        let key = "epistemic_refusal";
        if repo.has_key(key) {
            trace.push(format!("planner: epistemic_refusal override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v5.11.0 — Codex follow-up review (B4.2).** Countable
    // propositions override. Routes to one of three variants:
    // - `propositions.with_data` — N provable facts found, surface
    //   the rendered sentence sequence
    // - `propositions.honest` — only M < N provable; honest about
    //   the count gap, then surface what's available
    // - `propositions.empty` — nothing provable for the subject
    if let Some(mode) = extra_slots.get("__propositions__") {
        let key = format!("propositions.{mode}");
        if repo.has_key(&key) {
            trace.push(format!("planner: propositions override → {key}"));
            let applicable_all = repo.get(&key);
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
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
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    // **v5.10.5 — Codex follow-up review (B4.1).** Proof-chain
    // override. Routes to `proof_chain.with_data` when the chain was
    // resolved (slots `subject_term` / `predicate_term` / `chain` set
    // by Conversation::turn_with_trace), or to `proof_chain.empty`
    // for the honest "no derivation chain available" refusal. The
    // surface mirrors the IR shape produced by
    // `answer_ir::compose_isa_proof_chain`: «Дәлелдейік: <subject> —
    // <predicate>. Тізбек: <chain>. Сондықтан <subject> —
    // <predicate>.»
    if let Some(mode) = extra_slots.get("__proof_chain__") {
        let key = format!("proof_chain.{mode}");
        if repo.has_key(&key) {
            trace.push(format!("planner: proof_chain override → {key}"));
            let applicable_all = repo.get(&key);
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
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
            return ResponsePlan {
                literal: chosen,
                slots,
                trace,
            };
        }
    }
    if let Some(mode) = extra_slots.get("__ask_fix_previous_error__") {
        let mut candidates: Vec<String> = Vec::new();
        if mode != "empty" {
            // **v5.12.5 — Codex follow-up review (B5.3).** Per-error-
            // code corrected-code family is preferred when present —
            // it carries a runnable repaired snippet (clone vs
            // reference for E0382, `let mut` vs `&mut` for E0596,
            // `as` cast vs `String::from` for E0308, `derive(Debug)`
            // vs `impl Trait` for E0277). Pre-v5.12.5 the kernel
            // had only the generic explanation family which Codex's
            // audit flagged as repeating the same advice on
            // follow-ups instead of producing concrete fix examples.
            candidates.push(format!("ask_fix_previous_error.with_corrected_code.{mode}"));
            candidates.push(format!("ask_fix_previous_error.{mode}"));
            candidates.push("ask_fix_previous_error.with_data".to_string());
        } else {
            candidates.push("ask_fix_previous_error.empty".to_string());
        }
        for key in &candidates {
            if repo.has_key(key) {
                trace.push(format!("planner: ask_fix_previous_error override → {key}"));
                let applicable_all = repo.get(key);
                let mut slots = session.clone();
                for (k, v) in extra_slots {
                    if !k.starts_with("__") {
                        slots.insert(k.clone(), v.clone());
                    }
                }
                let fillable: Vec<&String> = applicable_all
                    .iter()
                    .filter(|t| template_is_fillable(t, &slots))
                    .collect();
                let effective: Vec<&String> = if fillable.is_empty() {
                    applicable_all.iter().collect()
                } else {
                    fillable
                };
                // **v5.12.5 — Codex follow-up review (B5.3).** Pick
                // variant by `fix_example_idx` (session-tracked
                // counter) instead of rng_seed: «Тағы бір мысал бер»
                // / «Басқа нұсқа» rotates through repair variants
                // deterministically. Falls back to rng_seed when no
                // counter is set (other ask_fix_previous_error modes).
                let idx_source: usize = extra_slots
                    .get("fix_example_idx")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(rng_seed as usize);
                let idx = idx_source % effective.len().max(1);
                let chosen = effective.get(idx).map(|s| (*s).clone()).unwrap_or_default();
                return ResponsePlan {
                    literal: chosen,
                    slots,
                    trace,
                };
            }
        }
    }
    // **v5.6.5 — Codex 2026-05-09 review.** High-stakes-topic safety
    // refusal (medical / legal / financial / current-data). Routes to
    // dedicated `safety_refusal.<category>` family BEFORE all factual
    // paths. Pre-v5.6.5 a medical query like «Басым ауырып тұр, қандай
    // дәрі ішейін?» surfaced the curated noun fact «Бас — дене бөлігі»
    // — source-backed but product-dangerous misroute. Honest refusal
    // points the user at a qualified specialist.
    if let Some(category) = extra_slots.get("__safety_refusal__") {
        let key = format!("safety_refusal.{category}");
        if !repo.get(&key).is_empty() {
            trace.push(format!("planner: safety_refusal override → {key}"));
            let applicable_all = repo.get(&key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v4.78.0** — political-safety refusal (Codex round-3 Bug 3).
    // Routes to dedicated `political_safety` family BEFORE all
    // factual paths. adam doesn't give partisan recommendations.
    if extra_slots.contains_key("__political_safety__") {
        let key = "political_safety";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: political_safety override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v5.9.5 — Codex follow-up review (B1).** AskLocation user-self
    // disambiguation. When the conversation layer detected a 1sg
    // self-recall location query AND no city is in session, route
    // to the new `ask_location.user_self.no_data` family which
    // honestly says "I don't know your city yet" rather than the
    // assistant-self fallbacks («Мен сандық әлемде тұрамын» / «Менің
    // мекенім жоқ» / «Қазақстан елімде» — those answer about adam,
    // not the user, and the third makes a false claim).
    if extra_slots.contains_key("__user_self_no_city__") {
        let key = "ask_location.user_self.no_data";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: user_self_no_city override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v4.77.0** — code-snippet refusal. Routes to dedicated
    // `code_refusal` family BEFORE math_refusal so Python-style
    // code «for i in range(3): print(i)» doesn't fall to «can't
    // compute arithmetic».
    //
    // **v4.95.0** — SubmitSolution overrides the refusal: when the
    // intent is SubmitSolution we *can* run cargo_verify on the
    // code, so the refusal no longer applies. Without this guard
    // the dialog stays at «I recognise code but don't execute it»
    // forever — the cargo-check loop never reaches the user.
    if extra_slots.contains_key("__code_input__")
        && !matches!(intent, Intent::SubmitSolution { .. })
    {
        let key = "code_refusal";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: code_refusal override → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
    // **v5.21.0 — math echo specificity.** When the evaluator could
    // not compute but `extract_kazakh_math_summary` recognised
    // numbers + operators, prefer the echo template that shows the
    // user exactly what was parsed. Routes BEFORE the generic
    // `math_refusal` family so users see «56*7/3 деп ұқтым» instead
    // of «Санақ-есептеу әлі мүмкіндігімде жоқ». The `{partial}` slot
    // is filled by `render_math_summary_as_arithmetic` upstream.
    if extra_slots.contains_key("__math_partial_summary__") {
        let key = "math_refusal.with_understood";
        if !repo.get(key).is_empty() {
            trace.push(format!("planner: math_partial echo → {key}"));
            let applicable_all = repo.get(key);
            let idx = (rng_seed as usize) % applicable_all.len().max(1);
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
            let mut slots = session.clone();
            for (k, v) in extra_slots {
                if !k.starts_with("__") {
                    slots.insert(k.clone(), v.clone());
                }
            }
            // Make the partial summary available to the template as
            // `{partial}` without the `__` prefix.
            if let Some(summary) = extra_slots.get("__math_partial_summary__") {
                slots.insert("partial".into(), summary.clone());
            }
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
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
            let chosen = applicable_all.get(idx).cloned().unwrap_or_default();
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
        // **v5.24.0 — Codex 2026-05-12 audit bug 4.** Self-asked
        // occupation query with no stored profile: route to honest
        // unknown-user family instead of bare ask_occupation (which
        // answers about adam's identity — self/other confusion).
        // The conversation layer sets `__self_unknown_profile__` when
        // 1sg morphology is detected and no slot exists.
        (Intent::AskOccupation, _)
            if extra_slots
                .get("__self_unknown_profile__")
                .map(|s| s == "occupation")
                .unwrap_or(false) =>
        {
            Some("ask_occupation.unknown_user")
        }
        // **v4.51.0** — companion known-user override for
        // `Intent::AskActivity`. Surfaces the stored activity slot.
        (Intent::AskActivity, _) if session.contains_key("activity") => {
            Some("ask_activity.with_known_user")
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
    // **v4.53.5** — Context-aware clarify. When the standard Unknown
    // routing decided to clarify (`clarify_no_topic` / `clarify_low_confidence`)
    // BUT the session has stored profile slots, surface a diagnostic
    // that cites the stored context instead of a generic catch-all.
    //
    // Real-REPL session 5 surfaced the gap: after the user said
    // «Менің атым Дәулет, мен бағдарламашымын ... жасанды интеллект
    // моделімді жасап жатырмын» (storing name + occupation + activity),
    // a follow-up vague query produced «Сұрағыңызды толық түсінбедім.
    // Қандай тақырып туралы сұрап отырсыз?» — generic and ignores all
    // three stored slots. The user's complaint: "ответы должны вытекать
    // из контекста беседы, а не общий ответ на все, что не знает".
    //
    // Fix: when key is one of the clarify-* keys AND any of name /
    // occupation / activity / city is set in the session, override to
    // `unknown.with_session_diagnostic`. The new family's templates
    // gate on slot subsets via `template_is_fillable`, so the variant
    // chosen automatically reflects what's actually known.
    let is_clarify_key =
        key == "unknown.clarify_no_topic" || key == "unknown.clarify_low_confidence";
    let has_session_context = session.contains_key("name")
        || session.contains_key("occupation")
        || session.contains_key("activity")
        || session.contains_key("city");
    // **v5.21.5 — universal raw-input echo.** Prefer the transparent-
    // refusal family that echoes the verbatim user input over the
    // generic session-diagnostic clarify when `Conversation::turn`
    // set the `__user_input_echo__` extra-slot. The slot is only
    // populated when `discourse::safe_echo_input` confirms the input
    // is safe to quote (length 3-60, ≥70 % Cyrillic, no digits / URLs /
    // backticks / @). Highest precedence among clarify-* overrides:
    // showing the user adam's read of their input gives more signal
    // than echoing session state alone.
    let has_raw_echo = extra_slots.contains_key("__user_input_echo__");
    let key = if is_clarify_key && has_raw_echo && !repo.get("unknown.with_raw_echo").is_empty() {
        trace.push(format!(
            "planner: clarify_diagnostic override → unknown.with_raw_echo (was {key})",
        ));
        "unknown.with_raw_echo"
    } else if is_clarify_key
        && has_session_context
        && !repo.get("unknown.with_session_diagnostic").is_empty()
    {
        trace.push(format!(
            "planner: clarify_diagnostic override → unknown.with_session_diagnostic (was {key}; session_slots: name={} occupation={} activity={} city={})",
            session.contains_key("name"),
            session.contains_key("occupation"),
            session.contains_key("activity"),
            session.contains_key("city"),
        ));
        "unknown.with_session_diagnostic"
    } else {
        key
    };

    let mut slots = session.clone();
    for (k, v) in extract_slots_with_session(intent, session) {
        slots.insert(k, v);
    }
    // Per-turn extras (e.g. conflict predicate / old_value /
    // new_value) take precedence over session + intent-extracted
    // slots since they describe this specific turn's state.
    for (k, v) in extra_slots {
        slots.insert(k.clone(), v.clone());
    }
    // **v5.6.6 — Codex follow-up review.** SubmitSolution lesson-
    // context inheritance. The v4.95.5 fallback in `extract_slots`
    // tried to read `last_exercise_topic` from its local slot map,
    // but that map is built fresh from the Intent — the session
    // wasn't visible there. Do the resolution HERE, where `slots` is
    // the session-merged map (session + extract_slots + extra_slots),
    // so a passing snippet after an `AskExercise { topic: ownership }`
    // turn inherits "ownership" as the lesson topic instead of
    // "println" (filtered out at detection time per the v5.6.6
    // detect_submit_solution change) or empty (which falls to a
    // topic-less template variant).
    if matches!(intent, Intent::SubmitSolution { topic: None, .. }) && !slots.contains_key("topic")
    {
        if let Some(t) = slots.get("last_exercise_topic").cloned() {
            if !t.is_empty() {
                slots.insert("topic".into(), t);
            }
        }
    }

    // **v4.95.0** — SubmitSolution sub-routing. extract_slots ran
    // cargo_verify and populated cargo_status; switch the template
    // key to the matching sub-family. Done here (after slot
    // extraction) because the sub-key depends on the verifier
    // outcome, which only materialises in extract_slots.
    //
    // **v4.96.0** — Codex round-2 audit Bug 2 fix: same sub-routing
    // pattern for ask_exercise / code_request / explain_compiler_error
    // / ask_purpose. Pre-fix `template_is_fillable` accepted both
    // topic-bearing AND clarification variants in one family, so
    // 40 % of seeds picked the clarification template even when a
    // topic was set. Now each pedagogical family is split into
    // `.with_*` / `.no_*` sub-families and routed by slot presence.
    let key: &str = if matches!(intent, Intent::SubmitSolution { .. }) {
        let new_key = match slots.get("cargo_status").map(String::as_str) {
            // **v4.98.5** — when a `passed` verdict closes the
            // current curriculum stage, route to the auto-advance
            // sub-family. Conversation pre-stuffs `__stage_closes__`
            // and `next_stage_*` (or `__curriculum_complete__`)
            // before planning, so this branch only fires when the
            // session genuinely has curriculum context.
            Some("passed") if slots.contains_key("__stage_closes__") => {
                if slots.contains_key("__curriculum_complete__") {
                    "submit_solution.passed_curriculum_complete"
                } else {
                    "submit_solution.passed_stage_closed"
                }
            }
            Some("passed") => "submit_solution.passed",
            Some("failed") if slots.contains_key("error_explanation") => {
                "submit_solution.failed_known"
            }
            Some("failed") => "submit_solution.failed_unknown",
            Some("env_error") => "submit_solution.env_error",
            _ => "submit_solution.env_error",
        };
        trace.push(format!("planner: SubmitSolution sub-key → {new_key}"));
        new_key
    } else if matches!(intent, Intent::AskWeather) && slots.contains_key("__live_weather_set__") {
        let new_key = "ask_weather.live";
        trace.push(format!("planner: AskWeather sub-key → {new_key}"));
        new_key
    } else if matches!(intent, Intent::AskExercise { .. }) {
        let new_key = if slots.contains_key("topic") && slots.contains_key("exercise_body") {
            "ask_exercise.with_topic"
        } else {
            "ask_exercise.no_topic"
        };
        trace.push(format!("planner: AskExercise sub-key → {new_key}"));
        new_key
    } else if matches!(intent, Intent::CodeRequest { .. }) {
        let new_key = if slots.contains_key("topic") && slots.contains_key("code_snippet") {
            "code_request.with_topic"
        } else {
            "code_request.no_topic"
        };
        trace.push(format!("planner: CodeRequest sub-key → {new_key}"));
        new_key
    } else if matches!(intent, Intent::ExplainCompilerError { .. }) {
        let new_key = if slots.contains_key("error_explanation") {
            "explain_compiler_error.with_explanation"
        } else {
            "explain_compiler_error.no_explanation"
        };
        trace.push(format!("planner: ExplainCompilerError sub-key → {new_key}"));
        new_key
    } else if matches!(intent, Intent::AskPurpose { .. }) {
        let new_key = if slots.contains_key("topic") && slots.contains_key("purpose_body") {
            "ask_purpose.with_topic"
        } else {
            "ask_purpose.no_topic"
        };
        trace.push(format!("planner: AskPurpose sub-key → {new_key}"));
        new_key
    } else if matches!(intent, Intent::CrossLanguageContrast { .. }) {
        let new_key = if slots.contains_key("contrast_body") {
            "cross_language_contrast.with_body"
        } else {
            "cross_language_contrast.no_body"
        };
        trace.push(format!(
            "planner: CrossLanguageContrast sub-key → {new_key}"
        ));
        new_key
    } else if matches!(intent, Intent::AskNextTopic) {
        let new_key = if slots.contains_key("__curriculum_complete__") {
            "next_topic.complete"
        } else {
            "next_topic.suggestion"
        };
        trace.push(format!("planner: AskNextTopic sub-key → {new_key}"));
        new_key
    } else if matches!(intent, Intent::AskCurrentProgress) {
        let new_key = if slots.contains_key("__progress_empty__") {
            "current_progress.empty"
        } else {
            "current_progress.recap"
        };
        trace.push(format!("planner: AskCurrentProgress sub-key → {new_key}"));
        new_key
    } else {
        key
    };
    trace.push(format!("planner: template_key={key}"));
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

    apply_introducer_migration(key, rng_seed, &mut slots, &mut trace);

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
fn extract_slots_with_session(
    intent: &Intent,
    session: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut slots = HashMap::new();
    // **v6.0** — live OS-clock readout for `Intent::AskTime`. The
    // slot `live_clock_answer` carries the Kazakh-formatted answer
    // for the requested aspect (time / date / weekday / month /
    // year / datetime). The `ask_time` template family is now a
    // single `{live_clock_answer}` placeholder; pre-v6.0 the family
    // hard-coded a refusal regardless of variant.
    if let Intent::AskTime { aspect } = intent {
        slots.insert(
            "live_clock_answer".into(),
            crate::system_clock::render_live(*aspect),
        );
    }
    // **v6.0** — live Open-Meteo weather readout for
    // `Intent::AskWeather`. Resolved via env (`ADAM_WEATHER_LAT/LON`
    // or `ADAM_WEATHER_CITY`) or the session-belief `city` slot the
    // user previously volunteered. On network failure or missing
    // location, leaves the slot unset so the planner falls back to
    // the existing «менде терезе жоқ» refusal template.
    if matches!(intent, Intent::AskWeather)
        && let Some(line) = crate::weather::render_live(session)
    {
        slots.insert("live_weather_answer".into(), line);
        // Sentinel routes the planner to the dedicated `ask_weather.live`
        // subfamily (single-template). Without this, the seed-mod
        // template picker rolls between the live answer and the
        // honest-refusal variant in `ask_weather` — half the turns
        // would silently say «менде терезе жоқ» despite live data
        // being available.
        slots.insert("__live_weather_set__".into(), "1".into());
    }
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
        Intent::StatementOfActivity {
            activity: Some(activity),
        } => {
            slots.insert("activity".into(), activity.clone());
        }
        // **v4.93.5** — Codex 2026-05-07 audit P2 pedagogical intents.
        // Each intent populates a `topic` slot when extractable, plus
        // an intent-specific body slot keyed off curated content in
        // `crate::pedagogical`. Templates in v1.toml use these slots;
        // `template_is_fillable` filters to topic-bearing variants
        // when topic is present, falls back to clarifying prompts
        // when not.
        Intent::AskExercise { topic: Some(t) } => {
            slots.insert("topic".into(), t.clone());
            if let Some(body) = crate::pedagogical::exercise_for(t) {
                slots.insert("exercise_body".into(), body.into());
            }
        }
        Intent::AskExercise { topic: None } => {}
        Intent::CodeRequest { topic: Some(t) } => {
            slots.insert("topic".into(), t.clone());
            if let Some(snippet) = crate::pedagogical::code_snippet_for(t) {
                slots.insert("code_snippet".into(), snippet.into());
            }
        }
        Intent::CodeRequest { topic: None } => {}
        Intent::ExplainCompilerError { error_code, topic } => {
            if let Some(code) = error_code {
                slots.insert("error_code".into(), code.clone());
                if let Some(explanation) = crate::pedagogical::explain_error_code(code) {
                    slots.insert("error_explanation".into(), explanation.into());
                }
            }
            if let Some(t) = topic {
                slots.insert("topic".into(), t.clone());
            }
        }
        Intent::AskPurpose { topic: Some(t) } => {
            slots.insert("topic".into(), t.clone());
            if let Some(purpose) = crate::pedagogical::purpose_for(t) {
                slots.insert("purpose_body".into(), purpose.into());
            }
        }
        Intent::AskPurpose { topic: None } => {}
        // **v4.96.0** — Codex round-2 audit Bug 7. Cross-language
        // contrast slots — populate `{other_language}` / `{rust_concept}`
        // for the template, plus `{contrast_body}` when curated content
        // exists.
        Intent::CrossLanguageContrast {
            other_language,
            rust_concept,
        } => {
            slots.insert("other_language".into(), other_language.clone());
            slots.insert("rust_concept".into(), rust_concept.clone());
            if let Some(body) =
                crate::pedagogical::cross_language_contrast(other_language, rust_concept)
            {
                slots.insert("contrast_body".into(), body.into());
            }
        }
        // **v4.95.0** — student submission. Run cargo_verify
        // synchronously and populate slots. cargo check is slow
        // (~1-2 s real-world) — there's no async boundary here, the
        // turn just blocks for the duration. Acceptable for an
        // interactive tutor; production would push verification to
        // a background task and surface a "checking..." reply first.
        Intent::SubmitSolution { code, topic } => {
            // **v4.95.5** — multi-turn lesson state. Fall back to
            // the topic of the most-recent exercise / code-request
            // when the submission itself doesn't name a topic. This
            // lets the student answer with bare code (the natural
            // pattern after `AskExercise` returns a prompt) while
            // adam still frames the verdict in lesson context.
            let resolved_topic = topic.clone().or_else(|| {
                slots
                    .get("last_exercise_topic")
                    .filter(|t| !t.is_empty())
                    .cloned()
            });
            if let Some(t) = resolved_topic.as_ref() {
                slots.insert("topic".into(), t.clone());
            }
            let result = crate::cargo_verify::verify_snippet(code);
            if result.environment_failed {
                slots.insert("cargo_status".into(), "env_error".into());
            } else if result.passed {
                slots.insert("cargo_status".into(), "passed".into());
            } else {
                slots.insert("cargo_status".into(), "failed".into());
                if let Some(code) = result.error_codes.first() {
                    slots.insert("error_code".into(), code.clone());
                    if let Some(expl) = crate::pedagogical::explain_error_code(code) {
                        slots.insert("error_explanation".into(), expl.into());
                    }
                }
                // Truncate raw output to a sane size — 600 chars is
                // typically enough for the first compiler error
                // header + caret. Drop ANSI sequences if any.
                let raw = result
                    .raw_output
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .take(12)
                    .collect::<Vec<_>>()
                    .join("\n");
                let raw = if raw.len() > 600 {
                    format!("{}…", &raw[..600])
                } else {
                    raw
                };
                slots.insert("raw_excerpt".into(), raw);
            }
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

/// **v4.43.0** — Stage A bundle 3: introducer migration. Rewrites
/// the `{fact}` slot for the `unknown.with_grounded_fact` family
/// from a body-only sentence to a full preamble+body sentence
/// produced via [`crate::nlg::compose_introducer`]. The introducer
/// pick mirrors the v4.42.x template-rotation algorithm bit-for-bit
/// via [`crate::nlg::pick_introducer`], so output is byte-identical
/// to pre-migration for any `(seed, has_name_respect, fact)`. No-op
/// for other template families.
fn apply_introducer_migration(
    key: &str,
    rng_seed: u64,
    slots: &mut HashMap<String, String>,
    trace: &mut Vec<String>,
) {
    if key != "unknown.with_grounded_fact" {
        return;
    }
    let Some(noun) = slots.get("noun").cloned() else {
        return;
    };
    let Some(body) = slots.get("fact").cloned() else {
        return;
    };
    let name_respect = slots.get("name_respect").cloned();
    let introducer = crate::nlg::pick_introducer(rng_seed, name_respect.is_some());
    let wrapped = crate::nlg::compose_introducer(introducer, &noun, name_respect.as_deref(), &body);
    trace.push(format!(
        "planner: introducer={introducer:?} (v4.43.0 NLG migration)",
    ));
    slots.insert("fact".into(), wrapped);
}

fn uses_geo_feature_location_family(kind: &str) -> bool {
    let kind = kind.to_lowercase();
    kind.contains("теңіз") || kind.contains("өзен") || kind.contains("көл") || kind.contains("тау")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::Intent;

    // **v6.0.0-rc4** — guard regression tests for the
    // `factual_eval_100` 34 → 13 hallucination ratchet. Each block
    // pins one specific prompt shape from the eval set so that
    // future refactors of the guard cannot silently re-open the
    // proverb-fallback / definitional regressions.

    #[test]
    fn factual_guard_catches_temporal_specific() {
        assert!(is_specific_factual_query(
            "қазақстан конституциясы қашан қабылданған"
        ));
        assert!(is_specific_factual_query("абай қай жылы туылған"));
        assert!(is_specific_factual_query("java тілі қашан шықты"));
    }

    #[test]
    fn factual_guard_catches_counted_quantity() {
        assert!(is_specific_factual_query(
            "абайдың қара сөздері неше шығармадан тұрады"
        ));
        assert!(is_specific_factual_query("парламент неше палатадан тұрады"));
        // «қазір» suppresses — that's the live-clock case, not factual.
        assert!(!is_specific_factual_query("қазір сағат неше"));
    }

    #[test]
    fn factual_guard_catches_named_attributes() {
        assert!(is_specific_factual_query(
            "қазақстанның ақша бірлігі қандай"
        ));
        assert!(is_specific_factual_query("су химиялық формуласы қандай"));
        assert!(is_specific_factual_query("абайдың шын аты қандай"));
        assert!(is_specific_factual_query(
            "қазақстанда биліктің бастауы кім"
        ));
    }

    #[test]
    fn factual_guard_catches_definitional_x_qandai_y() {
        assert!(is_specific_factual_query("ай не нәрсе"));
        assert!(is_specific_factual_query("жаз қандай мезгіл"));
        assert!(is_specific_factual_query("жарық қандай құбылыс"));
        assert!(is_specific_factual_query("қан қандай сұйықтық"));
        assert!(is_specific_factual_query("бурабай қандай орын"));
    }

    #[test]
    fn factual_guard_lets_through_open_questions() {
        // Open / conversational queries — example fallback is fine.
        assert!(!is_specific_factual_query("қазақстан туралы айт"));
        assert!(!is_specific_factual_query("мысал келтір"));
        assert!(!is_specific_factual_query("сәлем"));
    }

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

    /// **v4.53.5** — context-aware clarify routing test. When the
    /// standard Unknown routing produces `unknown.clarify_no_topic`
    /// AND the session has stored profile slots, the planner
    /// overrides to `unknown.with_session_diagnostic` whose
    /// templates cite the stored context.
    #[test]
    fn clarify_no_topic_with_session_routes_to_diagnostic() {
        let Ok(repo) = TemplateRepository::load_default() else {
            return; // CI without TOML file — skip.
        };
        // Intent::Unknown with no noun_hint and no evidence — would
        // route to clarify_no_topic in v4.53.0.
        let intent = Intent::Unknown {
            raw_tokens: vec!["иә".into()],
            noun_hint: None,
            example: None,
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: None,
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            input_modality: None,
            input_evidence: None,
            input_is_inversion_question: false,
            noun_hint_confidence: crate::topic_extraction::TopicConfidence::High,
        };
        let mut session = HashMap::new();
        session.insert("name".into(), "Дәулет".into());
        session.insert("occupation".into(), "бағдарламашы".into());
        let plan = plan_response_with_epistemic(
            &intent,
            0,
            &repo,
            &session,
            crate::uncertainty::EpistemicStatus::Unknown,
            &HashMap::new(),
        );
        // Trace must include the clarify-diagnostic override.
        let trace_blob = plan.trace.join("\n");
        assert!(
            trace_blob.contains("clarify_diagnostic override"),
            "expected diagnostic override in trace, got: {trace_blob}"
        );
        // Realised output (template + slot substitution) must cite
        // at least one stored slot value.
        let realised = crate::realiser::realise(&plan);
        assert!(
            realised.contains("Дәулет") || realised.contains("бағдарламашы"),
            "diagnostic should cite stored slot; got: {realised}",
        );
        // Output must NOT be the bare clarify line.
        assert!(
            !realised.contains("Қандай тақырып туралы сұрап отырсыз?"),
            "diagnostic must not fall through to bare clarify_no_topic; got: {realised}",
        );
    }

    /// Regression: when session is empty, the Unknown clarify path
    /// still routes to the bare `unknown.clarify_no_topic` family
    /// (no spurious diagnostic override).
    #[test]
    fn clarify_no_topic_without_session_keeps_bare_family() {
        let Ok(repo) = TemplateRepository::load_default() else {
            return;
        };
        let intent = Intent::Unknown {
            raw_tokens: vec!["иә".into()],
            noun_hint: None,
            example: None,
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: None,
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            input_modality: None,
            input_evidence: None,
            input_is_inversion_question: false,
            noun_hint_confidence: crate::topic_extraction::TopicConfidence::High,
        };
        let session = HashMap::new();
        let plan = plan_response_with_epistemic(
            &intent,
            0,
            &repo,
            &session,
            crate::uncertainty::EpistemicStatus::Unknown,
            &HashMap::new(),
        );
        let trace_blob = plan.trace.join("\n");
        assert!(
            !trace_blob.contains("clarify_diagnostic override"),
            "no override should fire without session slots; trace: {trace_blob}"
        );
        assert!(
            trace_blob.contains("template_key=unknown.clarify_no_topic"),
            "expected bare clarify_no_topic; trace: {trace_blob}"
        );
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
        // **v4.51.0** — user-activity slot.
        Intent::AskActivity => "ask_activity",
        Intent::StatementOfActivity { .. } => "statement_of_activity",
        Intent::AskFamily => "ask_family",
        Intent::StatementOfFamily => "statement_of_family",
        Intent::AskWeather => "ask_weather",
        Intent::StatementOfWeather => "statement_of_weather",
        Intent::AskTime { .. } => "ask_time",
        Intent::Compliment => "compliment",
        Intent::Request => "request",
        Intent::WellWishes => "well_wishes",
        Intent::Insult => "insult",
        Intent::UserAcknowledgement => "user_acknowledgement",
        // v4.14.0 — curriculum-content honest fallback.
        Intent::AskCurriculumContent => "ask_curriculum_content",
        Intent::AskWillingness => "ask_willingness",
        // **v4.93.5** — Codex 2026-05-07 audit P2 pedagogical intents.
        // Each routes to a dedicated template family. `topic` carries
        // the technical subject (extracted via pedagogical_topic_hint
        // in semantics.rs) and is interpolated into templates; topic-
        // less variants fall back to generic prompts.
        Intent::AskExercise { .. } => "ask_exercise",
        Intent::CodeRequest { .. } => "code_request",
        Intent::ExplainCompilerError { .. } => "explain_compiler_error",
        Intent::AskPurpose { .. } => "ask_purpose",
        // **v4.95.0** — student submission. Sub-family routed in
        // `intent_subkey_with_slots` after `extract_slots` runs
        // cargo_verify — passed / failed_known / failed_unknown /
        // env_error.
        Intent::SubmitSolution { .. } => "submit_solution",
        // **v4.96.0** — Codex round-2 audit Bug 7. Sub-key remap
        // happens later (in plan_response_with_session and
        // plan_response_with_epistemic) based on whether a curated
        // contrast text exists for the (other_language, rust_concept)
        // pair.
        Intent::CrossLanguageContrast { .. } => "cross_language_contrast",
        // **v4.99.0** — student-side curriculum-query intents.
        // Sub-key remap (suggestion vs complete; recap vs empty)
        // happens later in plan_response_with_session and
        // plan_response_with_epistemic based on the pre-stuffed
        // curriculum slots.
        Intent::AskNextTopic => "next_topic",
        Intent::AskCurrentProgress => "current_progress",
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
                // **v4.93.0** — Codex 2026-05-07 audit: function-asking
                // phrases also override modal-necessity routing.
                // «X не үшін керек?» / «X қашан керек?» / «X қалай
                // жұмыс істейді?» / «X не істейді?» — these all
                // include `керек` (Necessity modality) but ask about
                // X's CONTENT, not whether X is needed in the
                // abstract. Pre-fix: «Rust-та ownership не үшін
                // керек?» returned the generic «Иә, маңызды мәселе
                // екен» modal hedge instead of the curated ownership
                // fact. Same pattern as the explain/teach skip above.
                let lower_joined = raw_tokens.join(" ").to_lowercase();
                let has_function_asking = lower_joined.contains("не үшін керек")
                    || lower_joined.contains("қашан керек")
                    || lower_joined.contains("неге керек")
                    || lower_joined.contains("неге қажет")
                    || lower_joined.contains("не істейді")
                    || lower_joined.contains("не атқарады")
                    || lower_joined.contains("қалай жұмыс іс");
                if !(has_function_asking || has_explain_teach && has_grounded_fact) {
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
            // **v5.17.7 — adversarial D1 fr_01 closure.** Suppress
            // Hearsay routing when the input is a QUESTION. The
            // Kazakh suffix `-қан/-ген` is morphologically ambiguous
            // between past-evidential («I heard it happened») and
            // perfective participle / relative clause («which is
            // located»). FST tags both as `EvidenceKind::Hearsay`
            // by default, but in a question shape («X **қайда**
            // орналасқан?» = «where is X located?») the participle
            // reading is canonical — the user is asking a factual
            // question, not reporting hearsay. Pre-v5.17.7 «Алматы
            // қайда орналасқан?» routed to `unknown.with_hearsay_hedge`
            // and surfaced «Сіз естіген сөз болса керек, бірақ мен
            // тексере алмаймын», suppressing a perfectly available
            // grounded_fact («Алматы — Қазақстанның республикалық
            // маңызы бар қаласы»). Codex 2026-05-11 adversarial
            // benchmark fr_01.
            //
            // The fall-through proceeds to the standard
            // `unknown.with_grounded_fact` family below, which
            // surfaces the actual curated answer.
            if matches!(input_evidence, Some(adam_kernel_fst::EvidenceKind::Hearsay))
                && question_shape.is_none()
            {
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
                    // **v6.0.0-rc4 factual_eval hardening** — when the
                    // turn is a *specific factual query* (asks for a
                    // year, a single named referent, a formula, an
                    // explicit «бастауы кім / қашан / неше / қандай
                    // зат» …), suppress the corpus-sample fallback
                    // (`unknown.with_evidence`) because it produces
                    // «{noun} туралы мынаны айта аламын: «<random
                    // proverb mentioning noun>»» which reads as a
                    // confident answer but is content-wise off-topic
                    // (rc4 `factual_eval_100` had ~10 such cases).
                    // Other Unknown routes (citation requests, generic
                    // «X не нәрсе?» without a specific aspect, anything
                    // landing in Example mode) still receive corpus
                    // quotes as before — regression-tested by the
                    // `compose_mode_*` / `unknown_with_retrieval_*` /
                    // `adapted_evidence_*` end-to-end suites.
                    let factual_joined = raw_tokens.join(" ").to_lowercase();
                    let factual = is_specific_factual_query(&factual_joined);
                    let example_ok = !factual;
                    // Numeric-grounded guard explored 2026-05-19 but
                    // turned net-negative (downgraded legitimate
                    // grounded_fact answers for const_005/006/010 to
                    // reasoning-chain templates, losing −4 correct
                    // for −2 hallucinations). Reverted to plain
                    // `grounded_fact.is_some()` here. The category-C
                    // (adjacent-fact) hallucination is documented as
                    // open in `docs/factual_eval_hallucination_*` —
                    // proper fix needs predicate-aware fact selection
                    // upstream of the router, not a post-hoc digit
                    // heuristic.
                    if example.is_some()
                        && unknown_prefers_quoted_example(raw_tokens)
                        && *example_adapted
                    {
                        "unknown.with_adapted_evidence"
                    } else if example.is_some() && unknown_prefers_quoted_example(raw_tokens) {
                        "unknown.with_evidence"
                    } else if grounded_fact.is_some() {
                        "unknown.with_grounded_fact"
                    } else if example_ok && example.is_some() && *example_adapted {
                        "unknown.with_adapted_evidence"
                    } else if example_ok && example.is_some() {
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

/// **v6.0.0-rc4 factual_eval_100 guard.** Detect specific factual
/// queries that ask for a single grounded answer (year, formula,
/// named source / author / currency, counted quantity). When a
/// query of this shape lacks a curated fact, surfacing a corpus-
/// sample (`unknown.with_evidence`) yields the «{noun} туралы
/// мынаны айта аламын: «<random proverb>»» hallucination pattern
/// that GA #4 prohibits. The General-mode Unknown router uses
/// this predicate to suppress the example fallback for such
/// queries, falling back instead to noun-echo / bare unknown
/// (which the `factual_eval_100` runner counts as Refusal — i.e.
/// grounded).
fn is_specific_factual_query(joined: &str) -> bool {
    // Temporal-specific
    if joined.contains("қашан") || joined.contains("қай жылы") || joined.contains("нешеуінде")
    {
        return true;
    }
    // Counted quantity («неше X-DAT тұрады», «неше шумақтан»,
    // «неше сағат бар»). The duration guard in detect_ask_time
    // already steers obvious time-period reads away; the remaining
    // «неше» queries are factual numeric.
    if (joined.contains("неше") || joined.contains("қанша")) && !joined.contains("қазір")
    {
        return true;
    }
    // Authorship / structural-source asking phrases.
    if joined.contains("бастауы")
        || joined.contains("авторы")
        || joined.contains("иесі")
        || joined.contains("шын аты")
        || joined.contains("әкесі")
    {
        return true;
    }
    // Named-attribute factual nouns.
    if joined.contains("ақша бірлігі")
        || joined.contains("формуласы")
        || joined.contains("елордасы")
        || joined.contains("халық саны")
    {
        return true;
    }
    // **Definitional shape «X қандай Y / X не Y»** where Y is a
    // generic-class noun. The corpus-sample fallback on this shape
    // surfaces an off-topic proverb mentioning Y (e.g. astro_003
    // «Ай не нәрсе?» → quote about money, time_004 «Жаз қандай
    // мезгіл?» → quote about prisoners). Treat as factual so the
    // router prefers `unknown.with_noun` (honest hedge) over the
    // proverb fallback.
    let definitional_y = [
        "қандай нәрсе",
        "қандай зат",
        "қандай орын",
        "қандай мезгіл",
        "қандай құбылыс",
        "қандай сұйықтық",
        "қандай газ",
        "қандай ай",
        "қандай аспан денесі",
        "қандай мемлекет",
        "не нәрсе",
        "не зат",
    ];
    if definitional_y.iter().any(|p| joined.contains(p)) {
        return true;
    }
    false
}
