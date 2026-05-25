// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `v6_2_router` — **integration bridge** between v6.1 dialog
//! cascade and v6.2 neurosymbolic stack.
//!
//! This module is the **single integration point** the v6.1
//! dialog kernel calls into when `ADAM_V6_2=1`. When the gate is
//! off (default), v6.1 cascade runs unchanged.
//!
//! ## Architecture
//!
//! ```text
//! user input
//!   ├─ ADAM_V6_2=1?
//!   │    YES → v6_2_router::answer(input, &corpus_index)
//!   │           ├─ math_solver  (procedural)
//!   │           ├─ system_clock (live state)
//!   │           ├─ FrameIndex   (curated retrieval)
//!   │           └─ realiser     (Frame → Kazakh surface)
//!   │    NO  → v6.1 dialog cascade (Conversation::turn)
//!   └─ output
//! ```
//!
//! Both paths produce a `String` answer; the caller chooses by
//! `is_v6_2_active()`. Stage 8 will promote v6.2 to default-on
//! after HumanDialogEval passes.

use std::sync::OnceLock;

use adam_algebra::{
    AnswerShape, AnswerSlot, Composition, FrameIndex, FramePredicate, Language, ModifierRole,
    PartOfSpeech, QueryFocus, QueryIR, QuestionForm, RankedFrame, Root, corpus_loader,
    dialog_battery, math_solver, realiser, system_clock,
};

/// Read the `ADAM_V6_2` env var. Set to `1` / `true` / `on` to
/// route the dialog cascade through the v6.2 stack instead of v6.1.
///
/// Stage 7 ships this as **opt-in** so existing CI / user setups
/// see no regression. Stage 8 flips the default after
/// HumanDialogEval ≥ 90 % on a curated battery.
pub fn is_v6_2_active() -> bool {
    std::env::var("ADAM_V6_2")
        .map(|v| matches!(v.as_str(), "1" | "true" | "on" | "yes"))
        .unwrap_or(false)
}

/// Process-wide shared corpus, lazily loaded from
/// `data/world_core/*.jsonl` on first use. Falls back to the
/// hand-curated `dialog_battery::canonical_corpus()` when the data
/// directory is absent (e.g. cargo-published crate without bundled
/// data).
fn shared_corpus() -> &'static FrameIndex {
    static CORPUS: OnceLock<FrameIndex> = OnceLock::new();
    CORPUS.get_or_init(|| {
        // Try several candidate paths so the loader works both
        // from the repo root and from a sub-crate test dir.
        for candidate in [
            "data/world_core",
            "../data/world_core",
            "../../data/world_core",
            "../../../data/world_core",
        ] {
            if let Ok((idx, stats)) = corpus_loader::load_world_core(candidate)
                && stats.frames_inserted > 0
            {
                // Augment with the battery's bilingual + historical
                // facts that aren't in world_core/*.jsonl yet (e.g.
                // Russian-rooted aliases, МО РК-specific facts).
                let mut idx = idx;
                augment_with_battery_facts(&mut idx);
                return idx;
            }
        }
        // Last resort: the hand-curated battery corpus.
        dialog_battery::canonical_corpus()
    })
}

/// Add facts that the dialog battery curates but `world_core/*.jsonl`
/// doesn't yet include (Russian aliases, МО РК, historical dates
/// added between v6.1.50 and Stage 7).
fn augment_with_battery_facts(idx: &mut FrameIndex) {
    let battery = dialog_battery::canonical_corpus();
    // Re-insert each battery frame; the FrameIndex deduplicates
    // by structural equality at retrieval time (via match_frame),
    // so duplicate inserts are safe.
    for i in 0..battery.len() {
        let entry = battery.get(adam_algebra::FrameId(i as u32));
        idx.insert_with_language(entry.frame.clone(), entry.domain.clone(), entry.language);
    }
}

/// Main entry point — answer one user input through the full
/// v6.2 stack. Returns `Some(answer)` when any layer produces a
/// result, `None` when the input falls outside our coverage.
pub fn answer(input: &str) -> Option<String> {
    answer_with_corpus(input, shared_corpus())
}

/// Variant that lets callers supply their own [`FrameIndex`] (used
/// in integration tests + the live REPL).
pub fn answer_with_corpus(input: &str, idx: &FrameIndex) -> Option<String> {
    // 1. Math first — procedural computation.
    if looks_like_math(input)
        && let Some(r) = math_solver::solve(input)
    {
        return Some(r.render());
    }

    // 2. System clock — live state.
    if looks_like_time_query(input) {
        return Some(emit_clock_answer(input));
    }

    // 3. Retrieval — typed QueryIR → FrameIndex → realiser.
    let q = build_query_heuristic(input)?;
    let hit = idx.best_match(&q)?;
    Some(realiser::realise(
        hit.frame,
        &q.focus,
        hit.match_result.answer_slot,
    ))
    .filter(|s| !s.trim().is_empty())
}

