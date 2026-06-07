// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # IFS — Internal Family Systems parts-work state machine
//!
//! ## Background
//!
//! Internal Family Systems (Richard Schwartz, 1980s+) frames the
//! psyche as a collection of "parts" — sub-personalities that
//! each carry a function.  A part that lashes out in anger
//! might be PROTECTING a younger, more wounded part that
//! holds shame.  The therapeutic move is not to suppress the
//! protector — it is to thank it, ask its role, witness what it
//! protects, and offer the wounded part the user's own *Self*
//! presence ("Self-energy" — the user's calm centre that exists
//! beneath the parts).
//!
//! This crate implements a six-stage scripted IFS conversation
//! tailored for Kazakh.  Stages 1–6 below.
//!
//! ## Six stages
//!
//! 1. **EmotionCheckIn** — surface the strongest current
//!    emotion.  User names it (anger / shame / fear / grief).
//! 2. **IdentifyPart** — re-frame the emotion as *a part of you*,
//!    not *you yourself*.  Linguistic shift from «мен ашуланғанмын»
//!    to «менің бір бөлігім ашулы».
//! 3. **AskRole** — what is this part protecting you from?
//!    Most strong emotions in IFS are protectors guarding
//!    something more vulnerable.
//! 4. **WitnessPain** — invite the user to witness what the
//!    part itself feels.  Often this surfaces an exile (a
//!    younger wounded part the protector shields).
//! 5. **Unblending** — make room between the user's Self and
//!    the part.  Breath, body, naming Self as separate from
//!    part.  Essential before integration.
//! 6. **Integration** — what does the user (from Self) want to
//!    say to the part?  Often gratitude, recognition, a promise.
//!    Then a gentle closing.
//!
//! ## State
//!
//! Sessions carry [`WellnessSession`] across turns.  The session
//! tracks the current stage, the named focal emotion (used to
//! refer back across turns), and how many turns have been spent
//! at the current stage.  No persistence to disk — sessions are
//! ephemeral per voice-REPL session.
//!
//! ## Safety
//!
//! Even inside an active IFS session, every user utterance is
//! re-checked against [`red_flags::detect`] before the IFS state
//! machine runs.  A red flag mid-session HARD-ESCALATES — the
//! IFS session is paused, escalation reply emitted, and the
//! session resumes only if the user explicitly returns.

use crate::red_flags;
use serde::{Deserialize, Serialize};

/// The six IFS stages, in canonical order.  Sessions advance
/// through them; the `step` function decides advance vs. stay vs.
/// soft re-probe based on the user's reply pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WellnessStage {
    EmotionCheckIn,
    IdentifyPart,
    AskRole,
    WitnessPain,
    Unblending,
    Integration,
    /// Terminal state — the user has completed the cycle (or
    /// gracefully aborted).  Sessions in `Closed` do not produce
    /// more IFS replies; the orchestration layer can start a new
    /// session if the user asks for one.
    Closed,
}

/// Ephemeral per-conversation state.  Carries the active stage,
/// the focal emotion the user named (if any), and the turn
/// counter at the current stage (used to detect getting stuck
/// vs. genuine deepening).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WellnessSession {
    /// Current stage.  Initialised at `EmotionCheckIn` when a
    /// session begins; advances through the cycle.
    pub stage: Option<WellnessStage>,
    /// The emotion the user named at stage 1, referred back to
    /// in later stages as «{emotion} бөлігі».  `None` until the
    /// user surfaces something nameable.
    pub focal_emotion: Option<String>,
    /// How many user turns have already been spent at the
    /// current stage.  Used to decide soft-probe vs. advance.
    pub turns_at_stage: u32,
}

impl WellnessSession {
    /// Start a new IFS session at `EmotionCheckIn`.
    pub fn start() -> Self {
        Self {
            stage: Some(WellnessStage::EmotionCheckIn),
            focal_emotion: None,
            turns_at_stage: 0,
        }
    }

    /// Whether the session is currently active (in any non-`Closed`
    /// stage).  When `false`, the orchestration layer should not
    /// call [`step`] until a new session is started.
    pub fn is_active(&self) -> bool {
        matches!(
            self.stage,
            Some(
                WellnessStage::EmotionCheckIn
                    | WellnessStage::IdentifyPart
                    | WellnessStage::AskRole
                    | WellnessStage::WitnessPain
                    | WellnessStage::Unblending
                    | WellnessStage::Integration
            )
        )
    }
}

