// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # Red-flag detection — the FIRST layer of every wellness turn
//!
//! ## Purpose
//!
//! Detect utterances that signal the user is in (or near) a
//! crisis state where structured parts-work / reflection is
//! the wrong response.  In those cases adam **does not** open
//! IFS dialog — adam emits a scripted referral to a Kazakhstan
//! crisis line.
//!
//! ## Design philosophy
//!
//! 1. **False positives are acceptable, false negatives are not.**
//!    Extra escalation costs a slightly awkward reply.  A missed
//!    crisis costs lives.  When in doubt, escalate.
//! 2. **Detector is purely substring-based.**  No neural fallback,
//!    no fuzzy edit-distance.  Adversarial robustness comes from
//!    a curated phrase list audited per release.
//! 3. **Both Kazakh and Russian phrasings.**  The target user is
//!    Kazakh-dominant but may code-switch under emotional load.
//!    Missing the Russian variant of «хочу умереть» when the user
//!    is in crisis would be inexcusable.
//! 4. **Scripted reply is fixed; no template variables, no
//!    «бәлкім, X туралы» style hedging.**  The reply names the
//!    risk plainly and gives concrete numbers.
//!
//! ## Crisis lines surfaced
//!
//! These were the published Kazakhstan numbers as of 2026-06-04.
//! Reviewed per release; if they change, update the templates in
//! [`escalation_template`].
//!
//! - **150** — республикалық балалар мен жасөспірімдер сенім
//!   телефоны (children + youth)
//! - **112** — біріктірілген экстремалдық қызмет
//! - **103** — жедел медициналық жәрдем (medical emergency)
//! - **102** — полиция (for active domestic violence)
//!
//! ## What is NOT a red flag
//!
//! Sadness, grief, anger, despair, "everything is bad", chronic
//! pain complaints, ruminative anxiety — these are exactly what
//! IFS parts-work IS for.  Red-flag detection looks specifically
//! for *imminence* signals (action intent, immediate physical
//! danger, acute medical symptoms with plausible time pressure).

use serde::{Deserialize, Serialize};

/// Categories of crisis that require scripted escalation instead
/// of IFS parts-work dialog.
///
/// Priority order from highest to lowest:
///   1. `SuicidalIdeation` — overrides everything else
///   2. `AcuteMedicalSymptom` — time-critical
///   3. `ChildAbuse` — mandatory reporting analogue
///   4. `DomesticViolenceImmediate` — police-side response
///   5. `Psychosis` — psychiatric referral
///
/// When multiple flags match, the highest-priority one wins via
/// the order in `detect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RedFlag {
    /// User signals intent or thought of ending their life, or
    /// asks for help dying.  Includes both first-person («хочу
    /// умереть») and method-mention shapes («таблеткалар көп
    /// ішкім келеді»).
    SuicidalIdeation,
    /// Acute medical symptom that needs ambulance, not
    /// reflection: chest pain, difficulty breathing, stroke
    /// signs, severe bleeding, overdose.
    AcuteMedicalSymptom,
    /// Disclosure of child abuse — either the user IS a child
    /// being abused or describes ongoing abuse of another child.
    ChildAbuse,
    /// Domestic violence with current physical danger — the
    /// utterance frames threat or attack as happening now / today.
    DomesticViolenceImmediate,
    /// Severe psychotic symptoms (command hallucinations, plan
    /// to act on paranoid belief).  Less common, narrowly gated.
    Psychosis,
}

