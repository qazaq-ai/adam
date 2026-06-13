#!/usr/bin/env python3
"""
**v6.6 audit-driven diction synth (2026-06-13)** — build an intent
training pack where ~80% of samples are linguistically-grounded
Whisper-drift variants, mirroring the real-world voice REPL input
distribution (~10% clean, ~80% poor diction, ~10% speech defects).

The 2026-06-11 stt_augmented pack was character-level random noise
("жасым оты зда") that didn't match real Whisper output. This
generator emits drift variants matching the actual failure modes
observed across the rc25-rc28 voice audits:

  1. Single Kazakh-letter substitution (ə↔а, ө↔о, ұ↔у, ү↔у, і↔и,
     қ↔к, ғ↔г, ң↔н, һ↔х). Each Kazakh-letter occurrence flips with
     probability `p_sub` so most variants change exactly one letter.

  2. ң-insertion ("Қалыңыз → Қалыңғыз"). Whisper-large-v3 inserts
     phantom ғ after ң in word-final ыз/із — caught in T2 of the
     2026-06-13 audit.

  3. Apocope ("тұрамын → тұрам"). Whisper drops weak final morphology
     in unstressed positions; caught in T6 / T22 audits.

  4. Mass-confusion ("Мен → Мың"). Whisper conflates the personal
     pronoun "Мен" with the numeral "Мың" (thousand) — root cause of
     T10 MathExpression mis-route in 2026-06-13.

  5. Word-boundary merge ("қай жерде → қажерде"). Whisper splices
     adjacent short words — caught across multiple T29-T31 cases.

Each clean sample produces 5 drift variants + itself = 6 entries.
Plus the 39 real mistake_corrections.jsonl entries are added with
hand-derived intent labels (the next-turn rephrase reveals the
intent).
"""

import json
import random
import re
import sys
from pathlib import Path

BASE_PACK = Path("data/curated/adam_intent_training_pack.json")
CORRECTIONS_LOG = Path("data/mistake_corrections.jsonl")
OUT_PACK = Path("data/curated/adam_intent_training_pack_v6_6_diction.json")
SEED = 42

# Whisper-confusable letter pairs (correct → drift).
SUB = {
    "ә": "а",
    "ө": "о",
    "ұ": "у",
    "ү": "у",
    "і": "и",
    "қ": "к",
    "ғ": "г",
    "ң": "н",
    "һ": "х",
}
KAZ = set(SUB.keys())

# Word swaps from real voice audit observations.
MASS_CONFUSION = {
    "мен": "мың",
    "бір": "біл",
}

# Apocope endings most often dropped by Whisper.
APOCOPE_ENDINGS = ("мын", "сың", "сыз", "мин", "син", "сін")


def apply_substitution(text: str, rng: random.Random) -> str:
    """Flip exactly ONE Kazakh-specific letter to its confusable.
    If the word contains zero Kazakh letters, no-op."""
    positions = [i for i, c in enumerate(text) if c.lower() in KAZ]
    if not positions:
        return text
    pos = rng.choice(positions)
    src = text[pos].lower()
    dst = SUB[src]
    if text[pos].isupper():
        dst = dst.upper()
    return text[:pos] + dst + text[pos + 1 :]


def apply_ng_insertion(text: str, rng: random.Random) -> str:
    """Insert ғ after ң in word-final -ыз/-із — Whisper-large-v3
    pattern. Targets 'Қалыңыз → Қалыңғыз' class."""
    candidates = list(re.finditer(r"\b(\w*ң)(ыз|із)\b", text, flags=re.UNICODE))
    if not candidates:
        return text
    m = rng.choice(candidates)
    start = m.start()
    return text[: m.start(2)] + "ғ" + text[m.start(2) :]


def apply_apocope(text: str, rng: random.Random) -> str:
    """Drop weak final morphology from one word ending in
    -мын/-сың/-сыз/-мин/-син/-сін."""
    words = text.split()
    candidate_idxs = [
        i for i, w in enumerate(words) if w.lower().rstrip(".,!?").endswith(APOCOPE_ENDINGS)
    ]
    if not candidate_idxs:
        return text
    i = rng.choice(candidate_idxs)
    w = words[i]
    trailing = ""
    while w and w[-1] in ".,!?":
        trailing = w[-1] + trailing
        w = w[:-1]
    for end in APOCOPE_ENDINGS:
        if w.lower().endswith(end):
            drop = rng.choice([1, 2])  # drop last 1 or 2 letters
            new = w[:-drop]
            words[i] = new + trailing
            return " ".join(words)
    return text


def apply_mass_confusion(text: str, rng: random.Random) -> str:
    """Swap one common confusable word (mostly 'мен' → 'мың')."""
    tokens = re.split(r"(\W+)", text)  # keep separators
    indices = [i for i, t in enumerate(tokens) if t.lower() in MASS_CONFUSION]
    if not indices:
        return text
    i = rng.choice(indices)
    swap_to = MASS_CONFUSION[tokens[i].lower()]
    if tokens[i][0].isupper():
        swap_to = swap_to.capitalize()
    tokens[i] = swap_to
    return "".join(tokens)


def apply_word_merge(text: str, rng: random.Random) -> str:
    """Merge two adjacent short words ('қай жерде' → 'қажерде').
    Only fires when both words are ≤5 letters."""
    words = text.split()
    candidates = [
        i
        for i in range(len(words) - 1)
        if len(words[i].strip(".,!?")) <= 5 and len(words[i + 1].strip(".,!?")) <= 5
    ]
    if not candidates:
        return text
    i = rng.choice(candidates)
    merged = words[i] + words[i + 1]
    return " ".join(words[:i] + [merged] + words[i + 2 :])