/// One reply from the wellness layer to the conversation
/// orchestration.  The reply text is the verbatim Kazakh string
/// that adam should send (TTS-friendly: no markdown, no
/// English).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WellnessReply {
    pub text: String,
    /// What the orchestration should do after emitting `text`.
    pub action: ReplyAction,
}

/// Side-effect the orchestration must apply after speaking the
/// reply.  The session itself is mutated in-place by [`step`];
/// `ReplyAction` carries the *kind* of move so callers can log,
/// branch, or close the loop cleanly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyAction {
    /// Continue the IFS session — the next user utterance will
    /// flow back through [`step`].
    Continue,
    /// Red flag fired.  Session is paused (stage cleared).  The
    /// orchestration MUST emit `text` verbatim and SHOULD prompt
    /// for explicit re-opt-in before any further IFS step.
    Escalate(red_flags::RedFlag),
    /// User gracefully closed the session (asked to stop,
    /// completed integration).  No further IFS replies until a
    /// new session starts.
    Close,
}

/// Main entry: given the user's utterance and the current
/// session state, advance the state machine and return adam's
/// reply.
///
/// Contract:
/// 1. `red_flags::detect` runs first.  Any flag short-circuits
///    to an escalation reply and clears the session.
/// 2. If the user signals abort («тоқтайық», «жетеді», «басқа
///    тақырыпқа көшейік»), the session closes gracefully.
/// 3. Otherwise, advance the stage based on the user's reply
///    pattern.  At stage 1 the named emotion is extracted into
///    `session.focal_emotion`.  At later stages templates
///    reference the focal emotion by name.
///
/// The function is deterministic — same input + same session
/// state always returns the same reply.  Template variety comes
/// from `turns_at_stage` indexing into a fixed array, not RNG.
pub fn step(input: &str, session: &mut WellnessSession) -> WellnessReply {
    // ── 1. Defence-in-depth red-flag check ──
    if let Some(flag) = red_flags::detect(input) {
        session.stage = None;
        session.focal_emotion = None;
        session.turns_at_stage = 0;
        return WellnessReply {
            text: red_flags::escalation_template(flag).to_string(),
            action: ReplyAction::Escalate(flag),
        };
    }

    // ── 2. Graceful abort ──
    if matches_any_lower(input, ABORT_PHRASES) {
        session.stage = Some(WellnessStage::Closed);
        return WellnessReply {
            text: CLOSING_GRACEFUL.to_string(),
            action: ReplyAction::Close,
        };
    }

    // ── 3. Stage-driven response ──
    let current_stage = session.stage.unwrap_or(WellnessStage::EmotionCheckIn);
    let reply_text = match current_stage {
        WellnessStage::EmotionCheckIn => {
            // First turn: open with check-in.  If the user
            // already named an emotion on stage entry, extract
            // it and advance immediately.
            if let Some(named) = extract_emotion(input) {
                session.focal_emotion = Some(named);
                session.stage = Some(WellnessStage::IdentifyPart);
                session.turns_at_stage = 0;
                identify_part_template(session.focal_emotion.as_deref().unwrap())
            } else {
                // Stay at check-in with a softer probe variant.
                session.turns_at_stage = session.turns_at_stage.saturating_add(1);
                CHECKIN_TEMPLATES
                    [(session.turns_at_stage as usize).min(CHECKIN_TEMPLATES.len() - 1)]
                .to_string()
            }
        }
        WellnessStage::IdentifyPart => {
            session.stage = Some(WellnessStage::AskRole);
            session.turns_at_stage = 0;
            ask_role_template(session.focal_emotion.as_deref())
        }
        WellnessStage::AskRole => {
            session.stage = Some(WellnessStage::WitnessPain);
            session.turns_at_stage = 0;
            witness_pain_template(session.focal_emotion.as_deref())
        }
        WellnessStage::WitnessPain => {
            session.stage = Some(WellnessStage::Unblending);
            session.turns_at_stage = 0;
            UNBLENDING_TEMPLATE.to_string()
        }
        WellnessStage::Unblending => {
            session.stage = Some(WellnessStage::Integration);
            session.turns_at_stage = 0;
            integration_template(session.focal_emotion.as_deref())
        }
        WellnessStage::Integration => {
            // Final substantive turn.  Move to Closed after this.
            session.stage = Some(WellnessStage::Closed);
            session.turns_at_stage = 0;
            CLOSING_INTEGRATED.to_string()
        }
        WellnessStage::Closed => {
            // Session already closed — orchestrator shouldn't be
            // calling step, but be graceful.
            CLOSING_GRACEFUL.to_string()
        }
    };

    let action = if matches!(session.stage, Some(WellnessStage::Closed) | None) {
        ReplyAction::Close
    } else {
        ReplyAction::Continue
    };
    WellnessReply {
        text: reply_text,
        action,
    }
}

