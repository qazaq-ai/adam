#!/usr/bin/env python3
"""
Phase 19 step A (2026-06-02) — Build labeled training pack for the
neural intent classifier.

User directive: «Давай начнем Phase 19 (neural intent classifier)».

This script unifies the per-category labels we already have across
three sources into a single training pack keyed by **top-level
Intent variants** from `crates/adam-dialog/src/intent.rs`:

  1. `data/curated/adam_training_dialog_pack.json`
       2 179 samples across ~300 fine-grained `category` strings.
       Most map cleanly to a top-level Intent; sub-variants
       (e.g. `ask_about_system.creator`) fold into the parent
       (`AskAboutSystem`).

  2. `data/curated/adam_training_adversarial_pairs_pack.json`
       4 020 samples across 21 `adversarial.<name>` domains.
       The adversarial categories were designed as contrastive
       minimal-pair seeds — many target specific Intent classes
       (e.g. `adversarial.name_daulet_canonical` →
       `StatementOfName`, `adversarial.bugin_today` → `Date`).

  3. `data/eval/*.json`
       978 eval `query` strings already labeled with `category`.
       Use the same parent-folding map as the dialog corpus.

The mapping table below covers the ~50 most-frequent Intent labels
that voice REPL uses in production.  Categories we can't map
cleanly (rust curriculum, async tutoring, code error sub-codes)
fold into `Unknown` for now — they are too sparse and out-of-scope
for the voice path.

Output: `data/curated/adam_intent_training_pack.json`:
  { "version": "...", "intents": [...51 labels...],
    "samples": [{"text": "Сәлем", "intent": "Greeting"}, ...] }

The classifier's training script (step B) BPE-tokenizes the text
and converts intent labels to dense ids using the `intents` array.

Run:
  python3 tools/build_intent_training_pack/build.py
"""

from __future__ import annotations
import json
import os
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIALOG_PACK = ROOT / "data/curated/adam_training_dialog_pack.json"
ADVERSARIAL_PACK = ROOT / "data/curated/adam_training_adversarial_pairs_pack.json"
EVAL_DIR = ROOT / "data/eval"
OUT_PACK = ROOT / "data/curated/adam_intent_training_pack.json"

# Ordered Intent label list — the classifier's output dimension.
# Order matters: matches the dense id ↔ string mapping the Rust
# loader uses.  Don't reorder without updating the loader.
INTENT_LABELS: list[str] = [
    # Social / dialog acts
    "Greeting",
    "Farewell",
    "Thanks",
    "Apology",
    "Affirmation",
    "Negation",
    "Compliment",
    "Insult",
    "UserDisagrees",
    "UserAcknowledgement",
    "WellWishes",
    "IntroProposal",
    "Casual",
    # How-are-you / wellbeing
    "AskHowAreYou",
    "StatementOfWellbeing",
    # Identity (user & system)
    "AskName",
    "StatementOfName",
    "AskAboutSystem",
    "AskAge",
    "StatementOfAge",
    "AskLocation",
    "StatementOfLocation",
    "AskOccupation",
    "StatementOfOccupation",
    "AskActivity",
    "StatementOfActivity",
    "AskFamily",
    "StatementOfFamily",
    # Time / weather
    "AskTime",        # «сағат неше», current time
    "AskDate",        # «бүгін қай күн»
    "AskWeather",
    "StatementOfWeather",
    # Topical / factual queries
    "AskAboutTopic",   # «X туралы айт»
    "AskDefinition",   # «X деген не / кім»
    "InventoryQuery",  # «X-да қандай Y бар»
    "Request",
    # Math + curriculum
    "MathExpression",
    "AskPurpose",
    "AskExercise",
    "AskCurrentProgress",
    "AskCurriculumContent",
    "AskNextTopic",
    "SubmitSolution",
    "CodeRequest",
    "ExplainCompilerError",
    "CrossLanguageContrast",
    # Other / misc
    "AskWillingness",
    "Filler",
    "Mood",  # mood expressions (sad/happy/angry/etc) — collapsed
    "Clarification",
    "Question",  # bare wh-question without specific routing
    # Fallback
    "Unknown",
]

