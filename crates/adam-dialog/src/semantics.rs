//! Layer 2 — semantic interpreter.
//!
//! **Kazakh-only surface (post-v1.1.0).** The v0.9.6 multilingual
//! recogniser (Russian / English triggers) was reverted in v1.1.0 per
//! the Kazakh-first directive — `adam` accepts and produces only
//! Kazakh. All detectors below match Kazakh keywords / phrases
//! exclusively. For MVP social intents the FST parser is more than we
//! need; we work directly on the lowercased-cleaned token list.

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::Analysis;

use crate::intent::{GreetingKind, Intent, TimeOfDay};
use crate::language_core::{
    looks_like_named_place_candidate, normalize_place_name, normalize_proper_noun,
};

/// Entry point. Takes the raw input text; tokenises, lowercases, strips
/// punctuation, then dispatches by keyword rules.
///
/// The `_parses` argument is kept so callers stay forward-compatible:
/// v0.7.5 intents can start using morphological info without changing
/// the call site.
pub fn interpret_text(input: &str, parses: &[Analysis]) -> Intent {
    interpret_text_with_lexicon(input, parses, None)
}

/// Lexicon-aware variant used by `Conversation::turn` and
/// `respond_with_repo`. When a lexicon is supplied, the occupation
/// recogniser does a generic 1sg-copula strip + noun lookup instead
/// of consulting a fixed 6-form table — giving full Lexicon reach
/// (e.g. `философпын` → `философ` if present).
pub fn interpret_text_with_lexicon(
    input: &str,
    parses: &[Analysis],
    lexicon: Option<&LexiconV1>,
) -> Intent {
    // Keep two parallel token streams:
    //   `tokens`  — cleaned lowercase, used for keyword matching
    //   `raw_tokens` — case-preserving, used for PersonName extraction
    //                  (so "Дәулет" isn't turned into "дәулет")
    //
    // **v4.2.5** — digit characters now pass the filter. Pre-v4.2.5
    // both streams stripped digits via `c.is_alphabetic() || '-'`,
    // so a literal age like `30` in "менің жасым 30" never reached
    // `parse_kazakh_age` and `Intent::StatementOfAge` came out with
    // `years: None`. The age slot then never made it into session,
    // so AskAge couldn't surface a stored value via `belief_direct_answer`.
    // Surfaced by the v4.2.1 cognitive eval expansion's
    // `aspirational_direct_answer_age_surfaces_stored_value` scenario.
    let tokens: Vec<String> = input
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect();
    let raw_tokens: Vec<String> = input
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .collect();
    let joined = tokens.join(" ");

    // StatementOfName must come BEFORE greeting: "hi i am John" starts
    // with "hi" which would otherwise trip Greeting::Casual. The
    // statement-of-name rule requires an explicit pattern (атым/есімім/
    // зовут/my name is/call me/[greet] i am X) so false positives from
    // a bare greeting are ruled out.
    if let Some(name) = detect_statement_of_name(&tokens, &raw_tokens, &joined) {
        return Intent::StatementOfName { name };
    }
    if detect_ask_how_are_you(&joined) {
        return Intent::AskHowAreYou;
    }
    if let Some(g) = detect_greeting(&tokens, &joined) {
        return g;
    }
    if detect_farewell(&tokens, &joined) {
        return Intent::Farewell;
    }
    // Order matters: thanks/apology should be checked before affirmation,
    // because "рахмет" is a single-token thanks and shouldn't accidentally
    // fall into affirmation if we ever add "рахмет" there.
    if detect_thanks(&tokens, &joined) {
        return Intent::Thanks;
    }
    if detect_apology(&tokens, &joined) {
        return Intent::Apology;
    }
    // **v4.3.3 / v4.3.4** — `сен кімсің` / `сені кім жасады` /
    // `қашан пайда болдың` / `ерекшелігің не` ask about adam's
    // identity (general / creator / birthdate / architecture
    // aspects). The detector returns the `SystemAspect` it matched
    // so the planner can pick the right `ask_about_system.*`
    // template family. Must be checked BEFORE `detect_ask_name`.
    if let Some(aspect) = detect_ask_about_system(&tokens, &joined, input) {
        return Intent::AskAboutSystem { aspect };
    }
    // **v4.14.0** — curriculum-content question detector.
    // Pattern: subject in {`оқушы`, `студент`, `шәкірт`, `бала`} +
    // education locus (`мектеп`, `сабақ`, `сыныпта`, `университет`)
    // + question word `не` + learning verb (`оқу`, `үйрену`,
    // `өту`). Catches «Оқушылар мектепте физика пәнінен не оқиды?»
    // — pre-v4.14.0 surfaced a greedy IsA fact about the first
    // content noun (`оқушы` → `Оқушы мектеп құрамына кіреді`).
    // The honest fallback says: this is curriculum content, I
    // don't have it. Distinct from `MultiTopicCapability` (v4.13.5,
    // about LISTING subjects), `Knowledge` (v4.6.0, about adam's
    // domain breadth), and `Limitations` (v4.6.0, about what adam
    // can't do).
    if detect_curriculum_content_question(&joined) {
        return Intent::AskCurriculumContent;
    }
    if detect_ask_name(&joined) {
        return Intent::AskName;
    }
    // Statement-* is checked BEFORE Ask-* inside each topic pair: a
    // 1st-person marker ("келдім", "тұрамын", "жасым") unambiguously
    // means the user is stating, not asking. Without this ordering,
    // "қайдан келдім" would hit AskLocation (because of "қайдан")
    // before StatementOfLocation (which keys on "келдім").
    // **v4.4.5** — `detect_ask_age` runs BEFORE
    // `detect_statement_of_age` so a 1sg-self-recall like
    // `менің жасым қанша?` reaches `AskAge` instead of being
    // swallowed by `StatementOfAge { years: None }`. The 1sg-form
    // overlap (`жасым` is both "my age (statement)" and "my age
    // (in a self-recall question)") forced the change; the
    // ask-form detector now also matches `жасым + қанша/неше` and
    // the statement-form detector refuses to match when a
    // question particle is present.
    if detect_ask_age(&joined) {
        return Intent::AskAge;
    }
    if let Some(years) = detect_statement_of_age(&tokens, &joined) {
        return Intent::StatementOfAge { years };
    }
    if let Some(city) = detect_statement_of_location(&tokens, &raw_tokens, &joined, parses) {
        return Intent::StatementOfLocation { city };
    }
    if detect_ask_location(&joined) {
        return Intent::AskLocation;
    }
    if let Some(occupation) =
        detect_statement_of_occupation(&tokens, &raw_tokens, &joined, lexicon, parses)
    {
        return Intent::StatementOfOccupation { occupation };
    }
    if detect_ask_occupation(&joined) {
        return Intent::AskOccupation;
    }
    if detect_statement_of_family(&joined) {
        return Intent::StatementOfFamily;
    }
    if detect_ask_family(&joined) {
        return Intent::AskFamily;
    }
    if detect_statement_of_weather(&tokens, &joined) {
        return Intent::StatementOfWeather;
    }
    if detect_ask_weather(&joined) {
        return Intent::AskWeather;
    }
    if detect_ask_time(&joined) {
        return Intent::AskTime;
    }
    if detect_compliment(&tokens, &joined) {
        return Intent::Compliment;
    }
    if detect_request(&tokens, &joined) {
        return Intent::Request;
    }
    if detect_well_wishes(&joined) {
        return Intent::WellWishes;
    }
    if detect_insult(&tokens, &joined) {
        return Intent::Insult;
    }
    // **v4.6.20** — long, gracious user-acknowledgement: «Мен
    // сенің әлі бәрін білмейтініңді … түсіндім». Detected by the
    // pair (addressee + 1sg perfective realisation verb) in
    // `discourse::input_is_user_acknowledgement`. Must be checked
    // BEFORE the noun-hint fallback so the greedy `әлі`-grabbing
    // path is suppressed. Not gated on `pronoun` because the
    // realisation verb itself carries 1sg agreement.
    if crate::discourse::input_is_user_acknowledgement(&joined) {
        return Intent::UserAcknowledgement;
    }
    if detect_statement_of_wellbeing(&tokens, &joined) {
        return Intent::StatementOfWellbeing;
    }
    if detect_affirmation(&tokens, &joined) {
        return Intent::Affirmation;
    }
    if detect_negation(&tokens, &joined) {
        return Intent::Negation;
    }

    // Unknown — but try to extract a noun hint from the parses so the
    // fallback response can at least acknowledge context. `example` is
    // filled later by `Conversation::turn` via retrieval (v1.6.5).
    // `example_adapted` (v1.9.5) is also set there.
    // v4.0.21 — prefer multi-word entity match before single-noun fallback
    // so «Құс жолы туралы айтшы» stays intact (not reduced to «құс»).
    let noun_hint = best_noun_hint(input, parses);
    // **v4.12.0** — detect question shape at the same point we
    // populate `noun_hint`. Pure surface-level scan; cheap and
    // independent of the FST analyses. `None` for non-questions.
    let question_shape = crate::question_shape::detect(input);
    Intent::Unknown {
        raw_tokens: tokens,
        noun_hint,
        example: None,
        grounded_fact: None,
        example_adapted: false,
        reasoning_chain: None,
        question_shape,
    }
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
const NOT_A_TOPIC: &[&str] = &[
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
fn first_noun_root(parses: &[Analysis]) -> Option<String> {
    parses.iter().find_map(|a| match a {
        Analysis::Noun { root, .. } if !NOT_A_TOPIC.contains(&root.root.as_str()) => {
            Some(root.root.clone())
        }
        _ => None,
    })
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
const MULTIWORD_ENTITIES: &[&str] = &[
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
];

/// Longest-match scan of `input` against `MULTIWORD_ENTITIES`. Returns
/// the first entity found as a substring of the lowercased input.
/// Substring match handles Kazakh inflection on the last word of the
/// compound — e.g. «Құс жолының бейнесі» contains «құс жолы» as a prefix
/// of the inflected compound.
fn multiword_entity_hint(input: &str) -> Option<String> {
    let lowered = input.to_lowercase();
    for entity in MULTIWORD_ENTITIES {
        if lowered.contains(entity) {
            return Some((*entity).to_string());
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
const LATIN_TECH_SUBJECTS: &[&str] = &[
    "btreemap", "vecdeque", "rustfmt", "hashmap", "hashset", "refcell", "continue", "collect",
    "expect", "filter", "future", "string", "unwrap", "break", "cargo", "clippy", "loop", "mutex",
    "none", "option", "panic", "result", "rust", "rustc", "some", "usize", "while", "await",
    "bool", "char", "f32", "f64", "i32", "i64", "u32", "u64", "for", "map", "mod", "pub", "use",
    "vec", "arc", "box", "err", "rc", "ok", "str",
];

/// **v4.11.5** — scan input for any whitespace-separated word
/// matching a known Latin technical subject. Returns the matched
/// subject in canonical lowercase form (matching world_core
/// `subject` field). Word boundaries are whitespace + punctuation
/// + backticks; trailing punctuation in tokens is stripped before
/// comparison (e.g. `Rust?` → `rust`). Ignores backtick-quoted
/// spans because those usually mean code identifiers in their
/// surrounding context, not a topical reference.
fn latin_subject_hint(input: &str) -> Option<String> {
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
            if best.map_or(true, |b| hit.len() > b.len()) {
                best = Some(hit);
            }
        }
    }
    best.map(|s| s.to_string())
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
fn topic_marker_hint(input: &str, parses: &[Analysis]) -> Option<String> {
    const MARKERS: &[&str] = &["туралы", "жайында", "жөнінде", "хақында"];
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
fn best_noun_hint(input: &str, parses: &[Analysis]) -> Option<String> {
    // **v4.6.20** — adj+noun compound topic ("машиналық оқыту",
    // "жасанды интеллект", "табиғи тіл"). Pre-v4.6.20 the first-
    // noun-root strategy returned only the head noun (`оқыту`)
    // and silently dropped the modifier, so retrieval matched on
    // a generic concept instead of the compound. The compound
    // list is closed and audited in `discourse.rs`. Runs first so
    // the more-specific compound wins over any single-noun fallback.
    if let Some(c) = crate::discourse::find_adj_noun_compound(input) {
        return Some(c.to_string());
    }
    // **v4.11.5** — Latin-name passthrough for technical proper
    // nouns. Closes the v4.7.0 known limitation where queries
    // typed in Latin (`Rust`, `Cargo`, `String`) fall through to
    // `Intent::Unknown` because Cyrillic-only FST returns no
    // analysis. The closed list mirrors Latin-named subjects in
    // `programming_rust.jsonl`. When the user's question contains
    // any of these as a whole word (case-insensitive), use it as
    // the topic so SearchGraph can route to the per-Rust-concept
    // facts (`rust IsA бағдарламалау тілі`, `cargo IsA құрал`,
    // …). Runs BEFORE multiword/topic_marker so an explicit Latin
    // proper noun beats any contained Cyrillic compound.
    if let Some(latin) = latin_subject_hint(input) {
        return Some(latin);
    }
    // **v4.11.5** — `multiword_entity_hint` promoted to run BEFORE
    // `topic_marker_hint`. Pre-v4.11.5 the marker hint won first,
    // so questions like «Rust бағдарламалау тілі туралы не
    // білесіз?» extracted only the word right before `туралы`
    // (`тілі`/`тіл`) and missed the more specific multiword
    // compound (`бағдарламалау тілі`). When MULTIWORD_ENTITIES
    // contains a compound that appears in the input, that compound
    // is almost always the user's intended topic — more specific
    // than any single constituent. The reorder lets the world_core
    // graph lookup land on the compound subject (which world_core
    // entries explicitly model: `info_007 бағдарламалау тілі IsA
    // формалды тіл`) instead of falling through to a corpus
    // citation about an inflected single word.
    multiword_entity_hint(input)
        .or_else(|| topic_marker_hint(input, parses))
        // v4.4.13 — locative-attributive suffix recovery. The
        // `-дағы / -дегі / -тағы / -тегі` morpheme is a strong
        // "specifically located in X" topic-narrowing signal,
        // semantically equivalent to a `туралы` marker for the
        // word it attaches to. When present, the recovered stem
        // (e.g. `қазақстан` from `қазақстандағы`) is the most
        // specific topic in the question and should win over any
        // generic content noun (e.g. `тау` from `таулар`) found
        // elsewhere via `first_noun_root`.
        .or_else(|| locative_attributive_hint(input))
        .or_else(|| first_noun_root(parses))
        // **v4.11.6** — accusative-form fallback. Closes the
        // «биологияны білесің бе?» → "Түсінбедім" gap from the
        // 2026-04-30 transcript: FST has a lexicon gap on
        // `биологияны / химияны / тарихты` (loanword roots in
        // Accusative case) and emits no Noun analysis, so all
        // upstream strategies yield None. String-level stripper of
        // the six Accusative allomorphs recovers the bare stem.
        // Runs LAST as a fallback after FST-driven strategies
        // have all failed.
        .or_else(|| accusative_form_hint(input))
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
fn accusative_form_hint(input: &str) -> Option<String> {
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

fn locative_attributive_hint(input: &str) -> Option<String> {
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

/// Legacy-compatible wrapper: runs intent recognition on parse surface
/// forms only. Kept for tests that don't have raw input handy.
pub fn interpret(parses: &[Analysis]) -> Intent {
    let tokens = surface_tokens(parses);
    let joined = tokens.join(" ");

    if let Some(g) = detect_greeting(&tokens, &joined) {
        return g;
    }
    if detect_farewell(&tokens, &joined) {
        return Intent::Farewell;
    }
    if detect_thanks(&tokens, &joined) {
        return Intent::Thanks;
    }
    if detect_apology(&tokens, &joined) {
        return Intent::Apology;
    }
    if detect_ask_how_are_you(&joined) {
        return Intent::AskHowAreYou;
    }
    if detect_ask_name(&joined) {
        return Intent::AskName;
    }
    if detect_statement_of_wellbeing(&tokens, &joined) {
        return Intent::StatementOfWellbeing;
    }
    if detect_affirmation(&tokens, &joined) {
        return Intent::Affirmation;
    }
    if detect_negation(&tokens, &joined) {
        return Intent::Negation;
    }

    // Legacy path has no raw input, so multi-word matching is skipped —
    // callers using raw-text-aware `interpret_text_with_lexicon` get the
    // full v4.0.21 multiword treatment.
    Intent::Unknown {
        raw_tokens: tokens,
        noun_hint: first_noun_root(parses),
        example: None,
        grounded_fact: None,
        example_adapted: false,
        reasoning_chain: None,
        // **v4.12.0** — legacy path has no raw input, so question
        // shape cannot be detected (the detector is surface-level).
        // Always None on this code path.
        question_shape: None,
    }
}

fn surface_tokens(parses: &[Analysis]) -> Vec<String> {
    parses
        .iter()
        .map(|a| match a {
            Analysis::Noun { root, .. } => root.root.clone(),
            Analysis::Verb { root, .. } => root.root.clone(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Individual recognisers.
// Each returns `Some(Intent)` or a `bool` for match/no-match.
// ---------------------------------------------------------------------------

fn detect_greeting(tokens: &[String], joined: &str) -> Option<Intent> {
    // Time-of-day: "қайырлы таң / күн / кеш".
    if joined.contains("қайырлы") {
        let tod = if joined.contains("таң") {
            TimeOfDay::Morning
        } else if joined.contains("кеш") {
            TimeOfDay::Evening
        } else {
            TimeOfDay::Day
        };
        return Some(Intent::Greeting {
            kind: GreetingKind::TimeOfDay(tod),
        });
    }
    // Polite: "сәлеметсіз бе" / "сәлеметсің бе".
    if joined.contains("сәлеметсіз") || joined.contains("сәлеметсің") {
        return Some(Intent::Greeting {
            kind: GreetingKind::Polite,
        });
    }
    // Casual: "сәлем".
    if tokens.first().is_some_and(|t| t == "сәлем") {
        return Some(Intent::Greeting {
            kind: GreetingKind::Casual,
        });
    }
    // v4.4.10 — Introduction proposal: «танысайық» / «танысалық»
    // («let's get acquainted») as a stand-alone opener, plus
    // compound forms «танысып алайық» / «танысып алыңыз».
    // Surfaced by a 2026-04-28 real-REPL trace that landed on the
    // generic unknown refusal pre-v4.4.10. The polite imperative
    // `танысайық` is a 1pl exhortative, not a greeting in the
    // strict sense, but it opens an introduction exchange and
    // belongs in the Greeting bucket so the planner volunteers
    // adam's name and asks for the user's.
    if tokens
        .iter()
        .any(|t| t == "танысайық" || t == "танысалық" || t == "танысыңыз")
        || joined.contains("танысып алайық")
        || joined.contains("танысып алыңыз")
    {
        return Some(Intent::Greeting {
            kind: GreetingKind::IntroProposal,
        });
    }
    None
}

fn detect_farewell(tokens: &[String], joined: &str) -> bool {
    tokens.first().is_some_and(|t| t == "сау" || t == "қош")
        || joined.contains("кездескенше")
        || joined.contains("сау бол")
        || joined.contains("қош бол")
        || joined.contains("аман бол")
}

fn detect_affirmation(tokens: &[String], joined: &str) -> bool {
    if tokens.len() == 1 {
        let w = &tokens[0];
        return matches!(
            w.as_str(),
            "иә" | "ия" | "дұрыс" | "рас" | "мақұл" | "әрине"
        );
    }
    joined.contains("дұрыс айтасыз") || joined == "иә дұрыс"
}

fn detect_negation(tokens: &[String], joined: &str) -> bool {
    if tokens.len() == 1 {
        let w = &tokens[0];
        return matches!(w.as_str(), "жоқ" | "қате" | "емес");
    }
    joined.contains("жоқ емес") || joined.starts_with("жоқ")
}

// --- v0.7.5 new recognisers ------------------------------------------------

fn detect_thanks(tokens: &[String], joined: &str) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t.as_str(), "рахмет" | "рахметім" | "рақмет"))
        || joined.contains("көп рахмет")
        || joined.contains("көп рақмет")
}

fn detect_apology(tokens: &[String], joined: &str) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t.as_str(), "кешіріңіз" | "кешір" | "ғафу"))
        || joined.contains("кешір")
        || joined.contains("ғафу ет")
}

fn detect_ask_how_are_you(joined: &str) -> bool {
    joined.contains("қалайсың")
        || joined.contains("қалайсыз")
        || joined.contains("жағдайың қалай")
        || joined.contains("жағдайыңыз қалай")
        || joined.contains("халің қалай")
        || joined.contains("халіңіз қалай")
        || joined.contains("қалдарың қалай")
        || joined.contains("қалдарыңыз қалай")
        || joined == "қалың қалай"
        // **v4.6.12** — polite singular/plural forms surfaced by a
        // 2026-04-29 real-REPL transcript: «Қалыңыз қалай?». Maps
        // to «How is your state?» / «How are you (polite)?» —
        // standard Kazakh greeting-inquiry. Also covers the bare
        // 2sg-informal «қалың қалай» that was already there as an
        // exact-match (now matched as substring for robustness in
        // sentences like «Айтшы, қалың қалай?»).
        || joined.contains("қалыңыз қалай")
        || joined.contains("қалың қалай")
}

/// **v4.3.3** — match identity-question phrasings clearly addressed
/// to adam itself: pronoun + identity question.
///
/// Triggers (informal `сен` and formal `сіз` paired):
/// - `сен кімсің` / `сіз кімсіз` ("who are you")
/// - `сен қандай моделсің` / `сіз қандай моделсіз` ("what kind of
///   model are you")
/// - `сен қандайсың` / `сіз қандайсыз` ("what are you like" — used
///   in "what kind of system" sense in this dialog domain)
/// - `сен немен айналысасың` / `сіз немен айналысасыз` ("what do
///   you do" — addressed to system as worker)
///
/// Does NOT include the bare `атың кім` / `есіміңіз` phrasings —
/// those stay with `detect_ask_name` (and the v4.2.5 slot-aware
/// path) to preserve the v4.2.5 cognitive scenarios that exercise
/// the AnswerDirect rendering for stored user names. The
/// pronoun-led patterns here are unambiguously about adam.
/// **v4.14.0** — curriculum-content question detector.
///
/// Pattern: subject (student-class noun) + education locus +
/// question word `не` + learning verb. Conservative — requires all
/// four signals so a generic «оқушылар туралы не білесіз?» (the
/// IsA-of-students question) doesn't accidentally route here.
///
/// Surface anchors:
/// - subject: `оқушы` / `оқушылар` / `студент` / `шәкірт` / `бала`
///   (NOT `балалар` if used as "kids" generically — paired with
///   education locus to disambiguate)
/// - education locus: `мектеп` / `сабақ` / `сыныпта` / `университет`
/// - question word: `не` (the WHAT)
/// - learning verb: `оқу` / `оқиды` / `үйрену` / `үйренеді` / `өту`
fn detect_curriculum_content_question(joined: &str) -> bool {
    let has_student =
        joined.contains("оқушы") || joined.contains("студент") || joined.contains("шәкірт");
    let has_education_locus = joined.contains("мектеп")
        || joined.contains("сабақ")
        || joined.contains("сыныпта")
        || joined.contains("сыныпқа")
        || joined.contains("университет")
        || joined.contains("колледж");
    let has_what = joined.contains("не ")
        || joined.contains("не?")
        || joined.ends_with("не")
        || joined.contains("нені")
        || joined.contains("неден");
    let has_learning_verb = joined.contains("оқиды")
        || joined.contains("оқимыз")
        || joined.contains("оқисың")
        || joined.contains("оқисыз")
        || joined.contains("үйренеді")
        || joined.contains("үйрене");
    has_student && has_education_locus && has_what && has_learning_verb
}

fn detect_ask_about_system(
    tokens: &[String],
    joined: &str,
    raw_input: &str,
) -> Option<crate::system_identity::SystemAspect> {
    use crate::system_identity::SystemAspect;
    let pronoun = tokens.iter().any(|t| t == "сен" || t == "сіз");
    let has_addressee = pronoun || joined.contains("сені") || joined.contains("сізді");

    // **v4.3.4** — Creator aspect: "who made you" / "who built you".
    // Triggered by addressee-marker (сені/сізді/сен/сіз) plus
    // creator-question keyword. Checked first so multi-question
    // utterances ("сен кімсің және сені кім жасады?") pick this up
    // when the creator part is present — the architecture/birthdate
    // / general checks below all resolve to a less-specific aspect.
    if has_addressee
        && (joined.contains("кім жасады")
            || joined.contains("кім құрды")
            || joined.contains("кім жасап шығарды")
            || joined.contains("кім ойлап тапты")
            || joined.contains("авторың")
            || joined.contains("авторыңыз")
            || joined.contains("жасаушың")
            || joined.contains("жасаушыңыз")
            || joined.contains("кім құрастырды")
            // **v4.6.1** — additional creator-question verb forms
            // surfaced by a 2026-04-29 real-REPL transcript:
            //   «Ал сені кім жаратты?»     (created)
            //   «Сізді кім дамытқан?»      (developed)
            //   «Сізді қай бағдарламашы дайындады?» (which programmer prepared)
            // The first two key off the verb directly; the third
            // also covers the «бағдарламашы» (programmer) framing
            // since that's a creator-role question even without
            // a `кім жасады`-style verb.
            || joined.contains("кім жаратты")
            || joined.contains("кім дамытқан")
            || joined.contains("кім дамытты")
            || joined.contains("кім дайындады")
            || joined.contains("жаратушың")
            || joined.contains("жаратушыңыз")
            || joined.contains("қай бағдарламашы")
            || joined.contains("қандай бағдарламашы")
            || joined.contains("бағдарламашы дайындады")
            || joined.contains("бағдарламашы жасады"))
    {
        return Some(SystemAspect::Creator);
    }

    // **v4.3.4** — Birthdate aspect: "when were you born" / "when
    // did you appear". Pronoun gate optional because phrasings like
    // `туған күнің қашан` carry the addressee in the possessive
    // suffix.
    if joined.contains("қашан пайда болдың")
        || joined.contains("қашан пайда болдыңыз")
        || joined.contains("қашан жасалдың")
        || joined.contains("қашан жасалдыңыз")
        || joined.contains("қашан туылдың")
        || joined.contains("қашан туылдыңыз")
        || joined.contains("туған күнің")
        || joined.contains("туған күніңіз")
        || (has_addressee
            && (joined.contains("қашан жасады")
                || joined.contains("қашан құрды")
                || joined.contains("қашан жасап шығарды")
                // **v4.6.12** — additional creation-verb forms
                // mirroring the v4.6.5 Creator-aspect extension.
                // Real-REPL 2026-04-29: «Ал ол сені қашан
                // жаратты?» fell through pre-v4.6.12. Same
                // surface-level reasoning: `жаратты / дамытты /
                // дамытқан / дайындады` are common Kazakh
                // creation verbs that should pair with `қашан`
                // for the Birthdate aspect.
                || joined.contains("қашан жаратты")
                || joined.contains("қашан дамытты")
                || joined.contains("қашан дамытқан")
                || joined.contains("қашан дайындады")))
    {
        return Some(SystemAspect::Birthdate);
    }

    // **v4.3.4** — Architecture aspect: "how are you different" /
    // "what's special about you". Pronoun-led; the question targets
    // the system's distinguishing characteristics.
    if has_addressee
        && (joined.contains("ерекшелігің")
            || joined.contains("ерекшелігіңіз")
            || joined.contains("айырмашылығың")
            || joined.contains("айырмашылығыңыз")
            || joined.contains("неге басқашасың")
            || joined.contains("неге басқашасыз")
            || joined.contains("неге басқа модельдерден ерекшеленесің")
            || joined.contains("неге басқа модельдерден ерекшеленесіз")
            || joined.contains("қалай ерекшеленесің")
            || joined.contains("қалай ерекшеленесіз"))
    {
        return Some(SystemAspect::Architecture);
    }

    // **v4.6.0** — Capabilities aspect: "what can you do?".
    // Pronoun-led OR `сені/сізді`-marked OR a 2sg/2pl ability-modal
    // copula (`істей аласың / істей аласыз`). The verb form
    // `аласың / аласыз` is itself the addressee marker even without
    // a free-standing pronoun (the morpheme `-сың/-сыз` is 2nd
    // person), so the pronoun gate is loosened here. Real-REPL
    // 2026-04-29 transcript: «Не істей аласың?» (no leading pronoun).
    // **v4.12.0** — Implementation aspect. Surface forms:
    // «сіз қандай тілде жазылғансыз?», «не тілінде жасалғансың?»,
    // «қандай бағдарламалау тілінде жазылған?», «архитектурада не
    // тілі қолданылған?». Distinct from `Architecture` ("how are
    // you different?") and `SelfComparison` ("trade-off vs other
    // models"): this asks the literal "what programming language /
    // stack are you written with?". Closes the v4.11.7 known gap.
    // Runs BEFORE Capabilities (which has overlap on `жазылған`-
    // suffix forms).
    let implementation_marker = joined.contains("қандай тілде жазылған")
        || joined.contains("қандай тілде жазылғансың")
        || joined.contains("қандай тілде жазылғансыз")
        || joined.contains("не тілінде жазылған")
        || joined.contains("не тілінде жасалғансың")
        || joined.contains("не тілінде жасалғансыз")
        || joined.contains("қандай бағдарламалау тілінде жазылған")
        || joined.contains("қандай бағдарламалау тілінде жазылғансың")
        || joined.contains("қандай бағдарламалау тілінде жазылғансыз")
        || joined.contains("кодыңыз қай тілде")
        || joined.contains("коды қай тілде")
        || joined.contains("қандай тілде жасалғансыз")
        || joined.contains("қандай тілде жасалғансың");
    if implementation_marker {
        return Some(SystemAspect::Implementation);
    }

    let capabilities_marker = joined.contains("істей аласың")
        || joined.contains("істей аласыз")
        || joined.contains("қолыңнан не келеді")
        || joined.contains("қолыңыздан не келеді")
        || joined.contains("мүмкіндіктерің")
        || joined.contains("мүмкіндіктеріңіз")
        || (has_addressee
            && (joined.contains("не істей аласың")
                || joined.contains("не істей аласыз")
                || joined.contains("қандай мүмкіндіктер")))
        // **v4.11.7** — language-capability questions. Real-REPL
        // 2026-04-30: «Сіз қазақша білесіз бе?» pre-v4.11.7
        // returned "Түсінбедім." because no detector matched the
        // {language}-ша + 2nd-person-knowledge-verb pattern.
        // Routes to Capabilities aspect because
        // `capabilities_summary` already states «Қазақ тілінде
        // сөйлесе аламын». Pattern: closed list of common
        // language adverbs (`қазақша / орысша / ағылшынша /
        // түрікше`) paired with `білесің / білесіз / сөйлей
        // аласың / сөйлей аласыз / түсінесің / түсінесіз`.
        || joined.contains("қазақша білесің")
        || joined.contains("қазақша білесіз")
        || joined.contains("қазақша сөйлей аласың")
        || joined.contains("қазақша сөйлей аласыз")
        || joined.contains("қазақша түсінесің")
        || joined.contains("қазақша түсінесіз")
        || joined.contains("орысша білесің")
        || joined.contains("орысша білесіз")
        || joined.contains("орысша сөйлей аласың")
        || joined.contains("орысша сөйлей аласыз")
        || joined.contains("ағылшынша білесің")
        || joined.contains("ағылшынша білесіз")
        || joined.contains("ағылшынша сөйлей аласың")
        || joined.contains("ағылшынша сөйлей аласыз")
        || joined.contains("түрікше білесің")
        || joined.contains("түрікше білесіз");
    if capabilities_marker {
        return Some(SystemAspect::Capabilities);
    }

    // **v4.6.0** — Limitations aspect: "what CAN'T you do?". Same
    // 2nd-person ability marker but in the negative — `алмайсың /
    // алмайсыз`. Gated on an explicit interrogative marker
    // (question pronouns / particles / question mark) so a
    // declarative criticism «сен ештеңе білмейсің» (= "you know
    // nothing") doesn't accidentally route here — that's user
    // venting, not a query about limitations. The cognitive eval
    // scenario `qysqasy_discourse_particle_does_not_capture_topic`
    // pinned this distinction at v4.4.10 and the gate keeps it.
    let has_interrogative = joined.contains("?")
        || joined.contains("не ")
        || joined.ends_with("не")
        || joined.contains("нені")
        || joined.contains("қандай")
        || joined.contains("қалай")
        || joined.contains(" бе")
        || joined.ends_with("бе")
        || joined.contains(" ма")
        || joined.ends_with("ма");
    let limitations_marker_active = joined.contains("істей алмайсың")
        || joined.contains("істей алмайсыз")
        || joined.contains("шектеулерің")
        || joined.contains("шектеулеріңіз")
        || joined.contains("әлсіз тұстарың")
        || joined.contains("әлсіз тұстарыңыз")
        || joined.contains("несің әлсіз")
        || joined.contains("несіз әлсіз");
    let limitations_marker_passive = joined.contains("білмейсің") || joined.contains("білмейсіз");
    if limitations_marker_active || (limitations_marker_passive && has_interrogative) {
        return Some(SystemAspect::Limitations);
    }

    // **v4.6.0** — Knowledge aspect: "what do you know?". Surface
    // forms: `не білесің / не білесіз`, `қандай тақырыптар / қандай
    // салалар жайлы білесің`. Note: bare `Қазақстан туралы не
    // білесіз` should NOT route here (that's an Unknown topic
    // query about қазақстан). The disambiguator: `не білесің` /
    // `не білесіз` as standalone or with general scope qualifiers
    // (`қандай / жалпы`), but if there's a content noun + туралы
    // before it, fall through to the Unknown path.
    let knowledge_general_marker = (joined.contains("не білесің") || joined.contains("не білесіз"))
        && !joined.contains("туралы")
        && !joined.contains("жайында")
        && !joined.contains("жөнінде");
    let knowledge_explicit_marker = joined.contains("қандай салаларды білесің")
        || joined.contains("қандай салаларды білесіз")
        || joined.contains("қандай тақырыптар")
        || joined.contains("қандай тақырыптарды")
        || joined.contains("білімің не")
        || joined.contains("білімің қандай")
        || joined.contains("білімініз қандай");
    if knowledge_general_marker || knowledge_explicit_marker {
        return Some(SystemAspect::Knowledge);
    }

    // **v4.6.5** — Principles aspect: "what are your principles?".
    // Surface forms: `принциптерің / ұстанымдарың / заңдарың /
    // ережелерің / құндылықтарың` plus an interrogative qualifier
    // (`не`, `қандай`). The articulation layer — adam states the
    // values it upholds (respect humans, no fabrication, no
    // incitement, privacy, no illegal assistance, audit trail,
    // Kazakh-cultural respect, scope discipline). The underlying
    // guarantees are already safe-by-construction; this aspect
    // makes the value contract discoverable.
    let principles_marker = joined.contains("принциптерің")
        || joined.contains("принциптеріңіз")
        || joined.contains("ұстанымдарың")
        || joined.contains("ұстанымдарыңыз")
        || joined.contains("заңдарың")
        || joined.contains("заңдарыңыз")
        || joined.contains("ережелерің")
        || joined.contains("ережелеріңіз")
        || joined.contains("құндылықтарың")
        || joined.contains("құндылықтарыңыз");
    if principles_marker {
        return Some(SystemAspect::Principles);
    }

    // **v4.6.20** — SelfComparison aspect: "how are you
    // better/different from other AI models?". Real-REPL
    // 2026-04-29 transcript surfaced two phrasings —
    // «Басқа жасанды интеллект модельдерінен несімен артықсыз?»
    // and «… қалай жақсырақ бола аласыз?». Pre-v4.6.20 these fell
    // through to the greedy noun-hint path which grabbed
    // `басқа` / `қолданыс` and quoted random corpus material.
    // The detector lives in `discourse.rs` as a pair-signal
    // (comparison marker + addressee anchor); routing here
    // makes the planner pick the dedicated `self_comparison`
    // family that articulates the trade-off honestly.
    if crate::discourse::input_is_self_comparison_question(joined) {
        return Some(SystemAspect::SelfComparison);
    }

    // **v4.13.5** — Multi-topic capability marker. Pattern: 3+
    // comma-separated content nouns + `және` (Kazakh "and") + a
    // capability verb (`білесің / білесіз`). 2026-05-01 live REPL:
    // «Сіз математика, физика, химия, биология, астрономия және
    // тағы басқа пәндер бойынша мектептегі біліміңізді білесіз бе?»
    // pre-v4.13.5 grabbed `мектеп` as topic and surfaced an IsA
    // fact. The honest answer is: adam has surface-level
    // understanding of these subjects (knowledge_summary covers
    // them at the domain level) but no school-curriculum-level
    // depth. Route to the dedicated `multi_topic_capability`
    // template family. Detection requires:
    //   - 2+ commas (signals a list of nouns)
    //   - `және` (the canonical "and" connector)
    //   - one of the capability verbs (`білесің / білесіз` —
    //     other capability verbs like `сөйлей аласыз` are too
    //     specific to be safe to fire on a random list)
    // **Important**: `joined` strips punctuation; commas are gone by
    // the time we reach this detector. Use `raw_input` (lower-cased
    // for case-insensitive matching) for the comma-count, but
    // `joined` for the textual markers since the rest of this
    // function is `joined`-based.
    let raw_lower = raw_input.to_lowercase();
    let comma_count = raw_lower.matches(',').count();
    let multi_topic_capability = comma_count >= 2
        && raw_lower.contains("және")
        && (joined.contains("білесің") || joined.contains("білесіз"));
    if multi_topic_capability {
        return Some(SystemAspect::MultiTopicCapability);
    }

    // **v4.13.5** — Generic verb-capability marker. Pattern:
    // `<verb-converb> ала<person-suffix> <ма/ба/па>?` for any verb
    // OTHER than the language-capability ones already handled
    // above. 2026-05-01 live REPL: «Сіз оны бағдарламалай аласыз
    // ба, әлі жоқ па?» — pre-v4.13.5 the language-capabilities
    // detector required a {language adverb} prefix, so this fell
    // through to greedy retrieval and surfaced poetry. v4.13.5
    // catches the broader pattern: ANY converb + `ала+person` +
    // question particle = capability question on the verb. The
    // honest answer: «Жоқ, мен ондай әрекетті орындай алмаймын —
    // мен тілдік модельмін» (preserves the v4.6.0 trust contract
    // that adam doesn't pretend to do things it can't).
    //
    // Surface forms that signal the "ала + person" auxiliary:
    //   аласың / аласыз / алады / ала ма / ала ме
    // Combined with a question particle (бе/ма/ба/па + alternants)
    // OR a question mark.
    let aux_capability = joined.contains("аласың ба")
        || joined.contains("аласыз ба")
        || joined.contains("аласың ма")
        || joined.contains("аласыз ма")
        || joined.contains("ала ма?")
        || joined.contains("ала ме?")
        || joined.contains("алады ма")
        || joined.contains("алады ме");
    if aux_capability {
        return Some(SystemAspect::GenericCapability);
    }

    // **v4.3.3** — General aspect: pronoun-led identity question.
    // **v4.6.0** — Also fires on `өзіңіз туралы айт` style requests
    // (compound self-introduction openers from a 2026-04-29 real-
    // REPL transcript: «Өзіңіз туралы айтып беріңізші, …»). The
    // marker `өзің / өзіңіз` + `туралы` + speech-act verb (айт /
    // айтып бер / таныстыр) is unambiguous self-reference.
    let self_intro_request = (joined.contains("өзің туралы")
        || joined.contains("өзіңіз туралы")
        || joined.contains("өзің жайлы")
        || joined.contains("өзіңіз жайлы"))
        && (joined.contains("айт")
            || joined.contains("таныс")
            || joined.contains("берші")
            || joined.contains("беріңіз"));
    // **v4.6.20** — reflexive identity questions where the user
    // asks adam to *describe itself* in 2nd-person reflexive form:
    // «Өзіңді кім деп санайсың?» / «Өзіңізді кім деп санайсыз?» /
    // «Өзіңді қалай таныстырасың?» / «Өзіңізді қалай көресіз?». The
    // marker is `өзіңді / өзіңізді` (reflexive accusative) plus a
    // 2nd-person verb. Real-REPL 2026-04-29 fell through to a
    // misclassification ("user wants to talk about themselves").
    let reflexive_self_question = (joined.contains("өзіңді") || joined.contains("өзіңізді"))
        && (joined.contains("санайсың")
            || joined.contains("санайсыз")
            || joined.contains("таныстырасың")
            || joined.contains("таныстырасыз")
            || joined.contains("көресің")
            || joined.contains("көресіз")
            || joined.contains("қалай атайсың")
            || joined.contains("қалай атайсыз")
            || joined.contains("кім дейсің")
            || joined.contains("кім дейсіз"));
    if pronoun
        && (joined.contains("кімсің")
            || joined.contains("кімсіз")
            || joined.contains("қандай моделсің")
            || joined.contains("қандай моделсіз")
            || joined.contains("қандай ботсың")
            || joined.contains("қандай ботсыз")
            || joined.contains("қандай жасанды интеллектсің")
            || joined.contains("қандай жасанды интеллектсіз")
            || joined.contains("немен айналысасың")
            || joined.contains("немен айналысасыз"))
        || self_intro_request
        || reflexive_self_question
    {
        return Some(SystemAspect::General);
    }

    None
}

fn detect_ask_name(joined: &str) -> bool {
    (joined.contains("атың") && joined.contains("кім"))
        || (joined.contains("атыңыз") && joined.contains("кім"))
        || joined.contains("есімің")
        || joined.contains("есіміңіз")
        // v4.4.9 — 1sg self-recall form: "менің атым кім?",
        // "есімім кім / не / қандай?". Mirrors the v4.4.5 fix to
        // `detect_ask_age` for `менің жасым қанша?` and the v4.4.6
        // fix to `detect_ask_occupation` for `менің мамандығым не?`.
        // Pre-v4.4.9 the 1sg-possessive `атым` matched
        // `detect_statement_of_name`'s pattern 1 ("атым X") and
        // grabbed the literal `кім` as the user's name, then logged
        // a phantom contradiction (Дәулет vs Кім) the next time the
        // session already had a real name. The asymmetric guard in
        // `detect_statement_of_name` (refuses interrogative
        // pronouns as names) is the actual bug fix; this extension
        // routes the 1sg-self-recall question to `Intent::AskName`
        // so it answers from session storage via
        // `ask_name.with_known_user`.
        || (joined.contains("атым")
            && (joined.contains("кім") || joined.contains("не")))
        || (joined.contains("есімім")
            && (joined.contains("кім") || joined.contains("не")))
}

fn detect_statement_of_wellbeing(tokens: &[String], joined: &str) -> bool {
    let wellbeing_token = tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "жақсымын" | "жаманмын" | "жақсы" | "жаман" | "дұрысмын"
        )
    });
    wellbeing_token || joined.contains("жаман емес")
}

// --- v0.8.0 new recognisers ------------------------------------------------

/// User introduces self. Kazakh patterns only (v1.1.0):
///   1. [менің] атым <NAME>
///   2. мені <NAME> деп атайды
///   3. есімім <NAME>
///
/// Returns the extracted name (case-preserved, first-letter title-
/// cased) or `None` when none of the three patterns fire.
fn detect_statement_of_name(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
) -> Option<String> {
    // v4.4.9 — interrogative-pronoun guard. The 1sg-possessive
    // forms `атым` / `есімім` collide with the user asking about
    // their OWN stored name: `менің атым кім?` lexes as
    // `[менің, атым, кім, ?]`, pattern 1 below ("атым X" → name
    // is the next token) would grab the literal `Кім` as a name
    // and — once a real name was already stored — log a phantom
    // `BeliefConflict` (Дәулет vs Кім) followed by a clarifying
    // question that asked the user to pick between their actual
    // name and the question word. Refuse the match when the
    // candidate is an interrogative pronoun. Mirror of v4.4.5
    // `detect_statement_of_age` question-particle guard.
    let is_interrogative_pronoun = |t: &str| {
        let lower = t.to_lowercase();
        matches!(
            lower.as_str(),
            "кім" | "кiм" | "не" | "қандай" | "қайсысы" | "ким"
        )
    };

    // Pattern 1: "атым X".
    if let Some(i) = tokens.iter().position(|t| t == "атым") {
        if let Some(name) = raw_tokens.get(i + 1) {
            if is_interrogative_pronoun(name) {
                return None;
            }
            return Some(normalize_proper_noun(name));
        }
    }
    // Pattern 3: "есімім X".
    if let Some(i) = tokens.iter().position(|t| t == "есімім") {
        if let Some(name) = raw_tokens.get(i + 1) {
            if is_interrogative_pronoun(name) {
                return None;
            }
            return Some(normalize_proper_noun(name));
        }
    }
    // Pattern 2: "мені X деп атайды".
    if joined.contains("деп атайды") {
        if let (Some(start), Some(end)) = (
            tokens.iter().position(|t| t == "мені"),
            tokens.iter().position(|t| t == "деп"),
        ) {
            if end > start + 1 {
                if let Some(name) = raw_tokens.get(start + 1) {
                    if is_interrogative_pronoun(name) {
                        return None;
                    }
                    return Some(normalize_proper_noun(name));
                }
            }
        }
    }
    None
}

fn detect_ask_age(joined: &str) -> bool {
    let has_q = joined.contains("неше") || joined.contains("қанша");
    (joined.contains("жасың") && has_q)
        || (joined.contains("жасыңыз") && has_q)
        || joined.contains("қанша жастасың")
        || joined.contains("қанша жастасыз")
        // **v4.6.12** — `неше` (alongside `қанша`) variant of the
        // adessive-copula age question. Real-REPL 2026-04-29:
        // «Сіз неше жастасыз?» (= "how many years old are you,
        // polite") fell through to topic-extraction on `неше`
        // pre-v4.6.12, surfaced a tangential proverb. Adding the
        // `неше жастасың / неше жастасыз` patterns. Also catches
        // adam-self age questions: with no session.age, the path
        // falls through to the `ask_age` family («менің жасым
        // адамзат жасындай», «мен әлі жаспын») which is the
        // correct response for system-self age inquiries.
        || joined.contains("неше жастасың")
        || joined.contains("неше жастасыз")
        // **v4.11.7** — verb-form variants of the age question.
        // `жасайсың / жасайсыз` (= 2nd-person of `жасау` "to live")
        // is colloquial Kazakh for "how old are you?" alongside the
        // adessive `жастасың / жастасыз`. Live REPL 2026-04-30:
        // «Қанша жасайсыз?» pre-v4.11.7 returned "Түсінбедім."
        // because the existing `жастасың/жастасыз` patterns required
        // the adessive form. Adds the four `қанша/неше + жасайсың/
        // жасайсыз` permutations; pairs with v4.6.12's adessive set.
        || joined.contains("қанша жасайсың")
        || joined.contains("қанша жасайсыз")
        || joined.contains("неше жасайсың")
        || joined.contains("неше жасайсыз")
        // v4.4.5 — 1sg self-recall form: "менің жасым қанша?" /
        // "жасым неше?". Pre-v4.4.5 this matched
        // `detect_statement_of_age` (keyed on `жасым`) and emitted
        // a confirmation template that interpolated session.age
        // — coincidentally correct for the recorded value but
        // wrong for any phrasing where the template doesn't echo
        // the slot, and semantically misclassifying the user's
        // question as a statement.
        || (joined.contains("жасым") && has_q)
}

/// User reports age: "менің жасым N", "N жастамын", "N жасында".
/// Returns `Some(Some(n))` when the pattern matched AND a Kazakh
/// numeral 1–99 was parsed, `Some(None)` when the pattern matched but
/// no numeral was found, and `None` when the pattern didn't match at
/// all (so the caller can continue dispatching).
fn detect_statement_of_age(tokens: &[String], joined: &str) -> Option<Option<u32>> {
    let matched = joined.contains("жасым")
        || tokens
            .iter()
            .any(|t| t == "жастамын" || t == "жастаймын" || t == "жаспын");
    if !matched {
        return None;
    }
    // v4.4.5 — guard: a question particle (`қанша`/`неше`) flips
    // the polarity from statement → question. `detect_ask_age`
    // runs first now, but this defends against any future caller
    // ordering and keeps the matcher honest in isolation.
    if joined.contains("қанша") || joined.contains("неше") {
        return None;
    }
    Some(parse_kazakh_age(tokens))
}

/// Parse a Kazakh numeral in the range 1–99 out of a token stream.
/// Supports compound forms ("отыз бес" = 35) and bare tens/units, as
/// well as literal digit strings ("30"). Returns the first hit.
fn parse_kazakh_age(tokens: &[String]) -> Option<u32> {
    for (i, t) in tokens.iter().enumerate() {
        // Literal digit form, e.g. "30".
        if let Ok(n) = t.parse::<u32>() {
            if (1..200).contains(&n) {
                return Some(n);
            }
        }
        // Tens word, maybe followed by a unit word.
        if let Some(tens) = kazakh_tens_value(t) {
            if let Some(next) = tokens.get(i + 1) {
                if let Some(units) = kazakh_units_value(next) {
                    return Some(tens + units);
                }
            }
            return Some(tens);
        }
        // Bare unit word (rare for ages but handle it).
        if let Some(units) = kazakh_units_value(t) {
            return Some(units);
        }
    }
    None
}

fn kazakh_tens_value(token: &str) -> Option<u32> {
    match token {
        "он" => Some(10),
        "жиырма" => Some(20),
        "отыз" => Some(30),
        "қырық" => Some(40),
        "елу" => Some(50),
        "алпыс" => Some(60),
        "жетпіс" => Some(70),
        "сексен" => Some(80),
        "тоқсан" => Some(90),
        _ => None,
    }
}

fn kazakh_units_value(token: &str) -> Option<u32> {
    match token {
        "бір" => Some(1),
        "екі" => Some(2),
        "үш" => Some(3),
        "төрт" => Some(4),
        "бес" => Some(5),
        "алты" => Some(6),
        "жеті" => Some(7),
        "сегіз" => Some(8),
        "тоғыз" => Some(9),
        _ => None,
    }
}

fn detect_ask_location(joined: &str) -> bool {
    joined.contains("қай жерден")
        || joined.contains("қайдан")
        || joined.contains("қайда тұра")
        || joined.contains("қай қала")
        || joined.contains("қай аудан")
}

/// User states location: "мен Алматыданмын", "астанада тұрамын",
/// "ауылдан келдім". Returns `Some(city)` when the pattern fires,
/// with `city` being the extracted root — or `None` inside `Some`
/// when the pattern fires without a parseable city token.
///
/// v1.4.0 FST-NER primary path: scan the FST `parses` for a
/// Noun in Ablative or Locative case (possibly with P1Sg predicate
/// stacked). If found, its root is the city. Rule-based string-
/// stripping stays as fallback for words not in the Lexicon.
fn detect_statement_of_location(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
    parses: &[Analysis],
) -> Option<Option<String>> {
    use adam_kernel_fst::morphotactics::Case;

    if !has_first_person_location_context(tokens, joined, parses) {
        return None;
    }

    // Primary: look for a parsed Noun in Ablative or Locative case.
    // Prefer Ablative (stronger signal for origin: "X-дан+мын") over
    // bare Locative. Also accept Locative if co-occurring with
    // "тұрамын / тұрамыз".
    //
    // **v4.0.1** — Codex v4.0.0 review caught that «Неліктен?»
    // («why?» — interrogative) was parsed by the FST as `Нелік` +
    // Ablative, so this detector returned `StatementOfLocation { city:
    // "Нелік" }` and the REPL replied with «Нелікте тұрасыз ба» («Do
    // you live in Нелік?»). The v3.9.5 `NOT_A_TOPIC` sync only
    // filtered `first_noun_root` / `content_roots` — it never touched
    // this detector. Fix: skip any noun whose root is in `NOT_A_TOPIC`
    // (interrogatives, demonstratives, closed-class function words)
    // at the case-scan step. A legitimate city root is never in
    // `NOT_A_TOPIC`; an interrogative is.
    let mut ablative_root: Option<String> = None;
    let mut locative_root: Option<String> = None;
    for p in parses {
        if let Analysis::Noun { root, features } = p {
            if NOT_A_TOPIC.contains(&root.root.as_str()) {
                continue;
            }
            match features.case {
                Some(Case::Ablative) if ablative_root.is_none() => {
                    ablative_root = Some(normalize_place_name(&root.root));
                }
                Some(Case::Locative) if locative_root.is_none() => {
                    locative_root = Some(normalize_place_name(&root.root));
                }
                _ => {}
            }
        }
    }
    if let Some(c) = ablative_root {
        if generic_place_root(&c) {
            if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens) {
                return Some(Some(name));
            }
        }
        return Some(Some(c));
    }
    // v1.8.5: if a Noun stacks Locative + P1Sg ("Алматыдамын" = "I am
    // in Almaty"), that's a location statement on its own — no need for
    // a separate "тұрамын" verb. Without this branch, Locative+P1Sg
    // falls through to `detect_statement_of_occupation` and "Алматы"
    // gets miscategorised as an occupation.
    use adam_kernel_fst::morphotactics::Predicate;
    for p in parses {
        if let Analysis::Noun { root, features } = p {
            if NOT_A_TOPIC.contains(&root.root.as_str()) {
                continue;
            }
            if features.case == Some(Case::Locative) && features.predicate == Some(Predicate::P1Sg)
            {
                return Some(Some(normalize_place_name(&root.root)));
            }
        }
    }
    let live_verb = tokens.iter().any(|t| t == "тұрамын" || t == "тұрамыз");
    if live_verb {
        if let Some(c) = locative_root {
            if generic_place_root(&c) {
                if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens)
                {
                    return Some(Some(name));
                }
            }
            return Some(Some(c));
        }
    }

    // Fallback for out-of-lexicon inputs: string-based heuristics.
    detect_statement_of_location_rule_based(tokens, raw_tokens, joined)
}

fn has_first_person_location_context(tokens: &[String], joined: &str, parses: &[Analysis]) -> bool {
    use adam_kernel_fst::morphotactics::{Number, Person, Predicate};

    if tokens.iter().any(|t| t == "мен" || t == "біз") {
        return true;
    }
    if tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "тұрамын" | "тұрамыз" | "келдім" | "келдік" | "тұрмын" | "тұрмыз"
        )
    }) {
        return true;
    }
    if tokens.iter().any(|t| strip_ablative_copula(t).is_some()) {
        return true;
    }
    if tokens.iter().any(|t| strip_locative_copula(t).is_some()) {
        return true;
    }
    if joined.contains("данмын")
        || joined.contains("денмін")
        || joined.contains("танмын")
        || joined.contains("тенмін")
    {
        return true;
    }
    parses.iter().any(|p| match p {
        Analysis::Noun { features, .. } => {
            matches!(features.predicate, Some(Predicate::P1Sg | Predicate::P1Pl))
        }
        Analysis::Verb { features, .. } => {
            features.person == Some(Person::First)
                && matches!(
                    features.number,
                    None | Some(Number::Singular) | Some(Number::Plural)
                )
        }
    })
}