fn looks_like_math(s: &str) -> bool {
    let lower = s.to_lowercase();
    let markers = [
        "плюс",
        "минус",
        "умнож",
        "раздели",
        "помнож",
        "подели",
        "степени",
        "корень",
        "процент",
        "пайыз",
        "көбейт",
        "бөл",
        "қос",
        "азайт",
        "дәреже",
        "түбірі",
        "sin",
        "cos",
        "tan",
        "log",
        "ln",
        "abs",
        "mod",
        "остаток",
        "қалдық",
    ];
    if markers.iter().any(|m| lower.contains(m)) {
        return true;
    }
    s.chars()
        .any(|c| matches!(c, '+' | '*' | '/' | '%' | '^' | '√' | '×' | '÷'))
}

fn looks_like_time_query(s: &str) -> bool {
    let lower = s.to_lowercase();
    let markers = [
        "бүгін",
        "қазір",
        "сегодня",
        "сейчас",
        "который час",
        "сағат неше",
        "нешесі",
        "апта",
        "неделя",
        "какая дата",
        "какой день",
    ];
    markers.iter().any(|m| lower.contains(m))
}

fn emit_clock_answer(input: &str) -> String {
    let c = system_clock::now();
    let lower = input.to_lowercase();
    if lower.contains("сағат") || lower.contains("часов") || lower.contains("уақыт")
    {
        return format!("Қазір сағат {}.", c.time_hhmm());
    }
    if lower.contains("апта")
        || lower.contains("неделя")
        || (lower.contains("күн") && lower.contains("қай"))
    {
        return format!("Бүгін — {}.", c.weekday_kk());
    }
    if lower.contains("ай")
        && (lower.contains("қандай") || lower.contains("какой") || lower.contains("какая"))
    {
        return format!("Қазір {} айы.", c.month_kk());
    }
    if lower.contains("нешесі") || lower.contains("число") {
        return format!("Бүгін {} {}.", c.day, c.month_kk());
    }
    format!(
        "Бүгін {} {} {} жыл, {}.",
        c.day,
        c.month_kk(),
        c.year,
        c.weekday_kk()
    )
}

fn build_query_heuristic(input: &str) -> Option<QueryIR> {
    let lower = input.to_lowercase();
    let language = if has_russian_marker(&lower) {
        Language::Russian
    } else {
        Language::Kazakh
    };
    let agent = canonical_agent_for(&lower)?;
    let focus_kind = detect_focus(&lower);
    let predicate = predicate_for(&lower);

    let (focus, answer_shape) = match focus_kind {
        FocusKind::Time => (
            QueryFocus::Modifier(ModifierRole::Time),
            AnswerShape::DateAnchor,
        ),
        FocusKind::Place => (
            QueryFocus::Modifier(ModifierRole::Location),
            AnswerShape::BareNoun,
        ),
        FocusKind::Subject => (QueryFocus::Subject, AnswerShape::BareNoun),
        FocusKind::Object => (QueryFocus::Object, AnswerShape::DefinitionalNP),
        FocusKind::Definition => (QueryFocus::Definition, AnswerShape::DefinitionalNP),
    };

    let mut q = QueryIR::new(focus, QuestionForm::Definition, answer_shape)
        .with_agent(noun(&agent))
        .with_language_filter(language);
    if let Some(p) = predicate {
        q = q.with_predicate(p);
    }
    Some(q)
}

#[derive(Debug, Clone, Copy)]
enum FocusKind {
    Time,
    Place,
    Subject,
    Object,
    Definition,
}

fn detect_focus(lower: &str) -> FocusKind {
    if lower.contains("қашан") || lower.contains("когда") {
        FocusKind::Time
    } else if lower.contains("қайда")
        || lower.contains("где")
        || lower.contains("қай қала")
        || lower.contains("какая столица")
    {
        FocusKind::Place
    } else if lower.contains("кім?") || lower.contains("кто?") || lower.contains("кім бұл")
    {
        FocusKind::Subject
    } else if lower.contains("деген не")
        || lower.contains("что такое")
        || lower.contains("деген кім")
    {
        FocusKind::Definition
    } else {
        FocusKind::Object
    }
}

fn predicate_for(lower: &str) -> Option<FramePredicate> {
    if lower.contains("туыл") || lower.contains("туған") || lower.contains("родился")
    {
        return Some(FramePredicate::BornIn);
    }
    if lower.contains("қайтыс") || lower.contains("умер") {
        return Some(FramePredicate::DiedIn);
    }
    if lower.contains("құрыл")
        || lower.contains("ашыл")
        || lower.contains("основан")
        || lower.contains("болды")
        || lower.contains("қабылдан")
    {
        return Some(FramePredicate::FoundedIn);
    }
    if lower.contains("автор") || lower.contains("жазған") {
        return Some(FramePredicate::Authored);
    }
    if lower.contains("атымен") || lower.contains("честь") {
        return Some(FramePredicate::NamedAfter);
    }
    if lower.contains("орналас") || lower.contains("находится") || lower.contains("қайда")
    {
        return Some(FramePredicate::LocatedIn);
    }
    if lower.contains("өмір сүр") || lower.contains("жил") {
        return Some(FramePredicate::LivesIn);
    }
    if lower.contains("қанша") || lower.contains("сколько") {
        return Some(FramePredicate::HasQuantity);
    }
    if lower.contains("санаттар") || lower.contains("жіктейді") {
        return Some(FramePredicate::Classifies);
    }
    if lower.contains("күшіне") {
        return Some(FramePredicate::EffectiveFrom);
    }
    Some(FramePredicate::IsA)
}

