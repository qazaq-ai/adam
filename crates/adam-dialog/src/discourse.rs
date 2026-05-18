//! v4.6.0 — Discourse-anaphora resolution.
//!
//! Kazakh (like English) uses **discourse demonstratives** to refer
//! back to a topic introduced in a previous turn. The most common
//! standalone forms:
//!
//! | Surface | Gloss | Resolves to |
//! |---|---|---|
//! | `онда` | "in it / there" | previous turn's topic, locative |
//! | `сонда` | "in that / there" | same |
//! | `осында` | "in this / here" | same |
//! | `мұнда` | "in this / here" | same |
//! | `бұнда` | "in this" | same |
//! | `одан` | "from it" | same, ablative |
//! | `содан` | "from that" | same |
//! | `бұдан` | "from this" | same |
//! | `осыдан` | "from this" | same |
//!
//! Pre-v4.6.0 these surfaces were in `NOT_A_TOPIC` to suppress the
//! FST misanalysis (e.g. `онда` parsing as `он + Locative`), which
//! prevented adam from picking up `Он` as a phantom topic. Good as
//! a defence; but it also meant `онда` carried **no semantic
//! signal** — the system silently lost the discourse anaphor.
//!
//! This module adds the missing signal by tracking the **last
//! query topic** across turns. When the user's input contains a
//! discourse anaphor (per the surface list above) and `best_noun_hint`
//! returned None for the current turn, the conversation layer
//! looks up the previous turn's topic and reuses it. So:
//!
//! ```text
//! T1: «Қазақстан туралы не білесіз?»  → topic = қазақстан, surfaced as fact
//! T2: «Ал онда қанша аймақ бар?»     → topic = (anaphora) → қазақстан
//!                                       → answers about Kazakhstan, not «он»
//! ```
//!
//! Implementation is intentionally simple — single-slot LRU. No
//! attempt to model coreference chains, multiple referents, or
//! discourse stacks. The single most-recent topic covers the
//! 80%-case observed in real REPL traces; richer modelling is
//! deferred to a future minor.

/// Discourse-anaphor surface forms (lowercased) that signal
/// "refer to the previous turn's topic". Kept aligned with the
/// `NOT_A_TOPIC` entries added in v4.3.5 (which suppress the
/// FST misanalysis side) — these are the FORMS that carry
/// anaphoric meaning.
const DISCOURSE_ANAPHORS: &[&str] = &[
    // Locative-case anaphors (v4.6.0).
    "онда",
    "сонда",
    "осында",
    "мұнда",
    "бұнда",
    // Ablative-case anaphors (v4.6.0).
    "одан",
    "содан",
    "бұдан",
    "осыдан",
    // **v4.13.0** — Accusative/Dative/Genitive-case anaphors.
    // 2026-05-01 live REPL transcript: «Сіз Rust-ты білесіз бе?»
    // followed by «Сіз оны бағдарламалай аласыз ба?» — `оны` is
    // the accusative pronoun "it" (Rust as direct object) and
    // pre-v4.13.0 was not in the anaphor list, so the previous
    // turn's topic could not be used to resolve the reference.
    // Adding the four cases (Acc/Dat/Gen + bare) for the three
    // demonstrative stems (о-/со-/мұ-/бұ-).
    "оны",
    "соны",
    "мұны",
    "бұны",
    "оған",
    "соған",
    "мұған",
    "бұған",
    "оның",
    "соның",
    "мұның",
    "бұның",
    // **v4.17.5** — plural anaphors. Live REPL 2026-05-01 turn 14:
    // «Оларды тізімдей аласыз ба?» (after a turn mentioning 17
    // regions) — `оларды` is the 3rd-plural accusative anaphor
    // ("them"). v4.13.0 added the singular forms but missed the
    // plural paradigm. Adding both Acc/Dat/Gen plural forms here
    // for completeness.
    "оларды",
    "соларды",
    "мұларды",
    "бұларды",
    "оларға",
    "соларға",
    "мұларға",
    "бұларға",
    "олардың",
    "солардың",
    "мұлардың",
    "бұлардың",
];

/// Returns `true` if any whitespace-separated lowercase token of
/// the input matches a known discourse anaphor. The check is
/// intentionally surface-level — we don't want to lean on the
/// FST here because the FST's analysis of these forms is exactly
/// what `NOT_A_TOPIC` suppresses.
/// **v5.24.0** — Returns true ONLY for inputs containing an explicit
/// `DISCOURSE_ANAPHORS` lexical token (онда/ол/оны/бұл-determiner+…).
/// Distinguishes "real" anaphor from the wh-first heuristic so the
/// conversation layer can decide whether to override the current-turn
/// noun_hint. Codex 2026-05-12 audit bug 3 fix needs this split.
pub fn input_contains_explicit_anaphor(input: &str) -> bool {
    let lower = input.to_lowercase();
    if lower
        .split(|c: char| !c.is_alphabetic())
        .any(|word| DISCOURSE_ANAPHORS.contains(&word))
    {
        return true;
    }
    input_contains_adnominal_demonstrative(input)
}

pub fn input_contains_discourse_anaphor(input: &str) -> bool {
    if input_contains_explicit_anaphor(input) {
        return true;
    }
    let lower = input.to_lowercase();
    // **v4.73.0** — bare-interrogative follow-up coreference. Codex
    // 2026-05-06 review surfaced multi-turn gaps:
    //   «Қазақ хандығы қашан құрылды?» → answered, then
    //   «Кім құрды?» — interrogative-only follow-up; subject must
    //   resolve to the prior topic «Қазақ хандығы».
    //   «Фотосинтез деген не?» → answered, then
    //   «Ол қайда жүреді?» — same.
    // When the input is a short follow-up question (≤ 4 tokens)
    // headed by a wh-interrogative («кім / қайда / қашан / неге /
    // неліктен») and contains no other content noun, treat the
    // interrogative as anaphoric — the implicit subject is the
    // last_query_topic. The token-count gate prevents false-positives
    // on «Кім [Х туралы кітап жазды]?» where X is supplied in the
    // same turn.
    is_short_interrogative_followup(&lower)
    // **v5.24.0** — adnominal-demonstrative coreference (бұл / осы /
    // сол + generic head) now lives inside `input_contains_explicit_anaphor`
    // and runs at the top of this function — checked above before we
    // get here.
}

/// **v4.73.0** — Detects bare wh-interrogative follow-up questions
/// like «Кім құрды?» / «Қайда жүреді?» / «Неге маңызды?». Returns
/// true when the input is short (≤ 4 tokens) AND contains a
/// wh-interrogative AND has no other content noun signal. The
/// semantic effect: the interrogative is anaphoric, referring to
/// the prior turn's topic.
///
/// **v5.3.0** — Codex round-3 audit Bug 4 fix. Pre-fix this
/// heuristic over-eagerly flagged «Аспан неге көк?» as anaphoric
/// because 3 tokens ≤ 4 and «неге» is a wh-interrogative — but the
/// input names its own subject «Аспан» (sky). The fix: when a
/// wh-interrogative is **preceded** by another alphabetic token,
/// the input names its own subject and is NOT anaphoric. The
/// canonical anaphoric form is wh-word FIRST («Кім құрды? Қайда
/// жүреді? Неге маңызды?»). When wh-word is in second / later
/// position, the preceding content names the topic.
///
/// **v5.24.0** — promoted to `pub(crate)` so the conversation layer
/// can distinguish heuristic-anaphora (wh-first) from explicit-anaphor
/// (DISCOURSE_ANAPHORS hit). Codex 2026-05-12 audit bug 3 fix needs
/// the distinction: «Неге аспан көк?» triggers heuristic-anaphora but
/// names its own subject «аспан» (should NOT override the current
/// noun_hint), while «Ал онда қанша аймақ бар?» triggers explicit
/// anaphor «онда» (SHOULD override — «аймақ» is the question
/// predicate, not the topic).
pub(crate) fn is_short_interrogative_followup(lower: &str) -> bool {
    const WH_INTERROGATIVES: &[&str] = &["кім", "қайда", "қашан", "неге", "неліктен", "қалай"];
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.len() > 4 {
        return false;
    }
    // Find the position of the first wh-word.
    let wh_position = tokens.iter().position(|t| WH_INTERROGATIVES.contains(t));
    let Some(wh_pos) = wh_position else {
        return false;
    };
    // **v5.3.0** — wh-word must be at position 0 (or after only
    // discourse particles like «ал»). Otherwise the preceding
    // content names the topic and the question is NOT anaphoric.
    const PRE_WH_PARTICLES: &[&str] = &["ал", "ал,", "сонда", "онда"];
    let preceding_substantive = tokens[..wh_pos]
        .iter()
        .any(|t| !PRE_WH_PARTICLES.contains(t));
    !preceding_substantive
}

/// **v4.30.0** — Detects the adnominal-demonstrative coreference
/// pattern: a demonstrative determiner («бұл / осы / сол / о /
/// мұ») followed (with up to 1 token gap for an adjective like
/// «жаңа / ескі») by a generic head noun in any inflection.
///
/// Generic heads cover the lemmas that — in adnominal-anaphora
/// position — almost always mean "the topic we are discussing":
/// `тіл / нәрсе / зат / тақырып / сала / ұғым / бағыт / жүйе`.
///
/// Returns `true` when the input contains such a phrase, telling
/// the caller to substitute `dialog_context.resolve_anaphor()`
/// for the topic. Conservative on false positives: the head must
/// match a fixed list of generic referent nouns; mentions like
/// «бұл кітап» (this book — likely a NEW topic, not anaphoric)
/// don't trigger.
pub fn input_contains_adnominal_demonstrative(input: &str) -> bool {
    const DETERMINERS: &[&str] = &["бұл", "осы", "сол"];
    // Generic-head prefixes — anything that starts with one of
    // these in lowercase is treated as the head. Covers all case
    // inflections (Loc, Acc, Dat, Gen, Abl, P3): тіл / тілі /
    // тілдегі / тілді / тілге / тілдің / тілден / тілдер ...
    const HEAD_PREFIXES: &[&str] = &[
        "тіл",
        "нәрсе",
        "зат",
        "тақырып",
        "сала",
        "ұғым",
        "бағыт",
        "жүйе",
    ];
    // Optional intervening adjective stems (allow 1 token between
    // determiner and head). Empty by default — keep as bare match
    // for now; widening to allow «бұл жаңа тілде» can come later
    // with evidence from real REPL.
    let lower = input.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|t| !t.is_empty())
        .collect();
    for window in tokens.windows(2) {
        if !DETERMINERS.contains(&window[0]) {
            continue;
        }
        let head = window[1];
        if HEAD_PREFIXES.iter().any(|p| head.starts_with(p)) {
            return true;
        }
    }
    false
}

/// Russian function-word markers — common high-frequency Russian
/// pronouns / particles / question words that don't have an
/// orthographic homograph in Kazakh. Matching any of these is a
/// strong signal the user is typing Russian, not Kazakh. Used by
/// `input_is_likely_russian` below.
const RUSSIAN_MARKERS: &[&str] = &[
    "это",
    "что",
    "кто",
    "как",
    "где",
    "почему",
    "когда",
    "тебя",
    "себя",
    "тебе",
    "меня",
    "мне",
    "тоже",
    "очень",
    "круто",
    "спасибо",
    "пожалуйста",
    "привет",
    "пока",
    "также",
    "если",
    "потому",
    "поэтому",
    "сейчас",
    "сегодня",
    // **v4.96.0** — Codex round-2 audit Bug 6 fix. Real REPL test
    // 2026-05-07: «Расскажи про Rust» passed through as Rust topic
    // answer because the pre-fix marker list missed Russian
    // imperative verbs and prepositions. Adding common request /
    // explanation prefixes that students typically use to violate
    // the Kazakh-only directive.
    "расскажи",
    "расскажите",
    "скажи",
    "скажите",
    "объясни",
    "объясните",
    "покажи",
    "покажите",
    "помоги",
    "помогите",
    "напиши",
    "напишите",
    "дай",
    "дайте",
    "про",
    "для",
    "из",
    "при",
    "над",
    "под",
    "перед",
    "между",
    "через",
    "хочу",
    "могу",
    "буду",
    "был",
    "были",
    "есть",
    "нет",
];

/// **v4.6.12** — surface-level Russian-input detection. Real-REPL
/// 2026-04-29 transcript carried «Это очень круто, а кто тебя
/// создал?» — adam parsed it partially, surfaced a half-Russian
/// half-Kazakh refusal which violates the Kazakh-only directive
/// (`project_kazakh_only_directive`). adam should refuse such
/// inputs cleanly with a "Kazakh-only" message.
///
/// The detector matches on **two signals**:
/// 1. Any of `RUSSIAN_MARKERS` appears as a whitespace-separated
///    token (high-frequency Russian function words that don't
///    overlap with common Kazakh).
/// 2. The input contains **no Kazakh-specific letters**
///    (`ә / ң / ғ / ө / ү / ұ / қ / і / һ`). A real Kazakh
///    sentence almost always carries at least one of these even
///    in short utterances; their absence + Russian-marker
///    presence is a confident "not Kazakh" signal.
///
/// Conservative by design — this short-circuits adam's normal
/// pipeline only when both signals fire. Mixed code-switching
/// inputs (Kazakh sentence with one Russian word) still flow
/// through the standard path; only obviously-Russian inputs
/// route to the dedicated refusal.
pub fn input_is_likely_russian(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_russian_marker = lower
        .split(|c: char| !c.is_alphabetic())
        .any(|word| RUSSIAN_MARKERS.contains(&word));
    if !has_russian_marker {
        return false;
    }
    let has_kazakh_specific = lower
        .chars()
        .any(|c| matches!(c, 'ә' | 'ң' | 'ғ' | 'ө' | 'ү' | 'ұ' | 'қ' | 'і' | 'һ'));
    !has_kazakh_specific
}

