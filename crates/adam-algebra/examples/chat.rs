// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Live REPL demo — adam ARK in interactive mode.
//!
//! Reads a question per line from stdin, routes it through the
//! v6.2 deterministic pipeline (math_solver / system_clock /
//! FrameIndex), prints the answer + latency to stdout.
//!
//! Run: `cargo run --release --example chat -p adam-algebra`
//!
//! ## Routing
//!
//! ```text
//! input
//!   ├─ contains arithmetic markers?  → math_solver::solve
//!   ├─ contains «бүгін / қазір» time-of-day markers? → system_clock::now
//!   └─ otherwise → FrameIndex.best_match (Stage 4 retrieval)
//! ```
//!
//! This is **not** the v6.2 Stage 7 realiser — that one composes
//! natural surface forms from the typed `Frame` answer. The chat
//! demo emits the bare `root.surface` of the answer slot, which
//! is enough for the МО РК pitch to demonstrate determinism +
//! latency on real Kazakh questions.

use std::io::{self, BufRead, Write};
use std::time::Instant;

use adam_algebra::dialog_battery::canonical_corpus;
use adam_algebra::{
    AnswerShape, AnswerSlot, Composition, FramePredicate, Language, ModifierRole, PartOfSpeech,
    QueryFocus, QueryIR, QuestionForm, Root, math_solver, system_clock,
};

fn main() {
    let idx = canonical_corpus();
    println!("=== adam ARK — live REPL ===");
    println!("(deterministic; CPU-only; 0 MB model; no LLM)");
    println!(
        "Курс: {} curated facts. Print a question on each line; type «exit» to quit.",
        idx.len()
    );
    println!();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    loop {
        print!("? ");
        out.flush().ok();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }
        let q = line.trim();
        if q.is_empty() {
            continue;
        }
        if q == "exit" || q == "quit" || q == "выход" || q == "шығу" {
            println!("Сау болыңыз!");
            break;
        }

        let start = Instant::now();
        let answer = answer_one(q, &idx);
        let elapsed = start.elapsed();
        match answer {
            Some(text) => {
                println!("> {}", text);
                println!(
                    "  ({} ns / {:.3} µs)",
                    elapsed.as_nanos(),
                    elapsed.as_micros() as f64
                );
            }
            None => {
                println!("> (нет данных в curated corpus)");
                println!("  ({} ns)", elapsed.as_nanos());
            }
        }
        println!();
    }
}

/// Route one question through the deterministic pipeline. Returns
/// the answer's surface form when found.
fn answer_one(input: &str, idx: &adam_algebra::FrameIndex) -> Option<String> {
    // 1. Math first — if it looks like an arithmetic expression,
    //    let the solver answer.
    if looks_like_math(input)
        && let Some(r) = math_solver::solve(input)
    {
        return Some(r.render());
    }

    // 2. System clock — if it asks about today / now.
    if looks_like_time_query(input) {
        let c = system_clock::now();
        let lower = input.to_lowercase();
        if lower.contains("сағат") || lower.contains("часов") || lower.contains("уақыт")
        {
            return Some(c.time_hhmm());
        }
        if lower.contains("апта")
            || lower.contains("неделя")
            || lower.contains("күн") && lower.contains("қай")
        {
            return Some(c.weekday_kk().to_string());
        }
        if lower.contains("ай")
            && (lower.contains("қандай") || lower.contains("какой") || lower.contains("какая"))
        {
            return Some(c.month_kk().to_string());
        }
        if lower.contains("нешесі") || lower.contains("число") {
            return Some(format!("{}", c.day));
        }
        // Default: full ISO date.
        return Some(c.date_iso());
    }

    // 3. Retrieval — heuristic QueryIR construction. Detect the
    //    canonical agent + predicate by checking known surface
    //    patterns. This is the Stage 7 realiser's job in full;
    //    Stage 4.8 ships enough heuristics to demo on the bench.
    let q = build_query_heuristic(input)?;
    let hit = idx.best_match(&q)?;
    answer_surface(hit, &q)
}