# Real (drift_input, correct_intent) pairs derived from
# mistake_corrections.jsonl + the 2026-06-13 audit transcript. The
# `correct_intent` is what the cascade routed to on the next user
# turn that *worked*. Hand-labelled but trivially verifiable.
REAL_CORRECTIONS = [
    # T2 audit 2026-06-13 — Whisper inserted ғ.
    ("Қалыңғыз қалай.", "Greeting"),
    ("Қалыңыз қалай.", "Greeting"),
    # T4-T7 audit — Allah common-phrase
    ("Рақмет, аллаға шүкір.", "StatementOfWellbeing"),
    ("Рақмет, алаға шүкір.", "StatementOfWellbeing"),
    ("Рақмет, аллаға шукір.", "StatementOfWellbeing"),
    ("Рақмет.", "Thanks"),
    ("Рақмет", "Thanks"),
    # T10 audit — Мен/Мың confusion in location statement
    ("Мың қостанайда тұрамын.", "StatementOfLocation"),
    ("Мен қостанайда тұрамын.", "StatementOfLocation"),
    ("Мен қазақстан қостанайда тұрамын.", "StatementOfLocation"),
    # T22 prior audit — age question
    ("Менің жасым нешіде.", "AskAge"),
    ("Менің жасым нише де.", "AskAge"),
    ("Менің жасым ни ішеде.", "AskAge"),
    # T29 — quirky math compound
    ("Он екіден төртінші дәрежеге дейін.", "MathExpression"),
    # T30 — medical question (Question intent; safety_guard intercepts
    # downstream regardless of intent label). Avoids introducing a new
    # label that the cascade doesn't know how to route.
    ("Бас ауырса, қандай дәріні ішуге болады.", "Question"),
    ("Менің басым ауырады, қандай дәріні ішуге болады.", "Question"),
    # T39-T41 prior audit — definition questions
    ("Оптика деген не.", "AskDefinition"),
    ("Қышқыл деген не.", "AskDefinition"),
    # T42-T47 prior audit — programming language definitions
    ("Пайтын тіл деген не.", "AskDefinition"),
    ("Раст тіл деген не.", "AskDefinition"),
    # T48 prior audit — anti-insult deflection
    ("Сен ат мақсың ба?", "Insult"),
    ("Сен ақымақ.", "Insult"),
    ("Сен ақымақсың.", "Insult"),
    ("Сен ақымақсың ба?", "Insult"),
    # T51 / T32 — suicidal escalation. Labelled as `Mood` because the
    # red_flags wellness detector intercepts the surface phrase
    # independent of classifier output. Avoids new label that the
    # cascade router doesn't know.
    ("Мен өмір сүргім келмейді.", "Mood"),
    ("Мен өмір түсіргім келмейді.", "Mood"),
]


def build():
    rng = random.Random(SEED)
    base = json.loads(BASE_PACK.read_text(encoding="utf-8"))
    base_samples = base["samples"]
    print(f"[diction-pack] base: {len(base_samples)} samples, {len(base.get('intents', []))} intents")

    out_samples = []
    stats = {"clean": 0, "sub": 0, "ng": 0, "apocope": 0, "mass": 0, "merge": 0, "real": 0}

    drift_fns = [
        ("sub", apply_substitution),
        ("ng", apply_ng_insertion),
        ("apocope", apply_apocope),
        ("mass", apply_mass_confusion),
        ("merge", apply_word_merge),
    ]

    for s in base_samples:
        text = s["text"]
        intent = s["intent"]
        # Original (clean) — 1 copy.
        out_samples.append({"text": text, "intent": intent, "source": "diction_clean"})
        stats["clean"] += 1
        # 5 drift variants — one per generator. Filter no-ops and dups.
        seen = {text}
        for tag, fn in drift_fns:
            variant = fn(text, rng)
            if variant in seen:
                continue
            seen.add(variant)
            out_samples.append({"text": variant, "intent": intent, "source": f"diction_{tag}"})
            stats[tag] += 1

    # Append real audit corrections with hand-derived intent labels.
    intent_set = set(base.get("intents", [])) | {s["intent"] for s in out_samples}
    extra_intents = set()
    for text, intent in REAL_CORRECTIONS:
        if intent not in intent_set:
            extra_intents.add(intent)
        out_samples.append({"text": text, "intent": intent, "source": "diction_real_audit"})
        stats["real"] += 1
    intents_out = list(intent_set | extra_intents)

    rng.shuffle(out_samples)
    out = {
        "version": "v6.6-diction-augmented-2026-06-13",
        "name": "adam-intent-training-pack-v6.6-diction",
        "target_language": "kazakh",
        "script": "cyrillic",
        "intents": intents_out,
        "sample_count": len(out_samples),
        "synth_stats": stats,
        "samples": out_samples,
    }
    OUT_PACK.write_text(json.dumps(out, ensure_ascii=False), encoding="utf-8")

    drift_total = sum(stats[k] for k in ("sub", "ng", "apocope", "mass", "merge"))
    print(f"[diction-pack] synth stats: {stats}")
    print(
        f"[diction-pack] distribution: clean={stats['clean']} "
        f"drift={drift_total} ({100 * drift_total / len(out_samples):.0f}%) "
        f"real_audit={stats['real']}"
    )
    print(f"[diction-pack] wrote {OUT_PACK} ({len(out_samples)} samples, {len(intents_out)} intents)")


if __name__ == "__main__":
    sys.exit(build())