fn generic_place_root(root: &str) -> bool {
    matches!(
        root.to_lowercase().as_str(),
        "ауыл" | "қала" | "аудан" | "облыс" | "өңір" | "кент" | "ел"
    )
}

fn recover_named_place_before_generic_location(
    tokens: &[String],
    raw_tokens: &[String],
) -> Option<String> {
    for i in 1..tokens.len() {
        if !token_mentions_generic_place(&tokens[i]) {
            continue;
        }
        let prev_raw = raw_tokens.get(i - 1)?;
        let prev_token = tokens.get(i - 1)?;
        if NOT_A_TOPIC.contains(&prev_token.as_str()) || generic_place_root(prev_token) {
            continue;
        }
        if raw_looks_like_named_place(prev_raw) {
            return Some(normalize_place_name(prev_raw));
        }
    }
    None
}

fn recover_named_place_before_origin_marker(
    tokens: &[String],
    raw_tokens: &[String],
) -> Option<String> {
    for i in 1..tokens.len() {
        let marker = tokens[i].as_str();
        if !matches!(
            marker,
            "жақтанмын" | "жақтанбыз" | "маңынанмын" | "маңынанбыз"
        ) {
            continue;
        }
        let prev_raw = raw_tokens.get(i - 1)?;
        let prev_token = tokens.get(i - 1)?;
        if i >= 2 && token_mentions_geo_descriptor(&tokens[i - 1]) {
            let prev_prev_raw = raw_tokens.get(i - 2)?;
            let prev_prev_token = tokens.get(i - 2)?;
            if !NOT_A_TOPIC.contains(&prev_prev_token.as_str())
                && !generic_place_root(prev_prev_token)
                && raw_looks_like_named_place(prev_prev_raw)
            {
                let phrase = format!("{} {}", prev_prev_raw, prev_raw);
                return Some(normalize_place_name(&phrase));
            }
        }
        if NOT_A_TOPIC.contains(&prev_token.as_str()) || generic_place_root(prev_token) {
            continue;
        }
        if raw_looks_like_named_place(prev_raw) {
            return Some(normalize_place_name(prev_raw));
        }
    }
    None
}