# Category-string → Intent-label mapping.
# Covers the dialog_pack + eval datasets.  Unmatched categories
# fall through to Unknown.  Sub-category prefixes (e.g.
# `ask_about_system.creator`) match by `startswith` — the longest
# prefix wins.
CATEGORY_MAP: dict[str, str] = {
    # ----- Social / dialog acts -----
    "greeting.casual": "Greeting",
    "greeting.polite": "Greeting",
    "greeting.muslim": "Greeting",
    "greeting.morning": "Greeting",
    "greeting.day": "Greeting",
    "greeting.evening": "Greeting",
    "greeting.time_of_day": "Greeting",
    "greeting.intro_proposal": "IntroProposal",
    "farewell": "Farewell",
    "thanks": "Thanks",
    "apology": "Apology",
    "affirmation": "Affirmation",
    "agree": "Affirmation",
    "negation": "Negation",
    "disagree": "Negation",
    "compliment": "Compliment",
    "praise_action": "Compliment",
    "encouragement": "Compliment",
    "comfort": "Compliment",
    "congratulate": "WellWishes",
    "well_wishes": "WellWishes",
    "insult": "Insult",
    "disagreement_ack": "UserDisagrees",
    "user_acknowledgement": "UserAcknowledgement",
    "acknowledgment": "UserAcknowledgement",
    "filler": "Filler",
    "small_talk": "Casual",
    # ----- How-are-you / wellbeing -----
    "ask_how_are_you": "AskHowAreYou",
    "statement_of_wellbeing": "StatementOfWellbeing",
    # ----- Identity user -----
    "ask_name": "AskName",
    "ask_name.with_known_user": "AskName",
    "statement_of_name": "StatementOfName",
    "introduce_self": "StatementOfName",
    "ask_age": "AskAge",
    "ask_age.with_known_user": "AskAge",
    "statement_of_age": "StatementOfAge",
    "give_age": "StatementOfAge",
    "ask_origin": "AskLocation",
    "give_origin": "StatementOfLocation",
    "ask_location": "AskLocation",
    "ask_location.with_known_user": "AskLocation",
    "ask_location.with_known_user.geo_feature": "AskLocation",
    "ask_location.user_self.no_data": "AskLocation",
    "statement_of_location": "StatementOfLocation",
    "statement_of_location.geo_feature": "StatementOfLocation",
    "ask_work": "AskOccupation",
    "ask_occupation": "AskOccupation",
    "ask_occupation.unknown_user": "AskOccupation",
    "ask_occupation.with_known_user": "AskOccupation",
    "give_work": "StatementOfOccupation",
    "statement_of_occupation": "StatementOfOccupation",
    "ask_activity": "AskActivity",
    "ask_activity.with_known_user": "AskActivity",
    "statement_of_activity": "StatementOfActivity",
    "ask_family": "AskFamily",
    "statement_of_family": "StatementOfFamily",
    # ----- Identity system -----
    "ask_about_system": "AskAboutSystem",
    "system_identity_name": "AskAboutSystem",
    "system_identity_speaking_language": "AskAboutSystem",
    "identity": "AskAboutSystem",
    # ----- Time / date / weather -----
    "ask_time": "AskTime",
    "time": "AskTime",
    "ask_date": "AskDate",
    "ask_day_of_week": "AskDate",
    "ask_weather": "AskWeather",
    "ask_weather.no_data_known_city": "AskWeather",
    "ask_weather.live": "AskWeather",
    "statement_of_weather": "StatementOfWeather",
    # ----- Topical / factual -----
    "ask_about_topic": "AskAboutTopic",
    "ask_definition": "AskDefinition",
    "ask_session_recall": "AskAboutTopic",
    "factual_retrieval": "AskAboutTopic",
    "ask_constitution": "AskAboutTopic",
    "geography_kz": "InventoryQuery",
    "history_kz": "AskAboutTopic",
    "history_kazakhstan": "AskAboutTopic",
    "abai_works": "AskAboutTopic",
    "astronomy": "AskAboutTopic",
    "biology_basic": "AskAboutTopic",
    "chemistry_school": "AskAboutTopic",
    "physics_school": "AskAboutTopic",
    "mathematics_basic": "AskAboutTopic",
    "kz_industry": "AskAboutTopic",
    "kz_literature": "AskAboutTopic",
    "kz_constitution": "AskAboutTopic",
    "world_core_science": "AskAboutTopic",
    "world_core_geo": "AskAboutTopic",
    "world_core_culture": "AskAboutTopic",
    "world_core_history": "AskAboutTopic",
    "kazakh_cuisine_yesno": "AskAboutTopic",
    "kazakh_cuisine_definition": "AskDefinition",
    "natural_phenomena_yesno": "AskAboutTopic",
    "natural_phenomena_definition": "AskDefinition",
    "honest_unknown": "Unknown",
    "honest_unknown_check": "AskAboutTopic",
    "request": "Request",
    # ----- Math -----
    "math_arithmetic": "MathExpression",
    "math_word_problem": "MathExpression",
    "math_word_problems": "MathExpression",
    "math_equations": "MathExpression",
    "math_concepts": "AskDefinition",
    "math_answer": "MathExpression",
    "math_clarification": "Clarification",
    "math_refusal": "MathExpression",
    "math": "MathExpression",
    "math_with_real_numbers": "MathExpression",
    "math_canonical": "MathExpression",
    "check_answer.correct": "Affirmation",
    "check_answer.incorrect": "Negation",
    "explain_steps": "Clarification",
    "compositional_function": "MathExpression",
    # ----- Curriculum -----
    "ask_curriculum_content": "AskCurriculumContent",
    "ask_next_topic": "AskNextTopic",
    "next_topic.suggestion": "AskNextTopic",
    "next_topic.complete": "AskNextTopic",
    "current_progress.recap": "AskCurrentProgress",
    "current_progress.empty": "AskCurrentProgress",
    "ask_exercise": "AskExercise",
    "ask_exercise.with_topic": "AskExercise",
    "ask_exercise.no_topic": "AskExercise",
    "submit_solution.passed": "SubmitSolution",
    "submit_solution.passed_stage_closed": "SubmitSolution",
    "submit_solution.passed_curriculum_complete": "SubmitSolution",
    "submit_solution.failed_known": "SubmitSolution",
    "submit_solution.failed_unknown": "SubmitSolution",
    "submit_solution.env_error": "SubmitSolution",
    "code_request": "CodeRequest",
    "code_request.with_topic": "CodeRequest",
    "code_request.no_topic": "CodeRequest",
    "code_tutor_traps": "CodeRequest",
    "code_refusal": "CodeRequest",
    "code_block_routing": "CodeRequest",
    "explain_compiler_error.with_explanation": "ExplainCompilerError",
    "explain_compiler_error.no_explanation": "ExplainCompilerError",
    "ask_previous_error.with_data": "ExplainCompilerError",
    "ask_previous_error.empty": "ExplainCompilerError",
    "ask_previous_error": "ExplainCompilerError",
    "ask_fix_previous_error_v5100": "ExplainCompilerError",
    "ask_purpose": "AskPurpose",
    "ask_purpose.with_topic": "AskPurpose",
    "ask_purpose.no_topic": "AskPurpose",
    "ask_willingness": "AskWillingness",
    "willingness": "AskWillingness",
    "cross_language_contrast": "CrossLanguageContrast",
    "cross_language_contrast.with_body": "CrossLanguageContrast",
    "cross_language_contrast.no_body": "CrossLanguageContrast",
    # ----- Bare question shapes -----
    "question_what": "Question",
    "question_when": "AskDate",
    "question_where": "AskLocation",
    "question_who": "Question",
    "question_why": "Question",
    "question_how": "Question",
    "clarification": "Clarification",
    "compare_topics.dual": "Question",
    "compare_topics.hedge": "Question",
    "compare_topics": "Question",
    "resolve_contradiction": "UserDisagrees",
    "dismiss_contradiction": "UserDisagrees",
    "check_contradiction": "UserDisagrees",
    # ----- Mood expressions (collapsed) -----
    "mood_sad": "Mood",
    "mood_happy": "Mood",
    "mood_angry": "Mood",
    "mood_tired": "Mood",
    "mood_scared": "Mood",
    "mood_bored": "Mood",
    "mood_anxious": "Mood",
}