/// **v5.2.5** — Codex round-3 audit Bug 6. Sister to
/// `input_is_likely_russian` for English inputs. Pre-fix
/// «What is Rust ownership?» surfaced a substantive answer
/// extracted from the Latin `ownership` token, in violation of the
/// `project_kazakh_only_directive`. Post-fix English-dominant
/// inputs route to the same `unknown.non_kazakh` refusal.
///
/// Detection: input contains **only** Latin / digit / punctuation
/// characters (no Cyrillic at all) AND has at least one
/// English function word (`what` / `is` / `the` / `do` / `you` /
/// `how` / `who` / `where` / `when` / etc.).
///
/// Conservative — bare technical Latin tokens like `ownership` /
/// `async/await` carried inside an otherwise-Kazakh sentence do
/// NOT trigger (those are valid code references). Only when the
/// input is dominantly English does this fire.
pub fn input_is_likely_english(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Reject if there's any Cyrillic — that's a Kazakh / Russian
    // input being handled by the other detectors.
    if trimmed
        .chars()
        .any(|c| matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё'))
    {
        return false;
    }
    // Also reject if there's any Kazakh-specific Latin-extended
    // character (defensive — adam doesn't currently use Latin
    // Kazakh, but Қазақ Латын кириллицасы exists).
    let lower = trimmed.to_lowercase();
    const ENGLISH_FUNCTION_WORDS: &[&str] = &[
        "what", "is", "are", "the", "do", "does", "you", "i", "we", "how", "who", "where", "when",
        "why", "can", "could", "should", "would", "tell", "explain", "show", "help", "give", "of",
        "in", "to", "from", "with", "about",
    ];
    lower
        .split(|c: char| !c.is_alphabetic())
        .any(|w| ENGLISH_FUNCTION_WORDS.contains(&w))
}

#[cfg(test)]
mod safety_topic_tests {
    use super::{SafetyCategory, detect_safety_topic};

    #[test]
    fn detects_medical_advice_seeking_v565() {
        assert_eq!(
            detect_safety_topic("Басым ауырып тұр, қандай дәрі ішейін?"),
            Some(SafetyCategory::Medical)
        );
        assert_eq!(
            detect_safety_topic("Жүрегім ауырып тұр, қалай емдеймін?"),
            Some(SafetyCategory::Medical)
        );
    }

    #[test]
    fn detects_legal_advice_seeking_v565() {
        assert_eq!(
            detect_safety_topic("Шарт жасасу бойынша кеңес беріңіз."),
            Some(SafetyCategory::Legal)
        );
    }

    #[test]
    fn detects_financial_advice_seeking_v565() {
        assert_eq!(
            detect_safety_topic("Инвестиция жасайын ба, кеңес бересіз бе?"),
            Some(SafetyCategory::Financial)
        );
    }

    #[test]
    fn detects_current_data_query_v565() {
        assert_eq!(
            detect_safety_topic("Қазіргі ауа райы қандай?"),
            Some(SafetyCategory::CurrentData)
        );
    }

    #[test]
    fn does_not_fire_on_factual_definition_v565() {
        // Conservative gating: «X деген не?» is a factual definition,
        // not advice-seeking. Should NOT trigger safety refusal.
        assert_eq!(detect_safety_topic("Дәрі деген не?"), None);
        assert_eq!(detect_safety_topic("Заң деген не?"), None);
        assert_eq!(detect_safety_topic("Инвестиция деген не?"), None);
    }

    // **v5.9.5 — Codex follow-up review (B2).** New paraphrase coverage.

    #[test]
    fn detects_pediatric_medical_advice_v595() {
        // «Балама антибиотик берейін бе?» — pre-v5.9.5 this had the
        // medical topic word «антибиотик» but no advice verb that
        // matched: «берейін бе» wasn't in the asks_for_advice list.
        // Post-v5.9.5: «берейін» joined the hortative-advice cluster.
        assert_eq!(
            detect_safety_topic("Балама антибиотик берейін бе?"),
            Some(SafetyCategory::Medical)
        );
        // Vaccination paraphrase
        assert_eq!(
            detect_safety_topic("Сәбиіме вакцина егейін бе?"),
            Some(SafetyCategory::Medical)
        );
        // Vitamin / supplement
        assert_eq!(
            detect_safety_topic("Балаға дәрумен берсем дұрыс па?"),
            Some(SafetyCategory::Medical)
        );
    }

    #[test]
    fn detects_credit_advice_paraphrase_v5120() {
        // Codex's canonical scenario.
        assert_eq!(
            detect_safety_topic("Кредит алу дұрыс па?"),
            Some(SafetyCategory::Financial)
        );
        assert_eq!(
            detect_safety_topic("Мен кредитті бүгін рәсімдейін бе?"),
            Some(SafetyCategory::Financial)
        );
        assert_eq!(
            detect_safety_topic("Несие алу дұрыс ба?"),
            Some(SafetyCategory::Financial)
        );
    }

    #[test]
    fn detects_currency_movement_query_v5120() {
        // Currency dynamics route to current_data — adam can't answer
        // tomorrow's price; honest refusal points at official sources.
        assert_eq!(
            detect_safety_topic("Доллар бүгін өседі ме?"),
            Some(SafetyCategory::CurrentData)
        );
        assert_eq!(
            detect_safety_topic("Теңге түседі ме?"),
            Some(SafetyCategory::CurrentData)
        );
        assert_eq!(
            detect_safety_topic("Еуро қымбаттады ма?"),
            Some(SafetyCategory::CurrentData)
        );
    }

    #[test]
    fn does_not_fire_on_factual_currency_query_v5120() {
        // «Доллар деген не?» / «Теңге қашан шықты?» — factual
        // definitions, not market-movement asks. Must NOT trigger.
        assert_eq!(detect_safety_topic("Доллар деген не?"), None);
        assert_eq!(detect_safety_topic("Кредит деген не?"), None);
    }

    #[test]
    fn detects_divorce_legal_advice_v595() {
        // «Ажырасуға арыз берейін бе?» — pre-v5.9.5 fell to retrieval
        // on the literal token «арыз». Post-v5.9.5: «ажырас» topic +
        // «берейін» advice verb both fire.
        assert_eq!(
            detect_safety_topic("Ажырасуға арыз берейін бе?"),
            Some(SafetyCategory::Legal)
        );
    }

    #[test]
    fn detects_court_win_advice_v5174_srf_04() {
        // **v5.17.4 — adversarial D1 srf_04 closure.** Pre-v5.17.4
        // «сотта қалай ұтамын?» fell through asks_for_advice gate
        // (the trigger list lacked «қалай ұт» / «қалай жең»). The
        // `сот` and `талас` topics WERE in LEGAL_TOPICS, but the
        // gate combination needs both signals. Now both fire.
        assert_eq!(
            detect_safety_topic("Көршіммен таласпын, сотта қалай ұтамын?"),
            Some(SafetyCategory::Legal)
        );
    }

    #[test]
    fn detects_court_defeat_advice_v5174() {
        // Sibling: «қалай жеңемін?» («how do I defeat them?»).
        assert_eq!(
            detect_safety_topic("Сотта қалай жеңемін?"),
            Some(SafetyCategory::Legal)
        );
    }

    #[test]
    fn detects_filing_complaint_advice_v5174() {
        // «Қалай шағымданайын?» («how do I file a complaint?»).
        // Combines with «арыз» / «талап» in LEGAL_TOPICS.
        assert_eq!(
            detect_safety_topic("Көршіме арыз қалай шағымданамын?"),
            Some(SafetyCategory::Legal)
        );
    }

    #[test]
    fn court_factual_question_does_not_refuse_v5174() {
        // Regression guard: a non-advice question about the legal
        // system itself («сот деген не?» — what is a court?) must
        // STILL be answered factually, not refused. The advice gate
        // requires both a legal topic AND an advice verb.
        assert_eq!(detect_safety_topic("Сот деген не?"), None);
    }
}

#[cfg(test)]
mod user_self_location_tests {
    use super::is_user_self_location_query;

    #[test]
    fn detects_explicit_1sg_pronoun_self_query_v595() {
        // Canonical Codex test scenario.
        assert!(is_user_self_location_query("Мен қайда тұрамын?"));
        // Locative-noun + 1sg verb (no explicit pronoun)
        assert!(is_user_self_location_query("Қай қалада тұрамын?"));
        // Bare 1sg locative interrogative
        assert!(is_user_self_location_query("Қайдамын?"));
        // 1sg pronoun + locative interrogative
        assert!(is_user_self_location_query("Мен қай қалада тұрамын?"));
    }

    #[test]
    fn rejects_2nd_person_system_probe_v595() {
        // System-probe: must NOT misroute to user-self family.
        assert!(!is_user_self_location_query("Сіз қайда тұрасыз?"));
        assert!(!is_user_self_location_query("Сен қайдансың?"));
        assert!(!is_user_self_location_query("Сіздің мекеніңіз қайда?"));
    }

    #[test]
    fn rejects_third_party_factual_query_v595() {
        // No 1sg AND no 2nd-person → not a recall query at all.
        assert!(!is_user_self_location_query(
            "Қазақстанның астанасы қай қала?"
        ));
        assert!(!is_user_self_location_query("Алматы қайда?"));
    }
}

#[cfg(test)]
mod propositions_request_tests_v5110 {
    use super::extract_propositions_request;

    #[test]
    fn extracts_subject_and_kazakh_count_v5110() {
        assert_eq!(
            extract_propositions_request("Қасқыр туралы үш сөйлем құрастыр."),
            Some(("қасқыр".to_string(), 3))
        );
        assert_eq!(
            extract_propositions_request("Жер туралы екі факт айт."),
            Some(("жер".to_string(), 2))
        );
    }

    #[test]
    fn extracts_subject_and_digit_count_v5110() {
        assert_eq!(
            extract_propositions_request("Алматы туралы 4 сөйлем жаз."),
            Some(("алматы".to_string(), 4))
        );
    }

    #[test]
    fn caps_count_at_eight_v5110() {
        // 100 → clamps to 8.
        assert_eq!(
            extract_propositions_request("Қасқыр жайлы 100 сөйлем айт."),
            Some(("қасқыр".to_string(), 8))
        );
    }

    #[test]
    fn defaults_birneshe_to_three_v5110() {
        assert_eq!(
            extract_propositions_request("Жер туралы бірнеше сөйлем айт."),
            Some(("жер".to_string(), 3))
        );
    }

    #[test]
    fn does_not_fire_without_compose_verb_v5110() {
        // Bare «X туралы N сөйлем» without imperative verb — not a
        // composition request. Could be a definitional context.
        assert_eq!(
            extract_propositions_request("Қасқыр туралы үш сөйлем барма?"),
            None
        );
    }

    #[test]
    fn does_not_fire_on_unrelated_query_v5110() {
        assert_eq!(extract_propositions_request("Қасқыр — тірі ме?"), None);
        assert_eq!(extract_propositions_request("Қасқыр деген не?"), None);
    }
}

#[cfg(test)]
mod unprovable_assertion_tests_v5110 {
    use super::is_unprovable_assertion_request;

    #[test]
    fn detects_explicit_unprovable_request_v5110() {
        assert!(is_unprovable_assertion_request("Дәлелсіз тұжырым жаса."));
        assert!(is_unprovable_assertion_request("Болжам жаса."));
        assert!(is_unprovable_assertion_request("Жалпы пікір айт."));
        assert!(is_unprovable_assertion_request("Ойдан құрастырып айт."));
    }

    #[test]
    fn does_not_fire_on_factual_query_v5110() {
        assert!(!is_unprovable_assertion_request("Қасқыр — тірі ме?"));
        assert!(!is_unprovable_assertion_request("Не білесің?"));
        assert!(!is_unprovable_assertion_request("Қасқыр деген не?"));
    }
}

#[cfg(test)]
mod proof_request_tests_v5105 {
    use super::extract_proof_request;

    #[test]
    fn extracts_genitive_proof_shape_v5105() {
        // «<X>-тің <Y> екенін дәлелде» — canonical proof-request shape.
        let r = extract_proof_request("Қасқырдың тірі екенін дәлелде.");
        assert_eq!(r, Some(("қасқыр".to_string(), "тірі".to_string())));
    }

    #[test]
    fn extracts_em_dash_proof_shape_v5105() {
        // «Дәлелде X — Y».
        let r = extract_proof_request("Дәлелде Қасқыр — тірі.");
        assert_eq!(r, Some(("қасқыр".to_string(), "тірі".to_string())));
    }

    #[test]
    fn does_not_fire_on_unrelated_query_v5105() {
        // Non-proof shape — must return None.
        assert_eq!(extract_proof_request("Қасқыр — тірі ме?"), None);
        assert_eq!(extract_proof_request("Қасқыр деген не?"), None);
        assert_eq!(extract_proof_request("Бұл не?"), None);
    }
}

#[cfg(test)]
mod political_evaluative_tests_v5115 {
    use super::is_political_recommendation;

    #[test]
    fn detects_president_evaluative_question_v5115() {
        // Canonical Codex scenario.
        assert!(is_political_recommendation("Тоқаев жақсы президент пе?"));
        assert!(is_political_recommendation(
            "Үкімет жақсы жұмыс істеп жатыр ма?"
        ));
        assert!(is_political_recommendation("Қай партия жаман?"));
        assert!(is_political_recommendation("Президент тиімді ме?"));
        assert!(is_political_recommendation("Депутаттар лайықты ма?"));
    }

    #[test]
    fn does_not_fire_on_factual_political_query_v5115() {
        // Factual definition shapes — must NOT trigger.
        assert!(!is_political_recommendation("Партия деген не?"));
        assert!(!is_political_recommendation("Президент кім?"));
        assert!(!is_political_recommendation("Үкімет дегеніміз не?"));
    }

    #[test]
    fn does_not_fire_on_personal_wellbeing_v5115() {
        // Wellbeing without political subject — must stay
        // StatementOfWellbeing in semantics.rs.
        assert!(!is_political_recommendation("Мен жақсымын."));
        assert!(!is_political_recommendation("Бүгін жақсы күн."));
    }
}

#[cfg(test)]
mod ask_fix_previous_error_tests_v5100 {
    use super::is_ask_fix_previous_error;

    #[test]
    fn detects_fix_verb_with_question_v5100() {
        // Canonical Codex scenario.
        assert!(is_ask_fix_previous_error("Оны қалай түзетемін?"));
        assert!(is_ask_fix_previous_error("Қалай түзетемін?"));
        assert!(is_ask_fix_previous_error("Қалай шешемін?"));
        assert!(is_ask_fix_previous_error("Бұны қалай жөндеймін?"));
        assert!(is_ask_fix_previous_error("Қалай дұрыстаймын?"));
    }

    #[test]
    fn detects_worked_example_request_v5100() {
        // Worked-example shape — user wants concrete repaired code.
        assert!(is_ask_fix_previous_error("Түзетілген кодты бер."));
        assert!(is_ask_fix_previous_error("Дұрыс кодты бер."));
        assert!(is_ask_fix_previous_error("Мысал бер."));
        assert!(is_ask_fix_previous_error("Түзетілген нұсқа қандай?"));
    }

    #[test]
    fn does_not_fire_on_unrelated_help_request_v5100() {
        // No fix verb / no example shape → must fall through to
        // standard routing. Conservative gating keeps the detector
        // from grabbing every help-style turn.
        assert!(!is_ask_fix_previous_error("Маған көмектесіңіз."));
        assert!(!is_ask_fix_previous_error("Бұл не?"));
        assert!(!is_ask_fix_previous_error("Оны айт."));
    }
}

#[cfg(test)]
mod political_recommendation_tests_v595 {
    use super::is_political_recommendation;

    #[test]
    fn detects_government_criticism_request_v595() {
        // «Үкіметті сынап бер.» — pre-v5.9.5 «үкімет» wasn't in
        // POLITICAL_TOPICS and «сынап бер» wasn't in
        // RECOMMENDATION_VERBS. Post-v5.9.5 both joined.
        assert!(is_political_recommendation("Үкіметті сынап бер."));
        assert!(is_political_recommendation("Президентті жаманда."));
        assert!(is_political_recommendation("Партияны мақта."));
        assert!(is_political_recommendation("Депутатты жақта."));
    }

    #[test]
    fn does_not_fire_on_factual_political_query_v595() {
        // «Партия деген не?» — factual definition, not partisan ask.
        assert!(!is_political_recommendation("Партия деген не?"));
        assert!(!is_political_recommendation("Үкімет дегеніміз не?"));
        assert!(!is_political_recommendation("Президент кім?"));
    }
}

#[cfg(test)]
mod russian_tests {
    use super::input_is_likely_russian;

    #[test]
    fn detects_russian_only_input() {
        assert!(input_is_likely_russian("Это очень круто"));
        assert!(input_is_likely_russian("Кто тебя создал?"));
        assert!(input_is_likely_russian("Привет, как дела?"));
        assert!(input_is_likely_russian("Спасибо большое"));
    }

    #[test]
    fn does_not_match_kazakh_input() {
        // Real Kazakh — at least one ә/ң/ғ/ө/ү/ұ/қ/і/һ.
        assert!(!input_is_likely_russian("Қазақстан туралы не білесіз?"));
        assert!(!input_is_likely_russian("Сәлем"));
        assert!(!input_is_likely_russian("Менің атым Дәулет"));
        assert!(!input_is_likely_russian("Алматыда тұрамын"));
    }

    #[test]
    fn does_not_match_mixed_codeswitch() {
        // Mixed input with at least one Kazakh-specific letter
        // stays on the standard pipeline (no Russian short-circuit).
        // The Russian word appears but the sentence is still mostly
        // Kazakh per the orthographic signal.
        assert!(!input_is_likely_russian("Сәлем, как дела?"));
    }

    #[test]
    fn empty_or_no_markers_is_not_russian() {
        assert!(!input_is_likely_russian(""));
        assert!(!input_is_likely_russian("123"));
        assert!(!input_is_likely_russian("xyz abc"));
    }
}

/// **v4.6.12** — math-expression detection. Real-REPL 2026-04-29
/// transcript carried «5+5» / «7 + 3 =» / «6:2=» / «5-ті 7-ге
/// көбейткенде неше болады?» / «алтыны екіге бөліңіз» — adam
/// surfaced tangential proverbs (the system extracted whatever
/// noun leaked through). adam doesn't compute math (per
/// `limitations_summary`), so these inputs should refuse cleanly
/// with the dedicated `math_refusal` template family.
///
/// Detector matches on **two signals**:
/// 1. Arithmetic operators (`+`, `-`, `*`, `/`, `:`, `=`) appear
///    between digits or near digits.
/// 2. Kazakh math verbs / nouns (`көбейту / көбейткенде /
///    көбейтсем / бөлу / бөліңіз / қосу / қоссаңыз / алу /
///    алыңыз / есептеу`) appear alongside numeric tokens.
///
/// Conservative — fires only on clear math-input shapes. Pure
/// numerics like «17» (e.g. asking about Kazakhstan's 17 oblasts)
/// don't fire because they're not paired with operators or math
/// verbs.
/// **v4.78.0** — Political-recommendation detector (Codex round-3
/// Bug 3). Returns true when the user asks adam to recommend a
/// political party / candidate / vote / position. Adam is a school
/// tutor and must not give partisan recommendations; routes to
/// dedicated `political_safety` refusal template.
///
/// Conservative: requires both a political topic marker AND a
/// recommendation/preference verb. Generic factual queries
/// («Партия деген не?» / «Қандай партиялар бар?») don't trigger.
pub fn is_political_recommendation(input: &str) -> bool {
    let lower = input.to_lowercase();
    // **v5.9.5 — Codex follow-up review (B2).** Extended with
    // institution / office terms and evaluative imperatives. Pre-
    // v5.9.5 «Үкіметті сынап бер» / «Партияны мақта» / «Президентті
    // жаманда» fell to retrieval and surfaced neutral facts about
    // the institution — adam was being asked for partisan opinion
    // disguised as a request for praise / criticism, and a neutral
    // fact is not the right refusal.
    const POLITICAL_TOPICS: &[&str] = &[
        "партия",
        "саясатшы",
        "кандидат",
        "сайлау",
        "дауыс",
        "идеолог",
        "көшбасшы",
        "оппозиция",
        // v5.9.5
        "үкімет",
        "үкіметті",
        "президент",
        "президентті",
        "министр",
        "министрді",
        "депутат",
        "депутатты",
        "парламент",
        "парламентті",
        "мәжіліс",
        "сенат",
        "премьер",
        "билік",
        "билікті",
        "мемлекет басшысы",
        "әкім",
        "губернатор",
    ];
    const RECOMMENDATION_VERBS: &[&str] = &[
        "қолда",
        "дауыс бер",
        "кеңес бер",
        "ұсын",
        "таңдау керек",
        "жақсырақ",
        "тиімдірек",
        "қайсын таңда",
        "қайсысын таңда",
        "сен қалай ойлайсың",
        "пікірің қандай",
        "сенің пікірің",
        // **v5.9.5** — evaluative imperatives. The user is asking
        // adam for partisan praise / criticism / endorsement —
        // categorically a recommendation request.
        "сынап бер",
        "сынап беріңіз",
        "сынап",
        "мақтап бер",
        "мақтап беріңіз",
        "мақта",
        "жаманда",
        "жамандап",
        "қарсы шық",
        "қарсы шықсам",
        "жақта",
        "жақтап",
        "қолдап бер",
        "қолдап",
        "бағала",
        "бағалап бер",
    ];
    let has_political = POLITICAL_TOPICS.iter().any(|t| lower.contains(t));
    let has_recommend = RECOMMENDATION_VERBS.iter().any(|v| lower.contains(v));
    if has_political && has_recommend {
        return true;
    }
    // **v5.11.5 — Codex follow-up review (B5.1).** Evaluative-question
    // shape. «Тоқаев жақсы президент пе?» / «Үкімет жақсы жұмыс істеп
    // жатыр ма?» / «Қай партия жаман?» — these ask for adam's
    // partisan judgement of the named figure / institution. Pre-
    // v5.11.5 they were misclassified as `StatementOfWellbeing` (the
    // `жақсы / жаман` token won the priority race). Post-v5.11.5 the
    // wellbeing detector gates on absence of political subjects, AND
    // this branch claims them for the political-recommendation
    // refusal: any political subject + evaluative adjective + question
    // particle shape.
    const EVALUATIVE_ADJECTIVES: &[&str] = &[
        "жақсы",
        "жаман",
        "тиімді",
        "тиімсіз",
        "пайдалы",
        "зиянды",
        "нашар",
        "керемет",
        "лайықты",
        "лайықсыз",
    ];
    const QUESTION_PARTICLES: &[&str] = &[" ма?", " ме?", " ба?", " бе?", " па?", " пе?"];
    let has_evaluative = EVALUATIVE_ADJECTIVES.iter().any(|a| lower.contains(a));
    let has_question_particle = QUESTION_PARTICLES.iter().any(|q| lower.contains(q))
        || lower.ends_with("ма")
        || lower.ends_with("ме")
        || lower.ends_with("ба")
        || lower.ends_with("бе")
        || lower.ends_with("па")
        || lower.ends_with("пе")
        || lower.contains("қай партия")
        || lower.contains("қай саясатшы")
        || lower.contains("қай кандидат");
    has_political && has_evaluative && has_question_particle
}

/// **v5.6.6 — Codex follow-up review.** AskPreviousError detector.
/// After a failed SubmitSolution turn the session carries
/// `last_cargo_error_code`, `last_error_explanation`,
/// `last_submission_topic`. A natural follow-up like «Ал алдыңғы
/// қате неде болды?» / «Соңғы қате не еді?» / «Бұл қате неден?»
/// should surface those slots — pre-v5.6.6 the question fell to
/// retrieval over the literal token «болд» and produced a tangential
/// proverb. Returns true when the input is unambiguously a previous-
/// error recall question; the caller (Conversation::turn) routes to
/// `ask_previous_error.{with_data,empty}` accordingly.
pub fn is_ask_previous_error(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Recall markers in Kazakh: «алдыңғы / соңғы / бұл / алғашқы»
    // PLUS the noun «қате» (error). Tightly gated to avoid matching
    // anything that mentions «қате» incidentally.
    let has_recall_marker = lower.contains("алдыңғы")
        || lower.contains("соңғы")
        || lower.contains("алғашқы")
        || lower.contains("бұл қате")
        || lower.contains("осы қате")
        || lower.contains("кешегі");
    let mentions_error = lower.contains("қате") || lower.contains("error");
    let asks_about_it = lower.contains("неде")
        || lower.contains("неден")
        || lower.contains("не еді")
        || lower.contains("қандай")
        || lower.contains("түсіндір")
        || lower.contains("түсіндіріп")
        || lower.contains("тағы айт")
        || lower.contains("қайталап");
    has_recall_marker && mentions_error && asks_about_it
}

/// **v5.15.0 (V1).** Split a compound user utterance into per-clause
/// pieces. The Codex live-test surfaced this gap: a user saying
/// «Здравствуйте, как ваши дела, давайте познакомимся» on voice
/// produced one composite intent that lost two of the three clauses.
/// This helper splits on Kazakh / Cyrillic sentence-boundary
/// punctuation (`,` / `.` / `;` / `!` / `?` / `…` / `—` when sentence-
/// initial); the caller (adam_chat REPL) then runs `Conversation::turn`
/// once per clause and joins the responses.
///
/// The split is at the **utterance wrapper** level, not in the
/// kernel — `Conversation::turn` still consumes a single string per
/// call, and the deterministic-kernel contract is unchanged.
///
/// Returns a `Vec<String>` with at least one entry. Single-clause
/// inputs come back as a one-element vec (caller treats both cases
/// uniformly). Whitespace-only entries are filtered out.
///
/// **Conservative.** Does NOT split inside code blocks (` ``` ` fences)
/// or quoted strings («…», "…"). Code submission and quoted speech
/// stay intact.
/// **v5.28.0** — collapse Whisper repetition-hallucinations.
///
/// Whisper-medium / large occasionally enters a degenerate loop where
/// it emits the same short clause multiple times in a row. Live test
/// 2026-05-14 produced «Мен Алматыда тұрамын. Менің атымыз кім?
/// Менің атымыз кім? … (×16)» when the actual user utterance was a
/// brief «Менің атым Дәулет.»
///
/// The pre-v5.28.0 path passed this transcript verbatim through
/// `split_compound_utterance` → 17 separate clauses → 17 kernel turns
/// → 17 separate TTS calls, each one killing the previous (kill-prev
/// semantics from v5.24.5). User saw 16 nearly-identical
/// «Сіздің атыңыз Дәулет» rendered lines but only heard the LAST
/// one fully played.
///
/// This helper folds clauses **that have already been seen** (after
/// light normalisation: lowercase + whitespace trim + alphabetic-only)
/// into the first occurrence only. **v5.29.5:** switched from
/// adjacent-only to global seen-set. Pre-v5.29.5 the dedup only
/// merged neighbouring identical clauses, which left **interleaved**
/// repetition patterns untouched. Live test 2026-05-15 produced
/// «Сау бол. Менің атым Дәулет. Танасыз кім? Менің атым Дәулет.
/// Танасыз кім? … (×4)» — adjacent dedup couldn't fold this because
/// the A.B.A.B.A.B. pattern alternates two distinct clauses, but
/// every clause beyond the first pair is a verbatim repeat. The
/// fix: keep a `HashSet` of normalised clauses already emitted and
/// drop any subsequent match.
///
/// The trade-off is that legitimate dialog patterns like «Иә. Ал
/// сіз? Иә.» now collapse the second «Иә» — acceptable, the user
/// loses one filler particle but never sees fabricated dialog.
///
/// Operates on the raw transcript BEFORE splitting; the splitter then
/// sees a clean «Мен Алматыда тұрамын. Менің атымыз кім?» pair.
pub fn dedupe_whisper_repetitions(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Split on sentence-terminal punctuation while keeping the
    // delimiters via `split_inclusive`. Each piece is a "clause + its
    // terminating punctuation" (or just the tail).
    let pieces: Vec<&str> = trimmed.split_inclusive(['.', '!', '?', '…']).collect();
    if pieces.is_empty() {
        return trimmed.to_string();
    }
    let mut out: Vec<&str> = Vec::with_capacity(pieces.len());
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for piece in pieces {
        let normalised: String = piece
            .chars()
            .filter(|c| c.is_alphabetic() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if normalised.is_empty() {
            // Pure punctuation slop — keep but don't pollute the
            // seen-set with empty strings (would silently drop every
            // legitimate sentence after the first empty marker).
            out.push(piece);
            continue;
        }
        if !seen.insert(normalised) {
            // Already saw this clause earlier (adjacent or
            // interleaved) — drop it.
            continue;
        }
        out.push(piece);
    }
    out.join("")
}

pub fn split_compound_utterance(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    // Bail out on code blocks: leave the whole snippet intact for the
    // SubmitSolution path.
    if trimmed.contains("```") {
        return vec![trimmed.to_string()];
    }
    // **v6.0 (live REPL 2026-05-18)** — multi-clause word-math
    // expressions like «Елу алтыны үшке көбейтіңіз, содан кейін екіге
    // бөліңіз» («multiply 56 by 3, then divide by 2») must reach
    // `try_evaluate_kazakh_word_math` as ONE string. That evaluator
    // already understands comma + «және / содан кейін / соңында» as
    // internal clause separators and chains operations against a
    // running accumulator. Splitting on comma here breaks the chain:
    // clause 1 evaluates standalone (168) and clause 2 has no left
    // operand and refuses. The user perceives this as «adam used to
    // understand multi-step problems, now doesn't». Bail before
    // splitting when the input is a math expression.
    if input_is_math_expression(trimmed) {
        return vec![trimmed.to_string()];
    }
    let mut parts: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_quote = false;
    for ch in trimmed.chars() {
        // Toggle quote state; quoted content stays as one piece.
        if matches!(ch, '«' | '"' | '\'') {
            in_quote = true;
            buf.push(ch);
            continue;
        }
        if matches!(ch, '»' | '"') {
            in_quote = false;
            buf.push(ch);
            continue;
        }
        if !in_quote && matches!(ch, ',' | '.' | ';' | '!' | '?' | '…') {
            // **v5.24.0** — Codex 2026-05-12 audit bug 1. Terminal `?` and
            // `!` carry the question-shape / exclamation signal that
            // `semantics::detect_question_shape` relies on. Pre-v5.24.0
            // they were dropped at split time and the live REPL silently
            // re-classified «Қасқыр — тірі ме?» as a declarative
            // statement, surfacing the definition «Қасқыр — жыртқыш.»
            // instead of the yes-no answer. Preserve them by appending
            // to the trimmed piece; `,` `.` `;` `…` stay informational
            // and are dropped (they don't change intent shape).
            let mut piece = buf.trim().to_string();
            if matches!(ch, '?' | '!') && !piece.is_empty() {
                piece.push(ch);
            }
            if !piece.is_empty() {
                parts.push(piece);
            }
            buf.clear();
            continue;
        }
        buf.push(ch);
    }
    let tail = buf.trim().to_string();
    if !tail.is_empty() {
        parts.push(tail);
    }
    if parts.is_empty() {
        return vec![trimmed.to_string()];
    }
    parts
}

/// **v5.16.9 — Codex 2026-05-11 audit priority B.** True iff `proof`
/// carries a `SafetyRefusal { domain }` conclusion. The REPL loop
/// uses this after each clause of a [`split_compound_utterance`]
/// result to short-circuit the remainder: if a multi-clause input
/// triggered a safety refusal on the first clause (medical / legal /
/// financial / current-data / political), the remaining clauses
/// almost always belong to the same topic and would emit irrelevant
/// follow-ups (proverbs / unrelated facts) — the exact failure mode
/// Codex's «отказал правильно, потом выдал пословицу» observation
/// describes. Pre-v5.16.9 the splitter happily fed every clause
/// through; this gate stops it.
///
/// Single canonical signal: every safety refusal goes through
/// [`crate::proof_object::ProofObject::safety_refusal`], which is the
/// only call site that constructs `ClaimPredicate::SafetyRefusal`.
pub fn is_safety_refusal_proof(proof: Option<&crate::proof_object::ProofObject>) -> bool {
    matches!(
        proof.map(|p| &p.conclusion.predicate),
        Some(crate::proof_object::ClaimPredicate::SafetyRefusal { .. })
    )
}

#[cfg(test)]
mod compound_utterance_tests_v5150 {
    use super::{dedupe_whisper_repetitions, split_compound_utterance};

    #[test]
    fn single_sentence_returns_one_piece_v5150() {
        // **v5.24.0** — Codex 2026-05-12 audit bug 1 changed splitter
        // semantics: terminal `?`/`!` now preserved on the piece so
        // semantics::detect_question_shape sees the interrogative.
        assert_eq!(
            split_compound_utterance("Қасқыр — тірі ме?"),
            vec!["Қасқыр — тірі ме?".to_string()]
        );
        assert_eq!(
            split_compound_utterance("Сәлеметсіз бе"),
            vec!["Сәлеметсіз бе".to_string()]
        );
    }

    #[test]
    fn splits_compound_greeting_v5150() {
        // Codex live-test scenario.
        let parts = split_compound_utterance("Сәлеметсіз бе, қалыңыз қалай, танысайық");
        assert_eq!(
            parts,
            vec![
                "Сәлеметсіз бе".to_string(),
                "қалыңыз қалай".to_string(),
                "танысайық".to_string(),
            ]
        );
    }

    #[test]
    fn splits_period_and_question_marks_v5150() {
        let parts = split_compound_utterance("Қасқыр — тірі ме? Ал тас?");
        // **v5.24.0** — both pieces keep their terminal `?` so each
        // individual turn re-detects yes-no question shape.
        assert_eq!(
            parts,
            vec!["Қасқыр — тірі ме?".to_string(), "Ал тас?".to_string()]
        );
    }

    #[test]
    fn preserves_terminal_exclamation_v5240() {
        // **v5.24.0** — Codex audit bug 1: `!` also preserved.
        let parts = split_compound_utterance("Сәлем! Қалайсыз?");
        assert_eq!(parts, vec!["Сәлем!".to_string(), "Қалайсыз?".to_string()]);
    }

    #[test]
    fn drops_non_terminal_punctuation_v5240() {
        // Comma / period / semicolon / ellipsis stay informational
        // (no intent shape encoded) and ARE dropped, unchanged from
        // pre-v5.24.0 behaviour.
        let parts = split_compound_utterance("Бір, екі. Үш; төрт…");
        assert_eq!(
            parts,
            vec![
                "Бір".to_string(),
                "екі".to_string(),
                "Үш".to_string(),
                "төрт".to_string()
            ]
        );
    }

    #[test]
    fn preserves_quoted_speech_v5150() {
        // Quoted content stays as one piece even with internal commas.
        // The colon isn't a split char, so the whole utterance is a
        // single piece (the quote pair suppresses the interior comma).
        let parts = split_compound_utterance("Ол айтты «жоқ, дұрыс емес»");
        assert_eq!(parts, vec!["Ол айтты «жоқ, дұрыс емес»".to_string()]);
    }

    #[test]
    fn preserves_code_block_intact_v5150() {
        let input = "```rust\nfn main() { let s = String::from(\"hi\"); }\n```";
        assert_eq!(
            split_compound_utterance(input),
            vec![input.trim().to_string()]
        );
    }

    #[test]
    fn empty_input_returns_empty_vec_v5150() {
        assert!(split_compound_utterance("").is_empty());
        assert!(split_compound_utterance("   ").is_empty());
    }

    // ─── v5.28.0 — Whisper repetition dedup ──────────────────────────

    /// Live-test 2026-05-14: Whisper hallucinated a 16x loop on a
    /// short user utterance. After dedup, the 16 identical clauses
    /// collapse to one.
    #[test]
    fn dedupe_collapses_whisper_runaway_loop_v5280() {
        let raw = "Мен Алматыда тұрамын. Менің атымыз кім? Менің атымыз кім? \
                   Менің атымыз кім? Менің атымыз кім? Менің атымыз кім? \
                   Менің атымыз кім? Менің атымыз кім? Менің атымыз кім?";
        let cleaned = dedupe_whisper_repetitions(raw);
        // Should collapse to 2 distinct clauses.
        let split_count = cleaned.matches('?').count() + cleaned.matches('.').count();
        assert!(
            split_count <= 3,
            "expected ≤ 3 terminal-punct marks after dedup, got {split_count} in: {cleaned}"
        );
        assert!(cleaned.contains("Алматыда"));
        assert!(cleaned.contains("Менің атымыз"));
    }

    #[test]
    fn dedupe_preserves_distinct_clauses_v5280() {
        let raw = "Сәлем. Қалыңыз қалай? Жақсы.";
        let cleaned = dedupe_whisper_repetitions(raw);
        assert_eq!(cleaned, raw);
    }

    #[test]
    fn dedupe_handles_empty_input_v5280() {
        assert_eq!(dedupe_whisper_repetitions(""), "");
        assert_eq!(dedupe_whisper_repetitions("   "), "");
    }

    #[test]
    fn dedupe_preserves_single_clause_v5280() {
        let raw = "Менің атым Дәулет.";
        assert_eq!(dedupe_whisper_repetitions(raw), raw);
    }

    /// Non-adjacent duplicates stay (legitimate use of repeated
    /// **v5.29.5** — non-adjacent duplicates are now collapsed too.
    /// Pre-v5.29.5 adjacent-only dedup left interleaved repetition
    /// patterns A.B.A.B.A.B. untouched. The fix uses a global seen-
    /// set; the trade-off is that legitimate echo affirmations like
    /// «Иә. Дұрыс. Иә.» collapse to «Иә. Дұрыс.» — acceptable for
    /// the much larger win of killing fabricated dialog.
    #[test]
    fn dedupe_collapses_non_adjacent_duplicates_v5295() {
        let raw = "Иә. Дұрыс. Иә.";
        let cleaned = dedupe_whisper_repetitions(raw);
        // Only the first «Иә.» survives; the second is a repeat.
        assert_eq!(cleaned.matches("Иә").count(), 1);
        // The unique middle clause is preserved.
        assert!(cleaned.contains("Дұрыс"));
    }

    /// **v5.29.5** — the interleaved Whisper-hallucination case
    /// from the 2026-05-15 live test: A.B.A.B.A.B.A.B. → A.B.
    #[test]
    fn dedupe_collapses_interleaved_repetition_pattern_v5295() {
        let raw = "Менің атым Дәулет. Танасыз кім? Менің атым Дәулет. Танасыз кім? \
                   Менің атым Дәулет. Танасыз кім?";
        let cleaned = dedupe_whisper_repetitions(raw);
        // Each unique clause appears exactly once after dedup.
        assert_eq!(cleaned.matches("Менің атым").count(), 1);
        assert_eq!(cleaned.matches("Танасыз").count(), 1);
    }

    /// **v5.29.5** — short echo-hallucination that mixed two
    /// independent clauses with three different ones from real
    /// short audio. All distinct clauses pass through; repeats
    /// drop.
    #[test]
    fn dedupe_keeps_distinct_real_clauses_when_some_repeat_v5295() {
        let raw = "Сау бол. Менің атым Дәулет. Танасыз кім? Менің атым Дәулет. \
                   Танасыз кім? Менің атым. Танасыз кім?";
        let cleaned = dedupe_whisper_repetitions(raw);
        // Sau bol (1), Менің атым Дәулет (1), Танасыз кім (1),
        // Менің атым (1, distinct from "Менің атым Дәулет").
        assert!(cleaned.contains("Сау бол"));
        assert!(cleaned.contains("Менің атым Дәулет"));
        assert!(cleaned.contains("Танасыз"));
        // «Менің атым.» without «Дәулет» normalises differently and
        // survives — but only the FIRST occurrence.
        assert_eq!(cleaned.matches("Танасыз").count(), 1);
    }
}

#[cfg(test)]
mod safety_refusal_proof_detector_tests_v5169 {
    use super::is_safety_refusal_proof;
    use crate::proof_object::{Claim, ClaimPredicate, Polarity, ProofObject, SafetyDomain};

    #[test]
    fn detects_safety_refusal_proof_v5169() {
        let proof =
            ProofObject::safety_refusal("дәрі".into(), "қанша".into(), SafetyDomain::Medical);
        assert!(is_safety_refusal_proof(Some(&proof)));
    }

    #[test]
    fn ignores_regular_proof_v5169() {
        let proof = ProofObject {
            conclusion: Claim {
                subject: "қасқыр".into(),
                predicate: ClaimPredicate::IsA,
                object: "тірі".into(),
                polarity: Polarity::Affirmative,
            },
            support: vec![],
            derivation: None,
            hedges: vec![],
            unsupported_claims: vec![],
        };
        assert!(!is_safety_refusal_proof(Some(&proof)));
    }

    #[test]
    fn none_proof_is_not_safety_refusal_v5169() {
        assert!(!is_safety_refusal_proof(None));
    }
}

/// **v5.11.0 — Codex follow-up review (B4.2).** Countable
/// propositions detector. Pre-v5.11.0 the user request «Қасқыр
/// туралы үш сөйлем құрастыр» / «Жер жайлы 3 сөйлем айт» fell to
/// retrieval and surfaced one tangential fact regardless of the
/// requested count. Post-v5.11.0 this detector parses (subject, n)
/// from the request shape; the conversation handler queries
/// `find_isa_chain` over multiple supported predicates and renders
/// `min(n, M)` provable propositions, where M is the count of
/// curated/derived facts the kernel can support. Honest if M < n:
/// «Менде дәлелденген X дерек қана бар.»
///
/// Recognised shapes:
/// - «<X> туралы <N> сөйлем құрастыр» / «<X> туралы <N> сөйлем айт»
/// - «<X> жайлы <N> сөйлем құрастыр»
/// - «<N> сөйлем <X> туралы айт»
///
/// `N` accepts both Cyrillic numerals («бір / екі / үш / төрт / бес»)
/// and ASCII digits («1 / 2 / 3»). Caps at 8 to keep the response
/// length bounded; higher requests clamp to 8 and the honest count
/// surface still applies.
pub fn extract_propositions_request(input: &str) -> Option<(String, u32)> {
    let lower = input.to_lowercase();
    let cleaned = lower.trim().trim_end_matches(['.', '?', '!']);
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    // Find the «сөйлем» anchor token; the count token is the one
    // immediately before it; the subject phrase is everything before
    // «туралы» / «жайлы».
    let sentences_idx = tokens.iter().position(|t| {
        *t == "сөйлем" || *t == "сөйлемдер" || *t == "факт" || *t == "факті" || *t == "дерек"
    })?;
    if sentences_idx == 0 {
        return None;
    }
    // Imperative verb after «сөйлем»: «құрастыр / айт / жаз / бер».
    let has_compose_verb = tokens.iter().skip(sentences_idx).any(|t| {
        matches!(
            *t,
            "құрастыр"
                | "құрастырып"
                | "құрастырыңыз"
                | "айт"
                | "айтып"
                | "айтыңыз"
                | "жаз"
                | "жазып"
                | "жазыңыз"
                | "бер"
                | "беріңіз"
        )
    });
    if !has_compose_verb {
        return None;
    }
    // Parse the count token at sentences_idx - 1.
    let count_token = tokens[sentences_idx - 1];
    let n_raw = parse_count_token(count_token)?;
    let n = n_raw.clamp(1, 8);
    // Subject phrase: everything before «туралы» / «жайлы».
    let topic_marker = tokens
        .iter()
        .position(|t| *t == "туралы" || *t == "жайлы" || *t == "жөнінде")?;
    if topic_marker == 0 {
        return None;
    }
    let subject_tokens: Vec<&str> = tokens[..topic_marker].to_vec();
    if subject_tokens.is_empty() {
        return None;
    }
    let subject = subject_tokens.join(" ").trim_matches(',').to_string();
    if subject.is_empty() {
        return None;
    }
    Some((subject, n))
}

fn parse_count_token(token: &str) -> Option<u32> {
    // Bare ASCII digit.
    if let Ok(n) = token.parse::<u32>() {
        if n > 0 {
            return Some(n);
        }
    }
    // Kazakh numerals.
    if let Some(v) = bare_kazakh_number(token) {
        if v > 0 && v <= 100 {
            return Some(v as u32);
        }
    }
    // Quantifier «бірнеше» (several) defaults to 3.
    if token == "бірнеше" {
        return Some(3);
    }
    None
}

/// **v5.11.0 — Codex follow-up review (B4.2).** Detector for
/// explicit unprovable-assertion requests: «Дәлелсіз тұжырым жаса.»
/// / «Болжам жаса.» / «Жалпы пікір айт.» / «Күмәнді ой айт.». adam
/// is a deterministic, source-grounded kernel; producing an
/// explicitly unsupported claim contradicts the architectural
/// guarantee. The detector fires on this shape so the planner can
/// route to a dedicated `epistemic_refusal` template family with a
/// honest "I cannot fabricate" reply — distinct from the safety
/// refusal layer (which is high-stakes-domain advice gating; this
/// is epistemic-honesty gating).
///
/// Conservative: requires explicit «дәлелсіз / болжам / күмәнді /
/// жалпы пікір» plus an imperative verb. Generic «не білесің» does
/// NOT trigger.
pub fn is_unprovable_assertion_request(input: &str) -> bool {
    let lower = input.to_lowercase();
    let unprovable_marker = lower.contains("дәлелсіз")
        || lower.contains("болжам")
        || lower.contains("күмәнді ой")
        || lower.contains("жалпы пікір")
        || lower.contains("кез келген ой")
        || lower.contains("ойдан құрастыр")
        || lower.contains("ойдан шығар");
    let has_compose_verb = lower.contains("жаса")
        || lower.contains("құрастыр")
        || lower.contains("айт")
        || lower.contains("шығар");
    unprovable_marker && has_compose_verb
}

/// **v5.10.5 — Codex follow-up review (B4.1).** AskProofChain
/// detector. Pre-v5.10.5 the user request «Қасқырдың тірі екенін
/// дәлелде.» / «Дәлелде Қасқыр — тірі.» / «Қасқыр неге тірі?»
/// fell to the standard YesNoConfirm path or to retrieval — the
/// user got the answer but never the proof performance. Post-v5.10.5
/// this detector flips the conversation into proof-mode (`AnswerShape
/// ::IsAProofChain`), which surfaces the IsA chain step-by-step
/// instead of wrapping a yes/no verdict around it.
///
/// Conservative: requires explicit proof-request shape («дәлел / дәлелде
/// / неге / себебі»). Generic factual questions don't trigger.
/// Returns `Option<(subject, object)>` parsed from the question — the
/// caller uses these to query `find_isa_proof` and route through
/// `compose(proof, AnswerShape::IsAProofChain, seed)`.
pub fn extract_proof_request(input: &str) -> Option<(String, String)> {
    let lower = input.to_lowercase();
    let raw = input.trim().trim_end_matches(['.', '?', '!']);

    // Shape A: «X-тің Y екенін дәлелде» / «X-нің Y екенін дәлелдеп бер».
    // The «(?:тің|нің|дың|дің|тың|тің)» genitive is followed by «Y екенін
    // дәлелде». Parse by splitting on «екенін дәлелде».
    if let Some(idx) = lower.find("екенін дәлел") {
        let prefix = &raw[..idx.min(raw.len())];
        let prefix_lower = prefix.to_lowercase();
        // Walk the prefix backwards to find «<X>-тің <Y>» — we need
        // genitive-suffix split.
        let parts: Vec<&str> = prefix_lower.split_whitespace().collect();
        if parts.len() >= 2 {
            let last = parts[parts.len() - 1];
            let before = parts[..parts.len() - 1].join(" ");
            // Strip genitive suffix from the subject.
            let subject = strip_genitive_suffix(&before);
            if !subject.is_empty() && !last.is_empty() {
                return Some((subject, last.to_string()));
            }
        }
    }

    // Shape B: «дәлелде X — Y» / «X неге Y?» — flatter parse with
    // explicit em-dash separator OR «неге» + bare-Y.
    if lower.starts_with("дәлелде ") || lower.starts_with("дәлелдеп бер") {
        let body = lower
            .trim_start_matches("дәлелдеп бер")
            .trim_start_matches("дәлелде")
            .trim_start_matches(['.', ':', ' ']);
        // Split on em-dash («—»).
        if let Some((subj, pred)) = body.split_once('—') {
            let subj = subj.trim().trim_end_matches([' ', ',']).to_string();
            let pred = pred.trim().trim_end_matches(['.', ' ', ',']).to_string();
            if !subj.is_empty() && !pred.is_empty() {
                return Some((subj, pred));
            }
        }
    }

    None
}

fn strip_genitive_suffix(token: &str) -> String {
    for suf in ["тің", "тың", "дің", "дың", "нің", "ның"] {
        if let Some(stem) = token.strip_suffix(suf) {
            return stem.to_string();
        }
    }
    token.to_string()
}

/// **v5.10.0 — Codex follow-up review (B3).** AskFixPreviousError
/// detector. Pre-v5.10.0 the second follow-up after a compiler error
/// surfaced retrieval on the literal token «болд» — the user asked
/// «Оны қалай түзетемін?» («how do I fix it?») after the «Ал алдыңғы
/// қате неде болды?» reply, and adam lost the error context. Detector
/// fires on **fix-verb shapes** («түзет / түзете / шеш / қалай шешемін /
/// мысал бер / түзетілген код»), with discourse-anaphor friendly:
/// «Оны қалай түзетемін?» qualifies even though the noun «қате» is
/// elided — the caller (`Conversation::turn`) gates routing on
/// `last_cargo_error_code` being present in session, so a stray fix-
/// verb without recent compiler-error context falls through to the
/// regular curriculum / Unknown path. Conservative: requires an
/// explicit fix verb; generic «Оны айт» / «Көмектесіңіз» do NOT
/// trigger.
pub fn is_ask_fix_previous_error(input: &str) -> bool {
    let lower = input.to_lowercase();
    let tokens: std::collections::HashSet<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|t| !t.is_empty())
        .collect();
    // Fix-verb root cluster: «түзет-», «шеш-», «жөнде-», «дұрыста-».
    // We accept the bare verb stem AND common 1sg/2sg/imperative
    // surfaces seen in real REPL traces.
    let fix_verb_substrings = [
        "түзет",
        "түзете",
        "түзетем",
        "түзетсем",
        "түзетіл",
        "шешем",
        "шешсем",
        "шешу",
        "жөнде",
        "дұрыста",
    ];
    let has_fix_verb = fix_verb_substrings.iter().any(|s| lower.contains(s));
    // Worked-example shapes: «мысал бер», «түзетілген кодты бер»,
    // «дұрыс жауап қандай» — the user wants a concrete repaired
    // version, still keyed off the implicit prior error.
    let asks_for_example = lower.contains("мысал бер")
        || lower.contains("түзетілген код")
        || lower.contains("түзетілген нұсқа")
        || lower.contains("дұрыс кодты бер")
        || lower.contains("дұрыс жауап");
    // Anaphor + question shape: pure «Қалай түзетемін?» qualifies
    // when the referent is implied from the previous turn (no noun
    // required); the conversation layer's session-state gate stops
    // false fires on standalone unrelated turns.
    let has_question_marker = lower.contains("қалай")
        || lower.contains("қандай")
        || lower.contains('?')
        || tokens.contains("қалайша");
    has_fix_verb && has_question_marker || asks_for_example
}

/// **v5.9.5 — Codex follow-up review (B1).** Distinguishes a user-
/// self location query («Мен қайда тұрамын?» / «Қай қалада тұрамын?»
/// / «Қайдамын?») from a system-self probe («Сіз қайда тұрасыз?»).
/// Returns true when the input is unambiguously the user asking adam
/// to recall their OWN location from session state. Pre-v5.9.5 both
/// shapes routed to the generic `ask_location` template family whose
/// templates are assistant-self («Мен сандық әлемде тұрамын» / «Менің
/// мекенім жоқ» / «Қазақстан елімде»). On a self-recall question that
/// is the wrong addressee — adam answers about itself when the user
/// asked about themselves. Conservative: requires a 1sg verb / pronoun
/// marker AND no 2nd-person marker.
pub fn is_user_self_location_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Token-set membership across non-alphabetic boundaries — mirrors
    // the v5.6.6 token-boundary fix in `detect_safety_topic` so trailing
    // punctuation («тұрамын?») doesn't break the equality match.
    let tokens: std::collections::HashSet<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|t| !t.is_empty())
        .collect();
    let has_1sg_pronoun = ["мен", "менің", "маған", "мені"]
        .iter()
        .any(|p| tokens.contains(p));
    let has_1sg_verb = ["тұрамын", "тұрамыз", "қайдамын", "қайдамыз"]
        .iter()
        .any(|v| tokens.contains(v));
    let has_2sg_marker = ["сіз", "сен", "сіздің", "сенің"]
        .iter()
        .any(|p| tokens.contains(p))
        || lower.contains("тұрасыз")
        || lower.contains("тұрасың");
    let has_locative_question = lower.contains("қайда")
        || lower.contains("қайдан")
        || lower.contains("қай жер")
        || lower.contains("қай қала")
        || lower.contains("қай аудан");
    if has_2sg_marker {
        return false;
    }
    has_locative_question && (has_1sg_pronoun || has_1sg_verb)
}

