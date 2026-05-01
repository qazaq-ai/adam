//! `SentenceDecomposition` — case-role-aware sentence parser.
//!
//! **v4.13.0 — humanness foundation.** Three live-REPL failures from
//! 2026-05-01 (transcript: «Сіз оны бағдарламалай аласыз ба, әлі жоқ
//! па?» / «...математика, физика, химия... білесіз бе?» / «Оқушылар
//! мектепте физика пәнінен не оқиды?») all share one pathology:
//! **greedy first-noun-match** — the pipeline grabs the first content
//! noun and runs SearchGraph on it, ignoring sentence structure.
//!
//! The fix uses the typed function-composition view of Kazakh:
//! every word is `root + suffix-chain` where each case suffix is a
//! typed function `Noun → CaseMarkedNoun[Role]`. The FST already
//! decomposes this — `Analysis::Noun.features.case` carries the
//! case marker. This module converts case markers to **semantic
//! roles** via a deterministic table that has been linguistically
//! stable for ~150 years of Kazakh case-grammar tradition:
//!
//! ```text
//!   Nominative   → Subject
//!   Accusative   → Object
//!   Locative     → Locus
//!   LocativeAttr → Locus-modifier (attributive)
//!   Ablative     → Source / Topic-from
//!   Dative       → Recipient / Goal
//!   Genitive     → Possessor
//!   Instrumental → Instrument / Means
//! ```
//!
//! Plus question-word focus: `не` asks for OBJECT, `қайда` for LOCUS,
//! `қашан` for TIME, `қалай` for MANNER, `неліктен/неге` for CAUSE,
//! `кім` for SUBJECT.
//!
//! All operations are O(n) over tokens, hardmap lookups — fits a
//! CPU register, microseconds per query, fully predictable, fully
//! inspectable. **Zero ML.** When FST returns multiple parses we
//! take the first (existing v3.2.0 determinism contract); a
//! probabilistic suffix-chain prior could be layered later (v4.15+)
//! as a natural `root + function^n` learning task — but the
//! foundation here is purely structural.

use adam_kernel_fst::morphotactics::Case;
use adam_kernel_fst::parser::Analysis;

/// Top-level sentence type — distinct from `QuestionShape` which
/// refines the FORM of a question. `SentenceType` answers
/// "is this even a question?".
///
/// v4.13.0 stub on `Imperative` and `Exclamation`: the variants exist
/// for the planner to switch on, but the v4.13.0 routing only branches
/// on Question vs Statement. Imperative routing is reserved for
/// v4.13.5+ when adam learns to acknowledge commands («жасай аламын» /
/// «жасай алмаймын»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SentenceType {
    Question,
    Statement,
    Imperative,
    Exclamation,
}

/// Semantic role assigned to a token from its FST case marker (or
/// from positional / part-of-speech cues for verbs and question
/// words).
///
/// The mapping from `Case` to `Role` is the linguistically standard
/// Kazakh case-grammar table. The variants form a closed enum;
/// adding a new role is a deliberate architectural change, not a
/// data tweak.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    /// Nominative-case nominal — the agent / theme of the predicate.
    /// Includes 1st/2nd-person pronouns when they head the sentence
    /// («сіз ... білесіз бе?» → `сіз` is Subject).
    Subject,
    /// Accusative-case nominal — the direct object of the predicate.
    /// In a `не-question` sentence this is what the question is
    /// asking for («Не оқиды?» — what do they READ).
    Object,
    /// Locative case (`-да/-де/-та/-те`) — where the action takes
    /// place. Also covers `LocativeAttributive` («мектептегі білім»
    /// = "school knowledge" — the knowledge AT school).
    Locus,
    /// Ablative case (`-дан/-ден/-тан/-тен/-нан/-нен`) — source,
    /// origin, "from-X" topic. «Физика пәнінен» = "from physics" —
    /// the SCOPE of the question.
    Source,
    /// Dative case (`-ға/-ге/-қа/-ке/-на/-не`) — recipient, goal.
    /// «Маған айт» = "tell ME".
    Recipient,
    /// Genitive case (`-ның/-нің/-дың/-дің/-тың/-тің`) — possessor /
    /// modifier of a following noun.
    Possessor,
    /// Instrumental case (`-мен/-бен/-пен`) — means, accompaniment.
    Instrument,
    /// Verb root — the predicate. Carries the action.
    Predicate,
    /// Question word: `не / нені / қашан / қайда / қалай / неліктен /
    /// неге / кім / қандай`. Identifies what the question is asking
    /// FOR (object, time, place, manner, cause, subject, kind).
    QuestionWord,
    /// Modifier / closed-class word that does not contribute to the
    /// topic (postposition, demonstrative, particle, adverb).
    /// Filtered out by the focus-selection step.
    Closed,
    /// Coordination element of a topic list (the conjunction `және`,
    /// `мен/пен/бен`, the listing comma). Marks the surrounding
    /// nominals as members of a `TopicList`.
    Coord,
}

