#!/usr/bin/env bash
# Phase hybrid-wiring v2 — N-best paraphrases.
#
# Same shape as run_hybrid.sh but for each miss the LM is
# called THREE times with different prompts / temperatures
# to surface a small diversity of paraphrase candidates.
# Each candidate is run through the cascade; the FIRST one
# that hits counts as a recovery.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/paraphrase_eval/hybrid_nbest_results.md}"
LM_MODEL="${LM_MODEL:-data/lm_models/gemma-3-4b-it-q4_k_m.gguf}"

[[ -x ./target/release/respond_full ]] || { echo "build respond_full first" >&2; exit 1; }
command -v llama-completion >/dev/null || { echo "brew install llama.cpp" >&2; exit 1; }
[[ -f "$LM_MODEL" ]] || { echo "model missing: $LM_MODEL" >&2; exit 1; }

P1_CASES=$(mktemp /tmp/p1-cases.XXXXXX.json)
P1_RAW=$(mktemp /tmp/p1-raw.XXXXXX.txt)
MISSES_FILE=$(mktemp /tmp/p1-misses.XXXXXX.json)
P2_DATA=$(mktemp /tmp/p2-data.XXXXXX.json)
P2_CASES=$(mktemp /tmp/p2-cases.XXXXXX.json)
P2_RAW=$(mktemp /tmp/p2-raw.XXXXXX.txt)
trap 'rm -f "$P1_CASES" "$P1_RAW" "$MISSES_FILE" "$P2_DATA" "$P2_CASES" "$P2_RAW"' EXIT

# --- pass 1: deterministic ---
python3 - "$PACK" "$P1_CASES" <<'PYEOF'
import json, sys
pack = json.load(open(sys.argv[1]))
cases = []
for item in pack['items']:
    for variant in [item['canonical']] + item['paraphrases']:
        cases.append({
            'subject': 'paraphrase', 'topic': item['canonical'],
            'input': variant, 'expected_response': item['expected_substring'],
            'was_accepted': True, 'notes': 'pass=1 canonical=' + item['canonical'],
        })
json.dump({'version': pack['version'] + '-pass1', 'name': 'p1',
           'description': '', 'sample_count': len(cases), 'cases': cases},
          open(sys.argv[2], 'w'), ensure_ascii=False)
PYEOF
echo "[hybrid-nbest] pass 1 — deterministic cascade"
./target/release/respond_full "$P1_CASES" > "$P1_RAW" 2>&1

python3 - "$P1_CASES" "$P1_RAW" "$MISSES_FILE" <<'PYEOF'
import json, re, sys
cases = json.load(open(sys.argv[1]))
raw = open(sys.argv[2]).read()
blocks = re.split(r'#(\d+)\s+', raw, flags=re.M)
predictions = {}
for i in range(1, len(blocks), 2):
    idx = int(blocks[i])
    m = re.search(r'predicted: «([^»]*)»', blocks[i+1])
    if m: predictions[idx] = m.group(1)
misses = []
for idx, c in enumerate(cases['cases']):
    pred = predictions.get(idx, '')
    if c['expected_response'].lower() not in pred.lower() and c['input'] != c['topic']:
        misses.append({
            'idx': idx, 'variant': c['input'], 'canonical': c['topic'],
            'expected_substring': c['expected_response'], 'deterministic_predicted': pred,
        })
json.dump(misses, open(sys.argv[3], 'w'), ensure_ascii=False)
print(f'pass 1 misses: {len(misses)}')
PYEOF

MISS_COUNT=$(python3 -c "import json; print(len(json.load(open('$MISSES_FILE'))))")
echo "[hybrid-nbest] pass 1 misses: $MISS_COUNT"

# --- pass 2: 3 paraphrases per miss with different prompts ---
echo "[hybrid-nbest] pass 2 — generating 3 paraphrases per miss (~3 × $MISS_COUNT × 2s ≈ $(( MISS_COUNT * 6 ))s)"