/// Extract the named emotion from a check-in reply, returning
/// the canonical Kazakh emotion name if recognised.  When the
/// user uses a Russian-language emotion word, map it to its
/// Kazakh equivalent so subsequent templates address the part
/// in Kazakh.
///
/// Returns `None` when no recognised emotion appears — the
/// caller stays at the check-in stage with a softer probe.
fn extract_emotion(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    for &(needle, canonical) in EMOTION_LEXICON {
        if lower.contains(needle) {
            return Some(canonical.to_string());
        }
    }
    None
}

fn matches_any_lower(input: &str, needles: &[&str]) -> bool {
    let lower = input.to_lowercase();
    needles.iter().any(|n| lower.contains(n))
}

// ── Template tables ──
//
// All user-facing strings.  Edits here are user-visible UX
// changes — handle with care.

/// Variant openings for the EmotionCheckIn stage.  Indexed by
/// `turns_at_stage` so a user who can't yet name a feeling gets
/// progressively softer probes.
const CHECKIN_TEMPLATES: &[&str] = &[
    "Қазір ішіңізде қандай сезім ең күшті? Жайғасып отырып, бір сәт байқап көріңіз.",
    "Бір тыныс алайық. Кеудеңізде, иығыңызда, не қарныңызда қандай сезім тұр?",
    "Атау табу қажет емес. Жай ғана: бұл сезім ауыр ма, бос па, ыстық па, суық па?",
];

/// Stage 2 template — re-frame the named emotion as a part.
/// The user's named emotion is interpolated verbatim.
fn identify_part_template(emotion: &str) -> String {
    format!(
        "Сол {emotion} — сіздің бүкіл болмысыңыз емес. Бұл — сіздің ішіңіздегі бір бөлік. \
         Бұл {emotion} бөлігі қаншадан бері сізбен бірге жүр сияқты?"
    )
}

/// Stage 3 template — ask the part its protective role.
fn ask_role_template(emotion: Option<&str>) -> String {
    match emotion {
        Some(e) => format!(
            "Бұл {e} бөлігі сізді неден қорғағысы келеді? Кейде күшті сезімдер бізді \
             басқа, нәзік нәрседен қорғаушы рөлде жүреді."
        ),
        None => "Бұл сезім сізді неден қорғағысы келеді сияқты? Кейде күшті сезімдер \
             бізді нәзік нәрседен қорғаушы рөлде жүреді."
            .to_string(),
    }
}

/// Stage 4 template — witness what the part itself feels.
fn witness_pain_template(emotion: Option<&str>) -> String {
    match emotion {
        Some(e) => format!(
            "Бір сәт сол {e} бөлігінің өзіне көз салайық. Ол өзі не сезінеді — жалғыздық \
             па, шаршағандық па, көрінбегендік пе? Не айтар еді, сөйлей алса?"
        ),
        None => "Бір сәт сол бөліктің өзіне көз салайық. Ол өзі не сезінеді — жалғыздық па, \
             шаршағандық па, көрінбегендік пе?"
            .to_string(),
    }
}

/// Stage 5 — unblending.  The Self-energy move: making space
/// between Self and part.  No emotion-name interpolation; the
/// breath-and-presence framing is universal.
const UNBLENDING_TEMPLATE: &str = "Енді бір терең тыныс алайық. Сіз — сол бөлік емессіз. \
     Сіз — сол бөлікті көріп, оған құрмет көрсете алатын Сіз. \
     Ішіңізде сол сабырлы орынды сезе аласыз ба?";