/// **v4.3.2 — fix: prefix match, not substring match.**
///
/// Pre-v4.3.2 this used `token.contains(stem)`. The 2-letter stem
/// `ел` (country) is incidentally a substring of common modern
/// Kazakh tokens — `интеллект`, `келдім`, `белгі`, `сенделді`, etc.
/// — and produced a false positive that propagated up through
/// `recover_named_place_before_generic_location`, mis-extracting
/// the *preceding* word as a city. Concrete failure mode (real
/// dialog test): the input
///
///   «Мен жаңа жасанды интеллект моделін әзірлейтін бағдарламашымын»
///
/// matched `token.contains("ел")` on `интеллект`, so the recoverer
/// promoted `жасанды` to a city, the belief layer logged
/// `(USER, city, Жасанды)` against `(USER, city, Атырау)`, the
/// planner went into a permanent `CheckContradiction` for every
/// subsequent topic question, and the dialog became unrecoverable.
///
/// Switching to `starts_with` keeps every real word-formation
/// pattern that mentions a generic place (`қалада`, `ауылдан`,
/// `елде`, `елден`, `өңірде`) and rejects intra-word substring
/// matches. Validated by a regression test that re-runs the exact
/// failing dialog turn.
fn token_mentions_generic_place(token: &str) -> bool {
    ["ауыл", "қала", "аудан", "облыс", "өңір", "кент", "ел"]
        .iter()
        .any(|stem| token.starts_with(stem))
}