/// One token with its semantic role attached.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenRole {
    /// The lower-cased surface form as it appeared in the input.
    pub surface: String,
    /// The morphological root, when the FST gave us one. None for
    /// closed-class / unknown surfaces.
    pub root: Option<String>,
    pub role: Role,
    /// True for 3rd-person pronouns (`ол / оны / онда / соны / мұны
    /// / бұны`) — candidates for anaphora resolution.
    pub is_anaphor: bool,
}

/// Result of structural sentence decomposition. Used by the planner
/// to override the v4.11.x greedy noun-hint extractor.
///
/// The struct is intentionally small — five `Option`s and a `Vec` —
/// so storing it on `Intent::Unknown` adds no measurable memory
/// cost. Serialization-friendly via `serde` for trace dumps.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SentenceDecomposition {
    pub sentence_type: SentenceType,
    pub tokens: Vec<TokenRole>,
    /// Question word, if any (`не / қашан / қалай / ...`). Drives
    /// focus-role selection.
    pub question_word: Option<String>,
    /// The resolved focus root — what the question / statement is
    /// PRIMARILY about. Replaces greedy noun_hint downstream.
    /// `None` when no usable focus could be picked (very short
    /// utterances, all-closed-class, etc.).
    pub focus: Option<String>,
    /// Which role the focus came from. Diagnostic / for planner
    /// branch logging.
    pub focus_role: Option<Role>,
    /// Predicate root — the main verb, when one was identified.
    pub predicate: Option<String>,
    /// Multi-noun coordination — two or more nominals chained by
    /// `және / мен / пен / бен / ,`. When set, the planner can
    /// choose between a single-focus answer and a list-cohesion
    /// answer.
    pub topic_list: Option<Vec<String>>,
    /// Cohesion score in `[0.0, 1.0]`. v4.13.0 uses a coarse
    /// 3-level signal: 1.0 = single topic / clearly resolved;
    /// 0.6 = multi-topic but plausibly coherent; 0.3 = multi-topic
    /// across unrelated domains, planner should prefer a
    /// clarification fallback over greedy retrieval.
    pub cohesion: f32,
}

impl SentenceDecomposition {
    /// Quick cohesion threshold for honest-fallback routing.
    /// `< 0.5` → planner should ask for clarification rather than
    /// running greedy retrieval.
    pub fn cohesion_low(&self) -> bool {
        self.cohesion < 0.5
    }
}

/// Question words recognised by the focus extractor. The mapping
/// drives which Role gets promoted to focus when this word is
/// present.
fn question_word_focus_role(qw: &str) -> Option<Role> {
    match qw {
        "не" | "нені" | "неден" | "неге" => Some(Role::Object), // also covers "what"
        "кім" | "кімді" | "кімнің" => Some(Role::Subject),
        "қайда" => Some(Role::Locus),
        "қашан" => Some(Role::Predicate), // time-of-action — predicate-focused
        "қалай" => Some(Role::Predicate),
        "неліктен" => Some(Role::Predicate), // cause-of-action
        "қандай" => Some(Role::Subject),     // kind-of-X
        "қанша" | "неше" => Some(Role::Object),
        _ => None,
    }
}

