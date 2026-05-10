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
    //
    // **v4.72.0** — broadened from the original `қандай … бар` /
    // тізім-only matches after the live REPL battery surfaced
    // «X-тердің атаулары қандай?», «X-терді атаңыз», «барлық X-тер
    // тізімдеңіз» as common natural-Kazakh listing forms that fell
    // through to Definition shape. Each new marker is a closed-class
    // listing imperative or a list-noun that never appears in
    // singular-definition queries.
    (lower.contains("қандай") && (lower.contains(" бар") || lower.contains("тізім")))
        || lower.contains("атаулары")
        || lower.contains("атаңыз")
        || lower.contains("атап бер")
        || lower.contains("атап беріңіз")
        || lower.contains("тізімі")
        || lower.contains("тізім")
        || lower.contains("тізімдеңіз")
        || lower.contains("барлық ")
}

fn is_yes_no_check(lower: &str) -> bool {
    // Explicit confirmation markers fire first.
    if lower.contains("шынымен")
        || lower.contains("дұрыс па")
        || lower.contains("дұрыс ма")
        || lower.contains("растайсыз")
        || lower.contains("растайсың")
    {
        return true;
    }
    // **v5.4.0** — bare «<Subject> — <Predicate> (ме|ма|ба|бе|па|пе)?»
    // pattern. Pre-v5.4.0 these fell through to `Definition` and the
    // planner surfaced the most-central IsA fact about the subject,
    // ignoring the *predicate* the user asked about. The bridge-fact
    // work in v5.4.0 made transitive IsA chains reachable
    // (e.g. қасқыр → жыртқыш → жануар → тіршілік иесі → тірі); the
    // detector now routes these to YesNoCheck so the planner can
    // confirm or hedge per the chain query.
    extract_yes_no_isa_pair(lower).is_some() || extract_yes_no_property_pair(lower).is_some()
}

/// **v5.4.0** — extract `(subject, predicate)` from a bare yes/no IsA
/// question.
///
/// Recognised surface forms (after lowercasing):
///
///   - `<X> — <Y> (ме|ма|ба|бе|па|пе)?`   (em-dash separator)
///   - `<X> - <Y> (ме|ма|ба|бе|па|пе)?`   (hyphen used as dash)
///
/// Both subject and predicate may be multi-word. Returns `None` when
/// either side fails to yield a non-empty token sequence.
///
/// Leaves the broader question-shape decision to `is_yes_no_check`;
/// this helper only does the *structural* parse so the conversation
/// layer can resolve the chain in `data/retrieval/{facts,derived_facts}.json`.
pub fn extract_yes_no_isa_pair(input: &str) -> Option<(String, String)> {
    let lower = input.trim().to_lowercase();
    // Find a dash that separates two non-empty halves. Em-dash «—»
    // is the canonical Kazakh predicative separator; ASCII hyphen
    // also occurs in user input.
    let split_at = lower
        .find(" — ")
        .map(|i| (i, " — ".len()))
        .or_else(|| lower.find(" - ").map(|i| (i, " - ".len())))?;
    let (left, rest) = lower.split_at(split_at.0);
    let right = &rest[split_at.1..];
    let subject = left.trim();
    if subject.is_empty() {
        return None;
    }
    // Right side may end with the question-tag particle (and `?`).
    // Strip the particle so the predicate token doesn't include it.
    let mut predicate = right.trim_end_matches('?').trim().to_string();
    for tag in [
        " ме", " ма", " ба", " бе", " па", " пе", " ма?", " ме?", " ба?", " бе?", " па?", " пе?",
    ] {
        if let Some(stripped) = predicate.strip_suffix(tag.trim()) {
            let trimmed = stripped.trim_end();
            if !trimmed.is_empty() {
                predicate = trimmed.to_string();
                break;
            }
        }
    }
    if predicate.is_empty() {
        return None;
    }
    // Reject obviously non-noun-phrase predicates (a verb, an adjective
    // with a copula, etc.). The closed-list heuristic is conservative —
    // anything we don't recognise is allowed through and let the chain
    // query be the truth source.
    const NON_NP_PREDICATES: &[&str] = &["білесіз", "білесің", "білемін", "білдім", "бар", "жоқ"];
    if NON_NP_PREDICATES.iter().any(|w| predicate == *w) {
        return None;
    }
    Some((subject.to_string(), predicate))
}

/// **v5.5.0** — extract `(subject, predicate)` from the no-em-dash
/// property-form yes/no question.
///
/// Recognised surface form (after lowercasing):
///
///   - `<X (-genitive)?> <Y> (ме|ма|ба|бе|па|пе) ?`
///     where `<Y>` is a single-word predicate (typically an adjective
///     or short noun) and the input contains exactly TWO content
///     tokens before the question particle.
///
/// Examples that match:
///
///   - «Айдың дөңгелек пе?»          → ("ай", "дөңгелек")
///   - «Ай дөңгелек пе?»              → ("ай", "дөңгелек")
///   - «Темір қатты ма?»              → ("темір", "қатты")
///   - «Қазақстанның үлкен бе?»       → ("қазақстан", "үлкен")
///
/// Examples that don't match (delegated to em-dash form or rejected):
///
///   - «Ит — жануар ма?»              (em-dash present)
///   - «Сіз қалайсыз?»                 (no yes/no particle)
///   - «Біз қазір кешке кездесеміз бе?» (>2 content tokens — too long)
///
/// Conservative by design: only fires on the canonical 2-content-word
/// surface so the existing em-dash IsA detector and unrelated
/// patterns aren't shadowed.
pub fn extract_yes_no_property_pair(input: &str) -> Option<(String, String)> {
    let lower = input.trim().to_lowercase();
    if lower.contains(" — ") || lower.contains(" - ") {
        return None;
    }
    let trimmed = lower.trim_end_matches('?').trim();
    let last_particle = [" ме", " ма", " ба", " бе", " па", " пе"]
        .iter()
        .find(|tag| trimmed.ends_with(*tag))?;
    let body = trimmed[..trimmed.len() - last_particle.len()].trim_end();
    let tokens: Vec<&str> = body.split_whitespace().collect();
    if tokens.len() != 2 {
        return None;
    }
    // Strip optional genitive on the subject (1st token).
    let subject = strip_genitive(tokens[0]);
    let predicate = tokens[1].to_string();
    if subject.chars().count() < 2 || predicate.chars().count() < 2 {
        return None;
    }
    // Same NP-rejection list as the em-dash form so self-knowledge
    // questions don't get misrouted.
    const NON_NP_PREDICATES: &[&str] = &[
        "білесіз",
        "білесің",
        "білемін",
        "білдім",
        "бар",
        "жоқ",
        "болады",
        "болады?",
    ];
    if NON_NP_PREDICATES.iter().any(|w| predicate == *w) {
        return None;
    }
    Some((subject, predicate))
}

