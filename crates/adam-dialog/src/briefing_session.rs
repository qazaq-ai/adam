// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `briefing_session` — **v6.9.2 / L4.9 — safety-briefing session
//! engine (движок сессии инструктажа).**
//!
//! ## Why this module exists
//!
//! The industrial OT/TB (охрана труда / техника безопасности)
//! product must run the *full* service cycle the Kazakh regulation
//! prescribes (ст.182 ТК РК + Приказ №1019, дистанционный формат
//! разрешён Приказом №223 от 12.07.2026):
//!
//!   обучение → инструктаж → проверка знаний (устный опрос) →
//!   оценка (допуск / неуд) → документирование
//!
//! [`crate::v6_2_router`] already answers a *single* procedural
//! question from a [`ProcedureIR`].  A briefing SESSION is a
//! different shape: a stateful walk that (1) delivers every step in
//! order, (2) asks control questions, (3) grades the worker's oral
//! answers, and (4) emits a pass/fail protocol the ОТ/ТБ ИТР signs
//! off.  The law itself sanctions this — proverка знаний may be done
//! «с помощью технических средств обучения», which ADAM is.
//!
//! ## Design: standalone state machine, NOT a cascade route
//!
//! This is a distinct *mode*, not a per-turn factual lookup, so it
//! lives as a self-contained [`BriefingSession`] driven by an
//! explicit `advance(input)` loop.  The first increment does NOT
//! wire into [`crate::Conversation::turn`] — keeping the 193
//! `end_to_end` fixtures byte-identical while we prove the session
//! logic on real dialogs (per the etap-2 plan).  Wiring the engine
//! into `Conversation` as an activatable mode (using the existing
//! `ReferentKind::Procedure` discourse hook) is the next increment.
//!
//! ## No hallucination by construction
//!
//! Every control question is *generated deterministically* from the
//! curated procedure's own fields (`steps`, `hazards`,
//! `authorization`, `confirmation_gates`).  The engine never invents
//! a question or an expected answer — it only asks about content the
//! reviewer already curated, and grades against that same content.

use adam_algebra::ProcedureIR;

use crate::procedure_loader::shared_procedures;

/// Minimum common-prefix length (in chars) for two tokens to be
/// considered the same word.  Grading uses prefix overlap rather than
/// whole-word edit distance because Kazakh is agglutinative: a long
/// case/possessive suffix («құлып» → «құлыпты») inflates
/// edit-distance-over-length past any usable threshold, while the
/// shared root prefix stays stable.  This mirrors the retrieval
/// scorer's `word_overlap_match` in [`crate::v6_2_router`], so
/// grading vocabulary matches routing vocabulary.
const MIN_PREFIX_OVERLAP: usize = 4;

/// A control question passes when the best expected-phrase coverage
/// (fraction of that phrase's content tokens the worker mentioned)
/// reaches this floor.  0.4 means: for a two-concept phrase, naming
/// one core concept is enough; for a long step sentence, ~half its
/// content words.  Deliberately lenient — this is an oral briefing
/// check, not a written exam; the ИТР retains final sign-off.
const PASS_COVERAGE: f32 = 0.4;

/// Default session verdict threshold: the worker is *admitted*
/// (допущен) only when at least this fraction of control questions
/// pass.  Mirrors the regulation's «неудовлетворительно → не
/// допущен».  Configurable per [`BriefingSession::with_pass_ratio`].
const DEFAULT_PASS_RATIO: f32 = 0.6;

/// Upper bound on generated control questions.  Deep enough to test
/// every hazard's mitigation and every confirmation gate on a rich
/// procedure (LOTO lands exactly here), bounded so a session stays a
/// realistic oral briefing rather than a marathon exam.
const MAX_QUESTIONS: usize = 8;

/// Which curated field a control question was generated from — kept
/// so the protocol can show the ИТР *what* was tested, and so a
/// future increment can weight question types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionSource {
    /// Tests `authorization` — «кім жауапты?».
    Authority,
    /// Tests a specific `steps[i]` action.
    Step(u32),
    /// Tests a `hazards[i].kind_kk`.
    Hazard,
    /// Tests a `hazards[i].mitigation_kk`.
    Mitigation,
    /// Tests a `confirmation_gates[i]`.
    Gate,
}

/// One deterministically-generated control question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlQuestion {
    /// Kazakh prompt read aloud to the worker.
    pub prompt_kk: String,
    /// Acceptable answer phrases (disjunction — matching ANY ONE
    /// well is enough).  Each phrase is scored by content-token
    /// coverage; the question's grade is the best phrase's coverage.
    pub expected: Vec<String>,
    /// Provenance of this question.
    pub source: QuestionSource,
}

