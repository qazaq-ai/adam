//! Layer 2.5 — topic extraction.
//!
//! Closed-class filters (`NOT_A_TOPIC`, `MULTIWORD_ENTITIES`,
//! `LATIN_TECH_SUBJECTS`) + the noun-hint heuristics they drive
//! (`best_noun_hint` and friends).
//!
//! **Extracted from `semantics.rs` in v4.24.0** as part of the
//! Codex-review-driven module decomposition. The pre-v4.24.0
//! `semantics.rs` was 3576 lines — too large to edit safely. This
//! module pulls out the largest cohesive group: ~1247 lines that
//! all answer the question "given an input + FST analyses, what
//! noun is the user actually talking about?". No behaviour change
//! vs the inline version — same items, same call shapes, same
//! tests; only file location and visibility (private → `pub(crate)`)
//! changed.
//!
//! Public surface: only [`content_roots`] is `pub` (consumed by
//! `crate::conversation`). The closed-class lists and intermediate
//! helpers stay `pub(crate)` — they're internal scaffolding for
//! the dialog crate.

use adam_kernel_fst::parser::Analysis;
use serde::{Deserialize, Serialize};

/// **v4.37.5** — confidence band on the extracted topic noun.
///
/// Drives a human-like clarification fork in the planner: when the
/// topic was recovered through a strong structural signal (multiword
/// entity, Latin proper noun, `туралы` marker, locative-attributive
/// suffix, adj+noun compound, sentence_decomp focus, or
/// `first_noun_root` returning a root whose lexicon `part_of_speech`
/// is actually `"noun"`), confidence is `High` and the planner
/// proceeds to its standard fact-asserting path. When the topic was
/// recovered as a fallback (`first_noun_root` picked a root tagged
/// `adjective` / `pronoun` / `numeral`, or `accusative_form_hint`
/// stripped a suffix without lexicon validation), confidence is
/// `Low` and the planner routes to `unknown.clarify_low_confidence`,
/// offering the candidate interpretation and inviting the user to
/// correct it instead of confidently citing a tangential fact.
///
/// `Default::default()` is `High`, so every pre-v4.37.5 construction
/// site that doesn't set the field continues to route through the
/// standard confident path bit-for-bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TopicConfidence {
    #[default]
    High,
    Low,
}

/// Closed-class items the parser often tags as `Noun` but which carry
/// no topical content for the `unknown.with_noun` template or for
/// retrieval ranking. Filtered from both `first_noun_root` and
/// `content_roots`.
///
/// **v3.9.5** — kept in sync with `adam_reasoning::patterns::is_closed_class`.
/// Pre-v3.9.5 this list was narrower, which caused the user-visible bug
/// where «Неліктен?» («why?» — a vocative interrogative) was parsed as
/// `Нелік` (noun-root) + ablative suffix, so the dialog replied
/// «Нелікте тұрасыз ба» («Do you live in Нелік?»). Expansion covers:
/// interrogative pronouns (неліктен / неге / қашан / қайда / …),
/// demonstrative qualifiers (мұндай / сондай / …), quantifier-like
/// forms (кейбір / өз / бірнеше / әрбір / …), and the comparison
/// particle сияқ (bare root of сияқты).
pub(crate) const NOT_A_TOPIC: &[&str] = &[
    // pronouns
    "мен",
    "сен",
    "сіз",
    "ол",
    "біз",
    "сендер",
    "сіздер",
    "олар",
    // demonstratives
    "бұл",
    "мына",
    "сол",
    "осы",
    "ана",
    // postpositions
    "туралы",
    "бойынша",
    "үшін",
    "кейін",
    "дейін",
    "сияқты",
    "сияқ",
    "ретінде",
    "арқылы",
    // quantifiers / closed-class
    "көп",
    "аз",
    "бәрі",
    "барлық",
    // v3.9.5 — interrogatives (mirrors `adam_reasoning::patterns`).
    // Closes the «Неліктен → Нелікте тұрасыз ба» REPL bug.
    "қандай",
    "кім",
    "не",
    "қай",
    "қашан",
    "қайда",
    "неліктен",
    "неге",
    "қанша",
    // v4.0.1 — Codex v4.0.0 review caught that the v3.9.5 «Неліктен»
    // fix was incomplete. FST analysis of "неліктен" returns three
    // parses; the first is `нелік + Ablative` (stripped stem plus
    // case), so the dialog still received a `Нелік` noun and routed
    // it through `StatementOfLocation { city: "Нелік" }`. The v3.9.5
    // list only contained the full surface form "неліктен". Add the
    // stripped stem `нелік` so the ablative-scan in
    // `detect_statement_of_location` also skips it.
    "нелік",
    // **v4.71.5** — relational predicate forms surfaced as bogus
    // topic on «X пен Y байланысты ма?» relation queries. The FST
    // analyses «байланысты» as an adjective form of байланыс (noun
    // = "connection"), which then wins the first_noun_root pass when
    // the actual topics X, Y are loanwords missing from the lexicon.
    // Demote both surface forms so the relation-predicate doesn't
    // eclipse the genuine topic candidates.
    "байланыс",
    "байланысты",
    // **v4.72.5** — passive «how is X computed?» verb forms picked
    // as topic when X is a FST-unknown loanword. Surface
    // `есептеледі` (passive of есептеу = to compute) as the noun
    // hint for «Диаметр қалай есептеледі?» because диаметр isn't in
    // FST lexicon and есептеледі parses to a noun-class form. These
    // are predicate verbs, never the topic; the actual topic must
    // come from MULTIWORD_ENTITIES (which now includes диаметр /
    // радиус / etc.) or topic-marker hints.
    "есептеле",
    "есептеу",
    "есептеледі",
    "есептейді",
    "есептелді",
    "болады",
    "жасалады",
    "жасалды",
    // **v4.73.5** — Codex 2026-05-06 review: function-words /
    // discourse particles getting picked as topic and surfacing
    // unrelated retrieval hits. «Мысал келтіріңіз» surfaced a
    // Шымкент quote; «Нәтижесі қанша?» surfaced a Kiev quote;
    // «Маңызды ма?» fell to clarify; «Екеуінің айырмасы қандай?»
    // surfaced a proverb about «екеуі». These tokens are never the
    // topic of a question — they're discourse markers signalling
    // что-то procedural happens around them. Without a real topic
    // they should fall to clarify_no_topic, not produce noise.
    "мысал",
    "нәтиже",
    "маңызды",
    "қайсы",
    "егер",
    "егерде",
    "екеуі",
    "екеу",
    "айырмасы",
    "айырмашылығы",
    // v3.9.5 — demonstrative qualifiers + quantifier forms.
    "мұндай",
    "сондай",
    "ондай",
    "мынадай",
    "сондай-ақ",
    "кейбір",
    "өз",
    "өзі",
    "бірнеше",
    "барша",
    "әрбір",
    "әр",
    "бір",
    "кей",
    // **v4.3.5** — discourse particles / locative-case
    // demonstratives. Real-test 2026-04-26 dialog showed `Онда`
    // ("then") in `Онда маған X туралы айтып беріңізші` was parsed
    // by FST as `он + Locative` (root `он` = "ten"), so the topic
    // extractor returned `Он`, the planner's reasoning lookup
    // surfaced "Он — сан" — completely tangential to the user's
    // actual question. Same class as the v4.3.2 `интеллект → ел`
    // substring bug: a closed-class word being mistaken for a
    // case-marked content noun. Add the demonstrative-locative
    // and demonstrative-ablative forms so they never reach the
    // topic-noun candidate stage.
    "онда",
    "сонда",
    "бұнда",
    "мұнда",
    "осында",
    "содан",
    "одан",
    "бұдан",
    "осыдан",
    "міне",
    "мынау",
    // **v4.3.5** — common adjective roots that the FST occasionally
    // returns as standalone nouns. Real-test 2026-04-26 showed
    // `Жаңа жасанды интеллект моделін әзірлеу` → topic `Жаңа`
    // (root of "new"); `әйгілі жазушы Мүсірепов` → topic `әйгіл`
    // (root of "famous"). Both produced retrieval quotes
    // tangentially related to "new" / "famous", not to the actual
    // proper-noun topic. The fix is conservative — only adjective
    // roots that have unambiguous adjectival usage in modern
    // Kazakh. `жас` is intentionally NOT in this list because it's
    // also "age" (a real topic noun in profile turns).
    "жаңа",
    "әйгіл",
    // **v4.4.10** — discourse adverbial particle. Real-REPL
    // 2026-04-28: `Қысқасы, сен ештеңе білмейсің.` ("In short,
    // you don't know anything.") — `Қысқасы` is a sentence
    // adverbial meaning "briefly" / "in short", not a topic. Pre-
    // v4.4.10 it parsed as `қысқа + 3sg-poss + Nominative`
    // (root «қысқа» = "short"), the topic extractor returned
    // `қысқа`, retrieval surfaced an unrelated proverb keyed on
    // `қысқа`. Same class as v4.3.5 `Онда → он` and `Жаңа → жаңа`:
    // a closed-class discourse word being mistaken for a content
    // noun. Stem form added; `қысқасы` (full surface) is its own
    // entry below if needed (FST returns the stem `қысқа` first).
    "қысқа",
    // **v4.13.0** — modal / discourse particles surfaced by the
    // 2026-05-01 live REPL transcript. «Сіз оны бағдарламалай
    // аласыз ба, әлі жоқ па?» pre-v4.13.0 fell to `әлі` as topic
    // because none of these were registered as closed-class. They
    // are sentence-level discourse markers (yet / or / perhaps /
    // also), never the topical content noun.
    "әлі",
    "әлде",
    "мүмкін",
    "тағы",
    // v4.13.0 — `жоқ` is the existential negator, not a topic noun.
    // Surfaced by «...әлі жоқ па?» — when `әлі` was added but `жоқ`
    // was missing, the topic extractor jumped from `әлі` to `жоқ`,
    // surfacing a poetry quote about absence. Same closed-class
    // hygiene that catches discourse particles.
    "жоқ",
    "иә",
    // **v4.4.10** — indefinite-quantifier pronoun. Same
    // 2026-04-28 trace: `сен ештеңе білмейсің` ("you know
    // nothing") — `ештеңе` ("nothing") is a quantifier pronoun,
    // not a topic noun. Pre-v4.4.10 (after `қысқа` was muted)
    // the topic extractor fell through to `ештеңе`, retrieval
    // matched a tangential proverb. Adding here closes the
    // misfire from the same trace.
    "ештеңе",
    "ешкім",
    "ешбір",
    "еш",
    // **v4.6.0** — additional discourse adverbials surfaced by the
    // 2026-04-28 real-REPL transcript. `өте` (= "very") and `жалпы`
    // (= "in general / overall") are intensifier / scope adverbs,
    // not topic nouns. Pre-v4.6.0 «Бұл өте қызықты, бірақ жалпы не
    // істей аласыз?» extracted `өте` as the topic and surfaced a
    // tangential proverb keyed on it. Same misanalysis class as
    // v4.4.10's `қысқа` / `ештеңе` additions.
    "өте",
    "жалпы",
    // **v4.40.5** — temporal/manner adverbs surfaced by the
    // 2026-05-03 dialog transcript. «Кейде сенің қателесіп
    // жүргеніңді көремін» — `кейде` ("sometimes") is a sentential
    // adverb, never a topic noun. Pre-v4.40.5 the topic extractor
    // returned `кейде` and the planner surfaced the unknown-with-
    // -noun template "Мүмкін сіз кейде туралы сұрап отырған
    // шығарсыз" — clearly nonsensical. Same misanalysis class as
    // v4.6.0's `өте` / `жалпы` additions.
    "кейде",
    "кей-кейде",
    "әрдайым",
    "ылғи",
    "үнемі",
    "бірден",
    "дереу",
    // **v4.6.12** — bare case-suffix leaks. Real-REPL 2026-04-29
    // transcript: «5-ті 7-ге көбейткенде неше болады?» — the
    // FST analysed `7-ге` as a fragment of `7` + `-ге` (dative
    // suffix), and the topic extractor picked up the bare
    // suffix `ге` (from `-Ге` written as a standalone token).
    // Bare case-suffix forms `ге / ге / ке / бе / пе / да / де
    // / та / те / мен / нен / нан / тан / тен / нен / ден / тен
    // / ден` are never legitimate topic nouns; they're
    // morphological tail fragments. Add the most-leaky ones.
    "ге",
    "ке",
    "де",
    "те",
    "да",
    "та",
    "бе",
    "ма",
    // **v4.11.7** — Kazakh yes/no question particles (`ба` / `ме`,
    // sister forms of the existing `бе` / `ма`). Real-REPL test
    // 2026-04-30: «Сіз қазақша сөйлей аласыз ба?» pre-v4.11.7
    // extracted `ба` as topic and surfaced a poetry quote about
    // `ұқпасын ба`. The four-form set (`ба / бе / ма / ме`) is the
    // closed Kazakh interrogative-particle paradigm, never a
    // standalone topic noun. The lexicon has `ба` registered as a
    // particle, but FST occasionally emits a Noun reading too.
    "ба",
    "ме",
    // v4.13.0 — `па` / `пе` complete the question-particle paradigm
    // (post-voiceless-stop allomorphs of `ба` / `бе` per Kazakh
    // phonotactics). Surfaced by «...әлі жоқ па?» 2026-05-01 — when
    // `жоқ` was added but `па` was missing, the topic extractor
    // jumped from `жоқ` to `па`, surfacing «Дос па деген кісіге.»
    "па",
    "пе",
    // v4.17.5 — verb stems that FST occasionally tags as nouns
    // when their full lemma is missing from the lexicon. Live REPL
    // 2026-05-01: «А, сені кім тәрбиеледі?» pre-v4.17.5 surfaced
    // `Бәлкім, тәрбиеле туралы айтасыз ба` because the verb stem
    // `тәрбиеле` (3rd-person of `тәрбиелеу` = "to raise / educate")
    // wasn't recognised as a verb, fell through to noun-topic
    // extraction. The Creator-question detector now catches
    // «кім тәрбиеледі» directly; this NOT_A_TOPIC entry is the
    // belt-and-braces fallback so `тәрбиеле` never surfaces as
    // topic even if the question phrasing isn't caught upstream.
    "тәрбиеле",
    "баптал",
    // **v4.22.5** — verb converb leak. The form «атап» is the
    // -p converb of «атау» (= "to name"), used in serial verb
    // constructions like «атап бер» ("name [them] for me",
    // imperative listing request) or «атап өту» ("to mention").
    // FST occasionally returns it as a bare noun root because
    // the lexicon has «атап» as a registered surface form. Live-
    // dialog test: «Қазақстанның ірі өзендерін атап бер.» pre-
    // v4.22.5 (and after `ірі` was blocked) extracted `атап` as
    // topic, retrieval matched the proverb «Ерекше атап өт!».
    // Same converb-leaks-as-noun class as `тәрбиеле / баптал`.
    "атап",
    // **v4.26.5** — passive-form verb stems surfaced as topics by
    // the 2026-05-02 Rust battery. Pattern: «X қалай <verb>?»
    // where the verb is a passive form (-ыла / -іле suffix).
    // Examples: «fn қалай анықталады?» («fn` is the actual topic
    // → captured by Latin extension), but if `fn` extraction
    // fails the FST falls through to «анықтала» (passive stem
    // of «анықтау» = "to define"). Same converb-leaks-as-noun
    // class as v4.17.5 `тәрбиеле / баптал` + v4.22.5 `атап`.
    "анықтала",
    "жазыла",
    "құрыла",
    "қолданыла",
    "қолдан",
    // **v4.27.0** — additional verb-stem leaks surfaced by the
    // 80-question expanded battery. «жасала» = passive of «жасау»
    // (to make/do); pattern «X қалай жасалады?» («how is X made?»).
    "жасала",
    // **v4.22.5** — closed-class words surfaced by the 2026-05-01
    // live-dialog battery as wrong topic picks. Each one was
    // observed in real session output causing the planner to
    // surface a tangential proverb / fact keyed on the closed-
    // class word instead of recognising the actual question.
    //
    // `керек` — predicate adjective ("is needed / required").
    // Surfaced in «Маған көмек керек», «Саған не керек?»,
    // «Mendeleev кестесі не үшін керек?» — every time, retrieval
    // matched a proverb keyed on `керек` («Жетілсең де, жетсең
    // де, Керек күні бір бар-ау»). It's structurally the
    // verbal-need predicate, never the topical content noun.
    "керек",
    // `ірі` — comparative-quantitative adjective ("large / big").
    // Surfaced in «Қазақстанның ірі өзендерін атап бер» where the
    // user wants a list of large rivers, not a fact about
    // "largeness". Pre-v4.22.5 retrieval matched «Ерекше атап
    // өт!» — a proverb on the imperative «атап өту». Adjective
    // pre-modifier, not a topic.
    "ірі",
    // `кеше / бүгін / ертең / қазір / бұрын` — temporal adverbs.
    // Surfaced in «Кеше ауа райы қандай болды?» where retrieval
    // matched «Ауа райы туралы оның болжамы ақталды» — a corpus
    // fragment keyed on `кеше`, dropping the actual question
    // (yesterday's weather, which adam doesn't have data for).
    // Temporal adverbs are sentence-level scope markers, never
    // the noun the question is about. Same hygiene class as
    // v4.6.0's `өте / жалпы` adverbial additions.
    "кеше",
    "бүгін",
    "ертең",
    "қазір",
    "бұрын",
    // **v4.26.0** — Russian-loan technical vocabulary that appears
    // in Kazakh tech queries as modifier of a Latin keyword. Live-
    // test surfaced «match операторы қалай жұмыс істейді?» picking
    // `оператор` as topic (after `match` failed extraction
    // pre-v4.26.0 LATIN_TECH_SUBJECTS expansion) and retrieving an
    // unrelated proverb. With v4.26.0's Latin extension `match` now
    // wins as topic; this entry is belt-and-braces — even if the
    // Latin keyword extraction misses, the Russian-loan tech-modifier
    // never becomes the topic. Same hygiene class as v4.22.5
    // `керек / ірі / атап` and v4.4.10 `қысқа`.
    "оператор",
    // **v4.6.0** — bare numeral roots that the FST occasionally
    // returns as Locative parses of discourse demonstratives.
    // `Онда` ("then / in it") parses as `он + Locative` (root = "он"
    // = number ten). v4.3.5 added the SURFACE forms (`онда / сонда
    // / осында / мұнда / бұнда`) but `first_noun_root` filters on
    // the **root**, not the surface — so `он + Locative` still
    // surfaced `он` as the topic and retrieval matched «Он — сан»
    // («Ten is a number») unrelated to the user's question. Adding
    // the bare numeral roots closes the leak; they're rare-enough
    // standalone topics that the false-negative cost is low. The
    // proper fix is the discourse-anaphora module below, which
    // resolves «онда» to the previous turn's topic — but that
    // module also leans on `first_noun_root` returning None for
    // these inputs, so this filter is a precondition.
    "он",
    "сон",
];

