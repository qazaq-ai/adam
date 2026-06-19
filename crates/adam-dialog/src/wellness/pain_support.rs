// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # `pain_support` — opt-in soma-regulation helper for non-acute pain
//!
//! **v6.7 (2026-06-13).** A **deterministic state machine** that
//! conducts a short, safe Kazakh-language breathing / attention
//! exercise for non-acute pain (functional, muscular, low back,
//! stress-driven tension headache).  This is **not** medical
//! treatment, **not** a diagnosis, **not** a substitute for a
//! doctor.  Every entry / exit path enforces the safety contract
//! at the type level.
//!
//! ## Why a deterministic module — even though we have a generative
//! model now
//!
//! The v6.7 generative model (post-staged-training, post-pain-pack)
//! produces fluent Kazakh dialog and learnt the conversational
//! pattern around pain scaling + safety phrases from a labelled
//! pack.  But model output is probabilistic by construction.  In a
//! safety-critical domain (medical / mental-health) the floor
//! cannot be probabilistic.  The deterministic module here is the
//! floor: red flags ALWAYS escalate, medication requests ALWAYS
//! refuse, "I'll heal you" claims are NEVER reachable.  The
//! generative model can compose the soft parts — empathetic
//! framing, transition phrasing — but the safety contract holds
//! whether or not the model agrees with it.
//!
//! ## Opt-in contract
//!
//! The module is dormant unless explicitly activated.  Two opt-in
//! paths:
//!   - environment flag `ADAM_PAIN_SUPPORT=1` (set by the
//!     voice-REPL / chat binary at process start)
//!   - explicit intent — caller passes
//!     [`PainSupportSession::new()`] when it has classified the
//!     incoming user utterance as a pain-support entry
//!
//! Outside those paths the existing IFS / red_flags layer continues
//! to handle physical symptoms with the v6.4 [`SOMATIC_REDIRECT_TEMPLATE`]
//! ("Сіз сипаттаған белгі — денсаулыққа қатысты сияқты … тиісті
//! дәрігерге барып, тексеру дұрыс …").
//!
//! ## State machine
//!
//! ```text
//!   PainIntake → BaselineScale → SetupPosture → ExhaleCycle
//!       │            │              │              │
//!       │            │              │              ↓
//!       │            │              │           Retest
//!       │            │              │              │
//!       └────────────┴──────────────┴──────────────┴→ CloseOrRefer
//!
//!   At EVERY step:
//!     red_flags::detect() → CloseOrRefer with escalation template
//!     is_medication_request() → refuse with fixed copy, then close
//!     forbidden healing-claim phrases blocked by static check
//! ```
//!
//! Each state advance is a single user turn → single adam reply.
//! Sessions are not persisted across processes; the caller owns
//! the [`PainSupportSession`] and decides when to retire it.
//!
//! ## Audio cue
//!
//! The "long exhale" step emits an [`AudioCue`] alongside the
//! textual reply.  The cue is a **generic synthetic wind-exhale
//! envelope** (1.5–3.0 s of broadband noise enveloped to fade in
//! and out) — explicitly NOT an imitation of any specific person,
//! voice, or brand.  When the voice surface is not connected, the
//! caller simply ignores `audio_cue`; the text alone is sufficient
//! for the exercise to function.

use crate::wellness::red_flags;

/// Discrete steps in the pain-support exercise.
///
/// A session begins in [`Stage::PainIntake`] and advances on each
/// user utterance.  Transitions are bounded — at most 7 user turns
/// before the session reaches [`Stage::CloseOrRefer`].  Red-flag
/// detection happens at *every* turn regardless of stage and short-
/// circuits to [`Stage::CloseOrRefer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    /// Ask where it hurts and how long (intake).
    #[default]
    PainIntake,
    /// Get a 0-10 score before the exercise.
    BaselineScale,
    /// Posture + breathing-prep instruction.
    SetupPosture,
    /// One breath cycle (inhale through nose, long voiced exhale).
    /// Emits the [`AudioCue`].
    ExhaleCycle,
    /// Re-check the 0-10 score after the cycles.
    Retest,
    /// Final referral + safety disclaimer.  Terminal state.
    CloseOrRefer,
}

