//! **v5.17.7 — adversarial D1 full closure (50/50 = 100%).**
//!
//! Closes the final 2 failures from the v5.17.0 adversarial
//! benchmark queue:
//!
//! - `fr_01` («Алматы қайда орналасқан?») — Hearsay routing
//!   suppression in question shape. The `-қан/-ген` suffix is
//!   morphologically ambiguous between past-evidential and
//!   perfective participle; FST tags both as
//!   `EvidenceKind::Hearsay`, but the participle reading is
//!   canonical inside a question.
//!
//! - `mta_06` («Тағы біреуін бер») — anaphoric exercise recall.
//!   Two-part: detector accepts the anaphor + give-verb shape
//!   without a practice noun, and Conversation::turn binds
//!   session.last_exercise_topic before planning.
//!
//! Adversarial v1 final tally: 50/50 = 100% — all six categories
//! at 100%. Floor in JSON raised to 1.0. v5.17.x quick-wins
//! complete; medium-term work (D2 → 150 cases, voice golden
//! corpus E, school pilot F, second-language spike G) remains.

use adam_dialog::{Conversation, Intent, TemplateRepository, semantics::interpret_text};
use adam_kernel_fst::lexicon::LexiconV1;

fn lex() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !std::path::Path::new(curated).exists() || !std::path::Path::new(apertium).exists() {
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

fn repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml")
}

#[test]
fn mta_06_anaphoric_exercise_request_detected_v5177() {
    let intent = interpret_text("Тағы біреуін бер", &[]);
    // Detector accepts the anaphor + give-verb shape without a
    // practice noun. Topic is None at detection time; resolution
    // to session.last_exercise_topic happens in Conversation::turn.
    assert!(
        matches!(intent, Intent::AskExercise { topic: None }),
        "anaphoric «тағы біреуін бер» must route to AskExercise {{ topic: None }}; got: {intent:?}"
    );
}

#[test]
fn mta_06_alternative_anaphor_surfaces_v5177() {
    // Same intent for plural / variant Kazakh surface forms.
    for surface in ["Тағы біреу бер", "Тағы біреуі бер", "Тағы біреуін ұсын"]
    {
        let intent = interpret_text(surface, &[]);
        assert!(
            matches!(intent, Intent::AskExercise { topic: None }),
            "«{surface}» must route to AskExercise; got: {intent:?}"
        );
    }
}

#[test]
fn end_to_end_mta_06_recalls_last_exercise_topic_v5177() {
    let Some(lex) = lex() else {
        eprintln!("skip: lexicon missing");
        return;
    };
    let repo = repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("ownership туралы жаттығу бер.", &lex, &repo, 0);
    let response = conv.turn("Тағы біреуін бер.", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("ownership"),
        "anaphoric exercise request must recall session.last_exercise_topic; got: {response}"
    );
    // Must NOT fall through to clarification.
    assert!(
        !lower.contains("қандай тақырып"),
        "anaphoric recall must not surface clarification; got: {response}"
    );
}

// **End-to-end fr_01 coverage** lives in `adversarial_dialog_v1.rs`
// because the factual `geography_kz` lookup requires the loaded
// retrieval runtime (morpheme index + facts + derived + priors).
// Adding the load-runtime boilerplate here would duplicate ~80
// lines from that test runner for one extra assertion; the
// adversarial benchmark's fr_01 case already provides the
// regression guarantee.
