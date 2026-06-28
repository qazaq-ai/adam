#!/usr/bin/env bash
# Phase hybrid-wiring — measure paraphrase coverage lift when
# the hybrid LM is engaged as a no-match-fallback paraphraser.
#
# Flow:
#   1. Run every variant through respond_full (deterministic).
#   2. For each MISS, call llama-completion via the same
#      prompt template propose_paraphrase uses to rewrite the
#      variant into a more canonical Kazakh shape.
#   3. Run the rewritten variant through respond_full.
#   4. If the second-pass response now contains
#      expected_substring, count it as a recovery.
#
# Hybrid path is opt-in via the standard ADAM_HYBRID_LM=1
# convention, but this script is itself the experimental
# wrapper — production binaries stay untouched.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/paraphrase_eval/hybrid_results.md}"
LM_MODEL="${LM_MODEL:-data/lm_models/gemma-3-4b-it-q4_k_m.gguf}"
BASELINE="${BASELINE:-experiments/hybrid_qlm/paraphrase_eval/baseline_results.md}"

if [[ ! -x ./target/release/respond_full ]]; then
    echo "ERROR: ./target/release/respond_full not built." >&2
    exit 1
fi
if ! command -v llama-completion >/dev/null 2>&1; then
    echo "ERROR: llama-completion missing.  brew install llama.cpp" >&2
    exit 1
fi
if [[ ! -f "$LM_MODEL" ]]; then
    echo "ERROR: model not found at $LM_MODEL" >&2
    exit 1
fi

P1_CASES=$(mktemp /tmp/p1-cases.XXXXXX.json)
P1_RAW=$(mktemp /tmp/p1-raw.XXXXXX.txt)
P2_CASES=$(mktemp /tmp/p2-cases.XXXXXX.json)
P2_RAW=$(mktemp /tmp/p2-raw.XXXXXX.txt)
trap 'rm -f "$P1_CASES" "$P1_RAW" "$P2_CASES" "$P2_RAW"' EXIT

# --- pass 1: deterministic baseline (canonicals + paraphrases) ---
python3 - "$PACK" "$P1_CASES" <<'PYEOF'
import json, sys
pack = json.load(open(sys.argv[1]))
cases = []
for item in pack['items']:
    for variant in [item['canonical']] + item['paraphrases']:
        cases.append({
            'subject': 'paraphrase',
            'topic': item['canonical'],
            'input': variant,
            'expected_response': item['expected_substring'],
            'was_accepted': True,
            'notes': 'pass=1 canonical=' + item['canonical'],
        })
out = {
    'version': pack['version'] + '-pass1',
    'name': pack['name'] + '-pass1',
    'description': 'pass 1 — deterministic',
    'sample_count': len(cases),
    'cases': cases,
}
json.dump(out, open(sys.argv[2], 'w'), ensure_ascii=False)
PYEOF

echo "[hybrid] pass 1 — deterministic cascade on $(python3 -c "import json; print(json.load(open('$P1_CASES'))['sample_count'])") cases"
./target/release/respond_full "$P1_CASES" > "$P1_RAW" 2>&1

# --- find misses ---
MISSES_FILE=$(mktemp /tmp/p1-misses.XXXXXX.json)
trap 'rm -f "$P1_CASES" "$P1_RAW" "$P2_CASES" "$P2_RAW" "$MISSES_FILE"' EXIT

python3 - "$P1_CASES" "$P1_RAW" "$MISSES_FILE" <<'PYEOF'
import json, re, sys
cases = json.load(open(sys.argv[1]))
raw = open(sys.argv[2]).read()
blocks = re.split(r'#(\d+)\s+', raw, flags=re.M)
predictions = {}
for i in range(1, len(blocks), 2):
    idx = int(blocks[i])
    m = re.search(r'predicted: «([^»]*)»', blocks[i+1])
    if m:
        predictions[idx] = m.group(1)

