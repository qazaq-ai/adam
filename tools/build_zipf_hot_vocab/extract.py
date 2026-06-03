#!/usr/bin/env python3
"""
Phase 15g.B (2026-06-01) — build a Zipf-ranked hot-vocabulary from
the curated v6.3 corpus packs.

Drops ~3000 hand-picked entries from `adam-lexicon-curated` in favour
of true-frequency ranking from the real corpus the model already
ingested. Output is JSON consumed by `tools/voice_repl_v6_3` as the
runtime fuzzy-rescoring vocabulary.

Build-time tool (Python permitted per v6.3 directive); the runtime
pledge is pure-Rust and is preserved — Rust just `serde_json`-loads
the JSON this script writes.

Linguistic motivation (live REPL feedback 2026-06-01):
    Знание ~100 слов покрывает ~50% речи;
    ~1000 слов покрывает ~80% разговорного контекста;
    ~3000 слов покрывает ~90-95%.
                                                          — Zipf

So a 1000-entry frequency-ranked hot vocab covers the common path
without the manual `INTENT_VOCAB` curation that grew to 151 entries
ad-hoc.

Inputs:
    data/curated/cc100_kk_pack.json       (140 000 sentences)
    data/curated/wikipedia_kz_pack.json
    data/curated/abai_wikisource_pack.json

Output:
    data/voice_repl/zipf_hot_1000.json
        {
            "version": "v6.3-zipf-2026-06-01",
            "total_tokens": int,
            "distinct_tokens": int,
            "coverage_pct_top1000": float,
            "vocab": [
                {"word": "мен", "rank": 1, "count": 481392, "freq": 0.012},
                ...
            ]
        }

Run from repo root:
    python3 tools/build_zipf_hot_vocab/extract.py
"""

from __future__ import annotations
import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

SOURCES = [
    ROOT / "data/curated/cc100_kk_pack.json",
    ROOT / "data/curated/wikipedia_kz_pack.json",
    ROOT / "data/curated/abai_wikisource_pack.json",
]
OUT = ROOT / "data/voice_repl/zipf_hot_1000.json"

# Kazakh Cyrillic alphabet (incl. ә ғ қ ң ө ұ ү һ і)
KAZAKH_WORD_RE = re.compile(r"[А-Яа-яЁёӘәҒғҚқҢңӨөҰұҮүҺһІіЙй\-]+", re.UNICODE)

# Filter: pure-digit / single-letter junk, abbreviations the corpus
# is full of (they're real, but they aren't speech vocabulary).
def is_speech_token(tok: str) -> bool:
    if len(tok) < 2:
        return False
    if tok.startswith("-") or tok.endswith("-"):
        return False
    # Drop ALL-CAPS tokens (acronyms) — they shouldn't drive fuzzy
    # rescoring on a mic transcript.
    letters = [c for c in tok if c.isalpha()]
    if letters and all(c.isupper() for c in letters):
        return False
    return True

def main() -> int:
    counter: Counter[str] = Counter()
    total_in = 0
    for src in SOURCES:
        if not src.exists():
            print(f"[zipf] missing source (skipped): {src}", file=sys.stderr)
            continue
        with src.open() as f:
            doc = json.load(f)
        samples = doc.get("samples", [])
        print(f"[zipf] {src.name}: {len(samples)} samples")
        for s in samples:
            text = s.get("text", "")
            if not text:
                continue
            for raw in KAZAKH_WORD_RE.findall(text):
                w = raw.lower()
                if not is_speech_token(w):
                    continue
                counter[w] += 1
                total_in += 1

    distinct = len(counter)
    print(f"[zipf] total tokens: {total_in:,}")
    print(f"[zipf] distinct tokens: {distinct:,}")
    if distinct == 0:
        print("[zipf] no tokens extracted — bailing", file=sys.stderr)
        return 1

    top_n = 1000
    top = counter.most_common(top_n)

    sum_top = sum(c for _, c in top)
    coverage = sum_top / total_in if total_in else 0.0
    print(f"[zipf] top-{top_n} coverage: {coverage*100:.2f}%")

    out_vocab = []
    for rank, (word, count) in enumerate(top, start=1):
        out_vocab.append({
            "word": word,
            "rank": rank,
            "count": count,
            "freq": count / total_in if total_in else 0.0,
        })

    OUT.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "version": "v6.3-zipf-2026-06-01",
        "sources": [str(s.relative_to(ROOT)) for s in SOURCES if s.exists()],
        "total_tokens": total_in,
        "distinct_tokens": distinct,
        "top_n": top_n,
        "coverage_pct_top1000": round(coverage * 100, 2),
        "vocab": out_vocab,
    }
    with OUT.open("w") as f:
        json.dump(payload, f, ensure_ascii=False, indent=2)
    print(f"[zipf] wrote {OUT} ({len(out_vocab)} entries)")
    return 0

if __name__ == "__main__":
    sys.exit(main())