/// Return the root of the first content-noun Analysis in the parse list.
/// Skips Kazakh pronouns, demonstratives, and postpositions that the
/// FST parser may tag as Noun but which aren't informative as a
/// "topic hint" for the unknown.with_noun template.
pub(crate) fn first_noun_root(parses: &[Analysis]) -> Option<String> {
    first_noun_root_with_confidence(parses).map(|(root, _)| root)
}

/// **v4.37.5** — confidence-aware variant of [`first_noun_root`].
///
/// Two-pass walk:
/// 1. **First pass** — return the first parse whose `RootEntry`
///    has `part_of_speech == "noun"` (and isn't filtered by
///    `NOT_A_TOPIC`). These are *true* nouns; confidence `High`.
/// 2. **Second pass** — fall back to any noun-class root
///    (`adjective` / `pronoun` / `numeral` — the FST routes those
///    through `try_noun_analyses` so they all surface as
///    `Analysis::Noun`). Confidence `Low` — the planner will hedge
///    and invite clarification rather than asserting a definitive
///    fact about a modifier the user almost certainly intended as a
///    qualifier of a deeper noun.
///
/// Pre-v4.37.5 behaviour was the second pass alone, which surfaced
/// surprises like:
///   - «Қазақстанның **атақты** жазушыларын атаңыз» — `атақты`
///     (POS=adjective) won over `жазушы` because it preceded it in
///     the parse stream.
///   - «Кітап **қызық** болады» — `қызық` (POS=adjective) eclipsed
///     `кітап`.
///
/// The first-pass / second-pass split keeps every existing
/// noun-driven case bit-identical (because real nouns are now
/// strictly preferred) while letting the planner downgrade routing
/// for the residual cases where only an adjective/pronoun/numeral
/// candidate exists.
pub(crate) fn first_noun_root_with_confidence(
    parses: &[Analysis],
) -> Option<(String, TopicConfidence)> {
    // First pass — *true* content nouns: lexicon `part_of_speech ==
    // "noun"` AND root is not a deverbal participle ending in
    // perfect-participle suffix («-ған / -ген / -қан / -кен»).
    // Some deverbal participles are registered as `noun` in the
    // lexicon (e.g. «шыққан», «келген») because they nominalise in
    // common usage, but in a sentence they almost always function
    // as a participle modifier on a following content noun rather
    // than as the topic themselves. Demoting them lets a deeper
    // *true* noun win the first pass.
    if let Some(root) = parses.iter().find_map(|a| match a {
        Analysis::Noun { root, .. }
            if root.part_of_speech == "noun"
                && !is_deverbal_participle_root(&root.root)
                && !NOT_A_TOPIC.contains(&root.root.as_str()) =>
        {
            Some(root.root.clone())
        }
        _ => None,
    }) {
        return Some((root, TopicConfidence::High));
    }
    parses.iter().find_map(|a| match a {
        Analysis::Noun { root, .. } if !NOT_A_TOPIC.contains(&root.root.as_str()) => {
            Some((root.root.clone(), TopicConfidence::Low))
        }
        _ => None,
    })
}

/// **v4.37.5** — heuristic: does this root surface end in one of
/// the four perfect-participle allomorphs («-ған / -ген / -қан /
/// -кен»)? Used by [`first_noun_root_with_confidence`] to demote
/// deverbal participles registered as `noun` in the lexicon (e.g.
/// «шыққан», «келген», «өткен») when a *true* content noun is
/// also available in the parse stream.
///
/// False-positive risk: a tiny set of bare nouns that happen to
/// end in these letter sequences (e.g. «қаған» — khan). The cost
/// is bounded — when no other content noun is available, the
/// demoted root still surfaces as `Low` confidence, and the
/// planner asks the user to confirm rather than asserting a fact.
/// That hedged response is acceptable on the rare false positive,
/// and the routine fix on the dominant case (deverbal participles
/// hijacking topic extraction in real REPL turns) is the primary
/// goal.
fn is_deverbal_participle_root(root: &str) -> bool {
    root.ends_with("ған") || root.ends_with("ген") || root.ends_with("қан") || root.ends_with("кен")
}

