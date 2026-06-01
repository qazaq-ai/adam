#!/usr/bin/env python3
"""
Phase 15g.E (2026-06-01) — Convert curated world_core facts
into a training corpus of question–answer pairs for the
contextual LM.

User directive (2026-06-01): «Давай прекратим латать дыры,
сколько можно. Используя на все мощь нейронные сети с весами,
параметрами, токенами, как положенно с использованием
машинного обучения.»

Diagnosis: live REPL after phase 18.1 still missed factual
queries because adam-dialog's intent classifier substring-matched
«қандай таулар бар» to a definition lookup of «тау». The
root cause isn't the matcher — the LM has never seen «таулар
бар» / «көлдер бар» style queries as well-formed Kazakh. The
v2 dialog pack had ZERO factual Q&A pairs (the eval JSONs were
queries only — no canonical answers).

This script reads ALL 66 `data/world_core/*.jsonl` files (3 461
curated facts across geography, astronomy, biology, history,
abai works, etc.) and generates a Q&A training set:

  Each curated fact `kk` sentence stays as-is (the canonical
  statement). Then, for every entry, we synthesise 3–6 Q&A
  pairs:

    Q1: «<subject> туралы айт»            → A: <kk>
    Q2: «<subject> туралы не білесің»     → A: <kk>
    Q3: «<subject> кім / не»              → A: <kk> (for is_a facts)
    Q4: «<subject> қайда»                  → A: <kk> (for part_of / location facts)
    Q5: «<object>-да қандай <obj_class> бар» → A: <kk> (for plurals)

  Each (Q, A) is flattened into a single training sequence
  `<Q>. <A>.` so the LM learns the *conditional* structure
  «question — answer» at the level of token transitions.

Output: `data/curated/adam_training_world_core_qa_pack.json`
(text pack; the canonical Rust encode_corpus binary will
re-tokenise it through the same BPE-5188 vocab).

Run from repo root:
  python3 tools/build_world_core_qa_pack/build.py
"""

from __future__ import annotations
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORLD_CORE_DIR = ROOT / "data/world_core"
OUT_PACK = ROOT / "data/curated/adam_training_world_core_qa_pack.json"


def gen_qa(entry: dict) -> list[str]:
    """Return a list of textual training samples derived from one
    world_core entry. Each sample is `Q. A.` glued together so the
    LM learns the question→answer conditional."""
    kk = entry.get("kk", "").strip()
    facts = entry.get("facts", [])
    out: list[str] = []
    if not kk:
        return out

    # 1. Bare canonical statement (always emit).
    out.append(kk)

    if not facts:
        return out

    # Collect subject / object surfaces and predicates.
    subjects: set[str] = set()
    is_a_targets: set[str] = set()
    part_of_targets: set[str] = set()
    related_targets: set[str] = set()
    for f in facts:
        s = f.get("subject", "").strip().lower()
        p = f.get("predicate", "").strip()
        o = f.get("object", "").strip().lower()
        if s:
            subjects.add(s)
        if p == "is_a" and o:
            is_a_targets.add(o)
        elif p in ("part_of", "located_in", "is_in") and o:
            part_of_targets.add(o)
        elif p in ("related_to", "has_part", "contains") and o:
            related_targets.add(o)

    # 2. Per-subject Q&A pairs.
    for s in subjects:
        # «X туралы айт» / «X туралы не білесің» / «X деген не» / etc.
        out.append(f"{s} туралы айтшы. {kk}")
        out.append(f"{s} туралы айт. {kk}")
        out.append(f"{s} туралы не білесің. {kk}")
        out.append(f"{s} туралы не білесіз. {kk}")
        # «X кім / не»
        if is_a_targets:
            out.append(f"{s} кім. {kk}")
            out.append(f"{s} не. {kk}")
            out.append(f"{s} деген не. {kk}")
            out.append(f"{s} деген кім. {kk}")
        # «X қайда»
        if part_of_targets:
            out.append(f"{s} қайда. {kk}")
            out.append(f"{s} қайда орналасқан. {kk}")
            out.append(f"{s} қай жерде. {kk}")

    # 3. Per-object «what's inside» Q&A. For each `part_of` /
    # location relation, generate the inverse question
    # «<location>-да қандай <subject-class> бар» using the
    # canonical Kazakh locative suffix family.
    for obj in part_of_targets:
        # Plural form is approximate (Kazakh harmony) — just
        # tack «-да/-де» locative; the LM will learn the
        # equivalence class from many examples.
        loc = f"{obj}да"
        for s in list(subjects)[:1]:
            out.append(f"{loc} қандай {s} бар. {kk}")
            out.append(f"{loc} қандай {s}-лар бар. {kk}")

    # 4. Reverse «related_to» Q&A.
    for r in related_targets:
        out.append(f"{r} мен {next(iter(subjects), '')} туралы. {kk}")

    return out


def main() -> int:
    if not WORLD_CORE_DIR.is_dir():
        print(f"[wc-qa] missing: {WORLD_CORE_DIR}", file=sys.stderr)
        return 1

    seen: set[str] = set()
    samples: list[dict] = []
    facts_processed = 0
    for path in sorted(WORLD_CORE_DIR.iterdir()):
        if path.suffix != ".jsonl":
            continue
        with path.open() as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue
                facts_processed += 1
                for text in gen_qa(entry):
                    text = text.strip()
                    if not text:
                        continue
                    if text in seen:
                        continue
                    seen.add(text)
                    samples.append({
                        "id": f"wc_qa_{len(samples):06d}",
                        "domain": entry.get("domain", "world_core"),
                        "text": text,
                    })

    print(f"[wc-qa] facts processed: {facts_processed:,}")
    print(f"[wc-qa] dedup samples:   {len(samples):,}")

    out = {
        "version": "v6.3-world-core-qa-2026-06-01",
        "name": "adam-world-core-qa-pack",
        "target_language": "kazakh",
        "script": "cyrillic",
        "sample_count": len(samples),
        "samples": samples,
    }
    OUT_PACK.parent.mkdir(parents=True, exist_ok=True)
    OUT_PACK.write_text(json.dumps(out, ensure_ascii=False, indent=2))
    print(f"[wc-qa] wrote {OUT_PACK}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
