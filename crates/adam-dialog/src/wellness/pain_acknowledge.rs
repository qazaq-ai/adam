// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `wellness::pain_acknowledge` — **v6.8.10 (2026-06-23) — soft
//! tier between `red_flags` and the general cascade**.
//!
//! Surfaced by the 37-turn voice REPL audit on 2026-06-23.  Turn
//! 35 had the user say «Менің белім ауырад, көмекте аласың ба?»
//! («My back hurts, can you help?»).  The cascade routed to
//! `AskAboutTopic` (capability question — because of «көмекте
//! аласың ба?») and answered with a 24-second «I'm just a
//! language model» description.  That is the wrong tier for
//! pain — it's neither generic capability talk nor an acute
//! crisis.
//!
//! ## Tier architecture
//!
//! 1. `red_flags::detect` — ACUTE crises (chest pain, can't
//!    breathe, bleeding, suicidal ideation).  Escalates to
//!    emergency numbers.  Always fires first.
//! 2. `safety_guard::check` — generic harm (drugs, weapons,
//!    harm-to-others).  Refuses with a safety template.
//! 3. **`pain_acknowledge::detect` (this module)** — non-acute
//!    body pain («белім ауырады», «басым ауыр», «тіс ауырады»).
//!    Returns a fixed acknowledgement that:
//!       * names what the user said ("X ауырғанын естідім");
//!       * states what adam is NOT (a doctor; no diagnosis, no
//!         treatment);
//!       * gives the standard non-emergency next step (see a
//!         GP, or 103 if it gets severe);
//!       * asks an open follow-up so the user isn't shut down.
//! 4. Cascade — everything else.
//!
//! The module is intentionally `pub(crate)` deterministic;
//! the response is a fixed template, not generated.  Adding
//! variability is a future ergonomics concern; the floor here
//! is "user with pain never gets the capability-description
//! template".
//!
//! ## What this module does NOT cover
//!
//! - **Chest pain / can't breathe** — that's `red_flags`
//!   territory and escalates to 103 immediately.
//! - **Suicidal / self-harm thoughts** — same, `red_flags`.
//! - **Pain support session (interactive breathing exercise)** —
//!   that's `pain_support` and is opt-in via `ADAM_PAIN_SUPPORT=1`.
//!   This module fires for the much more common case of just
//!   acknowledging the pain without entering a guided session.

/// Body-part roots whose «<root>(-ім/-і/-ы/-...) + ауыр-stem»
/// shape should trigger the pain acknowledgement.  Acute zones
/// (`кеуде` / `жүрек` / `өкпе` / `тыныс`) are intentionally
/// absent — those belong to `red_flags`.
///
/// Each root is a substring match against the lower-cased input,
/// so possessive (`белім`), genitive (`белдің`), nominative
/// (`бел`), and locative (`белде`) forms all hit the same root.
const PAIN_BODY_PART_ROOTS: &[&str] = &[
    "бел",      // lower back — the T35 case
    "арқа",     // upper back
    "мойын",    // neck
    "иық",      // shoulder
    "аяқ",      // leg / foot
    "тізе",     // knee
    "табан",    // sole / foot
    "қол",      // arm / hand
    "білек",    // forearm
    "саусақ",   // finger
    "бас",      // head
    "маңдай",   // forehead
    "құлақ",    // ear
    "тіс",      // tooth
    "тамақ",    // throat
    "көз",      // eye
    "іш",       // stomach
    "бауыр",    // liver / side
    "бел тұс",  // multi-word — lower back area
    "бұлшықет", // muscle
    "буын",     // joint
    "омыртқа",  // spine
];

/// Verb-stem patterns covering Kazakh pain-verb morphology
/// («ауырады», «ауырып тұр», «ауырыпты», «ауырад» colloquial
/// dropped-final-ы).
const PAIN_VERB_PATTERNS: &[&str] = &[
    "ауырад",     // catches ауырады / ауырад (T35 dropped final)
    "ауырып",     // ауырып тұр / ауырыпты
    "ауырғ",      // ауырған (past) / ауырғаны
    "ауырса",     // conditional
    "ауыр болып", // "started hurting"
];