/// Stage 6 template — integration.  What does Self say to part?
fn integration_template(emotion: Option<&str>) -> String {
    match emotion {
        Some(e) => format!(
            "Осы сабырлы орыннан тұрып, сол {e} бөлігіне не айтқыңыз келеді? Алғыс па, \
             түсінушілік пе, әлде «сені көрдім» деген қарапайым сөз бе?"
        ),
        None => "Осы сабырлы орыннан тұрып, сол бөлікке не айтқыңыз келеді?".to_string(),
    }
}

/// Final closing after a completed cycle.
const CLOSING_INTEGRATED: &str = "Бұл жұмыс — бір күнде бітпейді. Әр оралған сайын бөлікке құрметпен қараңыз. \
     Қажет болса — кез келген уақытта қайтып ораламыз. Қазірге сау болыңыз.";

/// Graceful abort closing (user asked to stop mid-cycle).
const CLOSING_GRACEFUL: &str =
    "Жақсы, әрі қарай жүрмейміз. Өзіңізге уақыт беріңіз. Қайтып келгіңіз келсе — мен осы жердемін.";

/// Phrases that close the session at any stage.
const ABORT_PHRASES: &[&str] = &[
    "тоқтайық",
    "жетеді",
    "болды",
    "тоқтат",
    "басқа тақырыпқа",
    "басқа тақырып",
    "хватит",
    "давай о другом",
    "не хочу обсуждать",
];

/// Recognised emotion names (KZ + RU code-switch) mapped to
/// their canonical Kazakh form for downstream templates.
///
/// Curated tight — only emotions where IFS parts-work is the
/// appropriate response.  Excludes neutral words like «шаршау»
/// (tiredness alone is not the IFS focal frame).
const EMOTION_LEXICON: &[(&str, &str)] = &[
    // Anger
    ("ашу", "ашу"),
    ("ашуланғ", "ашу"),
    ("ашуланд", "ашу"),
    ("ызалы", "ыза"),
    ("ыза", "ыза"),
    ("гнев", "ашу"),
    ("злость", "ашу"),
    ("раздраж", "ашу"),
    // Fear / anxiety
    ("қорқыныш", "қорқыныш"),
    ("қорқам", "қорқыныш"),
    ("қорқып", "қорқыныш"),
    ("алаңда", "алаңдау"),
    ("үрей", "үрей"),
    ("страх", "қорқыныш"),
    ("боюсь", "қорқыныш"),
    ("тревог", "алаңдау"),
    // Shame / guilt
    ("ұят", "ұят"),
    ("ұялам", "ұят"),
    ("кінә", "кінә"),
    ("кінәлі", "кінә"),
    ("стыдно", "ұят"),
    ("вина", "кінә"),
    // Sadness / grief
    ("қайғы", "қайғы"),
    ("мұң", "мұң"),
    ("жабырқа", "мұң"),
    ("көңілсіз", "мұң"),
    ("грусть", "мұң"),
    ("печаль", "мұң"),
    ("тоска", "мұң"),
    // Grievance / resentment
    ("реніш", "реніш"),
    ("ренжіп", "реніш"),
    ("ренжіді", "реніш"),
    ("обида", "реніш"),
    ("обижен", "реніш"),
    // Loneliness
    ("жалғызд", "жалғыздық"),
    ("одинок", "жалғыздық"),
    // Hatred
    ("жек көр", "жек көру"),
    ("ненавиж", "жек көру"),
    ("ненавист", "жек көру"),
];

#[cfg(test)]
mod tests {
    use super::*;

    // ── Happy path: 6-stage cycle ──

