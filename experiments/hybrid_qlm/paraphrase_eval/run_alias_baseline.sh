#!/usr/bin/env bash
# Phase v5-prep — DETERMINISTIC alias-table baseline.
#
# Asks the cheap-baseline question: how many of the 19 LM-
# fallback misses can be closed by a CURATED Kazakh
# synonym table + simple FST-style rewrite, with NO LM in
# the loop?
#
# This is the «before paying for LM» sanity check.  If the
# alias path closes a significant fraction, the LM is over-
# kill for those shapes and the hybrid experiment's value
# proposition shrinks.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/paraphrase_eval/alias_baseline_results.md}"

[[ -x ./target/release/respond_full ]] || { echo "build respond_full first" >&2; exit 1; }

P1_CASES=$(mktemp /tmp/p1.XXXXXX.json)
P1_RAW=$(mktemp /tmp/p1.XXXXXX.txt)
P2_CASES=$(mktemp /tmp/p2.XXXXXX.json)
P2_RAW=$(mktemp /tmp/p2.XXXXXX.txt)
trap 'rm -f "$P1_CASES" "$P1_RAW" "$P2_CASES" "$P2_RAW"' EXIT

# Pass 1 — deterministic baseline (same as every other harness).
python3 - "$PACK" "$P1_CASES" <<'PYEOF'
import json, sys
pack = json.load(open(sys.argv[1]))
cases = []
for item in pack['items']:
    for v in [item['canonical']] + item['paraphrases']:
        cases.append({'subject': 'paraphrase', 'topic': item['canonical'],
                      'input': v, 'expected_response': item['expected_substring'],
                      'was_accepted': True, 'notes': ''})
json.dump({'version': 'p1', 'name': 'p1', 'description': '',
           'sample_count': len(cases), 'cases': cases},
          open(sys.argv[2], 'w'), ensure_ascii=False)
PYEOF
echo "[alias-baseline] pass 1 — deterministic cascade"
./target/release/respond_full "$P1_CASES" > "$P1_RAW" 2>&1

# Find misses.
python3 - "$P1_CASES" "$P1_RAW" "$P2_CASES" "$OUT" <<'PYEOF'
import json, re, sys
from collections import defaultdict

cases_path, raw_path, p2_path, out_path = sys.argv[1:5]
cases = json.load(open(cases_path))
raw = open(raw_path).read()

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
        misses.append({'idx': idx, 'variant': c['input'], 'canonical': c['topic'],
                       'expected_substring': c['expected_response'],
                       'deterministic_predicted': pred})

print(f'pass 1 misses: {len(misses)}')

# --- CURATED alias rules ---
# Each rule = (regex_pattern, replacement_text).
# Order matters — longer / more specific patterns first.
# All patterns are case-insensitive; replacement preserves
# the input's surrounding tokens.
ALIAS_RULES = [
    # Chemistry «символ» / «белгі» → «таңба» (the cascade's
    # canonical noun for chemical symbol).
    (r'(?i)\bхимиялық\s+символ', 'химиялық таңба'),
    (r'(?i)\bсимвол(ын|ы|ың|ыңды)?\b', 'таңбасы'),
    (r'(?i)\bбелгі(сін|сі|сің|сіңіз|ңіз|сіңізді)?\b', 'таңбасы'),
    # «қалай белгілейді» / «қалай жазады» — «what symbol» shape.
    (r'(?i)\bқалай\s+белгілейді\b', 'таңбасы қандай'),
    (r'(?i)\bқалай\s+жазады\b', 'формуласы қандай'),
    # «дегенді түсіндір» / «деген сөздің мағынасы» / «бұл не»
    # — definition-query shape; the canonical cascade form is
    # «дегеніміз не».
    (r'(?i)\bдегенді\s+түсіндір[іңіздерше]*\b', 'дегеніміз не'),
    (r'(?i)\bдеген\s+сөздің\s+мағынасы\b', 'дегеніміз не'),
    (r'(?i)\bбұл\s+не\b', 'дегеніміз не'),
    (r'(?i)\bне\s+екенін\s+түсіндір', 'дегеніміз не'),
    # «сөзінің мағынасы» (синоним shape).
    (r'(?i)\bсөзінің\s+мағынасы\b', 'дегеніміз не'),
    # «X не?» (bare definition) → «X дегеніміз не?»
    # Carefully — only when X is a noun ≥ 5 chars followed
    # by «не?» / «не».  Avoids matching «не үшін керек».
    (r'(?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?', r'\1 дегеніміз не?'),
]

import re
def apply_aliases(text):
    out = text
    applied = []
    for pat, repl in ALIAS_RULES:
        new = re.sub(pat, repl, out)
        if new != out:
            applied.append(pat)
            out = new
    return out, applied

# Apply aliases to each miss.
for m in misses:
    rewritten, applied = apply_aliases(m['variant'])
    m['alias_rewritten'] = rewritten
    m['alias_rules_applied'] = applied
    m['alias_changed'] = (rewritten != m['variant'])

# Build pass 2 cases ONLY for variants the aliases changed.
p2_cases = []
case_idx = 0
for m in misses:
    if not m['alias_changed']:
        m['case_idx'] = None
        continue
    m['case_idx'] = case_idx
    case_idx += 1
    p2_cases.append({'subject': 'paraphrase-alias', 'topic': m['canonical'],
                     'input': m['alias_rewritten'],
                     'expected_response': m['expected_substring'],
                     'was_accepted': True,
                     'notes': f"original={m['variant']} rules={m['alias_rules_applied']}"})