/// **v5.5.0** — strip the Kazakh genitive marker (`-ның / -нің /
/// -дың / -дің / -тың / -тің`) from a single token so the bare root
/// can be looked up in the typed-fact graph. Conservative — only
/// strips when the residue is at least 2 chars.
fn strip_genitive(token: &str) -> String {
    const GENITIVE_SUFFIXES: &[&str] = &["ның", "нің", "дың", "дің", "тың", "тің"];
    for suf in GENITIVE_SUFFIXES {
        if let Some(stem) = token.strip_suffix(suf) {
            if stem.chars().count() >= 2 {
                return stem.to_string();
            }
        }
    }
    token.to_string()
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
    fn detects_bare_yes_no_isa_questions() {
        // v5.4.0 — bare predicative pattern routes to YesNoCheck so
        // the planner can query the IsA chain instead of surfacing a
        // tangential definition.
        assert_eq!(detect("Қасқыр — тірі ме?"), Some(QuestionShape::YesNoCheck));
        assert_eq!(detect("Балта — зат па?"), Some(QuestionShape::YesNoCheck));
        assert_eq!(detect("Ит — жануар ма?"), Some(QuestionShape::YesNoCheck));
    }

    #[test]
    fn extracts_yes_no_isa_pair_with_em_dash() {
        assert_eq!(
            extract_yes_no_isa_pair("Қасқыр — тірі ме?"),
            Some(("қасқыр".into(), "тірі".into()))
        );
        assert_eq!(
            extract_yes_no_isa_pair("Жер сілкінісі — құбылыс па?"),
            Some(("жер сілкінісі".into(), "құбылыс".into()))
        );
    }

    #[test]
    fn extracts_yes_no_isa_pair_with_hyphen() {
        assert_eq!(
            extract_yes_no_isa_pair("ит - үй жануары ма?"),
            Some(("ит".into(), "үй жануары".into()))
        );
    }

    #[test]
    fn rejects_non_noun_phrase_predicates() {
        // «X — білемін бе?» is a self-knowledge question, not an IsA
        // query; should not extract a (subject, predicate) pair.
        assert_eq!(extract_yes_no_isa_pair("Мен — білемін бе?"), None);
    }

    #[test]
    fn extracts_property_pair_genitive_subject_v550() {
        // **v5.5.0** — adjective-as-predicate, genitive subject form.
        // Pre-v5.5.0 «Айдың дөңгелек пе?» had no em-dash and so the
        // v5.4.0 detector returned None; the question fell to the
        // retrieval path and surfaced a tangential proverb.
        assert_eq!(
            extract_yes_no_property_pair("Айдың дөңгелек пе?"),
            Some(("ай".into(), "дөңгелек".into()))
        );
        assert_eq!(
            extract_yes_no_property_pair("Қазақстанның үлкен бе?"),
            Some(("қазақстан".into(), "үлкен".into()))
        );
    }

    #[test]
    fn extracts_property_pair_bare_subject_v550() {
        // **v5.5.0** — adjective-as-predicate, bare-subject form.
        // «Ай дөңгелек пе?» / «Темір қатты ма?» — no genitive marker,
        // still extracts the pair.
        assert_eq!(
            extract_yes_no_property_pair("Ай дөңгелек пе?"),
            Some(("ай".into(), "дөңгелек".into()))
        );
        assert_eq!(
            extract_yes_no_property_pair("Темір қатты ма?"),
            Some(("темір".into(), "қатты".into()))
        );
        assert_eq!(
            extract_yes_no_property_pair("Су сұйық па?"),
            Some(("су".into(), "сұйық".into()))
        );
    }

    #[test]
    fn property_pair_rejects_em_dash_form_v550() {
        // **v5.5.0** — em-dash form is delegated to extract_yes_no_isa_pair;
        // the property-form detector must not shadow it.
        assert_eq!(extract_yes_no_property_pair("Ит — жануар ма?"), None);
    }

    #[test]
    fn property_pair_rejects_long_inputs_v550() {
        // **v5.5.0** — only fires on exactly 2 content tokens before
        // the question particle; longer inputs would over-match.
        assert_eq!(
            extract_yes_no_property_pair("Біз қазір кешке кездесеміз бе?"),
            None
        );
    }

    #[test]
    fn detects_property_yes_no_questions_v550() {
        // **v5.5.0** — detect() routes the property form to YesNoCheck
        // alongside the v5.4.0 em-dash IsA form.
        assert_eq!(
            detect("Айдың дөңгелек пе?"),
            Some(QuestionShape::YesNoCheck)
        );
        assert_eq!(detect("Темір қатты ма?"), Some(QuestionShape::YesNoCheck));
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
