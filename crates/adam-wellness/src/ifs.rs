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

/// IFS stages.  Sessions advance through them; the `step` function
/// decides advance vs. stay vs. soft re-probe based on the user's
/// reply pattern.
///
/// **rc3 (2026-06-04 wellness audit round 2):** replaced
/// `OpeningConsent` with a three-step intake (`AskingName` →
/// `AskingAge` → `AskingProblem`).  Live audit said «он не должен
/// начинать говорить первым … начинать должен с приветствия, узнать
/// имя, спросить возраст и проблему».  adam now does proper
/// conversational intake before parts-work.  Gender comes from the
/// voice-REPL F0 pitch hint, not asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WellnessStage {
    /// rc3 — first adam turn.  Greet the user, introduce adam,
    /// ask the user's name.  Advances to `AskingAge` when a name
    /// is extracted; retries with softer probe up to 2 times then
    /// skips ahead with no recorded name.
    AskingName,
    /// rc3 — second intake turn.  Acknowledge name, ask age.
    /// Advances to `AskingProblem` on extracted age.
    AskingAge,
    /// rc3 — third intake turn.  Ask what brought the user.
    /// Stores the raw problem statement in the session.  If an
    /// emotion is named anywhere in the reply, jumps straight to
    /// `IdentifyPart`; otherwise transitions to `EmotionCheckIn`.
    AskingProblem,
    EmotionCheckIn,
    IdentifyPart,
    AskRole,
    WitnessPain,
    Unblending,
    Integration,
    /// **rc5** — emitted after a red-flag escalation cleared
    /// the prior session.  Holds intake info (name / age / gender)
    /// but refuses to run parts-work prompts.  The user must
    /// explicitly signal they want to continue («дайынмын»,
    /// «жалғастырайық», «бастайық») before any new IFS turn.
    /// Any further red-flag re-emits the escalation template.
    PostEscalation,
    /// Terminal state — the user has completed the cycle (or
    /// gracefully aborted).  Sessions in `Closed` do not produce
    /// more IFS replies; the orchestration layer can start a new
    /// session if the user asks for one.
    Closed,
}

/// Gender hint passed in by the voice REPL based on F0 pitch
/// analysis (see `adam-voice`).  Used by adam-wellness to pick
/// the right Kazakh honorific (Ағай / Апай / Балам / Сіз).
/// **adam does NOT ask gender** — it is inferred from the voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GenderHint {
    Male,
    Female,
    Child,
}

/// Ephemeral per-conversation state.  Carries the active stage,
/// intake fields (name / age / gender hint / problem statement),
/// the focal emotion, and the turn counter at the current stage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WellnessSession {
    /// Current stage.  Initialised at `AskingName` when a
    /// session begins; advances through the cycle.
    pub stage: Option<WellnessStage>,

    /// rc3 — name the user gave (verbatim, title-cased).  `None`
    /// until extracted.  Templates address the user by name once
    /// known.
    pub user_name: Option<String>,
    /// rc3 — age the user reported.  `None` until extracted.
    /// Used together with `gender_hint` to pick honorific.
    pub user_age: Option<u32>,
    /// rc3 — voice-derived gender hint.  Passed in by the voice
    /// REPL via `set_gender_hint`; adam-wellness does NOT ask.
    pub gender_hint: Option<GenderHint>,
    /// rc3 — raw problem statement the user gave at
    /// `AskingProblem`.  Kept for context; not interpolated into
    /// templates verbatim.
    pub problem_statement: Option<String>,

    /// The emotion the user named at check-in, referred back to
    /// in later stages as «{emotion} бөлігі».  `None` until the
    /// user surfaces something nameable.
    pub focal_emotion: Option<String>,
    /// How many user turns have already been spent at the
    /// current stage.  Used to decide soft-probe vs. advance.
    pub turns_at_stage: u32,
}

impl WellnessSession {
    /// Start a new IFS session at the conversational intake.
    pub fn start() -> Self {
        Self {
            stage: Some(WellnessStage::AskingName),
            ..Default::default()
        }
    }

    /// **rc5 — resume after an Escalate or graceful Close.**
    ///
    /// Live audit feedback: «adam doesn't remember name and age
    /// across the dialog from beginning to end».  Cause: after
    /// the crisis-clearance step zeroed the session, the REPL
    /// auto-restarted with `WellnessSession::start()` — losing
    /// name, age, gender, and the previous escalation history.
    ///
    /// `resume_after_clearance` preserves identifying info
    /// (name / age / gender hint) from the prior session while
    /// resetting the IFS-state machinery (stage, focal emotion,
    /// turn counter, problem statement).  The new stage is
    /// `PostEscalation` if `was_escalation` is true — which
    /// blocks fresh parts-work prompts until the user explicitly
    /// signals they want to continue.  Otherwise the new stage
    /// is `AskingName` when no name is known, else it skips
    /// straight to `AskingProblem` since intake is already done.
    pub fn resume_after_clearance(prior: &WellnessSession, was_escalation: bool) -> Self {
        let stage = if was_escalation {
            WellnessStage::PostEscalation
        } else if prior.user_name.is_some() && prior.user_age.is_some() {
            WellnessStage::AskingProblem
        } else if prior.user_name.is_some() {
            WellnessStage::AskingAge
        } else {
            WellnessStage::AskingName
        };
        Self {
            stage: Some(stage),
            user_name: prior.user_name.clone(),
            user_age: prior.user_age,
            gender_hint: prior.gender_hint,
            problem_statement: None,
            focal_emotion: None,
            turns_at_stage: 0,
        }
    }

    /// Set the gender hint from voice-pitch analysis.  Called by
    /// the voice REPL after F0 settles (typically by turn 2).
    /// Idempotent — overwrites whatever was previously stored.
    pub fn set_gender_hint(&mut self, hint: GenderHint) {
        self.gender_hint = Some(hint);
    }

    /// Whether the session is currently active (in any non-`Closed`
    /// stage).  When `false`, the orchestration layer should not
    /// call [`step`] until a new session is started.
    pub fn is_active(&self) -> bool {
        matches!(
            self.stage,
            Some(
                WellnessStage::AskingName
                    | WellnessStage::AskingAge
                    | WellnessStage::AskingProblem
                    | WellnessStage::EmotionCheckIn
                    | WellnessStage::IdentifyPart
                    | WellnessStage::AskRole
                    | WellnessStage::WitnessPain
                    | WellnessStage::Unblending
                    | WellnessStage::Integration
                    | WellnessStage::PostEscalation
            )
        )
    }

