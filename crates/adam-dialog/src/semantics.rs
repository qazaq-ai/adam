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
    // StatementOfName must be checked BEFORE AskName: "менің атым X"
    // contains "атым" which the AskName rule tolerates. Statements
    // always carry a concrete name slot; questions do not.
    if let Some(name) = detect_statement_of_name(&tokens, &raw_tokens, &joined) {
        return Intent::StatementOfName { name };
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
    // Casual: "сәлем" alone or as first token.
    if tokens.first().is_some_and(|t| t == "сәлем") {
        return Some(Intent::Greeting {
            kind: GreetingKind::Casual,
        });
    }
    // Polite multi-word: "сәлеметсіз бе".
    if joined.contains("сәлеметсіз") || joined.contains("сәлеметсің") {
        return Some(Intent::Greeting {
            kind: GreetingKind::Polite,
        });
    }
    // Time-of-day: "қайырлы таң/күн/кеш" (morning/day/evening).
    if joined.contains("қайырлы") {
        let kind = if joined.contains("таң") {
            TimeOfDay::Morning
        } else if joined.contains("кеш") {
            TimeOfDay::Evening
        } else {
            TimeOfDay::Day
        };
        return Some(Intent::Greeting {
            kind: GreetingKind::TimeOfDay(kind),
        });
    }
    None
}

fn detect_farewell(tokens: &[String], joined: &str) -> bool {
    tokens.first().is_some_and(|t| t == "сау" || t == "қош")
        || joined.contains("кездескенше")
        || joined.contains("сау бол")
        || joined.contains("қош бол")
}

fn detect_affirmation(tokens: &[String], joined: &str) -> bool {
    let single = tokens.len() == 1;
    if single {
        let w = &tokens[0];
        matches!(w.as_str(), "иә" | "ия" | "дұрыс" | "рас" | "мақұл")
    } else {
        joined.contains("дұрыс айтасыз") || joined == "иә дұрыс"
    }
}

fn detect_negation(tokens: &[String], joined: &str) -> bool {
    let single = tokens.len() == 1;
    if single {
        let w = &tokens[0];
        matches!(w.as_str(), "жоқ" | "қате" | "емес")
    } else {
        joined.contains("жоқ емес") || joined.starts_with("жоқ")
    }
}

// --- v0.7.5 new recognisers ------------------------------------------------

fn detect_thanks(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| t == "рахмет" || t == "рахметім")
        || joined.contains("көп рахмет")
        || joined.contains("рақмет")
}

fn detect_apology(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| t == "кешіріңіз" || t == "ғафу")
        || joined.contains("кешір")
        || joined.contains("ғафу ет")
}

/// "How are you?" — қалайсың / қалайсыз / жағдайыңыз қалай / хал қалай.
fn detect_ask_how_are_you(joined: &str) -> bool {
    joined.contains("қалайсың")
        || joined.contains("қалайсыз")
        || joined.contains("жағдайың қалай")
        || joined.contains("жағдайыңыз қалай")
        || joined.contains("халің қалай")
        || joined.contains("халіңіз қалай")
        || joined == "қалың қалай"
}

/// "What's your name?" — атың кім, есімің қалай, атыңыз кім.
fn detect_ask_name(joined: &str) -> bool {
    (joined.contains("атың") && joined.contains("кім"))
        || (joined.contains("атыңыз") && joined.contains("кім"))
        || joined.contains("есімің")
        || joined.contains("есіміңіз")
}

/// User is saying how they are: жақсымын, жаман емеспін, т.б.
fn detect_statement_of_wellbeing(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "жақсымын" | "жаманмын" | "жақсы" | "жаман" | "дұрысмын"
        )
    }) || joined.contains("жаман емес")
}

// --- v0.8.0 new recognisers ------------------------------------------------

/// User introduces self: "менің атым Дәулет", "атым Дәулет",
/// "мені Дәулет деп атайды". Returns the extracted name (case
/// preserved) or None if the pattern doesn't match.
///
/// The three patterns are:
///   1.  [менің] атым <NAME>
///   2.  мені <NAME> деп атайды
///   3.  есімім <NAME>
fn detect_statement_of_name(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
) -> Option<String> {
    // Pattern 1: "атым X" or "менің атым X" — name is the token right
    // after "атым".
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
    // Pattern 2: "мені X деп атайды" — X is between "мені" and "деп".
    if joined.contains("деп атайды") {
        if let (Some(start), Some(end)) = (
            tokens.iter().position(|t| t == "мені"),
            tokens.iter().position(|t| t == "деп"),
        ) {
            if end > start + 1 {
                // Take the token(s) between them; for MVP just the first.
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

/// "How old are you?" — жасың неше, жасыңыз қанша, қанша жастасың.
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

/// "Where are you from / where do you live?" — қай жерденсің,
/// қайда тұрасыз, қай қаладан, қайдан келдің.
fn detect_ask_location(joined: &str) -> bool {
    joined.contains("қай жерден")
        || joined.contains("қайдан")
        || joined.contains("қайда тұра")
        || joined.contains("қай қала")
        || joined.contains("қай аудан")
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

/// "What do you do for work?" — немен айналысасың, жұмысың не,
/// кәсібің қандай, мамандығың не.
fn detect_ask_occupation(joined: &str) -> bool {
    joined.contains("немен айналыс")
        || (joined.contains("жұмысың") && joined.contains("не"))
        || (joined.contains("жұмысыңыз") && joined.contains("не"))
        || joined.contains("кәсібің")
        || joined.contains("кәсібіңіз")
        || joined.contains("мамандығың")
        || joined.contains("мамандығыңыз")
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

/// "What's the weather?" — ауа райы қалай, бүгін қалай,
/// сыртта қалай.
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

/// "What time / day is it?" — сағат неше, қазір уақыт,
/// бүгін қандай күн.
fn detect_ask_time(joined: &str) -> bool {
    (joined.contains("сағат") && (joined.contains("неше") || joined.contains("қанша")))
        || joined.contains("қазір уақыт")
        || joined.contains("қандай күн")
        || joined.contains("қай күн")
}

/// Compliments: жарайсың, өте жақсы, керемет, тамаша.
fn detect_compliment(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "жарайсың" | "жарайсыз" | "керемет" | "тамаша" | "мықты"
        )
    }) || joined.contains("өте жақсы")
}

/// Polite request / please: өтінемін, сұраймын, көмектесіңізші,
/// көмектесіңіз.
fn detect_request(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "өтінемін" | "сұраймын" | "көмектесіңізші" | "көмектесіңіз" | "көмектес"
        )
    }) || joined.contains("көмек керек")
}

/// Well-wishes: жақсы күн тілеймін, сәттілік, табысты болыңыз.
fn detect_well_wishes(joined: &str) -> bool {
    joined.contains("жақсы күн тіле")
        || joined.contains("сәттілік")
        || joined.contains("табысты бол")
        || joined.contains("денсаулық тіле")
}