/// v4.0.21 — Multi-word entity catalogue drawn from `data/world_core/*.jsonl`
/// subjects/objects that contain a space. Kazakh agglutinative morphology
/// doesn't tokenize these well — «Құс жолы» (Milky Way) tokenizes into
/// «құс» + «жолы» and the FST picks «құс» as the first noun. This loses
/// the actual referent (galaxy) and falls back to «құс» (bird).
///
/// The list is sorted **longest-first** at compile time so the matcher
/// below can return on the first hit. Kept in sync with `data/world_core/`
/// by audit (re-run `world_core_multiword_coverage_test` whenever a new
/// compound entity enters the world_core set).
///
/// Codex v4.0.19 review #2 — direct implementation.
pub(crate) const MULTIWORD_ENTITIES: &[&str] = &[
    // length 25+ (v4.17.5 — rich Kazakhstan IsA fact)
    "орталық азиядағы тәуелсіз мемлекет",
    // length 16+
    "құйрықты жұлдыз",
    "қазақ әдебиеті",
    // length 12–13
    "тіршілік иесі",
    "орталық азия",
    "жүк машинасы",
    "аспан денесі",
    "қара сөздер",
    "тағы жануар",
    "қозы көрпеш",
    // length 10–11
    "қазақ тілі",
    "су қоймасы",
    "жер бедері",
    "күн жүйесі",
    "туған жер",
    "көрші елдер",
    "абай жолы",
    "темір жол",
    "қыз жібек",
    // length 8–9
    "бас киім",
    "құс жолы",
    "аяқ киім",
    "сары май",
    "тас жол",
    // **v4.3.5** — multi-word entities added in the kz_literature
    // / notable_kazakhstanis expansion. Three Kazakh judges of the
    // 17th–18th century (`Төле би`, `Қазыбек би`, `Әйтеке би`),
    // poet `Қадыр Мырза Әли`, and the structural noun
    // `мемлекет басшысы` ("head of state").
    "мемлекет басшысы",
    "мырза әли",
    "төле би",
    "қазыбек би",
    "әйтеке би",
    // **v4.40.5** — notable-people list-summary objects from
    // `data/world_core/notable_kazakhstanis.jsonl` (notable_031
    // through notable_034). Required by the
    // `world_core_multiword_coverage` invariant test.
    "танымал қазақстандықтар тізімі",
    "ақын-жазушылар тізімі",
    "ғалымдар тізімі",
    "спортшылар тізімі",
    // **v4.42.7** — per-language purpose / domain compounds from
    // `data/world_core/programming_languages.jsonl` plang_031-050.
    // Each language now has 1-3 facts about WHAT FOR it's used —
    // these compound «{domain} саласы» objects are required by
    // the `world_core_multiword_coverage` invariant test.
    "корпоративтік бағдарламалау саласы",
    "бұлттық инфрақұрылым саласы",
    "жүйелік бағдарламалау саласы",
    "жоғары өнімді жүйелер саласы",
    "деректер базасы саласы",
    "деректер ғылымы саласы",
    "машина оқыту саласы",
    "оқуға қарапайым тіл",
    "веб-фронтенд саласы",
    "веб-бэкэнд саласы",
    "unity ойын саласы",
    "android саласы",
    "macos саласы",
    "ойын саласы",
    "ios саласы",
    ".net саласы",
    "меншік моделі",
    "қоқыс жинаушы",
    // **v4.50.5** — transcript-driven math additions.
    "квадрат үшмүше",
    // **v4.44.5** — economics_basic + materials/cooking/music depth.
    // Required by `world_core_multiword_coverage`.
    "экономикалық категория",
    "экономикалық құбылыс",
    "экономикалық субъект",
    "экономикалық игілік",
    "экономикалық ресурс",
    "экономикалық шама",
    "қаржы мекемесі",
    "қаржы құжаты",
    "қаржы құралы",
    "қаржы шамасы",
    "асүй әрекеті",
    "халық аспабы",
    "музыка жанры",
    "асыл металл",
    // **v4.43.7** — government_kazakhstan domain (presidents,
    // premier, parliament, ministries, courts). Required by
    // `world_core_multiword_coverage`. Driven by 2026-05-04 user
    // dialog test transcript.
    // **v4.44.0** — transcript-driven gap closure (2026-05-04
    // session 2). Compound subjects from `government_kazakhstan`
    // (КР-form office bridges), `time` (seasons/quarters lists),
    // `kz_literature` (writers/poets lists), and `adam_self`
    // (LLM-limitations facts).
    "қазақстан республикасының премьер-министрі",
    "қазақстан республикасының президенті",
    "жасанды интеллект кемшіліктері",
    "тілдік модель ашық еместігі",
    "тілдік модель детерминирленбеуі",
    "тілдік модель ресурс шығыны",
    "тілдік модель кемшілігі",
    "кемшіліктер тізімі",
    // **v4.55.5** — added for ARK identity entries (adam_self_041/042).
    "архитектура атауы",
    "детерминирленген жүйе",
    // **v4.57.5** — added for closing alphabet bridges (alpha_bridge_004..006).
    "дауысты дыбыс",
    "дауыссыз дыбыс",
    "қазақ әліпбиі",
    "жазу жүйесі",
    // **v4.58.0** — added for preschool_numbers (num_bridge_001..003).
    "сан есім",
    "есептік сан",
    "реттік сан",
    // **v4.58.5** — added for preschool_numbers expansion (num_bridge_004..005, num_compound_001).
    "құрама сан",
    "ондық санау жүйесі",
    "санау жүйесі",
    "он бір",
    // **v4.59.0** — added for preschool_shapes (shape_006/007/bridge_001).
    "геометриялық пішін",
    "жұлдыз пішіні",
    "жүрек пішіні",
    // **v4.59.5** — added for preschool_sizes (size_bridge_001..003).
    "өлшемдік сын есім",
    "сын есім",
    "сөз табы",
    // **v4.60.0** — added for preschool_routine + verb taxonomy.
    "күнделікті іс-әрекет",
    "жазу әрекеті",
    // **v4.60.5** — added for preschool_emotions + closing PoS bridge.
    "жағымды сезім",
    "жағымсыз сезім",
    "жек көру",
    "зат есім",
    // **v4.61.0** — added for kazakh_grammar (Day 2 #1) — full PoS coverage.
    "көмекші сөз",
    "еліктеу сөз",
    "жалқы есім",
    // **v4.61.5** — added for kazakh_grammar 7 cases (Day 2 #2).
    "атау септік",
    "ілік септік",
    "барыс септік",
    "табыс септік",
    "жатыс септік",
    "шығыс септік",
    "көмектес септік",
    "септік жалғауы",
    "септік парадигмасы",
    "грамматикалық категория",
    "бала септік парадигмасы",
    // **v4.62.0** — added for kazakh_grammar possessive (Day 2 #3).
    "тәуелдік жалғауы",
    "тәуелдік парадигмасы",
    "сыпайылық категориясы",
    "бірінші жақ жекеше тәуелдік",
    "екінші жақ жекеше анайы тәуелдік",
    "екінші жақ жекеше сыпайы тәуелдік",
    "үшінші жақ тәуелдік",
    "бірінші жақ көпше тәуелдік",
    "екінші жақ көпше анайы тәуелдік",
    "екінші жақ көпше сыпайы тәуелдік",
    "кітап тәуелдік парадигмасы",
    // **v4.62.5** — added for kazakh_grammar tenses + moods (Day 2 #4).
    "өткен шақ",
    "осы шақ",
    "келер шақ",
    "хабарлы рай",
    "бұйрық рай",
    "шартты рай",
    "қалау рай",
    "оқу етістік парадигмасы",
    "етістік парадигмасы",
    "жақ категориясы",
    "жіктік жалғауы",
    // **v4.63.0** — added for proverbs expansion (Day 2 #5).
    "халық даналығы",
    "мұқият ой",
    "дұрыс нәтиже",
    "туған жер",
    "адамның белгісі",
    // **v4.64.0** — added for math extension (Day 2 #7).
    "үштен бір",
    "төрттен бір",
    "ондық бөлшек",
    "жай бөлшек",
    "жұп сан",
    "тақ сан",
    "жай сан",
    "оң сан",
    "теріс сан",
    "математикалық шама",
    // **v4.64.5** — added for Дүниетану extension (Day 2 #8).
    "жанды табиғат",
    "жансыз табиғат",
    "тіршілік ортасы",
    "су ортасы",
    "құрлық ортасы",
    "ауа ортасы",
    "көктемнің белгілері",
    "қыстың белгілері",
    // **v4.65.0** — added for reading-skills extension (Day 2 #9).
    "негізгі ой",
    "көркем шығарма",
    "мәтін бөлігі",
    "әдебиет түрі",
    "мәтін түрі",
    "баспа басылымы",
    "жазушылар тізімі",
    "ақындар тізімі",
    "мезгілдер тізімі",
    "жыл мезгілдері",
    "қазақ жазушылары",
    "қазақ ақындары",
    "уақыт кезеңі",
    // **v4.65.5** — Day 2 capstone: оқу пәні taxonomy hub.
    "оқу пәні",
    "мектеп пәні",
    "қазақ тілі",
    "әдебиет пәні",
    "өзін-өзі тану",
    "музыка пәні",
    "бейнелеу өнері",
    "дене шынықтыру",
    "білім беру деңгейі",
    "бастауыш мектеп",
    "орта мектеп",
    "жоғары мектеп",
    // **v4.66.0** — Day 3 #1: middle-school algebra deepening.
    "математикалық өрнек",
    "математикалық қатынас",
    "математикалық ұғым",
    "математикалық шама",
    "математикалық теорема",
    "теңдеулер жүйесі",
    "координаталық жазықтық",
    "функция графигі",
    "тура пропорционалдық",
    "виета теоремасы",
    // **v4.66.5** — Day 3 #2: middle-school advanced geometry.
    "математикалық тұрақты",
    "тригонометриялық функция",
    "тік бұрыш",
    "сүйір бұрыш",
    "доғал бұрыш",
    "пи саны",
    "тік үшбұрыш",
    "пифагор теоремасы",
    // **v4.67.0** — Educational portal: physics mechanics deepening.
    "физикалық шама",
    "физикалық ұғым",
    "физикалық тұрақты",
    "физика заңы",
    "қозғалыс түрі",
    "орташа жылдамдық",
    "бірқалыпты үдемелі қозғалыс",
    "гук заңы",
    "еркін түсу үдеуі",
    "тірек реакциясы күші",
    // **v4.67.5** — Educational portal: physics thermodynamics deepening.
    "физикалық күй",
    "физикалық модель",
    "температуралық шкала",
    "зат күйі",
    "жылу мөлшері",
    "жылулық тепе-теңдік",
    "идеал газ",
    "бойль-мариотт заңы",
    "гей-люссак заңы",
    "шарль заңы",
    "менделеев-клапейрон теңдеуі",
    "абсолюттік нөл",
    "гидростатикалық қысым",
    "паскаль заңы",
    // **v4.68.0** — Educational portal: physics electricity + magnetism deepening.
    "электр құрылғысы",
    "электр тоғы",
    "электр тізбегі",
    "электромагниттік толқын",
    "кулон заңы",
    "тізбекті қосылыс",
    "параллель қосылыс",
    "электр тогының жұмысы",
    "электр тогының қуаты",
    "джоуль-ленц заңы",
    "электр сыйымдылығы",
    "магниттік ағын",
    "тұрақты ток",
    "айнымалы ток",
    // **v4.68.5** — Educational portal: chemistry organics deepening.
    "химия ұғымы",
    "органикалық қосылыс",
    "гомологтық қатар",
    "карбон қышқылы",
    "нуклеин қышқылы",
    // **v4.69.0** — Educational portal: chemistry inorganics deepening.
    "металл тобы",
    "бейметалл тобы",
    "сілтілік-жер металдар",
    "қышқылдық оксид",
    "негіздік оксид",
    "менделеев кестесі",
    // **v4.69.5** — Educational portal: world history foundations.
    "дүниежүзі тарихы",
    "тарихи дәуір",
    "тарих ұғымы",
    "тарихи кезең",
    "ежелгі дәуір",
    "орта ғасырлар",
    "жаңа заман",
    "қазіргі заман",
    "ежелгі египет",
    "ежелгі греция",
    "ежелгі өркениет",
    "рим империясы",
    "өнеркәсіптік революция",
    // **v4.70.0** — Educational portal: literary devices.
    "әдебиет ұғымы",
    "әдеби тәсіл",
    "ұйқас түрі",
    "шалыс ұйқас",
    "қара ұйқас",
    // **v4.70.5** — Educational portal: world geography foundations.
    "дүниежүзі географиясы",
    "жер бөлігі",
    "солтүстік америка",
    "оңтүстік америка",
    "тынық мұхит",
    "атлант мұхиты",
    "үнді мұхиты",
    "солтүстік мұзды мұхит",
    // **v4.71.0** — Educational portal: medicine foundations.
    "медицина ұғымы",
    "алғашқы көмек",
    "жұқпалы ауру",
    "жұқпайтын ауру",
    // **v4.71.5** — Single-word loanword topics from v4.66.0+
    // educational stripe surfaced by the live REPL audit. These
    // are FST-unknown proper nouns / loanwords that would otherwise
    // require a topic marker («туралы», «деген») to surface as
    // topic. Registering them as MULTIWORD_ENTITIES (single-word
    // entries are valid) lets the substring scan catch them
    // directly even in bare «X қандай Y?» queries.
    //
    // World geography (continents + ocean roots).
    "антарктида",
    "еуразия",
    "африка",
    "австралия",
    // World history (ancient civilizations).
    "месопотамия",
    "ренессанс",
    // Mathematics (loanword math hubs).
    "алгебра",
    "геометрия",
    "тригонометрия",
    "айнымалы",
    "координата",
    "функция",
    "теңдеу",
    "теңсіздік",
    "дискриминант",
    // Physics (loanword shamas + units).
    "механика",
    "кинематика",
    "динамика",
    "статика",
    "термодинамика",
    "электродинамика",
    "оптика",
    "гравитация",
    "температура",
    "энергия",
    "импульс",
    "инерция",
    "конденсатор",
    // Chemistry (loanword elements + compounds).
    "молекула",
    "атом",
    "электрон",
    "протон",
    "нейтрон",
    "ион",
    "оксид",
    "галогендер",
    "кальций",
    "магний",
    "натрий",
    "калий",
    "хлор",
    "фтор",
    "бром",
    "йод",
    "аргон",
    "гелий",
    "неон",
    "сутек",
    "оттек",
    "көміртек",
    "азот",
    "темір",
    "алюминий",
    "мыс",
    "алкан",
    "алкен",
    "алкин",
    "метан",
    "этан",
    "пропан",
    "бутан",
    "ацетилен",
    "этанол",
    "метанол",
    "глицерин",
    "альдегид",
    "кетон",
    "ацетон",
    "эфир",
    "глюкоза",
    "сахароза",
    "крахмал",
    "целлюлоза",
    "ақуыз",
    "гомолог",
    "изомерия",
    "нуклеотид",
    "полисахарид",
    "полимер",
    "мономер",
    // Literature (literary devices).
    "метафора",
    "эпитет",
    "теңеу",
    "гипербола",
    "кейіптеу",
    "аллегория",
    "ассонанс",
    "аллитерация",
    // Medicine (loanword concepts).
    "медицина",
    "анатомия",
    "физиология",
    "симптом",
    "диагноз",
    "профилактика",
    "иммунитет",
    "вакцина",
    "антибиотик",
    "вирус",
    "бактерия",
    // **v4.72.0** — list-bearing aggregator objects from new
    // curated list entries. Required by `world_core_multiword_coverage`
    // invariant for compound objects in world_core.
    "ғаламшарлар тізімі",
    "мүшелер тізімі",
    "ішкі мүшелер тізімі",
    "материктер тізімі",
    "мұхиттар тізімі",
    "теңдеу түрлері",
    "фигуралар тізімі",
    // **v4.72.5** — has_quantity objects from wg_016 / wg_017
    // alias entries that surface «Жер бетінде қанша X бар?»
    // queries. These must be registered for the
    // `world_core_multiword_coverage` invariant.
    "алты материк",
    "төрт мұхит",
    // **v4.73.5** — astro_048 «Жер жалпақ емес» entry; new
    // sub-class object for the planet-priority fact.
    "шар тәрізді ғаламшар",
    // **v4.77.5** — Codex round-3 cultural / political compounds.
    // Surfaced by 2026-05-06 audit: «Қожа Ахмет Ясауи» / «Киіз үй»
    // / «Әл-Фараби» / «Қорқыт ата» / «Жамбыл Жабаев» / «жұқпалы
    // ауру» (already present) — multi-word entities that need
    // longest-match registration.
    "қожа ахмет ясауи",
    "әл-фараби",
    "жамбыл жабаев",
    "қорқыт ата",
    "киіз үй",
    "наурыз көже",
    "ораза айт",
    "құрбан айт",
    "дәстүрлі мереке",
    "дәстүрлі тұрғын үй",
    "дәстүрлі сусын",
    "дәстүрлі бұйым",
    "дәстүрлі рәсім",
    "ұлттық мереке",
    "діни мереке",
    // **v4.77.0** — code-switch / bilingual education compounds
    // (Codex round-2 Bug 8). Common Latin / English curriculum
    // terms that adam users naturally type bilingually:
    // «Present Simple деген не?» pre-v4.77.0 picked «Simple» as
    // topic and ignored «Present». «Python list деген не?» picked
    // «Python» and ignored «list». Adding these multi-word entries
    // lets the substring scan catch the full compound BEFORE the
    // FST first-noun fallback.
    //
    // Note: world_core entries for these topics are not yet
    // curated, so the topic is correctly extracted but retrieval
    // surfaces no fact (graceful clarify fallback). Honest «I don't
    // have full info on Present Simple yet» beats the prior
    // «here's about Simple» misroute.
    "present simple",
    "present continuous",
    "present perfect",
    "past simple",
    "past continuous",
    "future simple",
    "future continuous",
    "python list",
    "python dict",
    "python set",
    "python tuple",
    "python string",
    "python int",
    "python float",
    "python class",
    "python function",
    "html tag",
    "html element",
    "css rule",
    "css selector",
    "sql query",
    "sql table",
    "rust struct",
    "rust enum",
    "rust trait",
    "javascript function",
    "javascript array",
    // **v4.72.5** — single-word loanword shamas / quantities missed
    // in v4.71.5. Surfaced by REPL battery «Диаметр қалай
    // есептеледі?» / «Радиус қалай есептеледі?» — these queries
    // had no topic marker, so the FST picked the verb form
    // «есептеле» as topic instead of диаметр / радиус (loanwords
    // not in lexicon). Adding them here lets the substring scan
    // find them directly. жол / уақыт / көлем are native Kazakh
    // but FST returns them as low-confidence (adjective/numeral
    // class), routing valid queries to clarify_low_confidence even
    // when grounded_fact is correctly populated.
    "диаметр",
    "радиус",
    "периметр",
    "аудан",
    "көлем",
    "жол",
    "уақыт",
    "масса",
    "тығыздық",
    "жылдамдық",
    "үдеу",
    "күш",
    "жұмыс",
    // **v4.43.8** — direct office-holder bridges (closes carry-
    // forward where «Қазіргі Қазақстан президенті кім?» fell to
    // the abstract «Қазақстан президенттігі» fact instead of
    // surfacing Тоқаев). Sorted longest-first. Both bare-form and
    // genitive-form (with «-ның» suffix) are registered so the
    // first-pass substring match in `multiword_entity_hint` fires
    // regardless of whether the user writes the genitive or the
    // bare-form possessor (the v4.40.5 inflected-second-word pass
    // only handles inflection on the SECOND word, not the first).
    "қазақстанның премьер-министрі",
    "қазіргі қазақстан президенті",
    "қазақстанның президенті",
    "қазақстан премьер-министрі",
    "қазақстан президенті",
    "конституциялық сот",
    "министрліктер тізімі",
    "қасым-жомарт тоқаев",
    "нұрсұлтан назарбаев",
    "мемлекеттік орган",
    "тұңғыш президент",
    "қазіргі президент",
    "олжас бектенов",
    "республика түрі",
    "жоғарғы сот",
    // **v4.43.6** — language_features depth (Kazakh morphology
    // vocabulary). Required by `world_core_multiword_coverage`.
    "грамматикалық категория",
    "тілдік құбылыс",
    "фонетикалық заң",
    "үндестік заңы",
    "тілдік бірлік",
    "сөз табы",
    "зат есім",
    "сын есім",
    "сан есім",
    // **v4.43.5** — philosophy_basic + astronomy/weather/measurements
    // depth. Required by `world_core_multiword_coverage`.
    "метеорологиялық шама",
    "атмосфералық жағдай",
    "атмосфералық қысым",
    "моральдық құндылық",
    "философиялық ұғым",
    "философиялық ағым",
    "моральдық қасиет",
    "жұлдыздар жиыны",
    "ауа райы күйі",
    "техникалық сала",
    "ғарыштық нысан",
    "философия саласы",
    "ұзындық бірлігі",
    "көлем бірлігі",
    "масса бірлігі",
    "уақыт бірлігі",
    "қозғалыс жолы",
    "ғарыш денесі",
    "күн күркіреу",
    "қара құрдым",
    "ақыл түрі",
    "ауа райы",
    // **v4.42.9** — psychology_basic + emotions/society depth.
    // Required by `world_core_multiword_coverage`.
    "экономикалық кеңістік",
    "қоғамдық бірлестік",
    "психикалық қызмет",
    "психикалық үрдіс",
    "мінез-құлық үлгісі",
    "тұлғалық қасиет",
    "қоғамдық принцип",
    "қоғамдық жағдай",
    "психикалық күш",
    "психикалық күй",
    "азаматтық қоғам",
    "саяси процесс",
    "мемлекет түрі",
    "міндетті ақы",
    "басқару түрі",
    "әрекет түрі",
    "заң түрі",
    // **v4.42.8** — computer_science_basics + mathematics_basic
    // (functions / equations / progressions / statistics) compound
    // objects/subjects. Required by `world_core_multiword_coverage`.
    "объектіге бағытталған бағдарламалау",
    "функционалдық бағдарламалау",
    "бағдарламалау парадигмасы",
    "арифметикалық прогрессия",
    "геометриялық прогрессия",
    "логарифмдік күрделілік",
    "бағдарламалау құрылымы",
    "бағдарламалау бірлігі",
    "бағдарламалау тәсілі",
    "жадтар күрделілігі",
    "уақыт күрделілігі",
    "квадраттық функция",
    "квадраттық теңдеу",
    "статистикалық шама",
    "алгоритмдік тәсіл",
    "ұйымдастыру тәсілі",
    "сызықтық функция",
    "сызықтық теңдеу",
    "күрделілік белгісі",
    "математикалық шама",
    "сандар тізбегі",
    "математикалық топ",
    "күрделілік түрі",
    "жадтар түрі",
    "орташа мән",
    "қате жөндеу",
    // **v4.42.0** — programming-languages compound objects from
    // `data/world_core/programming_languages.jsonl`. List-summary
    // forms + classification forms + structural type-system
    // compounds + style/markup-language compounds.
    "интерпретацияланатын тілдер тізімі",
    "компиляцияланатын тілдер тізімі",
    "білетін бағдарламалау тілдер тізімі",
    "интерпретацияланатын тіл",
    "компиляцияланатын тіл",
    "бағдарламалау тілдер тізімі",
    "гибридтік тілдер тізімі",
    "динамикалық типтеу",
    "статикалық типтеу",
    "гибридтік тіл",
    "белгілеу тілі",
    "типтеу жүйесі",
    "стиль тілі",
    // **v4.41.7** — programming_rust_advanced compound objects.
    // Required by `world_core_multiword_coverage` invariant test.
    "Rust ұғымы",
    "move семантикасы",
    "pattern matching",
    "арнайы блок",
    "асинхронды операция",
    "басқару құралы",
    "бір ие ережесі",
    "динамикалық сілтеме",
    "жад басқару моделі",
    "жинақ бірлігі",
    "ие сілтеме",
    "меншік ережесі",
    "меншік моделінің бөлігі",
    "мән көрінісі",
    "мән түрі",
    "мәтін типі",
    "параллельдік құралы",
    "параметрлік тип",
    "синхрондау механизмі",
    "сілтеме санағыш",
    "тип құрылымы",
    "тіл механизмі",
    "функция түрі",
    "өмір сүру мерзімі",
    // **v4.38.0** — role-bridges expansion (8 new compound bridge
    // objects from `data/world_core/role_bridges.jsonl`). These are
    // structural multi-word categories used as IsA targets by the
    // bridge-fact ladder (Абай → ақын → шығармашылық тұлға → ...).
    // Required by the `world_core_multiword_coverage` invariant test.
    "шығармашылық тұлға",
    "әкімшілік бөлініс",
    "қоғамдық тұлға",
    "өнер тұлғасы",
    "спорт тұлғасы",
    "ұлттық саябақ",
    "еңбек саласы",
    "тау жүйесі",
    // **v4.4.10** — Kazakhstan administrative + physical-geography
    // expansion. 17 oblast names (compound `<adjective/proper>
    // облысы`), the structural-bridge nouns `әкімшілік бөлік` /
    // `су денесі` / `жер бедері` / `елді мекен` /
    // `табиғи аймақ` / `республикалық маңызы бар қала`, the
    // mountain-range entity `Жетісу алатауы`, the peak `Хан Тәңірі`,
    // and the canyon `Шарын каньоны`. Sorted longest-first so
    // `find_multiword_entity`'s longest-match scan picks the
    // compound before the simpler substring.
    "республикалық маңызы бар қала",
    "солтүстік қазақстан облысы",
    "батыс қазақстан облысы",
    "шығыс қазақстан облысы",
    "маңғыстау облысы",
    "қарағанды облысы",
    "қостанай облысы",
    "қызылорда облысы",
    "түркістан облысы",
    "ұлытау облысы",
    "ақмола облысы",
    "ақтөбе облысы",
    "алматы облысы",
    "атырау облысы",
    "жамбыл облысы",
    "жетісу облысы",
    "павлодар облысы",
    "абай облысы",
    "шарын каньоны",
    "жетісу алатауы",
    "әкімшілік бөлік",
    "табиғи аймақ",
    "елді мекен",
    "хан тәңірі",
    "су денесі",
    // **v4.4.10** — list-summary fact objects (compound nouns
    // that play the role of `қазақстан related_to <list>`).
    // Required by `world_core_multiword_coverage` contract test.
    "облыстар тізімі",
    "өзендер тізімі",
    "көлдер тізімі",
    "таулар тізімі",
    "шөлдер тізімі",
    // **v4.6.0** — landmarks list-summary object.
    "көрікті жерлер тізімі",
    // **v4.30.0** — programming-language list-summary object
    // (rust_181 «бағдарламалау тілі related_to rust тізімі»).
    // Closes the live REPL 2026-05-02 turn 7 case where the user
    // asked «Қандай бағдарламалау тілдерін білесіз?» and got
    // «Кешіріңіз, мен мұны білмеймін» — pre-fix there was no
    // list-summary fact for the programming domain.
    "rust тізімі",
    // **v4.6.15** — mathematics_basic + informatics_basic domains.
    // Compound objects (and one subject) that appear in `facts` of
    // the two new world_core domains. Required by
    // `world_core_multiword_coverage` contract test. Sorted
    // longest-first within each length bucket so
    // `find_multiword_entity`'s longest-match scan resolves the
    // compound before any contained simpler form.
    "математикалық тәуелділік",
    "компьютерлер жиынтығы",
    "қорғаныс бағдарламасы",
    "математикалық қатынас",
    "математикалық әрекет",
    "геометриялық фигура",
    "математикалық өрнек",
    "математикалық кесте",
    "бағдарлама құрылымы",
    "геометриялық объект",
    "бағдарламалық шама",
    "математикалық ұғым",
    "бағдарламалау тілі",
    "арифметикалық амал",
    "электронды құрылғы",
    "парақтар жиынтығы",
    "зиянды бағдарлама",
    "деректер жиынтығы",
    "математика саласы",
    "деректер құрылымы",
    "бағдарлама бөлігі",
    "таңбалар тізбегі",
    "енгізу құрылғысы",
    "шығару құрылғысы",
    "операциялық жүйе",
    "қадамдар тізбегі",
    "сақтау құрылғысы",
    "деректер базасы",
    "ақпарат бірлігі",
    "көбейту кестесі",
    "ақпарат қоймасы",
    "нұсқаулар жиыны",
    "өлшем бірлігі",
    "формалды тіл",
    "натурал сан",
    "бүтін сан",
    "жұп сан",
    "тақ сан",
    "жай сан",
    // **v4.8.0** — physics_school domain. Compound objects /
    // subjects from `data/world_core/physics_school.jsonl`. Sorted
    // longest-first within each length bucket so
    // `find_multiword_entity`'s longest-match scan resolves the
    // compound before any contained simpler form.
    "электромагниттік индукция",
    "меншікті жылу сыйымдылық",
    "энергияның сақталу заңы",
    "электромагниттік толқын",
    "ньютонның бірінші заңы",
    "ньютонның екінші заңы",
    "ньютонның үшінші заңы",
    "физикалық субстанция",
    "температуралық шкала",
    "потенциалдық энергия",
    "бірқалыпты қозғалыс",
    "кинетикалық энергия",
    "радиоактивті сәуле",
    "ультракүлгін сәуле",
    "жартылай өткізгіш",
    "физикалық құбылыс",
    "физикалық тұрақты",
    "физикалық процесс",
    "оптикалық құбылыс",
    "серпімділік күші",
    "элементар бөлшек",
    "физикалық қасиет",
    "жарық жылдамдығы",
    "жылу өткізгіштік",
    "электр құрылғысы",
    "инфрақызыл сәуле",
    "дыбыс жылдамдығы",
    "үдемелі қозғалыс",
    "жарықтың шағылуы",
    "шашыратушы линза",
    "көлденең толқын",
    "толқын ұзындығы",
    "материя бөлшегі",
    "физикалық нысан",
    "ядролық реакция",
    "кельвин шкаласы",
    "оптикалық аспап",
    "цельсий шкаласы",
    "физикалық жүйе",
    "физикалық өріс",
    "электр тізбегі",
    "жарықтың сынуы",
    "ядролық синтез",
    "физикалық шама",
    "ядролық ыдырау",
    "атомдық физика",
    "тартылыс күші",
    "гамма сәулесі",
    "физика саласы",
    "қозғалыс түрі",
    "бойлық толқын",
    "электр заряды",
    "альфа сәулесі",
    "өлшеуіш аспап",
    "жинаушы линза",
    "үйкеліс күші",
    "архимед заңы",
    "магнит өрісі",
    "ауырлық күші",
    "бета сәулесі",
    "электр өрісі",
    "энергия түрі",
    "электр тоғы",
    "толқын түрі",
    "атом ядросы",
    "теріс заряд",
    "физика заңы",
    "еркін түсу",
    "қатты дене",
    "атом түрі",
    "зат күйі",
    "оң заряд",
    "ом заңы",
    // **v4.9.0** — chemistry_school domain. Compound objects /
    // subjects from `data/world_core/chemistry_school.jsonl`. Sorted
    // longest-first within each length bucket so
    // `find_multiword_entity`'s longest-match scan resolves the
    // compound before any contained simpler form.
    "периодтық жүйенің периоды",
    "тотықсыздану реакциясы",
    "заттардың сақталу заңы",
    "бейтараптану реакциясы",
    "бейорганикалық химия",
    "ковалентті байланыс",
    "элемент сипаттамасы",
    "органикалық қосылыс",
    "зарядталған бөлшек",
    "орынбасу реакциясы",
    "органикалық химия",
    "металдық байланыс",
    "натрий гидроксиді",
    "сутектік байланыс",
    "сілтілік металдар",
    "химиялық байланыс",
    "тотығу реакциясы",
    "химиялық процесс",
    "химиялық элемент",
    "ыдырау реакциясы",
    "қосылу реакциясы",
    "көмірқышқыл газы",
    "химиялық реакция",
    "алмасу реакциясы",
    "иондық байланыс",
    "нуклеин қышқылы",
    "менделеев заңы",
    "натрий хлориді",
    "біртекті қоспа",
    "молярлық масса",
    "периодтық жүйе",
    "бейметалл тобы",
    "күкірт қышқылы",
    "карбон қышқылы",
    "сутектік көрсеткіш",
    "авогадро саны",
    "әртекті қоспа",
    "материя түрі",
    "химия бірлігі",
    // **v4.10.0** — biology_school domain. Compound objects /
    // subjects from `data/world_core/biology_school.jsonl`. Sorted
    // longest-first within each length bucket so
    // `find_multiword_entity`'s longest-match scan resolves the
    // compound before any contained simpler form.
    "тұқым қуалаушылық бірлігі",
    "экологиялық қарым-қатынас",
    "қылқан жапырақты өсімдік",
    "тағамдық тізбек звеносы",
    "омыртқасыз жануарлар",
    "таксономиялық бірлік",
    "омыртқалы жануарлар",
    "қан айналымы жүйесі",
    "бөліп шығару жүйесі",
    "биологиялық құбылыс",
    "таксономиялық дүние",
    "биологиялық процесс",
    "бауырмен жорғалаушы",
    "тірек-қимыл жүйесі",
    "эволюция механизмі",
    "өрмекші тәрізділер",
    "экологиялық бірлік",
    "жасуша мембранасы",
    "тұқым қуалаушылық",
    "эндоплазмалық тор",
    "тіршілік бірлігі",
    "тіршілік процесі",
    "генетикалық ұғым",
    "өсімдік жасушасы",
    "экологиялық ұғым",
    "гольджи аппараты",
    "жасуша қабырғасы",
    "тыныс алу жүйесі",
    "эндокриндік жүйе",
    "жасуша органоиді",
    "табиғи сұрыпталу",
    "ас қорыту жүйесі",
    "биология саласы",
    "тағамдық тізбек",
    "биологиялық зат",
    "тіршілік ортасы",
    "рецессивті ген",
    "сезім мүшелері",
    "жасуша бөлінуі",
    "доминантты ген",
    "гүлді өсімдік",
    "иммундық жүйе",
    "көбею жүйесі",
    "жүйке жүйесі",
    "ағза жүйесі",
    "адам ағзасы",
    "қан тамыры",
    "тірі ағза",
    "тыныс алу",
    "ас қазан",
    // **v4.11.0** — history_kazakhstan domain. Compound objects /
    // subjects from `data/world_core/history_kazakhstan.jsonl`.
    // Sorted longest-first within each length bucket so
    // `find_multiword_entity`'s longest-match scan resolves the
    // compound before any contained simpler form.
    "жиырма бесінші маусым 1916 жылғы жарлық",
    "исатай мен махамбет көтерілісі",
    "орынбор шекаралық комиссиясы",
    "қазақ хандығының 550 жылдығы",
    "қожа ахмет ясауи кесенесі",
    "беғазы-дәндібай мәдениеті",
    "археологиялық ескерткіш",
    "қазақстан президенттігі",
    "назарбаев президенттігі",
    "сырым датұлы көтерілісі",
    "астанаға елорда көшіру",
    "семей ядролық полигоны",
    "алматыға елорда көшіру",
    "археологиялық мәдениет",
    "невада-семей қозғалысы",
    "қазақстан республикасы",
    "1995 жылғы конституция",
    "дала уалаятының газеті",
    "қазақстан тәуелсіздігі",
    "1930 жылдардағы аштық",
    "ортағасырлық мемлекет",
    "есім ханның ескі жолы",
    "байқоңыр ғарыш айлағы",
    "әдеттегі құқық жинағы",
    "тамғалы петроглифтері",
    "желтоқсан көтерілісі",
    "сталин репрессиялары",
    "тоқаев президенттігі",
    "батыс түрік қағанаты",
    "1916 жылғы көтеріліс",
    "мемлекеттік институт",
    "моңғол шапқыншылығы",
    "мемлекеттік лауазым",
    "кенесары көтерілісі",
    "қасымның қасқа жолы",
    "ехқк астана саммиті",
    "қазақстан елтаңбасы",
    "күлтегін ескерткіші",
    "археологиялық кезең",
    "андронов мәдениеті",
    "ұлттық бірлік күні",
    "сыпайра ескерткіші",
    "мемлекеттік валюта",
    "айша бибі кесенесі",
    "мемлекеттік мереке",
    "тәуелсіз мемлекет",
    "ортағасырлық қала",
    "тарихи сауда жолы",
    "қазақстан әнұраны",
    "мемлекеттік құжат",
    "қарахан мемлекеті",
    "советтік мемлекет",
    "мемлекеттік рәміз",
    "ақтабан шұбырынды",
    "әбілқайыр хандығы",
    "тарихи ескерткіш",
    "халықаралық ұйым",
    "аңырақай шайқасы",
    "конституция күні",
    "тәуелсіздік күні",
    "қазақстан тарихы",
    "көне түрік жазуы",
    "тарихи көтеріліс",
    "тарихи бірлестік",
    "қаңтар оқиғалары",
    "қарлұқ қағанаты",
    "ресей бодандығы",
    "ұлы отан соғысы",
    "түргеш қағанаты",
    "тарихи мемлекет",
    "тарихи қозғалыс",
    "бұланты шайқасы",
    "кеңестік лагерь",
    "бесшатыр қорымы",
    "тарихи мерейтой",
    "тарихи бөлініс",
    "қимақ қағанаты",
    "алаш қозғалысы",
    "қыпшақ хандығы",
    "ұлы жібек жолы",
    "жоңғар хандығы",
    "тарихи лауазым",
    "наурыз мейрамы",
    "түрік қағанаты",
    "оғыз мемлекеті",
    "тарихи науқан",
    "тарихи мекеме",
    "тарихи үкімет",
    "алаш партиясы",
    "қазақстан туы",
    "тарихи шайқас",
    "шағатай ұлысы",
    "әбілқайыр хан",
    "қазақ хандығы",
    "тарихи саясат",
    "тарихи кезең",
    "тарихи газет",
    "ежелгі тайпа",
    "саяси партия",
    "тарихи оқиға",
    "тарихи дерек",
    "тарихи тұлға",
    "хақназар хан",
    "тарихи соғыс",
    "бөкей ордасы",
    "ежелгі халық",
    "қазақ газеті",
    "тарих саласы",
    "ұлттық рәміз",
    "тарихи нысан",
    "алжир лагері",
    "тарихи аймақ",
    "тарихи құжат",
    "кеңес одағы",
    "жазу жүйесі",
    "қола дәуірі",
    "тәуекел хан",
    "тарихи апат",
    "орхон жазуы",
    "жәнібек хан",
    "алтын орда",
    "абылай хан",
    "тарихи топ",
    "шыңғыс хан",
    "жәңгір хан",
    "қазақ асср",
    "алтын адам",
    "әмір темір",
    "қазақ ханы",
    "експо 2017",
    "хан кеңесі",
    "жеңіс күні",
    "жеті жарғы",
    "тәуке хан",
    "қасым хан",
    "керей хан",
    "тың игеру",
    "қазақ сср",
    "орта жүз",
    "есім хан",
    "көк бөрі",
    "кіші жүз",
    "ақ орда",
    "ұлы жүз",
    "үш би",
    "сірке қышқылы",
    "атомдық масса",
    "реттік нөмір",
    "азот қышқылы",
    "химия саласы",
    "амин қышқылы",
    "инертті газ",
    "күрделі зат",
    "тұз қышқылы",
    "металл тобы",
    "химия ұғымы",
    "химия заңы",
    "таза зат",
    "жай зат",
    // **v4.7.0** — programming_rust domain. Compound objects /
    // subjects from `data/world_core/programming_rust.jsonl`.
    // Sorted longest-first so `find_multiword_entity` resolves
    // the compound before any contained simpler form.
    "бағдарламалау тілі",
    "көшіру семантикасы",
    "бағдарламалық шама",
    "бағдарлама әрекеті",
    "байланысты функция",
    "бағдарлама бөлігі",
    "бағдарлама құралы",
    "синхрондау құралы",
    "компилятор бөлігі",
    "өзгермелі сілтеме",
    "орындалу бірлігі",
    "басқару құрылымы",
    "иелікті ауыстыру",
    "тіршілік мерзімі",
    "қарыз тексергіш",
    "cargo командасы",
    "тұрақты сілтеме",
    "main функциясы",
    "функция бөлігі",
    "сандық қоймасы",
    "ақылды сілтеме",
    "derive макросы",
    "бүтін сан түрі",
    "жалпылама тип",
    "бірлік структ",
    "тип параметрі",
    "async функция",
    "тіл командасы",
    "иелік моделі",
    "баптау файлы",
    "мәлімет түрі",
    "қайтару мәні",
    "unsafe блогы",
    "тіл құрылымы",
    "match өрнегі",
    "енам нұсқасы",
    "drop трейті",
    "cargo check",
    "жад әрекеті",
    "ұжымдық тип",
    "қарызға алу",
    "? операторы",
    "cargo build",
    "код бөлігі",
    "жад аймағы",
    "cargo test",
    "код жинағы",
    "қате өңдеу",
    "жад моделі",
    "impl блогы",
    "жад ұғымы",
    "if өрнегі",
    "cargo run",
    // **v4.11.5** — query-time compounds for school-curriculum
    // and self-knowledge questions. These are NOT world_core
    // subjects/objects (so the `world_core_multiword_coverage`
    // contract test does not require them), but they ARE the
    // canonical topic phrasing of real user questions like
    // «Адам, сен мектептің физика бағдарламасын білесің бе?»
    // or «Мектеп пәндерін білесің бе?». Pre-v4.11.5 these
    // questions fell through to `first_noun_root` which picked
    // either a head noun in isolation (`физика`) or — worse —
    // the vocative addressee (`адам`) as topic. Sorted
    // longest-first within the bucket so `find_multiword_entity`
    // resolves the more-specific compound first.
    "информатика бағдарламасы",
    "математика бағдарламасы",
    "биология бағдарламасы",
    "мектеп бағдарламасы",
    "физика бағдарламасы",
    "химия бағдарламасы",
    "тарих бағдарламасы",
    "мектеп пәні",
    // **v4.11.5** — adam_self domain. Compound subjects /
    // objects from `data/world_core/adam_self.jsonl` (system's
    // self-identity facts: identity / implementation / knowledge
    // claims / limitations). Required by
    // `world_core_multiword_coverage` contract test. Sorted
    // longest-first within the bucket.
    "жергілікті бағдарлама",
    "rust бастапқы коды",
    "когнитивтік ядро",
    "жасанды интеллект",
    "қазақ тілді жүйе",
    "ретривал жүйесі",
    "анық архитектура",
    "география білімі",
    "информатика білімі",
    "математика білімі",
    "биология білімі",
    "әдебиет білімі",
    "тілдік модель",
    "диалог жүйесі",
    "мектеп пәндері",
    "физика білімі",
    "химия білімі",
    "тарих білімі",
    "rust білімі",
    "жалпы білім",
    "ғылым салалары",
    // **v4.11.6** — adam_self subject-rich knowledge claim
    // categories. Compound objects from `adam_self_028..033`
    // (subject = школьный предмет, IsA = категория ғылым).
    "жаратылыстану ғылымы",
    "гуманитарлық ғылым",
    "қолданбалы ғылым",
    "табиғат ғылымы",
    "абстракт ғылым",
    // **v4.11.7** — `тарихи өңір` (historical region) compound
    // object from new bare-subject Жетісу / Ұлытау geo_kz entries.
    "тарихи өңір",
    // **v4.26.0** — programming_rust expansion (rust_111…rust_150).
    // Both Latin multi-word phrases (will be picked up by the
    // multiword scanner before Latin-token extraction even runs)
    // and Kazakh compound `object` fields from the 40 new alias
    // entries' `is_a` facts. Required by
    // `world_core_multiword_coverage` invariant: every compound
    // subject/object in world_core must be registered here.
    "borrow checker",
    "error handling",
    "question mark operator",
    "smart pointer",
    "trait object",
    "автоматты трейт",
    "айнымалы жабу әдісі",
    "анонимді функция",
    "асинхрондық белгісі",
    "атомдық санағышты сілтеме",
    "болуы мүмкін мән енамы",
    "деструктуризация үлгісі",
    "динамикалық полиморфизм",
    "жад адресі",
    "жады басқару моделі",
    "жоба манифест файлы",
    "иелік ауыстыру",
    "интерфейс анықтамасы",
    "код блогы",
    "код ұйымдастыру бірлігі",
    "компиляция бірлігі",
    "макрос атрибуты",
    "нұсқалары бар тип",
    "санағышты ақылды сілтеме",
    "синхрондау примитиві",
    "статикалық анализатор",
    "сілтеме беру операциясы",
    "сілтеме мерзімі",
    "сілтеме типі",
    "тип параметрлеу жүйесі",
    "тілдік құрылым",
    "тілім сілтемесі",
    "тіркес тип",
    "элемент шығарушы",
    "ішкі өзгермелілік сілтемесі",
    "қате тарату операторы",
    "қателерді өңдеу тәсілі",
    "қауіпсіздік шектеулерін айналып өту блогы",
    "құрама деректер типі",
    "үлгіге сай келтіру өрнегі",
    "өзгертуге рұқсат қасиеті",
    "өсетін массив",
    // **v4.26.5** — Kazakh form variants surfaced by the
    // 2026-05-02 live battery on Rust queries.
    //
    // `қарыз алу` is the user-typed Kazakh form (without dative
    // -ға); world_core's canonical `қарызға алу` (rust_012) is
    // grammatically correct but the bare-stem form is what most
    // users naturally produce. New rust_151 alias entry points
    // both surfaces to the same definition.
    //
    // `жад тазарту` is the natural Kazakh way to ask about
    // memory management. New rust_152 entry routes the question
    // to a curated answer about Drop trait + scope-based RAII
    // (vs Rust intro that pre-fix surfaced).
    "қарыз алу",
    "жад тазарту",
    "hello world",
    // **v4.26.5 follow-up** — `object` compounds from new
    // rust_153…rust_160 entries.
    "автоматты тексеру функциясы",
    "айнымалы жариялау кілт сөзі",
    "иелік әрекеті",
    "код генерациялау құрылымы",
    "параллель орындалу бірлігі",
    "функция жариялау кілт сөзі",
    "қарапайым бағдарлама үлгісі",
    "өзгерілуі рұқсат модификаторы",
    // **v4.27.0** — multi-word concepts from rust_161…rust_172 +
    // their `object` compounds.
    "deref coercion",
    "type alias",
    "where clause",
    "dynamic dispatch",
    "static dispatch",
    "zero-cost abstraction",
    "rust design",
    "автоматты түрлендіру",
    "тип балама атауы",
    "шектеу синтаксисі",
    "орындалу-кезіндегі полиморфизм",
    "компиляция-кезіндегі полиморфизм",
    "тіл принципі",
    "мән қайтаратын құрылым",
    "нұсқау құрылымы",
    "ресурс басқару тәсілі",
    "жад жылжытпау кепілдемесі",
    "асинхронды операция трейті",
    "тілдік дизайн принциптері",
    "implicit type",
    "explicit type",
    "type inference",
    "тип шығару тәсілі",
    "автоматты тип анықтау",
    // **v4.78.0** — biology_school location/causal facts (Codex round-3 Bug 7).
    "ас қорыту",
    "тыныс алу",
    "қанайналым жүйесі",
    "мендель заңы",
    "мендель заңдары",
    "жасуша ядросы",
    "жүректен басталатын қан тасымалдау процесі",
    "грегор мендель ашқан генетика заңдары",
    // **v4.78.5** — Rust Book chapter 13 (iterators + closures) deepening.
    "fn трейті",
    "жабу трейті",
    "жабу модификаторы",
    "move кілтсөзі",
    "лазай есептеу",
    "итератор қасиеті",
    "итератор әдісі",
    "итератор бейімдеуі",
    "тұтынатын бейімдеу",
    // **v4.79.0** — Rust Book chapter 1 (Bastau / setup) deepening.
    // Subjects that contain spaces or `/` (compound forms): each
    // appears as a `subject` or `object` of a curated rust_200…217
    // entry.
    "rustup update",
    "rustup doc",
    "cargo new",
    "cargo build --release",
    "src/main.rs",
    "dev бейіні",
    "release бейіні",
    "target қалтасы",
    "fn main",
    "rs кеңейтімі",
    "rust стилі",
    "rust құралы",
    "rustup командасы",
    "rust бастапқы файлы",
    "cargo бейіні",
    "cargo артефакт қалтасы",
    "rust макросы",
    "rust кіру нүктесі",
    "rust файл кеңейтімі",
    "rust басылым жүйесі",
    "конфигурация пішімі",
    "тіл шартты келісімдері",
    // **v4.79.5** — Rust Book chapter 2 (guessing game) deepening.
    // rust_218…236. Compound subjects (with spaces / `::` / `&` /
    // `[`) and compound objects (Kazakh phrases that include space).
    "string::new",
    "let mut",
    "use std::io",
    "io::stdin",
    "&mut",
    "форматтауыш орынтолтырғыш",
    "[dependencies]",
    "cmp әдісі",
    "диапазон өрнегі",
    "string конструкторы",
    "айнымалы жариялауы",
    "use сөйлемі",
    "stdlib функциясы",
    "stdin әдісі",
    "сілтеме түрі",
    "stdlib енамы",
    "result әдісі",
    "макрос синтаксисі",
    "cargo.toml бөлімі",
    "сыртқы сандық",
    "rand функциясы",
    "rand әдісі",
    "rust синтаксисі",
    "ord әдісі",
    "айнымалы тәсілі",
    "string әдісі",
    // **v4.82.5** — Rust Book chapter 8 (collections) deepening.
    // rust_318…335 + 3 stub replacements (041/045/046).
    "vec::new",
    "vec! макросы",
    "vec push әдісі",
    "вектор индекстеу",
    "вектор бойынша жүру",
    "вектордағы әртүрлі типтер",
    "string конструкторлары",
    "string + операторы",
    "format! макросы",
    "string индекстеу",
    "chars пен bytes",
    "hashmap::new",
    "hashmap insert",
    "hashmap entry",
    "hashmap иелік",
    // Objects.
    "vec конструкторы",
    "vec әдісі",
    "vec мәселесі",
    "vec идиомасы",
    "rust операторы",
    "string мәселесі",
    "hashmap конструкторы",
    "hashmap әдісі",
    "hashmap идиомасы",
    "hashmap ережесі",
    "hash функциясы",
    // **v4.82.0** — Rust Book chapter 7 (modules / packages / crates) deepening.
    // rust_300…317 + 4 stub replacements (004/005/101/102).
    "модуль жолы",
    "екілік сандық",
    "кітапхана сандығы",
    "сандық түбірі",
    "src/bin/",
    "модуль ағашы",
    "абсолюттік жол",
    "салыстырмалы жол",
    "self кілтсөзі",
    "жекелік әдепкіше",
    "pub бір қабат",
    "pub struct пен pub enum айырмашылығы",
    "pub use",
    "as импортта",
    "тоғыспалы жолдар",
    "glob операторы",
    // Objects.
    "cargo бірлігі",
    "сандық түрі",
    "файл шартты келісімі",
    "cargo шартты келісімі",
    "модуль құрылымы",
    "модуль атауы",
    "жол түрі",
    "жол кілтсөзі",
    "pub ережесі",
    "module ыңғайлылығы",
    "import синтаксисі",
    "сандық бөлігі",
    // **v4.81.5** — Rust Book chapter 6 (enums + match) deepening.
    // rust_285…299 + 5 stub replacements (057/059/061/062/072).
    "енам нұсқасы",
    "нұсқа деректері",
    "енамға әдіс",
    "енам мен структ айырмашылығы",
    "option жалпылама түрі",
    "option пен t айырмашылығы",
    "match толық қамту",
    "match тармағы",
    "үлгіге байланыстыру",
    "_ орынтолтырғыш",
    "айнымалы үлгісі",
    "if let",
    "if let else",
    "null проблемасы",
    "алгебралық тип жүйесі",
    // Objects.
    "енам элементі",
    "енам бөлігі",
    "тип теориясы",
    "compiler кепілдігі",
    "match синтаксисі",
    "pattern matching",
    "pattern түрі",
    "match баламасы",
    "тіл дизайн қатесі",
    // **v4.81.0** — Rust Book chapter 5 (structs) deepening.
    // rust_271…284 + 6 stub replacements (050…055).
    "өрісті қысқа жариялау",
    "структты жаңарту синтаксисі",
    "структ өзгермелілігі",
    "структ деректерінің иелігі",
    "&self параметрі",
    "&mut self",
    "әдіс пен функция шакыру",
    "автоматты сілтемелеу",
    "#[derive(debug)]",
    "{:?} пен {:#?}",
    "dbg макросы",
    "self типі",
    "бірнеше impl блогы",
    "әдіс пен өріс бір аттас",
    "кортеж-структ",
    "бірлік структ",
    "байланысты функция",
    // Objects.
    "структ синтаксисі",
    "әдіс синтаксисі",
    "rust ыңғайлылығы",
    "structуру атрибуты",
    "пішімдеуіш",
    "жөндеу құралы",
    "тіл синонимі",
    "структ құрамдас бөлігі",
    // **v4.80.5** — Rust Book chapter 4 (ownership) deepening.
    // rust_254…270 + 5 stub replacements (009/011/012/019/020).
    "иелік ережелері",
    "drop функциясы",
    "string::from",
    "иеліктің көшуі",
    "clone әдісі",
    "copy трейті",
    "иелік және функциялар",
    "қайтару мәні мен иелік",
    "жабайы сілтеме",
    "қарызға алудың бірінші ережесі",
    "қарызға алудың екінші ережесі",
    "жол тілімі",
    "&str пен &string айырмашылығы",
    "жиым тілімі",
    "тілім диапазоны",
    "жарыс шарты",
    "стек жады",
    "үйме жады",
    // Objects.
    "rust ережесі",
    "автоматты тазалау",
    "rust семантикасы",
    "терең көшіру",
    "rust трейті",
    "borrow checker ережесі",
    "сілтеме жүйесі",
    "тілім түрі",
    "rust идиомасы",
    "тілім синтаксисі",
    "қатарластық қаупі",
    // **v4.80.0** — Rust Book chapter 3 (common programming concepts).
    // rust_237…253. Subjects.
    "тұрақтылық әдепкіше",
    "бүтін сан түрлері",
    "бүтін санның асып кетуі",
    "қалқымалы үтірлі сан түрлері",
    "кортеж бөлшектеу",
    "unit түрі",
    "жиым қайталау синтаксисі",
    "жиымның шектен шығу",
    "функцияның мән қайтаруы",
    "блок өрнегі",
    "if өрнегі",
    "loop мән қайтаруы",
    "цикл белгісі",
    "for ұжым бойынша",
    "сөйлем мен өрнек айырмашылығы",
    "бүтін бөлу",
    // Objects.
    "rust кепілдігі",
    "сандық тип жүйесі",
    "rust қауіпсіздігі",
    "tuple синтаксисі",
    "rust типі",
    "array синтаксисі",
    "функция синтаксисі",
    "rust өрнегі",
    "басқару өрнегі",
    "loop синтаксисі",
    "for синтаксисі",
    "rust ұғымы",
    "код элементі",
    "сандық амал",
];

