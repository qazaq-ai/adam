#!/usr/bin/env python3
"""
**v6.7 generative pivot — bigger data pass (2026-06-13)** — mine
36k world_core statements into ~100k (question, answer) pairs by
generating multiple question framings per fact.

Pattern types:

  T1: "X — Y." → ("X кім?", "Y.")               (entity-definition)
  T2: "X — Y." → ("X туралы айтыңыз.", "X — Y.") (descriptive)
  T3: "X — Y." → ("X не?", "Y.")                 (predicate-only)

Filter: skip statements where X is too long (> 6 words) or Y is too
short (< 3 words). Skip pronouns and demonstratives as subjects.
"""

import json
import re
from collections import Counter
from pathlib import Path

WC_PATH = Path("data/curated/adam_training_world_core_qa_pack.json")
INTENT_PATH = Path("data/curated/adam_intent_training_pack.json")
DIALOG_PATH = Path("data/curated/adam_training_dialog_pack.json")
REAL_PATH = Path("data/curated/adam_real_voice_audit_pack.json")
TEMPLATES_TOML = Path("data/dialog/templates/v1.toml")

OUT = Path("data/curated/adam_finetune_text_pack_v4_big.json")

SKIP_SUBJECTS = {
    "мен", "сен", "ол", "біз", "сіз", "олар", "бұл", "осы", "сол",
    "мынау", "анау", "соны", "осыны", "мұны", "анық",
}


def is_good_subject(s: str) -> bool:
    s = s.strip()
    if not s or len(s.split()) > 6:
        return False
    if s.lower() in SKIP_SUBJECTS:
        return False
    # Skip if starts with a verb-like ending
    return True


def mine_world_core():
    p = json.loads(WC_PATH.read_text(encoding="utf-8"))
    samples = p["samples"]
    pairs = []
    skipped = 0
    for s in samples:
        text = s["text"].strip()
        # Pattern: "X — Y." (em dash)
        m = re.match(r"^([^—]+)\s+—\s+(.+?)\.?$", text)
        if not m:
            skipped += 1
            continue
        subject = m.group(1).strip()
        predicate = m.group(2).strip()
        if not is_good_subject(subject):
            skipped += 1
            continue
        if len(predicate.split()) < 3:
            skipped += 1
            continue
        domain = s.get("domain", "?")
        # T1: entity-definition
        pairs.append({
            "prompt": f"{subject} кім?",
            "response": f"{predicate}.",
            "source": f"world_core_t1:{domain}",
        })
        # T2: descriptive
        pairs.append({
            "prompt": f"{subject} туралы айтыңыз.",
            "response": f"{subject} — {predicate}.",
            "source": f"world_core_t2:{domain}",
        })
        # T3: predicate-only (variant)
        pairs.append({
            "prompt": f"{subject} не?",
            "response": f"{predicate}.",
            "source": f"world_core_t3:{domain}",
        })
    print(f"world_core: {len(pairs)} Q&A pairs from {len(samples)} statements (skipped {skipped})")
    return pairs


def parse_templates():
    raw = TEMPLATES_TOML.read_text(encoding="utf-8")
    families = {}
    for m in re.finditer(
        r'\[\[families\]\]\s*[\s\S]*?key\s*=\s*"([^"]+)"\s*templates\s*=\s*\[\s*([\s\S]*?)\]',
        raw,
    ):
        key = m.group(1)
        body = m.group(2)
        templates = re.findall(r'"((?:[^"\\]|\\.)*)"', body)
        families[key] = templates
    return families


def intent_to_family_key(intent: str) -> list:
    aliases = {
        "greeting": ["greeting.casual", "greeting.polite"],
        "askname": ["ask_name"],
        "askage": ["ask_age"],
        "asklocation": ["ask_location"],
        "statementofname": ["statement_of_name"],
        "statementofage": ["statement_of_age"],
        "statementoflocation": ["statement_of_location"],
        "statementofwellbeing": ["statement_of_wellbeing"],
        "farewell": ["farewell"],
        "thanks": ["thanks"],
        "apology": ["apology"],
        "affirmation": ["affirmation"],
        "negation": ["negation"],
        "introproposal": ["greeting.intro_proposal"],
        "askhowareyou": ["ask_how_are_you"],
        "askaboutsystem": ["ask_about_system"],
        "askdate": ["ask_date"],
        "asktime": ["ask_time"],
        "mathexpression": ["math_refusal"],
        "askdefinition": ["ask_about_topic"],
        "askabouttopic": ["ask_about_topic"],
        "insult": ["insult"],
        "compliment": ["compliment"],
        "mood": ["mood", "mood.positive", "mood.negative"],
        "inventoryquery": ["inventory_query", "ask_inventory"],
        "question": ["ask_about_topic"],
        "userdisagrees": ["disagreement_ack"],
        "request": ["request"],
        "codeRequest": ["code_request"],
    }
    return aliases.get(intent.lower(), [re.sub(r"(?<=[a-z])(?=[A-Z])", "_", intent).lower()])


