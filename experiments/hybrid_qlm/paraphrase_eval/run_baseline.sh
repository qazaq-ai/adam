#!/usr/bin/env bash
# Phase paraphrase-eval — deterministic-only baseline.
#
# Runs every paraphrase from `paraphrase_coverage_eval.json`
# through `respond_full` and counts how many resolve to a
# response that contains the canonical's `expected_substring`.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/paraphrase_eval/baseline_results.md}"

if [[ ! -x ./target/release/respond_full ]]; then
    echo "ERROR: ./target/release/respond_full not built." >&2
    exit 1
fi

CASES=$(mktemp /tmp/paraphrase-cases.XXXXXX.json)
RAW=$(mktemp /tmp/paraphrase-raw.XXXXXX.txt)
trap 'rm -f "$CASES" "$RAW"' EXIT

python3 - "$PACK" "$CASES" <<'PYEOF'
import json, sys
pack_path, out_path = sys.argv[1], sys.argv[2]
with open(pack_path) as f:
    pack = json.load(f)
cases = []
for item in pack['items']:
    canonical = item['canonical']
    exp = item['expected_substring']
    for variant in [canonical] + item['paraphrases']:
        cases.append({
            'subject': 'paraphrase',
            'topic': canonical,
            'input': variant,
            'expected_response': exp,
            'was_accepted': True,
            'notes': 'canonical=' + canonical,
        })
out = {
    'version': pack['version'],
    'name': pack['name'] + '-cases',
    'description': pack['description'],
    'sample_count': len(cases),
    'cases': cases,
}
with open(out_path, 'w') as f:
    json.dump(out, f, ensure_ascii=False, indent=2)
print('expanded', len(pack['items']), 'canonicals →', len(cases), 'cases')
PYEOF

echo "[paraphrase-baseline] running cascade on $(python3 -c "import json; print(json.load(open('$CASES'))['sample_count'])") cases..."
./target/release/respond_full "$CASES" > "$RAW" 2>&1

mkdir -p "$(dirname "$OUT")"
python3 - "$PACK" "$CASES" "$RAW" "$OUT" <<'PYEOF'
import json, re, sys
from collections import defaultdict

pack_path, cases_path, raw_path, out_path = sys.argv[1:5]
with open(pack_path) as f:
    pack = json.load(f)
with open(cases_path) as f:
    cases = json.load(f)
with open(raw_path) as f:
    raw = f.read()

blocks = re.split(r'#(\d+)\s+', raw, flags=re.M)
predictions = {}
for i in range(1, len(blocks), 2):
    idx = int(blocks[i])
    m = re.search(r'predicted: «([^»]*)»', blocks[i+1])
    if m:
        predictions[idx] = m.group(1)

groups = defaultdict(lambda: {
    'canonical_hit': False,
    'paraphrases_hit': [],
    'paraphrases_total': 0,
    'paraphrases': [],
})

for idx, case in enumerate(cases['cases']):
    canonical = case['topic']
    expected = case['expected_response']
    variant = case['input']
    is_canonical = (variant == canonical)
    predicted = predictions.get(idx, '')
    hit = expected.lower() in predicted.lower()
    g = groups[canonical]
    if is_canonical:
        g['canonical_hit'] = hit
    else:
        g['paraphrases_total'] += 1
        if hit:
            g['paraphrases_hit'].append(variant)
        g['paraphrases'].append((variant, hit, predicted[:60]))

total_canon_hit = sum(1 for g in groups.values() if g['canonical_hit'])
total_para_hit = sum(len(g['paraphrases_hit']) for g in groups.values())
total_para = sum(g['paraphrases_total'] for g in groups.values())

lines = [
    '# Phase paraphrase-eval — deterministic baseline',
    '',
    '**Pack:** `' + pack_path + '`',
    '**Cascade:** deterministic only (no hybrid LM in loop)',
    '',
    '## Summary',
    '',
    f'- Canonicals hit: **{total_canon_hit} / {len(groups)}** ({100*total_canon_hit//len(groups)}%)',
    f'- Paraphrases hit: **{total_para_hit} / {total_para}** ({100*total_para_hit//total_para}%)',
    '',
    '## Per-canonical breakdown',
    '',
]
for canonical, g in groups.items():
    canon_mark = '✓' if g['canonical_hit'] else '✗'
    lines.append(f'### {canon_mark} `{canonical}`')
    lines.append('')
    lines.append(f"Paraphrases hit: **{len(g['paraphrases_hit'])} / {g['paraphrases_total']}**")
    lines.append('')
    for v, hit, pred in g['paraphrases']:
        mark = '✓' if hit else '✗'
        lines.append(f'- {mark} `{v}` → «{pred}»')
    lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print('wrote', out_path)
print(f'CANONICALS: {total_canon_hit}/{len(groups)} | PARAPHRASES: {total_para_hit}/{total_para}')
PYEOF
