// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 22 step A (2026-06-03)** — context-aware STT corrections.
//!
//! The dialog kernel is deterministic and cannot «learn» new
//! associations at runtime — every fix has to land in source so it
//! ships to every user. But each new live-REPL audit surfaces
//! Whisper STT-drift patterns that are unambiguous given the
//! surrounding context, so we codify them here as deterministic
//! pattern substitutions BEFORE the input reaches fuzzy / LM
//! rescoring / intent routing.
//!
//! ## What counts as a context correction
//!
//! Only patterns where the corrected form is grammatically valid AND
//! the original form is grammatically invalid (or pragmatically
//! absurd) in the same context. Example:
//!
//!   - «Менің атом X» → «Менің атым X»  (Whisper-drifts «атым» to
//!     «атом»; «менің атом» without a possessive «-ым» is grammatical
//!     nonsense in standalone position. Always safe to correct.)
//!
//! We do NOT codify drifts that could be either correct in context —
//! those go through fuzzy / LM rescoring instead.
//!
//! ## Why this lives in voice REPL, not adam-dialog
//!
//! The corrections are STT-drift artifacts of audio recognition;
//! the text REPL (`adam_chat`) never sees them. Keeping the patch
//! here keeps the deterministic dialog kernel STT-agnostic.

/// Apply known context-aware STT corrections. Returns the input
/// unchanged when no pattern matches. **Stateless version** — for
/// the more powerful session-aware variant that reacts to adam
/// asking «What's your name?» in the previous turn, use
/// [`apply_with_context`].
///
/// Kept as a public API for direct callers (and for the
/// stateless-path unit tests below); the voice REPL itself uses
/// `apply_with_context` so it can drive the awaiting-name flag.
#[allow(dead_code)]
pub fn apply(input: &str) -> String {
    apply_with_context(input, false)
}

/// **Phase 22 step B (2026-06-03)** — conversation-aware STT
/// correction. The voice REPL pipes the «awaiting name» flag from
/// the session here.  When `awaiting_name` is true (i.e. adam's
/// previous reply was «Атыңызды айта аласыз ба?» or similar), we
/// activate a more aggressive name-extraction pass that handles
/// Whisper drifts the stateless pattern set cannot catch:
///
///   - «Менің NN Дәулет» where NN is any Whisper drift for «атым»
///     (атом / есім / ату / адым / ат / …) → «Менің атым Дәулет»
///   - «Бұл NN Дәулет» / «Мен NN Дәулет» → same canonical form
///
/// The user request that motivated this: «Pattern fix catches
/// 'атом' but the model should hold the dialog context — when adam
/// just asked for my name, ANY Whisper distortion in that slot is
/// my name, not a chemistry question.»
pub fn apply_with_context(input: &str, awaiting_name: bool) -> String {
    let lower = input.to_lowercase();

    // Session-aware path — fires only when adam just asked for the
    // name. Catches a broader set of Whisper drifts than the
    // stateless patterns below.
    if awaiting_name {
        if let Some(rewritten) = rewrite_as_name_statement(input) {
            return rewritten;
        }
    }

    // Pattern 1: «менің атом ____» → «менің атым ____»
    // «атым» (my-name, possessive-1sg) Whisper-drifts to «атом» (atom).
    // The drift collides with the `Атом` chemistry definition in
    // world_core, so adam answers «Атом — заттың химиялық қасиеттерін…»
    // when the user is introducing themselves. The standalone «менің
    // атом» form (without further possessive suffix) is ungrammatical
    // in Kazakh — there's no «my atom» reading here that wouldn't
    // require «менің атомым». Always safe to correct.
    if lower.contains("менің атом ") || lower.contains("менім атом ") {
        return rewrite_word(input, "атом ", "атым ");
    }
    // Same with terminal «атом.» / «атом?» / «атом!».
    if lower.contains("менің атом")
        && (lower.ends_with("атом")
            || lower.ends_with("атом.")
            || lower.ends_with("атом?")
            || lower.ends_with("атом!"))
    {
        return rewrite_word(input, "атом", "атым");
    }

    // Pattern 2: «сенің атом» / «сенім атом» → «сенің атың» / «сенің атың».
    // Mirror of pattern 1 for second-person form. Less common but
    // appears when adam is asked "what's your name" with a slip.
    if lower.contains("сенің атом ") || lower.contains("сенім атом ") {
        return rewrite_word(input, "атом ", "атың ");
    }

    // **Phase 22.C (2026-06-03 evening)** — accusative→locative city
    // case suffix normalization. Live REPL caught «Мен Қостанайды
    // тұрамын» — Whisper drifted the locative `‑да` to accusative
    // `‑ды` («Where in X I live» → «X-acc I live»). The v6.1
    // StatementOfLocation parser only recognises the locative form,
    // so the accusative-drifted input fell through to the generic
    // clarification fallback («Сұрағыңызды толық түсінбедім»).
    //
    // When the input clearly looks like a first-person dwelling
    // statement (мен/менің + known KZ city + тұрамын) AND the city
    // carries the accusative suffix, rewrite the suffix to locative.
    // Same length, preserves casing for the rest of the input.
    if (lower.starts_with("мен ") || lower.starts_with("менім ")) && lower.contains("тұрамын")
    {
        let cities = [
            ("қостанайды", "қостанайда"),
            ("алматыны", "алматыда"),
            ("астананы", "астанада"),
            ("шымкентті", "шымкентте"),
            ("ақтөбені", "ақтөбеде"),
            ("таразды", "таразда"),
            ("өскеменді", "өскеменде"),
            ("семейді", "семейде"),
            ("павлодарды", "павлодарда"),
            ("атырауды", "атырауда"),
            ("ақтауды", "ақтауда"),
            ("оралды", "оралда"),
            ("қызылорданы", "қызылордада"),
            ("көкшетауды", "көкшетауда"),
            ("петропавлды", "петропавлда"),
            ("жезқазғанды", "жезқазғанда"),
            ("темиртауды", "темиртауда"),
            ("талдықорғанды", "талдықорғанда"),
        ];
        for (acc_form, loc_form) in cities {
            if lower.contains(acc_form) {
                // Case-insensitive replace preserves the rest of the
                // original input; we just normalise the city suffix.
                return rewrite_word(input, acc_form, loc_form);
            }
        }
    }

    input.to_string()
}