/// The worker's graded response to one control question.
#[derive(Debug, Clone)]
pub struct AnsweredQuestion {
    pub prompt_kk: String,
    pub source: QuestionSource,
    pub user_answer: String,
    pub passed: bool,
    /// Best expected-phrase coverage in `[0.0, 1.0]`.
    pub coverage: f32,
}

/// Where the session currently is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Just showed `steps[i]`, awaiting the worker's acknowledgement.
    Instruct(usize),
    /// Just asked `questions[i]`, awaiting the worker's answer.
    Quiz(usize),
    /// Nothing more to do; [`BriefingSession::protocol`] is ready.
    Done,
}

/// What the engine says on a single `advance` turn.
#[derive(Debug, Clone)]
pub struct BriefingReply {
    /// Kazakh surface to show the worker (feedback + next prompt).
    pub text: String,
    /// `true` once the session has finished and a protocol exists.
    pub done: bool,
}

/// The signed-off outcome of a completed session.
#[derive(Debug, Clone)]
pub struct BriefingProtocol {
    pub procedure_id: String,
    pub title_kk: String,
    pub answers: Vec<AnsweredQuestion>,
    pub passed_count: usize,
    pub total: usize,
    /// `true` → допущен; `false` → не допущен (повторный инструктаж).
    pub admitted: bool,
}

impl BriefingProtocol {
    /// Render the protocol as a Kazakh journal entry the ИТР can
    /// review before signing.  A caller with a clock stamps the
    /// date/identity around this body.
    pub fn render_kk(&self) -> String {
        let mut out = String::new();
        out.push_str("=== Нұсқаулық хаттамасы ===\n");
        out.push_str(&format!(
            "Рәсім: {} ({})\n",
            self.title_kk, self.procedure_id
        ));
        for (i, a) in self.answers.iter().enumerate() {
            out.push_str(&format!(
                "{}. {} — {} (қамту {:.0}%)\n",
                i + 1,
                a.prompt_kk,
                if a.passed {
                    "дұрыс"
                } else {
                    "толық емес"
                },
                a.coverage * 100.0,
            ));
        }
        out.push_str(&format!(
            "Нәтиже: {}/{} дұрыс — {}\n",
            self.passed_count,
            self.total,
            if self.admitted {
                "ЖҰМЫСҚА ЖІБЕРІЛДІ (допущен)"
            } else {
                "ЖІБЕРІЛМЕДІ — қайта нұсқаулық қажет (не допущен)"
            },
        ));
        out.push_str("ИТҚ растауы: ____________________\n");
        out
    }
}

/// A stateful safety-briefing session over one [`ProcedureIR`].
#[derive(Debug, Clone)]
pub struct BriefingSession {
    procedure_id: String,
    title_kk: String,
    applies_to: Vec<String>,
    /// Pre-rendered step lines, in order.
    steps: Vec<String>,
    questions: Vec<ControlQuestion>,
    answers: Vec<AnsweredQuestion>,
    phase: Phase,
    pass_ratio: f32,
}

impl BriefingSession {
    /// Build a session from a curated procedure.
    pub fn from_procedure(p: &ProcedureIR) -> Self {
        let steps = p
            .steps
            .iter()
            .map(|s| format!("Қадам {}: {}", s.sequence, s.action_kk))
            .collect();
        Self {
            procedure_id: p.id.clone(),
            title_kk: p.title_kk.clone(),
            applies_to: p.applies_to.clone(),
            steps,
            questions: build_questions(p),
            answers: Vec::new(),
            phase: Phase::Instruct(0),
            pass_ratio: DEFAULT_PASS_RATIO,
        }
    }

    /// Look a procedure up by its stable id and start a session.
    /// Returns `None` when no procedure carries that id.
    pub fn from_id(id: &str) -> Option<Self> {
        shared_procedures()
            .iter()
            .find(|p| p.id == id)
            .map(Self::from_procedure)
    }

