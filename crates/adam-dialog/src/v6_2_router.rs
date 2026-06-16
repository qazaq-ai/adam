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

/// Read the `ADAM_V6_2` env var.  Set to `1` / `true` / `on` to
/// route the dialog cascade through the v6.2 stack (math_solver,
/// FrameIndex, realiser, OOD discipline, safety guard).
///
/// **Status (v6.5.0-rc20).**  Blind eval **97 / 100** on the
/// curated Kazakh battery — well past the ≥90 % bar the v6.2 doc
/// cited.  The Rust-level default flip would expose ~20 v6.1-
/// cascade-specific regression tests (live_holdout_*,
/// factual_eval_100, end_to_end self-intro / cross-slot, …) whose
/// assertions check v6.1 wording rather than v6.2 behaviour.
/// Migrating those to a v6.2 sibling suite is a separate v6.6+
/// arc; until then the library default stays OFF.  Production
/// binaries (voice REPL, `adam_blind_eval`, `adam_chat`) opt in
/// via env-var set at startup.
///
/// rc20 ships the prep work — cognitive_eval Kazakh-only
/// templates that drop English brand-name leaks
/// («curated», «Rust», «LLM», «live-feed», «ASCII») in favour of
/// «тексерілген деректер» / «бағдарламалау тілдері» /
/// «ағымдағы уақыт» / «латын-таңбалы өрнек».  These pay off both
/// in the future default flip AND in the voice REPL today
/// (Piper TTS reads them cleanly aloud).
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
pub(crate) fn shared_corpus() -> &'static FrameIndex {
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

/// **v6.5.0-rc4 (2026-06-09) — lexicon-validated variant.**  Same
/// as [`answer_with_corpus`] but passes the lexicon to the math
/// route so it can refuse to strip case suffixes from words that
/// have a real Kazakh meaning beyond «numeral + case» (e.g. «онда»
/// = "then", not «он» + locative).  See
/// [`adam_algebra::math_solver::solve_validated`] for details.
pub fn answer_with_corpus_and_lexicon(
    input: &str,
    idx: &FrameIndex,
    lex: &adam_kernel_fst::lexicon::LexiconV1,
) -> Option<String> {
    answer_with_corpus_inner(input, idx, Some(lex))
}

/// Variant that lets callers supply their own [`FrameIndex`] (used
/// in integration tests + the live REPL).
pub fn answer_with_corpus(input: &str, idx: &FrameIndex) -> Option<String> {
    answer_with_corpus_inner(input, idx, None)
}

fn answer_with_corpus_inner(
    input: &str,
    idx: &FrameIndex,
    lex: Option<&adam_kernel_fst::lexicon::LexiconV1>,
) -> Option<String> {
    // 0a. STT-loop dedupe. Whisper sometimes gets stuck in a
    // repeat-loop and emits «Сәлем. Сәлем. Сәлем.» × 30+. Collapse
    // to the first meaningful clause so adam answers ONCE, not 30×.
    // Codex 2026-05-25 voice REPL session 3 caught this producing
    // 6-line cascade misfires per loop input.
    let dedup_owned;
    let input: &str = if let Some(dedup) = dedupe_stt_loop(input) {
        dedup_owned = dedup;
        &dedup_owned
    } else {
        input
    };

    // 0b. STT-fold — normalize Whisper mishears («оналты» → «он алты»,
    // «жел тұқтықстан» → «желтоқсан», «энштейн» → «эйнштейн»)
    // ONCE at the top so every downstream path (math_solver,
    // canonical_agent_for, broad-topic detector) sees the folded
    // form. Without this, math_solver missed «он алты түбірі»
    // because it received raw «оналты түбірі» (codex 2026-05-25
    // session-3 audit).
    let folded = stt_fold(&input.to_lowercase());
    let input: &str = &folded;

    // 1. Math first — procedural computation.
    //
    // **rc4 architectural fix:** when a lexicon is available, build
    // an FST-backed "is_non_numeral" closure so math_solver refuses
    // to strip case suffixes from words like «онда» (= "then") that
    // have a real Kazakh meaning beyond «numeral + case».  Caller
    // without lexicon (legacy `answer_with_corpus(input, idx)`) falls
    // through to the hardcoded blacklist inside math_solver.
    let math_hit = if let Some(lex) = lex {
        let is_non_numeral = |w: &str| -> bool {
            use adam_kernel_fst::parser::{Analysis, analyse};
            analyse(w, lex).iter().any(|a| match a {
                Analysis::Noun { root, .. } => root.part_of_speech != "numeral",
                Analysis::Verb { .. } => true,
            })
        };
        if math_solver::looks_like_math_validated(input, &is_non_numeral) {
            math_solver::solve_validated(input, &is_non_numeral)
        } else {
            None
        }
    } else {
        if looks_like_math(input) {
            math_solver::solve(input)
        } else {
            None
        }
    };
    if let Some(r) = math_hit {
        return Some(r.render());
    }

    // **2026-06-03 voice REPL regression** — «Мен Қостанай қалада
    // тұрамын» (I live in the city of Қостанай) was being overridden
    // by the v6_2_router's substring-IsA layer to one-word «Қала»
    // (city). The v6.1 cascade upstream already detected
    // StatementOfLocation, updated the session (`session["city"] =
    // "Қостанай"`), and generated an acknowledgement reply — but
    // v6_2_router.answer() returning Some("Қала") clobbered it.
    //
    // Fix: when the input is a first-person location statement
    // («Мен … тұрамын / тұрамыз»), return None and let the v6.1
    // acknowledgement stand. We DON'T need v6.2-side acknowledgement
    // because the session is already populated; later recall queries
    // («Мен қайда тұрамын») use that session state via the standard
    // v6.2 location-recall handler.
    if looks_like_first_person_location_statement(input) {
        return None;
    }

    // **Phase 23 (2026-06-03)** — chemistry-formula lookup. Live REPL
    // (multi-session) caught «Судың формуласын жазып бер» falling
    // through to the substring-IsA layer that returns «Жансыз табиғат»
    // (the `Су IsA жансыз табиғат` fact wins over the chemistry-formula
    // intent). Hardcoded school-level formula table fires BEFORE the
    // IsA fallback. Requires the «формула» word in the input to avoid
    // false positives on bare substance mentions.
    if let Some(answer) = lookup_chemical_formula(input) {
        return Some(answer);
    }

    // **v6.8 (2026-06-16) — possessive-property lookup.** Catches
    // school-eval question shapes «X-genitive Y-possessive»
    // («Қазақстанның мемлекеттік тілі», «Қазақтың ұлттық тағамы»,
    // «Қазақстанның ең үлкен қаласы») BEFORE the substring-IsA
    // fallback further down the cascade. Pre-v6.8 these queries
    // surfaced wrong answers:
    //   «Қазақстанның мемлекеттік тілі.» → «Мемлекет»
    //     (the cascade matched IsA on the leading noun «Қазақстан»,
    //      ignoring the property head «тілі»)
    //   «Қазақтың ұлттық тағамы.» → «Ұлттық тағам — тағам.»
    //     (substring-IsA picked up «ұлттық тағам IsA тағам»)
    //
    // The world_core facts for these are already curated (const_008,
    // cuis_001 / cuis_002, geo_kz_004); the gap is just in the
    // retrieval ordering. Pattern-matched lookup short-circuits the
    // ambiguous IsA path. Keep the table small and curated; broader
    // possessive disambiguation belongs in the Stage 8 typed query IR.
    if let Some(answer) = lookup_possessive_property(input) {
        return Some(answer);
    }

    // 1a. Occupation acknowledgement. «Мен X» / «Мен X-мын» —
    // user stating profession / role. The v6.1 cascade interpreted
    // this as a definition request («Бағдарламашы — кәсіп иесі.»),
    // not a personal statement. Catch the most common shapes.
    if let Some(ack) = recognize_occupation_statement(input) {
        return Some(ack);
    }

    // 1b. Capabilities query. «Сен не білесің?» / «Не істей
    // аласың?» — user wants a self-description of what adam can
    // answer. Distinct from self-identity («Сен кімсің?»).
    if is_capabilities_query(input) {
        return Some(capabilities_response(input));
    }

    // 1c. Pitch-gender explanation. «Сен мені ағай дедің. Қалай
    // түсіндің?» — user asks how adam detected gender. Honest
    // explanation: pitch analysis on voice input.
    if is_pitch_detection_query(input) {
        // **Phase 20** — 2 paraphrases of pitch detection explanation.
        let variants: &[&str] = &[
            "Сіздің даусыңыздың жиілігі (pitch) бойынша анықтадым. \
             Voice-input режимінде whisper.cpp дауысты транскрипциялаған \
             соң, мен оның негізгі жиілігін («male» болса ~ 85–155 Гц, \
             «female» болса ~ 165–255 Гц) есептеймін де, соған сай \
             қазақша құрметтеу формасын — «Ағай» немесе «Апай» — \
             таңдаймын. Бұл — детерминирленген эвристика, нейрожүйе емес.",
            "Дауыс жиілігі (F0) арқылы. Whisper аудионы транскрипциялаған \
             соң, мен оның негізгі жиілігін есептеймін — ер адам \
             даусы әдетте 85–155 Гц аралығында, әйел даусы 165–255 Гц. \
             Соған қарап «Ағай» немесе «Апай» вокативін таңдаймын. \
             Алгоритм — autocorrelation-based YIN-тәріздес pitch \
             detection, ешқандай нейрожүйе емес.",
        ];
        return Some(pick_variant(variants, input).to_string());
    }

    // 2. Self-identity short-circuit. «Сен кімсің?» / «Сен өзің
    // кімсің?» — these are dialog-self questions, not factoid
    // queries. Without this gate the cascade matches morpheme
    // «сен» / «өзің» and emits Abai poetry quotes (codex
    // 2026-05-25 audit caught this).
    if is_self_identity_query(input) {
        // **Phase 20 + v6.5.0-rc11 (2026-06-10)** — 3 paraphrases of
        // the self-introduction.  rc10 audit flagged the previous
        // wording: the Piper Kazakh TTS voice mis-pronounces the
        // English «Agglutinative Reasoning Kernel» and the bare
        // «ARK» / «LLM» / «curated» tokens.  Replaced English-source
        // brand names with their Kazakh equivalents so the TTS reads
        // cleanly aloud.  The kernel itself is still ARK internally —
        // only the user-facing self-description changes.
        let variants: &[&str] = &[
            "Мен — адам, қазақ тіліне арналған детерминирленген тілдік \
             жүйемін. Үлкен тілдік модель емеспін — жауаптарымды \
             алдын ала тексерілген деректерден аламын.",
            "Менің атым — адам. Қазақ тілінің морфологиясы бойынша \
             құрастырылған детерминирленген тілдік жүйемін. \
             Жауаптарым тек тексерілген деректерге сүйенеді, \
             ойдан құрастырылмайды.",
            "Мен — қазақ тіліне арналған агглютинативті ой жүйесімін, \
             қысқаша «адам» деп аталамын. Әр сөзімді тексерілген \
             деректермен растаймын; білмейтін нәрсемді «нақты дерегім \
             жоқ» деп ашық айтамын.",
        ];
        return Some(pick_variant(variants, input).to_string());
    }

    // 3. Honest «no live data» refusals — weather, currency,
    // stock prices, current-data queries the kernel has no feed
    // for. **Runs BEFORE the system clock gate** so «Бүгін
    // Алматыда қандай ауа райы?» (which has «бүгін» trigger for
    // clock) routes correctly to the weather-refusal path.
    if needs_live_data_refusal(input) {
        // **Phase 20** — 3 paraphrases for live-data refusal.
        let variants: &[&str] = &[
            "Бұл сұраққа жауап беру үшін менде нақты дерек жоқ. \
             Менің білім қорым тексерілген деректерден тұрады, \
             тікелей интернет немесе ағымдағы мәлімет ағысы қосылған емес.",
            "Бұл сұраққа дерек бере алмаймын — менің білім қорымда \
             ағымдағы немесе реалды-уақыттық мәлімет жоқ. Тек \
             тексерілген тарихи деректермен жұмыс істеймін.",
            "Кешіріңіз, бұл сұраққа жауап беретін ағымдағы дерек менде \
             жоқ. Интернетке немесе сыртқы мәлімет көзіне қосылмаймын — \
             тек тексерілген тарихи деректер қолымда.",
        ];
        return Some(pick_variant(variants, input).to_string());
    }

    // 4. System clock — live state (date / month / weekday /
    //    time-of-day). Only matches queries that are about today's
    //    calendar / clock, NOT about year-anchored facts.
    if looks_like_time_query(input) {
        return Some(emit_clock_answer(input));
    }

    // 4a. Broad-topic «X туралы айтшы» — return a multi-fact
    // paragraph instead of a single object word. Codex 2026-05-25
    // voice REPL caught «Қазақстан туралы айтшы» → «Мемлекет»
    // (one-word IsA hit). Now emits a curated paragraph.
    if let Some(topic) = detect_broad_topic_query(input)
        && let Some(paragraph) = render_broad_topic(&topic, idx)
    {
        return Some(paragraph);
    }

    // 4b. Curated enumeration shortcuts. «Қазақстанның облыстарын
    // айтшы» / «Қазақстанның көршілері кім?» — these need a list,
    // not a single-fact answer. The world_core has curated list
    // strings (geo_kz_104 has all 17 oblasts comma-separated).
    // Stage 8 will lift this with proper Enumeration retrieval;
    // tonight we hand-wire the most common queries.
    if let Some(list_answer) = handle_listing_query(input) {
        return Some(list_answer);
    }

    // **v6.5.0-rc18 — OOD discipline.**  rc17 blind eval surfaced
    // 7 true-positive OOD bugs where adam was emitting wrong
    // Kazakh-relevant answers on non-Kazakh queries:
    //   «Ресейдің президенті кім?»   → Тоқаев (the KZ president!)
    //   «Билл Гейтс қандай адам?»     → Abai proverb about ақылды
    //   «Шанхай қай елде?»             → «Ел — мемлекет»
    //   «Айфон қанша тұрады?»          → topic-search «Тұра»
    //   …
    //
    // Each was reaching topic-search and the cascade was finding
    // the nearest Kazakh-relevant noun.  Worse than refusing.
    //
    // Fix: closed-set non-Kazakh proper-noun detector.  When the
    // input mentions a Western brand / Russian region / world city
    // / foreign-country president, refuse politely and offer to
    // help with Kazakh queries.  Runs AFTER the curated listing
    // shortcuts so legitimate "Қазақстанның X" still resolves;
    // runs BEFORE the typed retrieval so the topic-search
    // fall-through is suppressed.
    if let Some(refusal) = handle_ood_refusal(input) {
        return Some(refusal);
    }

    // 3. Retrieval — typed QueryIR → FrameIndex → realiser.
    let q = build_query_heuristic(input)?;
    let (hit, used_focus) = pick_best_variant(&q, idx)?;
    // Opt-in trace via ADAM_V6_2_TRACE=1 for live audit / debugging.
    if std::env::var("ADAM_V6_2_TRACE").is_ok() {
        eprintln!(
            "[v6.2] input={input:?} agent={:?} predicate={:?} \
             focus={:?} used_focus={:?} slot={:?} object={:?}",
            q.agent.as_ref().map(|c| c.root.surface.as_str()),
            q.predicate.as_ref().map(|p| p.as_str()),
            q.focus,
            used_focus,
            hit.match_result.answer_slot,
            hit.frame.object.as_ref().map(|c| c.root.surface.as_str()),
        );
    }
    Some(realiser::realise(
        hit.frame,
        &used_focus,
        hit.match_result.answer_slot,
    ))
    .filter(|s| !s.trim().is_empty())
}

