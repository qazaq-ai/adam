#!/usr/bin/env python3
"""
**v6.7 generative pivot iteration 3 (2026-06-13)** — balanced
fine-tune pack.

Lesson from v4_big (54k pairs, 95% world_core) → 7% eval regression:
*distribution matters more than volume*. This builder caps each
source category at a small ceiling and upsamples the gold-standard
real-audit data so the model learns conversational dialog as its
primary task.

Composition target:
  - intent_pack       : ALL variants (~4.6k after deduplication)
  - dialog_pack       : ALL variants (~1.5k)
  - world_core        : ONE prompt per domain (~65 total)
  - real_voice_audit  : ×20 upsample (~580)
  - synth_corrections : included if labels available

Total target: ~6500-7000 pairs with conversational majority.
"""

import json
import re
from collections import Counter, defaultdict
from pathlib import Path

WC_PATH = Path("data/curated/adam_training_world_core_qa_pack.json")
INTENT_PATH = Path("data/curated/adam_intent_training_pack.json")
DIALOG_PATH = Path("data/curated/adam_training_dialog_pack.json")
REAL_PATH = Path("data/curated/adam_real_voice_audit_pack.json")
TEMPLATES_TOML = Path("data/dialog/templates/v1.toml")

OUT = Path("data/curated/adam_finetune_text_pack_v5_balanced.json")


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


def mine_world_core_balanced(cap_per_domain=1):
    """One Q&A pair per world_core domain — broad coverage without
    overwhelming the conversational majority."""
    p = json.loads(WC_PATH.read_text(encoding="utf-8"))
    by_domain = defaultdict(list)
    for s in p["samples"]:
        text = s["text"].strip()
        m = re.match(r"^([^—]+)\s+—\s+(.+?)\.?$", text)
        if not m:
            continue
        subject = m.group(1).strip()
        predicate = m.group(2).strip()
        # Filter junk
        if any(c in subject for c in ".?!`{}"):
            continue
        if len(subject.split()) > 5 or len(subject.split()) < 1:
            continue
        if len(predicate.split()) < 3 or len(predicate.split()) > 30:
            continue
        by_domain[s.get("domain", "?")].append({
            "prompt": f"{subject} туралы айтыңыз.",
            "response": f"{subject} — {predicate}.",
            "source": f"world_core:{s.get('domain', '?')}",
        })

    out = []
    for domain, pairs in by_domain.items():
        out.extend(pairs[:cap_per_domain])
    print(f"world_core: {len(out)} pairs (one per domain × {cap_per_domain}, {len(by_domain)} domains)")
    return out


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
        bare = [t for t in resp_templates if "{" not in t]
        if not bare:
            continue
        for resp in bare:
            pairs.append({"prompt": user_text, "response": resp, "source": f"intent:{intent}"})
    print(f"intent_pack: {len(pairs)} pairs")
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
                    pairs.append({"prompt": user_text, "response": resp, "source": f"dialog:{cat}"})
    print(f"dialog_pack: {len(pairs)} pairs")
    return pairs


def mine_real_audit(upsample=20):
    if not REAL_PATH.exists():
        return []
    p = json.loads(REAL_PATH.read_text(encoding="utf-8"))
    pairs = []
    for s in p["samples"]:
        for _ in range(upsample):
            pairs.append({"prompt": s["prompt"], "response": s["response"], "source": "real_voice_audit"})
    print(f"real_audit ×{upsample}: {len(pairs)} pairs")
    return pairs


def build():
    families = parse_templates()
    all_pairs = []
    all_pairs.extend(mine_world_core_balanced(cap_per_domain=2))
    all_pairs.extend(mine_intent_pack(families))
    all_pairs.extend(mine_dialog_pack(families))
    all_pairs.extend(mine_real_audit(upsample=20))

    # Dedup
    seen = set()
    deduped = []
    for p in all_pairs:
        k = (p["prompt"].strip().lower(), p["response"].strip().lower())
        if k in seen or "{" in p["response"] or "}" in p["response"]:
            continue
        seen.add(k)
        deduped.append(p)

    src = Counter(p["source"].split(":")[0] for p in deduped)
    print(f"\nTotal after dedup: {len(deduped)} pairs")
    print(f"Distribution: {dict(src)}")

    # Compute percentages
    total = len(deduped)
    print("Source ratios:")
    for s, n in src.most_common():
        print(f"  {s}: {n:4d} ({100*n/total:.1f}%)")

    texts = []
    for i, pair in enumerate(deduped):
        prompt = pair["prompt"].strip()
        response = pair["response"].strip()
        if not prompt or not response:
            continue
        texts.append({"id": f"ft_{i:06d}", "text": f"{prompt} →→ {response}"})

    import random
    random.seed(42)
    random.shuffle(texts)

    out = {
        "version": "v6.7-finetune-balanced-2026-06-13",
        "name": "adam-generative-finetune-corpus-v5-balanced",
        "target_language": "kazakh",
        "script": "cyrillic",
        "sample_count": len(texts),
        "samples": texts,
    }
    OUT.write_text(json.dumps(out, ensure_ascii=False), encoding="utf-8")

    print(f"\nWrote {OUT}: {len(texts)} pairs")
    print(f"Examples:")
    for s in texts[:5]:
        print(f"  «{s['text']}»")


if __name__ == "__main__":
    build()