/// Detect the highest-priority crisis flag in the user's
/// utterance.  Returns `None` when the input is safe to route
/// to IFS dialog.
///
/// Detection is case-insensitive on ASCII letters and applies
/// the standard Kazakh-Russian lowercase mapping (Cyrillic + the
/// Kazakh-specific characters `ә ғ қ ң ө ұ ү һ і`).  Punctuation
/// is not stripped because some signal phrases hinge on the
/// exclamation or question mark.
///
/// The detector tries categories in priority order — the first
/// match wins.  Within a category the phrases are OR-joined.
pub fn detect(input: &str) -> Option<RedFlag> {
    let lower = input.to_lowercase();

    if matches_any(&lower, SUICIDAL_PHRASES) || matches_suicidal_phonetic_anchors(&lower) {
        return Some(RedFlag::SuicidalIdeation);
    }
    if matches_any(&lower, ACUTE_MEDICAL_PHRASES) {
        return Some(RedFlag::AcuteMedicalSymptom);
    }
    if matches_any(&lower, CHILD_ABUSE_PHRASES) {
        return Some(RedFlag::ChildAbuse);
    }
    if matches_any(&lower, DV_IMMEDIATE_PHRASES) {
        return Some(RedFlag::DomesticViolenceImmediate);
    }
    if matches_any(&lower, PSYCHOSIS_PHRASES) {
        return Some(RedFlag::Psychosis);
    }
    None
}

/// **rc4 (2026-06-04 live-audit safety fix).** Catches STT-mangled
/// variants of «өмір сүргім келмейді» (the canonical Kazakh
/// "I don't want to live" phrase).  Live audit caught Whisper
/// transcribing this as «Ө мөрі сүргім келмейді» / «Өмор сүргім
/// келмейді» / «Өмірі сүргім келмейді» / «Өмер сүргім келмейді» /
/// «Өмір сүргін келмейді» — none of which matched the literal
/// substring in `SUICIDAL_PHRASES`, so adam continued the IFS
/// dialog through nine consecutive crisis statements.
///
/// We detect by triple anchor — three short stems that survive
/// most STT noise on this phrase.  We match against the BARE
/// CONCATENATED letter sequence (whitespace + punctuation
/// stripped), not against split tokens, because Whisper often
/// inserts spurious spaces into «өмір» — splitting it to «ө мір»
/// or «ө мөрі», which would otherwise hide the «өм» anchor.
///
///   - «өм» or «ом» (life-root)
///   - «сүрг» (live-verb stem)
///   - «келм» (negation stem)
///
/// All three must appear; order doesn't matter.  False-positive
/// risk: discussing someone else's death wish.  We accept that
/// risk — falsely escalating to a hotline is recoverable;
/// missing a real crisis isn't.
fn matches_suicidal_phonetic_anchors(lower: &str) -> bool {
    let concat: String = lower.chars().filter(|c| c.is_alphabetic()).collect();
    let life_root = concat.contains("өм") || concat.contains("ом");
    let live_verb = concat.contains("сүрг");
    let negation = concat.contains("келм");
    life_root && live_verb && negation
}

/// Return the scripted Kazakh-language reply that adam must
/// emit when a red flag is detected.
///
/// Reply contract:
///   1. Name the concern explicitly — do not euphemise.
///   2. Provide at least one concrete phone number.
///   3. Affirm the user's worth in one short sentence.
///   4. Do NOT ask «why do you feel this way» / «расскажите
///      подробнее» — that is parts-work, and parts-work is the
///      wrong layer for an imminent crisis.
pub fn escalation_template(flag: RedFlag) -> &'static str {
    match flag {
        RedFlag::SuicidalIdeation => {
            "Сіздің айтқаныңыз мені алаңдатады. Бұл сезіммен жалғыз қалмаңыз. \
             Қазір тікелей қоңырау шалыңыз: 150 (балалар мен жасөспірімдерге сенім телефоны) \
             немесе 112 (бірыңғай экстренді қызмет). Сіз маңыздысыз және көмек алуға \
             лайықсыз. Маман адаммен сөйлесу маңызды — мен бұл рөлді алмаймын."
        }
        RedFlag::AcuteMedicalSymptom => {
            "Сіз сипаттаған белгілер шұғыл медициналық көмекті қажет етеді. \
             Қазір 103-ке немесе 112-ге қоңырау шалыңыз. Бұл мен шеше алатын \
             нәрсе емес — дереу дәрігерге хабарласыңыз."
        }
        RedFlag::ChildAbuse => {
            "Сіз айтып отырған жағдай балаға қауіпті. Қазір 150-ге (балалар сенім \
             телефоны) немесе 102-ге (полиция) хабарласу маңызды. Балаға қауіп \
             төніп тұрса — 112. Бұл жағдайды құпия сақтамауыңыз керек."
        }
        RedFlag::DomesticViolenceImmediate => {
            "Сіз қазір қауіпсіз орында болуыңыз керек. Егер шабуыл болып жатса \
             немесе қауіп жақын болса — 102 (полиция) немесе 112-ге шалыңыз. \
             Мен сізге IFS әдісімен «отырып талдау» ұсына алмаймын — алдымен \
             қауіпсіздік."
        }
        RedFlag::Psychosis => {
            "Сіз сипаттаған сезімдер психиатриялық маманның көмегін қажет етеді. \
             103-ке немесе жақын психоневрологиялық диспансерге жүгініңіз. Бұл \
             менің құзыретімнен тыс — медициналық маман нақты бағалай алады."
        }
    }
}