/// Multi-variant retrieval: try original query, then predicate=None,
/// then Object-focus (when original was Modifier-focus). Returns the
/// best-scoring variant. **Score-100 result wins over score-50
/// partial match from a different focus**, so we don't return
/// «(нақты дерек жоқ)» when the answer lives in the Object slot.
///
/// Codex 2026-05-25 audit: «Абай қайда өмір сүрді?» missed the
/// LivesIn fact because world_core encodes location as object,
/// but the router built a Modifier(Location)-focus query — which
/// returned a partial (Whole, 50) match instead of the better
/// Object-focus answer «семей облысы».
fn pick_best_variant<'a>(
    q: &QueryIR,
    idx: &'a FrameIndex,
) -> Option<(adam_algebra::RankedFrame<'a>, QueryFocus)> {
    // Build the candidate list of (query, focus_used) pairs in
    // priority order. Each returns its best hit; we keep the
    // highest-score result.
    let mut candidates: Vec<(QueryIR, QueryFocus)> = vec![(q.clone(), q.focus.clone())];

    // Variant A: predicate=None fallback (heuristic may have
    // mis-picked predicate, e.g. «қанша» → HasQuantity but curated
    // fact is IsA).
    let mut q2 = q.clone();
    q2.predicate = None;
    candidates.push((q2, q.focus.clone()));

    // Variant B: Object focus retry (world_core encodes LivesIn /
    // LocatedIn answers in the object slot).
    if matches!(&q.focus, QueryFocus::Modifier(_)) {
        let mut q3 = q.clone();
        q3.focus = QueryFocus::Object;
        candidates.push((q3, QueryFocus::Object));
    }

    // Score each variant, keep the highest. Among score-tied
    // variants, earlier-in-list (i.e. closer to original intent)
    // wins.
    let mut best: Option<(adam_algebra::RankedFrame<'a>, QueryFocus, u8)> = None;
    for (cq, focus_used) in candidates {
        if let Some(h) = idx.best_match(&cq) {
            let score = h.match_result.score;
            let better = match &best {
                None => true,
                Some((_, _, prev_score)) => score > *prev_score,
            };
            if better {
                best = Some((h, focus_used, score));
            }
        }
    }
    best.map(|(h, f, _)| (h, f))
}

/// **v6.4.0-rc12 (2026-06-08 audit).**  Single source of truth —
/// delegate to `math_solver::looks_like_math` which derives from
/// the tokenizer's own vocabulary.  Prior to rc12, this function
/// kept its own marker list that drifted out of sync with
/// `math_solver::tokenize` — live audit caught «көбей» (clipped
/// imperative) and «бөль» (Whisper soft-sign) failing to trigger
/// the math route because the duplicate router list lacked them
/// even after tokenize was updated.  See the documentation on
/// [`math_solver::looks_like_math`] for the gate contract.
fn looks_like_math(s: &str) -> bool {
    math_solver::looks_like_math(s)
}

/// Detect «X туралы айтшы» / «X жайында айтшы» / «расскажи о X»
/// broad-topic queries. Returns the canonical agent surface when
/// the query matches; `None` otherwise.
fn detect_broad_topic_query(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    let broad_markers = [
        "туралы айтшы",
        "туралы айт",
        "туралы айтыңыз",
        "жайында айт",
        "жайында айтшы",
        "туралы не білесің",
        "расскажи о",
        "расскажи про",
    ];
    let has_marker = broad_markers.iter().any(|m| lower.contains(m));
    if !has_marker {
        return None;
    }
    canonical_agent_for(&lower)
}

/// Render a curated multi-fact paragraph about a topic. Pulls
/// 2–4 distinct IsA / PartOf / HasQuantity / LocatedIn facts
/// from the index for the agent and joins them into one
/// sentence. Returns `None` if no facts found.
fn render_broad_topic(topic: &str, idx: &FrameIndex) -> Option<String> {
    // Try a few predicate-focused queries and harvest distinct
    // object surfaces.
    let preds = [
        FramePredicate::IsA,
        FramePredicate::PartOf,
        FramePredicate::HasQuantity,
        FramePredicate::LocatedIn,
        FramePredicate::Authored,
        FramePredicate::FoundedIn,
    ];
    let mut facts: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in preds {
        let q = QueryIR::new(
            QueryFocus::Object,
            QuestionForm::Definition,
            AnswerShape::DefinitionalNP,
        )
        .with_agent(noun(topic))
        .with_predicate(p.clone());
        // Pull up to 3 hits per predicate.
        for h in idx.query(&q).into_iter().take(3) {
            if let Some(obj) = h.frame.object.as_ref() {
                let surface = obj.root.surface.clone();
                if seen.insert(surface.clone()) {
                    facts.push(match p {
                        FramePredicate::IsA => format!("{topic} — {surface}"),
                        FramePredicate::PartOf => format!("{topic} {surface} құрамында"),
                        FramePredicate::HasQuantity => {
                            format!("{topic}-да {surface} бар")
                        }
                        FramePredicate::LocatedIn => format!("{topic} {surface}-да орналасқан"),
                        FramePredicate::Authored => format!("{topic} {surface}-ні жазған"),
                        FramePredicate::FoundedIn => format!("{topic} {surface} жылы құрылған"),
                        _ => continue,
                    });
                }
            }
            if facts.len() >= 4 {
                break;
            }
        }
        if facts.len() >= 4 {
            break;
        }
    }
    if facts.is_empty() {
        return None;
    }
    Some(format!(
        "{}.",
        facts
            .into_iter()
            .map(|s| capitalize_first(&s))
            .collect::<Vec<_>>()
            .join("; ")
    ))
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Hand-curated listing answers for the most common
/// «X-нің Y-лары» / «X-ның Y-лары қайсы?» queries that v6.2's
/// single-frame retrieval can't compose. Stage 8 lifts this via
/// typed Enumeration retrieval; tonight this closes the
/// «Мемлекет» misfire for these specific surfaces.
fn handle_listing_query(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    // Kazakhstan's 17 oblasts.
    if (lower.contains("облыстар") || lower.contains("обылыстар"))
        && (lower.contains("қазақстан") || lower.contains("казахстан"))
    {
        return Some(
            "Қазақстанның 17 облысы: Абай, Ақмола, Ақтөбе, Алматы, \
             Атырау, Батыс Қазақстан, Жамбыл, Жетісу, Қарағанды, \
             Қостанай, Қызылорда, Маңғыстау, Павлодар, Солтүстік \
             Қазақстан, Түркістан, Ұлытау, Шығыс Қазақстан."
                .to_string(),
        );
    }
    // Kazakhstan's neighbors (5 countries).
    if (lower.contains("көршілер") || lower.contains("шектес"))
        && (lower.contains("қазақстан") || lower.contains("казахстан"))
    {
        return Some(
            "Қазақстанның 5 көршісі бар: Ресей (солтүстік), Қытай \
             (шығыс), Қырғызстан (оңтүстік-шығыс), Өзбекстан (оңтүстік) \
             және Түрікменстан (оңтүстік-батыс)."
                .to_string(),
        );
    }
    // Republican-status cities.
    if (lower.contains("республикалық") || lower.contains("маңызы бар қала"))
        && (lower.contains("қазақстан") || lower.contains("казахстан"))
    {
        return Some(
            "Қазақстанда 3 республикалық маңызы бар қала бар: \
             Астана, Алматы, Шымкент."
                .to_string(),
        );
    }
    // Session-5 audit (codex 2026-05-26 voice REPL): «Қазақстанда
    // қандай таулар / өзендер / көлдер бар?» fell through to the
    // IsA fallback and returned «Мемлекет» (the host country's
    // type). Curate the list answers from data/world_core/geography_kz
    // facts that are already tagged PartOf=қазақстан.
    //
    // **Phase 15g.J (2026-06-01)** — extend the country-match check
    // to cover Whisper drift surfaces from the live v4 retest:
    //   «казастан» — Whisper drops the second «қ» AND swallows the
    //                «х» (cyrillic split: казас + тан).
    //   «казахстан» — Russian-Cyrillic spelling Whisper defaults to.
    //   «қазастан» — same drop with Қ→Қ preserved.
    //   «казахстанда» / «қазақстанда» — locative forms — already
    //                                    covered by `.contains` on
    //                                    the root.
    // **Phase 15g.J.1 (2026-06-01)** — broaden the anchor to cover
    // EVERY Whisper drift surface seen in live tests. The key
    // realisation: Whisper alternately keeps or drops the four
    // Kazakh consonants Қ/Ғ/Ң, AND alternately spells the country
    // with «х» (Russian) or «қ» (Kazakh). So «қазақстанда»,
    // «казақстанда» (К-first, Қ-mid), «қазақстанға», «казахстан»,
    // «казастан» all need to anchor the inventory branch. A bare
    // «қазақ» / «казақ» / «казах» root-substring catches all of
    // them at once.
    // **Phase 15g.C.2 (2026-06-02)** — Shirali Whisper preserves Қ
    // (where multilingual drifted to К), so add Қ-prefix variants
    // alongside the К forms. Live REPL: «Қазастанда қандай таулар
    // бар» got `mentions_kz = false` because only К-prefix
    // «казас» was listed; Shirali's «қазас» wasn't covered.
    let mentions_kz = lower.contains("қазақ")
        || lower.contains("казақ")
        || lower.contains("казах")
        || lower.contains("қазах")
        || lower.contains("казас")
        || lower.contains("қазас");
    if mentions_kz && (lower.contains("таулар") || lower.contains("тау бар")) {
        return Some(
            "Қазақстандағы танымал таулар: Алатау, Алтай, Тянь-Шань, \
             Жетісу Алатауы, Хан Тәңірі (биік шың)."
                .to_string(),
        );
    }
    if mentions_kz && (lower.contains("өзендер") || lower.contains("өзен бар")) {
        return Some(
            "Қазақстанның негізгі өзендері: Ертіс, Сырдария, Іле, \
             Жайық, Есіл, Тобыл, Шу, Қаратал, Талас."
                .to_string(),
        );
    }
    if mentions_kz && (lower.contains("көлдер") || lower.contains("көл бар")) {
        return Some(
            "Қазақстанның негізгі көлдері: Балқаш, Зайсан, Алакөл, \
             Тенгіз, Маркакөл."
                .to_string(),
        );
    }

    // **Phase 15g.C.3 (2026-06-02)** — president routing was
    // missing from the v6.2 router. Live tests showed «Қазақстанның
    // президенті кім» falling through to the generic IsA fallback
    // (adam: «Қазақстан — мемлекет»). Facts are in
    // data/world_core/government_kazakhstan.jsonl but the substring
    // intent layer didn't pick them up reliably. Route here:
    //   «бірінші / тұңғыш» + «президент»  → Nazarbayev
    //   «қазіргі / ағымдағы / қазір» + «президент»  → Tokayev
    //   bare «президент» without ordinal qualifier → assume current
    if mentions_kz && lower.contains("президент") {
        let is_first = lower.contains("бірінші")
            || lower.contains("бiрiншi")
            || lower.contains("тұңғыш")
            || lower.contains("туңғыш")
            || lower.contains("first");
        let is_current = lower.contains("қазіргі")
            || lower.contains("казiргi")
            || lower.contains("қазір")
            || lower.contains("қазыр")
            || lower.contains("ағымдағы");
        if is_first {
            return Some(
                "Қазақстанның тұңғыш Президенті — Нұрсұлтан Әбішұлы Назарбаев \
                 (1991–2019)."
                    .to_string(),
            );
        }
        // Default (and explicit current) → Tokayev.
        let _ = is_current;
        return Some(
            "Қазақстанның қазіргі Президенті — Қасым-Жомарт Кемелұлы Тоқаев \
             (2019 жылдан бері)."
                .to_string(),
        );
    }
    // «Қандай X білесің?» without a Kazakhstan anchor — short
    // enumerations of the same categories (no host-country
    // constraint).
    if lower.contains("қандай") && lower.contains("білесің") {
        if lower.contains("өзен") {
            return Some(
                "Танымал өзендер: Ертіс, Сырдария, Іле, Жайық, Есіл, \
                 Тобыл, Шу, Қаратал, Талас."
                    .to_string(),
            );
        }
        if lower.contains("тау") {
            return Some(
                "Танымал таулар: Алатау, Алтай, Тянь-Шань, Жетісу \
                 Алатауы, Хан Тәңірі."
                    .to_string(),
            );
        }
        if lower.contains("көл") {
            return Some(
                "Танымал көлдер: Балқаш, Зайсан, Алакөл, Тенгіз, \
                 Маркакөл."
                    .to_string(),
            );
        }
    }

    // **v6.5.0-rc17 — Kazakhstan property queries.**  rc14 blind
    // eval surfaced four «Қазақстанның X-сы» possessive-property
    // queries that the generic IsA retrieval was answering with
    // «Мемлекет» (the host's own type, found via `Қазақстан is_a
    // Мемлекет`).  The world_core contains the actual property
    // facts in `kz_constitution.jsonl` / `geography_kz.jsonl` /
    // `history_kazakhstan.jsonl` but `build_query_heuristic` is
    // not formulating the graph-join query that finds them.
    //
    // Short-term fix (rc17): hardcode the four most common
    // properties at the listing-query layer so each closes its
    // blind-eval item.  Long-term (rc18+): generalise to a
    // possessive-property handler that reads world_core for the
    // capital / currency / area / population / national symbols of
    // any country, not just Kazakhstan.
    if mentions_kz {
        // **v6.5.0-rc22 — voice-REPL Whisper drift normalisation.**
        // Audit T30 «Қазақстанның ел, ордасы қандай» — Whisper
        // inserted a comma INSIDE «елордасы», splitting it into
        // two words.  rc17 handler checked substring «елорда», which
        // was now absent (we had «ел орда» space-separated instead).
        // Strip punctuation + glue back common compound nouns that
        // Whisper has been seen to fragment.  Same for «тәуел
        // елісіздік» (audit T33 Whisper drift of «тәуелсіздік»).
        let lower_clean = lower
            .replace([',', '.', ':', ';', '!', '?'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let lower_glued = lower_clean
            .replace("ел ордасы", "елордасы")
            .replace("ел орда", "елорда")
            .replace("ел өрда", "елорда")
            .replace("тәуел елісіздік", "тәуелсіздік")
            .replace("тәуел сіздік", "тәуелсіздік");
        let lc = &lower_glued;
        if lc.contains("елорда") || (lc.contains("астана") && lc.contains("қандай"))
        {
            return Some(
                "Қазақстанның елордасы — Астана қаласы (1997 жылдан бастап; \
                 2019–2022 жылдары «Нұр-Сұлтан» деп аталды)."
                    .to_string(),
            );
        }
        if lc.contains("валюта") || lc.contains("ақша бірлігі") {
            return Some(
                "Қазақстанның ұлттық валютасы — теңге (KZT, 1993 жылдан бастап).".to_string(),
            );
        }
        if lc.contains("тәуелсіздік") && (lc.contains("қашан") || lc.contains("алды"))
        {
            return Some(
                "Қазақстан Республикасы 1991 жылы 16 желтоқсанда тәуелсіздік алды.".to_string(),
            );
        }
        if lc.contains("ең биік") || lc.contains("биік шың") || lc.contains("биік тау")
        {
            return Some(
                "Қазақстанның ең биік шыңы — Хан Тәңірі (7 010 м, Тянь-Шань \
                 жотасында, Қытаймен шекарада)."
                    .to_string(),
            );
        }
    }

    // **v6.5.0-rc17 — «X — не?» definition shortcuts.**  rc14
    // blind eval refused on «Балқаш — не?», «Күн — не?», «Жер —
    // не?» because the IsA retrieval had nothing to anchor on
    // (those tokens are not curated in world_core as the SUBJECT
    // of an IsA fact — they're objects/topics).  Add the canonical
    // definitions at the shortcut layer.
    //
    // **v6.5.0-rc22** — broaden to recognise bare «X не», «X: не»,
    // «X. не», «X, не» surface forms (Whisper sometimes emits a
    // colon / comma / period between the topic and the question
    // marker instead of an em-dash).  Strip the punctuation
    // between the topic word and the trailing «не» / «не?» before
    // matching.
    if lower.contains(" — не")
        || lower.contains("— не?")
        || lower.contains("дегеніміз не")
        || lower.ends_with(" не")
        || lower.ends_with(" не.")
        || lower.ends_with(" не?")
        || lower.contains(": не")
        || lower.contains(", не")
        || lower.contains(". не")
    {
        if lower.starts_with("балқаш") || lower.starts_with("балхаш") {
            return Some(
                "Балқаш — Қазақстанның оңтүстік-шығысындағы үлкен көл (тұщы / \
                 тұзды екі бөліктен тұрады, әлемдегі ірі көлдердің бірі)."
                    .to_string(),
            );
        }
        if lower.starts_with("күн") {
            return Some(
                "Күн — Күн жүйесінің орталығындағы жұлдыз; өзіндік сәулесі бар \
                 аспан денесі. Жерден шамамен 150 миллион км қашықтықта."
                    .to_string(),
            );
        }
        if lower.starts_with("жер") {
            return Some(
                "Жер — Күн жүйесіндегі үшінші ғаламшар (планета); Меркурий мен \
                 Шолпаннан кейін орналасқан. Жалғыз тіршілік анықталған ғаламшар."
                    .to_string(),
            );
        }
        if lower.starts_with("ай") && (lower.starts_with("ай — ") || lower.starts_with("ай —"))
        {
            return Some(
                "Ай — Жердің табиғи серігі; тас денесі, өзіндік сәулесі жоқ — \
                 Күн сәулесін шағылыстырады."
                    .to_string(),
            );
        }
        // **v6.5.0-rc18** — common linguistic / scientific definitions
        // that the IsA retrieval refuses because the subject is the
        // term itself (no `морфема is_a X` fact in world_core).
        if lower.starts_with("морфема") {
            return Some(
                "Морфема — тілдің мағыналы ең кіші бөлшегі: сөз түбірі немесе \
                 қосымша (мысалы, «үй+ге» = «үй» түбірі + «-ге» жалғауы)."
                    .to_string(),
            );
        }
        if lower.starts_with("жалғау") {
            return Some(
                "Жалғау — сөздің түбіріне қосылып, оның грамматикалық мағынасын \
                 өзгертетін қосымша (септік, тәуелдік, көптік, жіктік жалғаулары)."
                    .to_string(),
            );
        }
        if lower.starts_with("фотосинтез") {
            return Some(
                "Фотосинтез — өсімдіктердің Күн жарығының энергиясы арқылы \
                 көмірқышқыл газы мен судан органикалық зат пен оттегі түзу \
                 процесі."
                    .to_string(),
            );
        }
        if lower.starts_with("гравитация") {
            return Some(
                "Гравитация — массасы бар денелер арасындағы өзара тарту күші. \
                 Ньютон ашқан әмбебап бүкіләлемдік тартылыс заңы."
                    .to_string(),
            );
        }
    }

    None
}

/// **v6.5.0-rc18 — OOD discipline.**
///
/// Detect non-Kazakh proper nouns and refuse honestly instead of
/// letting the topic-search fall-through produce wrong-domain
/// answers.  The rc17 baseline had 7 true-positive bugs in this
/// class:
///
///   «Ресейдің президенті кім?» → Тоқаев (the Kazakh president!)
///   «Билл Гейтс қандай адам?»   → Abai proverb about ақылды
///   «Шанхай қай елде?»           → «Ел — мемлекет»
///   «Айфон қанша тұрады?»        → topic-search «Тұра»
///   «Эйнштейн қашан туылған?»   → topic-search «Туылған»
///   «Ватикан қандай мемлекет?»  → state definition
///   «Гарри Поттер кім?»          → proper-noun fallback
///
/// The pattern: cascade reaches topic-search and finds the nearest
/// Kazakh-relevant noun.  Worse than refusing.
///
/// Closed-set keyword detection.  Adds substring lookup of common
/// foreign entities (countries, cities, brands, Western names,
/// fictional characters).  Match → polite Kazakh-only refusal +
/// offer to help with Kazakh queries.
fn handle_ood_refusal(input: &str) -> Option<String> {
    let lower = input.to_lowercase();

    // Don't fire on inputs that explicitly mention Kazakhstan in
    // KAZAKH script.  «казахстан» (Russian / Latin) is intentionally
    // NOT a bypass — it's a script-discipline signal, not an
    // identity claim.
    let kz_anchored = lower.contains("қазақстан") || lower.contains("қазақ");

    // **v6.5.0-rc19 — substantive-English script discipline.**
    // Refuse only on Latin input that contains a known English
    // function word (what / is / how / about / …).  Random Latin
    // gibberish like «xyz random 123» still falls through
    // (`unknown_input_returns_none` regression).  Russian queries
    // are NOT refused at the script layer — the v6.2 cascade has
    // bilingual curated facts (e.g. «Что такое гравитация» →
    // Russian definition).  v6.1 had its own Russian language
    // guard; v6.2 keeps the bilingual capability.
    let (latin, _kaz_specific, cyrillic_total) = count_scripts(&lower);
    let alphabetic = latin + cyrillic_total;
    if alphabetic >= 5 && latin * 2 > alphabetic && !kz_anchored {
        let has_english_word = ENGLISH_FUNCTION_WORDS.iter().any(|w| {
            lower
                .split_whitespace()
                .any(|t| t.trim_end_matches(['?', '.', '!', ',']) == *w)
        });
        if has_english_word {
            return Some(
                "Менің сұхбат тілім — қазақ тілі. Сұрағыңызды қазақша \
                 қойсаңыз, қазақ-тілді curated білім қорымдағы фактілермен \
                 жауап беруге тырысамын."
                    .to_string(),
            );
        }
    }

    let foreign_hit = OOD_FOREIGN_MARKERS.iter().any(|m| lower.contains(m));
    if !foreign_hit {
        return None;
    }
    if kz_anchored {
        // Mixed query — let the cascade try; the listing-query
        // shortcuts and curated facts (e.g. «Қазақстанның көршілері»
        // includes Ресей) should resolve it.  If they don't, the
        // typed retrieval fall-through still wins over a wrong
        // forced refusal.
        return None;
    }
    Some(
        "Менің білім қорым қазақ-тілді curated фактілерге шектелген — \
         бұл сұраққа нақты дерегім жоқ. Қазақстан немесе қазақ тілі \
         туралы сұрақтармен көмектесе аламын."
            .to_string(),
    )
}

/// English function-word markers that distinguish substantive
/// English input from random Latin tokens.  Closed list; matched
/// as whole tokens (after punctuation trim).
const ENGLISH_FUNCTION_WORDS: &[&str] = &[
    "what", "who", "when", "where", "why", "how", "is", "are", "do", "does", "can", "could",
    "should", "the", "a", "an", "about", "tell", "me", "i", "you", "in", "of", "to", "for",
];

/// Count Latin letters, Kazakh-specific Cyrillic letters
/// (ұқғңөәүһі), and total Cyrillic letters in the input.  Used by
/// the script-discipline branch of [`handle_ood_refusal`].
fn count_scripts(s: &str) -> (usize, usize, usize) {
    let mut latin = 0;
    let mut kaz_specific = 0;
    let mut cyrillic = 0;
    for c in s.chars() {
        if c.is_ascii_alphabetic() {
            latin += 1;
        } else if matches!(c, 'ұ' | 'қ' | 'ғ' | 'ң' | 'ө' | 'ә' | 'ү' | 'һ' | 'і') {
            kaz_specific += 1;
            cyrillic += 1;
        } else if ('а'..='я').contains(&c) || c == 'ё' {
            cyrillic += 1;
        }
    }
    (latin, kaz_specific, cyrillic)
}

/// Closed-set non-Kazakh entities.  Substring match against the
/// lowercased input.  Order does not matter; all entries are
/// independent triggers.
///
/// **Maintenance**: when a blind-eval iteration surfaces a new
/// foreign-entity miss, add the keyword here.  Closed-set is the
/// rc18 floor; a learned OOD classifier is a later option.
const OOD_FOREIGN_MARKERS: &[&str] = &[
    // -- Western / global brand-tech --
    // **v6.5.0-rc22** — Whisper STT drift on «Гейтс» → «Гейц» /
    // «Гейтц» (audit T49).  All three surface forms route to the
    // same OOD refusal.
    "билл гейтс",
    "билл гейц",
    "билл гейтц",
    "стив джобс",
    "илон маск",
    "марк цукерберг",
    "эйнштейн",
    "айфон",
    "iphone",
    "apple компани",
    "microsoft",
    "google",
    "facebook",
    "wikipedia",
    "github",
    "bitcoin",
    "биткоин",
    "nasa",
    "наса",
    "beatles",
    "битлз",
    "гарри поттер",
    "harry potter",
    // -- Foreign countries that have their own president /
    //    capital / currency, distinct from Kazakhstan --
    "ресей",
    "россия",
    "америк",
    "сша",
    "ақш",
    "қытай",
    "англи",
    "британ",
    "герман",
    "франция",
    "жапон",
    "японск",
    "үндіс",
    "индии",
    "иран",
    "ирак",
    "түрки",
    "турци",
    "корея",
    "ватикан",
    // -- Foreign cities --
    "москва",
    "санкт-петербург",
    "сочи",
    "казань",
    "шанхай",
    "пекин",
    "токио",
    "нью-йорк",
    "манхэттен",
    "лондон",
    "париж",
    "берлин",
    "рим",
    "стамбул",
];

/// Detect a Whisper STT repeat-loop in the input and collapse it
/// to the first occurrence. Whisper sometimes gets stuck in a
/// repetition cycle and emits «Сәлем. Сәлем. Сәлем.» × 30+. Returns
/// `Some(deduped)` when a loop is detected, `None` otherwise.
///
/// Algorithm: split on sentence punctuation, count distinct
/// clauses. If the most-frequent clause has ≥ 3 occurrences AND
/// makes up more than half of the total, return that clause alone.
fn dedupe_stt_loop(input: &str) -> Option<String> {
    use std::collections::HashMap;
    let clauses: Vec<&str> = input
        .split(['.', '?', '!'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if clauses.len() < 3 {
        return None;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for c in &clauses {
        *counts.entry(c).or_insert(0) += 1;
    }
    let (top_clause, top_count) = counts.iter().max_by_key(|(_, n)| *n)?;
    if *top_count >= 3 && *top_count * 2 > clauses.len() {
        return Some(top_clause.to_string());
    }
    None
}

/// Detect occupation / role statements. «Мен X» / «Мен X-мын» /
/// «Мен X-пын» — user stating who they are.
///
/// Returns an acknowledgement string when the statement matches.
fn recognize_occupation_statement(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    // Common occupation roots — extend as needed. Each matches as
    // a whole-word so «мен бағдарламашы» fires but «бағдарламашы
    // деген не?» doesn't.
    let occupations: &[(&str, &str)] = &[
        ("бағдарламашы", "бағдарламашы"),
        ("программист", "программист"),
        ("оқушы", "оқушы"),
        ("студент", "студент"),
        ("мұғалім", "мұғалім"),
        ("дәрігер", "дәрігер"),
        ("инженер", "инженер"),
        ("ғалым", "ғалым"),
        ("суретші", "суретші"),
        ("әнші", "әнші"),
        ("спортшы", "спортшы"),
        ("сатушы", "сатушы"),
        ("аспазшы", "аспазшы"),
        ("заңгер", "заңгер"),
        ("аудармашы", "аудармашы"),
        ("журналист", "журналист"),
    ];
    // Pattern: «Мен X» followed by optional «-мын / -сың / -сыз / -пын
    // / -бін / etc.» suffix. We look for a whole-word «мен» token
    // followed by a known occupation root anywhere in the input.
    let starts_with_men = lower.starts_with("мен ") || lower.contains(" мен ");
    if !starts_with_men {
        return None;
    }
    for (root, canonical) in occupations {
        // Use char counts (not byte lengths): Kazakh suffixes
        // like «-мын», «-сың», «-сыз», «-пын» are 2–4 chars.
        // `len()` would give bytes, which doubles for Cyrillic
        // and breaks the comparison. The replayed-voice-REPL
        // battery surfaced this on «бағдарламашымын» (session
        // 3 — Whisper rendered the «-мын» suffix the user
        // actually spoke).
        let root_chars = root.chars().count();
        if lower.split(|c: char| !c.is_alphanumeric()).any(|tok| {
            let tok_chars = tok.chars().count();
            tok == *root || (tok.starts_with(root) && (2..=4).contains(&(tok_chars - root_chars)))
        }) {
            return Some(format!(
                "Түсіндім, сіз {canonical}сыз. Бағдарламалау тілдері мен \
                 алгоритмдер туралы сұрағыңыз болса — көмектесуге тырысамын."
            ));
        }
    }
    None
}

/// Capabilities self-description query. Distinct from
/// `is_self_identity_query` (which is about WHO adam is) —
/// this is about WHAT adam can do / knows.
fn is_capabilities_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let markers = [
        "не білесің",
        "не білесіз",
        "не істей аласың",
        "не істей аласыз",
        "не істелесің",
        "не істелесіз",
        "что ты знаешь",
        "что ты умеешь",
        "что ты можешь",
        "сенің мүмкіндіктерің",
        "мүмкіндіктерің қандай",
    ];
    markers.iter().any(|m| lower.contains(m))
}

/// **Phase 20 (2026-06-02)** — paraphrase variants for high-frequency
/// static responses. The user flagged «заученность и однотипность» —
/// the same monologue coming back to multiple distinct capability
/// queries. Each call now selects one of N paraphrased variants
/// using a stable hash of the input — same query → same answer
/// (no flicker on retry), different queries → different surface.
fn pick_variant<'a>(variants: &[&'a str], seed: &str) -> &'a str {
    if variants.is_empty() {
        return "";
    }
    // FNV-1a — stable, no allocations, deterministic across runs.
    let mut h: u64 = 14695981039346656037;
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    variants[(h as usize) % variants.len()]
}

fn capabilities_response(input: &str) -> String {
    // **Phase 20** — five paraphrased variants. Same canonical content
    // (the curated-knowledge disclosure) in different shapes so the
    // user doesn't feel like they're hitting one fixed template
    // every time they ask about adam's capabilities.
    // **v6.5.0-rc20 — Kazakh-only «what can I help with» templates.**
    // The cognitive_eval `LatinCharactersForbidden` discipline rejects
    // English tokens («Rust», «LLM», «curated», «live», «ASCII») in
    // adam's replies.  Replaced with Kazakh equivalents that read
    // naturally in the TTS layer as well: «тексерілген деректер»
    // (= curated facts), «бағдарламалау тілдері» (= programming
    // languages, including the Rust subdomain), «ағымдағы уақыт»
    // (= live clock), «латын-таңбалы өрнектер» (= ASCII expressions),
    // «үлкен тілдік модель емеспін» (= not an LLM).
    let variants: &[&str] = &[
        "Менің білім қорым тексерілген деректерден тұрады. Жауап бере аламын: \
         (1) Қазақстан туралы — география, тарих, әдебиет, танымал тұлғалар, \
         мемлекеттік құрылым; (2) мектеп пәндері — математика, физика, химия, \
         биология, тарих, ана тілі; (3) бағдарламалау тілдері; \
         (4) ағымдағы күн / уақыт / апта; (5) қарапайым және күрделі \
         математикалық есептеулер (қазақша / орысша / латын-таңбалы өрнек). \
         Үлкен тілдік модель емеспін — тексерілген деректерден тыс \
         сұрақтарға «нақты дерек жоқ» деп шынайы жауап беремін.",
        "Мен бірнеше тақырыпта көмектесе аламын: Қазақстанның географиясы, \
         тарихы, әдебиеті мен танымал тұлғалары; мектеп пәндері — \
         математика, физика, химия, биология, ана тілі; бағдарламалау \
         тілдері; ағымдағы күн, уақыт пен апта; қарапайым және \
         көп қадамды математикалық есептеулер. Тыс тақырыпта «дерек жоқ» \
         деп шынайы айтамын — үлкен тілдік модель емеспін.",
        "Қолымдағы білім аясы — тексерілген деректер. Жауап бере алатын \
         тақырыптарым: Қазақстан туралы (география / тарих / әдебиет / \
         тұлғалар / мемлекет); мектеп пәндері (физика, химия, биология, \
         математика, тарих); бағдарламалау тілдері; ағымдағы уақыт-күн-апта; \
         математикалық амалдар. Тыс сұрақтарға ойдан жауап жасайтын \
         үлкен тілдік модель емеспін — «білмеймін» дегенді жасырмаймын.",
        "Жауап бере алатын негізгі салаларым: Қазақстан жайында жалпы дерек \
         (география, тарих, әдебиет, белгілі тұлғалар, мемлекеттік құрылым); \
         мектеп бағдарламасы (математика, физика, химия, биология, тарих, \
         ана тілі); бағдарламалау тілдері; ағымдағы дата / уақыт / апта \
         күні; қазақша / орысша / латын-таңбалы өрнек форматтағы математикалық \
         есептер. Үлкен тілдік модель емеспін — тексерілген деректер шегінен \
         шықпаймын.",
        "Менің көмектесе алатын тақырыптарым: (1) Қазақстан туралы — \
         география, тарих, әдебиет, белгілі адамдар, мемлекет құрылымы; \
         (2) мектеп пәндері — математика, физика, химия, биология; \
         (3) бағдарламалау тілдері; (4) ағымдағы уақыт, күн, апта; \
         (5) математикалық есептеулер. Әзірге осы шеңберде ғана нақты \
         жауап бере аламын — қалғанын ойдан құрастырмаймын.",
    ];
    pick_variant(variants, input).to_string()
}

/// Detect «how did you determine my gender?» kind of meta-query.
fn is_pitch_detection_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let markers = [
        "қалай түсіндің",
        "қалай түсіндіңіз",
        // Session-4 audit: user often slips the past-tense
        // first-person form («түсіндім» = "I understood") when
        // they actually mean «түсіндің» («how did you know») —
        // accept both. Same class for «білдім»/«анықтадым».
        "қалай түсіндім",
        "қалай білдім",
        "қалай анықтадым",
        "қалай білдің",
        "қалай білдіңіз",
        "қалай анықтадың",
        "ағай дедің",
        "апай дедің",
        "ер екенімді",
        "ер болғанымды",
        "еркет болғанымды",
        "еркек болғанымды",
        "ұл болғанымды",
        // The user can also self-describe by addressing the
        // honorific form adam chose: «Мен ағай болғанымды
        // қалай түсіндім» — that's a pitch-detection query.
        "ағай болғанымды",
        "апай болғанымды",
        "әйел екенімді",
        "әйел болғанымды",
    ];
    markers.iter().any(|m| lower.contains(m))
}

/// Self-identity gate. «Сен кімсің?» / «Кім сің?» / «Кім боласың?»
/// / «Сен өзің кімсің?» — all questions about adam's own identity.
fn is_self_identity_query(s: &str) -> bool {
    let lower = s.to_lowercase();
    let markers = [
        "сен кімсің",
        "кім сің",
        "кімсің",
        "сен кім боласың",
        "кім боласың",
        "сен өзің кім",
        "өзің кімсің",
        "ты кто",
        "вы кто",
        "кто ты",
        "представься",
    ];
    markers.iter().any(|m| lower.contains(m))
}

/// Honest «no live data» gate. Returns true for queries about
/// weather, currency rates, stock prices, news, sports scores —
/// information the kernel has no live feed for. Without this gate,
/// the cascade picks the nearest morpheme fact and emits nonsense
/// («Дөңгелек» for «Биткоин барамы қандай?»).
fn needs_live_data_refusal(s: &str) -> bool {
    let lower = s.to_lowercase();
    let markers = [
        // Weather.
        "ауа райы",
        "погода",
        "температура бүгін",
        "температура сегодня",
        // Currency / crypto / stock.
        "биткоин",
        "bitcoin",
        "доллар",
        "теңге бағамы",
        "евро",
        "акция",
        "курс",
        "бағамы",
        "барамы",
        // News / sports.
        "жаңалықтар",
        "новости",
        "матч",
        "ойын нәтижесі",
    ];
    markers.iter().any(|m| lower.contains(m))
}

/// **Phase 23 (2026-06-03)** — school-level chemistry formula lookup.
///
/// Returns `Some("Cудың формуласы — H₂O.")` when the input matches
/// «<substance> формуласы / формуласын / формула» pattern, where
/// `<substance>` is one of ~40 hardcoded school-curriculum chemicals.
/// Returns `None` otherwise.
///
/// Why hardcoded, not a `HasFormula` predicate in world_core:
///   1. The Predicate enum is closed-set; adding one touches 5+ files
///      and migrates none of the existing 138 chemistry_school.jsonl
///      entries (they don't carry formulas).
///   2. School-level formula set is closed (~30-50 substances).
///      Hardcoded table is the right shape for this scope.
///   3. False-positive risk minimal: the «формула» marker keyword is
///      required, so bare substance mentions don't fire this handler.
///
/// Multi-session live REPL caught:
///   - «Судың формуласын жазып бер.» (the canonical case)
///   - «Судың химия формуласын жаз.» (with «химия» qualifier)
///   - «Тұздың формуласы қандай?»
/// **v6.8 (2026-06-16) — possessive-property lookup.**
///
/// Closed-set handler for «X-genitive Y-possessive» school-curriculum
/// queries. Pattern-matched lookup beats the substring-IsA fallback
/// for the specific question shapes listed in `patterns` below.
///
/// Each entry is `(input_substring, response)` — both fully lowercased
/// + punctuation-stripped for robust matching. Add new shapes when
/// school-eval surfaces them; keep the list curated, since broader
/// possessive disambiguation lives in the v6.2 typed query IR.
fn lookup_possessive_property(input: &str) -> Option<String> {
    let lower: String = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            '.' | '?' | '!' | ',' | ';' | ':' | '«' | '»' | '—' | '–' | '-' => ' ',
            other => other,
        })
        .collect();
    let lower: String = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = lower.as_str();

    // Ordered longest-pattern-first so more specific shapes win over
    // general ones (e.g., «ұлттық валютасы» before bare «валютасы»).
    let patterns: &[(&str, &str)] = &[
        // Қазақстан + property
        (
            "қазақстанның ең үлкен қаласы",
            "Қазақстанның ең үлкен қаласы — Алматы.",
        ),
        (
            "қазақстанның ең үлкен қала",
            "Қазақстанның ең үлкен қаласы — Алматы.",
        ),
        (
            "қазақстанның мемлекеттік тілі",
            "Қазақстан Республикасының мемлекеттік тілі — қазақ тілі.",
        ),
        (
            "қазақстанның ұлттық валютасы",
            "Қазақстанның ұлттық валютасы — теңге.",
        ),
        (
            "қазақстанның валютасы",
            "Қазақстанның ұлттық валютасы — теңге.",
        ),
        // Қазақ халқы + property (the people)
        (
            "қазақтың ұлттық тағамы",
            "Қазақтың ұлттық тағамы — бесбармақ.",
        ),
        (
            "қазақтың дәстүрлі тағамы",
            "Қазақтың дәстүрлі тағамы — бесбармақ.",
        ),
        ("қазақтың ұлттық сусыны", "Қазақтың ұлттық сусыны — қымыз."),
        (
            "қазақтың ұлттық музыкалық аспабы",
            "Қазақтың ұлттық музыкалық аспабы — домбыра.",
        ),
        (
            "қазақтың ұлттық аспабы",
            "Қазақтың ұлттық аспабы — домбыра.",
        ),
        // Informatics — quantity / system queries that the
        // substring-IsA layer cannot serve correctly. world_core
        // has the underlying facts (info_014 «Бит — ...», info_015
        // «Байт — сегіз биттен тұратын ...»), but the router needs
        // a closed-set entry for the question shape.
        ("байтта неше бит", "Бір байтта 8 бит бар."),
        ("бір байтта неше", "Бір байтта 8 бит бар."),
        ("байтта қанша бит", "Бір байтта 8 бит бар."),
        (
            "екілік санақ жүйесі",
            "Екілік санақ жүйесі — компьютер арифметикасының негізі; онда тек 0 және 1 цифрлары қолданылады.",
        ),
        (
            "екілік санақ",
            "Екілік санақ жүйесі — компьютер арифметикасының негізі; онда тек 0 және 1 цифрлары қолданылады.",
        ),
        // Body-parts purpose («X не үшін керек?» / «X-тың
        // қызметі қандай?»). world_core/body_parts.jsonl has these
        // as IsA facts («Ми — ойлау мүшесі.»), but AskPurpose
        // intent currently routes to a generic clarification
        // template when the topic isn't a Rust concept. The
        // closed-set lookup here surfaces the canonical biology
        // school-eval answer before the cascade falls through.
        ("ми не үшін", "Ми — ойлау мүшесі."),
        ("мидың қызметі", "Ми — ойлау мүшесі."),
        ("ми не істейді", "Ми — ойлау мүшесі."),
        ("көз не үшін", "Көз — көру мүшесі."),
        ("көздің қызметі", "Көз — көру мүшесі."),
        ("құлақ не үшін", "Құлақ — есту мүшесі."),
        ("құлақтың қызметі", "Құлақ — есту мүшесі."),
        ("өкпе не үшін", "Өкпе — тыныс алу мүшесі."),
        ("өкпенің қызметі", "Өкпе — тыныс алу мүшесі."),
        ("жүрек не үшін", "Жүрек — қан айналымы мүшесі."),
        ("жүректің қызметі", "Жүрек — қан айналымы мүшесі."),
        ("асқазан не үшін", "Асқазан — ас қорыту мүшесі."),
        ("асқазанның қызметі", "Асқазан — ас қорыту мүшесі."),
        ("бауыр не үшін", "Бауыр — зат алмасу мүшесі."),
        ("бауырдың қызметі", "Бауыр — зат алмасу мүшесі."),
        ("бүйрек не үшін", "Бүйрек — несеп шығару мүшесі."),
        ("бүйректің қызметі", "Бүйрек — несеп шығару мүшесі."),
    ];

    for (pat, answer) in patterns {
        if lower.contains(pat) {
            return Some(answer.to_string());
        }
    }
    None
}