/// Surface forms of the closed Kazakh interrogative-pronoun set.
/// Distinct from `NOT_A_TOPIC` (which is broader) because we want
/// to KEEP these in the decomposition tagged as `QuestionWord`,
/// not strip them.
const QUESTION_WORDS: &[&str] = &[
    "не",
    "нені",
    "неден",
    "неге",
    "неліктен",
    "кім",
    "кімді",
    "кімге",
    "кімнің",
    "қайда",
    "қашан",
    "қалай",
    "қандай",
    "қанша",
    "неше",
];

/// 3rd-person pronouns and demonstratives that act as anaphora when
/// they appear in object / locative position. Matched on bare
/// surface form (post-lowercase). `ол / оны / онда / содан / мұны
/// / бұл / соны`.
fn is_anaphor_surface(s: &str) -> bool {
    matches!(
        s,
        "ол" | "оны"
            | "оған"
            | "одан"
            | "онда"
            | "оның"
            | "сол"
            | "соны"
            | "соған"
            | "содан"
            | "сонда"
            | "соның"
            | "бұл"
            | "бұны"
            | "мұны"
            | "мұнда"
            | "осы"
    )
}

/// Closed-class items that should never be picked as focus, even
/// when the FST tags them as Noun. Mirrors `semantics::NOT_A_TOPIC`
/// but kept here for self-containedness — the decomposer is a
/// downstream consumer of FST analyses, not of the dialog
/// taxonomy. The two lists may diverge intentionally (e.g. the
/// decomposer wants to KEEP `не` as a QuestionWord while
/// `NOT_A_TOPIC` filters it).
fn is_decomp_closed_class(s: &str) -> bool {
    matches!(
        s,
        "мен"
            | "сен"
            | "сіз"
            | "ол"
            | "біз"
            | "сендер"
            | "сіздер"
            | "олар"
            | "бұл"
            | "мына"
            | "сол"
            | "осы"
            | "ана"
            | "туралы"
            | "бойынша"
            | "үшін"
            | "кейін"
            | "дейін"
            | "сияқты"
            | "сияқ"
            | "ретінде"
            | "арқылы"
            | "көп"
            | "аз"
            | "бәрі"
            | "барлық"
            | "әлі"
            | "әлде"
            | "мүмкін"
            | "тағы"
            | "және"
            | "пен"
            | "бен"
            | "жоқ"
            | "иә"
    )
}

/// Coordination markers — `,`, `және`, `мен/пен/бен`. When two or
/// more content nominals are separated by these, they form a
/// TopicList.
fn is_coord_surface(s: &str) -> bool {
    matches!(s, "және" | "мен" | "пен" | "бен")
}

/// Map a `Case` to a semantic `Role`. The linguistic table.
fn case_to_role(case: Case) -> Role {
    match case {
        Case::Nominative => Role::Subject,
        Case::Accusative => Role::Object,
        Case::Locative | Case::LocativeAttributive => Role::Locus,
        Case::Ablative => Role::Source,
        Case::Dative => Role::Recipient,
        Case::Genitive => Role::Possessor,
        Case::Instrumental => Role::Instrument,
    }
}

/// Detect the sentence type from raw input. Pure surface scan —
/// punctuation + closed list of imperative cues.
fn detect_sentence_type(input: &str) -> SentenceType {
    let lower = input.to_lowercase();
    if lower.contains('!') {
        return SentenceType::Exclamation;
    }
    if lower.contains('?')
        || lower.contains(" ма?")
        || lower.contains(" ме?")
        || lower.contains(" ба?")
        || lower.contains(" бе?")
        || lower.contains("ма?")
        || lower.contains("ме?")
        || lower.contains("ба?")
        || lower.contains("бе?")
        || QUESTION_WORDS.iter().any(|qw| {
            // unbounded contains is fine — overlap with non-question
            // sentences containing these surface forms is rare and
            // the planner re-checks downstream.
            lower.contains(&format!(" {qw} ")) || lower.starts_with(&format!("{qw} "))
        })
    {
        return SentenceType::Question;
    }
    // Imperative: 2nd-person command suffix `-ңыз/-ңіз` (polite) or
    // bare verb stem (informal). v4.13.0 keeps this minimal — only
    // the polite form, which is unambiguous. Bare-stem detection
    // requires verb-lookup against the lexicon and is reserved for
    // v4.13.5.
    if lower.contains("ңыз ")
        || lower.contains("ңіз ")
        || lower.ends_with("ңыз")
        || lower.ends_with("ңіз")
        || lower.contains("шы ")
        || lower.contains("ші ")
        || lower.ends_with("шы")
        || lower.ends_with("ші")
    {
        return SentenceType::Imperative;
    }
    SentenceType::Statement
}