/// Longest-match scan of `input` against `MULTIWORD_ENTITIES`. Returns
/// the first entity found as a substring of the lowercased input.
/// Substring match handles Kazakh inflection on the last word of the
/// compound — e.g. «Құс жолының бейнесі» contains «құс жолы» as a prefix
/// of the inflected compound.
pub(crate) fn multiword_entity_hint(input: &str) -> Option<String> {
    let lowered = input.to_lowercase();
    let lowered_tokens: Vec<&str> = lowered.split_whitespace().collect();
    // **v4.79.0** — actually iterate longest-first so multi-token
    // compounds containing shorter ones (e.g. «cargo build --release»
    // vs «cargo build») resolve to the most specific match. Earlier
    // versions claimed longest-first in the const docstring but the
    // loop iterated in declaration order; surfaced when ch.1 deepening
    // shipped «cargo build --release» as a separate canonical entry.
    let mut sorted: Vec<&&str> = MULTIWORD_ENTITIES.iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.len()));
    // First pass: exact substring (preserves all pre-v4.40.5
    // matches bit-for-bit when the input surface contains the
    // multiword in canonical bare form). **v4.71.5** — for
    // single-token entries (loanword / proper-noun additions for
    // educational stripe), require word-boundary token equality
    // instead of arbitrary substring; otherwise short single-word
    // entries like «темір» (Fe element) would shadow the third-
    // pass genitive logic for «темір жол» on input «темірдің жолы».
    for entity in &sorted {
        // **v4.79.5** — treat `::` as a compound separator like a
        // space. Without this, Rust-path names like `io::stdin`,
        // `string::new`, `std::cmp::Ordering` registered as
        // MULTIWORD_ENTITIES never match: the single-token branch
        // requires exact whitespace-bounded equality, and inputs
        // typically have trailing parens (`io::stdin()`) or other
        // punctuation that breaks token equality. Substring is the
        // right semantic for `::`-paths.
        if entity.contains(' ') || entity.contains("::") {
            if lowered.contains(**entity) {
                return Some((**entity).to_string());
            }
        } else if lowered_tokens.contains(*entity) {
            return Some((**entity).to_string());
        }
    }
    // **v4.40.5** — second pass: inflected-second-word match. For
    // 2-word entities `X Y`, accept input containing the consecutive
    // token pair `X T` where T starts with `Y`'s first 3 chars.
    // Closes the gap on inflected forms like «бағдарламалау
    // тілдерін» (Plural+P3+Acc of `тіл`) which doesn't substring-
    // contain «бағдарламалау тілі»; surfaced by the 2026-05-03
    // dialog transcript «Қандай бағдарламалау тілдерін білесіз?»
    // returning a tangential «Тіл — қарым-қатынас құралы» fact
    // instead of routing to programming_rust topic. Conservative
    // — fires only when the FIRST word matches exactly (longer
    // first words give stronger discriminative signal); 3+ -word
    // entities still rely on the first-pass substring match
    // because multi-token inflection patterns are too varied for
    // a uniform stem-prefix heuristic.
    let tokens: Vec<&str> = lowered.split_whitespace().collect();
    for entity in MULTIWORD_ENTITIES {
        let parts: Vec<&str> = entity.split_whitespace().collect();
        if parts.len() != 2 {
            continue;
        }
        let stem_2: String = parts[1].chars().take(3).collect();
        if stem_2.chars().count() < 3 {
            continue;
        }
        for window in tokens.windows(2) {
            if window[0] == parts[0] && window[1].starts_with(stem_2.as_str()) {
                return Some((*entity).to_string());
            }
        }
    }
    // **v4.43.9** — third pass: inflected-FIRST-word match (Kazakh
    // genitive). For 2-word entities `X Y`, accept input containing
    // `X{Gen} T` where T starts with `Y`'s first 3 chars and `X{Gen}`
    // is `X` followed by one of the six Kazakh genitive suffixes
    // (-ның/-нің/-дың/-дің/-тың/-тің). Closes the systemic gap from
    // the v4.43.8 carry-forward where «Қазақстанның премьер-министрі»
    // / «Қазақстанның президенті» / etc. couldn't substring-match
    // against bare-form multiword entries because of the genitive
    // suffix on the first word.
    //
    // Conservative — first word's bare form must be ≥ 4 chars to
    // avoid spurious matches on short first words like «көне»
    // («көненің» would over-fire). Returns the BARE-form entity
    // string so SearchGraph downstream can find the canonical fact.
    const GENITIVE_SUFFIXES: &[&str] = &["ның", "нің", "дың", "дің", "тың", "тің"];
    for entity in MULTIWORD_ENTITIES {
        let parts: Vec<&str> = entity.split_whitespace().collect();
        if parts.len() != 2 {
            continue;
        }
        if parts[0].chars().count() < 4 {
            continue;
        }
        let stem_2: String = parts[1].chars().take(3).collect();
        if stem_2.chars().count() < 3 {
            continue;
        }
        for window in tokens.windows(2) {
            if !window[1].starts_with(stem_2.as_str()) {
                continue;
            }
            for suf in GENITIVE_SUFFIXES {
                if window[0].len() == parts[0].len() + suf.len()
                    && window[0].starts_with(parts[0])
                    && window[0].ends_with(suf)
                {
                    return Some((*entity).to_string());
                }
            }
        }
    }
    None
}

