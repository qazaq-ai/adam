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
    let tokens: Vec<String> = input
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect();
    let raw_tokens: Vec<String> = input
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
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
    if detect_ask_how_are_you(&joined) {
        return Intent::AskHowAreYou;
    }
    if detect_ask_name(&joined) {
        return Intent::AskName;
    }
    // Statement-* is checked BEFORE Ask-* inside each topic pair: a
    // 1st-person marker ("келдім", "тұрамын", "жасым") unambiguously
    // means the user is stating, not asking. Without this ordering,
    // "қайдан келдім" would hit AskLocation (because of "қайдан")
    // before StatementOfLocation (which keys on "келдім").
    if let Some(years) = detect_statement_of_age(&tokens, &joined) {
        return Intent::StatementOfAge { years };
    }
    if detect_ask_age(&joined) {
        return Intent::AskAge;
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
    let noun_hint = first_noun_root(parses);
    Intent::Unknown {
        raw_tokens: tokens,
        noun_hint,
        example: None,
        example_adapted: false,
        reasoning_chain: None,
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

    Intent::Unknown {
        raw_tokens: tokens,
        noun_hint: first_noun_root(parses),
        example: None,
        example_adapted: false,
        reasoning_chain: None,
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
        || joined == "қалың қалай"
}

fn detect_ask_name(joined: &str) -> bool {
    (joined.contains("атың") && joined.contains("кім"))
        || (joined.contains("атыңыз") && joined.contains("кім"))
        || joined.contains("есімің")
        || joined.contains("есіміңіз")
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
    // Pattern 1: "атым X".
    if let Some(i) = tokens.iter().position(|t| t == "атым") {
        if let Some(name) = raw_tokens.get(i + 1) {
            return Some(capitalise(name));
        }
    }
    // Pattern 3: "есімім X".
    if let Some(i) = tokens.iter().position(|t| t == "есімім") {
        if let Some(name) = raw_tokens.get(i + 1) {
            return Some(capitalise(name));
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
                    return Some(capitalise(name));
                }
            }
        }
    }
    None
}

/// Capitalise the first character of a Unicode string (Kazakh Cyrillic
/// names should render title-cased regardless of user input casing).
fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn detect_ask_age(joined: &str) -> bool {
    (joined.contains("жасың") && (joined.contains("неше") || joined.contains("қанша")))
        || (joined.contains("жасыңыз") && (joined.contains("неше") || joined.contains("қанша")))
        || joined.contains("қанша жастасың")
        || joined.contains("қанша жастасыз")
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

    // Primary: look for a parsed Noun in Ablative or Locative case.
    // Prefer Ablative (stronger signal for origin: "X-дан+мын") over
    // bare Locative. Also accept Locative if co-occurring with
    // "тұрамын / тұрамыз".
    let mut ablative_root: Option<String> = None;
    let mut locative_root: Option<String> = None;
    for p in parses {
        if let Analysis::Noun { root, features } = p {
            match features.case {
                Some(Case::Ablative) if ablative_root.is_none() => {
                    ablative_root = Some(capitalise(&root.root));
                }
                Some(Case::Locative) if locative_root.is_none() => {
                    locative_root = Some(capitalise(&root.root));
                }
                _ => {}
            }
        }
    }
    if let Some(c) = ablative_root {
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
            if features.case == Some(Case::Locative) && features.predicate == Some(Predicate::P1Sg)
            {
                return Some(Some(capitalise(&root.root)));
            }
        }
    }
    let live_verb = tokens.iter().any(|t| t == "тұрамын" || t == "тұрамыз");
    if live_verb {
        if let Some(c) = locative_root {
            return Some(Some(c));
        }
    }

    // Fallback for out-of-lexicon inputs: string-based heuristics.
    detect_statement_of_location_rule_based(tokens, raw_tokens, joined)
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
        let city = (0..verb_idx)
            .rev()
            .find_map(|i| strip_locative(&tokens[i]).map(|_| raw_tokens[i].clone()))
            .map(|raw| strip_locative_preserving(&raw));
        return Some(city);
    }
    // Ablative + 1sg copula: "Алматыданмын" → "Алматы".
    for (i, t) in tokens.iter().enumerate() {
        if let Some(root) = strip_ablative_copula(t) {
            let raw = raw_tokens
                .get(i)
                .map(|r| strip_ablative_copula_preserving(r).unwrap_or_else(|| root.clone()))
                .unwrap_or(root);
            return Some(Some(capitalise(&raw)));
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

fn detect_ask_occupation(joined: &str) -> bool {
    joined.contains("немен айналыс")
        || (joined.contains("жұмысың") && joined.contains("не"))
        || (joined.contains("жұмысыңыз") && joined.contains("не"))
        || joined.contains("кәсібің")
        || joined.contains("кәсібіңіз")
        || joined.contains("мамандығың")
        || joined.contains("мамандығыңыз")
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
}