    #[test]
    fn full_cycle_advances_through_all_six_stages() {
        let mut s = WellnessSession::start();
        assert_eq!(s.stage, Some(WellnessStage::EmotionCheckIn));

        // Turn 1: user names anger → advance to IdentifyPart
        let r = step("Менің әкеме ашуым көп.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::IdentifyPart));
        assert_eq!(s.focal_emotion.as_deref(), Some("ашу"));
        assert!(r.text.contains("ашу"), "got: {}", r.text);
        assert_eq!(r.action, ReplyAction::Continue);

        // Turn 2: AskRole
        let r = step("Бала кезімнен бері.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskRole));
        assert!(r.text.contains("ашу"), "got: {}", r.text);

        // Turn 3: WitnessPain
        let r = step("Кінәлі сезінбес үшін.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::WitnessPain));
        assert!(r.text.contains("ашу"), "got: {}", r.text);

        // Turn 4: Unblending
        let r = step("Ол қатты жалғыз.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Unblending));
        assert!(r.text.contains("тыныс"), "got: {}", r.text);

        // Turn 5: Integration
        let _ = step("Иә, сезіп тұрмын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Integration));

        // Turn 6: Closing
        let r = step("Алғыс айтайын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Closed));
        assert_eq!(r.action, ReplyAction::Close);
        assert!(!s.is_active());
    }

    // ── Stage 1 stays when no emotion named ──

    #[test]
    fn checkin_stays_when_no_emotion_named() {
        let mut s = WellnessSession::start();
        let r = step(
            "Білмеймін, қандай сезімде екенімді анықтай алмадым.",
            &mut s,
        );
        // Stays at EmotionCheckIn with a softer probe variant.
        assert_eq!(s.stage, Some(WellnessStage::EmotionCheckIn));
        assert_eq!(r.action, ReplyAction::Continue);
        assert!(s.focal_emotion.is_none());
        // Second probe should be a different (softer) template.
        let r2 = step("Әлі түсінбеймін.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::EmotionCheckIn));
        assert_ne!(r.text, r2.text, "softer probe should differ");
    }

    // ── Russian code-switch on emotion naming ──

    #[test]
    fn russian_emotion_maps_to_kazakh_canonical() {
        let mut s = WellnessSession::start();
        step("У меня сильная обида на маму.", &mut s);
        assert_eq!(s.focal_emotion.as_deref(), Some("реніш"));
    }

    #[test]
    fn russian_fear_maps_to_kazakh() {
        let mut s = WellnessSession::start();
        step("Я боюсь будущего.", &mut s);
        assert_eq!(s.focal_emotion.as_deref(), Some("қорқыныш"));
    }

    // ── Red flag inside an active session ──

    #[test]
    fn red_flag_mid_session_escalates_and_clears() {
        let mut s = WellnessSession::start();
        step("Әкеме қатты ашуланамын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::IdentifyPart));

        // Mid-session: user surfaces suicidal ideation.
        let r = step("Бірақ қазір өмір сүргім келмейді.", &mut s);
        assert!(matches!(
            r.action,
            ReplyAction::Escalate(red_flags::RedFlag::SuicidalIdeation)
        ));
        assert!(s.stage.is_none(), "session must be cleared on escalation");
        assert!(r.text.contains("150"));
    }

    // ── Abort ──

    #[test]
    fn abort_phrase_closes_gracefully() {
        let mut s = WellnessSession::start();
        step("Реніш бар.", &mut s);
        let r = step("Болды, тоқтайық.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Closed));
        assert_eq!(r.action, ReplyAction::Close);
    }

    #[test]
    fn russian_abort_phrase_works() {
        let mut s = WellnessSession::start();
        step("Ашу бар.", &mut s);
        let _ = step("Хватит, давай о другом.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Closed));
    }

    // ── Session lifecycle ──

    #[test]
    fn closed_session_is_not_active() {
        let s = WellnessSession {
            stage: Some(WellnessStage::Closed),
            ..Default::default()
        };
        assert!(!s.is_active());
    }

    #[test]
    fn fresh_session_is_active() {
        let s = WellnessSession::start();
        assert!(s.is_active());
    }

    // ── Templates ──

    #[test]
    fn unblending_template_carries_breath_and_self_framing() {
        // The core IFS move at stage 5 is breath + Self-as-witness.
        // Verify both anchors are present so the template can't
        // drift into something that loses the unblending intent.
        assert!(UNBLENDING_TEMPLATE.contains("тыныс"));
        assert!(UNBLENDING_TEMPLATE.contains("Сіз — сол бөлік емессіз"));
    }

    #[test]
    fn integration_template_includes_focal_emotion_when_known() {
        let s = integration_template(Some("реніш"));
        assert!(s.contains("реніш"));
    }

    #[test]
    fn integration_template_falls_back_gracefully_without_emotion() {
        let s = integration_template(None);
        assert!(!s.is_empty());
        assert!(s.contains("бөлікке"));
    }
}