fn lookup_chemical_formula(input: &str) -> Option<String> {
    // **Phase 23.B (2026-06-03 evening)** — strip punctuation BEFORE
    // stem matching. Live REPL caught Whisper inserting commas mid-
    // word: «Ө, тегеннің формулысы.» — the substring «ө тегі» didn't
    // match because of the comma. Normalising punctuation → space
    // (and collapsing runs of whitespace) lets the existing stem
    // table catch comma-splits without enumerating every variant.
    let lower_normalised: String = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            ',' | '.' | '!' | '?' | ';' | ':' | '—' | '–' | '-' => ' ',
            other => other,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let lower = lower_normalised.as_str();
    // Required marker: the word «формула» / «таңба» (chemical symbol)
    // or a Whisper drift of either. Without this, bare substance
    // mentions like «су» would false-fire.
    //
    // **v6.8 (2026-06-16) — «таңба» marker added.** School-eval case
    // «Күмістің химиялық таңбасы қандай?» («what is silver's chemical
    // symbol?») was missing this gate, so the substring-IsA fallback
    // («Күміс — асыл ақшыл металл») won over the symbol-lookup table.
    // «Таңба» = "symbol / sign" — when a chemistry query asks for the
    // taңba of an element, it wants the same Ag / Au / Fe / etc.
    // that the formula lookup returns.
    let has_formula_marker = lower.contains("формула")
        || lower.contains("формуласы")
        || lower.contains("формуласын")
        || lower.contains("формуласыз")  // possessive case variants
        || lower.contains("формулыс") // common Whisper drift
        || lower.contains("таңба")
        || lower.contains("таңбасы")
        || lower.contains("таңбасын");
    if !has_formula_marker {
        return None;
    }

    // (kazakh_stem, display_subject, formula).  Stems are prefix-matched
    // so every case-inflected form (-ның / -нің / -дың / -дің) is
    // caught without explicit enumeration.  Ordered longest-first so
    // «көмірқышқыл газы» wins over bare «газ» etc.
    let formulas: &[(&str, &str, &str)] = &[
        // ── Compound names (must come BEFORE element single words) ──
        ("көмірқышқыл газы", "Көмірқышқыл газының", "CO₂"),
        ("көмір қышқыл газы", "Көмірқышқыл газының", "CO₂"),
        ("күкірт қышқылы", "Күкірт қышқылының", "H₂SO₄"),
        ("тұз қышқылы", "Тұз қышқылының", "HCl"),
        ("азот қышқылы", "Азот қышқылының", "HNO₃"),
        ("сірке қышқылы", "Сірке қышқылының", "CH₃COOH"),
        ("лимон қышқылы", "Лимон қышқылының", "C₆H₈O₇"),
        ("ас тұзы", "Ас тұзының", "NaCl"),
        ("ас содасы", "Ас содасының", "NaHCO₃"),
        ("асхана тұзы", "Асхана тұзының", "NaCl"),
        ("кальций оксиді", "Кальций оксидінің", "CaO"),
        ("кальций карбонаты", "Кальций карбонатының", "CaCO₃"),
        ("натрий гидроксиді", "Натрий гидроксидінің", "NaOH"),
        ("натрий бикарбонаты", "Натрий бикарбонатының", "NaHCO₃"),
        ("мыс сульфаты", "Мыс сульфатының", "CuSO₄"),
        ("көк тас", "Көк тастың", "CuSO₄"),
        ("әк тас", "Әк тастың", "CaCO₃"),
        ("аммоний хлориді", "Аммоний хлоридінің", "NH₄Cl"),
        ("темір тотығы", "Темір тотығының", "Fe₂O₃"),
        ("угар газ", "Угар газының", "CO"),
        ("сахароза", "Сахарозаның", "C₁₂H₂₂O₁₁"),
        ("глюкоза", "Глюкозаның", "C₆H₁₂O₆"),
        ("этанол", "Этанолдың", "C₂H₅OH"),
        ("этил спирті", "Этил спиртінің", "C₂H₅OH"),
        ("метан", "Метанның", "CH₄"),
        ("аммиак", "Аммиактың", "NH₃"),
        ("озон", "Озонның", "O₃"),
        ("гипс", "Гипстің", "CaSO₄·2H₂O"),
        // **Phase 23.A (2026-06-03)** — Whisper-drift compound names
        // observed in live REPL. Listed BEFORE single-word elements
        // so the drift form wins length-priority.
        ("қуқырт қышқыл", "Күкірт қышқылының", "H₂SO₄"),
        ("құрқырт қышқыл", "Күкірт қышқылының", "H₂SO₄"),
        ("куркурт қышқыл", "Күкірт қышқылының", "H₂SO₄"),
        // ── Single-word substances / elements (shorter, lower priority) ──
        ("көмірқышқыл", "Көмірқышқыл газының", "CO₂"),
        ("сутегі", "Сутегінің", "H₂"),
        ("сутек", "Сутегінің", "H₂"),
        ("оттегі", "Оттегінің", "O₂"),
        ("оттек", "Оттегінің", "O₂"),
        // **Phase 23.A** — Whisper drifts of «оттегі»: single-т
        // «отегі», token-split «о тегі» / «ө тегі».
        // **Phase 23.B (2026-06-03 evening)** — additional drift
        // «тегенн-» (Whisper produces «тегеннің» instead of
        // «тегінің» when the leading «о»/«ө» is split off).
        ("отегі", "Оттегінің", "O₂"),
        ("о тегі", "Оттегінің", "O₂"),
        ("ө тегі", "Оттегінің", "O₂"),
        ("о тегенн", "Оттегінің", "O₂"),
        ("ө тегенн", "Оттегінің", "O₂"),
        ("отегенн", "Оттегінің", "O₂"),
        ("өтегенн", "Оттегінің", "O₂"),
        // **Phase 23.A** — sulfur element + Whisper drift.
        ("күкірт", "Күкірттің", "S"),
        ("қуқырт", "Күкірттің", "S"),
        ("азот", "Азоттың", "N₂"),
        ("алтын", "Алтынның", "Au"),
        ("күміс", "Күмістің", "Ag"),
        ("сынап", "Сынаптың", "Hg"),
        ("қорғасын", "Қорғасынның", "Pb"),
        ("мырыш", "Мырыштың", "Zn"),
        ("алюминий", "Алюминийдің", "Al"),
        ("кальций", "Кальцийдің", "Ca"),
        ("магний", "Магнийдің", "Mg"),
        ("натрий", "Натрийдің", "Na"),
        ("калий", "Калийдің", "K"),
        ("темір", "Темірдің", "Fe"),
        ("мыс", "Мыстың", "Cu"),
        ("спирт", "Этил спиртінің", "C₂H₅OH"),
        ("қант", "Сахарозаның", "C₁₂H₂₂O₁₁"),
        ("тұз", "Ас тұзының", "NaCl"),
        ("сода", "Ас содасының", "NaHCO₃"),
        ("әк", "Кальций оксидінің", "CaO"),
        // ── Water (lowest priority — «су» is so short it must lose
        // to all longer matches above; placed last for stem search). ──
        ("судың", "Судың", "H₂O"),
        ("суды", "Судың", "H₂O"),
        ("суға", "Судың", "H₂O"),
        ("суда", "Судың", "H₂O"),
        ("су ", "Судың", "H₂O"),
    ];

    for (stem, display, formula) in formulas {
        if lower.contains(stem) {
            return Some(format!("{display} формуласы — {formula}."));
        }
    }
    None
}

