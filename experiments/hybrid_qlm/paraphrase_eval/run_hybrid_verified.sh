#!/usr/bin/env bash
# Phase hybrid-wiring v3 — N-best + topic-preservation verifier.
#
# Adds a verifier gate between the LM paraphrase output and
# the cascade re-run.  For each LM-rewritten variant, we
# extract the «topic noun» from the ORIGINAL user query and
# check the paraphrase still mentions it (substring match
# tolerant of Kazakh case suffixes).  If the topic noun is
# gone, we REJECT the paraphrase and fall back to the
# deterministic miss — no junk propagates through.
#
# This is the canonical verifier shape from Codex's L4.5
# architectural contract — LM proposes, adam-side check
# grounds, ungrounded outputs are dropped.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/paraphrase_eval/hybrid_verified_results.md}"
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

# --- pass 1: deterministic (unchanged) ---
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
json.dump({'version': 'p1', 'name': 'p1', 'description': '',
           'sample_count': len(cases), 'cases': cases},
          open(sys.argv[2], 'w'), ensure_ascii=False)
PYEOF
echo "[hybrid-verified] pass 1 — deterministic cascade"
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
echo "[hybrid-verified] pass 1 misses: $MISS_COUNT"

# --- pass 2: 3 paraphrases per miss, then verifier-gate ---
echo "[hybrid-verified] pass 2 — N-best paraphrase + topic-preservation verifier"

python3 - "$MISSES_FILE" "$LM_MODEL" "$P2_DATA" <<'PYEOF'
import json, re, subprocess, sys

misses_path, model_path, out_path = sys.argv[1:4]
misses = json.load(open(misses_path))

PROMPTS = [
    ('synonym-swap', 0.2,
     'Қазақша сұрақтағы сөздерді синонимдермен ауыстыр.  '
     'Мағынасын сақта.  Қысқа жауап.\n\n'
     'Бастапқы: {q}\nСиноним нұсқа:'),
    ('rephrase-loose', 0.4,
     'Қазақша сұрақты басқаша сөздермен қайталап жаз.  '
     'Сұрақ түрін сақта.  Қысқа жауап.\n\n'
     'Бастапқы: {q}\nҚайталанған:'),
    ('canonical-form', 0.2,
     'Қазақша сұрақты неғұрлым қарапайым түрге ауыстыр.  '
     'Бір ғана сұрақ жаз.\n\n'
     'Бастапқы: {q}\nҚарапайым:'),
]

def call(prompt, temp):
    p = subprocess.run(
        ['llama-completion', '-m', model_path, '-ngl', '99',
         '--no-warmup', '-c', '512', '-n', '64', '--temp', str(temp),
         '-p', prompt],
        capture_output=True, text=True, timeout=60,
    )
    out, in_model, payload = p.stdout.splitlines(), False, []
    for line in out:
        t = line.strip()
        if t == 'model': in_model = True; continue
        if t.startswith('> EOF'): in_model = False; continue
        if in_model and t: payload.append(t)
    return '\n'.join(payload).strip()

# Topic-noun extractor — the heart of the verifier.  Takes
# the original variant, returns the longest substantive
# noun token (≥ 4 chars, alphabetic, not a Kazakh
# function-word).  A paraphrase MUST mention this noun (or
# its first 4-char prefix to allow inflectional drift) to
# clear the gate.
FUNCTION_WORDS = {
    'деген', 'дегеніміз', 'қандай', 'қандайды', 'қанша',
    'қалай', 'қайда', 'қай', 'неге', 'нені', 'кім',
    'мен', 'сен', 'сіз', 'осы', 'сол', 'бұл', 'мынау',
    'үшін', 'үшінде', 'туралы', 'арқылы', 'бірге',
    'болса', 'болғанда', 'болсын', 'еді', 'екен',
    'айтшы', 'айтыңыз', 'түсіндір', 'түсіндіріңіз',
    'жауап', 'сұрақ', 'нұсқа', 'не', 'жоқ', 'иә',
}
def topic_noun(text):
    tokens = re.findall(r'[А-Яа-яҚқҒғҢңӨөҰұҮүҺһІі]+', text)
    cands = [
        t for t in tokens
        if len(t) >= 4 and t.lower() not in FUNCTION_WORDS
    ]
    if not cands:
        return None
    # Pick the longest — most likely the content noun.
    return max(cands, key=len)