/// **v4.3.2 — same fix as `token_mentions_generic_place`** for the
/// wider geo-descriptor set used by
/// `recover_named_place_before_origin_marker`. Same false-positive
/// risk, same prefix-match resolution.
fn token_mentions_geo_descriptor(token: &str) -> bool {
    [
        "ауыл",
        "қала",
        "аудан",
        "облыс",
        "өңір",
        "кент",
        "ел",
        "өзен",
        "көл",
        "теңіз",
        "тау",
    ]
    .iter()
    .any(|stem| token.starts_with(stem))
}

fn raw_looks_like_named_place(token: &str) -> bool {
    looks_like_named_place_candidate(token)
}

/// Pre-v1.4.0 string-based heuristic retained as fallback when the FST
/// can't parse the form (e.g. the city isn't in the Lexicon yet).
fn detect_statement_of_location_rule_based(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
) -> Option<Option<String>> {
    // 1st-person "live" verb: `X-да/-де тұрамын` — the city is the
    // token ending in locative that precedes the verb.
    if let Some(verb_idx) = tokens.iter().position(|t| t == "тұрамын" || t == "тұрамыз")
    {
        if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens) {
            return Some(Some(name));
        }
        let city = (0..verb_idx)
            .rev()
            .find_map(|i| strip_locative(&tokens[i]).map(|_| raw_tokens[i].clone()))
            .map(|raw| strip_locative_preserving(&raw));
        return Some(city);
    }
    // Ablative + 1sg copula: "Алматыданмын" → "Алматы".
    if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens) {
        for token in tokens {
            if token_mentions_generic_place(token) {
                return Some(Some(name));
            }
        }
    }
    if let Some(name) = recover_named_place_before_origin_marker(tokens, raw_tokens) {
        return Some(Some(name));
    }
    for (i, t) in tokens.iter().enumerate() {
        if let Some(root) = strip_locative_copula(t) {
            let raw = raw_tokens
                .get(i)
                .map(|r| strip_locative_copula_preserving(r).unwrap_or_else(|| root.clone()))
                .unwrap_or(root);
            return Some(Some(normalize_place_name(&raw)));
        }
        if let Some(root) = strip_ablative_copula(t) {
            let raw = raw_tokens
                .get(i)
                .map(|r| strip_ablative_copula_preserving(r).unwrap_or_else(|| root.clone()))
                .unwrap_or(root);
            return Some(Some(normalize_place_name(&raw)));
        }
    }
    // "келдім" + ауыл/қала somewhere — matched but no precise city.
    if joined.contains("келдім") && (joined.contains("ауыл") || joined.contains("қала"))
    {
        return Some(None);
    }
    None
}

