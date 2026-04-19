//! Layer 2 — semantic interpreter.
//!
//! For MVP social intents the FST parser is more than we need — we just
//! want to match on surface keywords (сәлем, жоқ, иә, т.б.) regardless
//! of whether the word is in the lexicon. Later intents that need
//! morphological info (person/number/tense for "где ты живёшь") will
//! also get the parse sequence; for now we work directly on the
//! lowercased-cleaned token list.

use adam_kernel_fst::parser::Analysis;

use crate::intent::{GreetingKind, Intent, TimeOfDay};

/// Entry point. Takes the raw input text; tokenises, lowercases, strips
/// punctuation, then dispatches by keyword rules.
///
/// The `_parses` argument is kept so callers stay forward-compatible:
/// v0.7.5 intents can start using morphological info without changing
/// the call site.
pub fn interpret_text(input: &str, _parses: &[Analysis]) -> Intent {
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
    if let Some(city) = detect_statement_of_location(&tokens, &raw_tokens, &joined) {
        return Intent::StatementOfLocation { city };
    }
    if detect_ask_location(&joined) {
        return Intent::AskLocation;
    }
    if let Some(occupation) = detect_statement_of_occupation(&tokens, &raw_tokens, &joined) {
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
    if detect_statement_of_wellbeing(&tokens, &joined) {
        return Intent::StatementOfWellbeing;
    }
    if detect_affirmation(&tokens, &joined) {
        return Intent::Affirmation;
    }
    if detect_negation(&tokens, &joined) {
        return Intent::Negation;
    }

    Intent::Unknown { raw_tokens: tokens }
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

    Intent::Unknown { raw_tokens: tokens }
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
    // Time-of-day: "доброе утро" / "good morning" etc. — check BEFORE
    // generic casual so that "доброе утро" isn't ambiguously parsed.
    if let Some(tod) = detect_time_of_day(joined) {
        return Some(Intent::Greeting {
            kind: GreetingKind::TimeOfDay(tod),
        });
    }
    // Polite multi-word: "сәлеметсіз бе" / "здравствуйте" / "good day".
    if joined.contains("сәлеметсіз")
        || joined.contains("сәлеметсің")
        || tokens
            .iter()
            .any(|t| t == "здравствуйте" || t == "здравствуй")
        || joined == "good day"
    {
        return Some(Intent::Greeting {
            kind: GreetingKind::Polite,
        });
    }
    // Casual: "сәлем" / "hi" / "hello" / "hey" / "привет".
    if tokens
        .first()
        .is_some_and(|t| matches!(t.as_str(), "сәлем" | "hi" | "hello" | "hey" | "привет"))
    {
        return Some(Intent::Greeting {
            kind: GreetingKind::Casual,
        });
    }
    None
}

/// Time-of-day greeting detection across Kazakh / Russian / English.
fn detect_time_of_day(joined: &str) -> Option<TimeOfDay> {
    // Kazakh canonical: "қайырлы таң/күн/кеш".
    if joined.contains("қайырлы") {
        if joined.contains("таң") {
            return Some(TimeOfDay::Morning);
        }
        if joined.contains("кеш") {
            return Some(TimeOfDay::Evening);
        }
        return Some(TimeOfDay::Day);
    }
    // Russian: "доброе утро / добрый день / добрый вечер".
    if joined.contains("доброе утро") || joined.contains("добрый утро") {
        return Some(TimeOfDay::Morning);
    }
    if joined.contains("добрый день") {
        return Some(TimeOfDay::Day);
    }
    if joined.contains("добрый вечер") {
        return Some(TimeOfDay::Evening);
    }
    // English: "good morning / good afternoon / good evening".
    if joined.contains("good morning") {
        return Some(TimeOfDay::Morning);
    }
    if joined.contains("good afternoon") {
        return Some(TimeOfDay::Day);
    }
    if joined.contains("good evening") {
        return Some(TimeOfDay::Evening);
    }
    None
}

fn detect_farewell(tokens: &[String], joined: &str) -> bool {
    // Kazakh: сау/қош leading + standard phrases.
    if tokens.first().is_some_and(|t| t == "сау" || t == "қош")
        || joined.contains("кездескенше")
        || joined.contains("сау бол")
        || joined.contains("қош бол")
    {
        return true;
    }
    // Russian farewells.
    if joined.contains("до свидания") || joined == "пока" || joined.contains("пока пока")
    {
        return true;
    }
    // English farewells.
    if joined == "bye" || joined == "goodbye" || joined == "bye bye" || joined.contains("see you") {
        return true;
    }
    false
}

fn detect_affirmation(tokens: &[String], joined: &str) -> bool {
    let single = tokens.len() == 1;
    if single {
        let w = &tokens[0];
        matches!(
            w.as_str(),
            // Kazakh
            "иә" | "ия" | "дұрыс" | "рас" | "мақұл"
            // Russian
            | "да" | "ага" | "угу" | "конечно"
            // English
            | "yes" | "yeah" | "yep" | "yup" | "sure" | "ok" | "okay"
        )
    } else {
        joined.contains("дұрыс айтасыз") || joined == "иә дұрыс" || joined == "of course"
    }
}

fn detect_negation(tokens: &[String], joined: &str) -> bool {
    let single = tokens.len() == 1;
    if single {
        let w = &tokens[0];
        matches!(
            w.as_str(),
            // Kazakh
            "жоқ" | "қате" | "емес"
            // Russian
            | "нет" | "неправда"
            // English
            | "no" | "nope" | "nah"
        )
    } else {
        joined.contains("жоқ емес") || joined.starts_with("жоқ") || joined == "no way"
    }
}

// --- v0.7.5 new recognisers ------------------------------------------------

fn detect_thanks(tokens: &[String], joined: &str) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t.as_str(), "рахмет" | "рахметім" | "спасибо" | "thanks"))
        || joined.contains("көп рахмет")
        || joined.contains("рақмет")
        || joined.contains("большое спасибо")
        || joined.contains("thank you")
}