# Verifier — paraphrase must contain the first 4 chars of
# the original topic noun (case-insensitive).  This is
# generous: lets «мыс» → «мысты», «сынап» → «сынапты»
# clear, but rejects «сынап» → «сыныптың» (class) because
# stem «сыны» ≠ «сына».
def verify(original, paraphrase):
    noun = topic_noun(original)
    if not noun:
        # No identifiable topic noun → can't verify → pass
        # through (conservative).  Edge case.
        return True, 'no-noun (passed-through)'
    stem = noun[:4].lower()
    if stem in paraphrase.lower():
        return True, f'topic-noun «{noun}» (stem «{stem}») preserved'
    return False, f'topic-noun «{noun}» (stem «{stem}») LOST in paraphrase'

for m in misses:
    candidates = []
    for name, temp, tmpl in PROMPTS:
        paraphrase = call(tmpl.format(q=m['variant']), temp)
        first = next((ln for ln in paraphrase.splitlines() if ln.strip()), '')
        verified, reason = verify(m['variant'], first)
        candidates.append({
            'prompt': name, 'paraphrase': first,
            'verified': verified, 'verify_reason': reason,
        })
        mark = '✓' if verified else '✗'
        print(f"  [{name}] {mark} {m['variant']!r} → {first!r}  [{reason}]")
    m['candidates'] = candidates
json.dump(misses, open(out_path, 'w'), ensure_ascii=False)
PYEOF

# Build pass-2 cases ONLY for verifier-passed candidates.
python3 - "$P2_DATA" "$P2_CASES" <<'PYEOF'
import json, sys
misses = json.load(open(sys.argv[1]))
cases = []
case_idx = 0
for m in misses:
    for cand in m['candidates']:
        if not cand['paraphrase'] or not cand['verified']:
            cand['case_idx'] = None
            continue
        cand['case_idx'] = case_idx
        case_idx += 1
        cases.append({
            'subject': 'paraphrase-verified', 'topic': m['canonical'],
            'input': cand['paraphrase'],
            'expected_response': m['expected_substring'],
            'was_accepted': True,
            'notes': f"miss_idx={m['idx']} prompt={cand['prompt']}",
        })
json.dump({'version': 'p2', 'name': 'p2', 'description': '',
           'sample_count': len(cases), 'cases': cases},
          open(sys.argv[2], 'w'), ensure_ascii=False)
json.dump(misses, open(sys.argv[1], 'w'), ensure_ascii=False)
print(f'pass 2 cases (verifier-passed only): {len(cases)}')
PYEOF

P2_PASSED=$(python3 -c "import json; print(json.load(open('$P2_CASES'))['sample_count'])")
if [[ "$P2_PASSED" -gt 0 ]]; then
    ./target/release/respond_full "$P2_CASES" > "$P2_RAW" 2>&1
else
    : > "$P2_RAW"
fi

# Final scoring.
python3 - "$PACK" "$P2_DATA" "$P2_RAW" "$OUT" "$MISS_COUNT" <<'PYEOF'
import json, re, sys
from collections import defaultdict

pack_path, p2_data, p2_raw, out_path, miss_count = sys.argv[1:6]
miss_count = int(miss_count)
pack = json.load(open(pack_path))
misses = json.load(open(p2_data))

total_para = sum(len(item['paraphrases']) for item in pack['items'])

predictions = {}
try:
    raw = open(p2_raw).read()
    blocks = re.split(r'#(\d+)\s+', raw, flags=re.M)
    for i in range(1, len(blocks), 2):
        idx = int(blocks[i])
        m = re.search(r'predicted: «([^»]*)»', blocks[i+1])
        if m: predictions[idx] = m.group(1)
except Exception:
    pass