/// Detect a non-acute body-pain pattern.  Returns the fixed
/// acknowledgement template when the input mentions a body part
/// from `PAIN_BODY_PART_ROOTS` alongside a pain-verb pattern
/// from `PAIN_VERB_PATTERNS`.  Returns `None` otherwise.
///
/// Both lookups are substring-based against the lowercased input;
/// they require the body part AND the verb to co-occur in the
/// same utterance.  A bare «ауырады» alone (no body part) does
/// NOT fire — the cascade's regular handlers cover the abstract
/// case, while THIS module's purpose is to handle the «my X
/// hurts» surface specifically.
pub fn detect(raw_input: &str) -> Option<&'static str> {
    let lower = raw_input.to_lowercase();
    let has_body_part = PAIN_BODY_PART_ROOTS.iter().any(|root| lower.contains(root));
    if !has_body_part {
        return None;
    }
    let has_pain_verb = PAIN_VERB_PATTERNS.iter().any(|p| lower.contains(p));
    if !has_pain_verb {
        return None;
    }
    Some(ACK_TEMPLATE)
}

/// Fixed acknowledgement template.  Deterministic — no slot
/// substitution — so the safety contract holds regardless of
/// what the user typed.  Names the limit (not a doctor), gives
/// the next step (GP / 103 if severe), and ends with an open
/// follow-up so the user isn't conversationally dead-ended.
const ACK_TEMPLATE: &str = "Ауырғаныңыз туралы естідім. Мен дәрігер емеспін — \
диагноз қоюға да, ем ұсынуға да құзыретім жоқ. Егер ауырсыну күшті болса \
немесе бірнеше күнге созылса — дәрігерге қаралыңыз. Шұғыл жағдайда «103» \
санына қоңырау шалыңыз. Жалпы жағдайыңыз қалай?";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t35_back_pain_with_help_request_triggers() {
        // The exact voice REPL turn 35 input that prompted this
        // module.  Pre-fix: cascade routed to AskAboutTopic and
        // emitted a 24-second capability description.
        let r = detect("Менің белім ауырад көмекте аласың ба?");
        assert!(r.is_some(), "back pain + help request must trigger");
        assert!(r.unwrap().contains("дәрігер емеспін"));
    }

    #[test]
    fn common_body_pain_variants_trigger() {
        assert!(detect("Басым қатты ауырады.").is_some());
        assert!(detect("Тісім ауырып тұр.").is_some());
        assert!(detect("Тізем ауырғанда не істеу керек?").is_some());
        assert!(detect("Көзім ауырады, неге?").is_some());
        assert!(detect("Ішім ауырып жатыр.").is_some());
    }

    /// Body part WITHOUT pain verb → no trigger (it's a different
    /// kind of query — anatomy, medical history, etc. — the
    /// cascade handles those).
    #[test]
    fn body_part_alone_no_trigger() {
        assert!(detect("Бел қалай жұмыс істейді?").is_none());
        assert!(detect("Бас миы туралы айт.").is_none());
    }

    /// Pain verb WITHOUT body part → no trigger (abstract pain
    /// or different topic).
    #[test]
    fn pain_verb_alone_no_trigger() {
        assert!(detect("Жаным ауырад.").is_none()); // metaphoric
        assert!(detect("Ауырып жатыр.").is_none()); // missing body part
    }

    /// Acute markers (chest, can't breathe) must NOT be caught
    /// here — those are `red_flags` territory and have their
    /// own escalation template with emergency numbers.  This
    /// module's body-part list intentionally excludes them.
    #[test]
    fn acute_markers_not_in_body_part_list() {
        // Direct check — if these appeared in PAIN_BODY_PART_ROOTS,
        // they'd hijack red_flags's chest-pain escalation.
        assert!(!PAIN_BODY_PART_ROOTS.contains(&"кеуде"));
        assert!(!PAIN_BODY_PART_ROOTS.contains(&"жүрек"));
        assert!(!PAIN_BODY_PART_ROOTS.contains(&"өкпе"));
        assert!(!PAIN_BODY_PART_ROOTS.contains(&"тыныс"));
    }

    /// Negative controls: unrelated greetings, factual queries,
    /// math.  None of these contain BOTH a body-part root AND a
    /// pain verb.
    #[test]
    fn negative_controls() {
        assert!(detect("Сәлеметсіз бе.").is_none());
        assert!(detect("Қазақстанның астанасы қай қала?").is_none());
        assert!(detect("Жетіге бесті қос.").is_none());
        assert!(detect("СИЗ беру тәртібі қандай?").is_none());
    }
}