fn detect_apology(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "кешіріңіз"
                | "ғафу"
                | "извини"
                | "извините"
                | "прости"
                | "простите"
                | "sorry"
        )
    }) || joined.contains("кешір")
        || joined.contains("ғафу ет")
        || joined.contains("my apologies")
        || joined.contains("excuse me")
}

/// "How are you?" — Kazakh / Russian / English.
fn detect_ask_how_are_you(joined: &str) -> bool {
    // Kazakh
    if joined.contains("қалайсың")
        || joined.contains("қалайсыз")
        || joined.contains("жағдайың қалай")
        || joined.contains("жағдайыңыз қалай")
        || joined.contains("халің қалай")
        || joined.contains("халіңіз қалай")
        || joined == "қалың қалай"
    {
        return true;
    }
    // Russian
    if joined.contains("как дела") || joined.contains("как ты") || joined.contains("как вы")
    {
        return true;
    }
    // English
    joined.contains("how are you") || joined.contains("how r u") || joined.contains("hows it")
}

/// "What's your name?" — Kazakh / Russian / English.
fn detect_ask_name(joined: &str) -> bool {
    // Kazakh
    if (joined.contains("атың") && joined.contains("кім"))
        || (joined.contains("атыңыз") && joined.contains("кім"))
        || joined.contains("есімің")
        || joined.contains("есіміңіз")
    {
        return true;
    }
    // Russian
    if joined.contains("как тебя зовут") || joined.contains("как вас зовут")
    {
        return true;
    }
    // English — the tokeniser strips `'` so "what's your name" becomes
    // "whats your name" by the time we see the joined string.
    joined.contains("what is your name") || joined.contains("whats your name")
}

/// User is saying how they are — Kazakh / Russian / English.
fn detect_statement_of_wellbeing(tokens: &[String], joined: &str) -> bool {
    let wellbeing_token = tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            // Kazakh
            "жақсымын" | "жаманмын" | "жақсы" | "жаман" | "дұрысмын"
            // Russian
            | "хорошо" | "нормально" | "плохо" | "отлично"
            // English
            | "fine" | "great"
        )
    });
    wellbeing_token
        || joined.contains("жаман емес")
        || joined.contains("im good")
        || joined.contains("i am good")
        || joined.contains("im fine")
        || joined.contains("i am fine")
        || joined.contains("у меня всё хорошо")
        || joined.contains("все хорошо")
}

// --- v0.8.0 new recognisers ------------------------------------------------