# Attach pass2 result to each candidate.  Score per miss:
# recovered iff ANY verifier-passed candidate hits.
rejected_count = 0
for m in misses:
    for c in m['candidates']:
        if not c['verified']:
            c['predicted'] = '(rejected by verifier)'
            c['hit'] = False
            rejected_count += 1
            continue
        if c.get('case_idx') is None:
            c['predicted'] = ''; c['hit'] = False; continue
        pred = predictions.get(c['case_idx'], '')
        c['predicted'] = pred
        c['hit'] = m['expected_substring'].lower() in pred.lower()
    m['recovered'] = any(c['hit'] for c in m['candidates'])

recoveries = [m for m in misses if m['recovered']]
no_recovery = [m for m in misses if not m['recovered']]
baseline_hits = total_para - miss_count
hybrid_hits = baseline_hits + len(recoveries)

# Count rejections that would have produced WRONG cascade
# output had we kept them (verifier value = caught hallucinations).
# Heuristic: a rejected paraphrase that has zero overlap of
# topic-noun stem with original was a topic-drift hallucination.
rejected_paraphrases = [
    c for m in misses for c in m['candidates']
    if not c['verified'] and c['paraphrase']
]

prompt_wins = defaultdict(int)
for m in misses:
    for c in m['candidates']:
        if c['hit']:
            prompt_wins[c['prompt']] += 1
            break

lines = [
    '# Phase hybrid-wiring v3 — N-best + topic-preservation verifier',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Path:** deterministic → on miss, 3 LM paraphrase candidates → topic-noun verifier gate → first-hit wins',
    '',
    '## Summary',
    '',
    f'- Deterministic baseline:       **{baseline_hits} / {total_para}** ({100*baseline_hits//total_para}%)',
    f'- Hybrid N-best + verifier:     **{hybrid_hits} / {total_para}** ({100*hybrid_hits//total_para}%)',
    f'- Recoveries:                   **{len(recoveries)} / {miss_count}** misses ({100*len(recoveries)//miss_count if miss_count else 0}% recovery rate)',
    f'- Verifier rejections:          **{rejected_count}** candidates blocked at the gate',
    f'- Lift:                         **+{100*len(recoveries)//total_para} pp**',
    '',
    f'### Verifier impact',
    '',
    f'- Of {len(rejected_paraphrases)} paraphrases that lost the topic noun, the verifier blocked **{len(rejected_paraphrases)}/{len(rejected_paraphrases)}** (100% by construction).',
    f'- The remaining `(3 × {miss_count}) - {rejected_count} = {3*miss_count - rejected_count}` candidates reached the cascade.',
    '',
    f'### Per-prompt wins',
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
        if not c['paraphrase']:
            continue
        if not c['verified']:
            lines.append(f"- ✗ [{c['prompt']}] `{c['paraphrase']}` → REJECTED: {c['verify_reason']}")
        else:
            mark = '✓' if c['hit'] else '✗'
            lines.append(f"- {mark} [{c['prompt']}] `{c['paraphrase']}` → «{c['predicted'][:50]}»")
    lines.append('')

lines.append(f'## No recovery ({len(no_recovery)})')
lines.append('')
for m in no_recovery:
    lines.append(f"### ✗ `{m['canonical']}`")
    lines.append('')
    lines.append(f"- variant: `{m['variant']}`")
    for c in m['candidates']:
        if not c['paraphrase']:
            continue
        if not c['verified']:
            lines.append(f"- ✗ [{c['prompt']}] `{c['paraphrase']}` → REJECTED: {c['verify_reason']}")
        else:
            lines.append(f"- ✗ [{c['prompt']}] `{c['paraphrase']}` → «{c['predicted'][:50]}»")
    lines.append('')

with open(out_path, 'w') as f:
    f.write('\n'.join(lines))
print(f'wrote {out_path}')
print(f'BASELINE: {baseline_hits}/{total_para} | HYBRID+VERIFIER: {hybrid_hits}/{total_para} | LIFT: +{len(recoveries)} ({100*len(recoveries)//total_para}pp) | REJECTED: {rejected_count}')
PYEOF