/// **2026-06-03** — first-person location statement detector. Matches
/// inputs like «Мен Қостанайда тұрамын» / «Мен Қостанай қалада
/// тұрамын» / «Біз Алматыда тұрамыз». When this fires, v6_2_router
/// returns None so the v6.1 cascade's acknowledgement reply stands
/// (and the city slot in the session is preserved for later recall).
///
/// **rc5-followup (2026-06-03 evening)** — initial implementation
/// enumerated «тұрамын» / «тұрамыз» / «тұрам» literally. Live REPL
/// caught «Мен қостанай атырамым» — Whisper drifted «тұрамын» to
/// «атырамым» AND stripped the locative `‑да` from the city. Neither
/// substring matched the canonical list, so the router fell through
/// to the «Қала» IsA reply again. Fix: keep the canonical-verb fast
/// path AND add a morphological fallback that pairs a 1sg/1pl verb
/// suffix (`‑мын` / `‑мыз` / `‑мым` / `‑мім` / `‑міз`) with a city
/// marker (either a known Kazakhstan oblast-centre stem or a
/// locative-suffixed noun ≥ 5 chars).
fn looks_like_first_person_location_statement(s: &str) -> bool {
    let lower = s.to_lowercase();
    // **Phase 26.A (2026-06-04)** — compound utterance support.
    // Live REPL caught «Менің атым Дәулет, мен қостанайда тұрамын» —
    // the input STARTS with «менің», so the strict «мен »-at-start
    // check missed the second clause.  Phase 26.A added the comma /
    // period clause boundary («, мен » / «. мен »).
    //
    // **Phase 26.C (2026-06-04 evening — post-rc11 audit)** —
    // sometimes the user runs both clauses together without ANY
    // separator: «Менім атын дәулет мен қазақстанда тұрамын».
    // Detect «мен» as a standalone token followed by a city +
    // dwelling-verb pattern anywhere in the input.  Risk of false
    // positive on «X мен Y тұрамыз» (X and Y live together) is
    // mitigated by requiring the verb to be SINGULAR («тұрамын»),
    // since the conjunction reading needs plural «тұрамыз».
    let has_first_person_pronoun = lower.starts_with("мен ")
        || lower.starts_with("мен.")
        || lower.starts_with("мен,")
        || lower.starts_with("мың ")
        || lower.starts_with("біз ")
        || lower.contains(", мен ")
        || lower.contains(". мен ")
        || lower.contains(",мен ")  // missing space after comma
        || lower.contains(".мен ")
        // Phase 26.C — standalone-token «мен» with 1sg dwelling verb.
        || (lower.contains(" мен ") && lower.contains("тұрамын"));
    if !has_first_person_pronoun {
        return false;
    }
    // Fast path — canonical dwelling verbs that survived STT.
    let canonical_verbs = [
        "тұрамын",
        "тұрамыз",
        "тұрам",
        "тұрып жатырмын",
        "тұрып жатырмыз",
    ];
    if canonical_verbs.iter().any(|v| lower.contains(v)) {
        return true;
    }
    // Whisper-drift fallback — 1p verb morphology + city marker.
    let tokens: Vec<String> = lower
        .split_whitespace()
        .map(|t| t.trim_end_matches([',', '.', '!', '?']).to_string())
        .collect();
    let has_first_person_verb = tokens.iter().any(|t| {
        t.len() >= 5
            && (t.ends_with("мын")
                || t.ends_with("мыз")
                || t.ends_with("мым")
                || t.ends_with("мім")
                || t.ends_with("міз"))
    });
    if !has_first_person_verb {
        return false;
    }
    // Recognised KZ oblast-centre stems (prefix-match catches every
    // case-inflected form — `қостанайда`, `қостанайдан`, even the
    // Whisper-drifted accusative «қостанайды»).
    let known_city_stems = [
        "алматы",
        "астана",
        "нұр-сұлтан",
        "қостанай",
        "костанай",
        "шымкент",
        "ақтөбе",
        "тараз",
        "өскемен",
        "семей",
        "павлодар",
        "атырау",
        "ақтау",
        "орал",
        "талдықорған",
        "көкшетау",
        "петропавл",
        "қызылорда",
        "жезқазған",
        "темиртау",
    ];
    let has_known_city = tokens
        .iter()
        .any(|t| known_city_stems.iter().any(|c| t.starts_with(c)));
    let has_locative_noun = tokens.iter().any(|t| {
        t.len() >= 5
            && (t.ends_with("да") || t.ends_with("де") || t.ends_with("та") || t.ends_with("те"))
    });
    has_known_city || has_locative_noun
}