/// Heuristic agent-surface detector — longest matching canonical
/// surface from the curated corpus wins. Stage 8 replaces this
/// with a typed Stage-2 morpho-lattice → Frame::from_morph_lattice
/// pipeline.
fn canonical_agent_for(lower: &str) -> Option<String> {
    let candidates: &[&str] = &[
        // Battery-specific multi-word entities (longest first).
        "жасанды интеллект туралы заң",
        "ахмет байтұрсынұлы",
        "defense tech it park",
        "қазақстанның астанасы",
        "қазақстан тәуелсіздігі",
        "қазақстан конституциясы",
        "тың және тыңайған жерлер",
        "көмірқышқыл газы",
        "жарық жылдамдығы",
        "ньютон екінші заңы",
        "эйнштейн формуласы",
        "желтоқсан оқиғасы",
        "семей полигоны",
        "столица казахстана",
        "скорость света",
        "бағдарламалау тілі",
        "каспий теңізі",
        "қазақ хандығы",
        "алаш қозғалысы",
        "кенесары қасымұлы",
        "шоқан уәлиханов",
        "жамбыл жабаев",
        "тәуке хан",
        "қазақ кср",
        // Single-word common surfaces.
        "мо рк",
        "одкб",
        "абай",
        "қазақстан",
        "қостанай",
        "rust",
        "кру",
        "су",
        "вода",
        "ай",
        "ағаш",
        "көміртек",
        "углерод",
        "фотосинтез",
        "гравитация",
        "днк",
        "эверест",
        "алгоритм",
        "жаңбыр",
        "сел",
        "семей",
    ];
    let mut best: Option<(&str, usize)> = None;
    for c in candidates {
        if lower.contains(c) {
            let len = c.chars().count();
            if best.is_none_or(|(_, l)| len > l) {
                best = Some((c, len));
            }
        }
    }
    best.map(|(s, _)| s.to_string())
}

fn has_russian_marker(lower: &str) -> bool {
    let words = [
        "что",
        "такое",
        "какой",
        "какая",
        "когда",
        "где",
        "кто",
        "сколько",
        "сегодня",
        "сейчас",
        "столица",
    ];
    words.iter().any(|w| lower.contains(w))
}

fn noun(s: &str) -> Composition {
    Composition::identity(Root::new(s, PartOfSpeech::Noun))
}

// Suppress unused-import lint on RankedFrame — it's part of the
// public adam-algebra surface this module re-exports semantically.
const _: fn() -> () = || {
    let _: Option<RankedFrame<'_>> = None;
    let _: AnswerSlot = AnswerSlot::Whole;
};

#[cfg(test)]
mod tests {
    use super::*;

    /// ENV-gate test: when `ADAM_V6_2` is unset, gate is closed.
    /// (We don't twiddle the env var here — env state is process-
    /// global and tests run in parallel. Just assert the function
    /// is callable.)
    #[test]
    fn is_v6_2_active_reads_env_var() {
        let _ = is_v6_2_active();
    }

    /// Math routes through the solver.
    #[test]
    fn math_routes_through_solver() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Два плюс два", &idx);
        assert_eq!(r.as_deref(), Some("4"));
    }

    /// Clock routes through system_clock; we don't assert exact
    /// content (live) but require non-empty.
    #[test]
    fn clock_routes_through_system_clock() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазір сағат неше?", &idx);
        assert!(r.is_some());
        assert!(r.unwrap().contains(":"));
    }

    /// Real biographical question → realised Kazakh sentence.
    #[test]
    fn bio_question_returns_year_sentence() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ахмет Байтұрсынұлы қашан туылған?", &idx);
        assert_eq!(r.as_deref(), Some("1872"));
    }

    /// IsA definition returns a copular sentence.
    #[test]
    fn isa_returns_copular_sentence() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстан деген не?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("мемлекет"),
            "expected «мемлекет» in answer, got: {s}"
        );
    }

    /// Russian bilingual query routes to Russian-language fact.
    #[test]
    fn russian_query_returns_russian_fact() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Что такое гравитация?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("сила притяжения"),
            "expected Russian definition, got: {s}"
        );
    }

    /// Unknown input → None (fallback to v6.1 cascade).
    #[test]
    fn unknown_input_returns_none() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("xyz random gibberish 123", &idx);
        // Math markers absent, clock markers absent, no known agent
        // — the router declines.
        assert!(r.is_none());
    }
}