    /// Override the admission threshold (fraction of questions that
    /// must pass).  Clamped to `[0.0, 1.0]`.
    #[must_use]
    pub fn with_pass_ratio(mut self, ratio: f32) -> Self {
        self.pass_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    pub fn procedure_id(&self) -> &str {
        &self.procedure_id
    }

    pub fn questions(&self) -> &[ControlQuestion] {
        &self.questions
    }

    /// Number of instruction steps in this session.  A driver needs
    /// this to know how many acknowledgement turns precede the quiz.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Opening announcement + the first step.  Call once before the
    /// `advance` loop.  A session with no steps is a curator bug (the
    /// loader rejects empty-step procedures), so `steps[0]` is safe.
    pub fn begin(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "«{}» бойынша нұсқаулық сессиясы басталды.\n",
            self.title_kk
        ));
        if !self.applies_to.is_empty() {
            out.push_str(&format!("Қолданылады: {}\n", self.applies_to.join("; ")));
        }
        out.push_str(
            "Мен қадамдарды ретімен түсіндіремін, содан кейін бақылау сұрақтарын қоямын. \
             Әр қадамнан кейін «түсінікті» деп жауап беріңіз.\n\n",
        );
        out.push_str(&self.steps[0]);
        out
    }

    /// Consume one worker utterance and advance the session.  During
    /// the instruction phase the utterance is an acknowledgement;
    /// during the quiz phase it is graded.
    pub fn advance(&mut self, input: &str) -> BriefingReply {
        match self.phase {
            Phase::Instruct(i) => {
                let next = i + 1;
                if next < self.steps.len() {
                    self.phase = Phase::Instruct(next);
                    BriefingReply {
                        text: self.steps[next].clone(),
                        done: false,
                    }
                } else {
                    // Steps exhausted → open the quiz (or finish if,
                    // pathologically, no questions could be built).
                    if self.questions.is_empty() {
                        self.phase = Phase::Done;
                        BriefingReply {
                            text: "Нұсқаулық аяқталды. Бақылау сұрақтары жоқ.".into(),
                            done: true,
                        }
                    } else {
                        self.phase = Phase::Quiz(0);
                        BriefingReply {
                            text: format!(
                                "Нұсқаулық аяқталды. Енді бақылау сұрақтары.\n\nСұрақ 1: {}",
                                self.questions[0].prompt_kk
                            ),
                            done: false,
                        }
                    }
                }
            }
            Phase::Quiz(i) => {
                let (passed, coverage) = grade(&self.questions[i], input);
                self.answers.push(AnsweredQuestion {
                    prompt_kk: self.questions[i].prompt_kk.clone(),
                    source: self.questions[i].source,
                    user_answer: input.to_string(),
                    passed,
                    coverage,
                });
                let feedback = if passed {
                    "Дұрыс."
                } else {
                    "Толық емес — бұл сұрақ қайта қаралуы тиіс."
                };
                let next = i + 1;
                if next < self.questions.len() {
                    self.phase = Phase::Quiz(next);
                    BriefingReply {
                        text: format!(
                            "{feedback}\n\nСұрақ {}: {}",
                            next + 1,
                            self.questions[next].prompt_kk
                        ),
                        done: false,
                    }
                } else {
                    self.phase = Phase::Done;
                    let proto = self.build_protocol();
                    let text = format!("{feedback}\n\n{}", proto.render_kk());
                    BriefingReply { text, done: true }
                }
            }
            Phase::Done => BriefingReply {
                text: "Сессия аяқталды.".into(),
                done: true,
            },
        }
    }

    /// The signed-off protocol, available once the session is `Done`.
    pub fn protocol(&self) -> Option<BriefingProtocol> {
        if self.phase == Phase::Done {
            Some(self.build_protocol())
        } else {
            None
        }
    }

    fn build_protocol(&self) -> BriefingProtocol {
        let passed_count = self.answers.iter().filter(|a| a.passed).count();
        let total = self.answers.len();
        let admitted = total > 0 && (passed_count as f32 / total as f32) >= self.pass_ratio;
        BriefingProtocol {
            procedure_id: self.procedure_id.clone(),
            title_kk: self.title_kk.clone(),
            answers: self.answers.clone(),
            passed_count,
            total,
            admitted,
        }
    }
}