/// User introduces self in Kazakh / Russian / English. Returns the
/// extracted name (case preserved, first-letter title-cased) or None.
///
/// Supported patterns:
///   Kazakh:
///     1. [менің] атым <NAME>
///     2. мені <NAME> деп атайды
///     3. есімім <NAME>
///   Russian:
///     4. меня зовут <NAME>
///     5. моё имя <NAME> / мое имя <NAME>
///   English:
///     6. my name is <NAME>
///     7. i am <NAME> / i'm <NAME>   (only when preceded by "hi/hello")
///     8. call me <NAME>
fn detect_statement_of_name(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
) -> Option<String> {
    // Kazakh pattern 1: "атым X".
    if let Some(i) = tokens.iter().position(|t| t == "атым") {
        if let Some(name) = raw_tokens.get(i + 1) {
            return Some(capitalise(name));
        }
    }
    // Kazakh pattern 3: "есімім X".
    if let Some(i) = tokens.iter().position(|t| t == "есімім") {
        if let Some(name) = raw_tokens.get(i + 1) {
            return Some(capitalise(name));
        }
    }
    // Kazakh pattern 2: "мені X деп атайды".
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
    // Russian pattern 4: "меня зовут X".
    if let Some(i) = tokens.iter().position(|t| t == "зовут") {
        if let Some(name) = raw_tokens.get(i + 1) {
            return Some(capitalise(name));
        }
    }
    // Russian pattern 5: "моё имя X" / "мое имя X".
    if let Some(i) = tokens.iter().position(|t| t == "имя") {
        if let Some(name) = raw_tokens.get(i + 1) {
            return Some(capitalise(name));
        }
    }
    // English pattern 6: "my name is X".
    if let Some(i) = tokens
        .windows(3)
        .position(|w| w[0] == "my" && w[1] == "name" && w[2] == "is")
    {
        if let Some(name) = raw_tokens.get(i + 3) {
            return Some(capitalise(name));
        }
    }
    // English pattern 8: "call me X".
    if let Some(i) = tokens
        .windows(2)
        .position(|w| w[0] == "call" && w[1] == "me")
    {
        if let Some(name) = raw_tokens.get(i + 2) {
            return Some(capitalise(name));
        }
    }
    // English pattern 7: "hi/hello/hey, i am/i'm X" — the leading greet
    // token disambiguates self-intro from a bare first-person "I am X"
    // (which is too generic — could be occupation, age, etc.)
    if let Some(leader) = tokens.first() {
        if matches!(leader.as_str(), "hi" | "hello" | "hey" | "привет") {
            if let Some(i) = tokens
                .windows(2)
                .position(|w| w[0] == "i" && (w[1] == "am" || w[1] == "m"))
            {
                if let Some(name) = raw_tokens.get(i + 2) {
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

/// "How old are you?" — Kazakh / Russian / English.
fn detect_ask_age(joined: &str) -> bool {
    // Kazakh
    if (joined.contains("жасың") && (joined.contains("неше") || joined.contains("қанша")))
        || (joined.contains("жасыңыз") && (joined.contains("неше") || joined.contains("қанша")))
        || joined.contains("қанша жастасың")
        || joined.contains("қанша жастасыз")
    {
        return true;
    }
    // Russian
    if joined.contains("сколько тебе лет") || joined.contains("сколько вам лет")
    {
        return true;
    }
    // English
    joined.contains("how old are you")
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

/// "Where are you from / where do you live?" — Kazakh / Russian / English.
fn detect_ask_location(joined: &str) -> bool {
    // Kazakh
    if joined.contains("қай жерден")
        || joined.contains("қайдан")
        || joined.contains("қайда тұра")
        || joined.contains("қай қала")
        || joined.contains("қай аудан")
    {
        return true;
    }
    // Russian
    if joined.contains("откуда ты") || joined.contains("откуда вы") || joined.contains("где живёшь")
    {
        return true;
    }
    // English
    joined.contains("where are you from") || joined.contains("where do you live")
}

/// User states location: "мен Алматыданмын", "астанада тұрамын",
/// "ауылдан келдім". Returns `Some(city)` when the pattern fires,
/// with `city` being the extracted root (case-preserved, nominative)
/// — or `None` inside `Some` when the pattern fires without a
/// parseable city token.
fn detect_statement_of_location(
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
fn strip_ablative_copula(token: &str) -> Option<String> {
    for suffix in ["данмын", "денмін", "танмын", "тенмін"] {
        if token.ends_with(suffix) && token.chars().count() > suffix.chars().count() + 1 {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn strip_ablative_copula_preserving(token: &str) -> Option<String> {
    let lower = token.to_lowercase();
    for suffix in ["данмын", "денмін", "танмын", "тенмін"] {
        if lower.ends_with(suffix) && lower.chars().count() > suffix.chars().count() + 1 {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

/// "What do you do for work?" — Kazakh / Russian / English.
fn detect_ask_occupation(joined: &str) -> bool {
    // Kazakh
    if joined.contains("немен айналыс")
        || (joined.contains("жұмысың") && joined.contains("не"))
        || (joined.contains("жұмысыңыз") && joined.contains("не"))
        || joined.contains("кәсібің")
        || joined.contains("кәсібіңіз")
        || joined.contains("мамандығың")
        || joined.contains("мамандығыңыз")
    {
        return true;
    }
    // Russian
    if joined.contains("кем работаешь")
        || joined.contains("кем вы работаете")
        || joined.contains("чем занимаешься")
    {
        return true;
    }
    // English
    joined.contains("what do you do") || joined.contains("whats your job")
}

/// User states occupation: "мен мұғаліммін", "дәрігермін",
/// "мен жұмыс істеймін". Returns `Some(Some(root))` when a known
/// occupation token + 1sg copula matched, with `root` being the
/// stripped occupation noun (nominative). Falls back to
/// `Some(None)` for "жұмыс істеймін" (no specific occupation parseable).
fn detect_statement_of_occupation(
    tokens: &[String],
    _raw_tokens: &[String],
    joined: &str,
) -> Option<Option<String>> {
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
    if joined.contains("жұмыс істеймін") {
        return Some(None);
    }
    None
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

/// "What's the weather?" — Kazakh / Russian / English.
fn detect_ask_weather(joined: &str) -> bool {
    // Kazakh
    if (joined.contains("ауа райы") && joined.contains("қалай"))
        || (joined.contains("бүгін") && joined.contains("ауа райы"))
        || (joined.contains("сыртта") && joined.contains("қалай"))
    {
        return true;
    }
    // Russian
    if joined.contains("какая погода") || joined.contains("какая сегодня погода")
    {
        return true;
    }
    // English
    joined.contains("how is the weather")
        || joined.contains("hows the weather")
        || joined.contains("whats the weather")
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

/// "What time / day is it?" — Kazakh / Russian / English.
fn detect_ask_time(joined: &str) -> bool {
    // Kazakh
    if (joined.contains("сағат") && (joined.contains("неше") || joined.contains("қанша")))
        || joined.contains("қазір уақыт")
        || joined.contains("қандай күн")
        || joined.contains("қай күн")
    {
        return true;
    }
    // Russian
    if joined.contains("сколько времени") || joined.contains("который час")
    {
        return true;
    }
    // English
    joined.contains("what time is it") || joined.contains("whats the time")
}

/// Compliments — Kazakh / Russian / English.
fn detect_compliment(tokens: &[String], joined: &str) -> bool {
    let token = tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            // Kazakh
            "жарайсың" | "жарайсыз" | "керемет" | "тамаша" | "мықты"
            // Russian
            | "молодец" | "отлично" | "здорово" | "прекрасно"
            // English
            | "great" | "awesome" | "wonderful" | "excellent" | "perfect"
        )
    });
    token
        || joined.contains("өте жақсы")
        || joined.contains("well done")
        || joined.contains("good job")
}

/// Polite request / please — Kazakh / Russian / English.
fn detect_request(tokens: &[String], joined: &str) -> bool {
    let token = tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            // Kazakh
            "өтінемін" | "сұраймын" | "көмектесіңізші" | "көмектесіңіз" | "көмектес"
            // Russian
            | "пожалуйста" | "помогите" | "помоги"
            // English
            | "please"
        )
    });
    token
        || joined.contains("көмек керек")
        || joined.contains("need help")
        || joined.contains("can you help")
}

/// Well-wishes — Kazakh / Russian / English.
fn detect_well_wishes(joined: &str) -> bool {
    // Kazakh
    if joined.contains("жақсы күн тіле")
        || joined.contains("сәттілік")
        || joined.contains("табысты бол")
        || joined.contains("денсаулық тіле")
    {
        return true;
    }
    // Russian
    if joined.contains("удачи")
        || joined.contains("всего наилучшего")
        || joined.contains("всего хорошего")
    {
        return true;
    }
    // English
    joined.contains("good luck") || joined.contains("all the best")
}