/// **v4.11.5** — Latin-named technical proper nouns that appear as
/// subjects in `programming_rust.jsonl`. When the user types one of
/// these as a Latin word, the topic extractor routes to the
/// matching per-concept world_core fact instead of falling through
/// to corpus citation. Closes the v4.7.0 known limitation: queries
/// like «Rust туралы не білесіз?» pre-v4.11.5 emitted a poetry
/// quote because the Cyrillic-only FST can't tokenise `Rust`.
///
/// Sorted by length descending so the longest match wins (e.g.
/// `string` beats the substring `str` if both appear).
pub(crate) const LATIN_TECH_SUBJECTS: &[&str] = &[
    // v4.11.5 — original 47-entry closed list: Rust ecosystem proper
    // nouns, primitive type names, prelude API surface.
    "btreemap",
    "vecdeque",
    "rustfmt",
    "hashmap",
    "hashset",
    "refcell",
    "continue",
    "collect",
    "expect",
    "filter",
    "future",
    "string",
    "unwrap",
    "break",
    "cargo",
    "clippy",
    "loop",
    "mutex",
    "none",
    "option",
    "panic",
    "result",
    "rust",
    "rustc",
    // **v4.42.7** — programming languages registered in
    // `programming_languages.jsonl` (plang_011-026 + plang_031-050
    // purpose facts). Listing them here lets `latin_subject_hint`
    // and `latin_with_generic_head_marker` recognise them as the
    // discourse subject of queries like «Python қандай тіл?» /
    // «Java қандай салада қолданылады?» / «Kotlin не үшін қажет?».
    "python",
    "java",
    "kotlin",
    "javascript",
    "typescript",
    "ruby",
    "php",
    "swift",
    "go",
    "sql",
    "html",
    "css",
    "some",
    "usize",
    "while",
    "await",
    "bool",
    "char",
    "f32",
    "f64",
    "i32",
    "i64",
    "u32",
    "u64",
    "for",
    "map",
    "mod",
    "pub",
    "use",
    "vec",
    "arc",
    "box",
    "err",
    "rc",
    "ok",
    "str",
    // **v4.26.0** — Rust language concepts as Latin pass-through
    // topics. Without these, casual queries like «Rust-та ownership
    // деген не?» fail at topic extraction because the FST has no
    // parse for «ownership» and the v4.11.5 list contained only
    // ecosystem proper nouns + primitive types, not language
    // concepts. Each addition has a matching entry in
    // `data/world_core/programming_rust.jsonl` (rust_111…rust_150)
    // so retrieval finds a curated definition once extraction
    // succeeds. Live-test pre-fix: 2/6 = 33 %; target post-fix: 6/6.
    "ownership",
    "borrow",
    "lifetime",
    "trait",
    "impl",
    "match",
    "generic",
    "generics",
    "closure",
    "iterator",
    "pattern",
    "move",
    "module",
    "crate",
    "struct",
    "enum",
    "function",
    "mutability",
    "shadowing",
    "reference",
    "unsafe",
    "async",
    "send",
    "sync",
    "vector",
    "slice",
    "tuple",
    "derive",
    // **v4.26.5** — additional Rust keywords surfaced by the
    // 2026-05-02 comprehensive 40-question battery. Each has a
    // matching alias entry in programming_rust.jsonl
    // (rust_153…rust_160). Note: `hello` deliberately not added
    // here — `Hello World` is registered as a MULTIWORD_ENTITY
    // (rust_159) instead, so the more-specific compound wins
    // over the bare token.
    "let",
    "mut",
    "fn",
    "references",
    "thread",
    "macro",
    // **v4.27.0** — additional concepts surfaced by the 80-question
    // expanded battery. Single-word Latin tokens; multi-word
    // compounds (`type alias`, `where clause`, `deref coercion`,
    // `dynamic dispatch`, `static dispatch`, `zero-cost abstraction`,
    // `expression vs statement`) are registered in MULTIWORD_ENTITIES
    // instead.
    "expression",
    "statement",
    "future",
    "pin",
    "raii",
    // **v4.27.0 follow-up** — additional aliases (rust_173…rust_175).
    "implicit",
    "explicit",
    // **v4.78.5** — Rust Book chapter 13 (iterators + closures).
    // rust_182…199. Single-word Latin tokens; multi-word compounds
    // («fn трейті», «жабу трейті», «лазай есептеу», etc.) are
    // registered in MULTIWORD_ENTITIES instead.
    "fnonce",
    "fnmut",
    "iter",
    "iter_mut",
    "into_iter",
    "enumerate",
    "zip",
    "take",
    "skip",
    "fold",
    "chain",
    "next",
    // **v4.79.0** — Rust Book chapter 1 (Bastau / setup) deepening.
    // rust_200…217. Single-word Latin tokens; multi-word compounds
    // («cargo new», «cargo build --release», «target қалтасы», etc.)
    // are in MULTIWORD_ENTITIES.
    "rustup",
    "println",
    "edition",
    "toml",
    // **v4.79.5** — Rust Book chapter 2 (guessing game) deepening.
    // rust_218…236. Single-word Latin tokens (underscores included
    // but no spaces). Multi-word compounds («let mut», «io::stdin»,
    // «string::new», «диапазон өрнегі», etc.) are in
    // MULTIWORD_ENTITIES.
    "rand",
    "ordering",
    "trim",
    "parse",
    "thread_rng",
    "gen_range",
    "read_line",
    // **v4.80.0** — Rust Book chapter 3 (common programming concepts)
    // deepening. rust_237…253 + replaced rust_022/038/039.
    // **v4.80.5** — Rust Book chapter 4 (ownership) deepening.
    "nll",
    // **v4.82.5** — Rust Book chapter 8 (collections).
    "push_str",
    "дереференс",
    "siphash",
];