/// Deterministically generate 3–5 control questions from the
/// procedure's curated fields.  Order is by pedagogical priority
/// (who is responsible → first action → hazard → mitigation → gate);
/// capped at 5.  Falls back to extra steps if the richer fields are
/// empty so even a sparse procedure yields ≥3 questions when it has
/// enough steps.
fn build_questions(p: &ProcedureIR) -> Vec<ControlQuestion> {
    let mut qs: Vec<ControlQuestion> = Vec::new();

    if !p.authorization.is_empty() {
        qs.push(ControlQuestion {
            prompt_kk: format!("«{}» рәсімінде кім жауапты?", p.title_kk),
            expected: p.authorization.clone(),
            source: QuestionSource::Authority,
        });
    }
    if let Some(first) = p.steps.first() {
        qs.push(ControlQuestion {
            prompt_kk: "Бұл рәсімнің бірінші қадамы неден басталады?".into(),
            expected: vec![first.action_kk.clone()],
            source: QuestionSource::Step(first.sequence),
        });
    }
    // Hazard identification — one question, any curated kind is a
    // correct answer (disjunction over all hazard kinds).
    if !p.hazards.is_empty() {
        qs.push(ControlQuestion {
            prompt_kk: "Бұл жұмыстағы қауіптер қандай?".into(),
            expected: p.hazards.iter().map(|h| h.kind_kk.clone()).collect(),
            source: QuestionSource::Hazard,
        });
    }
    // Mitigation — one question per hazard, because the protective
    // measure is the safety-critical knowledge and differs per hazard.
    for h in &p.hazards {
        qs.push(ControlQuestion {
            prompt_kk: format!("«{}» қаупінен қалай қорғанады?", h.kind_kk),
            expected: vec![h.mitigation_kk.clone()],
            source: QuestionSource::Mitigation,
        });
    }
    // Confirmation gates — ONE question (disjunction over all gates).
    // Per-gate questions would have to name the gate in the prompt,
    // leaking the answer; instead the worker must recall at least one
    // документируемое условие допуска unprompted.
    if !p.confirmation_gates.is_empty() {
        qs.push(ControlQuestion {
            prompt_kk: "Жұмысқа жіберу үшін қандай шарттар міндетті түрде \
                        орындалуы (ресімделуі) тиіс?"
                .into(),
            expected: p.confirmation_gates.clone(),
            source: QuestionSource::Gate,
        });
    }

    // Backfill from later steps if the richer fields left us short.
    if qs.len() < 3 {
        for s in p.steps.iter().skip(1) {
            if qs.len() >= 3 {
                break;
            }
            qs.push(ControlQuestion {
                prompt_kk: format!("{}-қадамда не істеледі?", s.sequence),
                expected: vec![s.action_kk.clone()],
                source: QuestionSource::Step(s.sequence),
            });
        }
    }

    qs.truncate(MAX_QUESTIONS);
    qs
}

/// Grade one answer.  Returns `(passed, best_coverage)`.
fn grade(q: &ControlQuestion, answer: &str) -> (bool, f32) {
    let user_tokens = content_tokens(answer);
    if q.expected.is_empty() {
        return (true, 1.0);
    }
    let best = q
        .expected
        .iter()
        .map(|phrase| phrase_coverage(&user_tokens, phrase))
        .fold(0.0_f32, f32::max);
    (best >= PASS_COVERAGE, best)
}

/// Fraction of `phrase`'s content tokens that appear (fuzzily) in the
/// worker's answer.  An empty phrase (no content tokens) scores 1.0
/// so it never blocks admission.
fn phrase_coverage(user_tokens: &[String], phrase: &str) -> f32 {
    let expected = content_tokens(phrase);
    if expected.is_empty() {
        return 1.0;
    }
    let matched = expected
        .iter()
        .filter(|e| user_tokens.iter().any(|u| token_match(u, e)))
        .count();
    matched as f32 / expected.len() as f32
}

/// Two tokens match when their common prefix is at least
/// [`MIN_PREFIX_OVERLAP`] chars AND at least half the shorter token's
/// length.  Tolerates Kazakh inflection tails while rejecting words
/// that merely share a 4-letter coincidence with a much longer token.
fn token_match(a: &str, b: &str) -> bool {
    let common = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    if common < MIN_PREFIX_OVERLAP {
        return false;
    }
    let shorter = a.chars().count().min(b.chars().count());
    common >= shorter.div_ceil(2)
}