# Adversarial pack uses `domain` field with `adversarial.<name>` prefix.
ADVERSARIAL_MAP: dict[str, str] = {
    "adversarial.name_daulet_canonical": "StatementOfName",
    "adversarial.name_other_canonical": "StatementOfName",
    "adversarial.saulet_architecture": "AskAboutTopic",
    "adversarial.bugin_today": "AskDate",
    "adversarial.ugym_notion_only": "AskDefinition",
    "adversarial.birinshi_first": "AskAboutTopic",
    "adversarial.berinshi_2nd_etc": "AskAboutTopic",
    "adversarial.birneshe_several": "Question",
    "adversarial.qalynyz_how_are_you": "AskHowAreYou",
    "adversarial.qalanyz_city": "AskLocation",
    "adversarial.kun_day": "AskDate",
    "adversarial.kun_sun_celestial": "AskDefinition",
    "adversarial.kun_value_archaic": "AskDefinition",
    "adversarial.qan_blood": "AskDefinition",
    "adversarial.togyz_nine": "MathExpression",
    "adversarial.salem_greetings": "Greeting",
    "adversarial.math_canonical": "MathExpression",
    "adversarial.math_with_real_numbers": "MathExpression",
    "adversarial.kz_geo_inventory": "InventoryQuery",
    "adversarial.kim_questions": "AskAboutSystem",
    "adversarial.president_kz": "AskAboutTopic",
}


