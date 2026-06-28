#!/usr/bin/env bash
# Phase rescore-eval — does the hybrid LM's rescore_n_best
# pick the clean canonical from a Whisper-style N-best list?
#
# For each item in rescore_nbest_eval.json, calls
# `rescore_n_best` via the adam-hybrid-llm crate (env-gated
# ADAM_HYBRID_LM=1).  Compares LM-picked index to the
# `correct_index` (= position of the clean canonical in the
# shuffled candidate list).

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/rescore_eval/rescore_nbest_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/rescore_eval/baseline_results.md}"
LM_MODEL="${LM_MODEL:-data/lm_models/gemma-3-4b-it-q4_k_m.gguf}"

command -v llama-completion >/dev/null || { echo "brew install llama.cpp" >&2; exit 1; }
[[ -f "$LM_MODEL" ]] || { echo "model missing: $LM_MODEL" >&2; exit 1; }

# We invoke llama-completion directly here, NOT through the
# Rust crate, so the harness doesn't depend on the
# experimental crate being linked.  Same prompt template the
# crate uses (Kazakh «pick the most natural sentence's
# index»).

python3 - "$PACK" "$LM_MODEL" "$OUT" <<'PYEOF'
import json, re, subprocess, sys
from collections import defaultdict, Counter

pack_path, model_path, out_path = sys.argv[1:4]
pack = json.load(open(pack_path))

def rescore(candidates):
    numbered = '\n'.join(f'{i}. {c}' for i, c in enumerate(candidates))
    prompt = (
        'Қазақша сөйлемдер тізімі.  Ең табиғи және '
        'грамматикалық дұрыс сөйлемнің НӨМІРІН ҒАНА жаз.\n\n'
        f'{numbered}\n\n'
        'Ең дұрыс нөмір:'
    )
    p = subprocess.run(
        ['llama-completion', '-m', model_path, '-ngl', '99',
         '--no-warmup', '-c', '512', '-n', '24', '--temp', '0.2',
         '-p', prompt],
        capture_output=True, text=True, timeout=60,
    )
    out, in_model, payload = p.stdout.splitlines(), False, []
    for line in out:
        t = line.strip()
        if t == 'model': in_model = True; continue
        if t.startswith('> EOF'): in_model = False; continue
        if in_model and t: payload.append(t)
    cleaned = '\n'.join(payload).strip()
    # First digit in the response.
    digits = ''.join(c for c in cleaned if c.isdigit())
    if not digits:
        return None, cleaned
    return int(digits[0]), cleaned

results = []
for item in pack['items']:
    picked_idx, raw_response = rescore(item['candidates'])
    correct = (picked_idx == item['correct_index'])
    results.append({
        'topic': item['topic'],
        'defective': item['defective'],
        'clean_canonical': item['clean_canonical'],
        'candidates': item['candidates'],
        'correct_index': item['correct_index'],
        'picked_index': picked_idx,
        'picked_text': item['candidates'][picked_idx] if picked_idx is not None and picked_idx < len(item['candidates']) else None,
        'raw_response': raw_response,
        'correct': correct,
    })
    mark = '✓' if correct else '✗'
    detail = '(clean)'
    if not correct:
        if picked_idx is not None and picked_idx < len(item['candidates']):
            detail = '(picked ' + repr(item['candidates'][picked_idx]) + ')'
        else:
            detail = '(no parse)'
    print(f"  {mark} [{item['topic']:12s}] correct={item['correct_index']} picked={picked_idx} {detail}")

total = len(results)
correct = sum(1 for r in results if r['correct'])
by_topic = defaultdict(lambda: [0, 0])
for r in results:
    by_topic[r['topic']][1] += 1
    if r['correct']: by_topic[r['topic']][0] += 1

# Random-baseline reference — uniform over N candidates.
n_candidates = len(pack['items'][0]['candidates']) if pack['items'] else 5
random_baseline_pct = 100 // n_candidates

lines = [
    '# Phase rescore-eval — baseline (no fine-tune)',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Items:** {total} Whisper-style N-best probes (5 candidates each, exactly 1 clean canonical, 1 defective input, 3 same-topic distractors).',
    f'**Method:** prompt LM with numbered candidate list, parse first digit of response.',
    '',
    '## Summary',
    '',
    f'- Correct picks:           **{correct} / {total}** ({100*correct//total}%)',
    f'- Random-baseline floor:   {random_baseline_pct}% (uniform over {n_candidates} candidates)',
    f'- LM lift over random:     +{100*correct//total - random_baseline_pct} pp',
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
    if r['correct']:
        continue
    lines.append(f"### ✗ `{r['defective']}` (topic: `{r['topic']}`)")
    lines.append('')
    lines.append(f"- clean canonical: `{r['clean_canonical']}`")
    lines.append(f"- LM picked:       `{r['picked_text']}` (idx {r['picked_index']})")
    lines.append(f"- raw LM response: `{r['raw_response']!r}`")
    lines.append('')
    lines.append('Candidates:')
    for i, c in enumerate(r['candidates']):
        mark = '✓ CLEAN' if i == r['correct_index'] else '← picked' if i == r['picked_index'] else '       '
        lines.append(f"  - [{i}] `{c}` {mark}")
    lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print(f'\nwrote {out_path}')
print(f'RESCORE: {correct}/{total} correct ({100*correct//total}%) vs random {random_baseline_pct}%')
PYEOF