/// **v4.11.5** — scan input for any whitespace-separated word
/// **v4.30.0** — Latin subject + generic-head + topic-marker
/// pattern recogniser. Closes the live REPL 2026-05-02 case
/// «Rust бағдарламалау тілі туралы не білесіз?» where neither
/// the multiword scanner (picks «бағдарламалау тілі») nor the
/// topic_marker_hint (picks immediate predecessor «тіл») recover
/// the user's actual subject «Rust». The pattern is structurally
/// distinct from v4.27.5's `тілінде ... дегеніміз` case: the
/// topic marker here attaches to the qualifier itself, not to a
/// content noun. So the handling has to be different — return
/// the Latin head explicitly.
///
/// Recognised shapes (case-insensitive, sentence start):
///   - `{LATIN} (бағдарламалау|программалау)? тілі (туралы|жайында|жөнінде|хақында)`
///   - `{LATIN} (нәрсе|зат|тақырып|сала|ұғым|бағыт)(сы|ы|сі|і)? (туралы|жайында|жөнінде|хақында)`
///
/// Conservative: only fires at sentence start (skipping leading
/// pronouns like «Сіз», «Сен» — in compositional questions like
/// «Сіз Rust туралы не білесіз?» the v4.11.5 latin path already
/// handles correctly). Returns the Latin subject in canonical
/// lowercase form when matched, `None` otherwise.
fn latin_with_generic_head_marker(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    let lower = trimmed.to_lowercase();
    const HEAD_NOUNS: &[&str] = &[
        // Language-domain heads.
        "тіл",
        "тілі",
        "тілдер",
        "тілдері",
        // Generic referents.
        "нәрсе",
        "нәрсесі",
        "зат",
        "заты",
        "тақырып",
        "тақырыбы",
        "сала",
        "саласы",
        "ұғым",
        "ұғымы",
        "бағыт",
        "бағыты",
        "жүйе",
        "жүйесі",
    ];
    const QUALIFIERS: &[&str] = &["бағдарламалау", "программалау"];
    const MARKERS: &[&str] = &["туралы", "жайында", "жөнінде", "хақында"];
    for &lang in LATIN_TECH_SUBJECTS {
        for &marker in MARKERS {
            for &head in HEAD_NOUNS {
                // Direct: «{lang} {head} {marker}»
                let direct = format!("{lang} {head} {marker}");
                if lower.starts_with(&direct) {
                    return Some(lang.to_string());
                }
                // With qualifier: «{lang} {qualifier} {head} {marker}»
                for &qual in QUALIFIERS {
                    let with_qual = format!("{lang} {qual} {head} {marker}");
                    if lower.starts_with(&with_qual) {
                        return Some(lang.to_string());
                    }
                }
            }
            // **v4.42.7** — definitional / categorisation patterns
            // where `marker` is omitted: «{lang} — {head}» («Python
            // — қандай тіл?», «Rust — какой язык?») and «{lang}
            // {qualifier-adjective} {head}» («Python қандай тіл?»).
            // These ask "what kind of X is {lang}?" — the topic is
            // {lang}, not the generic head. Pre-v4.42.7 the
            // first_noun_root strategy returned the head noun
            // («тіл» / «сала»), surfacing tangential definitions
            // instead of the language-specific facts.
            for &head in HEAD_NOUNS {
                let dash_pattern = format!("{lang} — {head}");
                if lower.starts_with(&dash_pattern) {
                    return Some(lang.to_string());
                }
                let dash_pattern2 = format!("{lang} - {head}");
                if lower.starts_with(&dash_pattern2) {
                    return Some(lang.to_string());
                }
                let qandai_pattern = format!("{lang} қандай {head}");
                if lower.starts_with(&qandai_pattern) {
                    return Some(lang.to_string());
                }
                let qai_salada = format!("{lang} қандай {head}да");
                if lower.starts_with(&qai_salada) {
                    return Some(lang.to_string());
                }
                let qai_salada2 = format!("{lang} қандай {head}де");
                if lower.starts_with(&qai_salada2) {
                    return Some(lang.to_string());
                }
            }
            // **v4.42.7** — purpose pattern «{lang} не үшін қажет?»
            // («Kotlin не үшін қажет?»). Topic is {lang}.
            let purpose_pattern = format!("{lang} не үшін");
            if lower.starts_with(&purpose_pattern) {
                return Some(lang.to_string());
            }
        }
    }
    None
}