json.dump({'version': 'p2', 'name': 'p2', 'description': '',
           'sample_count': len(p2_cases), 'cases': p2_cases},
          open(p2_path, 'w'), ensure_ascii=False)
# Save miss data — re-loaded by the scorer step.
json.dump(misses, open('/tmp/alias_misses.json', 'w'), ensure_ascii=False)
print(f'pass 2 cases (alias-rewritten only): {len(p2_cases)}')
PYEOF

P2_PASSED=$(python3 -c "import json; print(json.load(open('$P2_CASES'))['sample_count'])")
if [[ "$P2_PASSED" -gt 0 ]]; then
    ./target/release/respond_full "$P2_CASES" > "$P2_RAW" 2>&1
else
    : > "$P2_RAW"
fi

# Score.
python3 - "$PACK" "$P2_RAW" "$OUT" <<'PYEOF'
import json, re, sys

pack_path, p2_raw_path, out_path = sys.argv[1:4]
pack = json.load(open(pack_path))
misses = json.load(open('/tmp/alias_misses.json'))

predictions = {}
try:
    raw = open(p2_raw_path).read()
    blocks = re.split(r'#(\d+)\s+', raw, flags=re.M)
    for i in range(1, len(blocks), 2):
        idx = int(blocks[i])
        m = re.search(r'predicted: «([^»]*)»', blocks[i+1])
        if m: predictions[idx] = m.group(1)
except Exception:
    pass

for m in misses:
    if m.get('case_idx') is None:
        m['alias_predicted'] = ''
        m['alias_hit'] = False
        continue
    pred = predictions.get(m['case_idx'], '')
    m['alias_predicted'] = pred
    m['alias_hit'] = m['expected_substring'].lower() in pred.lower()

recoveries = [m for m in misses if m['alias_hit']]
no_recovery = [m for m in misses if not m['alias_hit']]
total_para = sum(len(item['paraphrases']) for item in pack['items'])
miss_count = len(misses)
baseline_hits = total_para - miss_count
alias_hits = baseline_hits + len(recoveries)

# Group recoveries by rule set used.
from collections import Counter
rules_used = Counter()
for m in recoveries:
    if m['alias_rules_applied']:
        rules_used[' + '.join(m['alias_rules_applied'])] += 1

lines = [
    '# Phase v5-prep — DETERMINISTIC alias-table baseline',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Method:** deterministic cascade → on miss, apply curated Kazakh synonym rules → re-run cascade',
    '**LM in loop:** NONE.',
    '',
    '## Summary',
    '',
    f'- Deterministic baseline:   **{baseline_hits} / {total_para}** ({100*baseline_hits//total_para}%)',
    f'- Alias-rewrite path:       **{alias_hits} / {total_para}** ({100*alias_hits//total_para}%)',
    f'- Recoveries (no LM):       **{len(recoveries)} / {miss_count}** misses ({100*len(recoveries)//miss_count if miss_count else 0}% recovery rate)',
    f'- Lift (no LM):             **+{100*len(recoveries)//total_para} pp**',
    '',
    '### Comparison vs hybrid variants',
    '',
    '| Variant                          | Recoveries | Recovery | Lift  | LM calls |',
    '| -------------------------------- | ---------- | -------- | ----- | -------- |',
    f'| deterministic alias (no LM)      | {len(recoveries)} / 19     | {100*len(recoveries)//19 if miss_count else 0} %    | +{100*len(recoveries)//total_para} pp | 0        |',
    '| v1 single-shot LM (no verifier)  | 4 / 19     | 21 %     | +4 pp | 19       |',
    '| v2 N-best LM (no verifier)       | 9 / 19     | 47 %     | +9 pp | 57       |',
    '| v3 N-best + naive verifier       | 5 / 19     | 26 %     | +5 pp | 57       |',
    '| v4 N-best + smart verifier       | 4 / 19     | 21 %     | +4 pp | 57       |',
    '',
    '### Rules that pulled their weight',
    '',
]
for rules, count in rules_used.most_common():
    lines.append(f'- `{rules}`: {count} recoveries')
lines.append('')

lines.append(f'## Recoveries ({len(recoveries)})')
lines.append('')
for m in recoveries:
    lines.append(f"### ✓ `{m['canonical']}`")
    lines.append('')
    lines.append(f"- variant:    `{m['variant']}`")
    lines.append(f"- rewritten:  `{m['alias_rewritten']}`")
    lines.append(f"- rules:      `{' + '.join(m['alias_rules_applied'])}`")
    lines.append(f"- cascade:    «{m['alias_predicted'][:60]}»")
    lines.append('')

lines.append(f'## No recovery ({len(no_recovery)})')
lines.append('')
for m in no_recovery:
    lines.append(f"### ✗ `{m['canonical']}`")
    lines.append('')
    lines.append(f"- variant:    `{m['variant']}`")
    if m['alias_changed']:
        lines.append(f"- rewritten:  `{m['alias_rewritten']}`")
        lines.append(f"- rules:      `{' + '.join(m['alias_rules_applied'])}`")
        lines.append(f"- cascade:    «{m['alias_predicted'][:60]}»")
    else:
        lines.append('- (no alias rule matched this variant)')
    lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print(f'wrote {out_path}')
print(f'BASELINE: {baseline_hits}/{total_para} | ALIAS-PATH: {alias_hits}/{total_para} | LIFT: +{len(recoveries)} ({100*len(recoveries)//total_para}pp) | NO LM CALLS')
PYEOF