/// Aggressive name-extraction when adam just asked for the name.
/// Detects `«<menin/menim/men> <drift-of-atym> <name>»` and rewrites
/// to the canonical form `«Менің атым <name>»`. Returns `None` if
/// the input doesn't match the name-introduction shape.
fn rewrite_as_name_statement(input: &str) -> Option<String> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }
    // First token must be a first-person possessive marker.
    let first = tokens[0]
        .trim_matches(|c: char| !c.is_alphanumeric() && c != 'ң' && c != 'і')
        .to_lowercase();
    if !matches!(first.as_str(), "менің" | "менім" | "мен") {
        return None;
    }
    // Known Whisper drifts for «атым» (my-name, possessive-1sg).
    // Includes the canonical «атым» itself so the function is
    // idempotent when re-invoked on already-corrected input.
    let drift_words = [
        "атым",
        "атом",
        "есім",
        "есімім",
        "есі",
        "ату",
        "атту",
        "адым",
        "ат",
        "оты",
        "атам",
    ];
    // Find the first drift word after position 0; everything after
    // it is the name (preserve casing).
    for (idx, tok) in tokens.iter().enumerate().skip(1) {
        let tok_lower = tok
            .trim_matches(|c: char| !c.is_alphanumeric() && c != 'ң' && c != 'і')
            .to_lowercase();
        if drift_words.contains(&tok_lower.as_str()) {
            // Tail = everything after the drift token, joined.
            let tail = tokens[idx + 1..].join(" ");
            if tail.trim().is_empty() {
                // «Менің атом» with no name → not a complete intro.
                return None;
            }
            // Preserve terminal punctuation from the original drift
            // token if it's the LAST token before tail.
            return Some(format!("Менің атым {tail}"));
        }
    }
    None
}

/// Detect whether the previous adam reply asked for the user's
/// name. Used by the voice REPL to set the `voice_awaiting_name`
/// session flag.
pub fn reply_asks_for_name(reply: &str) -> bool {
    let lower = reply.to_lowercase();
    lower.contains("атыңызды айта аласыз")
        || lower.contains("атыңды айта аласың")
        || lower.contains("атыңыз кім")
        || lower.contains("атың кім")
        || lower.contains("есіміңізді айт")
        || lower.contains("есіміңіз қалай")
}