/// Strip a trailing locative suffix (`-да/-де/-та/-те`) returning the
/// root. Lower-case variant; returns `None` when the suffix isn't
/// present.
fn strip_locative(token: &str) -> Option<String> {
    for suffix in ["да", "де", "та", "те"] {
        if token.ends_with(suffix) && token.chars().count() > suffix.chars().count() + 1 {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

/// Case-preserving version used on `raw_tokens` so proper nouns keep
/// their surface capitalisation.
fn strip_locative_preserving(token: &str) -> String {
    for suffix in ["да", "де", "та", "те", "Да", "Де", "Та", "Те"] {
        if token.ends_with(suffix) && token.chars().count() > suffix.chars().count() + 1 {
            let take = token.chars().count() - suffix.chars().count();
            return token.chars().take(take).collect();
        }
    }
    token.to_string()
}

/// Strip ablative + 1sg copula: `-данмын / -денмін / -танмын / -тенмін`.
/// Requires a root stem of at least 3 characters — prevents a greedy
/// match on short words like "наданмын" ("I am ignorant") leaving
/// stem "на" that would be misrecognised as a city.
fn strip_ablative_copula(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    for suffix in ["данмын", "денмін", "танмын", "тенмін"] {
        if token.ends_with(suffix) && token.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn strip_ablative_copula_preserving(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    let lower = token.to_lowercase();
    for suffix in ["данмын", "денмін", "танмын", "тенмін"] {
        if lower.ends_with(suffix) && lower.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn strip_locative_copula(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    for suffix in ["дамын", "демін", "тамын", "темін"] {
        if token.ends_with(suffix) && token.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn strip_locative_copula_preserving(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    let lower = token.to_lowercase();
    for suffix in ["дамын", "демін", "тамын", "темін"] {
        if lower.ends_with(suffix) && lower.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn detect_ask_occupation(joined: &str) -> bool {
    joined.contains("немен айналыс")
        || (joined.contains("жұмысың") && joined.contains("не"))
        || (joined.contains("жұмысыңыз") && joined.contains("не"))
        || joined.contains("кәсібің")
        || joined.contains("кәсібіңіз")
        || joined.contains("мамандығың")
        || joined.contains("мамандығыңыз")
        // v4.4.6 — 1sg self-recall form: "менің мамандығым не?",
        // "менің кәсібім қандай?". Mirrors the v4.4.5 fix to
        // `detect_ask_age` for `менің жасым қанша?`. Pre-v4.4.6
        // these fell through to `Intent::Unknown` with
        // `noun_hint = мамандық/кәсіп` and routed to
        // `unknown.with_grounded_fact`, surfacing a generic
        // definition («Мамандық — адамның кәсібі») instead of
        // recalling the user's stored value via
        // `ask_occupation.with_known_user`. The 1sg-possessive
        // morphemes (`-ым`/`-ім`) plus a question particle
        // (`не`/`қандай`) are an unambiguous self-recall signal.
        || ((joined.contains("мамандығым") || joined.contains("кәсібім"))
            && (joined.contains("не") || joined.contains("қандай")))
}

/// User states occupation.
///
/// Priority chain (v1.4.0):
/// 1. **FST parse**: any Noun with `predicate = P1Sg` in the parses is
///    the occupation. Root from the Analysis is returned.
/// 2. **Lexicon fallback**: strip 1sg-copula suffix + lookup noun POS.
///    Runs when parser didn't produce a Noun+P1Sg (e.g. word not in
///    Lexicon).
/// 3. **Fixed table**: final fallback with 6 hand-written forms for
///    callers that pass neither parses nor a lexicon.
/// 4. Bare "жұмыс істеймін" matches as occupation = None.
fn detect_statement_of_occupation(
    tokens: &[String],
    _raw_tokens: &[String],
    joined: &str,
    lexicon: Option<&LexiconV1>,
    parses: &[Analysis],
) -> Option<Option<String>> {
    use adam_kernel_fst::morphotactics::Predicate;

    // Priority 1 — FST parse with P1Sg predicate. Only accept real
    // nouns (POS-filtered) — the parser also returns adjective
    // analyses under Analysis::Noun, but "жақсымын" (adj жақсы +
    // P1Sg) is wellbeing, not an occupation.
    //
    // v1.8.5 guard: reject Locative / Ablative case on the noun.
    // "Алматыдамын" (loc+P1Sg) and "Алматыданмын" (abl+P1Sg) are
    // location statements ("I am in / from Almaty"), NOT occupation
    // statements — they're handled by `detect_statement_of_location`.
    use adam_kernel_fst::morphotactics::Case;
    for p in parses {
        if let Analysis::Noun { root, features } = p {
            if features.predicate == Some(Predicate::P1Sg)
                && root.part_of_speech == "noun"
                && !matches!(features.case, Some(Case::Locative) | Some(Case::Ablative))
            {
                return Some(Some(root.root.clone()));
            }
        }
    }

    // Priority 2 — Lexicon-backed copula-strip lookup.
    if let Some(lex) = lexicon {
        if let Some(root) = strip_copula_and_lookup_noun(tokens, lex) {
            return Some(Some(root));
        }
    } else {
        // Priority 3 — fixed table for no-lexicon callers.
        const OCCUPATIONS: &[(&str, &str)] = &[
            ("мұғаліммін", "мұғалім"),
            ("дәрігермін", "дәрігер"),
            ("студентпін", "студент"),
            ("инженермін", "инженер"),
            ("оқушымын", "оқушы"),
            ("жұмысшымын", "жұмысшы"),
        ];
        for t in tokens {
            for (form, root) in OCCUPATIONS {
                if t == form {
                    return Some(Some((*root).to_string()));
                }
            }
        }
    }
    if joined.contains("жұмыс істеймін") {
        return Some(None);
    }
    None
}

/// For every token ending in a Kazakh 1sg-copula suffix, strip the
/// suffix and check whether the residue is a `noun` in the lexicon.
/// Returns the first hit. Skips residues that are tagged as adjectives
/// (otherwise `"жақсымын"` — adj "жақсы" + 1sg — would falsely register
/// as an occupation statement).
fn strip_copula_and_lookup_noun(tokens: &[String], lex: &LexiconV1) -> Option<String> {
    const COPULA_SUFFIXES: &[&str] = &["мын", "мін", "пын", "пін", "бын", "бін"];
    for t in tokens {
        for suffix in COPULA_SUFFIXES {
            let Some(root) = strip_suffix_chars(t, suffix) else {
                continue;
            };
            // Minimum stem length of 2 chars — guards against stripping
            // short function words.
            if root.chars().count() < 2 {
                continue;
            }
            if let Some(entry) = lex.get(&root) {
                if entry.part_of_speech == "noun" {
                    return Some(root);
                }
            }
        }
    }
    None
}

/// Strip `suffix` from the end of `token` using Unicode-aware
/// character counting (avoids byte-slicing into a UTF-8 codepoint).
fn strip_suffix_chars(token: &str, suffix: &str) -> Option<String> {
    if !token.ends_with(suffix) {
        return None;
    }
    let take = token.chars().count() - suffix.chars().count();
    Some(token.chars().take(take).collect())
}

/// "Family question" — үйлендің бе, балаларың бар ма, отбасың бар ма.
fn detect_ask_family(joined: &str) -> bool {
    joined.contains("үйлендің")
        || joined.contains("үйлендіңіз")
        || (joined.contains("балаларың") && joined.contains("бар"))
        || (joined.contains("балаларыңыз") && joined.contains("бар"))
        || (joined.contains("отбасың") && joined.contains("бар"))
        || (joined.contains("отбасыңыз") && joined.contains("бар"))
}

/// User talks about family: "менің балам бар", "үйленгенмін",
/// "отбасым бар".
fn detect_statement_of_family(joined: &str) -> bool {
    joined.contains("балам бар")
        || joined.contains("балаларым бар")
        || joined.contains("үйленгенмін")
        || joined.contains("отбасым бар")
        || joined.contains("отбасым жақсы")
}

fn detect_ask_weather(joined: &str) -> bool {
    (joined.contains("ауа райы") && joined.contains("қалай"))
        || (joined.contains("бүгін") && joined.contains("ауа райы"))
        || (joined.contains("сыртта") && joined.contains("қалай"))
}

/// User describes weather: "бүгін суық", "жылы", "қар жауып тұр",
/// "күн ашық".
fn detect_statement_of_weather(tokens: &[String], joined: &str) -> bool {
    let weather_token = tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "суық" | "жылы" | "ыстық" | "салқын" | "жаңбырлы" | "қарлы"
        )
    });
    let weather_phrase = joined.contains("қар жауып")
        || joined.contains("жаңбыр жауып")
        || joined.contains("күн ашық")
        || joined.contains("ауа райы жақсы")
        || joined.contains("ауа райы жаман");
    // Guard: "жақсы" / "жаман" alone is wellbeing, not weather —
    // require the keyword OR the phrase, never bare goodness tokens.
    (weather_token && (joined.contains("бүгін") || joined.contains("қазір"))) || weather_phrase
}

fn detect_ask_time(joined: &str) -> bool {
    (joined.contains("сағат") && (joined.contains("неше") || joined.contains("қанша")))
        || joined.contains("қазір уақыт")
        || joined.contains("қандай күн")
        || joined.contains("қай күн")
}

fn detect_compliment(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "жарайсың" | "жарайсыз" | "керемет" | "тамаша" | "мықты"
        )
    }) || joined.contains("өте жақсы")
}

fn detect_request(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "өтінемін" | "сұраймын" | "көмектесіңізші" | "көмектесіңіз" | "көмектес"
        )
    }) || joined.contains("көмек керек")
}

fn detect_well_wishes(joined: &str) -> bool {
    joined.contains("жақсы күн тіле")
        || joined.contains("сәттілік")
        || joined.contains("табысты бол")
        || joined.contains("денсаулық тіле")
}

/// Insult / rudeness — polite non-engagement (v1.1.0).
/// The model doesn't escalate or retaliate; responds with dignity.
fn detect_insult(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "ақымақ"
                | "ақымақсың"
                | "ақымақсыз"
                | "надан"
                | "наданмын"
                | "надансың"
                | "өтірік"
        )
    }) || joined.contains("ақылсыз")
        || joined.contains("түкке тұрмайсың")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v3.9.5 regression: «Неліктен» / «Неге» / «Қашан» etc. must not be
    /// extracted as a topic-noun. Pre-v3.9.5 the REPL would reply with
    /// «Нелікте тұрасыз ба» (= "Do you live in Нелік?") to the input
    /// «Неліктен?» because the FST parsed it as `Нелік` + ablative and
    /// `NOT_A_TOPIC` did not include the interrogative.
    #[test]
    fn not_a_topic_covers_v3_9_5_additions() {
        // Interrogatives — primary fix.
        for word in [
            "неліктен",
            "неге",
            "қашан",
            "қайда",
            "қандай",
            "кім",
            "қанша",
        ] {
            assert!(
                NOT_A_TOPIC.contains(&word),
                "interrogative `{word}` must be in NOT_A_TOPIC"
            );
        }
        // Demonstrative qualifiers.
        for word in ["мұндай", "сондай", "ондай", "кейбір", "өз", "әрбір"]
        {
            assert!(
                NOT_A_TOPIC.contains(&word),
                "demonstrative `{word}` must be in NOT_A_TOPIC"
            );
        }
        // Content nouns still pass through the gate.
        for word in ["бала", "кітап", "мектеп", "қазақстан", "жер"] {
            assert!(
                !NOT_A_TOPIC.contains(&word),
                "content noun `{word}` must NOT be in NOT_A_TOPIC"
            );
        }
    }

    /// v4.0.21 — multi-word entity matcher returns the compound entity
    /// before the single-word fallback. Pre-v4.0.21 «Құс жолы туралы
    /// айтшы» (tell me about the Milky Way) tokenised to «құс» + «жолы»
    /// and the reply was about birds («құс»), losing the galaxy referent.
    #[test]
    fn multiword_entity_hint_matches_compound_entities() {
        assert_eq!(
            multiword_entity_hint("Құс жолы туралы айтшы"),
            Some("құс жолы".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Күн жүйесі өте кең"),
            Some("күн жүйесі".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Аспан денесі жайлы"),
            Some("аспан денесі".to_string())
        );
        // Inflected last-word: substring match still fires (Kazakh
        // agglutinates on the compound tail).
        assert_eq!(
            multiword_entity_hint("Құс жолының бейнесі"),
            Some("құс жолы".to_string())
        );
    }

    /// **v4.11.5** — query-time school-curriculum compounds are
    /// resolved to the canonical compound topic, regardless of
    /// inflection on the last word. Real-REPL 2026-04-30:
    /// «Адам, сен мектептің физика бағдарламасын білесің бе?»
    /// pre-v4.11.5 fell through to either `физика` (ignoring the
    /// program-of context) or — worse — `адам` (the vocative).
    #[test]
    fn multiword_entity_hint_matches_curriculum_compounds() {
        assert_eq!(
            multiword_entity_hint("мектептің физика бағдарламасын білесің бе?"),
            Some("физика бағдарламасы".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Сен мектеп пәндерін білесің бе?"),
            Some("мектеп пәндері".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Биология бағдарламасы туралы айтшы"),
            Some("биология бағдарламасы".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Информатика бағдарламасын білесің бе?"),
            Some("информатика бағдарламасы".to_string())
        );
    }

    /// v4.0.21 — no multi-word match ⇒ None, so the single-word fallback
    /// activates downstream.
    #[test]
    fn multiword_entity_hint_returns_none_for_simple_input() {
        assert_eq!(multiword_entity_hint("құс туралы айтшы"), None);
        assert_eq!(multiword_entity_hint("мектеп керек пе"), None);
        assert_eq!(multiword_entity_hint(""), None);
    }

    #[test]
    fn generic_location_phrase_recovers_named_place_prefix() {
        let got = interpret_text("Мен Қашар ауылында тұрамын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Қашар".into())
            }
        );
    }

    #[test]
    fn statement_of_name_normalises_lowercase_and_mixed_script() {
        let got = interpret_text("атым дӘУЛEТ", &[]);
        assert_eq!(
            got,
            Intent::StatementOfName {
                name: "Дәулет".into()
            }
        );
    }

    #[test]
    fn statement_of_location_recovers_lowercase_named_place_prefix() {
        let got = interpret_text("мен қашар ауылында тұрамын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Қашар".into())
            }
        );
    }

    #[test]
    fn statement_of_location_normalises_mixed_script_city() {
        let got = interpret_text("мен Aлматыданмын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Алматы".into())
            }
        );
    }

    #[test]
    fn statement_of_location_recovers_geo_feature_before_origin_marker() {
        let got = interpret_text("мен каспий жақтанмын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Каспий".into())
            }
        );
    }

    #[test]
    fn statement_of_location_normalises_curated_geo_alias() {
        let got = interpret_text("мен Алма-Атадамын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Алматы".into())
            }
        );
    }

    #[test]
    fn statement_of_location_recovers_geo_feature_phrase_before_origin_marker() {
        let got = interpret_text("мен каспий теңізі жақтанмын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Каспий".into())
            }
        );
    }

    /// v4.0.26 — Codex v4.0.23 residual: the v4.0.21 MULTIWORD_ENTITIES
    /// docstring referenced a `world_core_multiword_coverage_test` that
    /// didn't actually exist. This test closes that maintenance trap.
    ///
    /// It scans every JSONL entry in `data/world_core/` and asserts that
    /// every compound subject/object (value containing a space) appears
    /// in the `MULTIWORD_ENTITIES` const. Adding a new compound to
    /// world_core without extending the const will now fail CI.
    ///
    /// Skips silently when the data directory is absent (trimmed CI
    /// checkouts, external crate consumers). Production CI runs from
    /// the repo root so the data is always present.
    #[test]
    fn world_core_multiword_coverage() {
        let dir = std::path::Path::new("../../data/world_core");
        if !dir.exists() {
            eprintln!(
                "world_core_multiword_coverage: skipping — {} not present",
                dir.display()
            );
            return;
        }
        let mut observed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let entries = std::fs::read_dir(dir).expect("read_dir data/world_core");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let contents = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let Some(facts) = parsed.get("facts").and_then(|v| v.as_array()) else {
                    continue;
                };
                for fact in facts {
                    for key in ["subject", "object"] {
                        if let Some(value) = fact.get(key).and_then(|v| v.as_str()) {
                            if value.contains(' ') {
                                observed.insert(value.to_string());
                            }
                        }
                    }
                }
            }
        }
        let const_set: std::collections::BTreeSet<String> =
            MULTIWORD_ENTITIES.iter().map(|s| s.to_string()).collect();
        let missing: Vec<&String> = observed.difference(&const_set).collect();
        assert!(
            missing.is_empty(),
            "world_core has {} compound entities not in MULTIWORD_ENTITIES; add them to the const in semantics.rs: {missing:?}",
            missing.len()
        );
    }
}
