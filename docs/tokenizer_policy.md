# Tokenizer Policy

## Goal

Train a tokenizer for Kazakh text, not a generic multilingual fallback.

## Current Constraints

- language: Kazakh
- script: Cyrillic
- normalization: lowercase + trim while preserving Cyrillic content
- special tokens must be explicit

## Non-Goals

- transliteration support
- multilingual token balancing
- early optimization for code-switching

## Evaluation Priority

Tokenizer work must be judged by:

- token efficiency on Kazakh text
- morphology-aware segmentation behavior
- stability on common agglutinative forms

## Deterministic Allomorphy

Agglutinative alternations must be encoded explicitly from root-final sound class.

- suffix choice must follow declared harmony and final-sound constraints
- imperative and negation variants must not be inferred statistically from tail similarity