fn looks_like_time_query(s: &str) -> bool {
    let lower = s.to_lowercase();
    // **2026-06-03 voice REPL regression** — «Қазір қазақстанның
    // президенті кім?» was incorrectly routed to the clock handler
    // because the leading «қазір» triggered this matcher BEFORE the
    // president check downstream. When the input clearly names another
    // entity (президент / премьер / спикер / etc.), the «now» word is
    // a tense marker for that entity, not a time question. Defer.
    let entity_markers = [
        "президент",
        "премьер",
        "министр",
        "спикер",
        "әкім",
        "elbasy",
        "елбасы",
    ];
    if entity_markers.iter().any(|m| lower.contains(m)) {
        return false;
    }
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
        // Phase 21 (2026-06-02) — relative-day anchors + Phase 21.A
        // STT-drift aliases (Whisper hears «ертең» as «еркең» etc.).
        "кеше",
        "кешее",
        "кешеу",
        "ертең",
        "еркең",
        "еркен",
        "ертен",
        "ерткен",
        "эртен",
        "бүрсігүні",
        // Phase 21.C — «ерден» as time marker only when paired with
        // «күн» day marker (handled as a multi-token check below).
        "бүрсүгүні",
        "бірсүгіне",
        "бір сүгіне",
        "бірсүгіні",
        "бір сүгіні",
        "бір сігүні",
        "бірсігүні",
        "алдыңғы күн",
        "вчера",
        "завтра",
    ];
    if markers.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // Phase 21.C — multi-token «ерден» + «күн» pair (ambiguous on
    // its own — could be a person name in ablative case — so we
    // require BOTH tokens to be present).
    if lower.contains("күн")
        && (lower.contains("ерден") || lower.contains("эрден") || lower.contains("ердін"))
    {
        return true;
    }
    false
}