def mine_intent_pack(families):
    p = json.loads(INTENT_PATH.read_text(encoding="utf-8"))
    pairs = []
    unmatched = Counter()
    for s in p["samples"]:
        user_text = s["text"]
        intent = s["intent"]
        keys = intent_to_family_key(intent)
        resp_templates = None
        for k in keys:
            if k in families:
                resp_templates = families[k]
                break
        if not resp_templates:
            unmatched[intent] += 1
            continue
        # Emit EVERY non-slot template, not just first — much richer data
        bare = [t for t in resp_templates if "{" not in t]
        if not bare:
            continue
        for resp in bare:
            pairs.append({
                "prompt": user_text,
                "response": resp,
                "source": f"intent:{intent}",
            })
    print(f"intent_pack: {len(pairs)} pairs ({len(unmatched)} unmatched intent labels)")
    return pairs


def mine_dialog_pack(families):
    p = json.loads(DIALOG_PATH.read_text(encoding="utf-8"))
    pairs = []
    for s in p["samples"]:
        user_text = s["text"]
        cat = s["category"]
        if cat in families:
            bare = [t for t in families[cat] if "{" not in t]
            for resp in bare:
                if resp.lower().strip(".,!? ") != user_text.lower().strip(".,!? "):
                    pairs.append({
                        "prompt": user_text,
                        "response": resp,
                        "source": f"dialog:{cat}",
                    })
    print(f"dialog_pack: {len(pairs)} pairs")
    return pairs


def mine_real_audit():
    if not REAL_PATH.exists():
        print("real_audit_pack: skipping (file not found)")
        return []
    p = json.loads(REAL_PATH.read_text(encoding="utf-8"))
    pairs = []
    # Upsample real audit ×10 — it's the gold standard for runtime distribution
    for s in p["samples"]:
        for _ in range(10):
            pairs.append({
                "prompt": s["prompt"],
                "response": s["response"],
                "source": "real_voice_audit",
            })
    print(f"real_audit (×10 upsample): {len(pairs)} pairs")
    return pairs


def build():
    families = parse_templates()
    all_pairs = []
    all_pairs.extend(mine_world_core())
    all_pairs.extend(mine_intent_pack(families))
    all_pairs.extend(mine_dialog_pack(families))
    all_pairs.extend(mine_real_audit())

    # Dedup by (prompt, response) — some intent/dialog variants overlap
    seen = set()
    deduped = []
    for p in all_pairs:
        k = (p["prompt"].strip().lower(), p["response"].strip().lower())
        if k in seen:
            continue
        seen.add(k)
        deduped.append(p)

    print(f"\nTotal after dedup: {len(deduped)} (from {len(all_pairs)} raw)")

    # Convert to text-pack format ("{prompt} →→ {response}")
    texts = []
    for i, pair in enumerate(deduped):
        prompt = pair["prompt"].strip()
        response = pair["response"].strip()
        if not prompt or not response or "{" in response or "}" in response:
            continue
        texts.append({"id": f"ft_{i:08d}", "text": f"{prompt} →→ {response}"})

    import random
    random.seed(0)
    random.shuffle(texts)

    out = {
        "version": "v6.7-finetune-big-2026-06-13",
        "name": "adam-generative-finetune-corpus-v4-big",
        "target_language": "kazakh",
        "script": "cyrillic",
        "sample_count": len(texts),
        "samples": texts,
    }
    OUT.write_text(json.dumps(out, ensure_ascii=False), encoding="utf-8")

    print(f"\nWrote {OUT}: {len(texts)} text pairs")
    print(f"Examples:")
    for s in texts[:5]:
        print(f'  «{s["text"]}»')


if __name__ == "__main__":
    build()
