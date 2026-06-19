#!/usr/bin/env python3
# export_curriculum_to_pack.py — read curriculum.db, emit a training
# pack JSON in the same shape as data/curated/adam_v6_8_step_b_corpus.json.
#
# Output format matches what the BPE encoder expects:
#   {"version": ..., "samples": [{"id": str, "text": "Q →→ A"}, ...]}
#
# Sequential ordering is preserved: samples are emitted in
# grade → subject → topic_order → subtopic_order → qa_order.
# That order matters for curriculum-learning style training.

from __future__ import annotations

import argparse
import json
import sqlite3
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DB_PATH = REPO_ROOT / "data" / "curriculum" / "curriculum.db"
OUT_PATH = REPO_ROOT / "data" / "curated" / "adam_curriculum_pack.json"
SEPARATOR = " →→ "


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", default=str(DB_PATH))
    parser.add_argument("--out", default=str(OUT_PATH))
    parser.add_argument("--upsample", type=int, default=1,
                        help="Repeat each pair N times to weight against larger corpora.")
    args = parser.parse_args()

    db_path = Path(args.db)
    if not db_path.exists():
        print(f"[export] missing DB: {db_path}", file=sys.stderr)
        return 1

    conn = sqlite3.connect(db_path)
    rows = conn.execute(
        """
        SELECT qa_id, question, answer, subject, grade, topic_order, subtopic_order, qa_order
        FROM curriculum
        ORDER BY grade, subject, topic_order, subtopic_order, qa_order
        """
    ).fetchall()
    conn.close()

    samples = []
    seen = set()
    for qa_id, q, a, subj, grade, t_ord, s_ord, q_ord in rows:
        text = f"{q.strip()}{SEPARATOR}{a.strip()}"
        for k in range(args.upsample):
            sid = qa_id if k == 0 else f"{qa_id}_u{k:02d}"
            samples.append({"id": sid, "text": text})
            seen.add(sid)

    out = {
        "version": "v6.8-curriculum-pack-2026-06-15",
        "name": "adam-curriculum-pack",
        "target_language": "kazakh",
        "script": "cyrillic+latin_chem_symbols",
        "sample_count": len(samples),
        "philosophy": (
            "Hand-authored Kazakh school curriculum Q&A in pedagogical order. "
            "Source of truth = data/curriculum/curriculum.db (SQLite). "
            "Each pair is grade-tagged + topic-ordered for curriculum-aligned "
            "training. Chemical symbols (H, O, Ag, NaCl, …) preserved verbatim "
            "— FST-lite mask in respond binary whitelists single-ASCII-letter "
            "tokens so these survive generation."
        ),
        "samples": samples,
    }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)

    print(f"[export] wrote {len(samples)} samples to {out_path}")
    print(f"[export]   size: {out_path.stat().st_size} bytes")
    if args.upsample > 1:
        print(f"[export]   upsample x{args.upsample} (unique pairs: {len(seen) // args.upsample})")
    print(f"[export]   first 3 samples:")
    for s in samples[:3]:
        print(f"     {s['id']}: {s['text'][:80]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