/// Lowercase content tokens (≥ 4 chars), deduplicated, preserving
/// first-seen order.  Mirrors the `v6_2_router` scorer's 4-char
/// content-word floor so grading vocabulary matches retrieval.
fn content_tokens(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 4)
    {
        let t = raw.to_string();
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_algebra::{Hazard, ProcedureSource, ProcedureStep};

    fn sample() -> ProcedureIR {
        ProcedureIR {
            id: "kk_test_loto_001".into(),
            title_kk: "Жабдықты блоктау тәртібі".into(),
            title_ru: Some("Блокировка оборудования".into()),
            title_en: None,
            aliases_kk: vec![],
            aliases_ru: vec![],
            aliases_en: vec![],
            domain: adam_algebra::ProcedureDomain::OkhranaTruda,
            applies_to: vec!["жөндеу тобы".into()],
            prerequisites: vec!["наряд-рұқсат ресімделген".into()],
            steps: vec![
                ProcedureStep {
                    sequence: 1,
                    action_kk: "Энергетик электр қуатын ажыратады.".into(),
                    actor: Some("энергетик".into()),
                    condition: None,
                    evidence: Some("блоктау биркасы".into()),
                },
                ProcedureStep {
                    sequence: 2,
                    action_kk: "Механик гидравликалық жүйені босатады.".into(),
                    actor: Some("механик".into()),
                    condition: None,
                    evidence: None,
                },
            ],
            hazards: vec![Hazard {
                kind_kk: "күтпеген іске қосылу".into(),
                mitigation_kk: "жеке құлып пен бирка орнату".into(),
            }],
            authorization: vec!["энергетик".into(), "цех бастығы".into()],
            confirmation_gates: vec!["барлық блоктаулар тізілімге енгізілуі тиіс".into()],
            source: ProcedureSource {
                regulation_kk: "Еңбек кодексі".into(),
                regulation_id: "414-V".into(),
                article: Some("182-бап".into()),
                version_date: "2024-04-15".into(),
                retrieved_at: "2026-07-02".into(),
                url: None,
            },
        }
    }

    #[test]
    fn builds_questions_from_curated_fields() {
        let qs = build_questions(&sample());
        // authority + step1 + hazard + mitigation + gate = 5.
        assert_eq!(qs.len(), 5);
        assert_eq!(qs[0].source, QuestionSource::Authority);
        assert!(qs.iter().any(|q| q.source == QuestionSource::Hazard));
        assert!(qs.iter().any(|q| q.source == QuestionSource::Gate));
    }

    #[test]
    fn content_tokens_drop_short_words_and_dedupe() {
        let t = content_tokens("Цех бастығы цех бастығы жауапты");
        // «цех» (3 chars) dropped; «бастығы» deduped.
        assert_eq!(t, vec!["бастығы".to_string(), "жауапты".to_string()]);
    }

    #[test]
    fn grade_passes_on_correct_authority_answer() {
        let qs = build_questions(&sample());
        let authority = &qs[0];
        // Worker names one valid role → best phrase coverage 1.0.
        let (passed, cov) = grade(authority, "Цех бастығы жауапты");
        assert!(passed, "naming a valid authority role must pass");
        assert!(cov >= 0.99);
    }

    #[test]
    fn grade_fails_on_irrelevant_answer() {
        let qs = build_questions(&sample());
        let (passed, cov) = grade(&qs[0], "Білмеймін, ауа райы жақсы");
        assert!(!passed, "irrelevant answer must fail");
        assert!(cov < PASS_COVERAGE);
    }

    #[test]
    fn grade_tolerates_inflection() {
        // Expected mitigation contains «құлып»/«бирка»; inflected
        // «құлыпты», «бирканы» should still match via similarity.
        let q = ControlQuestion {
            prompt_kk: "?".into(),
            expected: vec!["жеке құлып пен бирка орнату".into()],
            source: QuestionSource::Mitigation,
        };
        let (passed, _) = grade(&q, "құлыпты орнатамыз");
        assert!(passed, "inflected forms must grade as a match");
    }

    #[test]
    fn full_session_admits_a_competent_worker() {
        let mut s = BriefingSession::from_procedure(&sample());
        let _intro = s.begin();
        // Acknowledge both steps.
        assert!(!s.advance("түсінікті").done); // step1 → step2
        assert!(!s.advance("түсінікті").done); // step2 → quiz Q1
        // Answer every question with curated content.
        s.advance("энергетик пен цех бастығы жауапты"); // authority
        s.advance("энергетик электр қуатын ажыратады"); // step1
        s.advance("күтпеген іске қосылу қаупі"); // hazard
        s.advance("жеке құлып пен бирка орнатамыз"); // mitigation
        let last = s.advance("барлық блоктаулар тізілімге енгізіледі"); // gate
        assert!(last.done, "session must finish after the last question");
        let proto = s.protocol().expect("protocol ready when done");
        assert_eq!(proto.total, 5);
        assert!(
            proto.admitted,
            "a worker answering all 5 correctly must be admitted; got {}/{}",
            proto.passed_count, proto.total
        );
    }

    #[test]
    fn full_session_denies_a_failing_worker() {
        let mut s = BriefingSession::from_procedure(&sample());
        let _ = s.begin();
        s.advance("ok");
        s.advance("ok");
        // Answer everything with noise.
        for _ in 0..s.questions().len() {
            s.advance("білмеймін");
        }
        let proto = s.protocol().expect("done");
        assert!(
            !proto.admitted,
            "a worker who answers nothing must be denied"
        );
        assert_eq!(proto.passed_count, 0);
    }
}
