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
    let joined = tokens.join(" ");

    if let Some(g) = detect_greeting(&tokens, &joined) {
        return g;
    }
    if detect_farewell(&tokens, &joined) {
        return Intent::Farewell;
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