/// Build a `SentenceDecomposition` from the raw input + FST parses
/// + (optionally) the last-turn topic for anaphora resolution.
///
/// Pure function over its arguments. Same input → same output.
pub fn decompose(
    input: &str,
    parses: &[Analysis],
    last_topic: Option<&str>,
) -> SentenceDecomposition {
    let lower = input.to_lowercase();
    let surfaces: Vec<String> = lower
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .collect();

    let sentence_type = detect_sentence_type(input);

    // Token-by-token role assignment. We walk surfaces aligned to
    // parses where possible, but the FST parser often returns FEWER
    // analyses than tokens (closed-class words don't analyse), so
    // we keep the streams parallel only for the noun-bearing
    // positions and fall back to surface-based heuristics for the
    // gaps.
    let mut tokens = Vec::with_capacity(surfaces.len());
    let mut question_word: Option<String> = None;
    let mut predicate: Option<String> = None;
    let mut subject: Option<String> = None;
    let mut object: Option<String> = None;
    let mut locus: Option<String> = None;
    let mut source: Option<String> = None;

    let mut parse_iter = parses.iter();

    for surf in &surfaces {
        // Question word recognition — surface match against the
        // closed list. Has highest priority because `не` is not a
        // content noun even though FST may parse it as one.
        if QUESTION_WORDS.contains(&surf.as_str()) {
            if question_word.is_none() {
                question_word = Some(surf.clone());
            }
            tokens.push(TokenRole {
                surface: surf.clone(),
                root: None,
                role: Role::QuestionWord,
                is_anaphor: false,
            });
            // Question words also have FST analyses (often as
            // ablative-case noun), so consume one parse to keep the
            // streams aligned.
            let _ = parse_iter.next();
            continue;
        }

        // Coordination markers.
        if is_coord_surface(surf) {
            tokens.push(TokenRole {
                surface: surf.clone(),
                root: None,
                role: Role::Coord,
                is_anaphor: false,
            });
            let _ = parse_iter.next();
            continue;
        }

        // Closed-class filter.
        if is_decomp_closed_class(surf) {
            let is_ana = is_anaphor_surface(surf);
            tokens.push(TokenRole {
                surface: surf.clone(),
                root: None,
                role: Role::Closed,
                is_anaphor: is_ana,
            });
            let _ = parse_iter.next();
            continue;
        }

        // Anaphors with case markers — `оны / соны / мұны / онда /
        // содан`. These surface forms ARE in the closed class, but
        // we want to handle them earlier so anaphora-resolution can
        // work. (Note: `is_decomp_closed_class` would catch only
        // bare `ол / бұл / сол / осы`; the cased forms slip through.)
        if is_anaphor_surface(surf) {
            // Case-bearing anaphors carry the role their case
            // dictates. Without FST data on the pronoun lookup we
            // fall back to a surface-form table.
            let role = match surf.as_str() {
                "оны" | "соны" | "мұны" | "бұны" => Role::Object,
                "онда" | "сонда" | "мұнда" | "бұнда" => Role::Locus,
                "одан" | "содан" | "мұнан" | "бұдан" => Role::Source,
                "оған" | "соған" | "мұған" | "бұған" => Role::Recipient,
                "оның" | "соның" | "мұның" | "бұның" => Role::Possessor,
                _ => Role::Subject,
            };
            // Anaphora resolution: replace the surface with
            // `last_topic` so downstream sees the resolved entity.
            let resolved = last_topic.map(|s| s.to_lowercase());
            tokens.push(TokenRole {
                surface: surf.clone(),
                root: resolved.clone(),
                role: role.clone(),
                is_anaphor: true,
            });
            // Promote to the corresponding slot ONLY if resolved.
            if let Some(r) = resolved {
                match role {
                    Role::Object => {
                        if object.is_none() {
                            object = Some(r);
                        }
                    }
                    Role::Locus => {
                        if locus.is_none() {
                            locus = Some(r);
                        }
                    }
                    Role::Source => {
                        if source.is_none() {
                            source = Some(r);
                        }
                    }
                    _ => {}
                }
            }
            let _ = parse_iter.next();
            continue;
        }

        // Content word: consult next FST analysis.
        let parse = parse_iter.next();
        match parse {
            Some(Analysis::Verb { root, .. }) => {
                if predicate.is_none() {
                    predicate = Some(root.root.clone());
                }
                tokens.push(TokenRole {
                    surface: surf.clone(),
                    root: Some(root.root.clone()),
                    role: Role::Predicate,
                    is_anaphor: false,
                });
            }
            Some(Analysis::Noun { root, features }) => {
                let role = features.case.map(case_to_role).unwrap_or(Role::Subject);
                let r = root.root.clone();
                match role {
                    Role::Subject => {
                        if subject.is_none() {
                            subject = Some(r.clone());
                        }
                    }
                    Role::Object => {
                        if object.is_none() {
                            object = Some(r.clone());
                        }
                    }
                    Role::Locus => {
                        if locus.is_none() {
                            locus = Some(r.clone());
                        }
                    }
                    Role::Source => {
                        if source.is_none() {
                            source = Some(r.clone());
                        }
                    }
                    _ => {}
                }
                tokens.push(TokenRole {
                    surface: surf.clone(),
                    root: Some(r),
                    role,
                    is_anaphor: false,
                });
            }
            None => {
                // Unparsed surface — likely a proper noun or
                // multi-word phrase the lexicon doesn't carry.
                // Treat as bare Nominative (Subject) candidate, no
                // root.
                tokens.push(TokenRole {
                    surface: surf.clone(),
                    root: None,
                    role: Role::Subject,
                    is_anaphor: false,
                });
            }
        }
    }

    // Topic-list detection. If we see a pattern of 2+ Subject-role
    // tokens separated by Coord tokens, collect them.
    let topic_list = collect_topic_list(&tokens);

    // Focus selection priority (v4.13.0):
    //   1. If question_word present, prefer the Role it asks for.
    //   2. Else, prefer Subject.
    //   3. Else, fall back to first non-closed token root.
    let (focus, focus_role) = pick_focus(
        &tokens,
        question_word.as_deref(),
        subject.as_deref(),
        object.as_deref(),
        locus.as_deref(),
        source.as_deref(),
        predicate.as_deref(),
        topic_list.as_ref(),
    );

    // Cohesion: 1.0 single-focus, 0.6 multi-topic, 0.3 multi-topic
    // with no question_word (genuinely ambiguous).
    let cohesion = compute_cohesion(&topic_list, &question_word, &focus);

    SentenceDecomposition {
        sentence_type,
        tokens,
        question_word,
        focus,
        focus_role,
        predicate,
        topic_list,
        cohesion,
    }
}

