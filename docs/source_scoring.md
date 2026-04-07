# Source Scoring

## Purpose

Source acceptance should not be manual guesswork.

## Rule

Each source is evaluated by explicit scoring rules stored in:

- `data/raw/source_scoring_rules.json`

## Current Acceptance Logic

- open license increases score
- reviewed and training-ready quality increase score
- administrative and reference domains receive a modest bonus
- raw stage reduces score
- review-required or internal-only license status reduces score
- seed quality reduces score

Training acceptance is allowed only when:

- the source is already marked `allowed_for_training`
- and the computed score reaches the minimum acceptance threshold

## Output Contract

Scoring does not stop at an integer.

Each accepted or rejected source is written into:

- `data/curated/source_acceptance_report.json`

The report preserves:

- source id
- final score
- acceptance decision
- positive signals
- negative signals
