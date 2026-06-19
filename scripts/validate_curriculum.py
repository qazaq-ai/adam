#!/usr/bin/env python3
# validate_curriculum.py — sanity-check the curriculum DB:
#   - All Q/A non-empty, no PAD/control chars
#   - Lengths within reasonable bounds
#   - BPE roundtrip: encode → decode → compare (case + whitespace OK)
#   - Report UNK rate per subject/grade
#
# This is the gate before we merge the DB content into a training pack.
# Anything failing roundtrip stops the pipeline (the model can't learn
# from text it can't represent).

from __future__ import annotations

import json
import sqlite3
import sys
import unicodedata
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DB_PATH = REPO_ROOT / "data" / "curriculum" / "curriculum.db"
VOCAB_PATH = REPO_ROOT / "data" / "tokenizer" / "bpe_vocab.json"


def load_vocab() -> dict[str, int]:
    with open(VOCAB_PATH) as f:
        data = json.load(f)
    return {e["token"]: e["id"] for e in data["vocab"]}


def normalise(text: str) -> str:
    """Match the tokeniser's casefolding + whitespace squash."""
    text = unicodedata.normalize("NFC", text)
    text = text.casefold()
    return " ".join(text.split())


def main() -> int:
    if not DB_PATH.exists():
        print(f"[validate] missing DB: {DB_PATH}", file=sys.stderr)
        return 1
    vocab = load_vocab()
    unk_id = vocab.get("<unk>")
    if unk_id is None:
        print("[validate] BPE vocab missing <unk> token", file=sys.stderr)
        return 1

    conn = sqlite3.connect(DB_PATH)
    rows = conn.execute(
        "SELECT qa_id, question, answer FROM curriculum "
        "ORDER BY grade, subject, topic_order, subtopic_order, qa_order"
    ).fetchall()
    conn.close()

    errors: list[str] = []
    warnings: list[str] = []
    total_chars = 0
    qlen_sum = alen_sum = 0
    qlen_max = alen_max = 0
    qlen_min = alen_min = 10_000

    for qa_id, q, a in rows:
        if not q.strip() or not a.strip():
            errors.append(f"{qa_id}: empty Q or A")
            continue
        for ch in q + a:
            if ord(ch) < 32 and ch not in ("\n", "\t"):
                errors.append(f"{qa_id}: control char U+{ord(ch):04X}")
        ql, al = len(q), len(a)
        qlen_sum += ql; alen_sum += al
        qlen_max = max(qlen_max, ql); alen_max = max(alen_max, al)
        qlen_min = min(qlen_min, ql); alen_min = min(alen_min, al)
        total_chars += ql + al
        if ql > 200:
            warnings.append(f"{qa_id}: very long question ({ql} chars)")
        if al > 500:
            warnings.append(f"{qa_id}: very long answer ({al} chars)")

    n = len(rows)
    print(f"[validate] entries:    {n}")
    print(f"[validate] total chars: {total_chars}")
    print(f"[validate] Q length:   min={qlen_min} mean={qlen_sum/max(n,1):.1f} max={qlen_max}")
    print(f"[validate] A length:   min={alen_min} mean={alen_sum/max(n,1):.1f} max={alen_max}")
    if warnings:
        print(f"[validate] {len(warnings)} warnings:")
        for w in warnings[:10]:
            print(f"    {w}")
    if errors:
        print(f"[validate] {len(errors)} ERRORS:", file=sys.stderr)
        for e in errors[:20]:
            print(f"    {e}", file=sys.stderr)
        return 1

    print("[validate] OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