fn collect_topic_list(tokens: &[TokenRole]) -> Option<Vec<String>> {
    // Walk tokens, collect contiguous Subject/Object roots
    // separated only by Coord tokens. Need >=2 to count as a list.
    let mut out: Vec<String> = Vec::new();
    let mut last_was_content = false;
    for t in tokens {
        match (&t.role, t.root.as_ref()) {
            (Role::Subject, Some(r)) | (Role::Object, Some(r)) | (Role::Source, Some(r)) => {
                out.push(r.clone());
                last_was_content = true;
            }
            (Role::Coord, _) => {
                // Coord allowed between content tokens.
                if !last_was_content {
                    // Coord with no preceding content — break the
                    // run.
                    out.clear();
                }
            }
            (Role::Closed, _) | (Role::QuestionWord, _) => {
                // Don't reset on closed-class — postpositions can
                // appear mid-list («физика, химия туралы»).
            }
            _ => {
                // Any other role (Predicate, Locus, Recipient,
                // Possessor, Instrument) breaks the run.
                if !out.is_empty() && out.len() >= 2 {
                    return Some(out);
                }
                out.clear();
                last_was_content = false;
            }
        }
    }
    if out.len() >= 2 { Some(out) } else { None }
}

#[allow(clippy::too_many_arguments)]
fn pick_focus(
    tokens: &[TokenRole],
    question_word: Option<&str>,
    subject: Option<&str>,
    object: Option<&str>,
    locus: Option<&str>,
    source: Option<&str>,
    predicate: Option<&str>,
    topic_list: Option<&Vec<String>>,
) -> (Option<String>, Option<Role>) {
    // Question-word-driven focus.
    if let Some(qw) = question_word {
        if let Some(role) = question_word_focus_role(qw) {
            // For Subject role, prefer the Subject slot we already
            // resolved; otherwise fall through to predicate.
            let pick = match role {
                Role::Subject => subject,
                Role::Object => object.or(source).or(subject),
                Role::Locus => locus,
                Role::Source => source.or(object),
                Role::Recipient => None,
                Role::Predicate => predicate,
                _ => None,
            };
            if let Some(p) = pick {
                return (Some(p.to_string()), Some(role));
            }
        }
    }

    // Topic-list takes precedence when present — its first member
    // is a reasonable focus, but the planner gets the full list via
    // `topic_list` for cohesion-aware routing.
    if let Some(list) = topic_list {
        if let Some(first) = list.first() {
            return (Some(first.clone()), Some(Role::Subject));
        }
    }

    // Default: Subject > Object > Source > predicate > first
    // content token.
    if let Some(s) = subject {
        return (Some(s.to_string()), Some(Role::Subject));
    }
    if let Some(o) = object {
        return (Some(o.to_string()), Some(Role::Object));
    }
    if let Some(src) = source {
        return (Some(src.to_string()), Some(Role::Source));
    }
    if let Some(p) = predicate {
        return (Some(p.to_string()), Some(Role::Predicate));
    }
    for t in tokens {
        if matches!(
            t.role,
            Role::Subject | Role::Object | Role::Locus | Role::Source
        ) {
            if let Some(r) = &t.root {
                return (Some(r.clone()), Some(t.role.clone()));
            }
        }
    }
    (None, None)
}