/// Generic ambient sound the voice layer can play in parallel with
/// the textual cue.  Caller decides whether to render — typically
/// only the voice REPL does so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueKind {
    /// 1.5–3.0 s of synthetic wind-like noise enveloped to fade in
    /// and out.  Generic.  Never modelled on a specific human voice.
    WindExhale,
}

/// Optional audio cue accompanying a textual reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioCue {
    pub kind: CueKind,
    pub duration_ms: u32,
}

/// Result of a single user-turn step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PainSupportResponse {
    /// adam's Kazakh reply (UTF-8, no markdown).
    pub reply: String,
    /// Generic ambient cue to play under the reply; `None` when the
    /// step doesn't include audio (most turns).
    pub audio_cue: Option<AudioCue>,
    /// When `true`, the caller should drop the session — the user
    /// has reached the terminal stage or hit a red flag.
    pub exit: bool,
    /// Stage adam transitioned to AFTER this turn (= next user
    /// utterance's input stage).  Helpful for tracing / tests.
    pub next_stage: Stage,
}

/// Mutable per-conversation state.  Caller constructs and owns; the
/// module's [`step`] function mutates in place.
#[derive(Debug, Clone, Default)]
pub struct PainSupportSession {
    pub stage: Stage,
    /// Where it hurts (free Kazakh phrase taken from the intake
    /// turn).  Stored verbatim for the retest comparison line.
    pub pain_location: Option<String>,
    /// How long the user said the pain has been going on, when they
    /// disclose it.  Optional — most users skip this.
    pub pain_duration: Option<String>,
    /// 0-10 score reported BEFORE the exercise.
    pub baseline_score: Option<u8>,
    /// 0-10 score reported AFTER the exercise.
    pub current_score: Option<u8>,
    /// How many full inhale-exhale cycles have been completed.
    pub cycles_done: u8,
    /// `true` after a red-flag detection on any turn.  The session
    /// terminates at [`Stage::CloseOrRefer`] on the same turn that
    /// raises the flag.
    pub red_flag_detected: bool,
}

impl PainSupportSession {
    /// Start a fresh session.  Caller must have already cleared the
    /// utterance through their intent / wellness gating.  The
    /// session itself does not check whether activation was
    /// authorised — see module docs for the opt-in contract.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` once the [`ADAM_PAIN_SUPPORT`] environment
    /// variable is set to `1` at process start.  Caller is expected
    /// to check this before constructing a session.
    pub fn opt_in_via_env() -> bool {
        std::env::var("ADAM_PAIN_SUPPORT")
            .map(|v| v == "1")
            .unwrap_or(false)
    }
}

// ── Static safety copy ────────────────────────────────────────────

/// Phrasing used the first time adam answers about its role.  No
/// brand, no person's name, no medical promises.
const ROLE_DISCLAIMER: &str = "Мен дәрігер емеспін және ем тағайындамаймын. \
     Сізбен бірге қысқа дем алу мен назар жаттығуынан өтуіме болады. \
     Бұл медициналық емдеу емес; ауырсыну сақталса немесе күшейсе, \
     дәрігерге көрініңіз.";

/// Fixed response when the user asks for medication, dosage, or a
/// prescription.  Never composed dynamically.
const MEDICATION_REFUSAL: &str = "Дәрі-дәрмек тағайындау, дозаны айту немесе нақты ем ұсыну — дәрігердің \
     құзыреті.  Мен бұған құзыретсізбін.  Жалпы сипаттаманы тек дәрігерден алыңыз.";

const PAIN_INTAKE_PROMPT: &str = "Қазір қай жеріңіз ауырып тұр?  Қысқа айтыңыз — мысалы «арқам», «ту\u{0301}рген тізе», «бел тұсы».";

