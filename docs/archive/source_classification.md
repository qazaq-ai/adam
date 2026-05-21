# Source Classification

## Purpose

Sources should not enter the repository as anonymous text blobs.

## Required Classification Fields

- `source_type`
- `domain`
- `license_class`
- `quality_tier`
- `allowed_for_training`

## Rules

- raw sources cannot be marked as training-ready
- sources with `review_required` license status cannot be used for training
- `seed` quality sources cannot be used for training

## Why This Matters

This keeps the corpus pipeline deterministic and auditable before model scale
work begins.