/// Case-preserving single-token replace. Operates on lowercased
/// pattern matching against the lowercased input, applies the same
/// byte ranges back to the original-cased input.
fn rewrite_word(input: &str, needle_lower: &str, replacement: &str) -> String {
    let lower = input.to_lowercase();
    if let Some(pos) = lower.find(needle_lower) {
        let mut out = String::with_capacity(input.len());
        out.push_str(&input[..pos]);
        out.push_str(replacement);
        out.push_str(&input[pos + needle_lower.len()..]);
        return out;
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_atom_to_atym_in_name_frame() {
        let out = apply("Менің атом Дәулет.");
        assert_eq!(out, "Менің атым Дәулет.");
    }

    #[test]
    fn fixes_atom_to_atym_lowercase() {
        let out = apply("менің атом дәулет");
        assert_eq!(out, "менің атым дәулет");
    }

    #[test]
    fn fixes_terminal_atom() {
        let out = apply("Менің атом");
        assert_eq!(out, "Менің атым");
    }

    #[test]
    fn fixes_menim_typo() {
        // «менім» typo for «менің» — also common in transcripts.
        let out = apply("Менім атом Дауыл.");
        assert_eq!(out, "Менім атым Дауыл.");
    }

    #[test]
    fn leaves_real_atom_query_alone() {
        // «Атом деген не?» — real question about atom, no «менің» prefix.
        let input = "Атом деген не?";
        assert_eq!(apply(input), input);
    }

    #[test]
    fn leaves_proper_grammar_alone() {
        // Grammatically correct «менің атомым» — possessive. Don't touch.
        let input = "Менің атомым — Уран.";
        assert_eq!(apply(input), input);
    }

    #[test]
    fn fixes_senin_atom() {
        let out = apply("Сенің атом кім?");
        assert_eq!(out, "Сенің атың кім?");
    }

    // ── Phase 22 step B — session-aware tests ──

    #[test]
    fn awaiting_name_rewrites_any_drift() {
        // After adam asks for name, ANY Whisper drift of «атым»
        // in a «Менің <drift> X» frame gets canonicalised.
        assert_eq!(
            apply_with_context("Менің атом Дәулет.", true),
            "Менің атым Дәулет."
        );
        assert_eq!(
            apply_with_context("Менің есім Айгүл", true),
            "Менің атым Айгүл"
        );
        assert_eq!(
            apply_with_context("Менің ату Болат.", true),
            "Менің атым Болат."
        );
        assert_eq!(
            apply_with_context("Менім адым Дауыл", true),
            "Менің атым Дауыл"
        );
    }

    #[test]
    fn awaiting_name_idempotent_on_canonical() {
        // Already correct «менің атым X» → unchanged shape.
        let out = apply_with_context("Менің атым Дәулет.", true);
        assert!(out.starts_with("Менің атым"), "got: {out}");
        assert!(out.contains("Дәулет"), "got: {out}");
    }

    #[test]
    fn awaiting_name_does_nothing_without_drift() {
        // «Сау бол» / «Қайырлы күн» / arbitrary non-name input —
        // even when expecting a name, don't force-rewrite arbitrary
        // utterances.
        assert_eq!(apply_with_context("Сау бол.", true), "Сау бол.");
        assert_eq!(apply_with_context("Қайырлы күн.", true), "Қайырлы күн.");
    }

    #[test]
    fn awaiting_name_requires_trailing_name() {
        // «Менің атом» (no name after) → not a complete intro,
        // leave alone so the regular pattern path still runs.
        let out = apply_with_context("Менің атом", true);
        // Falls back to stateless pattern 1 which rewrites the
        // standalone terminal «атом».
        assert_eq!(out, "Менің атым");
    }

    #[test]
    fn reply_asks_for_name_recognises_intro_proposal() {
        // The IntroProposal trailer should be detected.
        assert!(reply_asks_for_name(
            "Иә, әрине. Мен — қазақша сөйлесуге арналған ARK, атым адам. Атыңызды айта аласыз ба?"
        ));
        assert!(reply_asks_for_name("Атыңыз кім?"));
        assert!(reply_asks_for_name("Есіміңізді айтып беріңізші"));
    }

    #[test]
    fn reply_asks_for_name_false_on_arbitrary_reply() {
        assert!(!reply_asks_for_name("Сау болыңыз."));
        assert!(!reply_asks_for_name("Бүгін — сейсенбі."));
        assert!(!reply_asks_for_name(
            "Қазақстанның бірінші Президенті — Назарбаев."
        ));
    }

    // ── Phase 22.C — accusative → locative city rewrite ──

    #[test]
    fn rewrites_accusative_kostanay_to_locative() {
        // Live REPL transcript: «Мен қостанайды тұрамын.» — Whisper
        // drifted -да (locative) to -ды (accusative). The v6.1
        // StatementOfLocation parser only recognises locative;
        // rewriting here lets it acknowledge.
        let out = apply("Мен қостанайды тұрамын.");
        assert_eq!(out, "Мен қостанайда тұрамын.");
    }

    #[test]
    fn rewrites_accusative_almaty_to_locative() {
        let out = apply("Мен алматыны тұрамын.");
        assert_eq!(out, "Мен алматыда тұрамын.");
    }

    #[test]
    fn does_not_rewrite_canonical_locative() {
        let input = "Мен қостанайда тұрамын.";
        assert_eq!(apply(input), input);
    }

    #[test]
    fn does_not_rewrite_outside_first_person_frame() {
        // No «мен/менім» prefix → not our pattern.
        let input = "Қостанайды көрдім.";
        assert_eq!(apply(input), input);
    }
}