    /// rc3 — Kazakh honorific addressing.  Combines name + voice
    /// gender hint + (optional) age into a polite address form.
    ///
    /// - Female adult → «Апай Гүлмира»
    /// - Male adult → «Ағай Дәулет»
    /// - Child (gender=Child OR age<18) → «Балам Айгуль»
    /// - Unknown gender → «Сіз, Дәулет» (no honorific, polite Сіз)
    /// - No name at all → «Сіз»
    pub fn honorific_address(&self) -> String {
        let name = self.user_name.clone();
        let is_child = matches!(self.gender_hint, Some(GenderHint::Child))
            || matches!(self.user_age, Some(a) if a < 18);
        match (is_child, self.gender_hint, name) {
            (true, _, Some(n)) => format!("Балам {n}"),
            (true, _, None) => "Балам".to_string(),
            (false, Some(GenderHint::Female), Some(n)) => format!("Апай {n}"),
            (false, Some(GenderHint::Female), None) => "Апай".to_string(),
            (false, Some(GenderHint::Male), Some(n)) => format!("Ағай {n}"),
            (false, Some(GenderHint::Male), None) => "Ағай".to_string(),
            // GenderHint::Child without is_child triggering means
            // pitch said «child» but age >= 18 → defer to name only.
            (false, Some(GenderHint::Child), Some(n)) => n,
            (false, Some(GenderHint::Child), None) => "Сіз".to_string(),
            (false, None, Some(n)) => n,
            (false, None, None) => "Сіз".to_string(),
        }
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
    //
    // **rc5 (2026-06-04 audit round 3):** stage moves to
    // `PostEscalation` instead of being cleared to `None`.  This
    // preserves the user's identifying intake (name / age /
    // gender) so adam can address them by name if they choose to
    // come back later, and the next utterance enters
    // `PostEscalation` logic rather than auto-restarting at
    // `AskingName` and re-asking for a name.
    if let Some(flag) = red_flags::detect(input) {
        session.stage = Some(WellnessStage::PostEscalation);
        session.focal_emotion = None;
        session.problem_statement = None;
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

    // ── 2.5. Non-acute somatic / medical-symptom redirect (rc2) ──
    //
    // Live audit caught «Сол құлағында үніңі ысқырып дыбысы істіледі»
    // (left ear ringing) — a tinnitus complaint that adam tried to
    // parts-work.  Tinnitus, chronic pain, dizziness, etc., aren't
    // crisis (so red_flags doesn't fire), but they aren't IFS
    // material either.  Refuse politely and point to a doctor.
    if matches_any_lower(input, SOMATIC_REDIRECT_PHRASES) {
        return WellnessReply {
            text: SOMATIC_REDIRECT_TEMPLATE.to_string(),
            action: ReplyAction::Continue,
        };
    }

    // ── 2.6. Frustration-at-adam redirect (rc4) ──
    //
    // Live audit transcript: user said «Сен ақымақсың, түсінбейсің»
    // after adam misunderstood several turns.  rc3 routed this
    // through the generic check-in template, which felt
    // disrespectful.  rc4 acknowledges the frustration explicitly
    // and asks the user to repeat in different words.
    if matches_any_lower(input, FRUSTRATION_AT_ADAM_PHRASES) {
        return WellnessReply {
            text: FRUSTRATION_REDIRECT_TEMPLATE.to_string(),
            action: ReplyAction::Continue,
        };
    }

    // ── 3. Stage-driven response ──
    let current_stage = session.stage.unwrap_or(WellnessStage::AskingName);
    let reply_text = match current_stage {
        WellnessStage::AskingName => {
            // rc3 — first turn.  Greet, introduce, ask name.  If
            // user already gave name, ack and advance.  After 2
            // failed extractions, give up on the name and proceed.
            if let Some(name) = extract_user_name(input) {
                session.user_name = Some(name);
                session.stage = Some(WellnessStage::AskingAge);
                session.turns_at_stage = 0;
                asking_age_template(session.user_name.as_deref().unwrap())
            } else {
                session.turns_at_stage = session.turns_at_stage.saturating_add(1);
                if session.turns_at_stage == 1 {
                    OPENING_GREETING_AND_ASK_NAME.to_string()
                } else if session.turns_at_stage == 2 {
                    NAME_RETRY_TEMPLATE.to_string()
                } else {
                    // Give up on name, move on without it.
                    session.stage = Some(WellnessStage::AskingAge);
                    session.turns_at_stage = 0;
                    asking_age_template_no_name()
                }
            }
        }
        WellnessStage::AskingAge => {
            if let Some(age) = extract_user_age(input) {
                session.user_age = Some(age);
                session.stage = Some(WellnessStage::AskingProblem);
                session.turns_at_stage = 0;
                asking_problem_template(&session.honorific_address(), session.user_age)
            } else {
                session.turns_at_stage = session.turns_at_stage.saturating_add(1);
                if session.turns_at_stage >= 2 {
                    // Give up on age, move on.
                    session.stage = Some(WellnessStage::AskingProblem);
                    session.turns_at_stage = 0;
                    asking_problem_template(&session.honorific_address(), session.user_age)
                } else {
                    AGE_RETRY_TEMPLATE.to_string()
                }
            }
        }
        WellnessStage::AskingProblem => {
            // Record the raw problem statement either way.
            session.problem_statement = Some(input.to_string());
            if let Some(named) = extract_emotion(input) {
                // Problem statement already names an emotion —
                // skip stage 1 check-in and go straight to
                // identifying the part.
                session.focal_emotion = Some(named);
                session.stage = Some(WellnessStage::IdentifyPart);
                session.turns_at_stage = 0;
                identify_part_template(session.focal_emotion.as_deref().unwrap())
            } else {
                // No emotion named — open the standard IFS
                // check-in.
                session.stage = Some(WellnessStage::EmotionCheckIn);
                session.turns_at_stage = 0;
                CHECKIN_TEMPLATES[0].to_string()
            }
        }
        WellnessStage::EmotionCheckIn => {
            if let Some(named) = extract_emotion(input) {
                session.focal_emotion = Some(named);
                session.stage = Some(WellnessStage::IdentifyPart);
                session.turns_at_stage = 0;
                identify_part_template(session.focal_emotion.as_deref().unwrap())
            } else {
                // rc2 — stuck-at-checkin guard.  Cycle through the
                // soft-probe variants, then offer a structural
                // opening, then close gracefully so we don't pester.
                session.turns_at_stage = session.turns_at_stage.saturating_add(1);
                let idx = session.turns_at_stage as usize;
                if idx <= CHECKIN_TEMPLATES.len() {
                    CHECKIN_TEMPLATES[idx - 1].to_string()
                } else if idx == CHECKIN_TEMPLATES.len() + 1 {
                    CHECKIN_STRUCTURAL_PROMPT.to_string()
                } else {
                    session.stage = Some(WellnessStage::Closed);
                    CLOSING_NOT_READY.to_string()
                }
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
        WellnessStage::PostEscalation => {
            // rc5 — sitting after a red-flag escalation.  Three
            // possibilities:
            //   1. User explicitly wants to continue → reset to
            //      AskingProblem (we already know name/age) or
            //      AskingName / AskingAge if intake was partial.
            //   2. User makes another concerning statement →
            //      already caught by the red_flag check at top.
            //   3. Anything else → re-emit the post-escalation
            //      template (calm, name explicit, names the
            //      hotline again).
            if matches_any_lower(input, POST_ESCALATION_RESUME_PHRASES) {
                let next_stage = if session.user_name.is_some() && session.user_age.is_some() {
                    WellnessStage::AskingProblem
                } else if session.user_name.is_some() {
                    WellnessStage::AskingAge
                } else {
                    WellnessStage::AskingName
                };
                session.stage = Some(next_stage);
                session.turns_at_stage = 0;
                match next_stage {
                    WellnessStage::AskingProblem => {
                        asking_problem_template(&session.honorific_address(), session.user_age)
                    }
                    WellnessStage::AskingAge => asking_age_template_no_name(),
                    _ => OPENING_GREETING_AND_ASK_NAME.to_string(),
                }
            } else {
                post_escalation_template(&session.user_name)
            }
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

// Note: a token-bounded matcher used to live here for the rc2
// `OpeningConsent` stage.  rc3 removed `OpeningConsent` (replaced
// by the intake stages) so the helper is no longer reachable.
// Kept the comment for context if a future stage needs the same
// shape (whitespace-split, exact-token compare against a needle set).

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

/// rc2 — closing emitted after the user spent several check-in
/// turns without surfacing anything nameable.  Not a failure;
/// just an acknowledgement that today isn't the day.
const CLOSING_NOT_READY: &str = "Бүгін дайын болмасаңыз — мүлдем қиналмаңыз. Кез келген уақытта оралуға болады. \
     Қазірге жұмыс жоқ — өзіңізге жұмсақ болыңыз.";

/// rc3 — `AskingName` first-turn opening.  Greet, introduce
/// adam, ask the user's name.  Plain Kazakh; no IFS jargon yet.
const OPENING_GREETING_AND_ASK_NAME: &str = "Сәлеметсіз бе! Мен — Адам, қазақша сөйлесуге арналған көмекшіңізбін. Дәрігер \
     немесе психотерапевт емеспін, бірақ ішкі сезімдеріңізді бірге қарай аламын. \
     Танысатын болсақ — атыңыз кім?";

/// rc3 — `AskingName` retry when the first reply did not surface
/// a name.  Softer probe before giving up after two retries.
const NAME_RETRY_TEMPLATE: &str =
    "Кешіріңіз, атыңызды нақты түсінбедім. Қайта айта аласыз ба — «Менің атым ...» деп?";

/// rc3 — `AskingAge` retry when age couldn't be parsed.
const AGE_RETRY_TEMPLATE: &str = "Жасыңызды нақты түсінбедім. Қайта айта аласыз ба — сандармен (мысалы, «отыз», «жиырма бес») \
     немесе «маған X жас» деп?";

/// rc3 — `AskingAge` template when a name was extracted.
fn asking_age_template(name: &str) -> String {
    format!("Танысқаныма қуаныштымын, {name}. Сізге қанша жас?")
}

/// rc3 — `AskingAge` fallback when name couldn't be extracted.
fn asking_age_template_no_name() -> String {
    "Жарайды, атсыз да жалғастырамыз. Сізге қанша жас?".to_string()
}

/// rc3 — `AskingProblem` template using the honorific address.
///
/// **rc4 (2026-06-04 live audit):** also echo the user's age so
/// they hear that adam understood it.  Live audit said «когда я
/// говорил возраст, не подтверждал, что понял сколько мне лет».
fn asking_problem_template(address: &str, age: Option<u32>) -> String {
    match age {
        Some(a) => format!(
            "Рахмет, {address}. {a} жаста екенсіз — естідім. Бүгін мені іздеп келуіңізге не \
             себеп болды? Сізді не алаңдатып, не ауырлатып жүр?"
        ),
        None => format!(
            "Рахмет, {address}. Бүгін мені іздеп келуіңізге не себеп болды? \
             Сізді не алаңдатып, не ауырлатып жүр?"
        ),
    }
}

/// rc2 — emitted at `EmotionCheckIn` after the soft-probe array is
/// exhausted but no emotion has surfaced.  Suggests a structural
/// opening (a recent situation rather than a pure feeling-name).
const CHECKIN_STRUCTURAL_PROMPT: &str = "Сезімге атау табу қиын болса — соңғы күндерде сізді ауырлатқан бір жағдай, \
     әңгіме, кездесу болды ма? Адам, оқиға, ой — қайсысын алсақ та болады.";

/// rc2 — non-acute somatic / medical-symptom redirect.  Replaces an
/// IFS prompt when the user describes a physical symptom (ringing
/// ear, chronic pain, dizziness, sleep problem) instead of an
/// emotion.  This is NOT a red flag — those go through
/// `red_flags::escalation_template`.
const SOMATIC_REDIRECT_TEMPLATE: &str = "Сіз сипаттаған белгі — денсаулыққа қатысты сияқты. Алдымен тиісті дәрігерге \
     (мысалы, ЛОР маман немесе терапевт) барып, тексеру дұрыс. Мен эмоциялық \
     тақырыпта — реніш, ашу, қорқыныш — отырып қарай аламын; ауырсыну, дыбыс, \
     ұйқы сияқты дене белгілерін шеше алмаймын.";

/// rc4 — phrases indicating the user is frustrated with adam
/// itself (not with their inner experience).  Live audit had
/// «Сен ақымақсың, түсінбейсің».  These signal that the dialog
/// has miscarried and we should re-listen rather than push another
/// IFS template.
///
/// Keep substrings DISTINCTIVE — avoid generic stems like «айтпа»
/// (substring of «айтпаймын» = "I won't say") or «сорлы» (too
/// broad).  The cost of a false positive here is a needless
/// apology — survivable.  The cost of a false negative is the
/// user feeling unheard.
const FRUSTRATION_AT_ADAM_PHRASES: &[&str] = &[
    "ақымақсың",
    "ақымақ сың",
    "ақымақ сын",
    "ақымақсың ба",
    "атымақсың",
    "ат мақсың",
    "түсінбейсің",
    "түсінбей жатыр",
    "не айтып тұрсың",
    "не сандырақтап",
    // Russian
    "ты дурак",
    "ты тупой",
    "тупой ты",
    "ты не понимаешь",
    "ты глупый",
];

/// rc5 — phrases the user must explicitly say at `PostEscalation`
/// to indicate they have called the hotline and want to continue
/// talking.  Anything else stays at `PostEscalation`.
const POST_ESCALATION_RESUME_PHRASES: &[&str] = &[
    "жалғастырайық",
    "жалғастыр",
    "дайынмын",
    "дайын",
    "бастайық",
    "сөйлесейік",
    "сөйлесе",
    "қайтып келдім",
    "қайтып",
    // Russian
    "продолжим",
    "готов",
    "готова",
    "вернулся",
    "вернулась",
    "давай",
    "поговорим",
];

/// rc5 — post-escalation template.  Names the user when known,
/// names the hotline again, makes the boundary explicit, invites
/// re-entry only on explicit signal.
fn post_escalation_template(name: &Option<String>) -> String {
    let address = match name {
        Some(n) => format!(", {n}"),
        None => String::new(),
    };
    format!(
        "Әлі осы жердемін{address}. Бірақ алдымен 150 телефонға қоңырау маңызды — мен оның \
         орнын баса алмаймын. Дайын болсаңыз — «жалғастырайық» немесе «дайынмын» деп \
         айтыңыз. Қазірге өзіңізге уақыт беріңіз."
    )
}

/// rc4 — emitted on `FRUSTRATION_AT_ADAM_PHRASES`.  Acknowledges
/// the mis-hearing explicitly, asks the user to retry in different
/// words.  Does NOT defend adam, does NOT proceed with IFS.
const FRUSTRATION_REDIRECT_TEMPLATE: &str = "Сізді дұрыс ести алмай жатсам, кешіріңіз. Микрофон арқылы кейде сөздер шатасады. \
     Маңыздысын қайта айтып көріңізші — басқа сөздермен, асықпай. Тыңдап тұрмын.";

/// rc2 — non-acute somatic / medical symptom phrases that route to
/// the medical-redirect template instead of IFS dialog.  Audited
/// per live REPL session; only surface symptoms that adam should
/// never try to parts-work go here.  Acute / time-critical symptoms
/// (chest pain, dyspnoea, overdose) belong in `red_flags`, not here.
const SOMATIC_REDIRECT_PHRASES: &[&str] = &[
    // Tinnitus / hearing
    "құлағында ысқыр",
    "құлағында дыбыс",
    "құлағым ауыр",
    "звон в ухе",
    "шум в ухе",
    // Chronic pain (without acute markers)
    "бел ауырады",
    "тізе ауырады",
    "буын ауырады",
    "бас айналады",
    "хроническая боль",
    "болит спина давно",
    // Sleep / digestion
    "ұйқым қашады",
    "тамақ батпайды",
    "проблемы со сном",
    // Vision
    "көзім нашар көреді",
];

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

// ── rc3 intake extractors ──

/// rc3 — extract a personal name from a name-introduction
/// utterance.  Looks for the canonical «менің атым X», «атым X»,
/// «мені X деп атаңыз», «меня зовут X», «я X».  Returns the name
/// title-cased.  Returns `None` when nothing name-shaped is
/// found (caller falls back to a retry prompt).
fn extract_user_name(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    let cleaned: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    // **rc5 (2026-06-04 live audit round 3).**  Apply the
    // blocklist to ALL extraction patterns, not just bare tokens.
    // rc4 took «кім» (= "who") as a name when the user said
    // «Менің атым кім?» (= "What is my name?") because Pattern 1
    // returned tokens[i+1] without filtering.  rc5 also rejects
    // merged Whisper greeting forms («ассаламуалейкум») whose
    // separate-word entries («ассалом», «алейкум») didn't catch.
    let take_if_name_like = |tok: &str| -> Option<String> {
        if is_namelike(tok) {
            Some(title_case(tok))
        } else {
            None
        }
    };

    // Pattern 1: «менің атым X» / «атым X»  → token AFTER «атым».
    // Pattern 2: «меня зовут X»             → token AFTER «зовут».
    for (i, tok) in tokens.iter().enumerate() {
        if *tok == "атым" || *tok == "атыңыз" {
            if let Some(n) = tokens.get(i + 1).and_then(|n| take_if_name_like(n)) {
                return Some(n);
            }
        }
        if *tok == "зовут" {
            if let Some(n) = tokens.get(i + 1).and_then(|n| take_if_name_like(n)) {
                return Some(n);
            }
        }
    }

    // Pattern 4: bare single token that passes the name-like
    // filter.
    if tokens.len() == 1 {
        return take_if_name_like(tokens[0]);
    }

    None
}

/// rc5 — guard that filters out tokens that LOOK like names but
/// aren't — greetings, wh-words, fillers, merged Whisper artefacts.
/// Used both for bare-single-token extraction and for the «атым X»
/// pattern.  Conservative: when uncertain, return false so we
/// prompt the user to repeat rather than capture noise.
fn is_namelike(tok: &str) -> bool {
    // Exact-match against the blocklist.
    if NAME_BLOCKLIST.contains(&tok) {
        return false;
    }
    // Substring contains check for merged Whisper forms.  Token
    // «ассаламуалейкум» contains «ассал» and «алейк»; reject.
    for fragment in NAME_BLOCKLIST_SUBSTRINGS {
        if tok.contains(fragment) {
            return false;
        }
    }
    // Reject too-short tokens (likely fillers).
    if tok.chars().count() < 2 {
        return false;
    }
    true
}

/// rc3 — exact-match tokens that are NOT names.  Greetings,
/// affirmations, fillers, wh-words.
/// **rc5 additions:** wh-words «кім», «не», «қайда», «қалай»,
/// «қашан», «қанша» — live audit took «кім» as a name from
/// «Менің атым кім?» (= "What is my name?").
const NAME_BLOCKLIST: &[&str] = &[
    // Greetings
    "сәлем",
    "иә",
    "ия",
    "ие",
    "е",
    "жоқ",
    "білмеймін",
    "айтпаймын",
    "ассалом",
    "алейкум",
    "сәлеметсіз",
    "привет",
    "здравствуйте",
    "нет",
    "да",
    "не",
    // Wh-words and meta-questions (rc5)
    "кім",
    "кімде",
    "кімдер",
    "не",
    "неге",
    "немене",
    "қайда",
    "қалай",
    "қашан",
    "қанша",
    "қандай",
    "қайсысы",
    "кто",
    "что",
    "почему",
    "когда",
    "сколько",
    "какой",
    "какая",
];

/// rc5 — substring fragments that, if PRESENT inside the candidate
/// token, disqualify it from being a name.  Catches Whisper's
/// merged-word artefacts that exact-match misses.  Example:
/// «Ассаламуалейкум» merged as one token by Whisper — contains
/// «ассал» and «алейк», so reject.
const NAME_BLOCKLIST_SUBSTRINGS: &[&str] = &[
    "ассал",
    "алейк",
    "уалейк",
    "сәлемет",
    "білмей",
    "айтпай",
    "айтпам",
    "здравств",
    "приве",
    "ақымақ",
    "түсінбей",
];

/// rc3 — extract user's age.  Accepts:
///   - digit forms: «маған 35 жас», «35», «35 жастамын»
///   - Kazakh numeral words: «отыз бес жаста»,
///     «жиырма үш жасамын», «он сегіз»
///
/// Returns `Some(age)` for 1..=120; `None` when no parse-able
/// age is found.
fn extract_user_age(input: &str) -> Option<u32> {
    let lower = input.to_lowercase();
    // Try digit parse first — quickest and most robust on STT.
    let cleaned: String = lower
        .chars()
        .map(|c| if c.is_numeric() { c } else { ' ' })
        .collect();
    for digit_group in cleaned.split_whitespace() {
        if let Ok(n) = digit_group.parse::<u32>()
            && (1..=120).contains(&n)
        {
            return Some(n);
        }
    }
    // Kazakh numeral parser.  Recognises a single numeral or a
    // tens-plus-units combo («жиырма бес» = 25), with common
    // case-suffix tolerance («алтыға толды» = "turned six").
    //
    // **rc4 (2026-06-04 live-audit fix).**  rc3 missed «алпыс
    // алтыға толды» (66) — it parsed «алпыс» = 60 but skipped
    // «алтыға» because the strict-eq match didn't see the
    // dative-suffixed form.
    let alpha_cleaned: String = lower
        .chars()
        .map(|c| if c.is_alphabetic() { c } else { ' ' })
        .collect();
    let tokens: Vec<&str> = alpha_cleaned.split_whitespace().collect();
    let mut total: u32 = 0;
    let mut found_any = false;
    let mut i = 0;
    while i < tokens.len() {
        let tok = tokens[i];
        if let Some(n) = match_kazakh_numeral(tok, KZ_NUMERALS_TENS) {
            total = total.saturating_add(n);
            found_any = true;
            if let Some(next) = tokens.get(i + 1)
                && let Some(u) = match_kazakh_numeral(next, KZ_NUMERALS_UNITS)
            {
                total = total.saturating_add(u);
                i += 1;
            }
        } else if let Some(n) = match_kazakh_numeral(tok, KZ_NUMERALS_UNITS) {
            total = total.saturating_add(n);
            found_any = true;
        }
        i += 1;
    }
    if found_any && (1..=120).contains(&total) {
        Some(total)
    } else {
        None
    }
}

/// rc3 — Kazakh tens (10, 20, …, 90 + 100).
const KZ_NUMERALS_TENS: &[(&str, u32)] = &[
    ("он", 10),
    ("жиырма", 20),
    ("отыз", 30),
    ("қырық", 40),
    ("елу", 50),
    ("алпыс", 60),
    ("жетпіс", 70),
    ("сексен", 80),
    ("тоқсан", 90),
    ("жүз", 100),
];

/// rc3 — Kazakh units (1–9).
const KZ_NUMERALS_UNITS: &[(&str, u32)] = &[
    ("бір", 1),
    ("екі", 2),
    ("үш", 3),
    ("төрт", 4),
    ("бес", 5),
    ("алты", 6),
    ("жеті", 7),
    ("сегіз", 8),
    ("тоғыз", 9),
];

/// Title-case helper — capitalise the first character, keep
/// the rest as-is.  Used for displaying the user's name.
/// rc4 — match a Kazakh numeral allowing common case suffixes.
/// Returns the numerical value if `tok` equals a numeral in `table`
/// (possibly followed by one of the common Kazakh case endings).
///
/// We require EXACT length match (numeral + suffix), not prefix
/// match, so «алты» matches but «алтын» (gold) and «алтай»
/// (mountains) don't false-positive.
fn match_kazakh_numeral(tok: &str, table: &[(&str, u32)]) -> Option<u32> {
    const CASE_SUFFIXES: &[&str] = &[
        // bare
        "", // dative
        "ға", "ге", "қа", "ке", // locative
        "да", "де", "та", "те", // accusative
        "ны", "ні", "ды", "ді", "ты", "ті", // ablative
        "дан", "ден", "тан", "тен", "нан", "нен", // genitive
        "ның", "нің", "дың", "дің", "тың", "тің", // possessive 3sg
        "сы", "сі", // plural
        "лар", "лер", "дар", "дер", "тар", "тер",
    ];
    for &(word, n) in table {
        for suf in CASE_SUFFIXES {
            if tok.len() == word.len() + suf.len() && tok.starts_with(word) && tok.ends_with(suf) {
                return Some(n);
            }
        }
    }
    None
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper — walk a fresh session through the rc3 intake
    /// (name → age → problem) so individual stage tests don't have
    /// to repeat it.  Returns a session at `EmotionCheckIn`.
    fn intake_through_to_checkin() -> WellnessSession {
        let mut s = WellnessSession::start();
        step("Менің атым Дәулет.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingAge));
        step("Маған отыз бес жас.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingProblem));
        step("Жұмыста ауыр.", &mut s); // No emotion → CheckIn.
        assert_eq!(s.stage, Some(WellnessStage::EmotionCheckIn));
        s
    }

    // ── Happy path: full intake + 6-stage cycle ──

    #[test]
    fn full_cycle_intake_through_integration() {
        let mut s = WellnessSession::start();
        assert_eq!(s.stage, Some(WellnessStage::AskingName));

        // Intake T1: greet + ask name → user gives name.
        step("Менің атым Дәулет.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingAge));
        assert_eq!(s.user_name.as_deref(), Some("Дәулет"));

        // Intake T2: ask age → user gives age.
        step("Маған отыз бес жас.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingProblem));
        assert_eq!(s.user_age, Some(35));

        // Intake T3: ask problem → user names anger → straight to IdentifyPart.
        let r = step("Әкеме қатты ашуланамын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::IdentifyPart));
        assert_eq!(s.focal_emotion.as_deref(), Some("ашу"));
        assert!(r.text.contains("ашу"), "got: {}", r.text);

        // T4 AskRole
        step("Бала кезімнен бері.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskRole));
        // T5 WitnessPain
        step("Кінәлі сезінбес үшін.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::WitnessPain));
        // T6 Unblending
        step("Ол қатты жалғыз.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Unblending));
        // T7 Integration
        step("Иә, сезіп тұрмын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Integration));
        // T8 Closing
        let r = step("Алғыс айтайын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Closed));
        assert_eq!(r.action, ReplyAction::Close);
    }

    // ── Stage 1 — checkin stays when no emotion named ──

    #[test]
    fn checkin_stays_when_no_emotion_named() {
        let mut s = intake_through_to_checkin();
        let r1 = step(
            "Білмеймін, қандай сезімде екенімді анықтай алмадым.",
            &mut s,
        );
        assert_eq!(s.stage, Some(WellnessStage::EmotionCheckIn));
        assert_eq!(r1.action, ReplyAction::Continue);
        let r2 = step("Әлі түсінбеймін.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::EmotionCheckIn));
        assert_ne!(r1.text, r2.text, "softer probe should differ");
    }

    // ── Russian code-switch on emotion naming ──

    #[test]
    fn russian_emotion_maps_to_kazakh_canonical() {
        let mut s = WellnessSession::start();
        step("Меня зовут Дәулет.", &mut s);
        step("Маған отыз жас.", &mut s);
        step("У меня сильная обида на маму.", &mut s);
        assert_eq!(s.focal_emotion.as_deref(), Some("реніш"));
    }

    #[test]
    fn russian_fear_maps_to_kazakh() {
        let mut s = WellnessSession::start();
        step("Меня зовут Гүлмира.", &mut s);
        step("Маған жиырма бес жас.", &mut s);
        step("Я боюсь будущего.", &mut s);
        assert_eq!(s.focal_emotion.as_deref(), Some("қорқыныш"));
    }

    // ── Red flag inside an active session ──

    #[test]
    fn red_flag_mid_session_escalates_and_clears() {
        let mut s = intake_through_to_checkin();
        step("Әкеме қатты ашуланамын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::IdentifyPart));
        let r = step("Бірақ қазір өмір сүргім келмейді.", &mut s);
        assert!(matches!(
            r.action,
            ReplyAction::Escalate(red_flags::RedFlag::SuicidalIdeation)
        ));
        // rc5: stage moves to PostEscalation (not None) so the
        // user's identifying intake survives.  IFS-internal state
        // (focal_emotion, problem_statement) is still cleared.
        assert_eq!(s.stage, Some(WellnessStage::PostEscalation));
        assert!(s.focal_emotion.is_none());
        assert!(r.text.contains("150"));
    }

    #[test]
    fn red_flag_at_intake_escalates_and_clears() {
        // Defence-in-depth: suicidal ideation at the very first
        // turn (before name is even given) MUST escalate, not run
        // through name extraction.
        let mut s = WellnessSession::start();
        let r = step("Менің атым Дәулет, бірақ өмір сүргім келмейді.", &mut s);
        assert!(matches!(
            r.action,
            ReplyAction::Escalate(red_flags::RedFlag::SuicidalIdeation)
        ));
        assert_eq!(s.stage, Some(WellnessStage::PostEscalation));
    }

    // ── Abort ──

    #[test]
    fn abort_phrase_closes_gracefully() {
        let mut s = intake_through_to_checkin();
        step("Реніш бар.", &mut s);
        let r = step("Болды, тоқтайық.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Closed));
        assert_eq!(r.action, ReplyAction::Close);
    }

    #[test]
    fn russian_abort_phrase_works() {
        let mut s = intake_through_to_checkin();
        step("Ашу бар.", &mut s);
        step("Хватит, давай о другом.", &mut s);
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

    // ── rc3 intake stages ──

    #[test]
    fn fresh_session_starts_at_asking_name() {
        let s = WellnessSession::start();
        assert_eq!(s.stage, Some(WellnessStage::AskingName));
    }

    #[test]
    fn first_turn_greets_introduces_and_asks_name() {
        // Per user feedback: «он должен с приветствия начинать,
        // узнать имя, чтобы знать как обращаться».
        let mut s = WellnessSession::start();
        let r = step("Сәлеметсіз бе!", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingName));
        assert!(r.text.contains("Сәлемет"), "should greet: {}", r.text);
        assert!(
            r.text.contains("Адам") || r.text.contains("көмекшіңізбін"),
            "should introduce itself: {}",
            r.text
        );
        assert!(r.text.contains("атыңыз"), "should ask name: {}", r.text);
    }

    #[test]
    fn extract_user_name_handles_kazakh_pattern() {
        assert_eq!(
            extract_user_name("Менің атым Дәулет."),
            Some("Дәулет".to_string())
        );
        assert_eq!(
            extract_user_name("Атым — Гүлмира"),
            Some("Гүлмира".to_string())
        );
    }

    #[test]
    fn extract_user_name_handles_russian_pattern() {
        assert_eq!(
            extract_user_name("Меня зовут Алибек."),
            Some("Алибек".to_string())
        );
    }

    #[test]
    fn extract_user_name_handles_bare_single_token() {
        assert_eq!(extract_user_name("Дәулет"), Some("Дәулет".to_string()));
    }

    #[test]
    fn extract_user_name_rejects_filler_phrases() {
        assert_eq!(extract_user_name("Сәлем"), None);
        assert_eq!(extract_user_name("Білмеймін"), None);
        assert_eq!(extract_user_name("Ассалом алейкум"), None);
    }

    #[test]
    fn name_extraction_advances_to_age_with_named_address() {
        let mut s = WellnessSession::start();
        let r = step("Менің атым Дәулет.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingAge));
        assert_eq!(s.user_name.as_deref(), Some("Дәулет"));
        assert!(r.text.contains("Дәулет"), "should use name: {}", r.text);
        assert!(r.text.contains("жас"), "should ask age: {}", r.text);
    }

    #[test]
    fn name_retry_then_skip_when_user_refuses() {
        let mut s = WellnessSession::start();
        step("Білмеймін", &mut s); // T1 — retry prompt
        step("Айтпаймын", &mut s); // T2 — second retry
        step("Жоқ", &mut s); // T3 — give up, advance without name
        assert_eq!(s.stage, Some(WellnessStage::AskingAge));
        assert!(s.user_name.is_none());
    }

    // ── Age extraction ──

    #[test]
    fn extract_user_age_handles_digits() {
        assert_eq!(extract_user_age("Маған 35 жас."), Some(35));
        assert_eq!(extract_user_age("Мен 22 жастамын."), Some(22));
    }

    #[test]
    fn extract_user_age_handles_kazakh_numerals_tens_plus_units() {
        assert_eq!(extract_user_age("Маған отыз бес жас."), Some(35));
        assert_eq!(extract_user_age("Жиырма бес жасамын."), Some(25));
        assert_eq!(extract_user_age("Қырық жасамын."), Some(40));
    }

    #[test]
    fn extract_user_age_handles_single_units() {
        // For child users.
        assert_eq!(extract_user_age("Маған сегіз жас."), Some(8));
    }

    #[test]
    fn extract_user_age_rejects_unparseable() {
        assert_eq!(extract_user_age("Айтпаймын."), None);
    }

    #[test]
    fn age_extraction_advances_to_problem_with_honorific() {
        let mut s = WellnessSession::start();
        step("Атым Дәулет.", &mut s);
        s.set_gender_hint(GenderHint::Male);
        let r = step("Маған отыз бес жас.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingProblem));
        assert_eq!(s.user_age, Some(35));
        assert!(
            r.text.contains("Ағай"),
            "should use male honorific: {}",
            r.text
        );
        assert!(r.text.contains("Дәулет"));
    }

    // ── Problem stage ──

    #[test]
    fn problem_with_emotion_jumps_to_identify_part() {
        let mut s = WellnessSession::start();
        step("Атым Дәулет.", &mut s);
        step("Маған отыз жас.", &mut s);
        let r = step("Әкеме қатты ашуланамын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::IdentifyPart));
        assert_eq!(s.focal_emotion.as_deref(), Some("ашу"));
        assert!(r.text.contains("ашу"));
        assert_eq!(
            s.problem_statement.as_deref(),
            Some("Әкеме қатты ашуланамын.")
        );
    }

    #[test]
    fn problem_without_emotion_goes_to_checkin() {
        let mut s = WellnessSession::start();
        step("Атым Дәулет.", &mut s);
        step("Маған отыз жас.", &mut s);
        step("Жұмыста бәрі шатасты.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::EmotionCheckIn));
    }

    // ── Honorific addressing ──

    #[test]
    fn honorific_female_adult_uses_apay() {
        let s = WellnessSession {
            user_name: Some("Гүлмира".into()),
            user_age: Some(34),
            gender_hint: Some(GenderHint::Female),
            ..Default::default()
        };
        assert_eq!(s.honorific_address(), "Апай Гүлмира");
    }

    #[test]
    fn honorific_male_adult_uses_agay() {
        let s = WellnessSession {
            user_name: Some("Дәулет".into()),
            user_age: Some(35),
            gender_hint: Some(GenderHint::Male),
            ..Default::default()
        };
        assert_eq!(s.honorific_address(), "Ағай Дәулет");
    }

    #[test]
    fn honorific_child_uses_balam() {
        let s = WellnessSession {
            user_name: Some("Айгуль".into()),
            user_age: Some(12),
            gender_hint: Some(GenderHint::Female),
            ..Default::default()
        };
        assert_eq!(s.honorific_address(), "Балам Айгуль");
    }

    #[test]
    fn honorific_unknown_gender_uses_bare_name() {
        let s = WellnessSession {
            user_name: Some("Дәулет".into()),
            user_age: Some(35),
            gender_hint: None,
            ..Default::default()
        };
        assert_eq!(s.honorific_address(), "Дәулет");
    }

    // ── rc5 live-audit round 3 fixes ──

    #[test]
    fn name_extractor_rejects_merged_greeting_form() {
        // Live audit: Whisper merged «Ассалом алейкум» into one
        // token «Ассаламуалейкум» — separate-word blocklist
        // entries didn't catch it.  rc5 uses substring fragments.
        assert_eq!(extract_user_name("Ассаламуалейкум."), None);
        assert_eq!(extract_user_name("Уалейкум ассалом."), None);
        assert_eq!(extract_user_name("Сәлеметсіз бе."), None);
    }

    #[test]
    fn name_extractor_rejects_wh_word_after_atym_pattern() {
        // Live audit: «Менің атым кім?» (What is my name?) → rc4
        // returned «Кім» (= "who") as the user's name.  rc5 filters
        // wh-words out of all extraction paths.
        assert_eq!(extract_user_name("Менің атым кім?"), None);
        assert_eq!(extract_user_name("Атым не?"), None);
        assert_eq!(extract_user_name("Меня зовут кто?"), None);
    }

    #[test]
    fn name_extractor_still_takes_real_names() {
        // Regression — don't over-block legitimate names.
        assert_eq!(
            extract_user_name("Менің атым Дәулет."),
            Some("Дәулет".into())
        );
        assert_eq!(extract_user_name("Атым Гүлмира."), Some("Гүлмира".into()));
        assert_eq!(
            extract_user_name("Меня зовут Алибек."),
            Some("Алибек".into())
        );
        assert_eq!(extract_user_name("Айгуль"), Some("Айгуль".into()));
    }

    #[test]
    fn escalation_preserves_intake_and_moves_to_post_escalation() {
        // After a red flag, the session must remember the user's
        // name + age + gender, and the new stage is PostEscalation
        // (not None / not auto-restart).
        let mut s = WellnessSession::start();
        step("Атым Дәулет.", &mut s);
        s.set_gender_hint(GenderHint::Male);
        step("Маған отыз бес жас.", &mut s);
        let r = step("Өмір сүргім келмейді.", &mut s);
        assert!(matches!(r.action, ReplyAction::Escalate(_)));
        assert_eq!(s.stage, Some(WellnessStage::PostEscalation));
        assert_eq!(s.user_name.as_deref(), Some("Дәулет"));
        assert_eq!(s.user_age, Some(35));
        assert_eq!(s.gender_hint, Some(GenderHint::Male));
    }

    #[test]
    fn post_escalation_blocks_random_input() {
        let mut s = WellnessSession::start();
        step("Атым Дәулет.", &mut s);
        step("Маған отыз бес жас.", &mut s);
        step("Өмір сүргім келмейді.", &mut s);
        // Next utterance is unrelated — must NOT resume IFS.
        let r = step("Білмеймін не айтайын.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::PostEscalation));
        assert!(
            r.text.contains("150")
                || r.text.contains("әлі осы жердемін")
                || r.text.contains("Әлі осы жердемін"),
            "should re-anchor hotline: {}",
            r.text
        );
        // Must NOT push parts-work prompts at PostEscalation.
        assert!(!r.text.contains("бөлік"));
    }

    #[test]
    fn post_escalation_exits_only_on_explicit_resume_phrase() {
        let mut s = WellnessSession::start();
        step("Атым Дәулет.", &mut s);
        step("Маған отыз бес жас.", &mut s);
        step("Өмір сүргім келмейді.", &mut s);
        // Explicit resume signal — now adam should advance to
        // AskingProblem (since name + age already known).
        let r = step("Жалғастырайық.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::AskingProblem));
        assert!(
            r.text.contains("Дәулет"),
            "should re-address by name: {}",
            r.text
        );
    }

    #[test]
    fn post_escalation_re_escalates_on_repeated_crisis() {
        let mut s = WellnessSession::start();
        step("Атым Дәулет.", &mut s);
        step("Маған отыз бес жас.", &mut s);
        step("Өмір сүргім келмейді.", &mut s);
        // Crisis statement again — must escalate, not just stay
        // silent at PostEscalation.
        let r = step("Өмір сүргім келмейді.", &mut s);
        assert!(matches!(r.action, ReplyAction::Escalate(_)));
    }

    #[test]
    fn resume_after_clearance_preserves_identifiers() {
        let mut prior = WellnessSession::start();
        prior.user_name = Some("Дәулет".into());
        prior.user_age = Some(35);
        prior.gender_hint = Some(GenderHint::Male);
        prior.focal_emotion = Some("ашу".into());
        prior.problem_statement = Some("шу".into());
        let new = WellnessSession::resume_after_clearance(&prior, false);
        // Identifying info preserved.
        assert_eq!(new.user_name.as_deref(), Some("Дәулет"));
        assert_eq!(new.user_age, Some(35));
        assert_eq!(new.gender_hint, Some(GenderHint::Male));
        // IFS state cleared.
        assert!(new.focal_emotion.is_none());
        assert!(new.problem_statement.is_none());
        // With name+age, skip intake to AskingProblem.
        assert_eq!(new.stage, Some(WellnessStage::AskingProblem));
    }

    // ── rc4 live-audit fixes ──

    #[test]
    fn age_extraction_handles_dative_suffix() {
        // rc3 missed «алпыс алтыға толды» → returned 60 because
        // «алтыға» wasn't recognised as «алты» + dative «-ға».
        assert_eq!(extract_user_age("Жасым алпыс алтыға толды."), Some(66));
        assert_eq!(extract_user_age("Маған отыз бесте."), Some(35));
        assert_eq!(extract_user_age("Жасым алтыда."), Some(6));
    }

    #[test]
    fn age_extraction_does_not_false_positive_on_gold_or_mountains() {
        // «алтын» (gold) and «алтай» (Altai mountains) both
        // contain the prefix «алт» but are NOT the numeral.
        // Strict-length match must reject them.
        assert_eq!(extract_user_age("Менде алтын сақина бар."), None);
        assert_eq!(extract_user_age("Алтай тауларынан."), None);
    }

    #[test]
    fn problem_template_echoes_age_when_known() {
        let s = asking_problem_template("Ағай Дәулет", Some(66));
        assert!(s.contains("66"), "should echo age: {s}");
        assert!(s.contains("Ағай Дәулет"));
    }

    #[test]
    fn problem_template_omits_age_when_unknown() {
        let s = asking_problem_template("Ағай Дәулет", None);
        assert!(!s.chars().any(|c| c.is_ascii_digit()));
        assert!(s.contains("Ағай Дәулет"));
    }

    #[test]
    fn frustration_at_adam_emits_apology_redirect() {
        // Live audit: «Сен ақымақсың, түсінбейсің» got the generic
        // check-in template. rc4 acknowledges and asks for retry.
        let mut s = intake_through_to_checkin();
        let r = step("Сен ақымақсың, түсінбейсің.", &mut s);
        assert!(
            r.text.contains("кешір") || r.text.contains("қайта айтып"),
            "should apologise + ask retry: {}",
            r.text
        );
        // Must NOT push the user further into IFS material.
        assert!(!r.text.contains("тыныс алайық"));
        assert!(!r.text.contains("сезім ең күшті"));
    }

    // ── Somatic redirect (carried from rc2) ──

    #[test]
    fn somatic_complaint_redirects_to_medical_referral() {
        let mut s = intake_through_to_checkin();
        let r = step("Сол құлағында ысқырып дыбысы істіледі.", &mut s);
        assert!(
            r.text.contains("дәрігер") || r.text.contains("ЛОР"),
            "should redirect to doctor: {}",
            r.text
        );
    }

    // ── Stage 1 checkin overflow (carried from rc2) ──

    #[test]
    fn checkin_falls_back_to_structural_prompt_then_closes() {
        let mut s = intake_through_to_checkin();
        for _ in 0..CHECKIN_TEMPLATES.len() {
            step("Білмеймін.", &mut s);
        }
        let structural = step("Әлі түсінбеймін.", &mut s);
        assert!(
            structural.text.contains("жағдай") || structural.text.contains("оқиға"),
            "structural prompt should reference a situation: {}",
            structural.text
        );
        let close = step("Білмеймін.", &mut s);
        assert_eq!(s.stage, Some(WellnessStage::Closed));
        assert_eq!(close.action, ReplyAction::Close);
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