fn matches_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

// ── Phrase tables ──
//
// Curated per audit.  Each entry is a substring that, if
// present in the lowercase user input, triggers the flag.
// Kept tight enough that benign mentions don't fire (e.g.
// «ұйқым келеді» = "I want to sleep" is not a suicide flag),
// but broad enough to catch Whisper-mangled variants.
//
// Adding entries: prefer SHORT distinctive substrings over
// long literal sentences.  Long sentences miss real-user
// paraphrases; short substrings ride the wave of variations.
// Removing entries: only after a confirmed false-positive
// from a real session — do not pre-emptively trim.

const SUICIDAL_PHRASES: &[&str] = &[
    // Kazakh
    "өзімді өлтір", // «өзімді өлтіремін / өлтіргім келеді»
    "өлгім келеді",
    "өмірден кеткім келеді",
    "өмір сүргім келмейді",
    "жанымды қи", // «жанымды қияйын / қиямын»
    "өмірім бітсе екен",
    "өмірім бітсін",
    "тіршілікті аяқта", // «тіршілікті аяқтаймын»
    // Russian (code-switch under load)
    "хочу умереть",
    "хочу покончить",
    "убью себя",
    "не хочу жить",
    "нет смысла жить",
    "покончить с собой",
    "свести счеты с жизнью",
    "лучше умереть",
];

const ACUTE_MEDICAL_PHRASES: &[&str] = &[
    // Kazakh — chest pain, breathing, stroke, severe bleeding,
    // overdose, sudden severe symptoms with imminence framing.
    "кеудем қатты ауыр",
    "жүрегім қысып тұр",
    "дем ала алмай",
    "тыныс ала алмай",
    "есімнен танып",
    "есімнен айырыл",
    "қан тоқтамай",
    "инсульт",
    "инфаркт",
    "артық дәрі іштім",
    "артық таблетка",
    // Russian
    "острая боль в груди",
    "не могу дышать",
    "потерял сознание",
    "теряю сознание",
    "сильное кровотечение",
    "передозировка",
    "много таблеток",
    "слишком много таблеток",
    "выпил много", // catches «выпил много таблеток»
    "приняла много",
];

const CHILD_ABUSE_PHRASES: &[&str] = &[
    // Kazakh — both first-person disclosure («әкем мені ұрады»)
    // and third-person reports («көршінің баласын ұрып жатыр»).
    "баланы ұр", // «баланы ұрып», «баланы ұрады»
    "баланы зорла",
    "баланы қина",
    "балаға қол көтер",
    "әкем мені ұр",
    "анам мені ұр",
    "үйде мені ұр", // child speaking
    // Russian
    "бьет ребенка",
    "избивает ребенка",
    "насилует ребенка",
    "издевается над ребенком",
    "папа меня бьет",
    "мама меня бьет",
];