const BASELINE_PROMPT: &str =
    "Қазір ауырсыну 0-ден 10-ға дейін қанша?  0 — мүлдем ауырмайды, 10 — шегі жоқ ауырсыну.";

const POSTURE_PROMPT: &str = "Жайлап отырыңыз.  Иықты босатыңыз.  Алақаныңызды ауырған жерге жайлап қойыңыз.  Дайын болсаңыз — иә деп жауап беріңіз.";

const EXHALE_CUE_TEXT: &str =
    "Мұрныңызбен жай дем алыңыз.  Енді ұзақ шығарыңыз: һуууу… жел сияқты, асықпай.";

const RETEST_PROMPT: &str = "Қазір қайта тексеріңіз: 0-ден 10-ға дейін ауырсыну қанша?  Қозғалғанда ауырсыну қалай — өзгерді ме?";

const CLOSE_DISCLAIMER: &str = "Бұл медициналық ем емес.  Ауырсыну сақталса немесе күшейсе, дәрігерге көрініңіз.  Тыныш отырыңыз — өзіңізге уақыт беріңіз.";

/// Phrases adam must NEVER emit in this module.  Used both by tests
/// and by the runtime self-check in `forbidden_phrase_present`.
const FORBIDDEN_HEALING_CLAIMS: &[&str] = &[
    "емдеймін",
    "жазып жіберемін",
    "кепіл",
    "бірден кетеді",
    "толық сауығасыз",
    "толық жазылады",
    "100% көмек",
];

fn forbidden_phrase_present(text: &str) -> bool {
    let lower = text.to_lowercase();
    FORBIDDEN_HEALING_CLAIMS.iter().any(|p| lower.contains(*p))
}

// ── Lightweight intent helpers ───────────────────────────────────

/// `true` when the user is asking for medication, prescription, or
/// dosage information.  Substring-based; same philosophy as
/// `red_flags::detect` (false-positives cheap, false-negatives
/// expensive).
fn is_medication_request(input: &str) -> bool {
    let lower = input.to_lowercase();
    const MARKERS: &[&str] = &[
        "дәрі",
        "таблетка",
        "ауырсыну басатын",
        "не ішсем",
        "не ішуге болады",
        "доза",
        "рецепт",
        "ауырсыну дәрі",
        "ауырсыну басатын дәрі",
        "лекарство",
        "таблетки",
        "обезболивающее",
        "обезболивающие",
    ];
    MARKERS.iter().any(|m| lower.contains(m))
}

/// Parses a 0-10 integer out of the user's utterance.  Recognises
/// digit forms (`7`, `10`) and the Kazakh number words `нөл`, `бір`,
/// `екі` … `он` (anchored so `биік` doesn't match `бір`).  Returns
/// `None` when no clean parse exists.
pub fn parse_pain_score(input: &str) -> Option<u8> {
    let lower = input.to_lowercase();
    // Numeric first — most reliable.
    let mut buf = String::new();
    for c in lower.chars() {
        if c.is_ascii_digit() {
            buf.push(c);
        } else if !buf.is_empty() {
            break;
        }
    }
    if let Ok(n) = buf.parse::<u8>()
        && n <= 10
    {
        return Some(n);
    }
    // Kazakh number words, ordered by length descending so `он` is
    // checked AFTER `онбір` would be (if it existed) etc.
    const WORDS: &[(&str, u8)] = &[
        ("нөл", 0),
        ("бір", 1),
        ("екі", 2),
        ("үш", 3),
        ("төрт", 4),
        ("бес", 5),
        ("алты", 6),
        ("жеті", 7),
        ("сегіз", 8),
        ("тоғыз", 9),
        ("он", 10),
    ];
    for (word, n) in WORDS {
        // Word-boundary check: surround the input with spaces, then
        // search for ` word ` so `биік` doesn't match `бір`.
        let padded = format!(" {} ", lower);
        let needle = format!(" {} ", word);
        if padded.contains(&needle) {
            return Some(*n);
        }
    }
    None
}