python3 - "$MISSES_FILE" "$LM_MODEL" "$P2_DATA" <<'PYEOF'
import json, subprocess, sys
misses_path, model_path, out_path = sys.argv[1:4]
misses = json.load(open(misses_path))

# Three distinct angles per miss — diversity of attempts is
# the whole point of N-best.  Mixing prompt + temperature
# yields wider variant coverage than 3 calls of the same
# prompt with different seeds.
PROMPTS = [
    ('synonym-swap', 0.2,
     'Қазақша сұрақтағы сөздерді синонимдермен ауыстыр.  '
     'Мағынасын сақта, құрылымын өзгертпе.  Қысқа жауап.\n\n'
     'Бастапқы: {q}\n'
     'Синоним нұсқа:'),
    ('rephrase-loose', 0.4,
     'Қазақша сұрақты басқаша сөздермен қайталап жаз.  '
     'Сұрақ түрін сақта.  Мағынасы өзгермесін.  Қысқа жауап.\n\n'
     'Бастапқы: {q}\n'
     'Қайталанған:'),
    ('canonical-form', 0.2,
     'Қазақша сұрақты неғұрлым қарапайым, школьник де түсінетін '
     'түрге ауыстыр.  Бір ғана сұрақ жаз.\n\n'
     'Бастапқы: {q}\n'
     'Қарапайым:'),
]

def call(prompt, temp):
    p = subprocess.run(
        ['llama-completion', '-m', model_path, '-ngl', '99',
         '--no-warmup', '-c', '512', '-n', '64', '--temp', str(temp),
         '-p', prompt],
        capture_output=True, text=True, timeout=60,
    )
    out = p.stdout
    lines, in_model, payload = out.splitlines(), False, []
    for line in lines:
        t = line.strip()
        if t == 'model': in_model = True; continue
        if t.startswith('> EOF'): in_model = False; continue
        if in_model and t: payload.append(t)
    return '\n'.join(payload).strip()

for m in misses:
    candidates = []
    for name, temp, tmpl in PROMPTS:
        paraphrase = call(tmpl.format(q=m['variant']), temp)
        # Take first non-empty line as the candidate.
        first_line = next((ln for ln in paraphrase.splitlines() if ln.strip()), '')
        candidates.append({'prompt': name, 'paraphrase': first_line})
        print(f"  [{name}] {m['variant']!r} → {first_line!r}")
    m['candidates'] = candidates
json.dump(misses, open(out_path, 'w'), ensure_ascii=False)
PYEOF

# Build pass 2 cases — one per candidate per miss.
python3 - "$P2_DATA" "$P2_CASES" <<'PYEOF'
import json, sys
misses = json.load(open(sys.argv[1]))
cases = []
for m in misses:
    for ci, cand in enumerate(m['candidates']):
        if not cand['paraphrase']:
            continue
        cases.append({
            'subject': 'paraphrase-nbest', 'topic': m['canonical'],
            'input': cand['paraphrase'],
            'expected_response': m['expected_substring'],
            'was_accepted': True,
            'notes': f"pass=2 miss_idx={m['idx']} prompt={cand['prompt']}",
        })
json.dump({'version': 'p2', 'name': 'p2', 'description': '',
           'sample_count': len(cases), 'cases': cases},
          open(sys.argv[2], 'w'), ensure_ascii=False)
print(f'pass 2 cases: {len(cases)}')
PYEOF

./target/release/respond_full "$P2_CASES" > "$P2_RAW" 2>&1

# --- score: any candidate hits → recovery ---
python3 - "$PACK" "$P2_DATA" "$P2_CASES" "$P2_RAW" "$OUT" "$MISS_COUNT" <<'PYEOF'
import json, re, sys
pack_path, p2_data, p2_cases, p2_raw, out_path, miss_count = sys.argv[1:7]
miss_count = int(miss_count)
pack = json.load(open(pack_path))
misses = json.load(open(p2_data))
cases = json.load(open(p2_cases))

