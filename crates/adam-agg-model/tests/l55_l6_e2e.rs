// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! End-to-end test that wires the v6.0 L5.5 → L6 contract:
//!
//!   neural model → constrained generation (morpheme ids)
//!     → detokenise → surface
//!     → verifier → PASS / BLOCK
//!
//! Uses the actual production tokenizer / verifier modules and a
//! randomly-initialised TinyAgt model. We do **not** train the
//! model here — the test verifies that the *plumbing* connects
//! the layers without panic, not that random weights produce
//! meaningful output. Training is exercised by `tests/loss_*` and
//! the PoC binaries; gate verification is exercised by
//! `tests/verifier_integration.rs`. This test bridges them.

use adam_agg_model::generate::generate_constrained;
use adam_agg_model::verifier::{BlockReason, Verdict, Verifier};
use adam_agg_model::{TinyAgt, TinyAgtConfig};
use adam_agg_tokenizer::AggTokenizer;
use adam_kernel_fst::lexicon::LexiconV1;
use burn::backend::NdArray;
use burn::backend::ndarray::NdArrayDevice;

type B = NdArray<f32>;

#[test]
fn l55_to_l6_plumbing_does_not_panic() {
    // Realistic-sized config so the generation paths exercise the
    // same code that production hits, minus the trained weights.
    let device = NdArrayDevice::default();
    let cfg = TinyAgtConfig::new(64, 16, 32, 4, 2, 64);
    let model: TinyAgt<B> = cfg.init(&device);

    // Compact label vocab matching what poc_kazakh_train builds at
    // runtime. We need at least one valid Root + a couple of typed
    // suffixes so generate_constrained finds non-empty masks.
    let mut labels: Vec<String> = vec!["<unk>".into(), "BOS".into(), "EOS".into(), "<spc>".into()];
    labels.push("R:бала".into()); // 4
    labels.push("S:Number(Plural)".into()); // 5
    labels.push("S:Case(Dative)".into()); // 6
    while labels.len() < 64 {
        labels.push("<pad>".into());
    }

    let prefix = vec![1i64, 4i64]; // [BOS, R:бала]
    let tokens = generate_constrained(&model, &labels, &prefix, 4, &device);
    // The decoder must return at least the prefix; it may stop
    // immediately at EOS or extend by up to 4 tokens.
    assert!(tokens.len() >= prefix.len());
    assert_eq!(&tokens[..prefix.len()], &prefix[..]);
}

#[test]
fn detokenized_surface_round_trips_through_verifier() {
    // This is the L5.5 → L6 contract from architecture_neural_v6 §3.1:
    //   neural output → detokenise → verifier::check(surface).
    // We construct a known-good surface from the production
    // tokenizer (bypassing the model, since random weights would
    // emit arbitrary ids), then exercise the verifier on it. The
    // resulting verdict structure is what production code routes
    // into the NeuralCallRecord audit field.
    let lex = LexiconV1::load(
        "../../data/tokenizer/segmentation_roots.json",
        "../../data/lexicon_v1/apertium_imported_roots.json",
    )
    .expect("lexicon load");
    let tokenizer = AggTokenizer::build(lex);
    let facts_idx =
        Verifier::load_facts_index("../../data/retrieval/facts.json").expect("facts.json load");
    let verifier = Verifier::new(tokenizer, facts_idx, false /* permissive */);

    // Surface that should both round-trip through FST and be
    // grounded in facts.json («адам»).
    let record = verifier.check("адам");
    match record.verdict {
        Verdict::Pass {
            surface, grounded, ..
        } => {
            assert_eq!(surface, "адам");
            assert!(grounded, "«адам» must be grounded in facts.json");
        }
        Verdict::Block(reason) => panic!("expected Pass on «адам», got Block({reason:?})"),
    }

    // Surface that fails the FST round-trip (Latin string is not a
    // Kazakh surface). In permissive mode it still passes — the
    // round-trip succeeds trivially because the tokenizer returns
    // Unk and detokenize returns the same string. This documents
    // the current behaviour: round-trip checks morphology, not
    // script.
    let record = verifier.check("blarg");
    match record.verdict {
        Verdict::Pass { .. } => {} // Unk passes, ungrounded but permissive.
        Verdict::Block(BlockReason::Ungrounded) => {} // also acceptable in strict mode
        other => panic!("unexpected verdict on Latin nonsense: {other:?}"),
    }
}

#[test]
fn audit_record_shape_matches_architecture_spec() {
    // Smoke-check that the AuditRecord exposes the fields
    // architecture_neural_v6 §3.3 names as part of
    // NeuralCallRecord (`input_surface`, verdict carrying root +
    // grounded). If a future refactor renames these, this test
    // fires.
    let lex = LexiconV1::load(
        "../../data/tokenizer/segmentation_roots.json",
        "../../data/lexicon_v1/apertium_imported_roots.json",
    )
    .expect("lexicon load");
    let tokenizer = AggTokenizer::build(lex);
    let facts_idx =
        Verifier::load_facts_index("../../data/retrieval/facts.json").expect("facts.json load");
    let verifier = Verifier::new(tokenizer, facts_idx, false);

    let record = verifier.check("дүние");
    assert_eq!(record.input_surface, "дүние");
    match record.verdict {
        Verdict::Pass { surface, root, .. } => {
            assert_eq!(surface, "дүние");
            assert!(
                root.is_some(),
                "Pass record must carry the FST-extracted root"
            );
        }
        Verdict::Block(_) => panic!("«дүние» should pass; failure indicates facts.json regression"),
    }
}
