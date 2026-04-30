//! `QuestionShape` — orthogonal to `Intent`, captures the *form* of a
//! user question independent of its topic.
//!
//! `Intent` classifies the dialog **act** (greeting, statement, ask,
//! unknown). `QuestionShape` refines questions into the *kind of
//! answer* they ask for: a definition, a causal explanation, a
//! capability check, a listing, a comparison, or a yes/no
//! confirmation. The two are composable: the same `Intent::Unknown`
//! can carry `QuestionShape::Definition` («Жасуша туралы не білесіз?»)
//! or `QuestionShape::Causal` («Неліктен жасуша өледі?»), and the
//! planner picks different template families accordingly.
//!
//! **v4.12.0** — first humanness work. The 2026-04-30 live REPL
//! battery surfaced two systemic gaps that QuestionShape closes:
//! - «Адам, неліктен жасуша өледі?» pre-v4.12.0 returned a generic
//!   IsA fact (`жасуша — физикалық субстанция`) because the planner
//!   could not distinguish a "what is X?" question from a
//!   "why does X happen?" question — both routed to `SearchGraph` and
//!   surfaced the most-central IsA fact.
//! - «Сіз қандай тілде жазылғансыз?» grounded on a generic
//!   `бағдарламалау тілі IsA формалды тіл` fact instead of adam's
//!   self-knowledge claim — same root cause: the form of the question
//!   ("what is X?" vs "what *are you* made of?") was invisible.
//!
//! The detector is pure surface-level (regex-style substring
//! matching), not FST-driven. This is intentional — `QuestionShape`
//! is a routing signal, not a structural analysis. The closed list of
//! markers is small (~25 surface forms total) and explicitly audited.

/// Form of a user question. Routed to template families per
/// `(Intent, QuestionShape)` in the planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QuestionShape {
    /// Default: "what is X?" / "tell me about X". Surface forms:
    /// `X туралы не білесіз / X дегеніміз не / X не / X айт`.
    /// Pre-v4.12.0 this was the only shape — every question fell
    /// here implicitly. Behaviour is unchanged at v4.12.0; the
    /// variant exists so the planner can switch on it.
    Definition,

    /// Causal "why?" question. Surface forms: `неліктен X / X неге /
    /// X себебі не / неге X / X қалай болады`. The planner routes
    /// this to a dedicated `unknown.causal` template family that
    /// hedges honestly when no Causes-predicate fact is found
    /// («Себебін нақты білмеймін, бірақ X жөнінде білетінім — …»).
    /// Pre-v4.12.0 these questions surfaced a generic IsA fact about
    /// X, which is logically wrong — adam asserted "X is Y" when the
    /// user asked "why does X happen?".
    Causal,

    /// Yes/no confirmation: "is X really Y?" / "X is Y, right?".
    /// Surface forms: `X шынымен Y ма / X дегеніміз шын Y ма`.
    /// Routes to a confirmation/denial framing rather than a
    /// definition. v4.12.0 stub — detector returns this only on
    /// explicit yes/no markers, no behavioural change yet (defaults
    /// fall to Definition); reserved for v4.12.5.
    YesNoCheck,

    /// Listing: "what X-es exist?" / "name the X-es". Surface forms:
    /// `қандай X-тер бар / X-тер тізімі / X-тердің атаулары`. Already
    /// handled at v4.4.11 via the `RelatedTo + тізім` list-summary
    /// renderer; this variant is the explicit form-side of that
    /// behaviour and gives the planner a name for it.
    Listing,

    /// Comparison: "is X better than Y?" / "what's the difference
    /// between X and Y?". Surface forms: `X пен Y айырмашылығы /
    /// X жақсырақ па / X-ден Y артық па`. v4.12.0 stub — defaults
    /// fall to Definition; reserved for v4.13+.
    Comparison,
}

impl QuestionShape {
    /// Stable string slug for diagnostic / template-key composition.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Causal => "causal",
            Self::YesNoCheck => "yes_no_check",
            Self::Listing => "listing",
            Self::Comparison => "comparison",
        }
    }
}

/// Detect the question shape in a raw user input. Returns `None` when
/// the input does not look like a question at all (no `?`, no
/// interrogative pronoun, no question particle).
///
/// Order of checks: more specific shapes first (Causal, Comparison,
/// Listing, YesNoCheck), Definition as the default catch-all. The
/// detector is intentionally conservative — when in doubt, return
/// `Definition` so existing v4.11.x routing is preserved.
pub fn detect(input: &str) -> Option<QuestionShape> {
    let lower = input.to_lowercase();

    if !is_question(&lower) {
        return None;
    }

    if is_causal(&lower) {
        return Some(QuestionShape::Causal);
    }
    if is_comparison(&lower) {
        return Some(QuestionShape::Comparison);
    }
    if is_listing(&lower) {
        return Some(QuestionShape::Listing);
    }
    if is_yes_no_check(&lower) {
        return Some(QuestionShape::YesNoCheck);
    }

    Some(QuestionShape::Definition)
}