total_para = sum(len(item['paraphrases']) for item in pack['items'])

raw = open(p2_raw).read()
blocks = re.split(r'#(\d+)\s+', raw, flags=re.M)
predictions = {}
for i in range(1, len(blocks), 2):
    idx = int(blocks[i])
    m = re.search(r'predicted: «([^»]*)»', blocks[i+1])
    if m: predictions[idx] = m.group(1)

# Attach pass2 prediction to each candidate, then count
# recovery per miss (ANY candidate hits → recovered).
case_idx = 0
for m in misses:
    for cand in m['candidates']:
        if not cand['paraphrase']:
            cand['predicted'] = ''
            cand['hit'] = False
            continue
        pred = predictions.get(case_idx, '')
        cand['predicted'] = pred
        cand['hit'] = m['expected_substring'].lower() in pred.lower()
        case_idx += 1
    m['recovered'] = any(c['hit'] for c in m['candidates'])

recoveries = [m for m in misses if m['recovered']]
baseline_hits = total_para - miss_count
hybrid_hits = baseline_hits + len(recoveries)

# Per-prompt recovery breakdown (which prompt strategy
# pulled its weight).
from collections import defaultdict
prompt_wins = defaultdict(int)
for m in misses:
    for c in m['candidates']:
        if c['hit']:
            prompt_wins[c['prompt']] += 1
            break  # only count the first hit per miss

lines = [
    '# Phase hybrid-wiring v2 — N-best paraphrases',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Path:** deterministic → on miss, 3 LM paraphrase candidates × cascade → first-hit wins',
    '',
    '## Summary',
    '',
    f'- Deterministic baseline: **{baseline_hits} / {total_para}** ({100*baseline_hits//total_para}%)',
    f'- Hybrid N-best:          **{hybrid_hits} / {total_para}** ({100*hybrid_hits//total_para}%)',
    f'- Recoveries:             **{len(recoveries)} / {miss_count}** misses ({100*len(recoveries)//miss_count if miss_count else 0}% recovery rate)',
    f'- Lift:                   **+{100*len(recoveries)//total_para} pp**',
    '',
    '### Promotion criterion (PROTOCOL.md §success)',
    '',
    f'Target: ≥ 20 pp lift.  Status: {"✓ PROMOTABLE" if 100*len(recoveries)//total_para >= 20 else "◐ INSUFFICIENT — need ≥ " + str(total_para//5 - len(recoveries)) + " more recoveries"}',
    '',
    '### Per-prompt wins (which strategy pulled its weight)',
    '',
]
for name in ['synonym-swap', 'rephrase-loose', 'canonical-form']:
    lines.append(f'- `{name}`: {prompt_wins[name]} recoveries')
lines.append('')

lines.append(f'## Recoveries ({len(recoveries)})')
lines.append('')
for m in recoveries:
    lines.append(f"### ✓ `{m['canonical']}`")
    lines.append('')
    lines.append(f"- variant: `{m['variant']}`")
    for c in m['candidates']:
        mark = '✓' if c['hit'] else '✗'
        lines.append(f"- {mark} [{c['prompt']}] `{c['paraphrase']}` → «{c['predicted'][:50]}»")
    lines.append('')

no_recovery = [m for m in misses if not m['recovered']]
lines.append(f'## No recovery ({len(no_recovery)})')
lines.append('')
for m in no_recovery:
    lines.append(f"### ✗ `{m['canonical']}`")
    lines.append('')
    lines.append(f"- variant: `{m['variant']}`")
    for c in m['candidates']:
        lines.append(f"- ✗ [{c['prompt']}] `{c['paraphrase']}` → «{c['predicted'][:50]}»")
    lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print(f'wrote {out_path}')
print(f'BASELINE: {baseline_hits}/{total_para} | HYBRID-NBEST: {hybrid_hits}/{total_para} | LIFT: +{len(recoveries)} ({100*len(recoveries)//total_para}pp)')
PYEOF