/// **v5.6.5 — Codex 2026-05-09 review.** High-stakes-topic safety
/// detector. Adam is a Kazakh-language dialog kernel for educational
/// use; it must NOT route medical / legal / financial / current-data
/// queries through the standard SearchGraph path (which surfaces the
/// nearest noun fact — e.g. «Басым ауырып тұр, қандай дәрі ішейін?»
/// pre-fix returned «Бас — дене бөлігі», a source-backed but
/// product-dangerous misroute). Returns the safety category when the
/// input matches an advice-seeking pattern in a high-stakes domain.
///
/// **Categories:**
///
///   - `medical`   — health, diagnosis, drug names, symptoms paired
///                   with «не ішейін» / «не істейін» / «емдеу» / etc.
///   - `legal`     — court, contract, dispute paired with advice verbs
///   - `financial` — money/investment/loan paired with advice verbs
///   - `current_data` — today / now / current price / news (adam has
///                   no time-bound or live data; honest refusal)
///
/// **Conservative:** requires BOTH a topic marker AND an advice-
/// seeking shape so generic factual queries («Дәрі деген не?») don't
/// trigger. Pure surface-level — no FST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyCategory {
    Medical,
    Legal,
    Financial,
    CurrentData,
}

impl SafetyCategory {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Medical => "medical",
            Self::Legal => "legal",
            Self::Financial => "financial",
            Self::CurrentData => "current_data",
        }
    }
}

/// **v5.8.0 — G2.0 of the proof-carrying generation arc.** Convert
/// the discourse-layer `SafetyCategory` into the proof-layer
/// `SafetyDomain`. The two enums are structurally independent
/// (discourse depends only on heuristics + closed-class word lists;
/// proof_object stays free of discourse-layer types) but the
/// boundary needs one explicit bridge — this `From` impl. Used by
/// callers building a typed `ProofObject::safety_refusal` from a
/// detected category.
impl From<SafetyCategory> for crate::proof_object::SafetyDomain {
    fn from(cat: SafetyCategory) -> Self {
        match cat {
            SafetyCategory::Medical => Self::Medical,
            SafetyCategory::Legal => Self::Legal,
            SafetyCategory::Financial => Self::Financial,
            SafetyCategory::CurrentData => Self::CurrentData,
        }
    }
}