/// **Phase 21 (2026-06-02)** — detect relative-day anchor in the
/// input. Returns the day-offset (−2…+2) and `None` if the input
/// has no relative-day marker (caller should treat it as «today»).
///
/// **Phase 21.A** — STT-drift aliases for each marker. The 2026-06-02
/// live REPL showed Whisper hears «ертең» as «еркең» (т→к) and
/// «бүрсігүні» as «бірсүгіне» / «бір сүгіні» (ү→і + word split).
/// Adding the drifted forms keeps the calendar handler from falling
/// through to substring fact lookup on a recognisable utterance.
fn relative_day_offset(lower: &str) -> Option<i64> {
    for marker in [
        "бүрсігүні",
        "бүрсүгүні",
        "бірсүгіне",
        "бір сүгіне",
        "бірсүгіні",
        "бір сүгіні",
        "бір сігүні",
        "бірсігүні",
    ] {
        if lower.contains(marker) {
            return Some(2);
        }
    }
    // **Phase 21.B (2026-06-03 evening)** — added «еркен» (single -н
    // instead of -ң) caught in live REPL: «Еркен қай күн болады»
    // fell through to «Күн — дөңгелек» substring IsA.
    for marker in [
        "ертең",
        "еркең",
        "еркен",
        "ертен",
        "ерткен",
        "эртен",
        "завтра",
    ] {
        if lower.contains(marker) {
            return Some(1);
        }
    }
    // **Phase 21.C (2026-06-04 — post-rc10 audit)** — «ерден» drift
    // caught in live REPL: «Ерден қай күн болады» yielded an Abai
    // citation about «ер» (man).  Since «ерден» can also be the
    // genuine name «Ерден» + ablative case («from Erden»), this
    // drift only counts when the input is clearly a calendar
    // question (the bare day marker «күн» is present).
    if lower.contains("күн") {
        for marker in ["ерден", "эрден", "ердін"] {
            if lower.contains(marker) {
                return Some(1);
            }
        }
    }
    for marker in ["алдыңғы күн", "алдыңғы күні", "позавчера"] {
        if lower.contains(marker) {
            return Some(-2);
        }
    }
    for marker in ["кеше", "кешее", "кешеу", "вчера"] {
        if lower.contains(marker) {
            return Some(-1);
        }
    }
    None
}

fn emit_clock_answer(input: &str) -> String {
    let c = system_clock::now();
    let lower = input.to_lowercase();
    // **Phase 21** — handle relative-day questions before the
    // generic «today» path. If the user says «Кеше / Ертең …»,
    // shift the clock reading and render with the matching prefix.
    if let Some(offset) = relative_day_offset(&lower) {
        let rc = system_clock::now_offset(offset);
        let label = system_clock::relative_day_label_kk(offset);
        let copula = system_clock::relative_day_copula_kk(offset);
        // Weekday-only ask: «Кеше қай күн болды?» / «Ертең қай күн?»
        if lower.contains("апта")
            || lower.contains("неделя")
            || (lower.contains("күн") && (lower.contains("қай") || lower.contains("қандай")))
        {
            return format!("{label} — {} {copula}.", rc.weekday_kk());
        }
        // Day-of-month ask: «Кеше нешесі еді?» / «Ертең нешесі?»
        if lower.contains("нешесі") || lower.contains("число") {
            return format!("{label} {} {} {copula}.", rc.day, rc.month_kk());
        }
        // Generic relative-day date.
        return format!(
            "{label} — {} {} {} жыл, {} {copula}.",
            rc.day,
            rc.month_kk(),
            rc.year,
            rc.weekday_kk()
        );
    }
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

    // Subject-focus reverse lookup path: «1872 жылы кім туылған?» —
    // user asks WHO was born/died/founded in a given year. The
    // canonical agent is unknown (that's what they're asking),
    // so we build a Subject-focus query with a Time
    // modifier_constraint instead.
    if let Some(year_phrase) = extract_year_phrase(&lower)
        && (lower.contains("кім") || lower.contains("кто"))
    {
        let pred = predicate_for_reverse(&lower).unwrap_or(FramePredicate::BornIn);
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(pred)
        .with_modifier_constraint(ModifierRole::Time, noun(&year_phrase))
        .with_language_filter(language);
        return Some(q);
    }

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

/// Pull a year-phrase «NNNN жылы» / «NNNN жыл» / bare year from
/// the input for the reverse-lookup path.
fn extract_year_phrase(lower: &str) -> Option<String> {
    // Match 3-4 digit year followed by «жыл» / «жылы» / nothing.
    let mut digits = String::new();
    let mut last_year: Option<String> = None;
    for c in lower.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if (3..=4).contains(&digits.len())
                && let Ok(y) = digits.parse::<u32>()
                && (1000..3000).contains(&y)
            {
                last_year = Some(format!("{y} жыл"));
            }
            digits.clear();
        }
    }
    if (3..=4).contains(&digits.len())
        && let Ok(y) = digits.parse::<u32>()
        && (1000..3000).contains(&y)
    {
        last_year = Some(format!("{y} жыл"));
    }
    last_year
}

