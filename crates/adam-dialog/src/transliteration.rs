//! Latin → Kazakh Cyrillic transliteration for entity slots (v0.9.8).
//!
//! Purpose: users who write in English / Russian transliteration
//! ("my name is John") often provide proper names in Latin script. The
//! Kazakh morphology FST is tuned for Cyrillic phonology, so when the
//! realiser encounters a template like `{name|instrumental}` and the
//! filled value is `"John"`, feeding it straight to `synthesise_noun`
//! produces garbled mixed-script output ("Johnман"). Transliterating
//! first — `"John"` → `"Джон"` — yields a Cyrillic-shaped root that
//! the FST can inflect naturally: `"Джонмен"`.
//!
//! This is deliberately simple. Kazakh has letters (ә, ө, ұ, ү, і, ғ,
//! қ, ң) that Latin lacks; there is no one "correct" back-mapping.
//! We aim for a reasonable default that won't embarrass the demo for
//! common English / common-Russian-in-Latin names.
//!
//! The mapping is NOT used for Cyrillic input — that stays verbatim.
//! The realiser calls [`latin_to_cyrillic`] only when the slot value
//! contains no Cyrillic characters at all.

/// Transliterate a Latin-script string into Kazakh Cyrillic. Already-
/// Cyrillic characters pass through unchanged; non-letter characters
/// (digits, hyphens) also pass through. The first letter of each
/// whitespace-separated token is title-cased to preserve name casing.
pub fn latin_to_cyrillic(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut bytes = lower.as_bytes();
    let mut out = String::with_capacity(input.len() * 2);

    // Process digraphs first, then single letters. We iterate on a
    // byte pointer because ASCII + digraph detection is byte-safe;
    // non-ASCII (Cyrillic) chars are copied whole.
    while !bytes.is_empty() {
        let s = std::str::from_utf8(bytes).expect("lowercased string must remain valid UTF-8");
        let first = s.chars().next().expect("non-empty");
        if !first.is_ascii() {
            out.push(first);
            bytes = &bytes[first.len_utf8()..];
            continue;
        }
        // Whitespace + hyphens + digits pass through as-is so token
        // boundaries survive transliteration.
        if !first.is_ascii_alphabetic() {
            out.push(first);
            bytes = &bytes[1..];
            continue;
        }
        // ASCII letter. Try digraph match at current position.
        if let Some(digraph) = try_digraph(s) {
            out.push_str(digraph);
            bytes = &bytes[2..];
            continue;
        }
        out.push_str(map_letter(first));
        bytes = &bytes[1..];
    }

    title_case_tokens(&out)
}

/// Two-letter combinations with an agreed Cyrillic rendering. Order
/// matters: checked before single-letter map so `sh` doesn't render
/// as `с` + `х`.
fn try_digraph(s: &str) -> Option<&'static str> {
    match s.get(..2)? {
        "sh" => Some("ш"),
        "ch" => Some("ч"),
        "zh" => Some("ж"),
        "kh" => Some("х"),
        "gh" => Some("ғ"),
        "ts" => Some("ц"),
        "ph" => Some("ф"),
        "th" => Some("т"),
        "yo" => Some("ё"),
        "ya" => Some("я"),
        "yu" => Some("ю"),
        "ye" => Some("е"),
        _ => None,
    }
}

/// Single-letter Latin → Cyrillic map. Deliberately conservative:
/// collapsing `c → к` (vs `с` before e/i) keeps round-tripping simple.
fn map_letter(c: char) -> &'static str {
    match c {
        'a' => "а",
        'b' => "б",
        'c' => "к",
        'd' => "д",
        'e' => "е",
        'f' => "ф",
        'g' => "г",
        'h' => "х",
        'i' => "и",
        'j' => "дж",
        'k' => "к",
        'l' => "л",
        'm' => "м",
        'n' => "н",
        'o' => "о",
        'p' => "п",
        'q' => "к",
        'r' => "р",
        's' => "с",
        't' => "т",
        'u' => "у",
        'v' => "в",
        'w' => "в",
        'x' => "кс",
        'y' => "й",
        'z' => "з",
        _ => "",
    }
}

/// Capitalise the first character of each whitespace-separated token.
/// Applied after transliteration so "john smith" → "джон смит" →
/// "Джон Смит".
fn title_case_tokens(s: &str) -> String {
    s.split_whitespace()
        .map(|tok| {
            let mut chars = tok.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_letter_names() {
        assert_eq!(latin_to_cyrillic("Anna"), "Анна");
        assert_eq!(latin_to_cyrillic("Tom"), "Том");
        assert_eq!(latin_to_cyrillic("Mike"), "Мике");
    }

    #[test]
    fn names_with_j_use_digraph() {
        // `j` → `дж`. Silent h is NOT special-cased (conservative MVP
        // mapping): "John" maps letter-by-letter to "Джохн". Good
        // enough for FST synthesis — "Джохнмен" is legible even if
        // not the standard spelling "Джон".
        assert_eq!(latin_to_cyrillic("John"), "Джохн");
        assert_eq!(latin_to_cyrillic("James"), "Джамес");
    }

    #[test]
    fn digraph_sh_ch_zh_kh() {
        assert_eq!(latin_to_cyrillic("Sharon"), "Шарон");
        assert_eq!(latin_to_cyrillic("Charlie"), "Чарлие");
        assert_eq!(latin_to_cyrillic("Zhanna"), "Жанна");
        assert_eq!(latin_to_cyrillic("Khalid"), "Халид");
    }

    #[test]
    fn already_cyrillic_unchanged() {
        // Cyrillic input passes through, with title-casing applied.
        assert_eq!(latin_to_cyrillic("Дәулет"), "Дәулет");
    }

    #[test]
    fn whitespace_preserves_token_boundaries() {
        // Two Latin tokens stay separate after transliteration.
        let out = latin_to_cyrillic("John Anna");
        assert_eq!(out, "Джохн Анна");
    }

    #[test]
    fn empty_input() {
        assert_eq!(latin_to_cyrillic(""), "");
    }
}