/// Does the input carry any signal of being a question?
fn is_question(lower: &str) -> bool {
    if lower.contains('?') {
        return true;
    }
    // Question particles + pronouns. The pronoun list is a subset of
    // `NOT_A_TOPIC` — these never appear as topic nouns but always
    // mark a question.
    const QUESTION_MARKERS: &[&str] = &[
        " ма?",
        " ме?",
        " ба?",
        " бе?",
        " ма ",
        " ме ",
        " ба ",
        " бе ",
        "не ",
        "не?",
        "кім ",
        "кім?",
        "қандай ",
        "қалай ",
        "қашан ",
        "қайда ",
        "қанша ",
        "неше ",
        "неліктен ",
        "неге ",
        "неге?",
    ];
    QUESTION_MARKERS.iter().any(|m| lower.contains(m))
}

fn is_causal(lower: &str) -> bool {
    lower.contains("неліктен")
        || lower.contains(" неге")
        || lower.starts_with("неге")
        || lower.contains("себебі")
        || lower.contains("себеп")
        || lower.contains("неге өледі")
        || lower.contains("неге болады")
        || lower.contains("неге пайда болады")
}

fn is_comparison(lower: &str) -> bool {
    lower.contains("айырмашылығы")
        || lower.contains("айырмашылық")
        || lower.contains("жақсырақ")
        || lower.contains("артық")
        || lower.contains("ерекшелігі")
        || lower.contains(" мен ") && (lower.contains("қайсысы") || lower.contains("қайсы"))
}

fn is_listing(lower: &str) -> bool {
    // "What X-es exist?" patterns. Distinct from "what is X?" which
    // is a Definition.
    (lower.contains("қандай") && (lower.contains(" бар") || lower.contains("тізім")))
        || lower.contains("атаулары")
        || lower.contains("тізімі")
        || lower.contains("тізім")
}

fn is_yes_no_check(lower: &str) -> bool {
    // Explicit confirmation markers. The bare "X-Y, ма?" form is
    // ambiguous — it could be a yes/no check or an Affirmation
    // request — so we require an explicit auxiliary like `шынымен`
    // ("really") or `дұрыс па` ("is it correct").
    lower.contains("шынымен")
        || lower.contains("дұрыс па")
        || lower.contains("дұрыс ма")
        || lower.contains("растайсыз")
        || lower.contains("растайсың")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_causal_questions() {
        assert_eq!(
            detect("Неліктен жасуша өледі?"),
            Some(QuestionShape::Causal)
        );
        assert_eq!(
            detect("Адам, неліктен су қайнайды?"),
            Some(QuestionShape::Causal)
        );
        assert_eq!(detect("Бұл неге болады?"), Some(QuestionShape::Causal));
        assert_eq!(
            detect("Жасушаның өлуінің себебі не?"),
            Some(QuestionShape::Causal)
        );
    }

    #[test]
    fn definition_is_the_default_for_what_is_x() {
        assert_eq!(
            detect("Жасуша туралы не білесіз?"),
            Some(QuestionShape::Definition)
        );
        assert_eq!(
            detect("Физика дегеніміз не?"),
            Some(QuestionShape::Definition)
        );
        assert_eq!(detect("Атом не?"), Some(QuestionShape::Definition));
    }

    #[test]
    fn detects_listing_questions() {
        assert_eq!(
            detect("Қазақстанда қандай облыстар бар?"),
            Some(QuestionShape::Listing)
        );
        assert_eq!(
            detect("Қазақ хандарының атаулары қандай?"),
            Some(QuestionShape::Listing)
        );
    }

    #[test]
    fn detects_comparison_questions() {
        assert_eq!(
            detect("Алматы мен Астана айырмашылығы қандай?"),
            Some(QuestionShape::Comparison)
        );
        assert_eq!(
            detect("Сіз басқа модельдерден жақсырақсыз ба?"),
            Some(QuestionShape::Comparison)
        );
    }

    #[test]
    fn non_questions_return_none() {
        assert_eq!(detect("Сәлем"), None);
        assert_eq!(detect("Менің атым Дәулет."), None);
        assert_eq!(detect("Жақсы екен"), None);
    }

    #[test]
    fn definition_does_not_swallow_listing_for_қандай_бар() {
        // `Қандай X бар?` is a listing question; `X қандай?` (without
        // "бар") is closer to a definition request. Ensure the
        // listing detector doesn't fire on the bare "X қандай?".
        assert_eq!(
            detect("Қазақстанның астанасы қандай?"),
            Some(QuestionShape::Definition)
        );
    }
}