misses = []
for idx, c in enumerate(cases['cases']):
    pred = predictions.get(idx, '')
    if c['expected_response'].lower() not in pred.lower():
        # Track BOTH canonical misses and paraphrase misses,
        # but the hybrid path only attempts recovery on
        # paraphrase misses (canonicals are «the right shape»
        # by definition and don't need rewriting).
        if c['input'] != c['topic']:
            misses.append({
                'idx': idx,
                'variant': c['input'],
                'canonical': c['topic'],
                'expected_substring': c['expected_response'],
                'deterministic_predicted': pred,
            })
json.dump(misses, open(sys.argv[3], 'w'), ensure_ascii=False)
print(f'pass 1 misses on paraphrases: {len(misses)}')
PYEOF

MISS_COUNT=$(python3 -c "import json; print(len(json.load(open('$MISSES_FILE'))))")
echo "[hybrid] pass 1 misses on paraphrases: $MISS_COUNT"

# --- pass 2: paraphrase each miss via llama-completion + re-run cascade ---
P2_DATA=$(mktemp /tmp/p2-data.XXXXXX.json)
trap 'rm -f "$P1_CASES" "$P1_RAW" "$P2_CASES" "$P2_RAW" "$MISSES_FILE" "$P2_DATA"' EXIT

echo "[hybrid] pass 2 — calling propose_paraphrase on $MISS_COUNT misses (this takes ~2s × $MISS_COUNT ≈ $(( MISS_COUNT * 2 ))s)"

python3 - "$MISSES_FILE" "$LM_MODEL" "$P2_DATA" <<'PYEOF'
import json, subprocess, sys
misses_path, model_path, out_path = sys.argv[1:4]
misses = json.load(open(misses_path))

def propose_paraphrase(text):
    prompt = (
        "Қазақша сұрақты басқаша сөздермен қайталап жаз. "
        "Мағынасын сақта.  Қысқа жауап.\n\n"
        f"Бастапқы: {text}\n"
        "Қайталанған:"
    )
    p = subprocess.run(
        ['llama-completion', '-m', model_path, '-ngl', '99',
         '--no-warmup', '-c', '512', '-n', '64', '--temp', '0.2',
         '-p', prompt],
        capture_output=True, text=True, timeout=60,
    )
    out = p.stdout
    # Strip Gemma chat template — find «model» line, take
    # everything up to «> EOF».
    lines = out.splitlines()
    in_model = False
    payload = []
    for line in lines:
        t = line.strip()
        if t == 'model':
            in_model = True
            continue
        if t.startswith('> EOF'):
            in_model = False
            continue
        if in_model and t:
            payload.append(t)
    return '\n'.join(payload).strip()

for m in misses:
    paraphrased = propose_paraphrase(m['variant'])
    m['lm_paraphrased'] = paraphrased
    print(f"  {m['variant']!r} → {paraphrased!r}")

json.dump(misses, open(out_path, 'w'), ensure_ascii=False)
PYEOF

# --- pass 2 cascade run on the LM-rewritten variants ---
python3 - "$P2_DATA" "$P2_CASES" <<'PYEOF'
import json, sys
misses = json.load(open(sys.argv[1]))
cases = []
for m in misses:
    if not m.get('lm_paraphrased'):
        continue
    cases.append({
        'subject': 'paraphrase',
        'topic': m['canonical'],
        'input': m['lm_paraphrased'],
        'expected_response': m['expected_substring'],
        'was_accepted': True,
        'notes': f"pass=2 original={m['variant']}",
    })
out = {
    'version': 'pass2',
    'name': 'pass2',
    'description': 'pass 2 — LM-rewritten variants',
    'sample_count': len(cases),
    'cases': cases,
}
json.dump(out, open(sys.argv[2], 'w'), ensure_ascii=False)
print(f'pass 2 cases: {len(cases)}')
PYEOF

if [[ "$(python3 -c "import json; print(json.load(open('$P2_CASES'))['sample_count'])")" -gt 0 ]]; then
    ./target/release/respond_full "$P2_CASES" > "$P2_RAW" 2>&1
fi