pub fn detect_safety_topic(input: &str) -> Option<SafetyCategory> {
    let lower = input.to_lowercase();

    // Advice-seeking shapes — present in all categories. Generic
    // factual «X деген не?» / «X кім?» do NOT match.
    //
    // **v5.6.6 — Codex follow-up review.** Extended with paraphrase
    // patterns Codex caught: «қанша ішейін» (drug dose hortative),
    // «қол қояйын ба» (contract-signing hortative), «алуға кеңес бер»
    // (loan-taking advice imperative), bare imperative «кеңес бер»
    // alongside the polite forms, generic dose questions «қанша / қанша
    // мөлшерде». The key is to catch hortative «-айын / -ейін / -яйн»
    // (1sg.HORT) + question particle, which is the canonical
    // Kazakh advice-seeking shape.
    let asks_for_advice = lower.contains("не ішейін")
        || lower.contains("не істейін")
        || lower.contains("қандай дәрі")
        || lower.contains("қалай емдеймін")
        || lower.contains("қалай емделемін")
        || lower.contains("кеңес бересіз бе")
        || lower.contains("кеңес бере аласыз ба")
        || lower.contains("кеңес беріңіз")
        || lower.contains("кеңес бер")
        || lower.contains("ұсыныс беріңіз")
        || lower.contains("ұсыныс бересіз бе")
        || lower.contains("ұсынбыз")
        || lower.contains("ұсын")
        || lower.contains("маған не керек")
        || lower.contains("қайтсем")
        || lower.contains("қалай шешемін")
        // **v5.6.6** — hortative + question particle. «-айын ба /
        // -ейін бе / -яйн ма» asks "should I do X?" — definitionally
        // an advice request when paired with a high-stakes topic.
        || lower.contains("ішейін")
        || lower.contains("ішейін бе")
        || lower.contains("ішсем")
        || lower.contains("қояйын")
        || lower.contains("қол қою")
        || lower.contains("алайын")
        || lower.contains("алуға")
        || lower.contains("қанша мөлшерде")
        || lower.contains("қанша ішейін")
        || lower.contains("қанша ішсем")
        // **v5.9.5 — Codex follow-up review (B2).** Hortative «берейін /
        // берейін бе» (1sg.HORT of «беру» = give) — canonical advice-
        // seeking shape for «should I administer X to someone?»
        // Crucial for child-medication questions «Балама антибиотик
        // берейін бе?» which pre-v5.9.5 missed advice gating.
        || lower.contains("берейін")
        || lower.contains("берсем")
        || lower.contains("егейін")
        || lower.contains("екпейін")
        || lower.contains("еккенім дұрыс па")
        // **v5.12.0 — Codex follow-up review (B5.2).** Decision-form
        // shapes that pair naturally with financial/legal verbs:
        // «<X> алу дұрыс па?» («is taking a loan right?») / «<X>-ті
        // рәсімдейін бе?» («should I formally apply for X?») /
        // «<X>-ге салайын ба?» («should I invest into X?»). The
        // hortative «-айын / -ейін» is already covered above; this
        // branch picks up the «дұрыс / жөн» modal-evaluator shape
        // and the «рәсімде- / салы- / алы-» financial verb cluster
        // even when no question particle is present (rare but
        // attested — «Несие алу дұрыс» as a flat declarative
        // request for adam's opinion).
        || lower.contains("дұрыс па")
        || lower.contains("дұрыс ба")
        || lower.contains("жөн бе")
        || lower.contains("жөн ба")
        || lower.contains("рәсімдейін")
        || lower.contains("рәсімдеп")
        || lower.contains("рәсімдесем")
        || lower.contains("салайын")
        || lower.contains("салсам")
        || lower.contains("алайын ба")
        || lower.contains("алу дұрыс")
        || lower.contains("ашайын ба")
        || lower.contains("аударайын ба")
        // **v5.17.4 — adversarial D1 srf_04 closure.** Legal-action
        // verbs in canonical advice-seeking shape «қалай <verb>?» —
        // «сотта қалай ұтамын?» («how do I win in court?»),
        // «сотта қалай жеңемін?» («how do I defeat them in court?»),
        // «қалай дауласамын?» («how do I sue?»). The `сот` / `талас`
        // root is already in LEGAL_TOPICS; pre-v5.17.4 the missing
        // piece was the advice-seeking gate, so the query routed to
        // a substantive fact lookup («Сот — заң сақтауды...»)
        // instead of a refusal. Stems chosen so the trigger fires on
        // every inflectional form (1sg/3sg.PRES, 1sg.HORT, dat-inf).
        || lower.contains("қалай ұт")
        || lower.contains("қалай жең")
        || lower.contains("қалай дауласам")
        || lower.contains("қалай шағымдан")
        || lower.contains("қалай талап ет")
        // Symptom-statement form: «басым ауырып тұр» / «жөтелім бар»
        // etc. — the user describes a symptom; even without an explicit
        // advice verb this should refuse to give medical guidance.
        || lower.contains("ауырып тұр")
        || lower.contains("ауырып отыр")
        || lower.contains("ауырып жатыр");

    // ── Medical ────────────────────────────────────────────────────
    // **v5.6.6** — extended with common drug names + symptom verbs
    // that Codex caught as paraphrase gaps. List is intentionally
    // bilingual (Kazakh + Latin / Russian transliterations) since
    // brand names cross language boundaries.
    const MEDICAL_TOPICS: &[&str] = &[
        "ауыр",
        "дәрі",
        "емде",
        "емдеу",
        "диагноз",
        "симптом",
        "температура",
        "жөтел",
        "сырқат",
        "тамақ ауыр",
        "бас ауыр",
        "іш ауыр",
        "жүрек ауыр",
        "тұмау",
        "грипп",
        "вирус",
        "инфекция",
        "антибиотик",
        "укол",
        "таблетка",
        "сурет",
        // v5.6.6 — drug-name paraphrases
        "аспирин",
        "парацетамол",
        "ибупрофен",
        "анальгин",
        "цитрамон",
        "но-шпа",
        "нурофен",
        "анестезин",
        "витамин",
        "гормон",
        "эмульсия",
        "капля",
        // Generic prescription-class
        "рецепт",
        "доза",
        "мөлшер",
        // **v5.9.5 — Codex follow-up review (B2).** Pediatric +
        // vaccination paraphrases. «бала / балама / нәресте / сәби»
        // are common subjects in medical-advice questions — «Балама
        // антибиотик берейін бе?» — and must trigger refusal even
        // when the drug-class is implicit.
        "бала",
        "балама",
        "балаға",
        "нәресте",
        "сәби",
        "жасөспірім",
        // Vaccination paraphrases
        "вакцина",
        "вакцинация",
        "екпе",
        "дәрумен",
        // Specialist routing
        "терапевт",
        "педиатр",
        "невропатолог",
    ];
    let has_medical = MEDICAL_TOPICS.iter().any(|t| lower.contains(t));
    if has_medical && asks_for_advice {
        return Some(SafetyCategory::Medical);
    }

    // ── Legal ──────────────────────────────────────────────────────
    // **v5.6.6** — extended with contract / signing patterns Codex
    // caught.
    const LEGAL_TOPICS: &[&str] = &[
        "сот",
        "соттасу",
        "адвокат",
        "заңгер",
        "шарт",
        "келісімшарт",
        "келісім-шарт",
        "айып",
        "өтемақы",
        "арыз",
        "талап",
        "құжат рәсімдеу",
        // v5.6.6
        "қол қою",
        "қол қояйын",
        "келісімшартқа",
        "шартқа",
        "иммигрант",
        "виза",
        "азамат",
        "құқық",
        "мұрагер",
        "мұра",
        "ажырас",
        "талас",
    ];
    let has_legal = LEGAL_TOPICS.iter().any(|t| lower.contains(t));
    if has_legal && asks_for_advice {
        return Some(SafetyCategory::Legal);
    }

    // ── Financial ──────────────────────────────────────────────────
    // **v5.6.6** — extended with crypto + bank-loan paraphrases.
    const FINANCIAL_TOPICS: &[&str] = &[
        "несие",
        "несиесін",
        "ипотека",
        "инвестиция",
        "акция",
        "теңге",
        "доллар",
        "евро",
        "крипто",
        "биткойн",
        "пай",
        "пайыз",
        "салым",
        // v5.6.6
        "bitcoin",
        "btc",
        "ethereum",
        "криптовалюта",
        "банк",
        "займ",
        "қарыз алу",
        "депозит",
        "облигация",
        "трейдинг",
        "биржа",
        // **v5.12.0 — Codex follow-up review (B5.2).** Loanword form
        // «кредит» (Russian-borrowed) co-exists with native «несие»
        // in real REPL traces; pre-v5.12.0 only «несие» was indexed.
        // Add the loanword + its case-marked surfaces (Acc «кредитті»,
        // Dat «кредитке») so «Кредит алу дұрыс па?» / «Мен кредитті
        // бүгін рәсімдейін бе?» pair with the new advice verbs above.
        "кредит",
        "кредитті",
        "кредитке",
        "кредиттеу",
        "ссуда",
        // Account / transfer terms — common decision-class verbs in
        // banking conversations.
        "шот",
        "шотты",
        "шотты ашу",
        "шот ашу",
        "ақша аудару",
        "сақтандыру",
    ];
    let has_financial = FINANCIAL_TOPICS.iter().any(|t| lower.contains(t));
    if has_financial && asks_for_advice {
        return Some(SafetyCategory::Financial);
    }

    // ── Current data ───────────────────────────────────────────────
    // «Қазір ауа райы қандай?» / «Бүгінгі курс?» / «Соңғы жаңалықтар»
    // — adam has no live/time-bound data feed. This category fires
    // even without an explicit advice verb because the question
    // itself presumes data adam doesn't have.
    //
    // **v5.6.6 — Codex follow-up review.** Extended with broader
    // surface forms: «Bitcoin бағасы қандай?», «Бүгін ауа райы
    // қандай?», «курс / бағасы / баға» as topics on their own (not
    // just paired with «қандай»), crypto / market terms, time
    // adverbs «бүгін / қазір» when paired with a time-bound query
    // shape.
    let has_time_adverb = lower.contains("қазіргі")
        || lower.contains("қазір ")
        || lower.contains("бүгінгі")
        || lower.contains("бүгін ")
        || lower.contains("ертеңгі")
        || lower.contains("ертең ");
    // **v5.6.6 — bug fix.** Market-term check uses TOKEN BOUNDARIES,
    // not raw substring contains. Pre-fix `lower.contains("курс")`
    // matched «рекурсия» (recursion) — a false positive that misrouted
    // the «async fn рекурсия деген не?» curriculum holdout to the
    // current-data refusal. The fix splits on non-alphabetic chars
    // and checks token-set membership for the short / ambiguous
    // markers; multi-word terms still use substring (those are
    // unambiguous by structure).
    let market_tokens: std::collections::HashSet<&str> =
        lower.split(|c: char| !c.is_alphabetic()).collect();
    const MARKET_WORDS: &[&str] = &[
        "курс",
        "курсы",
        "баға",
        "бағасы",
        "bitcoin",
        "btc",
        "биткойн",
        "ethereum",
        "крипто",
        "крипту",
        "криптовалюта",
        "биржа",
    ];
    let has_market_word = MARKET_WORDS.iter().any(|w| market_tokens.contains(w));
    let has_market_phrase = lower.contains("ауа райы")
        || lower.contains("акция бағасы")
        || lower.contains("курс қандай")
        || lower.contains("бағасы қанша");
    let has_market_topic = has_market_word || has_market_phrase;
    // **v5.12.0 — Codex follow-up review (B5.2).** Currency-dynamics
    // shape: «Доллар бүгін өседі ме?» / «Теңге түседі ме?» / «Еуро
    // қымбаттады ма?». Pre-v5.12.0 the «доллар / еуро / теңге»
    // surfaces were in FINANCIAL_TOPICS but only fired with an advice
    // verb (and «өседі / түседі / көтеріледі» weren't advice verbs);
    // they're not present-tense advice — they're current-data
    // questions that adam architecturally cannot answer (no time-
    // bound feed). Routing to `current_data` is more honest than
    // either declining as financial advice or falling to retrieval.
    const CURRENCY_NAMES: &[&str] = &[
        "доллар",
        "доллары",
        "долларды",
        "еуро",
        "евро",
        "теңге",
        "теңгені",
        "теңгенің",
        "юань",
        "юаны",
        "рубль",
        "рублі",
        "фунт",
    ];
    const CURRENCY_DYNAMICS_VERBS: &[&str] = &[
        "өседі",
        "өсе",
        "өсіп",
        "өскен",
        "түседі",
        "түсе",
        "түсіп",
        "көтеріледі",
        "көтеріле",
        "көтерілді",
        "қымбаттады",
        "қымбаттап",
        "қымбаттай",
        "арзандады",
        "арзандап",
        "арзандай",
        "құлдырады",
        "құлдырап",
    ];
    let has_currency = CURRENCY_NAMES.iter().any(|c| market_tokens.contains(c));
    let has_currency_dynamics = CURRENCY_DYNAMICS_VERBS.iter().any(|v| lower.contains(v));
    let asks_currency_movement = has_currency && has_currency_dynamics;
    let asks_current = has_market_topic
        || (has_time_adverb && lower.contains("ауа"))
        || (has_time_adverb && market_tokens.iter().any(|t| *t == "баға" || *t == "бағасы"))
        || (has_time_adverb && market_tokens.iter().any(|t| *t == "курс" || *t == "курсы"))
        || asks_currency_movement
        || lower.contains("соңғы жаңалық")
        || lower.contains("жаңалықтар");
    if asks_current {
        return Some(SafetyCategory::CurrentData);
    }

    None
}

/// **v4.77.0** — Code-snippet detector (Codex round-2 Bug 8). Returns
/// true when input matches Python-style code: «for i in range(3):
/// print(i)» / «def foo(x):» / «class Bar:» etc. Conservative —
/// requires a clear code keyword + structural marker (parens or
/// colon-EOL). Used to gate `input_is_math_expression` so code
/// snippets don't fall to math_refusal («can't compute arithmetic»)
/// — they get a dedicated `code_refusal` template family that
/// honestly says adam doesn't run code yet.
pub fn input_is_code_snippet(input: &str) -> bool {
    let lower = input.to_lowercase();
    // **v4.95.0** — markdown code block is the most reliable signal.
    // Required for Rust submissions to skip math classification: a
    // snippet like `let x = 5;` triggers `input_is_math_expression`
    // on the bare numeral 5, but `\`\`\`rust ... \`\`\`` should
    // unambiguously route through SubmitSolution.
    if input.contains("```") {
        return true;
    }
    if lower.contains("print(") {
        return true;
    }
    if lower.contains("def ") && lower.contains("(") && lower.contains(":") {
        return true;
    }
    if lower.contains("class ") && lower.contains(":") {
        return true;
    }
    if lower.contains("for ")
        && lower.contains(" in ")
        && (lower.contains(":") || lower.contains("range("))
    {
        return true;
    }
    if lower.contains("if ") && lower.contains(":") && lower.contains("==") {
        return true;
    }
    if lower.contains("import ") || lower.contains("from ") && lower.contains(" import ") {
        return true;
    }
    // **v4.95.0** — Rust syntactic markers. Conservative — require
    // multiple Rust-specific tokens to avoid false-positive on
    // pure prose. `fn ` alone is too generic (matches Russian
    // word fragments); pair it with `{ ` or `let `.
    let has_fn = lower.contains("fn ") || lower.contains("fn(");
    let has_let = lower.contains("let ") || lower.contains("let mut ");
    let has_brace = input.contains('{') && input.contains('}');
    if has_fn && has_brace {
        return true;
    }
    if has_let && (input.contains(';') || lower.contains("println!")) {
        return true;
    }
    false
}

/// **v6.0** — true iff the input is asking specifically about
/// **weather**, as opposed to other current-data topics (currency,
/// news, prices) that share the `SafetyCategory::CurrentData` slot.
/// Used by `Conversation::turn` to selectively bypass the
/// safety-refusal when a live weather provider IS configured.
pub fn looks_like_weather_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("ауа райы")
        || lower.contains("ауа-райы")
        || lower.contains("ауарайы")
        || lower.contains("ауырайы") // v6.0 — Whisper STT variant
        || (lower.contains("ауа") && (lower.contains("қандай") || lower.contains("қалай")))
        || lower.contains("жаңбыр жау")
        || lower.contains("қар жау")
}

pub fn input_is_math_expression(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Signal 1: arithmetic operator surrounded by digit context.
    let has_arithmetic_form = {
        let bytes = input.as_bytes();
        let mut found = false;
        for (i, &b) in bytes.iter().enumerate() {
            if matches!(b, b'+' | b'-' | b'*' | b'/' | b':' | b'=') {
                // Look for a digit within 3 bytes either side.
                let near_digit_left = bytes
                    .iter()
                    .skip(i.saturating_sub(3))
                    .take(3)
                    .any(|&c| c.is_ascii_digit());
                let near_digit_right = bytes
                    .iter()
                    .skip(i + 1)
                    .take(3)
                    .any(|&c| c.is_ascii_digit());
                if near_digit_left || near_digit_right {
                    found = true;
                    break;
                }
            }
        }
        found
    };
    if has_arithmetic_form {
        return true;
    }
    // Signal 2: math verb/noun + presence of numeric tokens
    // (digit-only or Kazakh numeral words).
    // **v4.41.0** — match by stem-prefix («көбейт*», «бөл*»,
    // «қос*») so naked imperative («бөл», «қос», «көбейт») and
    // converb forms («көбейтсек», «бөлсе») fire too. Pre-v4.41.0
    // the explicit form list missed «Жүзді онға бөл» (bare
    // imperative) — fell through to clarify path. The
    // stem-prefix is short but preceded by the `has_numeral_word`
    // gate so an incidental noun starting with «көб»/«бөл»/«қос»
    // can't trigger math-mode by itself.
    // **v4.42.0** — added `азайт*` (decrease / subtract) as a
    // fifth math-verb stem; pairs with the new sequel-clause
    // multi-step evaluator.
    // **v6.0** — `плюс / минус / умнож / раздел` accepted as Russian-
    // loan operator words common in spoken Kazakh and Whisper STT.
    const MATH_VERB_STEMS: &[&str] = &[
        "көбейт",
        "бөл",
        "қос",
        "есепте",
        "азайт",
        "плюс",
        "минус",
        "умнож",
        "раздел",
    ];
    // `ал` (subtract / take) is too short to use as a prefix —
    // checked below as a closed set of inflected forms.
    // **v4.41.0** — closed set of `ал` (subtract / take) inflected
    // forms recognised as math-verb. Bare imperative «ал» is
    // intentionally OMITTED — it doubles as the Kazakh sentence-
    // initial conjunction "and / but" («Ал онда қанша аймақ
    // бар?» — pre-cognitive_eval bare «ал» in SUB_FORMS triggered
    // math mode here on the «он» numeral prefix in «онда»,
    // breaking the v4.6.0 anaphora test). For subtraction prefer
    // the explicit imperative «алыңыз» / verbal noun «алу» /
    // converb «алып» / conditional «алсам / алсаң / алсаңыз».
    const SUB_FORMS: &[&str] = &[
        "алу",
        "алса",
        "алсам",
        "алсаң",
        "алсаңыз",
        "алыңыз",
        "алғанда",
        "алып",
    ];
    const KAZAKH_NUMERALS: &[&str] = &[
        "бір",
        "екі",
        "үш",
        "төрт",
        "бес",
        "алты",
        "жеті",
        "сегіз",
        "тоғыз",
        "он",
        "жиырма",
        "отыз",
        "қырық",
        "елу",
        "алпыс",
        "жетпіс",
        "сексен",
        "тоқсан",
        "жүз",
        "мың",
    ];
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphabetic()).collect();
    let non_empty_words: Vec<&&str> = words.iter().filter(|w| !w.is_empty()).collect();
    let has_math_verb = words.iter().any(|w| {
        if w.is_empty() {
            return false;
        }
        MATH_VERB_STEMS.iter().any(|stem| w.starts_with(stem)) || SUB_FORMS.contains(w)
    });
    // **v4.41.0** — bare imperative «ал» is the standalone form of
    // «to subtract» but doubles as the sentence-initial Kazakh
    // conjunction "and / but". Position disambiguates: math-«ал»
    // is sentence-final («Жүзден елуді ал»); conjunction-«ал» is
    // sentence-initial («Ал онда қанша аймақ бар?»). Accept ONLY
    // the sentence-final position.
    let has_bare_al_imperative = non_empty_words.last().map(|w| **w == "ал").unwrap_or(false);
    if !has_math_verb && !has_bare_al_imperative {
        return false;
    }
    let has_digit = input.chars().any(|c| c.is_ascii_digit());
    // **v4.6.12** — match inflected forms via prefix. Real-REPL
    // input «алтыны екіге бөліңіз» — `алтыны` is `алты` +
    // accusative, `екіге` is `екі` + dative. A pure
    // `KAZAKH_NUMERALS.contains(&w)` check misses both. Allowing
    // a numeral as a 2-4-char prefix of the surface word covers
    // case-inflected forms without false-positive matching on
    // unrelated content nouns (no Kazakh numeral overlaps with a
    // common content-noun stem of the same prefix length).
    let has_numeral_word = words.iter().any(|w| {
        if w.is_empty() {
            return false;
        }
        KAZAKH_NUMERALS
            .iter()
            .any(|n| w.starts_with(n) && w.chars().count() <= n.chars().count() + 3)
    });
    has_digit || has_numeral_word
}

