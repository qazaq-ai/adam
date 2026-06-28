#!/usr/bin/env bash
# Phase classify-eval — three classifiers head-to-head on the
# 30-item dialog-act labeled dataset.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/classify_eval/classify_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/classify_eval/results.md}"
LM_MODEL="${LM_MODEL:-data/lm_models/gemma-3-4b-it-q4_k_m.gguf}"

command -v llama-completion >/dev/null || { echo "brew install llama.cpp" >&2; exit 1; }
[[ -f "$LM_MODEL" ]] || { echo "model missing: $LM_MODEL" >&2; exit 1; }

python3 - "$PACK" "$LM_MODEL" "$OUT" <<'PYEOF'
import json, subprocess, sys, re
from collections import defaultdict, Counter

pack_path, model_path, out_path = sys.argv[1:4]
pack = json.load(open(pack_path))
LABELS = ['Greeting', 'FactualQuery', 'Clarify', 'RefusalSignal', 'Other']

# --- Deterministic keyword classifier ---
# Hand-curated rules, ordered by specificity.
GREETING_MARKERS = ['сәлем', 'сәлеметсіз', 'қайырлы таң', 'қайырлы күн',
                    'ассалаумағалейкум', 'ассалаумалейкум', 'ассалаумағалейкум',
                    'ассалаумалейкум', 'ассалаум алейкум', 'ассалаум алейкум',
                    'ассалаум алейкум', 'ассалам', 'жақсы кездестік', 'қош келдіңіз']
CLARIFY_MARKERS = ['түсінбедім', 'қайталашы', 'қайталаңыз', 'қалай дедіңіз',
                   'не дегенді білдіреді', 'түсіндірші', 'түсіндіріңіз',
                   'нақтырақ', 'басқаша айтып']
REFUSAL_MARKERS = ['жоқ.', 'жоқ,', 'жоқ ', 'нет.', 'нет,', 'нет ',
                   'дұрыс емес', 'бұл қате', 'қате']
FACTUAL_MARKERS = ['?', 'қандай', 'қанша', 'неше', 'қашан', 'қайда',
                   'қай', 'кім', 'не', 'қалай', 'неге', 'дегеніміз',
                   'деген не', 'неміз']

def classify_keyword(text):
    lower = text.lower()
    if any(m in lower for m in GREETING_MARKERS):
        return 'Greeting'
    if any(m in lower for m in CLARIFY_MARKERS):
        return 'Clarify'
    # Refusal — must be near the start / bare-rejection shape.
    if any(lower.startswith(m.rstrip(',. ')) for m in ['жоқ', 'нет']):
        return 'RefusalSignal'
    if any(m in lower for m in REFUSAL_MARKERS):
        return 'RefusalSignal'
    if any(m in lower for m in FACTUAL_MARKERS):
        return 'FactualQuery'
    return 'Other'

# --- LM classifier (mirrors adam_hybrid_llm::classify_dialog_act) ---
def classify_lm(text):
    prompt = (
        'Қазақ тіліндегі сөйлемді тек ОСЫНДАЙ бес санаттың '
        'бірімен таңбала: greeting, factual_query, clarify, '
        'refusal_signal, other.  Тек санатты жаз, басқа сөз қоспа.\n\n'
        f'Сөйлем: {text}\n'
        'Санат:'
    )
    p = subprocess.run(
        ['llama-completion', '-m', model_path, '-ngl', '99',
         '--no-warmup', '-c', '512', '-n', '16', '--temp', '0.2',
         '-p', prompt],
        capture_output=True, text=True, timeout=60,
    )
    out, in_model, payload = p.stdout.splitlines(), False, []
    for line in out:
        t = line.strip()
        if t == 'model': in_model = True; continue
        if t.startswith('> EOF'): in_model = False; continue
        if in_model and t: payload.append(t)
    lower = '\n'.join(payload).strip().lower()
    if 'greeting' in lower: return 'Greeting'
    if 'factual' in lower: return 'FactualQuery'
    if 'clarify' in lower: return 'Clarify'
    if 'refusal' in lower: return 'RefusalSignal'
    return 'Other'

results = []
for item in pack['items']:
    text = item['input']
    expected = item['label']
    kw = classify_keyword(text)
    lm = classify_lm(text)
    results.append({'input': text, 'expected': expected, 'keyword': kw, 'lm': lm,
                    'kw_correct': kw == expected, 'lm_correct': lm == expected})
    print(f"  [{expected:14s}] kw={kw:14s} lm={lm:14s} | {text!r}")

total = len(results)
kw_correct = sum(1 for r in results if r['kw_correct'])
lm_correct = sum(1 for r in results if r['lm_correct'])

# Per-class breakdown.
by_class = defaultdict(lambda: {'total': 0, 'kw': 0, 'lm': 0})
for r in results:
    by_class[r['expected']]['total'] += 1
    if r['kw_correct']: by_class[r['expected']]['kw'] += 1
    if r['lm_correct']: by_class[r['expected']]['lm'] += 1

random_pct = 100 // len(LABELS)

lines = [
    '# Phase classify-eval — dialog-act classifier comparison',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Items:** {total} labeled Kazakh queries across 5 classes (Greeting / FactualQuery / Clarify / RefusalSignal / Other).',
    '',
    '## Summary',
    '',
    f'- Keyword rules:  **{kw_correct} / {total}** ({100*kw_correct//total}%)',
    f'- LM-4B classify: **{lm_correct} / {total}** ({100*lm_correct//total}%)',
    f'- Random baseline: {random_pct}%',
    '',
    f'- LM lift over keyword: {100*lm_correct//total - 100*kw_correct//total:+d} pp',
    f'- Keyword lift over random: +{100*kw_correct//total - random_pct} pp',
    f'- LM lift over random:      +{100*lm_correct//total - random_pct} pp',
    '',
    '### Per-class breakdown',
    '',
    '| Class           | Total | Keyword     | LM-4B       |',
    '| --------------- | ----- | ----------- | ----------- |',
]
for cls in LABELS:
    d = by_class[cls]
    if d['total'] == 0:
        continue
    lines.append(f"| {cls:15s} | {d['total']:5d} | {d['kw']}/{d['total']} ({100*d['kw']//d['total']}%) | {d['lm']}/{d['total']} ({100*d['lm']//d['total']}%) |")
lines.append('')

lines.append('## Disagreements')
lines.append('')
for r in results:
    if r['kw_correct'] != r['lm_correct'] or not r['kw_correct']:
        kw_mark = '✓' if r['kw_correct'] else '✗'
        lm_mark = '✓' if r['lm_correct'] else '✗'
        lines.append(f"- expected `{r['expected']}` | kw {kw_mark} `{r['keyword']}` | lm {lm_mark} `{r['lm']}` | input: `{r['input']}`")
lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print(f'\nwrote {out_path}')
print(f'KEYWORD: {kw_correct}/{total} ({100*kw_correct//total}%) | LM-4B: {lm_correct}/{total} ({100*lm_correct//total}%) | RANDOM: {random_pct}%')
PYEOF