def map_category(cat: str, table: dict[str, str]) -> str | None:
    """Map a category string to an Intent label.

    First exact match, then longest-prefix match (for sub-categories
    like `ask_about_system.creator` → `AskAboutSystem`).  Returns
    None when no rule fires.
    """
    if cat in table:
        return table[cat]
    # Longest-prefix match.
    best_key = None
    best_len = 0
    for k in table:
        if cat.startswith(k + ".") and len(k) > best_len:
            best_key = k
            best_len = len(k)
    return table[best_key] if best_key else None


def load_dialog_pack() -> list[dict]:
    if not DIALOG_PACK.exists():
        return []
    d = json.loads(DIALOG_PACK.read_text())
    out = []
    for s in d.get("samples", []):
        cat = s.get("category", "")
        text = s.get("text", "").strip()
        if not text:
            continue
        intent = map_category(cat, CATEGORY_MAP)
        if intent is None:
            continue
        out.append({"text": text, "intent": intent, "source": "dialog_corpus"})
    return out


def load_adversarial_pack() -> list[dict]:
    if not ADVERSARIAL_PACK.exists():
        return []
    d = json.loads(ADVERSARIAL_PACK.read_text())
    out = []
    for s in d.get("samples", []):
        cat = s.get("domain", "")
        text = s.get("text", "").strip()
        if not text:
            continue
        intent = ADVERSARIAL_MAP.get(cat)
        if intent is None:
            continue
        out.append({"text": text, "intent": intent, "source": "adversarial"})
    return out


def load_eval_queries() -> list[dict]:
    if not EVAL_DIR.is_dir():
        return []
    out = []
    for f in sorted(EVAL_DIR.iterdir()):
        if not f.suffix == ".json":
            continue
        if any(skip in f.name for skip in ("benchmark", "manifest", "report")):
            continue
        try:
            d = json.loads(f.read_text())
        except Exception:
            continue
        cases = (
            d.get("cases")
            or d.get("queries")
            or d.get("samples")
            or (d if isinstance(d, list) else [])
        )
        if not isinstance(cases, list):
            continue
        for c in cases:
            if not isinstance(c, dict):
                continue
            text = c.get("query", "").strip()
            cat = c.get("category", "")
            if not text:
                continue
            intent = map_category(cat, CATEGORY_MAP)
            if intent is None:
                continue
            out.append({"text": text, "intent": intent, "source": f"eval/{f.name}"})
    return out


def has_template_placeholder(text: str) -> bool:
    """Reject samples that still contain unfilled `{slot}` template
    placeholders.  These slipped in from v1.toml response templates
    when build_dialog_corpus_pack.py harvested template surfaces —
    they are training noise from the classifier's perspective."""
    import re
    return bool(re.search(r"\{[a-z_|]+\}", text))


def main() -> int:
    dialog = load_dialog_pack()
    adv = load_adversarial_pack()
    evalq = load_eval_queries()

    # Drop samples with unfilled placeholders.
    before = len(dialog)
    dialog = [s for s in dialog if not has_template_placeholder(s["text"])]
    print(f"[intent-pack] dialog: dropped {before - len(dialog)} placeholder samples", file=sys.stderr)

    print(f"[intent-pack] dialog corpus:      {len(dialog):>5}", file=sys.stderr)
    print(f"[intent-pack] adversarial pack:   {len(adv):>5}", file=sys.stderr)
    print(f"[intent-pack] eval queries:       {len(evalq):>5}", file=sys.stderr)

    # Deduplicate on (text, intent).
    seen: set[tuple[str, str]] = set()
    samples: list[dict] = []
    for s in dialog + adv + evalq:
        key = (s["text"].strip().lower(), s["intent"])
        if key in seen:
            continue
        seen.add(key)
        samples.append(s)

    print(f"[intent-pack] unique after dedup: {len(samples):>5}", file=sys.stderr)

    # Per-intent distribution.
    by_intent = Counter(s["intent"] for s in samples)
    print(f"\n[intent-pack] per-intent sample counts:", file=sys.stderr)
    for label in INTENT_LABELS:
        n = by_intent.get(label, 0)
        marker = "" if n >= 20 else "  ⚠ sparse" if n < 5 else ""
        print(f"  {n:5d}  {label}{marker}", file=sys.stderr)

    # Pack output.
    out = {
        "version": "v6.3-intent-classifier-2026-06-02",
        "name": "adam-intent-training-pack",
        "target_language": "kazakh",
        "script": "cyrillic",
        "intents": INTENT_LABELS,
        "sample_count": len(samples),
        "samples": samples,
    }
    OUT_PACK.parent.mkdir(parents=True, exist_ok=True)
    OUT_PACK.write_text(json.dumps(out, ensure_ascii=False, indent=2))
    print(f"\n[intent-pack] wrote {OUT_PACK}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