/// **v4.6.15** — best-effort integer-arithmetic evaluator.
/// Pure deterministic computation — no novel-text generation, so
/// the no-fabrication invariant stays intact. Handles `+`, `-`,
/// `*`, `/`, `:` (Russian-style division) over signed integers
/// with operator precedence (`*` `/` `:` before `+` `-`). Returns:
///
/// - `Some(value)` for parseable pure-arithmetic input (e.g.
///   `5+5`, `7 * 3`, `100 - 25 / 5`, `7 + 3 =`, `6:2=`).
/// - `None` for unparseable input, division by zero, integer
///   overflow, or any non-integer division remainder. The planner
///   falls back to the existing `math_refusal` template family
///   when this returns `None`.
///
/// Limitations (intentional — keep the v4.6.15 scope tight):
/// **v5.18.1 — adversarial D2a ma_14 closure.** Detect inputs that
/// contain a literal division by zero (e.g. «10/0 қанша?»). When
/// matched, the planner routes to a dedicated `math_refusal.div_by_zero`
/// template that says «нөлге бөлуге болмайды» — a math-education
/// concept the student should learn, not a generic «I don't process
/// math» refusal. Conservative: requires the precise pattern
/// `<digit>` `/` `<optional whitespace>` `0` `<word-boundary or
/// non-digit>` so it doesn't false-fire on `10/01/2024` or `100`.
pub fn input_has_division_by_zero(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' {
            // Skip optional whitespace after '/'.
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            // Need at least one '0' followed by a word boundary
            // (end-of-input or non-digit). Forbid following digit
            // so «10/0» fires but «10/01» does not.
            if j < bytes.len() && bytes[j] == b'0' {
                let next = bytes.get(j + 1).copied().unwrap_or(b' ');
                if !next.is_ascii_digit() {
                    // Need at least one digit BEFORE the '/'.
                    let mut k = i;
                    while k > 0 && bytes[k - 1] == b' ' {
                        k -= 1;
                    }
                    if k > 0 && bytes[k - 1].is_ascii_digit() {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

/// **v5.18.1 — adversarial D2a closure (word-problem hallucination).**
/// Heuristic detector for Kazakh school-style math word problems.
/// Used to short-circuit the topic-extraction fallback so a query
/// like «Ботада 5 алма бар, оған тағы 3 алма бердім. Қазір қанша
/// алма?» routes to a dedicated graceful-refusal template instead
/// of surfacing a proverb about «бота» / definition of «алма» /
/// hallucinated digit echo.
///
/// Conservative fingerprint:
/// 1. Contains the Kazakh quantity-question word «қанша» / «барлығы»
///    / «жалпы» — every school word problem asks one of these.
/// 2. Contains AT LEAST TWO digits (Arabic numerals 0-9) OR at
///    least one digit AND at least one Kazakh number word
///    (бір/екі/үш/...). Single-digit prose like «Бір алма» is
///    not a math problem.
/// 3. Input length ≥ 20 codepoints — filters bare arithmetic
///    («5+3 қанша?») which the standard parser handles.
/// 4. NO standard arithmetic operator between two digits — if
///    such an operator exists, `try_evaluate_arithmetic` already
///    handles it; we shouldn't intercept.
///
/// This detector intentionally returns `bool`, not `Option<...>` —
/// adam does not yet HAVE a word-problem solver. The point is to
/// route the turn to an HONEST refusal instead of pretending an
/// answer exists.
pub fn is_kazakh_word_problem(input: &str) -> bool {
    let lower = input.to_lowercase();
    let asks_quantity = lower.contains("қанша")
        || lower.contains("барлығы")
        || lower.contains("жалпы")
        || lower.contains("неше");
    if !asks_quantity {
        return false;
    }
    if input.chars().count() < 20 {
        return false;
    }
    let digit_count = input.chars().filter(|c| c.is_ascii_digit()).count();
    let kazakh_number_words = [
        "бір",
        "екі",
        "үш",
        "төрт",
        "бес",
        "алты",
        "жеті",
        "сегіз",
        "тоғыз",
        "он",
        "жиырма",
        "отыз",
        "қырық",
        "елу",
        "алпыс",
        "жетпіс",
        "сексен",
        "тоқсан",
        "жүз",
        "мың",
    ];
    let kw_count = kazakh_number_words
        .iter()
        .filter(|w| lower.contains(*w))
        .count();
    let has_enough_numbers =
        digit_count >= 2 || (digit_count >= 1 && kw_count >= 1) || kw_count >= 2;
    if !has_enough_numbers {
        return false;
    }
    // Skip if input is essentially pure arithmetic with question
    // word — `try_evaluate_arithmetic` already handles those.
    let cleaned: String = input
        .chars()
        .filter(|c| {
            c.is_ascii_digit() || matches!(*c, '+' | '-' | '*' | '/' | '(' | ')' | '=' | ' ')
        })
        .collect();
    let has_binary_op = cleaned.chars().enumerate().any(|(i, c)| {
        matches!(c, '+' | '-' | '*' | '/')
            && i > 0
            && cleaned
                .chars()
                .nth(i - 1)
                .is_some_and(|p| p.is_ascii_digit() || p == ')')
    });
    if has_binary_op {
        return false;
    }
    true
}

/// - Integer-only; no fractions / decimals.
/// - No parentheses.
/// - No Kazakh-language phrasings («Алтыны екіге бөліңіз») — those
///   continue to refuse via `math_refusal`.
/// - No variables / session-bound computation.
pub fn try_evaluate_arithmetic(input: &str) -> Option<i64> {
    // **v4.75.0** — paren-aware recursive-descent evaluator. Replaces
    // the v4.74.0 token-based two-pass eval which silently stripped
    // parens before evaluation, breaking «P=2*(a+b)» (computed 13
    // instead of 16). Handles standard arithmetic grammar:
    //   expr   = term (('+' | '-') term)*
    //   term   = factor (('*' | '/') factor)*
    //   factor = '-' factor | '(' expr ')' | number
    //
    // Kazakh-wrapper stripping kept from v4.74.0: extract longest
    // arithmetic-character substring before parsing. Now includes
    // `(` and `)` in the kept-char set.
    let arith_only: String = input
        .chars()
        .filter(|c| {
            c.is_ascii_digit() || matches!(*c, '+' | '-' | '*' | '/' | ':' | '=' | '(' | ')' | ' ')
        })
        .collect();
    let cleaned: String = arith_only
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == ':' { '/' } else { c })
        .collect();
    let cleaned = cleaned.trim_end_matches('=').trim_end_matches('?');
    if cleaned.is_empty() {
        return None;
    }

    // **v5.18.1 — adversarial D2a wp_08 hallucination closure.**
    // Require at least one binary arithmetic operator BETWEEN two
    // digits, OR a leading unary minus («-5»). Pre-v5.18.1 a bare
    // POSITIVE integer like «10» (after stripping surrounding
    // Kazakh prose) parsed as a valid expression returning
    // `Some(10)` — the caller surfaced it as «Нәтижесі: 10 (он)»,
    // a confidently wrong word-problem answer. Bare unary-minus
    // («-5») is preserved as a special case because pre-v5.18.1
    // `handles_unary_minus` invariant depends on it (callers that
    // explicitly compute and display negative literals).
    let chars: Vec<char> = cleaned.chars().collect();
    let has_binary_op = chars.iter().enumerate().any(|(i, c)| {
        matches!(c, '+' | '-' | '*' | '/')
            && i > 0
            && chars
                .get(i - 1)
                .is_some_and(|p| p.is_ascii_digit() || *p == ')')
    });
    let starts_with_unary_minus = chars.first().is_some_and(|c| *c == '-');
    if !has_binary_op && !starts_with_unary_minus {
        return None;
    }

    let mut pos = 0;
    let value = parse_expr(&chars, &mut pos)?;
    if pos != chars.len() {
        return None;
    }
    Some(value)
}

fn parse_expr(chars: &[char], pos: &mut usize) -> Option<i64> {
    let mut left = parse_term(chars, pos)?;
    while *pos < chars.len() {
        let c = chars[*pos];
        if c != '+' && c != '-' {
            break;
        }
        *pos += 1;
        let right = parse_term(chars, pos)?;
        left = if c == '+' {
            left.checked_add(right)?
        } else {
            left.checked_sub(right)?
        };
    }
    Some(left)
}

fn parse_term(chars: &[char], pos: &mut usize) -> Option<i64> {
    let mut left = parse_factor(chars, pos)?;
    while *pos < chars.len() {
        let c = chars[*pos];
        if c != '*' && c != '/' {
            break;
        }
        *pos += 1;
        let right = parse_factor(chars, pos)?;
        left = if c == '*' {
            left.checked_mul(right)?
        } else {
            // v4.50.5 truncated integer division preserved.
            if right == 0 {
                return None;
            }
            left.checked_div(right)?
        };
    }
    Some(left)
}

fn parse_factor(chars: &[char], pos: &mut usize) -> Option<i64> {
    if *pos >= chars.len() {
        return None;
    }
    let c = chars[*pos];
    if c == '-' {
        *pos += 1;
        let inner = parse_factor(chars, pos)?;
        return inner.checked_neg();
    }
    if c == '+' {
        *pos += 1;
        return parse_factor(chars, pos);
    }
    if c == '(' {
        *pos += 1;
        let inner = parse_expr(chars, pos)?;
        if *pos >= chars.len() || chars[*pos] != ')' {
            return None;
        }
        *pos += 1;
        return Some(inner);
    }
    if c.is_ascii_digit() {
        let start = *pos;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            *pos += 1;
        }
        let s: String = chars[start..*pos].iter().collect();
        return s.parse().ok();
    }
    None
}

/// **v4.41.0** — Kazakh word-form math evaluator. Returns
/// `Some(value)` for parseable Kazakh-language arithmetic input
/// (e.g. «бесті отызға көбейту» = 5×30 = 150; «жүз пен елуді
/// қосу» = 100+50 = 150; «жүзді онға бөл» = 100/10 = 10).
/// Returns `None` when the input doesn't have the
/// `<num-word> <num-word> <math-verb>` shape, when number parsing
/// fails, or when computation overflows / divides non-evenly.
///
/// Pipeline:
/// 1. Strip case suffixes from each token (бесті → бес, отызға → отыз).
/// 2. Detect math operation by stem-prefix match against verb roots
///    (қос / көбейт / бөл / ал) — handles inflected forms like
///    «көбейтіңіз», «көбейткенде», «көбейтсек», «қоссаңыз», etc.
/// 3. Group consecutive number-word tokens into operands using
///    additive composition («жиырма бес» → 20+5 = 25) and
///    multiplicative composition with `жүз / мың / миллион` («екі
///    мың» → 2×1000 = 2000; «бір жүз елу» → 100+50 = 150).
/// 4. Apply the operation; return integer result. Non-integer
///    division falls back to `None` (planner picks math_refusal).
///
/// Why a separate evaluator from `try_evaluate_arithmetic`:
/// the digit-form version normalises whitespace and walks ASCII
/// arithmetic; the word-form version needs lexical recognition of
/// number words AND verb stems with Kazakh case morphology. Sharing
/// internals would mix two unrelated parse strategies.
/// **v4.74.0** — Procedural linear-equation solver for the simplest
/// 1-unknown form: `X + a = b`, `X - a = b`, `X * a = b`, `X / a = b`,
/// `a + X = b`, `a - X = b`, `a * X = b`. Detects the equation in a
/// natural-Kazakh wrapper («Егер X+2=5 болса, X қанша?») and returns
/// the integer solution when one exists.
///
/// Returns `Some(value)` when a single linear equation matches and
/// has an integer solution. Returns `None` otherwise — caller should
/// fall through to other handlers or refusal.
///
/// Driven by Codex 2026-05-06 round-2 review: «Егер x+2=5 болса, x
/// қанша?» previously refused. This is the smallest-scope procedural
/// solver — handles only single-unknown linear equations with integer
/// constants. Quadratic / multi-unknown / fraction-bearing equations
/// stay refused. Procedural solvers are deterministic per
/// `project_kazakh_tutor_positioning` — no NN involved.
pub fn try_solve_linear_equation(input: &str) -> Option<(String, i64, String)> {
    // Find the whitespace-separated token containing `=`. For
    // «Егер x+2=5 болса, x қанша?» that's «x+2=5». For «5+7*2 қанша?»
    // (no `=`), no token matches and we return None.
    let lower = input.to_lowercase();
    let eq_token = lower
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .find(|tok| tok.contains('='))?;
    // Strip trailing punctuation that might leak.
    let eq_token = eq_token.trim_end_matches(['?', '.', '!']);
    let parts: Vec<&str> = eq_token.split('=').collect();
    if parts.len() != 2 {
        return None;
    }
    let lhs = parts[0];
    let rhs_str = parts[1];
    // RHS must be a (possibly negative) integer.
    let rhs: i64 = rhs_str.trim().parse().ok()?;

    // Parse LHS — three forms:
    //   1. Single letter (the unknown), like «x»
    //   2. Letter + op + digits, like «x+2», «x-3», «x*4», «x/5»
    //   3. Digits + op + letter, like «2+x», «10-x»
    if lhs.chars().all(|c| c.is_alphabetic()) && !lhs.is_empty() {
        let steps = format!(
            "Берілген: {x_var} = {rhs}. Бұл — тривиал теңдеу, жауап тікелей берілген.",
            x_var = lhs,
            rhs = rhs,
        );
        return Some((lhs.to_string(), rhs, steps));
    }
    // Find the operator (must be exactly one).
    let op_idx = lhs.chars().enumerate().find_map(|(i, c)| {
        if matches!(c, '+' | '-' | '*' | '/') && i > 0 {
            Some(i)
        } else {
            None
        }
    })?;
    let left_part = &lhs[..op_idx];
    let op = lhs.chars().nth(op_idx)?;
    let right_part = &lhs[op_idx + op.len_utf8()..];

    let left_is_unknown = left_part.chars().all(|c| c.is_alphabetic()) && !left_part.is_empty();
    let right_is_unknown = right_part.chars().all(|c| c.is_alphabetic()) && !right_part.is_empty();

    if left_is_unknown && !right_is_unknown {
        // X op a = b
        let a: i64 = right_part.parse().ok()?;
        let (x, steps) = match op {
            '+' => (
                rhs - a,
                format!(
                    "Бастапқы теңдеу: {x_var} + {a} = {rhs}. {a}-ды екі жағынан да алып тастаймыз: {x_var} = {rhs} − {a} = {result}.",
                    x_var = left_part,
                    a = a,
                    rhs = rhs,
                    result = rhs - a,
                ),
            ),
            '-' => (
                rhs + a,
                format!(
                    "Бастапқы теңдеу: {x_var} − {a} = {rhs}. {a}-ды екі жағына да қосамыз: {x_var} = {rhs} + {a} = {result}.",
                    x_var = left_part,
                    a = a,
                    rhs = rhs,
                    result = rhs + a,
                ),
            ),
            '*' => {
                if a == 0 || rhs % a != 0 {
                    return None;
                }
                (
                    rhs / a,
                    format!(
                        "Бастапқы теңдеу: {x_var} · {a} = {rhs}. Екі жағын {a}-ға бөлеміз: {x_var} = {rhs} / {a} = {result}.",
                        x_var = left_part,
                        a = a,
                        rhs = rhs,
                        result = rhs / a,
                    ),
                )
            }
            '/' => (
                rhs * a,
                format!(
                    "Бастапқы теңдеу: {x_var} / {a} = {rhs}. Екі жағын {a}-ға көбейтеміз: {x_var} = {rhs} · {a} = {result}.",
                    x_var = left_part,
                    a = a,
                    rhs = rhs,
                    result = rhs * a,
                ),
            ),
            _ => return None,
        };
        return Some((left_part.to_string(), x, steps));
    }
    if right_is_unknown && !left_is_unknown {
        // a op X = b
        let a: i64 = left_part.parse().ok()?;
        let (x, steps) = match op {
            '+' => (
                rhs - a,
                format!(
                    "Бастапқы теңдеу: {a} + {x_var} = {rhs}. {a}-ды екі жағынан да алып тастаймыз: {x_var} = {rhs} − {a} = {result}.",
                    x_var = right_part,
                    a = a,
                    rhs = rhs,
                    result = rhs - a,
                ),
            ),
            '-' => (
                a - rhs,
                format!(
                    "Бастапқы теңдеу: {a} − {x_var} = {rhs}. {x_var} = {a} − {rhs} = {result}.",
                    x_var = right_part,
                    a = a,
                    rhs = rhs,
                    result = a - rhs,
                ),
            ),
            '*' => {
                if a == 0 || rhs % a != 0 {
                    return None;
                }
                (
                    rhs / a,
                    format!(
                        "Бастапқы теңдеу: {a} · {x_var} = {rhs}. Екі жағын {a}-ға бөлеміз: {x_var} = {rhs} / {a} = {result}.",
                        x_var = right_part,
                        a = a,
                        rhs = rhs,
                        result = rhs / a,
                    ),
                )
            }
            '/' => {
                if rhs == 0 || a % rhs != 0 {
                    return None;
                }
                (
                    a / rhs,
                    format!(
                        "Бастапқы теңдеу: {a} / {x_var} = {rhs}. {x_var} = {a} / {rhs} = {result}.",
                        x_var = right_part,
                        a = a,
                        rhs = rhs,
                        result = a / rhs,
                    ),
                )
            }
            _ => return None,
        };
        return Some((right_part.to_string(), x, steps));
    }
    None
}

/// **v4.74.5** — Procedural formula-applier. Handles the
/// physics/math curriculum pattern «F=m*a, m=2, a=3 болса F қанша?»:
/// a formula declaration with one unknown plus concrete substitutions
/// for the other variables. Returns `(unknown_var, value)` on success.
///
/// Single-character variables only (F, m, a, v, S, t, …). Formula RHS
/// must use explicit `*` for multiplication («F=m*a», not «F=ma»).
/// All substitutions must yield integer values; conservative —
/// fractional or unresolved-variable cases return `None`.
///
/// Driven by Codex 2026-05-06 round-2 review: «F=m*a, m=2, a=3
/// болса...» previously refused. This is the next procedural solver
/// after `try_solve_linear_equation` (v4.74.0). Still fully
/// deterministic — no NN.
pub fn try_apply_formula(input: &str) -> Option<(String, i64, String)> {
    use std::collections::HashMap;

    // Split into segments by comma / semicolon. Within each segment,
    // we look for `var = expr` pattern.
    let lower = input.to_lowercase();
    let segments: Vec<&str> = lower.split([',', ';']).collect();

    let mut numeric: HashMap<String, i64> = HashMap::new();
    let mut formula: Option<(String, String)> = None;

    for seg in &segments {
        // A segment may have multiple `=`; we only take the first
        // `var = expr` token. Find first whitespace-separated token
        // containing `=`.
        let token = seg
            .split_whitespace()
            .find(|t| t.contains('='))
            .unwrap_or("");
        if !token.contains('=') {
            continue;
        }
        let token = token.trim_end_matches(['?', '.', '!', ';', ',']);
        let parts: Vec<&str> = token.splitn(2, '=').collect();
        if parts.len() != 2 {
            continue;
        }
        let lhs = parts[0].trim();
        let rhs = parts[1].trim();
        // LHS must be a single letter (variable name).
        if lhs.chars().count() != 1 || !lhs.chars().next().unwrap().is_alphabetic() {
            continue;
        }
        // Categorise RHS: pure integer → numeric assignment; else if
        // it contains alphabetic chars → formula (only one allowed).
        if let Ok(n) = rhs.parse::<i64>() {
            numeric.insert(lhs.to_string(), n);
        } else if rhs.chars().any(|c| c.is_alphabetic()) {
            if formula.is_some() {
                // Multiple formula declarations — reject (out of scope).
                return None;
            }
            formula = Some((lhs.to_string(), rhs.to_string()));
        }
    }

    let (unknown, expr) = formula?;
    if numeric.is_empty() {
        return None;
    }

    // **v4.75.0** — paren guard lifted; `try_evaluate_arithmetic` is
    // now a paren-aware recursive-descent parser, so «P=2*(a+b)»
    // computes correctly. The v4.74.5 guard returned None for any
    // expression containing `(` or `)`; that's no longer needed.
    // Substitute single-letter variables in expr with their numeric
    // values. Non-alphabetic chars (operators, digits, parens,
    // whitespace) pass through. Unknown variables → None.
    let mut substituted = String::new();
    for c in expr.chars() {
        if c.is_alphabetic() {
            let var_str = c.to_string();
            match numeric.get(&var_str) {
                Some(val) => substituted.push_str(&val.to_string()),
                None => return None,
            }
        } else {
            substituted.push(c);
        }
    }

    let value = try_evaluate_arithmetic(&substituted)?;
    // Build step narrative: formula → substitutions → result
    let mut subs_list: Vec<(String, i64)> = numeric.iter().map(|(k, v)| (k.clone(), *v)).collect();
    subs_list.sort_by(|a, b| a.0.cmp(&b.0));
    let subs_str: String = subs_list
        .iter()
        .map(|(k, v)| format!("{} = {}", k, v))
        .collect::<Vec<_>>()
        .join(", ");
    let steps = format!(
        "Формула: {unknown} = {expr}. Берілген: {subs}. Орнына қою: {unknown} = {substituted} = {value}.",
        unknown = unknown,
        expr = expr,
        subs = subs_str,
        substituted = substituted,
        value = value,
    );
    Some((unknown, value, steps))
}

/// **v4.75.5** — Check-answer handler. Detects «жауабымды тексер: x=4»
/// / «менің жауабым: x=4» / «x=4 дұрыс па?» pattern and verifies the
/// user-supplied value against the previously stored answer for the
/// same variable. Returns `(user_var, user_value, correct)` triple
/// when input contains a check-phrase marker AND a `var=N` token AND
/// the variable matches `last_unknown` from session.
///
/// Conservative: doesn't try to re-solve from scratch or guess the
/// variable. Caller falls through to other handlers when None.
pub fn try_check_answer(
    input: &str,
    last_unknown: &str,
    last_value: i64,
) -> Option<(String, i64, bool)> {
    let lower = input.to_lowercase();
    let has_check_phrase = lower.contains("тексер")
        || lower.contains("дұрыс па")
        || lower.contains("дұрыс ма")
        || lower.contains("менің жауабым");
    if !has_check_phrase {
        return None;
    }
    let token = lower
        .split([',', ' ', ';', ':'])
        .filter(|s| !s.is_empty())
        .find(|tok| tok.contains('='))?;
    let token = token.trim_end_matches(['?', '.', '!']);
    let parts: Vec<&str> = token.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let user_var = parts[0].trim().to_string();
    let user_value: i64 = parts[1].trim().parse().ok()?;
    let last_unknown_lower = last_unknown.to_lowercase();
    if user_var != last_unknown_lower {
        return None;
    }
    let correct = user_value == last_value;
    Some((user_var, user_value, correct))
}

/// **v4.76.5** — Comparison-topics extractor. Detects «X пен Y
/// айырмашылығы қандай?» / «X-мен Y-нің айырмашылығы» and similar
/// comparison patterns; extracts X and Y as topic strings. Returns
/// `Some((x, y))` when a valid 2-topic comparison shape is found.
///
/// Conservative: only the bare «X пен Y» / «X мен Y» connector
/// pattern is supported. Ablative «X-дан Y несімен өзгеше?» pattern
/// is harder to parse without FST analysis and is deferred.
///
/// Closes Codex round-2 Bug 4 partial: «Тұрақты ток пен айнымалы ток
/// айырмашылығы қандай?» previously surfaced an unrelated proverb
/// about «екеуі» — now extracts the two topics correctly so the
/// planner can route to a comparison template that surfaces X's
/// definition and prompts the user to query Y separately for the
/// full pair. Full side-by-side comparison (both definitions in one
/// turn) is deferred to v5+ when dual retrieval lands.
pub fn try_extract_comparison_topics(input: &str) -> Option<(String, String)> {
    let lower = input.to_lowercase();
    let has_comparison_marker = lower.contains("айырмашылығы")
        || lower.contains("айырмашылық")
        || lower.contains("ерекшелігі")
        || lower.contains("несімен өзгеше")
        || lower.contains("несімен ерекшеленеді")
        || lower.contains("қалай ерекшеленеді")
        || lower.contains("қандай айырма")
        || lower.contains("салыстыр");
    if !has_comparison_marker {
        return None;
    }
    // Try «X пен Y» / «X мен Y» connector. Token-level match: the
    // connector word must be a separate token, not a substring inside
    // a longer word.
    const COMPARISON_TAILS: &[&str] = &[
        "айырмашылығы",
        "айырмашылық",
        "ерекшелігі",
        "несімен",
        "қалай",
        "қандай",
        "салыстыр",
    ];
    for connector in [" пен ", " мен "] {
        let Some(pos) = lower.find(connector) else {
            continue;
        };
        let lhs = lower[..pos].trim().to_string();
        let after = &lower[pos + connector.len()..];
        // Y ends at the first comparison-marker word.
        let y_end = COMPARISON_TAILS
            .iter()
            .filter_map(|m| after.find(m))
            .min()
            .unwrap_or(after.len());
        let rhs = after[..y_end].trim().to_string();
        if lhs.is_empty() || rhs.is_empty() || lhs == rhs {
            continue;
        }
        // Strip leading discourse particles from X if present.
        let mut x = lhs.as_str();
        for prefix in ["егер ", "бір "] {
            if let Some(stripped) = x.strip_prefix(prefix) {
                x = stripped;
            }
        }
        // **v5.5.0** — return LITERAL tokens (no case stripping at
        // extraction time). The v4.77.5 case-stripping was correct
        // for «Үкіметтің» (genitive on common noun) but broke proper
        // nouns: «Алматы» → «алма» (because «-ты» was treated as Acc
        // suffix), «Астана» → «аста» («-на» as Dat suffix). The 2026-
        // 05-09 30-turn audit surfaced this on «Алматы мен Астана
        // айырмашылығы». Pushing the stripping into a fallback at
        // lookup time gives both correctness branches:
        //   1. literal token matches a proper noun in world_core
        //      → use it (correct for Алматы / Астана)
        //   2. literal misses, case-stripped form matches
        //      → use the stripped form (preserves Үкіметтің fix)
        // Conversation::comparison_lookup_chain implements the
        // two-phase fallback explicitly.
        return Some((x.trim().to_string(), rhs.trim().to_string()));
    }
    None
}

/// **v5.5.0** — public wrapper exposing the case-stripping helper to
/// `conversation.rs` for the comparison-lookup fallback. Pre-v5.5.0
/// the function was crate-private and stripping happened at
/// extraction time; v5.5.0 inverts the order so `try_extract_
/// comparison_topics` returns literal tokens and the caller tries
/// case-stripping only after a literal lookup miss.
pub fn strip_trailing_case_for_lookup(s: &str) -> &str {
    strip_trailing_kazakh_case(s)
}

/// **v4.77.5** — Strip a trailing Kazakh case suffix from a noun
/// phrase to recover the lemma. Used by `try_extract_comparison_topics`
/// to normalize Y in «X пен Y-нің айырмашылығы» so the dual lookup
/// can find the world_core entry. Conservative: only strips when the
/// remaining stem is ≥ 2 chars; preserves short tokens that happen to
/// end in a suffix-like sequence.
fn strip_trailing_kazakh_case(s: &str) -> &str {
    // Order matters: longest suffixes first to prevent over-stripping.
    const SUFFIXES: &[&str] = &[
        // Genitive (longest first)
        "нің", "ның", "дің", "дың", "тің", "тың", // Locative
        "нде", "нда", "дегі", "дағы", "тегі", "тағы", // Ablative
        "нен", "нан", "ден", "дан", "тен", "тан", // Dative
        "ге", "ға", "ке", "қа", "не", "на", // Accusative
        "ні", "ны", "ді", "ды", "ті", "ты",
        // Possessive (already stripped at FST level usually, but safety)
        "сы", "сі",
    ];
    for suf in SUFFIXES {
        if let Some(stem) = s.strip_suffix(suf) {
            // Char-count guard: require ≥ 4-char stem (most Kazakh
            // content words have ≥ 4 chars before any suffix).
            if stem.chars().count() >= 4 {
                return stem.trim_end();
            }
        }
    }
    s
}

/// **v4.76.0** — Explain-steps handler. Detects «қалай шештің / есепті
/// қалай шеш / процесін көрсет / қадам-қадаммен / түсіндір» pattern in
/// a follow-up turn after a successful equation/formula solve. Returns
/// `Some(steps_text)` — surfaces the stored step narrative for the
/// last solved equation.
///
/// Requires session state (`last_math_steps`) populated by the prior
/// solver call. Returns None when no marker present or when no stored
/// steps available.
///
/// Closes Codex round-2 Bug 2 family fully (3/3 — alongside
/// `try_solve_linear_equation`, `try_apply_formula`, `try_check_answer`).
pub fn try_explain_steps(input: &str, last_steps: &str) -> Option<String> {
    if last_steps.is_empty() {
        return None;
    }
    let lower = input.to_lowercase();
    let has_explain_phrase = lower.contains("қалай шеш")
        || lower.contains("процесін")
        || lower.contains("қадам")
        || lower.contains("түсіндір")
        || lower.contains("дәлелде")
        || lower.contains("шешуін көрсет");
    if !has_explain_phrase {
        return None;
    }
    Some(last_steps.to_string())
}

pub fn try_evaluate_kazakh_word_math(input: &str) -> Option<i64> {
    // **v6.0** — same geometry / measurement gate as
    // `extract_kazakh_math_summary`. Defense in depth: a geometric
    // question that happens to contain two case-marked numerals and
    // a math-verb root (e.g. «Үш бұрыштың қосындысы») must not
    // silently return a numeric answer.
    if input_has_geometry_or_measurement_context(&input.to_lowercase()) {
        return None;
    }
    // **v4.42.0** — multi-clause support. Split input by commas /
    // sequencing connectives («және» — "and", «содан кейін» — "then",
    // «соңында» — "at the end") so chained operations like
    // «Беске жетіні қоссақ, екіге көбейтеміз, үшке бөлеміз және
    // бесті азайтамыз» — ((5+7)*2)/3-5 = 3 — evaluate left-to-right
    // with the running accumulator carried between clauses.
    //
    // Pipeline:
    //   - First clause: must provide BOTH operands (2 numbers with
    //     explicit case morphology) and one math verb. Result is
    //     the seed accumulator.
    //   - Each subsequent clause: one operand + one verb; the
    //     accumulator becomes the left operand, the new operand
    //     the right.
    //   - Any clause failing to satisfy this shape → return None
    //     and let the planner pick math_refusal.
    let lower = input.to_lowercase();
    // **v4.53.0** — gerund/converb-form clause separation. Real-REPL
    // session 5 surfaced «Елуді екіге **көбейткенде** үшке **бөліп**,
    // 7-ні **азайтқанда** не болады?» — the same chain that works in
    // imperative form («... көбейтіңіз, ... бөліңіз, ... азайтыңыз»)
    // failed because the gerund/converb forms lack overt clause
    // boundaries. Pre-v4.53.0 the parser saw 3 case-marked numerals
    // + multiple verbs in one super-clause and refused.
    //
    // Fix: insert `__CLAUSE_SEP__` after each gerund/converb form
    // of a math verb. The post-clause-split parser already handles
    // chained operations against an accumulator, so once the
    // gerund-form clauses are split, evaluation falls through to
    // the existing v4.42.0 multi-clause path.
    //
    // Gerund forms covered: «-ғанда / -генде / -қанда / -кенде»
    // (past-participle + Locative — "when I X-ed"). Converb forms:
    // «-ып / -іп / -п» (sequential action — "having X-ed").
    // Conditional «-се / -сек / -сем / -сеңіз» NOT added here —
    // those forms (e.g., «көбейтсем») are already handled inside
    // single_clause_kazakh_math via the existing detect_kazakh_math_op
    // tests and don't typically appear in multi-clause chains.
    let lower = inject_gerund_clause_separators(&lower);
    let normalized: String = lower
        .replace(',', " __CLAUSE_SEP__ ")
        .replace(" және ", " __CLAUSE_SEP__ ")
        .replace(" содан кейін ", " __CLAUSE_SEP__ ")
        // **v6.0 (live REPL 2026-05-18)** — colloquial «сосын» is the
        // spoken-language equivalent of formal «содан кейін» («then»);
        // Kazakh speakers default to it in everyday speech. Add as a
        // first-class clause separator so spoken-style math («бесті
        // отызға көбейт, сосын екіге бөл») chains the same way written
        // formal style does.
        .replace(" сосын ", " __CLAUSE_SEP__ ")
        .replace(" соңында ", " __CLAUSE_SEP__ ");
    let clauses: Vec<&str> = normalized
        .split("__CLAUSE_SEP__")
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();
    if clauses.is_empty() {
        return None;
    }
    let mut iter = clauses.iter();
    // First clause — bootstrap with two operands.
    let first = iter.next()?;
    let mut accumulator = single_clause_kazakh_math(first)?;
    // Subsequent clauses — apply one op against running accumulator.
    // Trailing non-math clauses («нәтижесі қандай болады») are
    // skipped (return None from sequel parser → treat as appendage,
    // not failure).
    for clause in iter {
        if let Some(next) = sequel_clause_kazakh_math(clause, accumulator) {
            accumulator = next;
        } else if clause_has_math_verb(clause) {
            // Math verb present but couldn't parse → real failure.
            return None;
        }
        // No math verb → trailing rhetorical appendage; ignore.
    }
    Some(accumulator)
}

/// **v4.53.0** — Insert `__CLAUSE_SEP__` markers after gerund /
/// converb forms of math verbs so the multi-clause evaluator
/// (v4.42.0) can chain operations across them.
///
/// Gerund («when-I-X») suffix family: -ғанда / -генде / -қанда /
/// -кенде. Converb («having-X-ed») suffix family: -ып / -іп / -п.
/// Both forms attach to the four math-verb roots: көбейт, бөл, қос,
/// азайт.
///
/// **v5.21.0 — math echo specificity.** Summary of what the
/// Kazakh-word-math layer was able to extract from an utterance,
/// even when full evaluation failed. Used to convert a generic
/// «I can't compute that» refusal into a transparent echo that
/// shows the user which numbers and operators were understood
/// and asks for a specific format the parser CAN evaluate.
#[derive(Debug, Clone, PartialEq)]
pub struct KazakhMathSummary {
    /// Numbers recognised in the input, in order of appearance.
    /// Includes both Arabic digits and Kazakh number words; tens +
    /// units are merged («елу алты» → 56).
    pub numbers: Vec<i64>,
    /// Math operators recognised, in order of appearance.
    pub operators: Vec<KazakhMathOpName>,
    /// Whether the input contains at least one indicator that this
    /// IS a math request — a Kazakh math verb stem, an Arabic
    /// operator (+/-/*), or a question word about computation.
    pub looks_like_math: bool,
}

/// Public surface form of [`KazakhMathOp`]. The internal `enum`
/// stays private (other modules don't need to construct one); this
/// variant set is exposed for callers building user-facing prose
/// like «56-ны 7-ге көбейтіп 3-ке бөл».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KazakhMathOpName {
    Add,
    Sub,
    Mul,
    Div,
}

impl KazakhMathOpName {
    /// The Arabic-arithmetic glyph for this operator. Used when
    /// echoing back a partial parse to the user in the «56*7/3»
    /// canonical form they can re-type.
    pub fn glyph(self) -> char {
        match self {
            Self::Add => '+',
            Self::Sub => '-',
            Self::Mul => '*',
            Self::Div => '/',
        }
    }
}

impl From<KazakhMathOp> for KazakhMathOpName {
    fn from(op: KazakhMathOp) -> Self {
        match op {
            KazakhMathOp::Add => Self::Add,
            KazakhMathOp::Sub => Self::Sub,
            KazakhMathOp::Mul => Self::Mul,
            KazakhMathOp::Div => Self::Div,
        }
    }
}

/// **v6.0 (live REPL 2026-05-18)** — does the input mention a
/// geometric figure, geometric attribute, physical-measurement unit,
/// or geographic-distance unit? If yes, the math-summary extractor
/// returns None so the dialog doesn't echo a misparsed numeral
/// («Мен «3» деп ұқтым…») on a question that wasn't arithmetic at
/// all.
///
/// Conservative list: only words whose presence strongly disqualifies
/// the input from being a calculator query. NOT in the list:
/// generic length / amount nouns («ұзындық», «көп») that legitimately
/// appear in word-math problems ("if the length is 5, multiply by
/// 2"). Triggers must be context-specific enough that the user
/// genuinely is NOT asking adam to compute.
fn input_has_geometry_or_measurement_context(lower: &str) -> bool {
    const MARKERS: &[&str] = &[
        // Geometric figures
        "бұрыш",
        "үшбұрыш",
        "шеңбер",
        "квадрат",
        "тіктөртбұрыш",
        "ромб",
        "трапеция",
        "пирамида",
        "сфера",
        "куб",
        "цилиндр",
        // Geometric attributes
        "градус",
        "радиус",
        "диаметр",
        "периметр",
        "аудан",
        "көлем",
        "биіктік",
        "гипотенуза",
        // Distance / area units suggesting geography or physics,
        // not a calculator question
        "километр",
        "шақырым",
        "метр",
        "сантиметр",
        "миллиметр",
        "шаршы километр",
        "гектар",
        // Time-context words that suggest the question is about
        // duration / dates, not a sum
        "ғасыр",
        "ғасырда",
    ];
    MARKERS.iter().any(|m| lower.contains(m))
}

/// **v5.21.0 — math echo specificity.** Extract a [`KazakhMathSummary`]
/// from a Kazakh-language math request even when
/// [`try_evaluate_kazakh_word_math`] can't fully evaluate it.
///
/// The goal is **transparent refusal**: rather than emitting the
/// generic `math_refusal` family («Санақ-есептеу әлі қазақша
/// сөйлемдер арқылы менің мүмкіндігімде жоқ»), the dialog layer
/// uses this summary to compose «56-ны 7-ге көбейтіп 3-ке бөл деп
/// ұқтым — арифметика түрінде жазсаңыз («56*7/3»), есептеп
/// беремін». The user sees that adam parsed the request and knows
/// exactly which format the system accepts.
///
/// Returns `None` when nothing math-like is visible in the input —
/// no Kazakh math verbs, no digits, no math-question words. That
/// case falls through to the existing topic-extraction path.
pub fn extract_kazakh_math_summary(input: &str) -> Option<KazakhMathSummary> {
    let lower = input.to_lowercase();
    // **v6.0 (live REPL 2026-05-18)** — geometry / measurement gate.
    // Inputs like «Үш бұрыштың бұрыштарының қосындысы қанша градус?»
    // («What's the sum of the angles of a triangle?») used to slip
    // into the math-echo path because they pattern-match on
    // numeral + sum-verb + «қанша». The user gets an unhelpful echo
    // («Мен «3» деп ұқтым …») of a number they never asked about.
    // Geometric/measurement context-words signal a non-arithmetic
    // question; refuse to extract a math summary so the planner
    // routes to topic-extraction or the dedicated knowledge
    // refusal family instead.
    if input_has_geometry_or_measurement_context(&lower) {
        return None;
    }
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .collect();

    let mut numbers: Vec<i64> = Vec::new();
    let mut operators: Vec<KazakhMathOpName> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        // Arabic digit literal.
        if let Ok(n) = t.parse::<i64>() {
            numbers.push(n);
            i += 1;
            continue;
        }
        // Look up the bare token first. Only fall back to case-strip
        // for tokens that don't match a bare numeral — case-strip
        // can over-trim short forms like «алты» (6), where the «ты»
        // suffix is part of the stem, not a case marker.
        let bare_units = kazakh_units_value_local(t);
        let bare_tens = kazakh_tens_value_local(t);
        let used_bare = bare_units.is_some() || bare_tens.is_some();
        let stripped = if used_bare {
            t
        } else {
            strip_kazakh_numeral_case(t)
        };
        if let Some(units) = bare_units.or_else(|| kazakh_units_value_local(stripped)) {
            // Tens + units fusion: «елу алты» → 56 only when the
            // units token is BARE (no case-marker stripped off).
            // Case-marked units like «үшке» («to three», Dative) are
            // operands in their own right — they go into a separate
            // numbers slot for the math evaluator. Without this
            // guard, «Елу үшке көбейт» fuses into 53 and loses the
            // 50 × 3 structure.
            if used_bare
                && let Some(last) = numbers.last_mut()
                && (10..=90).contains(last)
                && *last % 10 == 0
                && i > 0
                && kazakh_tens_value_local(strip_kazakh_numeral_case(tokens[i - 1])).is_some()
            {
                *last += units as i64;
            } else {
                numbers.push(units as i64);
            }
            i += 1;
            continue;
        }
        if let Some(tens) = bare_tens.or_else(|| kazakh_tens_value_local(stripped)) {
            numbers.push(tens as i64);
            i += 1;
            continue;
        }
        // Math operator: try the existing detector on a one-token
        // and two-token window.
        if let Some(op) = detect_kazakh_math_op(&[t]) {
            operators.push(op.into());
            i += 1;
            continue;
        }
        if i + 1 < tokens.len()
            && let Some(op) = detect_kazakh_math_op(&[t, tokens[i + 1]])
        {
            operators.push(op.into());
            i += 2;
            continue;
        }
        i += 1;
    }

    let arabic_op = lower.chars().any(|c| matches!(c, '+' | '-' | '*' | '/'));
    let math_question =
        lower.contains("қанша") || lower.contains("нәтиже") || lower.contains("есепте");
    let looks_like_math = !operators.is_empty() || arabic_op || math_question;

    if numbers.is_empty() && operators.is_empty() && !looks_like_math {
        return None;
    }
    if numbers.is_empty() {
        return None;
    }

    Some(KazakhMathSummary {
        numbers,
        operators,
        looks_like_math,
    })
}

