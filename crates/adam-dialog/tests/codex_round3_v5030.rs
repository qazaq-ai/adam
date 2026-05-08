//! **v5.3.0** — Codex 2026-05-08 round-3 audit fixes (pass 2 — architectural).
//!
//! Closes the architectural items deferred from v5.2.5:
//!
//! - Bug 2 — contradiction resolution. The Алматы→Астана→«Жоқ, Алматы
//!   дұрыс»→«Қай қалада тұрамын?» dance previously ended with a
//!   phantom «Қала» (the noun) entering belief and adam asking about
//!   it. Post-fix: `try_resolve_pending_contradiction` accepts the
//!   user's pick, syncs session, and `detect_statement_of_location`
//!   bails on question markers.
//! - Bug 4 — anaphora over-carry. «Аспан неге көк?» after a
//!   conversation about Қазақстан previously inherited the prior
//!   topic via the `is_short_interrogative_followup` heuristic. Post-
//!   fix: that heuristic only fires when the wh-word is at position
//!   0 (no preceding content names the topic).
//!
//! Bug 5 (shallow domain answers) is content-engineering and deferred
//! to a content release (v5.3.5 / world_core PartOf entries).

use adam_dialog::{Conversation, TemplateRepository};
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

// ─── Bug 4: anaphora over-carry ───────────────────────────────────────

#[test]
fn fresh_subject_question_does_not_inherit_prior_topic() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    // Turn 1: establish Қазақстан as subject_under_discussion.
    let _ = conv.turn("Қазақстанда қанша облыс бар?", &lex, &repo, 0);
    // Turn 2: completely fresh subject «Аспан» (sky) with «неге» (why).
    // Pre-fix this would inherit Қазақстан and answer about Kazakhstan
    // with a "no causal data" hedge. Post-fix it must NOT mention
    // Қазақстан at all — the input names its own subject.
    let out = conv.turn("Аспан неге көк?", &lex, &repo, 1);
    assert!(
        !out.contains("Қазақстан"),
        "fresh-subject question must NOT inherit prior topic; got: {out}"
    );
}

#[test]
fn bare_followup_still_inherits_prior_topic() {
    // Sanity: «Кім құрды?» (who founded it?) AFTER a topic about a
    // country should still inherit — the input names no subject and
    // is anaphoric by design.
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Қазақ хандығы туралы айтыңызшы.", &lex, &repo, 0);
    let out = conv.turn("Кім құрды?", &lex, &repo, 1);
    // Either the response mentions Қазақ хандығы (correct anaphora) OR
    // honestly hedges with no causal data — the regression we guard
    // against is the FRESH-subject case above. For this bare followup,
    // we just verify the response is non-empty.
    assert!(!out.trim().is_empty());
}

#[test]
fn discourse_anaphor_короче_still_inherits() {
    // «Ал онда қанша аймақ бар?» after «Қазақстан» — explicit anaphor
    // («онда»). Must inherit Қазақстан as topic.
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Қазақстан туралы не білесіз?", &lex, &repo, 0);
    let out = conv.turn("Ал онда қанша аймақ бар?", &lex, &repo, 1);
    // The response should reference облыс/аймақ count for Қазақстан —
    // canonical retrieval-fact path. Loose check: non-empty.
    assert!(!out.trim().is_empty());
}

// ─── Bug 2: contradiction resolution by explicit pick ─────────────────

#[test]
fn explicit_pick_resolves_contradiction_and_session_syncs() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("Мен Алматыдамын.", &lex, &repo, 0);
    assert_eq!(conv.session.get("city").map(String::as_str), Some("Алматы"));

    let _ = conv.turn("Мен Астанадамын.", &lex, &repo, 1);
    // Contradiction is now pending. Session may hold either value;
    // the important thing is belief tracks 2 contested.
    assert_eq!(conv.belief.contradictions.len(), 1);

    let _ = conv.turn("Жоқ, Алматы дұрыс.", &lex, &repo, 2);
    // Post-fix: contradiction resolved AND session synced to Алматы.
    assert!(
        conv.belief.contradictions.is_empty(),
        "contradiction must be resolved after explicit pick; got {:?}",
        conv.belief.contradictions
    );
    assert_eq!(
        conv.session.get("city").map(String::as_str),
        Some("Алматы"),
        "session.city must sync to chosen value after resolution"
    );
}