fn compute_cohesion(
    topic_list: &Option<Vec<String>>,
    question_word: &Option<String>,
    focus: &Option<String>,
) -> f32 {
    if focus.is_none() {
        return 0.3;
    }
    match (topic_list, question_word) {
        // Single topic, focus picked → high cohesion.
        (None, _) => 1.0,
        // Multi-topic with a question_word that asks about a
        // specific role → planner can answer for the matched role.
        (Some(list), Some(_)) if list.len() <= 4 => 0.7,
        // Multi-topic, no question_word, 2-3 nouns → plausible
        // but ambiguous.
        (Some(list), None) if list.len() <= 3 => 0.6,
        // Multi-topic, 4+ nouns → genuinely ambiguous, route to
        // clarification.
        (Some(_), _) => 0.4,
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the pure-function pieces of `sentence_decomp`.
    //! Lexicon-dependent end-to-end tests live in
    //! `tests/sentence_decomp_integration.rs`; running the lexicon
    //! load from a unit-test harness fails because cargo's test
    //! `cwd` for unit tests is the crate dir, not the workspace
    //! root (lexicon paths are workspace-relative).

    use super::*;

    #[test]
    fn case_to_role_table_is_complete() {
        // Sanity: every Case variant maps to some Role. If a future
        // PR adds a new Case, this test forces an explicit decision.
        assert_eq!(case_to_role(Case::Nominative), Role::Subject);
        assert_eq!(case_to_role(Case::Accusative), Role::Object);
        assert_eq!(case_to_role(Case::Locative), Role::Locus);
        assert_eq!(case_to_role(Case::LocativeAttributive), Role::Locus);
        assert_eq!(case_to_role(Case::Ablative), Role::Source);
        assert_eq!(case_to_role(Case::Dative), Role::Recipient);
        assert_eq!(case_to_role(Case::Genitive), Role::Possessor);
        assert_eq!(case_to_role(Case::Instrumental), Role::Instrument);
    }

    #[test]
    fn question_type_detected_from_question_mark() {
        assert_eq!(
            detect_sentence_type("Сіз қалайсыз?"),
            SentenceType::Question
        );
        assert_eq!(detect_sentence_type("сәлем"), SentenceType::Statement);
    }

    #[test]
    fn question_type_detected_from_particle() {
        // `ма?` immediately after a verb form.
        assert_eq!(
            detect_sentence_type("Сіз математиканы білесіз бе?"),
            SentenceType::Question
        );
    }

    #[test]
    fn exclamation_detected_from_bang() {
        assert_eq!(
            detect_sentence_type("Қандай тамаша!"),
            SentenceType::Exclamation
        );
    }

    #[test]
    fn imperative_detected_from_polite_suffix() {
        // `айтыңыз` = polite imperative "tell me".
        assert_eq!(
            detect_sentence_type("маған айтыңыз"),
            SentenceType::Imperative
        );
    }

    #[test]
    fn question_word_recognised_in_words_list() {
        assert!(QUESTION_WORDS.contains(&"не"));
        assert!(QUESTION_WORDS.contains(&"кім"));
        assert!(QUESTION_WORDS.contains(&"неліктен"));
    }

    #[test]
    fn anaphor_surface_recognises_3rd_person_pronouns() {
        assert!(is_anaphor_surface("оны"));
        assert!(is_anaphor_surface("соны"));
        assert!(is_anaphor_surface("мұны"));
        assert!(is_anaphor_surface("онда"));
        assert!(!is_anaphor_surface("Дәулет"));
    }

    #[test]
    fn closed_class_filter_includes_v4_13_0_additions() {
        // v4.13.0 additions: әлі / әлде / мүмкін / тағы. These
        // closed-class words pre-v4.13.0 leaked into noun_hint as
        // greedy first-noun (live REPL transcript 2026-05-01:
        // «Сіз оны бағдарламалай аласыз ба, әлі жоқ па?» surfaced
        // a poetry quote about `әлі`).
        assert!(is_decomp_closed_class("әлі"));
        assert!(is_decomp_closed_class("әлде"));
        assert!(is_decomp_closed_class("мүмкін"));
        assert!(is_decomp_closed_class("тағы"));
    }

    #[test]
    fn cohesion_signals_3_levels() {
        // Single topic → high.
        assert_eq!(compute_cohesion(&None, &None, &Some("X".into())), 1.0);
        // Multi-topic with question_word → planner can resolve.
        assert!(
            compute_cohesion(
                &Some(vec!["A".into(), "B".into()]),
                &Some("не".into()),
                &Some("A".into()),
            ) > 0.5
        );
        // Multi-topic, 4+ items, no question_word → low.
        let big = vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into()];
        assert!(compute_cohesion(&Some(big), &None, &Some("A".into())) < 0.5);
    }

    #[test]
    fn focus_falls_back_to_subject_when_no_question_word() {
        let (focus, role) = pick_focus(&[], None, Some("жасуша"), None, None, None, None, None);
        assert_eq!(focus.as_deref(), Some("жасуша"));
        assert_eq!(role, Some(Role::Subject));
    }

    #[test]
    fn focus_picks_predicate_when_question_is_қалай_or_қашан() {
        // `қалай` / `қашан` ask about the predicate (manner / time
        // of action), not about a specific noun-role.
        let (_, role) = pick_focus(
            &[],
            Some("қалай"),
            Some("оқушы"),
            None,
            None,
            None,
            Some("оқиды"),
            None,
        );
        assert_eq!(role, Some(Role::Predicate));
    }
}