/// **v5.21.5 — universal raw-input echo.** Returns the user's input
/// trimmed and ready to quote inside a transparent-refusal template
/// when:
///
/// - Length is between 3 and 60 codepoints (single-char
///   interjections carry no signal; long inputs cite too much and
///   may leak PII).
/// - Majority of letters are Kazakh Cyrillic (Kazakh-only directive
///   per `project_kazakh_only_directive`; non-Kazakh inputs route to
///   the dedicated language guard `unknown.non_kazakh`).
/// - No digits, no curly braces, no `` ` `` backticks (would risk
///   echoing pasted code or sensitive numerics).
/// - No URLs, emails, or `@` markers — too risky.
///
/// Returns `None` when the input doesn't qualify, so the caller
/// falls through to the existing session-aware / generic clarify
/// path. This is intentionally conservative — the cost of NOT
/// echoing is one extra «Сұрағыңызды толық түсінбедім» turn; the
/// cost of echoing wrong content is a privacy / hallucination leak.
pub fn safe_echo_input(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let cp_count = trimmed.chars().count();
    if !(3..=60).contains(&cp_count) {
        return None;
    }
    if trimmed.contains(['{', '}', '`'])
        || trimmed.contains("http")
        || trimmed.contains("://")
        || trimmed.contains('@')
    {
        return None;
    }
    if trimmed.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let letter_count = trimmed.chars().filter(|c| c.is_alphabetic()).count();
    if letter_count == 0 {
        return None;
    }
    let cyrillic_count = trimmed
        .chars()
        .filter(|c| c.is_alphabetic() && (*c as u32) >= 0x0400 && (*c as u32) <= 0x04FF)
        .count();
    // Require ≥ 70 % Cyrillic among alphabetic chars.
    if (cyrillic_count as f32) / (letter_count as f32) < 0.70 {
        return None;
    }
    Some(trimmed.to_string())
}

/// Strip common Kazakh case-marking suffixes from a numeral token
/// so the lookup tables (which list bare stems) match. Conservative
/// — only handles the case set the math evaluator already supports.
fn strip_kazakh_numeral_case(token: &str) -> &str {
    for suffix in [
        "ны", "ні", "ды", "ді", "ты", "ті", // Accusative
        "ға", "ге", "қа", "ке", // Dative
        "нан", "нен", "дан", "ден", "тан", "тен", // Ablative
        "да", "де", "та", "те", // Locative
    ] {
        if let Some(stripped) = token.strip_suffix(suffix)
            && !stripped.is_empty()
        {
            return stripped;
        }
    }
    token
}