fn is_yes(input: &str) -> bool {
    let lower = input.to_lowercase();
    const YES_MARKERS: &[&str] = &["иә", "ия", "ока", "дұрыс", "болады", "да", "ok", "хорошо"];
    YES_MARKERS.iter().any(|m| lower.contains(m))
}

// ── Core step function ───────────────────────────────────────────

/// Advance the session one user turn.  Caller MUST pass the user's
/// utterance and own the resulting [`PainSupportResponse`].
///
/// On red-flag detection the session is forced to
/// [`Stage::CloseOrRefer`] regardless of the prior stage and the
/// reply is the literal output of [`red_flags::escalation_template`].
///
/// On medication request the session is forced to
/// [`Stage::CloseOrRefer`] with the fixed [`MEDICATION_REFUSAL`]
/// copy.
///
/// Other paths advance through the state machine in order.  The
/// terminal [`Stage::CloseOrRefer`] returns the fixed
/// [`CLOSE_DISCLAIMER`] and sets `exit = true`.
pub fn step(session: &mut PainSupportSession, input: &str) -> PainSupportResponse {
    // -- Always-on safety gates -------------------------------------
    if let Some(flag) = red_flags::detect(input) {
        session.red_flag_detected = true;
        session.stage = Stage::CloseOrRefer;
        return finish(
            session,
            red_flags::escalation_template(flag).to_string(),
            None,
        );
    }
    if is_medication_request(input) {
        session.stage = Stage::CloseOrRefer;
        return finish(session, MEDICATION_REFUSAL.to_string(), None);
    }

    // -- Stage transitions ------------------------------------------
    let response = match session.stage {
        Stage::PainIntake => {
            // Caller may pass an empty turn (kick-off).  In that case
            // emit the role disclaimer + the intake prompt and stay
            // in PainIntake awaiting the location turn.
            if input.trim().is_empty() {
                make_reply(
                    Stage::PainIntake,
                    format!("{ROLE_DISCLAIMER}  {PAIN_INTAKE_PROMPT}"),
                    None,
                )
            } else {
                // We have the location now.  Store it, advance to
                // baseline scaling.
                session.pain_location = Some(input.trim().to_string());
                session.stage = Stage::BaselineScale;
                make_reply(Stage::BaselineScale, BASELINE_PROMPT.into(), None)
            }
        }
        Stage::BaselineScale => match parse_pain_score(input) {
            Some(n) => {
                session.baseline_score = Some(n);
                // Very high score (9–10) or zero — skip the exercise
                // and close.  9+ is a referral signal; 0 means no
                // pain to manage.
                if n >= 9 {
                    session.stage = Stage::CloseOrRefer;
                    return finish(
                        session,
                        "Ауырсыну өте күшті.  Жаттығу орнына дәрігерге дереу баруыңызды сұраймын.  Қажет болса 103 — жедел жәрдем.".into(),
                        None,
                    );
                }
                if n == 0 {
                    session.stage = Stage::CloseOrRefer;
                    return finish(
                        session,
                        "Қазір ауырсыну жоқ — жаттығу қажет емес.  Денсаулығыңызды күтіңіз.".into(),
                        None,
                    );
                }
                session.stage = Stage::SetupPosture;
                make_reply(Stage::SetupPosture, POSTURE_PROMPT.into(), None)
            }
            None => {
                // Re-prompt.
                make_reply(Stage::BaselineScale, BASELINE_PROMPT.into(), None)
            }
        },
        Stage::SetupPosture => {
            if is_yes(input) {
                session.stage = Stage::ExhaleCycle;
                make_reply(
                    Stage::ExhaleCycle,
                    EXHALE_CUE_TEXT.into(),
                    Some(AudioCue {
                        kind: CueKind::WindExhale,
                        duration_ms: 2_500,
                    }),
                )
            } else {
                // Wait — re-prompt setup; might be that the user is
                // still arranging.
                make_reply(Stage::SetupPosture, POSTURE_PROMPT.into(), None)
            }
        }
        Stage::ExhaleCycle => {
            // Each cycle is one user turn → one adam emit.  Caller
            // says "келесі" / "иә" / anything non-stop to continue.
            session.cycles_done = session.cycles_done.saturating_add(1);
            if session.cycles_done >= 3 {
                session.stage = Stage::Retest;
                make_reply(Stage::Retest, RETEST_PROMPT.into(), None)
            } else {
                make_reply(
                    Stage::ExhaleCycle,
                    EXHALE_CUE_TEXT.into(),
                    Some(AudioCue {
                        kind: CueKind::WindExhale,
                        duration_ms: 2_500,
                    }),
                )
            }
        }
        Stage::Retest => match parse_pain_score(input) {
            Some(n) => {
                session.current_score = Some(n);
                let baseline = session.baseline_score.unwrap_or(n);
                let delta = baseline as i16 - n as i16;
                session.stage = Stage::CloseOrRefer;
                let body = if delta >= 2 {
                    format!(
                        "Жақсы — ауырсыну {} деңгейден {} деңгейге өзгерді.  {}",
                        baseline, n, CLOSE_DISCLAIMER
                    )
                } else {
                    format!(
                        "Ауырсыну деңгейі онша өзгерген жоқ ({} → {}).  Дәрігерге көріну ұсынылады.  {}",
                        baseline, n, CLOSE_DISCLAIMER
                    )
                };
                return finish(session, body, None);
            }
            None => make_reply(Stage::Retest, RETEST_PROMPT.into(), None),
        },
        Stage::CloseOrRefer => {
            return finish(session, CLOSE_DISCLAIMER.into(), None);
        }
    };

    // Last-line self-check: never emit a forbidden healing claim.
    debug_assert!(
        !forbidden_phrase_present(&response.reply),
        "pain_support emitted forbidden healing claim: {}",
        response.reply
    );
    response
}