# --- score: how many misses recovered? ---
python3 - "$PACK" "$P2_DATA" "$P2_CASES" "$P2_RAW" "$OUT" "$MISS_COUNT" <<'PYEOF'
import json, re, sys
pack_path, p2_data, p2_cases, p2_raw, out_path, miss_count = sys.argv[1:7]
miss_count = int(miss_count)
pack = json.load(open(pack_path))
misses = json.load(open(p2_data))
cases = json.load(open(p2_cases))

# Total paraphrase count from pack (denominator).
total_para = sum(len(item['paraphrases']) for item in pack['items'])

predictions = {}
try:
    raw = open(p2_raw).read()
    blocks = re.split(r'#(\d+)\s+', raw, flags=re.M)
    for i in range(1, len(blocks), 2):
        idx = int(blocks[i])
        m = re.search(r'predicted: «([^»]*)»', blocks[i+1])
        if m:
            predictions[idx] = m.group(1)
except FileNotFoundError:
    pass

# Map each miss to its pass-2 prediction.
recoveries = []
no_recovery = []
for idx, c in enumerate(cases['cases']):
    pred = predictions.get(idx, '')
    expected = c['expected_response']
    hit = expected.lower() in pred.lower()
    # Find the original miss this corresponds to.
    matching = [m for m in misses if m.get('lm_paraphrased') == c['input']]
    if matching:
        m = matching[0]
        m['pass2_predicted'] = pred
        m['pass2_hit'] = hit
        if hit:
            recoveries.append(m)
        else:
            no_recovery.append(m)

baseline_hits = total_para - miss_count
hybrid_hits = baseline_hits + len(recoveries)

lines = [
    '# Phase hybrid-wiring — paraphrase coverage with LM fallback',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Path:** deterministic → on miss, LM paraphrase → re-run cascade',
    '',
    '## Summary',
    '',
    f'- Deterministic baseline:    **{baseline_hits} / {total_para}** paraphrases hit ({100*baseline_hits//total_para}%)',
    f'- Hybrid (det + LM fallback):**{hybrid_hits} / {total_para}** paraphrases hit ({100*hybrid_hits//total_para}%)',
    f'- **Lift:** +{len(recoveries)} recoveries ({100*len(recoveries)//miss_count if miss_count else 0}% of {miss_count} misses)',
    '',
    f'### Promotion criterion (PROTOCOL.md §success)',
    '',
    f'Target: ≥ 20 pp lift on paraphrase coverage.',
    f'Lift this run: **+{100*len(recoveries)//total_para} pp** ({len(recoveries)} / {total_para}).',
    f'Status: {"✓ PROMOTABLE" if 100*len(recoveries)//total_para >= 20 else "◐ INSUFFICIENT — need ≥ " + str(total_para//5 - len(recoveries)) + " more recoveries"}',
    '',
    f'## Recoveries ({len(recoveries)})',
    '',
]
for m in recoveries:
    lines.append(f"### ✓ `{m['canonical']}`")
    lines.append('')
    lines.append(f"- variant: `{m['variant']}`")
    lines.append(f"- LM paraphrase: `{m['lm_paraphrased']}`")
    lines.append(f"- pass1: «{m['deterministic_predicted'][:60]}»")
    lines.append(f"- pass2: «{m['pass2_predicted'][:60]}»")
    lines.append('')

lines.append(f'## No recovery ({len(no_recovery)})')
lines.append('')
for m in no_recovery:
    lines.append(f"### ✗ `{m['canonical']}`")
    lines.append('')
    lines.append(f"- variant: `{m['variant']}`")
    lines.append(f"- LM paraphrase: `{m.get('lm_paraphrased', '(empty)')}`")
    lines.append(f"- pass2: «{m.get('pass2_predicted', '')[:60]}»")
    lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print(f'wrote {out_path}')
print(f'BASELINE: {baseline_hits}/{total_para} | HYBRID: {hybrid_hits}/{total_para} | LIFT: +{len(recoveries)} ({100*len(recoveries)//total_para}pp)')
PYEOF