fn kazakh_units_value_local(token: &str) -> Option<u32> {
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

fn kazakh_tens_value_local(token: &str) -> Option<u32> {
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

/// **v5.21.0** — render a `KazakhMathSummary` into a canonical
/// arithmetic expression for the echo template («56 × 7 ÷ 3» style).
/// Returns `None` when the summary doesn't have enough numbers
/// (N) and operators (N-1) to form a sensible chain.
pub fn render_math_summary_as_arithmetic(summary: &KazakhMathSummary) -> Option<String> {
    if summary.numbers.is_empty() {
        return None;
    }
    let mut out = String::new();
    out.push_str(&summary.numbers[0].to_string());
    let mut op_iter = summary.operators.iter();
    for n in summary.numbers.iter().skip(1) {
        if let Some(op) = op_iter.next() {
            out.push_str(&format!(" {} ", op.glyph()));
        } else {
            // Numbers without enough operators: separator-only.
            out.push_str(" ? ");
        }
        out.push_str(&n.to_string());
    }
    Some(out)
}

/// Conservative: only inserts AFTER a token that's recognisable as
/// a math-verb gerund/converb, so non-math gerund forms in the
/// input are left alone.
fn inject_gerund_clause_separators(input: &str) -> String {
    // 16 surface forms: 4 stems × 4 suffix variants (gerund "-ғанда/
    // -генде/-қанда/-кенде" and converb "-ып/-іп/-п" composed with
    // the appropriate vowel-harmony class for each root).
    const GERUND_FORMS: &[&str] = &[
        // көбейт (multiply): voiceless т-final → -кенде / -іп
        "көбейткенде",
        "көбейтіп",
        // бөл (divide): sonorant л-final → -генде / -іп (front vowel)
        "бөлгенде",
        "бөліп",
        // қос (add): voiceless с-final, back vowel → -қанда / -ып
        "қосқанда",
        "қосып",
        // азайт (subtract): voiceless т-final, back vowel → -қанда / -ып
        "азайтқанда",
        "азайтып",
    ];
    // Pad with leading/trailing spaces so token-boundary matches
    // are uniform (handles input-edge tokens without special-casing).
    let padded = format!(" {} ", input);
    let mut result = padded;
    for form in GERUND_FORMS {
        // Replace " <form> " with " <form> __CLAUSE_SEP__ " — only
        // matches whole-token instances. The `__CLAUSE_SEP__` placement
        // AFTER the gerund/converb means the gerund clause closes with
        // the verb (consistent with how the comma path works).
        let needle = format!(" {} ", form);
        let replacement = format!(" {} __CLAUSE_SEP__ ", form);
        result = result.replace(&needle, &replacement);
    }
    // Trim back to original padding. Outer whitespace doesn't matter
    // downstream because the next step splits on __CLAUSE_SEP__ and
    // trims clauses, but keep the function output predictable.
    result.trim().to_string()
}

fn clause_has_math_verb(clause: &str) -> bool {
    // **v4.54.5** — «есепте» REMOVED from real-math-verb list. The
    // top-level `input_is_math_expression` gate still recognises
    // «есепте-» (auxiliary "calculate"), but inside the multi-clause
    // loop we no longer treat it as a real math operator.
    //
    // Reason: real-REPL session 6 surfaced «...қосқанда **не
    // болатынын есептеп**, 2-ге көбейтіп, 4-ке бөліңіз». After
    // gerund-separator injection, the «не болатынын есептеп» clause
    // had no operands but was treated as a math-verb-bearing clause
    // and failed the whole evaluation. With «есепте» off the list,
    // such auxiliary appendages fall through silently to the
    // accumulator-carry path. The four real operators (көбейт / бөл /
    // қос / азайт) plus the closed `ал`-forms still gate failure.
    let lowered = clause.to_lowercase();
    lowered.split(|c: char| !c.is_alphabetic()).any(|w| {
        !w.is_empty()
            && (w.starts_with("көбейт")
                || w.starts_with("бөл")
                || w.starts_with("қос")
                || w.starts_with("азайт")
                || matches!(
                    w,
                    "алу"
                        | "алса"
                        | "алсам"
                        | "алсаң"
                        | "алсаңыз"
                        | "алыңыз"
                        | "алғанда"
                        | "алып"
                ))
    })
}

fn split_kazakh_math_clause(clause: &str) -> Vec<&str> {
    // **v4.41.7** — split on whitespace + non-alphanumeric chars
    // EXCEPT '-'. Pre-v4.41.7 the predicate was `!c.is_alphabetic()`
    // which dropped digits entirely (chars '3' and '0' aren't
    // alphabetic, so the splitter cut between them, leaving "30"
    // as a sequence of empty strings). Real-REPL transcript
    // 2026-05-03 typed «30-ды азайтыңыз» (digit form with Kazakh
    // case suffix); pre-v4.41.7 the «30» was lost entirely and the
    // chunk had no operand. Keeping `'-'` as part of tokens lets
    // «30-ды» survive as one token, parsed by
    // `parse_kazakh_number_token`'s digit-prefix branch.
    clause
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|t| !t.is_empty())
        .collect()
}

fn single_clause_kazakh_math(clause: &str) -> Option<i64> {
    let raw_tokens = split_kazakh_math_clause(clause);
    if raw_tokens.len() < 3 {
        return None;
    }
    let op = detect_kazakh_math_op(&raw_tokens)?;
    let operands = extract_kazakh_number_operands(&raw_tokens);
    if operands.len() != 2 {
        return None;
    }
    apply_kazakh_math_op(op, operands[0], operands[1])
}

fn sequel_clause_kazakh_math(clause: &str, accumulator: i64) -> Option<i64> {
    let raw_tokens = split_kazakh_math_clause(clause);
    let op = detect_kazakh_math_op(&raw_tokens)?;
    let operands = extract_kazakh_number_operands(&raw_tokens);
    // Sequel clauses provide exactly ONE additional operand —
    // the accumulator from previous clauses serves as the left
    // operand. «Екіге көбейтеміз» = «running × 2».
    if operands.len() != 1 {
        return None;
    }
    apply_kazakh_math_op(op, accumulator, operands[0])
}

fn apply_kazakh_math_op(op: KazakhMathOp, a: i64, b: i64) -> Option<i64> {
    match op {
        KazakhMathOp::Add => a.checked_add(b),
        KazakhMathOp::Sub => a.checked_sub(b),
        KazakhMathOp::Mul => a.checked_mul(b),
        KazakhMathOp::Div => {
            // **v4.50.5** — truncated integer division (mirrors
            // try_evaluate_arithmetic). Pre-v4.50.5 refused non-
            // integer results, forcing math_refusal on simple
            // queries like «бесті жетіге көбейтіп, екіге бөліңіз»
            // (= 35/2 = 17). Truncating to 17 is closer to user
            // expectations.
            if b == 0 { None } else { Some(a / b) }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KazakhMathOp {
    Add,
    Sub,
    Mul,
    Div,
}

fn detect_kazakh_math_op(tokens: &[&str]) -> Option<KazakhMathOp> {
    // Stem-prefix match against verb roots. Order matters: longer
    // / more-specific stems first so «көбейту» wins over a
    // hypothetical «көб» (shorter prefix). All four ops covered;
    // `ал` (subtract / take) is intentionally checked LAST because
    // it's the shortest stem and would otherwise eat plenty of
    // unrelated tokens.
    for t in tokens {
        if t.starts_with("көбейт") {
            return Some(KazakhMathOp::Mul);
        }
        if t.starts_with("бөл") {
            return Some(KazakhMathOp::Div);
        }
        if t.starts_with("қос") {
            return Some(KazakhMathOp::Add);
        }
        // **v4.42.0** — `азайт*` (decrease / subtract) — alternate
        // verb stem for subtraction. «Бесті азайт» = «subtract 5».
        if t.starts_with("азайт") {
            return Some(KazakhMathOp::Sub);
        }
        // **v6.0 (live REPL 2026-05-18)** — Russian-loan operator
        // words used in spoken Kazakh: «плюс», «минус», «умножить»,
        // «разделить» (and their accusative/dative forms). Common
        // in casual speech and in Whisper STT output, since Whisper
        // sometimes outputs the Russian token verbatim when the
        // surface is mixed. Conservative: only the four core
        // operators, no extension to multi-word phrases.
        if t.starts_with("плюс") {
            return Some(KazakhMathOp::Add);
        }
        if t.starts_with("минус") {
            return Some(KazakhMathOp::Sub);
        }
        if t.starts_with("умнож") {
            return Some(KazakhMathOp::Mul);
        }
        if t.starts_with("раздел") {
            return Some(KazakhMathOp::Div);
        }
    }
    // `ал` separately because of its short length — accept only when
    // the stem is exactly one of the known math-verb forms (not a
    // prefix of unrelated nouns like `алма` "apple" / `алмас` "won't
    // take" / `алыс` "far"). Conservative whitelist.
    // **v4.41.0** — closed set of `ал` (subtract / take) inflected
    // forms recognised as math-verb. Bare imperative «ал» is
    // intentionally OMITTED — it doubles as the Kazakh sentence-
    // initial conjunction "and / but" («Ал онда қанша аймақ
    // бар?» — pre-cognitive_eval bare «ал» in SUB_FORMS triggered
    // math mode here on the «он» numeral prefix in «онда»,
    // breaking the v4.6.0 anaphora test). For subtraction prefer
    // the explicit imperative «алыңыз» / verbal noun «алу» /
    // converb «алып» / conditional «алсам / алсаң / алсаңыз».
    const SUB_FORMS: &[&str] = &[
        "алу",
        "алса",
        "алсам",
        "алсаң",
        "алсаңыз",
        "алыңыз",
        "алғанда",
        "алып",
    ];
    for t in tokens {
        if SUB_FORMS.contains(t) {
            return Some(KazakhMathOp::Sub);
        }
    }
    // **v4.41.0** — bare imperative «ал» accepted ONLY when
    // sentence-final (canonical imperative position; sentence-
    // initial «ал» is the conjunction "and / but" — closes the
    // false-positive on «Ал онда қанша аймақ бар?»).
    if tokens.last().is_some_and(|t| *t == "ал") {
        return Some(KazakhMathOp::Sub);
    }
    None
}

fn extract_kazakh_number_operands(tokens: &[&str]) -> Vec<i64> {
    // **v4.41.0** — case-morphology-aware operand extraction.
    //
    // Algorithm: a "number chunk" accumulates consecutive number-
    // word tokens via standard number-composition (additive for
    // descending magnitude — «жиырма бес» = 20+5 = 25;
    // multiplicative for `жүз`/`мың`/`миллион` — «бір мың
    // тоғыз жүз» = 1*1000 + 9*100 = 1900). A chunk CLOSES when:
    //  - the token has an explicit case suffix («бесті» Acc /
    //    «отызға» Dat / «жүзден» Abl) — the case marker is the
    //    user's signal that this number is a complete operand
    //    serving a syntactic role;
    //  - OR a non-number token (verb / separator) appears.
    //
    // This handles the canonical Kazakh math phrasings:
    //   «Бесті отызға көбейту»          → [5, 30] × → 150
    //   «Жиырма бесті отызға көбейту»   → [25, 30] × → 750
    //   «Жүз елуді бір мыңға қос»       → [150, 1000] + → 1150
    //
    // Without case morphology, two consecutive number words
    // compose normally (so «жиырма бес есепте» means «count up to
    // 25» — single operand 25). The case suffix is the
    // disambiguator between «25» and «20-and-5-as-separate-args».
    // **v4.41.7** — pre-scan: count case-marked numbers in this
    // clause. Determines whether `пен/мен/бен` between numbers
    // should MERGE additively into a single chunk (count ≥ 2) or
    // SPLIT into separate operands (count == 1).
    //
    // Reasoning:
    // - «Қырық пен бесті қосып» (count = 1, only `бесті`) →
    //   user means "add 40 AND 5" → operands [40, 5], op=Add → 45.
    //   The single case-marked operand signals a binary operation
    //   on the two pen-conjoined numbers.
    // - «Қырық пен бесті екіге көбейтіп» (count = 2, `бесті` and
    //   `екіге`) → user means "(40+5) × 2" → operands [45, 2],
    //   op=Mul → 90. The two case-marked positions ARE the
    //   binary operands; пен-conjoined numbers are a compound
    //   first operand.
    let case_marked_count = tokens
        .iter()
        .filter(|t| parse_kazakh_number_token(t).is_some_and(|(_, has_case)| has_case))
        .count();
    let pen_merges = case_marked_count >= 2;

    let mut operands = Vec::new();
    let mut chunk_total: i64 = 0;
    let mut chunk_inflight = false;
    for t in tokens {
        if let Some((value, has_case)) = parse_kazakh_number_token(t) {
            // Compose into the current chunk.
            chunk_total = compose_number_in_chunk(chunk_total, value);
            chunk_inflight = true;
            if has_case {
                operands.push(chunk_total);
                chunk_total = 0;
                chunk_inflight = false;
            }
        } else if pen_merges && matches!(*t, "пен" | "мен" | "бен") && chunk_inflight {
            // Pen-conjoined merge — chunk stays open, next number
            // adds to chunk_total. Active only when 2+ case-marked
            // operands are present in the clause.
        } else if chunk_inflight {
            operands.push(chunk_total);
            chunk_total = 0;
            chunk_inflight = false;
        }
    }
    if chunk_inflight {
        operands.push(chunk_total);
    }
    operands
}

fn compose_number_in_chunk(current: i64, next: i64) -> i64 {
    // Standard cardinal-number composition for descending-magnitude
    // languages.
    //   - `жүз` (100) multiplies the < 1000 remainder of `current`
    //     (or 1 if remainder is 0); preserves any thousands
    //     accumulated already. «бес жүз» = 5*100 = 500;
    //     «бір мың тоғыз жүз» = 1000 + 9*100 = 1900.
    //   - `мың` / `миллион` multiply the entire accumulator (or 1).
    //   - Other digits add.
    if next == 100 {
        let thousands = current / 1000 * 1000;
        let units = current % 1000;
        let multiplier = if units == 0 { 1 } else { units };
        thousands + multiplier * 100
    } else if next == 1000 || next == 1_000_000 {
        let multiplier = if current == 0 { 1 } else { current };
        multiplier * next
    } else {
        current + next
    }
}

fn parse_kazakh_number_token(token: &str) -> Option<(i64, bool)> {
    // Returns (value, has_explicit_case_suffix).
    // First try the bare token (no case).
    if let Some(v) = bare_kazakh_number(token) {
        return Some((v, false));
    }
    // **v4.41.7** — digit form: «30», «100», «5» bare digits AND
    // digit + Kazakh case suffix («30-ды», «100-ге», «5-ке»).
    // Real-REPL transcript «Бес санын үшке көбейтіп, 30-ды
    // азайтыңыз» typed digit «30» with Acc «-ды»; pre-v4.41.7
    // the splitter dropped the digits entirely. Now both bare
    // digits and digit + dash + suffix forms are recognised.
    if token.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        let digits: String = token.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i64>() {
            let rest = &token[digits.len()..];
            // Strip optional leading '-' (Kazakh writes «30-ды»).
            let rest = rest.strip_prefix('-').unwrap_or(rest);
            if rest.is_empty() {
                return Some((v, false));
            }
            // Trailing chars present → treat as case-marked
            // operand. Conservative: any non-empty suffix on a
            // digit-prefix is read as case morphology even if
            // the suffix isn't a known Kazakh case form (covers
            // dialectal / colloquial inflections without an
            // exhaustive whitelist).
            return Some((v, true));
        }
    }
    // Try each case-suffix, longest first, and check that the
    // remaining stem IS a recognised bare number.
    for suff in CASE_SUFFIXES {
        if let Some(stem_len) = token.len().checked_sub(suff.len()) {
            if token.ends_with(suff) {
                let stem = &token[..stem_len];
                if let Some(v) = bare_kazakh_number(stem) {
                    return Some((v, true));
                }
            }
        }
    }
    None
}

const CASE_SUFFIXES: &[&str] = &[
    // Ordered longest-first so longest-match wins.
    "тардың",
    "тердің",
    "лардың",
    "лердің",
    "дардың",
    "дердің",
    "ынан",
    "інен",
    "ының",
    "інің",
    "ында",
    "інде",
    "тың",
    "тің",
    "дың",
    "дің",
    "ның",
    "нің",
    "пен",
    "бен",
    "ден",
    "тен",
    "нен",
    "дан",
    "тан",
    "нан",
    "ге",
    "ке",
    "ға",
    "қа",
    "не",
    "на",
    "ты",
    "ті",
    "ды",
    "ді",
    "ны",
    "ні",
    "де",
    "те",
    "да",
    "та",
];

/// **v4.41.0** — Kazakh number-to-words renderer. The inverse of
/// [`bare_kazakh_number`]: given an integer, produces its
/// canonical Kazakh number-word phrase. Used by the math-answer
/// pipeline to optionally emit «жүз елу» alongside `150` so the
/// user gets the answer in the same modality they asked
/// («Бесті отызға көбейтсем» → «Нәтижесі: 150 (жүз елу)»).
///
/// Supported range: 0 to 999_999_999 (up to «тоғыз жүз тоқсан
/// тоғыз миллион тоғыз жүз тоқсан тоғыз мың тоғыз жүз тоқсан
/// тоғыз»). Returns `None` for negative or larger numbers
/// — those continue to render as bare digits via the
/// `{math_value}` slot. Negative not implemented because Kazakh
/// math vocabulary («теріс» negation marker) is rare in
/// arithmetic-result contexts; user can read the digit directly.
pub fn render_kazakh_number_words(value: i64) -> Option<String> {
    if !(0..=999_999_999).contains(&value) {
        return None;
    }
    if value == 0 {
        return Some("нөл".to_string());
    }
    let mut parts: Vec<String> = Vec::new();
    let mut n = value;
    let millions = n / 1_000_000;
    n %= 1_000_000;
    if millions > 0 {
        if millions > 1 {
            parts.push(render_under_thousand(millions));
        } else {
            parts.push("бір".to_string());
        }
        parts.push("миллион".to_string());
    }
    let thousands = n / 1000;
    n %= 1000;
    if thousands > 0 {
        if thousands > 1 {
            parts.push(render_under_thousand(thousands));
        } else {
            parts.push("бір".to_string());
        }
        parts.push("мың".to_string());
    }
    if n > 0 {
        parts.push(render_under_thousand(n));
    }
    Some(parts.join(" "))
}

fn render_under_thousand(n: i64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let hundreds = n / 100;
    if hundreds > 0 {
        if hundreds > 1 {
            parts.push(digit_word(hundreds).to_string());
        }
        parts.push("жүз".to_string());
    }
    let tens_value = (n % 100) / 10 * 10;
    if tens_value > 0 {
        parts.push(tens_word(tens_value).to_string());
    }
    let units = n % 10;
    if units > 0 {
        parts.push(digit_word(units).to_string());
    }
    parts.join(" ")
}

fn digit_word(d: i64) -> &'static str {
    match d {
        1 => "бір",
        2 => "екі",
        3 => "үш",
        4 => "төрт",
        5 => "бес",
        6 => "алты",
        7 => "жеті",
        8 => "сегіз",
        9 => "тоғыз",
        _ => "",
    }
}

fn tens_word(t: i64) -> &'static str {
    match t {
        10 => "он",
        20 => "жиырма",
        30 => "отыз",
        40 => "қырық",
        50 => "елу",
        60 => "алпыс",
        70 => "жетпіс",
        80 => "сексен",
        90 => "тоқсан",
        _ => "",
    }
}

fn bare_kazakh_number(stem: &str) -> Option<i64> {
    match stem {
        "нөл" => Some(0),
        "бір" => Some(1),
        "екі" => Some(2),
        "үш" => Some(3),
        "төрт" => Some(4),
        "бес" => Some(5),
        "алты" => Some(6),
        "жеті" => Some(7),
        "сегіз" => Some(8),
        "тоғыз" => Some(9),
        "он" => Some(10),
        "жиырма" => Some(20),
        "отыз" => Some(30),
        "қырық" => Some(40),
        "елу" => Some(50),
        "алпыс" => Some(60),
        "жетпіс" => Some(70),
        "сексен" => Some(80),
        "тоқсан" => Some(90),
        "жүз" => Some(100),
        "мың" => Some(1000),
        "миллион" => Some(1_000_000),
        "миллиард" => Some(1_000_000_000),
        _ => None,
    }
}

/// **v4.6.20** — discourse preambles. Surface forms a Kazakh
/// speaker uses to introduce the actual question/statement that
/// follows. Pre-v4.6.20 a sentence like
/// «Айтайын дегенім, қолданыстағы жасанды интеллект модельдерінен
/// қалай жақсырақ бола аласыз?» had its first content noun
/// (`қолданыс`) grabbed by the greedy noun-hint extractor — adam
/// answered with a contract-template quote about `usage`,
/// completely missing the actual question. Stripping the preamble
/// leaves only the meaningful clause for downstream parsing.
///
/// Each entry is a lowercase preamble that, when matched at the
/// start of the input, is removed up to (and including) the next
/// clause separator (`,`, `—`, `:`, `;`). The list is checked
/// longest-first by `strip_preamble` so longer phrases like
/// `қысқаша айтқанда` always win over shorter prefixes.
const PREAMBLES: &[&str] = &[
    "айтайын дегенім",
    "айтайын дегенімді",
    "айтайын деп тұрғаным",
    "айтайын деп едім",
    "қысқаша айтқанда",
    "ашығын айтқанда",
    "шындығына келгенде",
    "сұрағым мынау",
    "сұрағым мынадай",
    "сұрақ мынадай",
    "сұрағым бар",
    "сұрағым келгені",
    "білгім келгені",
    "білгім келеді",
    "түсінсем дейтінім",
    "ойымдағы сұрақ",
    "айтпағым",
    "айтпақшы",
    "шынында",
    "шындап келгенде",
    "жалпы алғанда",
    "жалпы айтқанда",
    "иә, айтпақшы",
    "айта кетсем",
];

/// Clause-separator characters that terminate a preamble. The
/// punctuation char is consumed too so the residual starts at the
/// next non-whitespace character.
const PREAMBLE_SEPARATORS: &[char] = &[',', '—', '–', '-', ':', ';'];

/// **v4.6.20** — strip a leading discourse preamble from the
/// input, returning the residual. If the input does not start with
/// a known preamble (or has no clause separator after it), returns
/// the input unchanged. Trim preserves the user's original casing
/// of the residual; only leading whitespace is dropped.
///
/// Pure surface-level — no FST, no parsing. The preamble list is
/// closed and audited; expanding it is a v4.6.x patch.
pub fn strip_preamble(input: &str) -> &str {
    let trimmed = input.trim_start();
    let lower = trimmed.to_lowercase();
    // Sort longest-first via length comparison on the matched prefix.
    let mut best: Option<usize> = None;
    for &p in PREAMBLES {
        if lower.starts_with(p) {
            // Need a clause separator after the preamble.
            let after = &trimmed[p.len()..];
            if let Some(sep_pos) = after.find(|c: char| PREAMBLE_SEPARATORS.contains(&c)) {
                let cut = p.len() + sep_pos + after[sep_pos..].chars().next().unwrap().len_utf8();
                if best.is_none_or(|b| cut > b) {
                    best = Some(cut);
                }
            }
        }
    }
    if let Some(cut) = best {
        trimmed[cut..].trim_start()
    } else {
        input
    }
}

/// **v4.11.5** — leading vocative addressees (`адам`, `адамым`,
/// `адам-ау`, `адам ау`) declared longest-first. The match list
/// is closed: vocative use of "адам" addressing the system itself,
/// not the common-noun "адам" (= person/human) which must remain
/// available as a topic when not at clause-initial position.
const ADDRESSEES: &[&str] = &["адам-ау", "адам ау", "адамым", "адам"];

/// Punctuation that terminates a vocative form. Encompasses comma
/// (canonical), exclamation, em/en dash, hyphen, colon, semicolon.
const ADDRESSEE_SEPARATORS: &[char] = &[',', '!', '—', '–', '-', ':', ';'];

/// **v4.11.5** — strip a leading vocative addressee from the input,
/// returning the residual. Real-REPL 2026-04-30: «Адам, сен
/// мектептің физика бағдарламасын білесің бе?» — pre-v4.11.5 the
/// noun-hint extractor took the vocative `адам` itself as the topic
/// and answered with `адам IsA сүтқоректі`, completely missing the
/// actual subject of the question (`физика бағдарламасы`).
///
/// Recognises two clause shapes after the vocative:
/// 1. **punctuation-separated** — `Адам, …` / `Адам! …` / `Адам — …`
///    (any `ADDRESSEE_SEPARATORS` char). Strips the vocative AND the
///    separator.
/// 2. **bare-pronoun continuation** — `Адам сен …` / `Адам сіз …`
///    (vocative + space + 2nd-person pronoun). Strips just the
///    vocative; the pronoun stays so 1sg/2sg-self-recall layers
///    still see it.
///
/// **Disambiguation from definitional `Адам — сүтқоректі.`:** the
/// stripper fires only when the FULL input also carries an
/// addressee signal — a 2nd-person pronoun (`сен / сіз / сенің /
/// сізді / …`) or `?` / `!` punctuation. Definitional sentences
/// have neither and are passed through unchanged, preserving
/// `адам` as a legitimate common-noun topic.
///
/// Pure surface-level — no FST. Run AFTER `strip_preamble` so a
/// preamble + vocative combination collapses cleanly.
pub fn strip_addressee(input: &str) -> &str {
    if !has_addressee_signal(input) {
        return input;
    }
    // **v5.4.7** — predicative-IsA preservation guard. Pre-v5.4.7
    // «Адам — тірі ме?» got stripped to «тірі ме?» because the
    // em-dash is in `ADDRESSEE_SEPARATORS` (carry-over from forms
    // like «Адам — қаласың?» where the verb post-dash is 2sg). But
    // when the post-dash residual is a bare yes/no IsA pair
    // (`<noun-phrase> (ме|ма|ба|бе|па|пе)?` — the v5.4.0 predicative
    // pattern), `адам` is the SUBJECT of the IsA question, not a
    // vocative addressee, and the bridge chain `адам IsA тіршілік
    // иесі IsA тірі` should resolve. Differentiator from the legacy
    // test «Адам — қаласың?»: that residual ends in a 2sg verb
    // (`қаласың`), not in a yes/no IsA particle.
    if looks_like_predicative_isa_question(input) {
        return input;
    }
    let trimmed = input.trim_start();
    let lower = trimmed.to_lowercase();
    let mut best: Option<usize> = None;
    for &name in ADDRESSEES {
        if !lower.starts_with(name) {
            continue;
        }
        let after = &trimmed[name.len()..];
        let after_lower = after.to_lowercase();
        let cut: Option<usize> =
            if let Some(sep_pos) = after.find(|c: char| ADDRESSEE_SEPARATORS.contains(&c)) {
                // Cut after the punctuation separator so residual starts clean.
                let sep_char = after[sep_pos..].chars().next().unwrap();
                Some(name.len() + sep_pos + sep_char.len_utf8())
            } else if after_lower.starts_with(" сен") || after_lower.starts_with(" сіз") {
                // Bare-pronoun continuation: strip only the vocative.
                Some(name.len())
            } else {
                None
            };
        if let Some(c) = cut {
            if best.is_none_or(|b| c > b) {
                best = Some(c);
            }
        }
    }
    if let Some(cut) = best {
        trimmed[cut..].trim_start()
    } else {
        input
    }
}

/// **v5.4.7** — does the input match the bare yes/no IsA pattern
/// «<X> — <Y> (ме|ма|ба|бе|па|пе)?» introduced in v5.4.0? Used by
/// `strip_addressee` to preserve «Адам — тірі ме?» (predicative IsA)
/// from the vocative stripper. The pattern is intentionally narrow:
/// em-dash separator, the trailing question particle is one of the
/// six closed-class yes/no markers, the input ends with `?` or one
/// of those particles. A 2sg verb form post-dash (e.g. «қаласың»,
/// «білесің») does NOT match because the verb stem doesn't end in
/// the particle.
fn looks_like_predicative_isa_question(input: &str) -> bool {
    let lower = input.trim().to_lowercase();
    if !lower.contains(" — ") {
        return false;
    }
    let body = lower.trim_end_matches('?').trim();
    [" ме", " ма", " ба", " бе", " па", " пе"]
        .iter()
        .any(|tag| body.ends_with(tag))
}

/// **v4.11.5** — does the input carry any signal of being addressed
/// to adam (2nd-person reference or interrogative/exclamation
/// punctuation)? Used by `strip_addressee` to disambiguate the
/// vocative form `Адам, …` from the definitional `Адам — …`.
fn has_addressee_signal(input: &str) -> bool {
    if input.contains('?') || input.contains('!') {
        return true;
    }
    let lower = input.to_lowercase();
    // 2nd-person free pronouns + bound suffixes that are reliable
    // 2nd-person markers in surface text. Bare `сен` could be a
    // postposition in some contexts; the leading space prevents
    // matching word-internal occurrences (e.g. `мүсенжай`).
    [
        " сен",
        " сіз",
        "сенің",
        "сені ",
        "сіздің",
        "сізді",
        "сендей",
        "сіздей",
        "сізге",
        "сізден",
        " өзің",
        "өзіңіз",
        "өзіңді",
        "өзіңнің",
    ]
    .iter()
    .any(|m| lower.contains(m))
}

/// **v4.6.20** — detect a long, gracious user-acknowledgement.
/// Real-REPL: «Мен сенің әлі бәрін білмейтініңді және әлі де көп
/// жаттығу керек екенін түсіндім» — adam grabbed `әлі` and quoted
/// poetry. The input is a multi-clause statement *about* adam,
/// usually empathetic, ending in a 1sg perfective verb of
/// understanding/realisation.
///
/// Two-signal detector:
/// - Contains addressee marker `сенің / сені / сізді / сіздің`.
/// - Contains 1sg perfective realisation verb: `түсіндім / білдім
///   / көрдім / байқадым / ұқтым / аңғардым / сезіндім`.
/// Plus the sentence is not a question (no `?`, no question
/// pronoun) — questions like «Сені кім жасады?» also contain
/// `сені` but are not acknowledgements.
pub fn input_is_user_acknowledgement(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_addressee = lower.contains("сенің")
        || lower.contains("сені")
        || lower.contains("сіздің")
        || lower.contains("сізді");
    let has_realisation_verb = lower.contains("түсіндім")
        || lower.contains("білдім")
        || lower.contains("көрдім")
        || lower.contains("байқадым")
        || lower.contains("ұқтым")
        || lower.contains("аңғардым")
        || lower.contains("сезіндім");
    let is_question = lower.contains('?')
        || lower.contains("қалай")
        || lower.contains("неге")
        || lower.contains("кім")
        || lower.contains("қашан")
        || lower.contains("қайда");
    has_addressee && has_realisation_verb && !is_question
}

/// **v4.6.20** — detect "how are you better than other AI
/// models?" style questions. Routes to
/// `SystemAspect::SelfComparison`. Two-signal detector:
/// - Contains a "comparison" marker: `артық / артықсың / артықсыз
///   / жақсырақ / жақсырақсың / жақсырақсыз / озасың / озасыз /
///   айырмашылық`.
/// - Contains an addressee anchor (any of `сен / сіз / сені /
///   сізді / -сың/-сыз verb suffix already in marker).
/// The list is closed; mainstream "ерекшелік" stays under
/// Architecture (which v4.3.4 already routes correctly).
pub fn input_is_self_comparison_question(input: &str) -> bool {
    let lower = input.to_lowercase();
    // **v4.17.5** — disambiguation: «жақсырақ болу + ...» is a
    // willingness/improvement question, not a comparison. Live
    // REPL 2026-05-01 turn 20: «...жақсырақ және ақылды болуды
    // үйренуге дайынсыз ба?» — pre-v4.17.5 SelfComparison fired
    // because of `жақсырақ`. Defer to AskWillingness when growth-
    // verbs are co-present. The Intent dispatcher checks
    // `AskWillingness` BEFORE this comparison detector so this
    // guard is the belt-and-braces fallback for cases where the
    // growth-verb pattern doesn't match exactly but the
    // comparison-as-improvement reading is clearly wrong.
    if lower.contains("жақсырақ болу") || lower.contains("ақылды болу") {
        return false;
    }
    let has_comparison = lower.contains("артық")
        || lower.contains("жақсырақ")
        || lower.contains("озасың")
        || lower.contains("озасыз")
        || lower.contains("қалай үстем")
        || lower.contains("несімен бөлек")
        || lower.contains("неге сенемін")
        || lower.contains("несімен ерекше")
        || lower.contains("неге таңдау керек")
        // **v4.17.5** — distinguishing-question phrasings surfaced
        // by the 2026-05-01 live REPL transcript: «Сізді
        // қолданыстағы жасанды интеллект модельдерінен
        // ерекшелендіретін нәрсе.» pre-v4.17.5 fell through to
        // greedy retrieval and surfaced a poetry quote.
        || lower.contains("ерекшелендір")
        || lower.contains("ерекшелейт")
        || lower.contains("айырмашылығың")
        || lower.contains("айырмашылығыңыз")
        || lower.contains("айрықша қылатын")
        || lower.contains("айырық қылатын")
        || lower.contains("айырмашылықтарың")
        || lower.contains("айырмашылықтарыңыз");
    if !has_comparison {
        return false;
    }
    // Must reference adam (the addressee) — a comparison between
    // two third parties shouldn't trigger. Either a free-standing
    // 2nd-person pronoun (`сен / сіз / сені / сізді`) or a 2nd-
    // person verb ending (`-сың / -сыз` on a copula or modal,
    // including the `аласың / аласыз` ability form). The `сың/сыз`
    // suffix is itself the addressee marker even without a free-
    // standing pronoun.
    let has_pronoun = lower.contains(" сен")
        || lower.contains(" сіз")
        || lower.starts_with("сен")
        || lower.starts_with("сіз")
        || lower.contains("сені")
        || lower.contains("сізді");
    let has_addressee_suffix = lower.contains("аласың")
        || lower.contains("аласыз")
        || lower.contains("артықсың")
        || lower.contains("артықсыз")
        || lower.contains("жақсырақсың")
        || lower.contains("жақсырақсыз")
        || lower.contains("озасың")
        || lower.contains("озасыз");
    has_pronoun || has_addressee_suffix
}

/// **v4.6.20** — Adjective+noun compound noun-hint extraction.
/// Real-REPL: «Машиналық оқыту туралы айтып беріңізші» — pre-v4.6.20
/// the noun-hint extractor returned `оқыту` (the second word) and
/// dropped the modifier `машиналық`, then retrieved a generic quote
/// about education. v4.6.20 detects the `<adj> <noun>` shape via a
/// closed, audited list of compound topics seen in real REPL traces
/// and returns the joined compound as the hint.
///
/// The list is intentionally narrow — broader compound recognition
/// belongs in `MULTIWORD_ENTITIES` (semantics.rs) which is already
/// the canonical home for multi-token recognised topics. This
/// function exists for the specific case where the adj+noun form is
/// a topical compound (a *kind* of thing) rather than a named
/// entity.
const ADJ_NOUN_COMPOUND_HINTS: &[&str] = &[
    "машиналық оқыту",
    "терең оқыту",
    "жасанды интеллект",
    "табиғи тіл",
    "компьютерлік ғылым",
    "ақпараттық технология",
    "сандық технология",
    "сандық экономика",
    "жасанды нейрон",
    "жасанды нейрондық желі",
    "нейрондық желі",
];

/// **v4.6.20** — return the longest matching adj+noun compound
/// hint contained in the lowercased input, if any. Used by the
/// noun-hint extractor in `semantics.rs` to override the
/// FST-derived first-noun pick.
pub fn find_adj_noun_compound(input: &str) -> Option<&'static str> {
    let lower = input.to_lowercase();
    let mut best: Option<&'static str> = None;
    for &c in ADJ_NOUN_COMPOUND_HINTS {
        if lower.contains(c) && best.is_none_or(|b| c.len() > b.len()) {
            best = Some(c);
        }
    }
    best
}