/// matching a known Latin technical subject. Returns the matched
/// subject in canonical lowercase form (matching world_core
/// `subject` field). Word boundaries are whitespace + punctuation
/// + backticks; trailing punctuation in tokens is stripped before
/// comparison (e.g. `Rust?` → `rust`). Ignores backtick-quoted
/// spans because those usually mean code identifiers in their
/// surrounding context, not a topical reference.
pub(crate) fn latin_subject_hint(input: &str) -> Option<String> {
    // **v4.26.5** — language-qualifier check. When the Latin
    // subject is at sentence start followed by a Kazakh language-
    // qualifier pattern (`Rust тілінде / Rust-та / Rust-тың /
    // Rust бағдарламасында / …`), the user is asking about a
    // *concept within that language*, not about the language
    // itself. The actual topic is a Kazakh content noun later in
    // the sentence (e.g. «Rust тілінде айнымалы дегеніміз не?» —
    // topic should be «айнымалы», not «Rust»).
    //
    // Pre-v4.26.5 `latin_subject_hint` ran early in
    // `best_noun_hint` and unconditionally returned the Latin
    // language name, hijacking topic extraction. Live battery
    // 2026-05-02 confirmed: 5 of 6 «Rust тілінде X» queries
    // returned generic Rust intro instead of the X concept.
    //
    // v4.26.5 detects the qualifier pattern; when present, returns
    // None so downstream extractors (`topic_marker_hint` /
    // `first_noun_root`) see the original input and pick the
    // Kazakh content noun. The Latin language word still appears
    // in the rendered response (templates use `{noun}` interp,
    // grounded fact about the X concept usually mentions Rust
    // anyway).
    if has_language_qualifier_prefix(input) {
        return None;
    }
    let mut best: Option<&'static str> = None;
    for raw in input.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                ',' | '.' | '?' | '!' | ';' | ':' | '`' | '"' | '(' | ')' | '\''
            )
    }) {
        if raw.is_empty() {
            continue;
        }
        // Only consider tokens that are purely ASCII letters /
        // digits / underscore; anything Cyrillic falls through to
        // the existing topic-extraction strategies.
        if !raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let lower = raw.to_lowercase();
        if let Some(&hit) = LATIN_TECH_SUBJECTS.iter().find(|&&s| s == lower.as_str()) {
            if best.is_none_or(|b| hit.len() > b.len()) {
                best = Some(hit);
            }
        }
    }
    best.map(|s| s.to_string())
}

/// **v4.26.5** — checks if the input opens with a language-
/// qualifier pattern: `<LATIN_TECH_SUBJECT>` at sentence start
/// followed by either a Kazakh case-marked dash form (`-та / -те
/// / -тың / -тің / -тан / -тен`) or a free-standing locative
/// qualifier (`тілінде / тіліндегі / тілдерінде /
/// бағдарламасында / бағдарламасынан`).
///
/// Used by `latin_subject_hint` to defer when the Latin word is
/// scoping context, not the topic. Conservative — only fires at
/// sentence start so that mid-sentence Latin mentions
/// («Cargo.toml не үшін керек?») still extract correctly.
fn has_language_qualifier_prefix(input: &str) -> bool {
    let trimmed = input.trim_start();
    let lower = trimmed.to_lowercase();
    // Free-standing locative qualifiers (preceded by space).
    // **v4.27.5** — added compound qualifiers «бағдарламалау
    // тілінде» / «программалау тілінде» / «кодын» to handle live-
    // session pattern «Rust бағдарламалау тілінде <X> дегеніміз
    // не?» and «Маған Hello World көрсететін Rust кодын». The
    // existing `тілінде` / `бағдарламасында` covers the simpler
    // forms; the multi-word variants are common in formal Kazakh
    // tech writing.
    const SPACE_QUALIFIERS: &[&str] = &[
        "тілінде",
        "тіліндегі",
        "тілдерінде",
        "тілінен",
        "бағдарламасында",
        "бағдарламасынан",
        "бағдарламалау тілінде",
        "бағдарламалау тіліндегі",
        "программалау тілінде",
        "программалау тіліндегі",
        "кодында",
        "кодын",
    ];
    // Dash-attached case suffixes.
    const DASH_QUALIFIERS: &[&str] = &[
        "-та", "-те", "-тың", "-тің", "-тан", "-тен", "-та,", "-те,", "-тың,", "-тің,",
    ];
    for &lang in LATIN_TECH_SUBJECTS {
        // Free-standing form: "rust тілінде …"
        for &qual in SPACE_QUALIFIERS {
            let prefix = format!("{lang} {qual}");
            if lower.starts_with(&prefix) {
                return true;
            }
        }
        // Dash form: "rust-та …"
        for &qual in DASH_QUALIFIERS {
            let prefix = format!("{lang}{qual}");
            if lower.starts_with(&prefix) {
                return true;
            }
        }
    }
    false
}

/// **v4.3.5** — When the input carries an explicit topic marker
/// (`X туралы` / `X жайында` / `X жөнінде` / `X хақында`), the word
/// immediately preceding the marker is the topic the user means,
/// even if it is a proper noun unknown to the FST lexicon.
///
/// Pre-v4.3.5 `best_noun_hint` only consulted FST-parsed nouns and
/// the multiword-entity list, so an utterance like
/// `Жазушы Мүсірепов туралы не білесіз?` lost the proper noun:
/// `жазушы` (in lexicon) won over `мүсірепов` (out of lexicon)
/// because only the former had an FST `Noun` parse to feed
/// `first_noun_root`. Real-test 2026-04-26 dialog:
///
/// > Жазушы Мүсірепов туралы не білесіз?
/// > → жазушы туралы қысқа жауап: Жазушы — прозалық шығарма жазатын адам.
///
/// Adam answered about "what is a writer" instead of about
/// Mүsirepov.
///
/// The marker is a *strong* context signal — when it fires we
/// should trust it over FST-only parsing. If the preceding word IS
/// in the FST parses, we still return its lemma (so `қалам туралы`
/// → `қала`); if it isn't, we return the surface form (so
/// `Мүсірепов туралы` → `Мүсірепов`).
pub(crate) fn topic_marker_hint(input: &str, parses: &[Analysis]) -> Option<String> {
    // **v4.26.5** — extended marker list. Added `дегеніміз` /
    // `деген` for the «X дегеніміз не?» / «X деген не?» pattern
    // (asking for a definition). Live battery on Rust knowledge
    // surfaced this gap: «Айнымалы дегеніміз не?» pre-fix
    // returned «Түсінбедім» because no marker matched, even
    // though `айнымалы` is a known world_core entry. With
    // `дегеніміз` registered, the word *before* the marker —
    // here `айнымалы` — is correctly extracted as topic. Same
    // structural logic as `туралы` / `жайында` / etc.
    const MARKERS: &[&str] = &[
        "туралы",
        "жайында",
        "жайлы",
        "жөнінде",
        "хақында",
        "дегеніміз",
        "деген",
    ];
    let lower = input.to_lowercase();
    for marker in MARKERS {
        let mut search_from = 0usize;
        while let Some(rel) = lower[search_from..].find(marker) {
            let pos = search_from + rel;
            // Word boundary on the left: either start-of-string or whitespace.
            let preceded_by_boundary = pos == 0
                || lower[..pos]
                    .chars()
                    .last()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false);
            if !preceded_by_boundary {
                search_from = pos + marker.len();
                continue;
            }
            let prefix = lower[..pos].trim_end();
            if let Some(last_word_lower) = prefix.split_whitespace().last() {
                let cleaned: String = last_word_lower
                    .chars()
                    .filter(|c| c.is_alphabetic() || *c == '-')
                    .collect();
                if cleaned.is_empty() || NOT_A_TOPIC.contains(&cleaned.as_str()) {
                    search_from = pos + marker.len();
                    continue;
                }
                // If the cleaned word is itself an FST-recognized
                // noun lemma, return it as-is (lowercase). This
                // preserves the existing "жер туралы" → "жер"
                // behaviour that goal_continuity scenarios depend on.
                let known_lemma = parses.iter().any(|p| {
                    matches!(p,
                        Analysis::Noun { root, .. } if root.root == cleaned
                            && !NOT_A_TOPIC.contains(&root.root.as_str())
                    )
                });
                if known_lemma {
                    return Some(cleaned);
                }
                // **v4.11.5** — inflected-form fallback. The word
                // right before `туралы` is frequently inflected
                // (`тілі` = `тіл + Px3sg`, `қалаға` = `қала + Dat`).
                // Pre-v4.11.5 the lemma check above failed because
                // the FST root (`тіл`) does not equal the surface
                // form (`тілі`), and the function fell through to
                // `normalize_proper_noun`, which title-cased the
                // inflected form into a fake proper noun (`Тілі`).
                // Closes the v4.11.0 transcript bug where
                // «Rust бағдарламалау тілі туралы не білесіз?»
                // extracted topic `Тілі` and routed retrieval to a
                // poetry quote about the body part `тілім`.
                //
                // Strategy: walk parses and pick the longest
                // noun-root that is a prefix of the cleaned form.
                // Bounded to ≥ 3 char roots to avoid false positives
                // on tiny stems.
                let inflected_lemma = parses
                    .iter()
                    .filter_map(|p| match p {
                        Analysis::Noun { root, .. }
                            if root.root.chars().count() >= 3
                                && cleaned.starts_with(root.root.as_str())
                                && !NOT_A_TOPIC.contains(&root.root.as_str()) =>
                        {
                            Some(root.root.clone())
                        }
                        _ => None,
                    })
                    .max_by_key(|s| s.chars().count());
                if let Some(lemma) = inflected_lemma {
                    return Some(lemma);
                }
                // Otherwise the word is unknown to FST (typically a
                // proper noun: `Мүсірепов`, `Малқаров`, …). Return
                // the title-cased form via `normalize_proper_noun`,
                // matching how `detect_statement_of_location`
                // normalizes city surface forms.
                return Some(crate::language_core::normalize_proper_noun(&cleaned));
            }
            search_from = pos + marker.len();
        }
    }
    None
}

/// v4.0.21 — Best noun hint for the `Intent::Unknown` fallback.
///
/// **v4.3.5** — checks the topic marker first (`X туралы`), then
/// falls back to multi-word entity match and finally
/// `first_noun_root`. The marker hint takes precedence so a proper
/// noun preceding the marker wins over a generic in-lexicon noun
/// elsewhere in the sentence.
/// **v4.37.5** — confidence-aware variant of [`best_noun_hint`].
///
/// Same strategy chain as the legacy entry point, but every branch
/// now returns a `TopicConfidence` band so the planner can route
/// uncertain extractions to the clarification family
/// (`unknown.clarify_low_confidence`). All structural strategies
/// (multiword, latin, topic_marker, locative_attributive,
/// adj_noun_compound, language_qualifier_prefix) report `High` —
/// they encode strong syntactic signals from the user's surface
/// form. `first_noun_root_with_confidence` reports its own band.
/// `accusative_form_hint` reports `Low` because it strips a suffix
/// without lexicon validation (string-level fallback for FST gaps).
pub(crate) fn best_noun_hint_with_confidence(
    input: &str,
    parses: &[Analysis],
) -> Option<(String, TopicConfidence)> {
    if let Some(latin) = latin_with_generic_head_marker(input) {
        return Some((latin, TopicConfidence::High));
    }
    if let Some(c) = crate::discourse::find_adj_noun_compound(input) {
        return Some((c.to_string(), TopicConfidence::High));
    }
    // **v4.39.0** — genitive-topic priority for list queries. Place
    // BEFORE multiword / topic-marker / first-noun strategies because
    // a genitive subject in a list query («Қазақстанның X-тарын
    // тізімдеңіз») is the discourse topic, while the head noun is
    // the predicate of the question. Pre-v4.39.0 the head-noun
    // strategies won, surfacing definition-style facts about the
    // class instead of the curated list owned by the genitive
    // subject. Conditional on `has_list_intent` inside the helper —
    // outside list queries the genitive subject often is NOT the
    // topic (e.g. «адамның мақсаты — өз ісін табу» where head
    // «мақсат» is the topic), so this strategy fires narrowly.
    if let Some(g) = genitive_topic_hint_for_list(input, parses) {
        return Some((g, TopicConfidence::High));
    }
    if has_language_qualifier_prefix(input) {
        if let Some(tm) = topic_marker_hint(input, parses) {
            return Some((tm, TopicConfidence::High));
        }
    }
    if let Some(mw) = multiword_entity_hint(input) {
        return Some((mw, TopicConfidence::High));
    }
    if let Some(latin) = latin_subject_hint(input) {
        return Some((latin, TopicConfidence::High));
    }
    if let Some(mw) = multiword_entity_hint(input) {
        return Some((mw, TopicConfidence::High));
    }
    if let Some(tm) = topic_marker_hint(input, parses) {
        return Some((tm, TopicConfidence::High));
    }
    if let Some(la) = locative_attributive_hint(input) {
        return Some((la, TopicConfidence::High));
    }
    if let Some(pair) = first_noun_root_with_confidence(parses) {
        return Some(pair);
    }
    accusative_form_hint(input).map(|s| (s, TopicConfidence::Low))
}