/// Lightweight check: does the input look like math?
fn looks_like_math(s: &str) -> bool {
    let lower = s.to_lowercase();
    let math_markers = [
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
    if math_markers.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // ASCII operator characters.
    s.chars()
        .any(|c| matches!(c, '+' | '*' | '/' | '%' | '^' | '√' | '×' | '÷'))
}

/// Is the user asking about today's date / time / weekday?
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

/// Build a QueryIR by matching well-known agent surfaces from the
/// canonical corpus. Returns None when no canonical agent fires.
fn build_query_heuristic(input: &str) -> Option<QueryIR> {
    let lower = input.to_lowercase();
    let language = if has_cyrillic_russian_marker(&lower) {
        Some(Language::Russian)
    } else {
        Some(Language::Kazakh)
    };

    // Try to find a known canonical agent surface in the input.
    let agent = canonical_agent_for(&lower)?;
    let focus_role = detect_question_focus(&lower);
    let predicate = predicate_for(&lower, &agent);

    let (focus, answer_shape) = match focus_role {
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
    };

    let mut q =
        QueryIR::new(focus, QuestionForm::Definition, answer_shape).with_agent(noun(&agent));
    if let Some(p) = predicate {
        q = q.with_predicate(p);
    }
    if let Some(l) = language {
        q = q.with_language_filter(l);
    }
    Some(q)
}

#[derive(Debug, Clone, Copy)]
enum FocusKind {
    Time,
    Place,
    Subject,
    Object,
}

fn detect_question_focus(lower: &str) -> FocusKind {
    if lower.contains("қашан") || lower.contains("когда") {
        FocusKind::Time
    } else if lower.contains("қайда")
        || lower.contains("где")
        || lower.contains("қай қала")
        || lower.contains("какая столица")
    {
        FocusKind::Place
    } else if lower.contains("кім") || lower.contains("кто") {
        FocusKind::Subject
    } else {
        FocusKind::Object
    }
}

/// Map an input + matched agent to a likely predicate. Heuristic
/// — Stage 7 will replace with a typed router.
fn predicate_for(lower: &str, _agent: &str) -> Option<FramePredicate> {
    if lower.contains("туыл") || lower.contains("туған") || lower.contains("родился")
    {
        return Some(FramePredicate::BornIn);
    }
    if lower.contains("қайтыс") || lower.contains("өл") || lower.contains("умер") {
        return Some(FramePredicate::DiedIn);
    }
    if lower.contains("құрыл") || lower.contains("ашыл") || lower.contains("основан")
    {
        return Some(FramePredicate::FoundedIn);
    }
    if lower.contains("жаз") || lower.contains("автор") {
        return Some(FramePredicate::Authored);
    }
    if lower.contains("атымен") || lower.contains("честь") {
        return Some(FramePredicate::NamedAfter);
    }
    if lower.contains("орналас") || lower.contains("находится") || lower.contains("қайда")
    {
        return Some(FramePredicate::LocatedIn);
    }
    if lower.contains("өмір сүр")
        || lower.contains("тұр")
        || lower.contains("живёт")
        || lower.contains("жил")
    {
        return Some(FramePredicate::LivesIn);
    }
    if lower.contains("қанша") || lower.contains("сколько") {
        return Some(FramePredicate::HasQuantity);
    }
    if lower.contains("санаттар") || lower.contains("жіктейді") || lower.contains("классифиц")
    {
        return Some(FramePredicate::Classifies);
    }
    if lower.contains("күшіне") || lower.contains("вступил в силу") {
        return Some(FramePredicate::EffectiveFrom);
    }
    // Default for «X деген не?» / «что такое X?» / IsA queries.
    Some(FramePredicate::IsA)
}

/// Canonical agent surfaces we know are in `canonical_corpus`.
/// Heuristic match — picks the longest matching surface so
/// «жасанды интеллект туралы заң» wins over «заң».
fn canonical_agent_for(lower: &str) -> Option<String> {
    let candidates: &[&str] = &[
        // Multi-word first (longest wins via descending length sort).
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
        "тауке хан",
        "тірі ағза",
        "қазақ кср",
        // Single words.
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

/// Detect if the input is in Russian by Cyrillic markers
/// distinctive to Russian (vs Kazakh). Both languages share
/// Cyrillic, but Kazakh-specific characters (қ ң ғ ү ұ ө һ і ә)
/// are absent in Russian.
fn has_cyrillic_russian_marker(lower: &str) -> bool {
    let russian_words = [
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
    russian_words.iter().any(|w| lower.contains(w))
}

fn noun(s: &str) -> Composition {
    Composition::identity(Root::new(s, PartOfSpeech::Noun))
}

/// Pull the surface form of the answer slot from the matched frame.
fn answer_surface(hit: adam_algebra::RankedFrame, _q: &QueryIR) -> Option<String> {
    match hit.match_result.answer_slot {
        AnswerSlot::Agent => hit.frame.agent.as_ref().map(|c| c.root.surface.clone()),
        AnswerSlot::Object => hit.frame.object.as_ref().map(|c| c.root.surface.clone()),
        AnswerSlot::Predicate => Some(hit.frame.predicate.as_str().to_string()),
        AnswerSlot::Modifier(role) => {
            let m = hit.frame.modifier(role.as_str())?;
            modifier_surface(m)
        }
        AnswerSlot::Whole => hit
            .frame
            .object
            .as_ref()
            .map(|c| c.root.surface.clone())
            .or_else(|| hit.frame.agent.as_ref().map(|c| c.root.surface.clone())),
    }
}

fn modifier_surface(m: &adam_algebra::Modifier) -> Option<String> {
    use adam_algebra::{Modifier, TimeAnchor};
    match m {
        Modifier::TimeAnchor(TimeAnchor::Phrase(c)) => Some(c.root.surface.clone()),
        Modifier::TimeAnchor(TimeAnchor::Year(y)) => Some(format!("{y}")),
        Modifier::TimeAnchor(TimeAnchor::Date { year, month, day }) => {
            Some(format!("{year:04}-{month:02}-{day:02}"))
        }
        Modifier::Location(c)
        | Modifier::Source(c)
        | Modifier::Instrument(c)
        | Modifier::Manner(c)
        | Modifier::Recipient(c)
        | Modifier::Possessor(c) => Some(c.root.surface.clone()),
    }
}