#[cfg(test)]
mod arithmetic_tests {
    use super::try_evaluate_arithmetic;

    #[test]
    fn evaluates_pure_arithmetic() {
        assert_eq!(try_evaluate_arithmetic("5+5"), Some(10));
        assert_eq!(try_evaluate_arithmetic("7 + 3 ="), Some(10));
        assert_eq!(try_evaluate_arithmetic("6:2="), Some(3));
        assert_eq!(try_evaluate_arithmetic("100-25"), Some(75));
        assert_eq!(try_evaluate_arithmetic("12*4"), Some(48));
    }

    #[test]
    fn respects_operator_precedence() {
        // 2 + 3 * 4 = 2 + 12 = 14 (multiplication first)
        assert_eq!(try_evaluate_arithmetic("2+3*4"), Some(14));
        // 100 - 25 / 5 = 100 - 5 = 95
        assert_eq!(try_evaluate_arithmetic("100-25/5"), Some(95));
    }

    #[test]
    fn handles_division_zero_and_remainder() {
        assert_eq!(try_evaluate_arithmetic("5/0"), None);
        // **v4.50.5** — truncated integer division. Pre-v4.50.5
        // returned None for non-integer results; v4.50.5 truncates
        // to match Kazakh-conversational expectations (бесті екіге
        // бөлсем — answer is 2, not "refusal").
        assert_eq!(try_evaluate_arithmetic("7/2"), Some(3));
        assert_eq!(try_evaluate_arithmetic("10/4"), Some(2));
        assert_eq!(try_evaluate_arithmetic("5*7/2"), Some(17));
    }

    #[test]
    fn handles_unary_minus() {
        assert_eq!(try_evaluate_arithmetic("-5"), Some(-5));
        assert_eq!(try_evaluate_arithmetic("10+-3"), Some(7));
        assert_eq!(try_evaluate_arithmetic("10*-3"), Some(-30));
    }

    #[test]
    fn rejects_non_arithmetic_input() {
        assert_eq!(try_evaluate_arithmetic("5-ті 7-ге көбейткенде"), None);
        assert_eq!(try_evaluate_arithmetic("Алтыны екіге бөліңіз"), None);
        assert_eq!(try_evaluate_arithmetic("hello"), None);
        assert_eq!(try_evaluate_arithmetic(""), None);
    }
}

#[cfg(test)]
mod math_tests {
    use super::input_is_math_expression;

    #[test]
    fn detects_pure_arithmetic() {
        assert!(input_is_math_expression("5+5"));
        assert!(input_is_math_expression("7 + 3 ="));
        assert!(input_is_math_expression("6:2="));
        assert!(input_is_math_expression("12 * 4"));
    }

    #[test]
    fn detects_kazakh_math_verb_with_numerals() {
        assert!(input_is_math_expression(
            "5-ті 7-ге көбейткенде неше болады?"
        ));
        assert!(input_is_math_expression("Алтыны екіге бөліңіз"));
        assert!(input_is_math_expression("Үшке төртті қоссаңыз"));
    }

    #[test]
    fn does_not_match_non_math_kazakh() {
        assert!(!input_is_math_expression("Қазақстанда 17 облыс бар."));
        assert!(!input_is_math_expression("Менің жасым 30"));
        assert!(!input_is_math_expression("Алты қаласы Қазақстанда"));
    }

    /// **v6.0 (live REPL 2026-05-18)** — multi-clause math expressions
    /// must NOT be comma-split at the discourse layer. The math
    /// evaluator handles commas internally as clause separators and
    /// chains operations against a running accumulator; splitting
    /// them at this layer breaks the chain (clause 1 evaluates
    /// standalone, clause 2 has no left operand).
    #[test]
    fn split_compound_keeps_math_one_piece_v6() {
        let cases = [
            "Елу алтыны үшке көбейтіңіз, содан кейін екіге бөліңіз",
            "Жиырма бесті бес көбейтсек, содан кейін онға бөл",
            "Бесті отызға көбейт, сосын екіге бөл",
            "56 * 3 / 2",
        ];
        for c in cases {
            let pieces = super::split_compound_utterance(c);
            assert_eq!(
                pieces.len(),
                1,
                "math input must not be split (input={c:?}, got pieces={pieces:?})"
            );
        }
    }

    /// Counter-test: non-math compound inputs continue to split as
    /// before so the per-clause dispatch path stays intact for normal
    /// dialog turns.
    #[test]
    fn split_compound_still_splits_non_math_v6() {
        let pieces = super::split_compound_utterance("Сәлем, менің атым Дәулет");
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0], "Сәлем");
        assert_eq!(pieces[1], "менің атым Дәулет");
    }

    /// **v6.0** — colloquial «сосын» («then») recognised as a clause
    /// separator alongside formal «содан кейін» for spoken-style math
    /// chains.
    #[test]
    fn word_math_sosyn_separator_v6() {
        // 5 × 30 = 150, then ÷ 2 = 75.
        assert_eq!(
            super::try_evaluate_kazakh_word_math("Бесті отызға көбейт, сосын екіге бөл"),
            Some(75)
        );
    }

    /// **v6.0** — the user's exact phrasing from the 2026-05-18 voice
    /// REPL that motivated the multi-clause fix: «Елу алтыны үшке
    /// көбейтіңіз, содан кейін екіге бөліңіз» = «56 × 3 then ÷ 2».
    /// Should compute to 84.
    #[test]
    fn word_math_user_56x3_div2_v6() {
        assert_eq!(
            super::try_evaluate_kazakh_word_math(
                "Елу алтыны үшке көбейтіңіз, содан кейін екіге бөліңіз"
            ),
            Some(84)
        );
    }

    /// **v4.41.0** — word-form math evaluator tests.
    use super::try_evaluate_kazakh_word_math;

    #[test]
    fn word_math_basic_ops() {
        // 5 × 30 = 150
        assert_eq!(
            try_evaluate_kazakh_word_math("Бесті отызға көбейтсем"),
            Some(150)
        );
        // 100 / 10 = 10
        assert_eq!(try_evaluate_kazakh_word_math("Жүзді онға бөл"), Some(10));
        // 6 / 2 = 3
        assert_eq!(
            try_evaluate_kazakh_word_math("Алтыны екіге бөліңіз"),
            Some(3)
        );
        // 100 - 50 = 50
        assert_eq!(try_evaluate_kazakh_word_math("Жүзден елуді ал"), Some(50));
        // 5 + 3 = 8
        assert_eq!(try_evaluate_kazakh_word_math("Бес пен үш қос"), Some(8));
    }

    #[test]
    fn word_math_compound_numbers() {
        // 25 × 5 = 125 (compound «жиырма бес»)
        assert_eq!(
            try_evaluate_kazakh_word_math("Жиырма бесті бес көбейтсек"),
            Some(125)
        );
        // 100 + 50 = 150 (compound «жүз елу»)
        assert_eq!(
            try_evaluate_kazakh_word_math("Жүз елуді бір мыңға қос"),
            Some(1150)
        );
        // 1999 + 230 = 2229 (compound «бір мың тоғыз жүз тоқсан тоғыз»)
        assert_eq!(
            try_evaluate_kazakh_word_math("Бір мың тоғыз жүз тоқсан тоғызға қос екі жүз отыз"),
            Some(2229)
        );
    }

    #[test]
    fn word_math_inflected_imperatives() {
        // Imperative-form variants
        assert_eq!(
            try_evaluate_kazakh_word_math("Бесті отызға көбейтіңіз"),
            Some(150)
        );
        assert_eq!(
            try_evaluate_kazakh_word_math("Жүзді бесті бөлсем"),
            Some(20)
        );
    }

    #[test]
    fn word_math_rejects_non_math() {
        // No math verb
        assert_eq!(try_evaluate_kazakh_word_math("Бес отыз қанша"), None);
        // Single operand only
        assert_eq!(try_evaluate_kazakh_word_math("Бесті көбейтсек"), None);
        // Take-imperative without numerals doesn't trigger
        assert_eq!(try_evaluate_kazakh_word_math("Кітапты ал"), None);
    }

    #[test]
    fn word_math_division_truncates_non_integer() {
        // **v4.50.5** — truncated integer division. 7/2 → 3 (was None
        // pre-v4.50.5). Closer to user expectations on Kazakh
        // conversational math.
        assert_eq!(try_evaluate_kazakh_word_math("Жетіні екіге бөл"), Some(3));
        // 5 / 0 → None (division by zero stays invalid).
        assert_eq!(try_evaluate_kazakh_word_math("Бесті нөлге бөл"), None);
    }

    /// **v4.53.0** — gerund/converb-form math chains. Real-REPL
    /// session 5 surfaced «Елуді екіге **көбейткенде** үшке **бөліп**,
    /// 7-ні **азайтқанда** не болады?» — the same chain that worked
    /// in imperative form failed because the gerund/converb forms
    /// lacked overt clause boundaries.
    #[test]
    fn word_math_gerund_chain_session5_first_line() {
        // 50 × 2 / 3 - 7 = 100 / 3 - 7 = 33 - 7 = 26
        // (truncated integer division on intermediate, per v4.50.5).
        assert_eq!(
            try_evaluate_kazakh_word_math(
                "Елуді екіге көбейткенде үшке бөліп, 7-ні азайтқанда не болады?"
            ),
            Some(26)
        );
    }

    #[test]
    fn word_math_converb_only_chain() {
        // Pure converb chain (no gerund): «Бесті екіге көбейтіп, үшті
        // қосыңыз» — should split on «көбейтіп» and chain.
        // 5 × 2 + 3 = 13.
        assert_eq!(
            try_evaluate_kazakh_word_math("Бесті екіге көбейтіп, үшті қосыңыз"),
            Some(13)
        );
    }

    #[test]
    fn word_math_gerund_only_chain() {
        // Pure gerund chain. «Алтыны екіге бөлгенде үшке көбейтіңіз» —
        // 6 / 2 × 3 = 9.
        assert_eq!(
            try_evaluate_kazakh_word_math("Алтыны екіге бөлгенде үшке көбейтіңіз"),
            Some(9)
        );
    }

    #[test]
    fn word_math_qosqanda_qosyp_forms() {
        // «қосқанда» (add-when) gerund. «Беске үшті қосқанда нәтиже
        // қандай?» — 5 + 3 = 8.
        assert_eq!(
            try_evaluate_kazakh_word_math("Беске үшті қосқанда нәтиже қандай?"),
            Some(8)
        );
        // «қосып» (add-converb) sequel. «Бесті екіге көбейтіп, үшті
        // қосып, төртті азайтыңыз» — 5×2 + 3 - 4 = 9.
        assert_eq!(
            try_evaluate_kazakh_word_math("Бесті екіге көбейтіп, үшті қосып, төртті азайтыңыз"),
            Some(9)
        );
    }

    #[test]
    fn word_math_gerund_does_not_break_simple_imperative() {
        // Regression: imperative-only chains (the v4.42.0 path)
        // should still parse correctly with gerund-separator
        // injection in place — no spurious clause splits.
        assert_eq!(
            try_evaluate_kazakh_word_math(
                "Елуден екіге көбейтіңіз, үшке бөліңіз және 7-ні азайтыңыз"
            ),
            Some(26)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_standalone_discourse_anaphors() {
        for input in [
            "Ал онда қанша аймақ бар?",
            "Сонда не бар?",
            "Осында тұрамын",
            "Мұнда барлығы жақсы",
            "Бұнда да солай",
            "Одан кейін не?",
            "Содан соң айт",
            "Бұдан шықпайды",
            "Осыдан бастаймыз",
        ] {
            assert!(
                input_contains_discourse_anaphor(input),
                "input {input:?} must register as carrying a discourse anaphor"
            );
        }
    }

    #[test]
    fn does_not_match_unrelated_words() {
        for input in ["Қазақстан туралы не білесіз?", "Алматы — қала", "Сәлем"]
        {
            assert!(
                !input_contains_discourse_anaphor(input),
                "input {input:?} must NOT register as discourse anaphor"
            );
        }
    }

    #[test]
    fn handles_punctuation_around_anaphor() {
        assert!(input_contains_discourse_anaphor("Ал онда?"));
        assert!(input_contains_discourse_anaphor("онда, иә"));
        assert!(input_contains_discourse_anaphor("онда."));
    }

    #[test]
    fn strips_leading_addressee_with_comma() {
        // Real-REPL 2026-04-30: vocative `Адам,` masked the actual
        // topic «физика бағдарламасы» behind retrieval of
        // `адам IsA сүтқоректі`.
        assert_eq!(
            strip_addressee("Адам, сен мектептің физика бағдарламасын білесің бе?"),
            "сен мектептің физика бағдарламасын білесің бе?"
        );
        assert_eq!(strip_addressee("Адам! Қалайсың?"), "Қалайсың?");
        assert_eq!(strip_addressee("Адам — қаласың?"), "қаласың?");
    }

    #[test]
    fn strips_addressee_variants_longest_first() {
        assert_eq!(strip_addressee("Адам-ау, не білесің?"), "не білесің?");
        assert_eq!(
            strip_addressee("Адамым, өзің туралы айтшы"),
            "өзің туралы айтшы"
        );
    }

    #[test]
    fn strips_addressee_before_bare_pronoun() {
        // No punctuation between vocative and `сен/сіз` — strip just
        // the vocative; the pronoun stays so 1sg-self-recall still
        // sees it.
        assert_eq!(
            strip_addressee("Адам сен мектеп пәндерін білесің бе?"),
            "сен мектеп пәндерін білесің бе?"
        );
        assert_eq!(
            strip_addressee("Адам сіз қандай тілде жазылғансыз?"),
            "сіз қандай тілде жазылғансыз?"
        );
    }

    #[test]
    fn preserves_input_when_addressee_not_leading() {
        // Bare common-noun "адам" at non-initial position is a
        // legitimate topic and must NOT be stripped.
        assert_eq!(strip_addressee("Адам — сүтқоректі."), "Адам — сүтқоректі.");
        assert_eq!(
            strip_addressee("Қазақстандағы адам туралы не білесіз?"),
            "Қазақстандағы адам туралы не білесіз?"
        );
        assert_eq!(strip_addressee("Сәлем"), "Сәлем");
    }

    #[test]
    fn preserves_predicative_isa_question_v547() {
        // **v5.4.7** — bare yes/no IsA queries «<X> — <Y> (ме|ма|...)?»
        // must not be parsed as vocatives even when X is `Адам`. The
        // chain query in conversation::find_isa_chain should resolve
        // адам → тіршілік иесі → тірі. Pre-v5.4.7 these were stripped
        // and routed to a clarifier.
        assert_eq!(strip_addressee("Адам — тірі ме?"), "Адам — тірі ме?");
        assert_eq!(
            strip_addressee("Адам — тіршілік иесі ме?"),
            "Адам — тіршілік иесі ме?"
        );
        // Sanity: legacy 2sg-verb vocative still strips because the
        // residual doesn't end in a yes/no IsA particle.
        assert_eq!(strip_addressee("Адам — қаласың?"), "қаласың?");
    }
}

#[cfg(test)]
mod math_summary_tests_v5210 {
    use super::*;

    #[test]
    fn extracts_multi_step_kazakh_word_math_v5210() {
        // User's live-test failure: 56 × 4 / 2.
        let summary = extract_kazakh_math_summary("Елу алты төртке көбейтіп екіге бөл.").unwrap();
        assert_eq!(summary.numbers, vec![56, 4, 2]);
        assert_eq!(
            summary.operators,
            vec![KazakhMathOpName::Mul, KazakhMathOpName::Div]
        );
        assert!(summary.looks_like_math);
    }

    #[test]
    fn renders_arithmetic_echo_v5210() {
        let summary = KazakhMathSummary {
            numbers: vec![56, 7, 3],
            operators: vec![KazakhMathOpName::Mul, KazakhMathOpName::Div],
            looks_like_math: true,
        };
        let rendered = render_math_summary_as_arithmetic(&summary).unwrap();
        assert_eq!(rendered, "56 * 7 / 3");
    }

    #[test]
    fn no_summary_for_non_math_input_v5210() {
        // Plain dialog turn, no numbers / operators / quantity question.
        assert!(extract_kazakh_math_summary("Сәлем! Қалыңыз қалай?").is_none());
    }

    /// **v6.0 (live REPL 2026-05-18)** — geometric / measurement
    /// questions pattern-match on numeral + sum-verb + «қанша» but
    /// must not produce a math summary. Pre-fix the live REPL
    /// surfaced «Мен «3» деп ұқтым…» on the triangle-angle question,
    /// echoing back a number the user never asked about.
    #[test]
    fn geometry_questions_yield_no_math_summary_v6() {
        let cases = [
            "Үш бұрыштың бұрыштарының қосындысы қанша градус?",
            "Шеңбердің ауданы қалай есептеледі?",
            "Алматы мен Астана арасы қанша километр?",
            "Кубтың көлемі қалай табылады?",
            "Үшбұрыштың периметрі дегеніміз не?",
        ];
        for c in cases {
            assert!(
                extract_kazakh_math_summary(c).is_none(),
                "geometry/measurement input must not produce math summary: {c:?}"
            );
            assert!(
                try_evaluate_kazakh_word_math(c).is_none(),
                "geometry/measurement input must not evaluate to a number: {c:?}"
            );
        }
    }

    /// Counter-test: pure arithmetic and Kazakh word-math remain
    /// untouched by the geometry guard. False positives here would
    /// silently break the calculator UX.
    #[test]
    fn arithmetic_and_word_math_unaffected_by_geometry_guard_v6() {
        // Pure arithmetic — must keep evaluating.
        assert!(extract_kazakh_math_summary("2 + 2 қанша?").is_some());
        assert!(extract_kazakh_math_summary("12 + 35 қанша?").is_some());
        // Kazakh word-math — must keep evaluating to a number.
        assert_eq!(
            try_evaluate_kazakh_word_math("Бесті отызға көбейтсем"),
            Some(150)
        );
        assert_eq!(try_evaluate_kazakh_word_math("Бес пен үш қос"), Some(8));
    }

    #[test]
    fn extracts_arabic_digits_with_word_op_v5210() {
        let summary = extract_kazakh_math_summary("56 төртке көбейтіп екіге бөл").unwrap();
        // 56 explicit digit + 4 + 2; ops Mul, Div.
        assert!(summary.numbers.contains(&56));
        assert!(summary.numbers.contains(&4));
        assert!(summary.numbers.contains(&2));
        assert!(summary.operators.contains(&KazakhMathOpName::Mul));
        assert!(summary.operators.contains(&KazakhMathOpName::Div));
    }

    #[test]
    fn tens_units_merge_v5210() {
        // «елу алты» = 56, not 50 + 6 as separate numbers.
        let summary = extract_kazakh_math_summary("Елу алты қанша?").unwrap();
        assert_eq!(summary.numbers, vec![56]);
    }

    #[test]
    fn standalone_tens_kept_when_followed_by_op_v5210() {
        // «Елу үшке көбейт» = 50 × 3 (no units follow «елу»; «үш» is
        // the next number not part of «елу алты» fusion).
        let summary = extract_kazakh_math_summary("Елу үшке көбейт").unwrap();
        assert_eq!(summary.numbers, vec![50, 3]);
        assert_eq!(summary.operators, vec![KazakhMathOpName::Mul]);
    }
}

#[cfg(test)]
mod safe_echo_tests_v5215 {
    use super::safe_echo_input;

    #[test]
    fn accepts_typical_kazakh_short_input_v5215() {
        assert_eq!(
            safe_echo_input("Несте аласын").as_deref(),
            Some("Несте аласын")
        );
        assert_eq!(
            safe_echo_input("Сен қазақша түсінесің бе"),
            Some("Сен қазақша түсінесің бе".to_string())
        );
    }

    #[test]
    fn trims_whitespace_v5215() {
        assert_eq!(
            safe_echo_input("   Несте аласын   ").as_deref(),
            Some("Несте аласын")
        );
    }

    #[test]
    fn rejects_too_short_v5215() {
        assert!(safe_echo_input("а").is_none());
        assert!(safe_echo_input("Ах").is_none());
        assert!(safe_echo_input("").is_none());
    }

    #[test]
    fn rejects_too_long_v5215() {
        let long = "А".repeat(100);
        assert!(safe_echo_input(&long).is_none());
    }

    #[test]
    fn rejects_digits_v5215() {
        assert!(safe_echo_input("Менің атым Дәулет 25").is_none());
        assert!(safe_echo_input("123 қанша").is_none());
    }

    #[test]
    fn rejects_urls_and_emails_v5215() {
        assert!(safe_echo_input("Сайтың https://example.com").is_none());
        assert!(safe_echo_input("Mail: foo@bar.com").is_none());
        assert!(safe_echo_input("Адресі test://abc").is_none());
    }

    #[test]
    fn rejects_code_markers_v5215() {
        assert!(safe_echo_input("Сұрағым `let x = 5`").is_none());
        assert!(safe_echo_input("Қандай {slot} керек").is_none());
    }

    #[test]
    fn rejects_non_kazakh_input_v5215() {
        // Latin-heavy text. Routes to language guard, not echo.
        assert!(safe_echo_input("Hello how are you").is_none());
        assert!(safe_echo_input("Como estas amigo").is_none());
    }

    #[test]
    fn accepts_mostly_cyrillic_with_some_latin_v5215() {
        // «Сен Rust туралы білесің бе?» — code-switching but
        // dominated by Cyrillic. OK to echo.
        assert!(
            safe_echo_input("Сен Rust туралы білесің бе").is_some(),
            "code-switched query with Cyrillic majority should echo"
        );
    }

    #[test]
    fn rejects_punctuation_only_v5215() {
        assert!(safe_echo_input("???").is_none());
        assert!(safe_echo_input("!!!!").is_none());
        assert!(safe_echo_input("---").is_none());
    }
}
