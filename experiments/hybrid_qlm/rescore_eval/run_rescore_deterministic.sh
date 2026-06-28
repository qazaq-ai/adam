#!/usr/bin/env bash
# Phase rescore-eval v2 — DETERMINISTIC vocab-membership
# rescorer.
#
# For each N-best item, score each candidate by counting how
# many of its tokens are present in the production vocabulary
# (world_core subjects/objects + curated high-frequency words).
# The highest-scoring candidate is picked.
#
# No LM in loop.  No ML.  Pure substring-membership lookup
# against the same word list `input_normalizer::shared_vocab`
# uses.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/rescore_eval/rescore_nbest_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/rescore_eval/deterministic_results.md}"

python3 - "$PACK" "$OUT" <<'PYEOF'
import json, re, sys, os
from collections import defaultdict

pack_path, out_path = sys.argv[1:3]
pack = json.load(open(pack_path))

# Build the same vocab `shared_vocab` builds:
#   1. CURATED_HIGH_FREQ (greetings, particles, common nouns)
#   2. Every distinct subject/object surface from world_core jsonl.
CURATED_HIGH_FREQ = {
    'сәлем', 'рақмет', 'оқасы жоқ', 'сау бол', 'бар бол',
    'иә', 'жоқ', 'мен', 'сен', 'сіз', 'ассалаумағалейкум',
    'уағалайкум', 'қош', 'хош',
    'керек', 'деген', 'менің', 'сенің', 'сіздің',
    'бұл', 'осы', 'сол', 'бірі', 'бірге',
    'онда', 'сонда', 'өзің', 'өзім',
}
vocab = set(CURATED_HIGH_FREQ)
import glob
for fp in glob.glob('data/world_core/*.jsonl'):
    with open(fp) as f:
        for line in f:
            try:
                e = json.loads(line)
            except Exception:
                continue
            for fact in e.get('facts', []):
                for k in ('subject', 'object'):
                    v = fact.get(k)
                    if isinstance(v, str) and v.strip():
                        vocab.add(v.strip().lower())

print(f'vocab size: {len(vocab)} entries')

def tokenize(text):
    return re.findall(r'[А-Яа-яҚқҒғҢңӨөҰұҮүҺһІі]+', text.lower())

def score_candidate(text, vocab):
    """
    Score = number of tokens whose lemma stem (first 4 chars)
    is a prefix of some vocab entry.  Generous enough to
    handle Kazakh inflectional drift («Қазақстанның» →
    «қазақстан» prefix → matches vocab entry «қазақстан»).
    """
    tokens = tokenize(text)
    if not tokens:
        return 0
    hits = 0
    for t in tokens:
        if len(t) < 4:
            # Function-word territory — skip the membership test
            # but don't penalise either.  This avoids the «бұл»
            # / «не» tokens distorting the score.
            continue
        stem = t[:4]
        for v in vocab:
            if v.startswith(stem):
                hits += 1
                break
    return hits

results = []
for item in pack['items']:
    scores = [score_candidate(c, vocab) for c in item['candidates']]
    # Tie-break: prefer candidate at correct_index when scores
    # tie, so we don't artificially inflate accuracy.  Actually
    # we use the FIRST candidate among ties to be honest.
    max_score = max(scores)
    picked = scores.index(max_score)
    correct = (picked == item['correct_index'])
    results.append({
        'topic': item['topic'],
        'defective': item['defective'],
        'clean_canonical': item['clean_canonical'],
        'candidates': item['candidates'],
        'scores': scores,
        'correct_index': item['correct_index'],
        'picked_index': picked,
        'correct': correct,
    })
    mark = '✓' if correct else '✗'
    print(f"  {mark} [{item['topic']:12s}] scores={scores} correct={item['correct_index']} picked={picked}")

total = len(results)
correct = sum(1 for r in results if r['correct'])

n_candidates = len(pack['items'][0]['candidates']) if pack['items'] else 5
random_baseline_pct = 100 // n_candidates

by_topic = defaultdict(lambda: [0, 0])
for r in results:
    by_topic[r['topic']][1] += 1
    if r['correct']: by_topic[r['topic']][0] += 1

lines = [
    '# Phase rescore-eval v2 — deterministic vocab-membership rescorer',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Method:** for each candidate, count tokens whose 4-char stem prefixes some `world_core` subject/object or curated high-frequency entry.  Highest count wins.',
    '**LM in loop:** NONE.',
    '',
    '## Summary',
    '',
    f'- Correct picks (deterministic):  **{correct} / {total}** ({100*correct//total}%)',
    f'- LM-4B rescore baseline:          6 / 20 (30 %)',
    f'- Random-baseline floor:           {random_baseline_pct}%',
    f'- Deterministic lift over random:  +{100*correct//total - random_baseline_pct} pp',
    f'- Deterministic lift over LM-4B:   +{100*correct//total - 30} pp',
    '',
    '### Per-topic breakdown',
    '',
]
for topic, (c, t) in sorted(by_topic.items()):
    lines.append(f'- `{topic}`: {c}/{t} ({100*c//t if t else 0}%)')
lines.append('')

lines.append(f'## Wrong picks ({total - correct})')
lines.append('')
for r in results:
    if r['correct']: continue
    lines.append(f"### ✗ `{r['defective']}` (topic: `{r['topic']}`)")
    lines.append('')
    lines.append(f"- clean canonical: `{r['clean_canonical']}`")
    lines.append(f"- picked (idx {r['picked_index']}): `{r['candidates'][r['picked_index']]}`")
    lines.append(f"- scores: {r['scores']}")
    lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print(f'\nwrote {out_path}')
print(f'DETERMINISTIC: {correct}/{total} ({100*correct//total}%) | LM-4B: 6/20 (30%) | RANDOM: {random_baseline_pct}%')
PYEOF