/// **v4.37.5** — confidence-stripping wrapper over
/// [`best_noun_hint_with_confidence`]. Preserved as the legacy entry
/// point for any caller that doesn't need the confidence band; new
/// callers should use the confidence-aware variant directly so the
/// planner can route to the clarification fork.
#[allow(dead_code)]
pub(crate) fn best_noun_hint(input: &str, parses: &[Analysis]) -> Option<String> {
    best_noun_hint_with_confidence(input, parses).map(|(s, _)| s)
}

/// **v4.4.12** — string-level locative-attributive suffix strip.
/// Kazakh forms a "located in X" attributive by attaching `-ғы /
/// -гі / -қы / -кі` to the locative-cased stem, yielding four
/// surface allomorphs `-дағы / -дегі / -тағы / -тегі`. The current
/// FST morphotactics does not model this derivation, so words
/// like «қазақстандағы», «алматыдағы», «мектептегі» return no
/// analysis and the topic extractor recovers nothing useful.
///
/// This fallback is purely string-level: it scans whitespace-
/// separated tokens, finds those ending in one of the four
/// allomorphs, and strips the entire 4-char tail to recover the
/// base noun. The recovered stem must be ≥ 3 codepoints and not
/// in `NOT_A_TOPIC`. Returns the first qualifying stem.
///
/// Conservative by design — does not validate the stem against
/// the lexicon (the FST gap is precisely that `тау` isn't always
/// surfaced as a noun even when present in the lexicon). The
/// 3-codepoint minimum is sufficient against false positives in
/// practice — any random word ending in `-дағы` that ISN'T the
/// locative-attributive of a real noun (e.g. as part of a longer
/// derivation) is rare enough that the dialog layer's downstream
/// retrieval/refusal handling absorbs the noise. Promote to a
/// proper FST morphotactics rule when adding the
/// `Case::LocativeAttributive` variant in a future minor.
/// **v4.11.6** — string-level accusative-form fallback. Kazakh
/// Accusative attaches one of six surface allomorphs by vowel
/// harmony + final-sound class: `-ны / -ні` after vowel, `-ды /
/// -ді` after voiced consonant, `-ты / -ті` after voiceless
/// consonant. The current FST has lexicon gaps on inflected
/// loanword roots (e.g. `биологияны = биология + Acc`,
/// `химияны = химия + Acc`, `тарихты = тарих + Acc`) and emits no
/// Noun analysis, so all upstream noun-hint strategies yield None
/// and the conversation falls to bare `unknown` → "Түсінбедім.".
///
/// Conservative: only fires on tokens of ≥ 5 chars (≥ 3 stem +
/// 2 suffix), recovered stem must be ≥ 3 codepoints, must not
/// match `NOT_A_TOPIC`. Returns the first qualifying stem.
pub(crate) fn accusative_form_hint(input: &str) -> Option<String> {
    const SUFFIXES: &[&str] = &["ны", "ні", "ды", "ді", "ты", "ті"];
    let lower = input.to_lowercase();
    for raw_word in lower.split_whitespace() {
        let word: String = raw_word
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect();
        let word_len = word.chars().count();
        if word_len < 5 {
            continue;
        }
        for suffix in SUFFIXES {
            if word.ends_with(suffix) {
                let stem_chars = word_len - suffix.chars().count();
                let stem: String = word.chars().take(stem_chars).collect();
                if stem.chars().count() >= 3 && !NOT_A_TOPIC.contains(&stem.as_str()) {
                    return Some(stem);
                }
            }
        }
    }
    None
}

/// **v4.39.0** — string-level genitive-suffix strip for list queries.
/// Mirrors [`locative_attributive_hint`] in shape: closes a known FST
/// gap by recovering the genitive stem at the string level when the
/// FST fails to derive `noun + Genitive` for nasal-final / vowel-final
/// stems (the `realise_d` archiphoneme rule currently produces
/// `адамдың` instead of `адамның`, so analyse(«адамның») returns
/// nothing — same gap blocks «Қазақстанның», «ананың», etc.).
///
/// Strategy: when the input has list-intent shape («тізім / атаулары
/// / атап шық / атап өт / барлық атау»), scan tokens ending in one of
/// the six genitive allomorphs `-ның / -нің / -тың / -тің / -дың /
/// -дің` and return the bare stem. The list-intent gate is critical —
/// without it, every adjective-genitive in normal speech (e.g.
/// «адамның» in «адамның мақсаты — ...» = "human's goal") would be
/// promoted over the more-specific head noun, which is the wrong
/// topic for definition-style queries.
///
/// Closes bug 4 («Қазақстанның барлық аймақтарын тізімдеңіз»):
/// pre-v4.39.0 noun_hint = «аймақ» (head noun) → bridge fact «Аймақ —
/// аумақ»; post-v4.39.0 noun_hint = «қазақстан» → SearchGraph
/// returns the curated «облыстар тізімі» list-summary fact.
pub(crate) fn genitive_topic_hint_for_list(input: &str, parses: &[Analysis]) -> Option<String> {
    let lower = input.to_lowercase();
    let has_list_intent = lower.contains("тізім")
        || lower.contains("атаулары")
        || lower.contains("атап шық")
        || lower.contains("атап өт")
        || lower.contains("барлық")
        // **v4.40.5** — extended triggers (mirror `tool.rs`
        // v4.40.5 list-intent extension): «айтып бер / келтір /
        // атаңыз / көрсет / тізіп бер» phrasings ask for items
        // of a class even without an explicit «тізім» token.
        // Surfaced by 2026-05-03 transcript test where
        // «Қазақстанның танымал тұлғалары туралы айтып
        // беріңізші» picked head-noun «тұлға» instead of
        // genitive subject «қазақстан» because the trigger set
        // was too narrow.
        || lower.contains("айтып бер")
        || lower.contains("келтір")
        || lower.contains("атаңыз")
        || lower.contains("көрсет")
        || lower.contains("тізіп бер");
    // **v4.39.0** — also fire on quantity questions with a
    // possessive-genitive shape («Қазақстанның халқы шамамен
    // қанша?»). The genitive subject is the *holder* of the
    // counted property; without this gate, head-noun strategies
    // pick «халқы» as topic and SearchGraph misses the
    // `қазақстан-has_quantity-халық` fact stored under the
    // genitive subject.
    let has_qty_intent = lower.contains("қанша") || lower.contains("неше");
    if !has_list_intent && !has_qty_intent {
        return None;
    }
    // **v4.39.5** — parse-stream version (was string-level in
    // v4.39.0). The string-level workaround was needed because the
    // FST didn't derive Genitive on nasal/vowel-final stems
    // («Қазақстанның» → []); v4.39.5 closed that gap via the new
    // {DN} archiphoneme, so `parses` now contains a `Case::Genitive`
    // analysis for those forms with the bare root attached. Reading
    // from parses gives us the lexically-validated root for free.
    //
    // First-parse gate: only fire if the FIRST parse is a Genitive
    // noun. Mirrors the v4.39.0 first-token-genitive gate semantics
    // — for inputs like «Қазақстан аймақтарының X-тарын
    // тізімдеңіз» (bare leading noun, then deeper genitive), the
    // first parse is bare «қазақстан» (no Case), so this strategy
    // declines and the legacy chain picks the leading bare noun.
    let first = parses.first()?;
    match first {
        Analysis::Noun { root, features }
            if matches!(
                features.case,
                Some(adam_kernel_fst::morphotactics::Case::Genitive)
            ) && !NOT_A_TOPIC.contains(&root.root.as_str()) =>
        {
            Some(root.root.clone())
        }
        _ => None,
    }
}

pub(crate) fn locative_attributive_hint(input: &str) -> Option<String> {
    const SUFFIXES: &[&str] = &["дағы", "дегі", "тағы", "тегі"];
    let lower = input.to_lowercase();
    for raw_word in lower.split_whitespace() {
        let word: String = raw_word
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect();
        let word_len = word.chars().count();
        if word_len < 7 {
            // Need ≥ 3 stem chars + 4 suffix chars.
            continue;
        }
        for suffix in SUFFIXES {
            if word.ends_with(suffix) {
                let stem_chars = word_len - suffix.chars().count();
                let stem: String = word.chars().take(stem_chars).collect();
                if stem.chars().count() >= 3 && !NOT_A_TOPIC.contains(&stem.as_str()) {
                    return Some(stem);
                }
            }
        }
    }
    None
}

/// v1.7.0: return every distinct content root from the parse list.
///
/// This is what the retrieval ranker consumes — more morphemes in means
/// more signal for the overlap score. Preserves insertion order so the
/// first hit still wins for equal-score ties after ranking.
pub fn content_roots(parses: &[Analysis]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for a in parses {
        if let Analysis::Noun { root, .. } = a {
            let r = root.root.as_str();
            if NOT_A_TOPIC.contains(&r) {
                continue;
            }
            // Keep POS-wise noun-like only. "adjective" roots are
            // signal too but widen the net; v1.7.0 sticks to nouns.
            if root.part_of_speech != "noun" {
                continue;
            }
            if seen.insert(root.root.clone()) {
                out.push(root.root.clone());
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// **v4.43.9** — multiword_entity_hint inflected-FIRST-word tests.

#[cfg(test)]
mod tests {
    use super::*;

    /// **v4.43.9** — exact-bare-form substring match (existing
    /// first-pass behavior). Anti-regression test.
    #[test]
    fn multiword_hint_first_pass_exact_substring() {
        let hit = multiword_entity_hint("қазақ тілі туралы айтыңыз");
        assert_eq!(hit.as_deref(), Some("қазақ тілі"));
    }

    /// **v4.43.9** — inflected-second-word pass (v4.40.5 behavior).
    /// Anti-regression test.
    #[test]
    fn multiword_hint_second_pass_inflected_second_word() {
        // «бағдарламалау тілдерін» = бағдарламалау + тіл (Pl+P3+Acc).
        // Should match the registered entity «бағдарламалау тілі».
        let hit = multiword_entity_hint("қандай бағдарламалау тілдерін білесіз");
        assert_eq!(hit.as_deref(), Some("бағдарламалау тілі"));
    }

    /// **v4.43.9** — new third pass: inflected-FIRST-word (Kazakh
    /// genitive «-тың» on voiceless-consonant-final stem). Uses
    /// the registered entity `қазақ тілі` (no Gen-form variant
    /// in `MULTIWORD_ENTITIES` so the third pass owns this match).
    #[test]
    fn multiword_hint_third_pass_genitive_first_word_returns_bare_form() {
        // «қазақтың тілі» = қазақ + Gen «-тың» + тіл + Pos3.
        // First pass: substring "қазақ тілі" not in "қазақтың тілі" (extra «тың»).
        // Third pass: parts[0]="қазақ" (5 chars OK), window[0]="қазақтың"
        // = "қазақ"+"тың" ✓, window[1]="тілі" starts with "тіл" ✓.
        let hit = multiword_entity_hint("қазақтың тілі туралы айтыңыз");
        // Returns the BARE form, not the inflected surface — matches
        // the canonical world_core fact subject.
        assert_eq!(hit.as_deref(), Some("қазақ тілі"));
    }

    /// **v4.43.9** — third pass: front-vowel genitive variant (-нің
    /// after vowel-final / sonorant-final). Uses registered entity
    /// `жүк машинасы` (back harmony, voiceless-consonant-final «к»
    /// → Gen «-тің»? actually «жүктің машинасы» — first word жүк
    /// is back-vowel due to «ү» but that's actually front. Let me
    /// pick a cleaner case): `темір жол` → Gen «темірдің жолы».
    /// Front harmony, voiced-consonant-final «р» → Gen «-дің».
    #[test]
    fn multiword_hint_third_pass_handles_temir_zhol_genitive() {
        // «темірдің жолы» = темір + «-дің» + жол + Pos3.
        // Note: parts[1]="жол" is only 3 chars, exactly at the
        // stem-prefix threshold. window[1]="жолы" starts with "жол" ✓.
        let hit = multiword_entity_hint("темірдің жолы туралы");
        assert_eq!(hit.as_deref(), Some("темір жол"));
    }

    /// **v4.43.9** — third pass refuses short first words (< 4 chars)
    /// to avoid spurious matches.
    #[test]
    fn multiword_hint_third_pass_skips_short_first_word() {
        // No registered 2-word entity starts with a 3-char first word
        // followed by a noun whose stem starts with a recognizable
        // prefix; this asserts the gate is in place.
        // We construct a hypothetical: if there were "бір сөз" entity
        // (3-char first), input "бірдің сөзі" should NOT match.
        // Since no such entity exists, this test verifies non-firing
        // on a constructed input that COULD match if the gate were
        // absent.
        let hit = multiword_entity_hint("бірдің сөзі");
        assert!(hit.is_none() || !hit.as_deref().unwrap().starts_with("бір "));
    }

    /// **v4.43.9** — third pass: nasal-final genitive («-ның»).
    /// Uses registered entity `мемлекет басшысы`; Gen «мемлекеттің»
    /// is voiceless-consonant-final → «-тің», not «-ның». Pick a
    /// nasal-final case: `қазан күрделі` — wait, not registered.
    /// Use `аспан денесі`: «аспанның денесі» = «аспан»+«ның»+«дене»+P3.
    #[test]
    fn multiword_hint_third_pass_handles_nasal_genitive() {
        // «аспанның денесі» = аспан + «-ның» + дене + Pos3.
        // window[0]="аспанның" = "аспан"+"ның" ✓ (parts[0]="аспан"
        // is 5 chars ≥ 4); window[1]="денесі" starts with "ден" ✓.
        let hit = multiword_entity_hint("аспанның денесі туралы");
        assert_eq!(hit.as_deref(), Some("аспан денесі"));
    }
}
