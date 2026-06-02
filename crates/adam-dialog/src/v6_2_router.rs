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
    if looks_like_math(input)
        && let Some(r) = math_solver::solve(input)
    {
        return Some(r.render());
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
        // **Phase 20** — 3 paraphrases of the self-introduction.
        let variants: &[&str] = &[
            "Мен — адам, толық атауым Agglutinative Reasoning Kernel \
             (ARK). Қазақ тіліне арналған детерминирленген тілдік \
             модель. Тілдік модельмін, бірақ LLM емес — мен \
             curated фактілер арқылы жауап беремін.",
            "Менің атым — адам. Толық атауым Agglutinative Reasoning \
             Kernel (ARK), қазақ тіліне арналған детерминирленген \
             тілдік модель. LLM емеспін: жауаптарым тек curated \
             деректерге сүйенеді.",
            "Мен — ARK (Agglutinative Reasoning Kernel) атты тілдік \
             модельмін, қысқаша «адам» деп аталамын. Қазақ тілінің \
             морфологиясы бойынша құрастырылған детерминирленген \
             жүйемін, әр сөзімді curated фактілермен растаймын.",
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
             Менің білім қорым curated фактілерден тұрады, тікелей \
             интернет немесе live-feed қосылған емес.",
            "Бұл сұраққа дерек бере алмаймын — менің білім қорымда \
             ағымдағы / реалды-уақыттық мәлімет жоқ. Тек curated \
             фактілермен жұмыс істеймін.",
            "Кешіріңіз, бұл сұраққа жауап беретін live-дерек менде \
             жоқ. Интернетке немесе сыртқы feed-ке қосылмаймын — \
             curated тарихи фактілер ғана қолымда.",
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

fn looks_like_math(s: &str) -> bool {
    let lower = s.to_lowercase();
    let markers = [
        // Russian.
        "плюс",
        "минус",
        "умнож",
        "раздели",
        "помнож",
        "подели",
        "степени",
        "корень",
        "процент",
        "остаток",
        // Kazakh (core verbs).
        "көбейт",
        "бөл",
        "қос",
        "азайт",
        "дәреже",
        "түбірі",
        "пайыз",
        "қалдық",
        // Voice-REPL STT variants (codex 2026-05-25 audit):
        // «жұп» / «зұп» (heard for «қос»),
        // «кубейт» / «кобейт» / «көбойт» (heard for «көбейт»).
        "жұп",
        "зұп",
        "кубейт",
        "кобейт",
        "көбойт",
        // English / functional.
        "sin",
        "cos",
        "tan",
        "log",
        "ln",
        "abs",
        "mod",
    ];
    if markers.iter().any(|m| lower.contains(m)) {
        return true;
    }
    s.chars()
        .any(|c| matches!(c, '+' | '*' | '/' | '%' | '^' | '√' | '×' | '÷'))
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
    None
}

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
                "Түсіндім, сіз {canonical}сыз. Бағдарламалау тілдері, \
                 алгоритмдер, Rust туралы сұрағыңыз болса — көмектесуге \
                 тырысамын."
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
    let variants: &[&str] = &[
        "Менің білім қорым curated деректерден тұрады. Жауап бере аламын: \
         (1) Қазақстан туралы — география, тарих, әдебиет, танымал тұлғалар, \
         мемлекеттік құрылым; (2) мектеп пәндері — математика, физика, химия, \
         биология, тарих, ана тілі; (3) бағдарламалау тілдері және Rust; \
         (4) дата / уақыт / апта күні (live clock); (5) қарапайым және күрделі \
         математикалық есептеулер (қазақша / орысша / ASCII). LLM емеспін, \
         curated деректерден тыс сұрақтарға «нақты дерек жоқ» деп шынайы \
         жауап беремін.",
        "Мен бірнеше тақырыпта көмектесе аламын: Қазақстанның географиясы, \
         тарихы, әдебиеті мен танымал тұлғалары; мектеп пәндері — \
         математика, физика, химия, биология, ана тілі; бағдарламалау \
         (әсіресе Rust); ағымдағы күн, уақыт пен апта; қарапайым және \
         көп қадамды математикалық есептеулер. Тыс тақырыпта «дерек жоқ» \
         деп шынайы айтамын — LLM емеспін.",
        "Қолымдағы білім аясы — curated facts. Жауап бере алатын тақырыптарым: \
         Қазақстан туралы (география / тарих / әдебиет / тұлғалар / мемлекет); \
         мектеп пәндері (физика, химия, биология, математика, тарих); \
         бағдарламалау тілдері мен Rust; live уақыт-күн-апта; математикалық \
         амалдар. Тыс сұрақтарға қалай ойдан жауап жасайтын LLM емеспін — \
         «білмеймін» дегенді жасырмаймын.",
        "Жауап бере алатын негізгі салаларым: Қазақстан жайында жалпы дерек \
         (география, тарих, әдебиет, белгілі тұлғалар, мемлекеттік құрылым); \
         мектеп бағдарламасы (математика, физика, химия, биология, тарих, \
         ана тілі); бағдарламалау, әсіресе Rust; live дата / уақыт / апта \
         күні; қазақша / орысша / ASCII форматтағы математикалық есептер. \
         LLM емеспін — curated деректер шегінен шықпаймын.",
        "Менің көмектесе алатын тақырыптарым: (1) Қазақстан туралы — \
         география, тарих, әдебиет, белгілі адамдар, мемлекет құрылымы; \
         (2) мектеп пәндері — математика, физика, химия, биология; \
         (3) Rust пен бағдарламалау тілдері; (4) уақыт, күн, апта (live); \
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