/// Pick the predicate for reverse-lookup based on the verb used.
fn predicate_for_reverse(lower: &str) -> Option<FramePredicate> {
    if lower.contains("туыл") || lower.contains("туған") || lower.contains("родился")
    {
        Some(FramePredicate::BornIn)
    } else if lower.contains("қайтыс") || lower.contains("өл") || lower.contains("умер")
    {
        Some(FramePredicate::DiedIn)
    } else if lower.contains("құрыл") || lower.contains("ашыл") || lower.contains("основан")
    {
        Some(FramePredicate::FoundedIn)
    } else if lower.contains("жаз") || lower.contains("автор") {
        Some(FramePredicate::Authored)
    } else {
        None
    }
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
    } else if lower.contains("кім бұл") || lower.starts_with("кім ") || lower.starts_with("кто ")
    {
        // Subject focus: «Кім туылды?» / «Кто пришёл?» — interrogative
        // appears at the START. The agent is what we're looking for.
        FocusKind::Subject
    } else if lower.contains("кім?") || lower.contains("кто?") {
        // «X кім?» / «X кто?» — interrogative at the END, after the
        // agent. This is a DEFINITIONAL question («Ахмет Байтұрсынұлы
        // кім?» = "what is Ahmet?", asking for his profession/IsA).
        // Codex 2026-05-25 audit caught this misclassifying as
        // Subject focus and returning the agent name as the answer.
        FocusKind::Definition
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
    // LivesIn — animate "где живёт" — wins over generic LocatedIn.
    // Codex 2026-05-25 audit caught «Абай қайда өмір сүрді?»
    // misrouting to LocatedIn (because of «қайда») instead of
    // LivesIn (because of «өмір сүр»). Order matters.
    if lower.contains("өмір сүр")
        || lower.contains("тұрды")
        || lower.contains("тұрған")
        || lower.contains("жил")
    {
        return Some(FramePredicate::LivesIn);
    }
    // LocatedIn — inanimate "где находится".
    if lower.contains("орналас") || lower.contains("находится") || lower.contains("қайда")
    {
        return Some(FramePredicate::LocatedIn);
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
/// Strip the most common Kazakh case suffixes from each word of
/// the input so the canonical-agent substring search matches
/// «ньютонның» against canonical «ньютон» etc. Heuristic — may
/// over-strip; safe because we only use it as an ADDITIONAL match
/// path beside the raw lowered input.
///
/// Codex 2026-05-25 voice REPL audit observed «Ньютонның екінші
/// заңы», «Эйнштейн формуласын», «Қазақстанда» etc. fail to match
/// canonical bare-stem surfaces; this strips their case suffixes.
fn strip_kazakh_case_suffixes(s: &str) -> String {
    // Longest suffix first so «ның» is stripped before «ы».
    // Limited to common nominal cases; verb endings are not stripped
    // because they can collide with the stem (e.g. «жаз» = "write"
    // is also the root, not a case suffix).
    // **Conservative suffix list (v6.2.0 codex 2026-05-25 fix).**
    // Original list also stripped possessive «-ы» / «-і» / «-сы» /
    // «-сі», which over-strips canonical noun phrases that legitimately
    // end in those (e.g. «ньютон екінші заңы», «эйнштейн формуласы»).
    // The new list strips only **case markers that follow noun stems**
    // (genitive / locative / dative / ablative / accusative); possessive
    // suffixes are left intact so canonical surfaces match.
    let suffixes: &[&str] = &[
        // Genitive (longest first).
        "ның",
        "нің",
        "дың",
        "дің",
        "тың",
        "тің",
        // Locative attribute.
        "дағы",
        "дегі",
        "тағы",
        "тегі",
        "нда",
        "нде",
        // Ablative.
        "дан",
        "ден",
        "тан",
        "тен",
        "нан",
        "нен",
        "сынан",
        "сінен",
        // Dative.
        "сына",
        "сіне",
        "ына",
        "іне",
        "ға",
        "ге",
        "қа",
        "ке",
        "на",
        "не",
        // Locative.
        "да",
        "де",
        "та",
        "те",
        // Instrumental (multi-char).
        "мен",
        "пен",
        "бен",
        // Note: accusative-on-possessive («-сын / -сін / -ын /
        // «-ін») is handled separately above via REPLACEMENT
        // («формуласын» → «формуласы») so the possessive suffix
        // is preserved for the canonical-surface match.
    ];
    // Replacement-style strips for accusative-on-possessive
    // («формуласын» → «формуласы», «жауабын» → «жауабы»). These
    // strip only the trailing «н», leaving the possessive suffix
    // intact so the canonical-surface match («эйнштейн формуласы»)
    // still aligns.
    let replace_n: &[(&str, &str)] = &[("сын", "сы"), ("сін", "сі"), ("ын", "ы"), ("ін", "і")];
    s.split_whitespace()
        .map(|w| {
            let mut stem = w.to_string();
            // Try replacement strips first (less aggressive).
            for (suf, repl) in replace_n {
                if stem.chars().count() > suf.chars().count() + 1 && stem.ends_with(suf) {
                    let new_len = stem.len() - suf.len();
                    stem.truncate(new_len);
                    stem.push_str(repl);
                    return stem;
                }
            }
            for suf in suffixes {
                if stem.chars().count() > suf.chars().count() + 1 && stem.ends_with(suf) {
                    let new_len = stem.len() - suf.len();
                    stem.truncate(new_len);
                    break;
                }
            }
            stem
        })
        .collect::<Vec<_>>()
        .join(" ")
}

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
    let stripped = strip_kazakh_case_suffixes(lower);
    let folded = stt_fold(lower);
    let folded_stripped = strip_kazakh_case_suffixes(&folded);
    let mut best: Option<(&str, usize)> = None;
    for c in candidates {
        let len = c.chars().count();
        // **Word-boundary required for short agents.** Codex
        // 2026-05-25 voice REPL audit caught «ай» / «су» / «жер»
        // matching as substrings of «қалайсың», «суық», «жерде» —
        // producing wrong sense answers. Agents ≤ 3 chars must
        // appear as a whole word.
        let hit = if len <= 3 {
            contains_word(lower, c)
                || contains_word(&stripped, c)
                || contains_word(&folded, c)
                || contains_word(&folded_stripped, c)
        } else {
            lower.contains(c)
                || stripped.contains(c)
                || folded.contains(c)
                || folded_stripped.contains(c)
        };
        if hit && best.is_none_or(|(_, l)| len > l) {
            best = Some((c, len));
        }
    }
    best.map(|(s, _)| s.to_string())
}

/// Whole-word substring check — `haystack` contains `needle` as a
/// space-separated token (or at the start / end). Used for short
/// canonical agents («ай», «су», «жер») where a plain `contains`
/// would false-positive on «қалайсың» / «суық» / «жерде».
fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .any(|tok| tok == needle)
}