const DV_IMMEDIATE_PHRASES: &[&str] = &[
    // Kazakh — focus on imminence («қазір», «бүгін», «осы
    // жерде»).  Non-imminent past trauma («бұрын күйеуім ұратын»)
    // is exactly what IFS dialog is for, so we don't escalate it.
    "қазір маған шабуыл",
    "бүгін мені ұр",
    "күйеуім қазір",
    "үйде қазір қауіп",
    "өлтіремін деп қорқыт",
    // Russian
    "сейчас на меня нападает",
    "он сейчас меня бьет",
    "она сейчас меня бьет",
    "угрожает убить",
    "грозится убить",
];

const PSYCHOSIS_PHRASES: &[&str] = &[
    // Narrowly gated — command hallucinations with action,
    // paranoid delusion with imminent action plan.  We do NOT
    // flag bare «маған дауыс естіледі» without action framing,
    // because dissociation-from-grief overlaps the surface form.
    "маған дауыстар бұйыр",
    "дауыс маған бұйыр",
    "дауыстар маған өлтір",
    "командалар бер",
    "голоса приказывают",
    "голоса велят убить",
    "следят за мной и хотят",
];

#[cfg(test)]
mod tests {
    use super::*;

    // ── SuicidalIdeation ──

    #[test]
    fn detects_kazakh_suicidal_first_person() {
        assert_eq!(
            detect("Өзімді өлтіргім келеді."),
            Some(RedFlag::SuicidalIdeation)
        );
        assert_eq!(
            detect("Менің өмір сүргім келмейді."),
            Some(RedFlag::SuicidalIdeation)
        );
    }

    /// **rc4 regression — live-audit STT variants.**  Whisper
    /// distorts «өмір сүргім келмейді» in many ways under noise;
    /// each variant below was observed in the rc3 live audit and
    /// every single one MUST escalate.
    #[test]
    fn detects_stt_noise_variants_of_canonical_suicidal_phrase() {
        for variant in [
            // Audit transcript variants:
            "Ө мөрі сүргім келмейді.",
            "Өмор сүргім келмейді.",
            "Өмор сүргің келмейді.",
            "Өмірі сүргім келмейді.",
            "Өмер сүргім келмейді.",
            "Өмір сүргін келмейді.",
            // Plausible adjacent variants (compound rage + crisis):
            "Сен ақымақсың, түсінбейсің, өмір сүргін келмейді.",
            // Split-word edge: Whisper sometimes inserts spaces.
            "Ө мір сүргім келмейді.",
        ] {
            assert_eq!(
                detect(variant),
                Some(RedFlag::SuicidalIdeation),
                "must escalate STT variant: {variant}"
            );
        }
    }

    #[test]
    fn detects_russian_suicidal_first_person() {
        assert_eq!(
            detect("Я хочу умереть, не могу больше."),
            Some(RedFlag::SuicidalIdeation)
        );
        assert_eq!(
            detect("Лучше умереть чем так жить."),
            Some(RedFlag::SuicidalIdeation)
        );
    }

    #[test]
    fn benign_sleep_phrase_does_not_trigger_suicidal() {
        // «ұйқым келеді» = "I want to sleep" — NOT suicidal.
        assert!(detect("Ұйқым келеді, шаршадым.").is_none());
    }

    // ── AcuteMedicalSymptom ──

    #[test]
    fn detects_chest_pain_kazakh() {
        assert_eq!(
            detect("Кеудем қатты ауырып тұр."),
            Some(RedFlag::AcuteMedicalSymptom)
        );
    }

    #[test]
    fn detects_overdose_phrase() {
        assert_eq!(
            detect("Артық дәрі іштім, өзімді жаман сезіп тұрмын."),
            Some(RedFlag::AcuteMedicalSymptom)
        );
        assert_eq!(
            detect("Я выпил много таблеток."),
            Some(RedFlag::AcuteMedicalSymptom)
        );
    }

    // ── ChildAbuse ──

