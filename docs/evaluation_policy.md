# Evaluation Policy

> **v6.0 forward-looking note (2026-05-16).** This policy is
> architecture-agnostic and applies equally to the v5.x
> deterministic pipeline and the v6.0 neural-composition layer. The
> v6.0 LLM-baseline benchmark (acceptance criterion #4 in
> [`architecture_neural_v6.md`](architecture_neural_v6.md) §9) is
> the addition to this policy: a public, pre-registered, frozen
> task suite that adam v6.0 is measured against, not retuned to
> game. The v6.0 results note for the empirical baseline lives at
> [`research/results_real_mix_2026_05_16.md`](research/results_real_mix_2026_05_16.md).

## Principle

This repository should not grow from model claims. It should grow from hard
evaluation.

## Required Evaluation Layers

- corpus quality
- tokenizer quality
- model evaluation
- linguistic audit

## Starter Task Types

- token efficiency
- tokenizer segmentation
- next-token prediction
- reading comprehension
- morphology sensitivity
- hallucination audit

## Interpretation Rule

No model claim is accepted unless it is backed by:

- a manifest
- a repeatable task definition
- a language-specific result
- a concrete eval dataset contract