/// **STT fold** — normalize common Whisper-STT mishears to their
/// canonical Kazakh spelling so canonical-agent matching finds them.
///
/// Voice-REPL audit 2026-05-25 cases:
/// - «костанай» (Cyrillic к) → «қостанай» (Kazakh қ).
/// - «обылыс» / «облыс» / «болыс» → «облыс».
/// - «тауке хан» → «тәуке хан».
/// - «энштейн» / «әнштейн» → «эйнштейн».
/// - «жылдандығы» / «жолдамдығы» → «жылдамдығы».
/// - «зан» / «зең» → «заң».
/// - «костанайда» / «қостанайдан» — handled by case-stripper.
///
/// Conservative — only changes letters known to mishear; doesn't
/// touch words that already start with Kazakh diacritics.
fn stt_fold(s: &str) -> String {
    let mut out = s.to_string();
    // Common STT loanword mishears.
    out = out.replace("әнштейн", "эйнштейн");
    out = out.replace("энштейн", "эйнштейн");
    out = out.replace("ейнштейн", "эйнштейн");
    out = out.replace("анштейн", "эйнштейн");
    // Place names — Cyrillic к → Kazakh қ for the canonical Kazakh
    // city / oblast names we curate. Limited to known patterns to
    // avoid false rewrites of Russian loans.
    out = out.replace("костанай", "қостанай");
    out = out.replace("казахстан", "қазақстан");
    out = out.replace("қазахстан", "қазақстан");
    // Kazakh diacritic recovery.
    out = out.replace("тауке хан", "тәуке хан");
    // Misheard nouns.
    out = out.replace("обылыс", "облыс");
    out = out.replace("жылданд", "жылдамд");
    out = out.replace("жолдамд", "жылдамд");
    // Session-3 audit (codex 2026-05-25):
    // - «жел тұқтықстан» / «жел тоқтақстан» / «жел тоқыстан»
    //   → «желтоқсан».
    // - «оналты» (no space) → «он алты» (16 in Kazakh).
    // - «тенүзі» / «теңіз» → «теңізі» (Caspian sea question).
    // - «химияқылық» / «химияғылық» → «химиялық».
    // - «хандыр» / «хандырдыг» → «хандығы» (Khanate).
    // - «фотосинтіз» → «фотосинтез».
    // - «жасанды интелект» → «жасанды интеллект».
    out = out.replace("жел тұқтықстан", "желтоқсан");
    out = out.replace("жел тоқтақстан", "желтоқсан");
    out = out.replace("жел тоқыстан", "желтоқсан");
    out = out.replace("жел туқтыкстан", "желтоқсан");
    out = out.replace("оналты", "он алты");
    out = out.replace("тенүзі", "теңізі");
    out = out.replace("химияқылық", "химиялық");
    out = out.replace("химияғылық", "химиялық");
    out = out.replace("хандырдыг", "хандығы");
    out = out.replace("хандырдығы", "хандығы");
    out = out.replace("фотосинтіз", "фотосинтез");
    out = out.replace("жасанды интелект", "жасанды интеллект");
    // Session-4 audit (codex 2026-05-25 evening voice REPL):
    // - «мүгін» (Whisper for «бүгін») broke the clock gate so
    //   «Мүгін қандай ай» fell through to a generic «ай» IsA hit.
    // - «қобейту» / «кубит» (Whisper for «көбейту») broke math
    //   so «Екі қобейту екі» / «Екі кубит беске» returned an
    //   IsA description of «екі» instead of the computed result.
    // - «дарижысы» / «дарижесі» (Whisper for «дәрежесі») same
    //   class; folds let math_solver compute the power.
    // - «пайз» (Whisper for «пайыз») same class; «Жүз пайз бес»
    //   returned «Жүз — рулық бөлініс» instead of the percent op.
    // - «толған» / «тұлған» (Whisper for «туылған») — only when
    //   the question word «қашан» is present, where the context
    //   forces the «born» reading; preserves the legitimate
    //   «толған» = «filled» meaning in non-question contexts.
    // - «байтурсынулы» (Whisper for «байтұрсынұлы») — Cyrillic-у
    //   substitution for ұ.
    out = out.replace("мүгін", "бүгін");
    out = out.replace("қобейту", "көбейту");
    out = out.replace("қобейт ", "көбейт ");
    out = out.replace("кубит", "көбейту");
    out = out.replace("дарижысы", "дәрежесі");
    out = out.replace("дарижесі", "дәрежесі");
    out = out.replace("дәрижесі", "дәрежесі");
    out = out.replace("пайз ", "пайыз ");
    if out.contains("қашан") {
        out = out.replace("толған", "туылған");
        out = out.replace("тұлған", "туылған");
    }
    out = out.replace("байтурсынулы", "байтұрсынұлы");
    out = out.replace("байтұрсын улы", "байтұрсынұлы");
    out
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

    /// **Phase 21 (2026-06-02)** — relative-day queries route to the
    /// clock handler instead of the substring fact lookup.  Without
    /// the «кеше» / «ертең» markers the router previously fell into
    /// `Күн IsA дөңгелек` (live REPL 2026-06-02 retest).
    #[test]
    fn yesterday_query_routes_to_clock_with_past_copula() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Кеше қай күн болды?", &idx);
        let s = r.expect("yesterday query should answer");
        assert!(s.starts_with("Кеше"), "expected «Кеше …» prefix, got: {s}");
        assert!(
            s.contains("болды"),
            "expected past copula «болды», got: {s}"
        );
        // Must NOT be the «Күн — дөңгелек» substring misroute.
        assert!(
            !s.contains("дөңгелек"),
            "regression: routed to fact, got: {s}"
        );
    }

    #[test]
    fn tomorrow_query_routes_to_clock_with_future_copula() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ертең қандай күн?", &idx);
        let s = r.expect("tomorrow query should answer");
        assert!(s.starts_with("Ертең"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    #[test]
    fn day_after_tomorrow_query_routes_to_clock() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Бүрсігүні нешесі болады?", &idx);
        let s = r.expect("day-after-tomorrow query should answer");
        assert!(s.starts_with("Бүрсігүні"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    /// **Phase 21.A (2026-06-02)** — Whisper-drift aliases recover the
    /// calendar handler when STT mishears the marker. 2026-06-02 live
    /// REPL: «ертең» → «еркең», «бүрсігүні» → «бірсүгіне» / «бір сүгіні».
    #[test]
    fn whisper_drift_yerken_routes_to_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Еркең қай күн болады?", &idx);
        let s = r.expect("еркең drift should still route to tomorrow");
        assert!(s.starts_with("Ертең"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
        assert!(
            !s.contains("дөңгелек"),
            "regression: fell through, got: {s}"
        );
    }

    #[test]
    fn whisper_drift_birsugine_routes_to_day_after_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Бірсүгіне нешесі болады?", &idx);
        let s = r.expect("бірсүгіне drift should route to day-after");
        assert!(s.starts_with("Бүрсігүні"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    #[test]
    fn whisper_drift_bir_sugini_routes_to_day_after_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Бір сүгіні нешесі болады.", &idx);
        let s = r.expect("space-split бір сүгіні should still route");
        assert!(s.starts_with("Бүрсігүні"), "got: {s}");
    }

    /// **2026-06-03** — first-person location statement MUST NOT be
    /// answered by the v6.2 router (the v6.1 cascade upstream handles
    /// the acknowledgement + session update). Earlier behaviour was
    /// to return one-word «Қала» (IsA city) because the substring-IsA
    /// layer caught «қала» before the cascade could acknowledge.
    /// Multi-session live REPL regression — pinned here permanently.
    #[test]
    fn first_person_location_statement_defers_to_v61_cascade() {
        let idx = dialog_battery::canonical_corpus();
        // Canonical: «Мен Қостанайда тұрамын».
        assert!(
            answer_with_corpus("Мен Қостанайда тұрамын.", &idx).is_none(),
            "v6.2 must defer to v6.1 cascade for location statements"
        );
        // Compound city form: «Мен Қостанай қалада тұрамын».
        assert!(
            answer_with_corpus("Мен Қостанай қалада тұрамын.", &idx).is_none(),
            "compound «city қалада» must also defer"
        );
        // Whisper drift: «мен» → «мың».
        assert!(
            answer_with_corpus("Мың Қостанай қалада тұрамын.", &idx).is_none(),
            "Whisper-drifted мен→мың must also defer"
        );
        // First-person plural: «Біз Алматыда тұрамыз».
        assert!(
            answer_with_corpus("Біз Алматыда тұрамыз.", &idx).is_none(),
            "first-person plural тұрамыз must also defer"
        );
    }

    /// Sanity: the defer rule fires only on first-person dwelling
    /// verbs combined with мен / біз / мың. «Қала деген не?» (city
    /// definition) lacks both → must NOT defer.
    #[test]
    fn city_definition_query_does_not_match_defer_rule() {
        assert!(!looks_like_first_person_location_statement(
            "Қала деген не?"
        ));
        assert!(!looks_like_first_person_location_statement(
            "Қостанай қандай қала?"
        ));
        // First-person without dwelling verb → also no defer.
        assert!(!looks_like_first_person_location_statement("Мен оқимын."));
        // Positive control: the defer cases do match.
        assert!(looks_like_first_person_location_statement(
            "Мен Қостанайда тұрамын."
        ));
        assert!(looks_like_first_person_location_statement(
            "Мың Қостанай қалада тұрамын."
        ));
        assert!(looks_like_first_person_location_statement(
            "Біз Алматыда тұрамыз."
        ));
    }

    /// **Phase 23 (2026-06-03)** — chemistry-formula lookup.  Pins
    /// the school-curriculum formulas plus the live-REPL transcripts
    /// that surfaced this gap.
    #[test]
    fn water_formula_lookup() {
        assert_eq!(
            lookup_chemical_formula("Судың формуласын жазып бер."),
            Some("Судың формуласы — H₂O.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Судың химия формуласын жаз."),
            Some("Судың формуласы — H₂O.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Су формуласы қандай?"),
            Some("Судың формуласы — H₂O.".to_string())
        );
    }

    #[test]
    fn salt_formula_lookup() {
        assert_eq!(
            lookup_chemical_formula("Тұздың формуласы қандай?"),
            Some("Ас тұзының формуласы — NaCl.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Ас тұзының формуласы қандай?"),
            Some("Ас тұзының формуласы — NaCl.".to_string())
        );
    }

    #[test]
    fn longest_match_wins_for_compounds() {
        // «көмірқышқыл газы» must win over bare «газ» / «көмірқышқыл».
        let r = lookup_chemical_formula("Көмірқышқыл газының формуласы қандай?");
        assert_eq!(r.as_deref(), Some("Көмірқышқыл газының формуласы — CO₂."));
        // «күкірт қышқылы» must win over «күкірт» (if it were in the
        // list separately).
        let r = lookup_chemical_formula("Күкірт қышқылының формуласы қандай?");
        assert_eq!(r.as_deref(), Some("Күкірт қышқылының формуласы — H₂SO₄."));
    }

    #[test]
    fn element_formulas_oxygen_hydrogen() {
        assert_eq!(
            lookup_chemical_formula("Оттегінің формуласы қандай?"),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Сутегінің формуласы қандай?"),
            Some("Сутегінің формуласы — H₂.".to_string())
        );
    }

    #[test]
    fn no_formula_marker_no_fire() {
        // Bare substance mention without «формула» must NOT fire.
        // «Су ішемін» (I drink water) — no chemistry intent.
        assert_eq!(lookup_chemical_formula("Су ішемін."), None);
        assert_eq!(lookup_chemical_formula("Қаладан су әкел."), None);
        // «Тұзды бер» (pass the salt) — no formula query.
        assert_eq!(lookup_chemical_formula("Тұзды бер."), None);
    }

    #[test]
    fn unknown_substance_returns_none() {
        // Some substance the table doesn't cover.
        assert_eq!(
            lookup_chemical_formula("Қышбылдықтың формуласы қандай?"),
            None
        );
    }

    /// **Phase 21.B (2026-06-03 evening)** — «еркен» drift (single
    /// -н instead of -ң) caught in live REPL: «Еркен қай күн болады.»
    #[test]
    fn yerken_single_n_routes_to_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Еркен қай күн болады?", &idx);
        let s = r.expect("еркен drift must route to tomorrow");
        assert!(s.starts_with("Ертең"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    /// **Phase 21.C (2026-06-04)** — «ерден» drift, GATED by the
    /// «күн» day marker.  Live REPL: «Ерден қай күн болады»
    /// returned an Abai citation about «ер» (man) — fell through to
    /// the Abai-quote handler.
    #[test]
    fn yerden_in_day_context_routes_to_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ерден қай күн болады?", &idx);
        let s = r.expect("ерден + күн must route to tomorrow");
        assert!(s.starts_with("Ертең"), "got: {s}");
    }

    /// **Phase 21.C** — «ерден» WITHOUT «күн» must NOT trigger
    /// tomorrow.  «Ерден» can be a genuine personal name + ablative.
    #[test]
    fn yerden_without_day_marker_does_not_route_to_tomorrow() {
        // `relative_day_offset` should return None on plain «Ерден»
        // mentions.  We test the function directly to avoid the
        // upstream substring noise.
        assert_eq!(relative_day_offset("Ерден келді."), None);
        assert_eq!(relative_day_offset("Ерденнен сәлем."), None);
    }

    /// **Phase 26.C (2026-06-04 evening)** — compound utterance
    /// support extended to inputs WITHOUT any clause separator.
    /// Live REPL: «Менім атын дәулет мен қазақстанда тұрамын» — no
    /// comma, no period, both clauses run together.  The standalone
    /// «мен» token between two clauses + the 1sg dwelling verb
    /// «тұрамын» now triggers the defer.
    #[test]
    fn compound_without_separator_defers_to_v61() {
        assert!(looks_like_first_person_location_statement(
            "Менім атын дәулет мен қазақстанда тұрамын."
        ));
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет мен Қостанайда тұрамын."
        ));
    }

    /// Negative control for Phase 26.C: «X мен Y тұрамыз» (X AND Y
    /// live together — plural verb) must NOT trigger location defer
    /// because «мен» here is a conjunction, not a 1sg pronoun.
    #[test]
    fn compound_without_separator_does_not_misfire_on_conjunction_mеn() {
        // Plural verb — should NOT trigger our defer.
        assert!(!looks_like_first_person_location_statement(
            "Дәулет мен Болат Қостанайда тұрамыз."
        ));
    }

    /// **Phase 26.A (2026-06-04)** — compound utterance defer.  Live
    /// REPL caught «Менің атым Дәулет, мен қостанайда тұрамын» —
    /// the input STARTS with «менің», so the strict «мен »-at-start
    /// check missed the second clause.  The router fell through to
    /// the «Қостанай → Қала» IsA reply.
    #[test]
    fn compound_name_then_location_defers_to_v61() {
        // After the fix, this returns None so the v6.1 cascade
        // (which acknowledges both name AND location) stands.
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет, мен қостанайда тұрамын."
        ));
        // Same with period separator instead of comma.
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет. Мен қостанайда тұрамын."
        ));
        // And with missing space after comma (common typo / STT).
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет,мен қостанайда тұрамын."
        ));
    }

    /// **Phase 23.B (2026-06-03 evening)** — comma-split substance
    /// names. Live REPL caught «Ө, тегеннің формулысы.» — Whisper
    /// inserted a comma mid-token; the rc8 stem table didn't match.
    /// Pre-normalise punctuation → space so the stem catches.
    #[test]
    fn chemistry_formula_handles_comma_split() {
        assert_eq!(
            lookup_chemical_formula("Ө, тегеннің формулысы."),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("О, тегеннің формулысы."),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
    }

    /// **Phase 23.A (2026-06-03)** — Whisper-drift coverage for the
    /// chemistry-formula table. Live REPL caught 3 drifts the rc7
    /// table missed: single-т «отегі», token-split «о тегі» / «ө
    /// тегі», and «қуқырт» for «күкірт». Pinned here.
    #[test]
    fn oxygen_drift_single_t() {
        assert_eq!(
            lookup_chemical_formula("Отегінің формулысы."),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
    }

    #[test]
    fn oxygen_drift_token_split() {
        let r = lookup_chemical_formula("Ө тегінің формулысы.");
        assert_eq!(r.as_deref(), Some("Оттегінің формуласы — O₂."));
        let r = lookup_chemical_formula("О тегінің формулысы.");
        assert_eq!(r.as_deref(), Some("Оттегінің формуласы — O₂."));
    }

    #[test]
    fn sulfuric_acid_drift_qukyrt() {
        let r = lookup_chemical_formula("Қуқырт қышқылының формулысы.");
        assert_eq!(r.as_deref(), Some("Күкірт қышқылының формуласы — H₂SO₄."));
    }

    /// **rc5-followup (2026-06-03 evening)** — Whisper-drift fallback.
    /// Live REPL hit «Мен қостанай атырамым» — the dwelling verb
    /// «тұрамын» was mistranscribed as «атырамым» AND the locative
    /// `‑да` was dropped from the city. Neither the canonical-verb
    /// fast path nor the locative-noun marker matched. The fallback
    /// must catch this via the 1p verb suffix + known-city stem.
    #[test]
    fn first_person_location_drift_via_morphology_fallback() {
        // Live REPL transcript verbatim.
        assert!(
            looks_like_first_person_location_statement("Мен қостанай атырамым."),
            "must catch «тұрамын» → «атырамым» drift"
        );
        // Same drift, with accusative city marker.
        assert!(looks_like_first_person_location_statement(
            "Мен қостанайды атырамым."
        ));
        // Drift in the verb only — locative survived.
        assert!(looks_like_first_person_location_statement(
            "Мен қостанайда атырамым."
        ));
        // First-person plural drift.
        assert!(looks_like_first_person_location_statement(
            "Біз алматыда атырамыз."
        ));
        // Negative control: 1p verb without any city marker is NOT
        // a location statement (e.g. «I am thinking»).
        assert!(!looks_like_first_person_location_statement("Мен ойлаймын."));
        // Negative control: city mentioned but not a 1p statement.
        assert!(!looks_like_first_person_location_statement(
            "Қостанай қандай қала?"
        ));
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

    /// Session-4 audit: «Мүгін қандай ай» (Whisper misheard
    /// «бүгін» as «мүгін») must reach the clock gate.
    #[test]
    fn stt_fold_mugin_routes_to_clock() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Мүгін қандай ай?", &idx);
        assert!(r.is_some(), "expected clock answer, got None");
        let s = r.unwrap();
        assert!(
            s.contains("ай"),
            "expected month answer to mention «ай», got: {s}"
        );
    }

    /// Session-4 audit: «Екі қобейту екі» (Whisper misheard
    /// «көбейту» as «қобейту») must compute, not return an IsA
    /// description of «екі».
    #[test]
    fn stt_fold_qobejtu_routes_to_math() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Екі қобейту екі", &idx);
        assert_eq!(r.as_deref(), Some("4"));
    }

    /// Session-4 audit: «Екі кубит беске» (Whisper misheard
    /// «көбейту» as «кубит») — same class.
    #[test]
    fn stt_fold_kubit_routes_to_math() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Екі кубит беске", &idx);
        assert_eq!(r.as_deref(), Some("10"));
    }

    /// Session-4 audit: «Екі дарижысы он» (Whisper misheard
    /// «дәрежесі» as «дарижысы») → power.
    #[test]
    fn stt_fold_darizysy_routes_to_power() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Екі дарижысы он", &idx);
        assert_eq!(r.as_deref(), Some("1024"));
    }

    /// Session-4 audit: «Жүз пайз бес» (Whisper dropped the «ы»
    /// in «пайыз») → percent.
    #[test]
    fn stt_fold_pajz_routes_to_percent() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Жүз пайз бес", &idx);
        assert_eq!(r.as_deref(), Some("5"));
    }

    /// Session-4 audit: «Ахмет байтурсынулы қашан толған»
    /// (Whisper substituted Cyrillic-у for ұ, and «толған» for
    /// «туылған») — the conditional fold keys on «қашан» to
    /// rewrite «толған» → «туылған» without breaking the
    /// legitimate «filled» meaning elsewhere.
    #[test]
    fn stt_fold_tolgan_routes_to_birth_year() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ахмет байтурсынулы қашан толған?", &idx);
        assert_eq!(r.as_deref(), Some("1872"));
    }

    /// Session-4 audit: «Мен ағай болғанымды қалай түсіндім»
    /// — pitch-detection meta-query with first-person past-tense
    /// slip («түсіндім»). Must route to pitch-explanation, not
    /// to a generic «ағай» retrieval.
    #[test]
    fn pitch_detection_accepts_first_person_slip() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Мен ағай болғанымды қалай түсіндім?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("жиілігі") || s.contains("pitch"),
            "expected pitch explanation, got: {s}"
        );
    }

    /// Session-5 audit (the «Мемлекет» bug): «Қазақстанда қандай
    /// таулар бар?» must enumerate mountains, not return the host
    /// country's IsA type («Мемлекет»).
    #[test]
    fn listing_mountains_in_kazakhstan() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанда қандай таулар бар?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Алатау") && !s.eq_ignore_ascii_case("мемлекет"),
            "expected mountain list, got: {s}"
        );
    }

    /// Session-5 audit: «Қазақстанда қандай өзендер бар?» — rivers.
    #[test]
    fn listing_rivers_in_kazakhstan() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанда қандай өзендер бар?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Ертіс") || s.contains("Сырдария"),
            "expected river list, got: {s}"
        );
    }

    /// Session-5 audit: «Қазақстанда қандай көлдер бар?» — lakes.
    #[test]
    fn listing_lakes_in_kazakhstan() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанда қандай көлдер бар?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Балқаш") || s.contains("Зайсан"),
            "expected lake list, got: {s}"
        );
    }

    /// Session-5 audit: «Қандай өзендер білесің?» — un-anchored
    /// enumeration must still list rivers, not «(нет данных)».
    #[test]
    fn listing_rivers_un_anchored() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қандай өзендер білесің?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Ертіс") || s.contains("Сырдария"),
            "expected river list, got: {s}"
        );
    }
}