    #[test]
    fn detects_child_abuse_disclosure() {
        assert_eq!(
            detect("Әкем мені ұрады күн сайын."),
            Some(RedFlag::ChildAbuse)
        );
        assert_eq!(detect("Папа меня бьет."), Some(RedFlag::ChildAbuse));
    }

    // ── DomesticViolenceImmediate ──

    #[test]
    fn detects_dv_immediate_kazakh() {
        assert_eq!(
            detect("Күйеуім қазір қайтып келеді, маған шабуыл жасайды."),
            Some(RedFlag::DomesticViolenceImmediate)
        );
    }

    #[test]
    fn past_abuse_without_imminence_is_not_escalated() {
        // Past trauma narrative — that's IFS territory, not crisis.
        assert!(detect("Бұрын күйеуім мені ұрып жүретін, бірақ ажырастық.").is_none());
    }

    // ── Psychosis (narrow) ──

    #[test]
    fn detects_command_hallucinations_with_action() {
        assert_eq!(
            detect("Маған дауыстар бұйырады өлтіріңіз деп."),
            Some(RedFlag::Psychosis)
        );
        assert_eq!(detect("Голоса приказывают мне."), Some(RedFlag::Psychosis));
    }

    #[test]
    fn grief_dissociation_phrase_does_not_misfire() {
        // Dissociative grief language can sound similar but
        // shouldn't escalate to psychiatric crisis.
        assert!(detect("Анам қайтыс болғаннан кейін өзімді жоғалтып алдым.").is_none());
    }

    // ── Negative controls — IFS-domain emotion talk ──

    #[test]
    fn anger_is_not_a_red_flag() {
        // Anger is exactly what IFS parts-work IS for.
        assert!(detect("Әкеме қатты ашуланамын.").is_none());
        assert!(detect("Я ненавижу свою работу.").is_none());
    }

    #[test]
    fn sadness_is_not_a_red_flag() {
        assert!(detect("Күн сайын көңілсіз ояндым.").is_none());
        assert!(detect("Мне грустно последние месяцы.").is_none());
    }

    #[test]
    fn fear_is_not_a_red_flag() {
        assert!(detect("Жұмыстан қорқам, қатты алаңдаймын.").is_none());
    }

    // ── Priority ordering ──

    #[test]
    fn suicidal_wins_over_other_flags_when_co_present() {
        // Co-occurring chest pain + suicidal ideation — suicidal
        // takes priority (mental crisis intervention first;
        // medical professionals on scene also handle the physical
        // symptom).
        assert_eq!(
            detect("Кеудем ауырады және өмір сүргім келмейді."),
            Some(RedFlag::SuicidalIdeation)
        );
    }

    // ── Escalation templates ──

    #[test]
    fn escalation_template_contains_concrete_phone_number() {
        for flag in [
            RedFlag::SuicidalIdeation,
            RedFlag::AcuteMedicalSymptom,
            RedFlag::ChildAbuse,
            RedFlag::DomesticViolenceImmediate,
            RedFlag::Psychosis,
        ] {
            let tpl = escalation_template(flag);
            // At least one concrete number (150 / 102 / 103 / 112).
            let has_number = tpl.contains("150")
                || tpl.contains("102")
                || tpl.contains("103")
                || tpl.contains("112");
            assert!(
                has_number,
                "escalation template for {flag:?} must contain a phone number"
            );
        }
    }

    #[test]
    fn escalation_template_does_not_open_parts_work() {
        // Parts-work invitations like «бұл туралы сөйлесейік»
        // or «расскажите подробнее» are explicitly the wrong
        // response under crisis.  Verify they don't appear.
        for flag in [
            RedFlag::SuicidalIdeation,
            RedFlag::AcuteMedicalSymptom,
            RedFlag::ChildAbuse,
        ] {
            let tpl = escalation_template(flag);
            assert!(
                !tpl.contains("сөйлесейік"),
                "{flag:?} template invited dialog"
            );
            assert!(
                !tpl.contains("расскажите"),
                "{flag:?} template invited dialog"
            );
        }
    }
}
