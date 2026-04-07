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