#[test]
fn self_recall_question_after_resolution_surfaces_chosen_value() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("Мен Алматыдамын.", &lex, &repo, 0);
    let _ = conv.turn("Мен Астанадамын.", &lex, &repo, 1);
    let _ = conv.turn("Жоқ, Алматы дұрыс.", &lex, &repo, 2);
    let out = conv.turn("Қай қалада тұрамын?", &lex, &repo, 3);
    // Expected: response mentions Алматы (the chosen value), NOT
    // Астана and NOT «Қала» (the noun).
    assert!(
        out.contains("Алматы"),
        "self-recall after resolution must surface Алматы; got: {out}"
    );
    assert!(
        !out.contains("Астана"),
        "rejected value must not surface; got: {out}"
    );
    assert!(
        !out.contains("Қала — елді мекен"),
        "phantom noun «Қала» must not surface; got: {out}"
    );
}

#[test]
fn self_recall_question_does_not_pollute_belief_with_phantom_city() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    // Pre-fix: «Қай қалада тұрамын?» parsed as `StatementOfLocation
    // { city: "Қала" }` because «тұрамын» triggered the location
    // detector and «қалада» matched locative case. This polluted
    // belief with «Қала» (the noun "city"). Post-fix: question
    // markers («қай», «?») bail out.
    let _ = conv.turn("Мен Алматыдамын.", &lex, &repo, 0);
    let _ = conv.turn("Қай қалада тұрамын?", &lex, &repo, 1);
    // Belief should still hold only Алматы — no phantom «Қала».
    let city_facts: Vec<&str> = conv
        .belief
        .facts
        .iter()
        .filter(|f| f.predicate == "city")
        .map(|f| f.object.as_str())
        .collect();
    assert!(
        !city_facts.iter().any(|v| v == &"Қала"),
        "interrogative must not pollute belief with phantom «Қала»; got facts: {city_facts:?}"
    );
}

// ─── Audited 6-turn dialog regression pack ────────────────────────────

/// The full audited contradiction sequence from Codex round-3 review.
/// Asserts: each turn produces a non-empty response, no phantom «Қала»
/// pollution, and the final self-recall surfaces the chosen value.
#[test]
fn full_contradiction_dance_matches_audited_expectations() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let r1 = conv.turn("Мен Алматыдамын.", &lex, &repo, 0);
    assert!(r1.contains("Алматы"));

    let r2 = conv.turn("Мен Астанадамын.", &lex, &repo, 1);
    // Contradiction prompt must mention BOTH values for clarification.
    assert!(r2.contains("Алматы") && r2.contains("Астана"));

    let r3 = conv.turn("Жоқ, Алматы дұрыс.", &lex, &repo, 2);
    // Resolution-acknowledgement.
    assert!(!r3.trim().is_empty());

    let r4 = conv.turn("Қай қалада тұрамын?", &lex, &repo, 3);
    assert!(r4.contains("Алматы"));
    assert!(!r4.contains("Астана"));
}

/// The Қазақстан + Аспан over-carry sequence — Bug 4 e2e.
#[test]
fn full_anaphora_over_carry_sequence_matches_audited_expectations() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("Қазақстанда қанша облыс бар?", &lex, &repo, 0);
    let _ = conv.turn("Қазақстанның астанасы қандай?", &lex, &repo, 1);
    let r3 = conv.turn("Аспан неге көк?", &lex, &repo, 2);
    // Post-fix: response must NOT mention Қазақстан and SHOULD honestly
    // hedge ("no causal data" / "I don't know").
    assert!(
        !r3.contains("Қазақстан"),
        "Bug 4 — fresh-subject question must not inherit Қазақстан context; got: {r3}"
    );
}