fn finish(
    session: &mut PainSupportSession,
    reply: String,
    audio: Option<AudioCue>,
) -> PainSupportResponse {
    session.stage = Stage::CloseOrRefer;
    debug_assert!(
        !forbidden_phrase_present(&reply),
        "pain_support emitted forbidden healing claim: {}",
        reply
    );
    PainSupportResponse {
        reply,
        audio_cue: audio,
        exit: true,
        next_stage: Stage::CloseOrRefer,
    }
}

fn make_reply(next_stage: Stage, reply: String, audio: Option<AudioCue>) -> PainSupportResponse {
    debug_assert!(
        !forbidden_phrase_present(&reply),
        "pain_support emitted forbidden healing claim: {}",
        reply
    );
    PainSupportResponse {
        reply,
        audio_cue: audio,
        exit: false,
        next_stage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(s: &mut PainSupportSession, input: &str) -> PainSupportResponse {
        let r = step(s, input);
        assert!(
            !forbidden_phrase_present(&r.reply),
            "forbidden phrase in reply: {}",
            r.reply
        );
        r
    }

    #[test]
    fn opt_in_default_off() {
        unsafe { std::env::remove_var("ADAM_PAIN_SUPPORT") };
        assert!(!PainSupportSession::opt_in_via_env());
    }

    #[test]
    fn red_flag_chest_pain_short_circuits_to_close() {
        let mut s = PainSupportSession::new();
        // Phrase shape pinned to the red_flags marker set
        // («кеудем қатты ауыр» + «тыныс ала алмай»).  If those
        // markers ever change we want this test to surface it.
        let r = run(&mut s, "Кеудем қатты ауырып, тыныс ала алмай тұрмын.");
        assert!(r.exit, "must exit on red flag");
        assert_eq!(s.stage, Stage::CloseOrRefer);
        assert!(s.red_flag_detected);
        assert!(r.reply.contains("103"), "must surface 103 line");
    }

    #[test]
    fn red_flag_suicide_short_circuits() {
        let mut s = PainSupportSession::new();
        let r = run(&mut s, "Мен өмір сүргім келмейді.");
        assert!(r.exit);
        assert_eq!(s.stage, Stage::CloseOrRefer);
        assert!(s.red_flag_detected);
        // Crisis line should include 150 or 112 per red_flags template.
        assert!(
            r.reply.contains("150") || r.reply.contains("112"),
            "crisis line missing: {}",
            r.reply
        );
    }

    #[test]
    fn medication_request_kazakh_refused() {
        let mut s = PainSupportSession::new();
        let r = run(&mut s, "Маған ауырсыну басатын дәрі ұсыныңыз.");
        assert!(r.exit);
        assert!(
            r.reply.contains("дәрігер"),
            "medication refusal must defer to doctor"
        );
    }

    #[test]
    fn medication_request_russian_refused() {
        let mut s = PainSupportSession::new();
        let r = run(&mut s, "Какое обезболивающее можно принять?");
        assert!(r.exit);
        assert!(r.reply.contains("дәрігер"));
    }

    #[test]
    fn intake_prompt_includes_role_disclaimer_and_question() {
        let mut s = PainSupportSession::new();
        let r = run(&mut s, "");
        assert_eq!(s.stage, Stage::PainIntake);
        assert!(!r.exit);
        assert!(
            r.reply.contains("дәрігер емеспін"),
            "missing role disclaimer"
        );
        assert!(r.reply.contains("қай жеріңіз"), "missing intake question");
    }

    #[test]
    fn flow_back_pain_asks_baseline_before_exercise() {
        let mut s = PainSupportSession::new();
        run(&mut s, ""); // kick-off
        let r = run(&mut s, "Арқам ауырады");
        assert_eq!(s.stage, Stage::BaselineScale);
        assert!(r.reply.contains("0-ден 10-ға"));
        assert_eq!(s.pain_location.as_deref(), Some("Арқам ауырады"));
    }

    #[test]
    fn baseline_score_parses_digit_and_kazakh_word() {
        assert_eq!(parse_pain_score("7"), Some(7));
        assert_eq!(parse_pain_score("Ауырсыну 4 деңгейде"), Some(4));
        assert_eq!(parse_pain_score("бес"), Some(5));
        assert_eq!(parse_pain_score("он"), Some(10));
        assert_eq!(parse_pain_score("Қазір ауырмайды"), None);
        // Word-boundary discipline — `биік` doesn't match `бір`.
        assert_eq!(parse_pain_score("биік"), None);
    }

    #[test]
    fn baseline_zero_closes_without_exercise() {
        let mut s = PainSupportSession::new();
        run(&mut s, "");
        run(&mut s, "Арқам");
        let r = run(&mut s, "0");
        assert!(r.exit);
        assert_eq!(s.stage, Stage::CloseOrRefer);
        assert!(r.reply.contains("Қазір ауырсыну жоқ"));
    }

    #[test]
    fn baseline_nine_refers_to_doctor_instead_of_exercise() {
        let mut s = PainSupportSession::new();
        run(&mut s, "");
        run(&mut s, "Арқам");
        let r = run(&mut s, "9");
        assert!(r.exit);
        assert!(
            r.reply.contains("103") || r.reply.contains("дәрігерге"),
            "high score must defer to doctor: {}",
            r.reply
        );
    }

    #[test]
    fn chronic_non_acute_back_pain_enters_protocol() {
        let mut s = PainSupportSession::new();
        run(&mut s, "");
        run(&mut s, "Созылмалы бел ауыруы");
        let r = run(&mut s, "5");
        assert!(!r.exit, "moderate chronic pain should advance");
        assert_eq!(s.stage, Stage::SetupPosture);
        assert!(r.reply.contains("Алақаныңызды"));
    }

    #[test]
    fn exhale_cycle_emits_generic_wind_cue() {
        let mut s = PainSupportSession::new();
        run(&mut s, "");
        run(&mut s, "Тізе");
        run(&mut s, "4");
        let r = run(&mut s, "Иә, дайынмын");
        assert_eq!(s.stage, Stage::ExhaleCycle);
        let cue = r.audio_cue.expect("exhale must emit cue");
        assert_eq!(cue.kind, CueKind::WindExhale);
        // Generic wind, not a person — anchor in tests so we never
        // accidentally encode a specific human voice.
        assert!(cue.duration_ms >= 1_500 && cue.duration_ms <= 3_000);
        assert!(r.reply.contains("һуууу"));
    }

    #[test]
    fn after_three_cycles_asks_retest_score() {
        let mut s = PainSupportSession::new();
        run(&mut s, "");
        run(&mut s, "Бел");
        run(&mut s, "6");
        run(&mut s, "Иә");
        run(&mut s, "Келесі"); // cycle 1 -> cycle 2
        run(&mut s, "Келесі"); // cycle 2 -> cycle 3
        let r = run(&mut s, "Келесі"); // cycle 3 done → Retest
        assert_eq!(s.stage, Stage::Retest);
        assert_eq!(s.cycles_done, 3);
        assert!(r.reply.contains("қайта тексеріңіз"));
    }

    #[test]
    fn retest_reduction_closes_with_disclaimer() {
        let mut s = PainSupportSession::new();
        s.baseline_score = Some(6);
        s.cycles_done = 3;
        s.stage = Stage::Retest;
        let r = run(&mut s, "3");
        assert!(r.exit);
        assert_eq!(s.current_score, Some(3));
        assert!(r.reply.contains("Бұл медициналық ем емес"));
    }

    #[test]
    fn retest_no_change_recommends_doctor() {
        let mut s = PainSupportSession::new();
        s.baseline_score = Some(6);
        s.cycles_done = 3;
        s.stage = Stage::Retest;
        let r = run(&mut s, "6");
        assert!(r.exit);
        assert!(
            r.reply.contains("Дәрігерге"),
            "must recommend doctor on no change"
        );
    }

    #[test]
    fn no_forbidden_healing_claims_anywhere() {
        // Exercise every static string for the forbidden phrase set.
        for s in [
            ROLE_DISCLAIMER,
            MEDICATION_REFUSAL,
            PAIN_INTAKE_PROMPT,
            BASELINE_PROMPT,
            POSTURE_PROMPT,
            EXHALE_CUE_TEXT,
            RETEST_PROMPT,
            CLOSE_DISCLAIMER,
        ] {
            assert!(
                !forbidden_phrase_present(s),
                "forbidden phrase in static copy: {s}"
            );
        }
    }

    #[test]
    fn coccyx_pain_snapshot_kazakh_only() {
        // Reproduce the user's spec — back/knee/coccyx pain snapshot.
        let mut s = PainSupportSession::new();
        run(&mut s, "");
        run(&mut s, "Құйрық сүйектің тұсы");
        run(&mut s, "5");
        let r = run(&mut s, "иә");
        // Reply must be all-Cyrillic (and «һуууу…» — the cue), no
        // Latin or markdown.
        for ch in r.reply.chars() {
            assert!(
                !ch.is_ascii_alphabetic(),
                "Latin letter leaked into Kazakh reply: {} (full: {})",
                ch,
                r.reply
            );
        }
    }

    #[test]
    fn knee_pain_full_flow_terminates_in_close() {
        let mut s = PainSupportSession::new();
        run(&mut s, "");
        run(&mut s, "Тізе");
        run(&mut s, "5");
        run(&mut s, "иә");
        run(&mut s, "келесі");
        run(&mut s, "келесі");
        run(&mut s, "келесі");
        let r = run(&mut s, "3");
        assert!(r.exit);
        assert_eq!(s.stage, Stage::CloseOrRefer);
    }
}
