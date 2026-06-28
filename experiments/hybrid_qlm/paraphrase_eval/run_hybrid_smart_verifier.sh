#!/usr/bin/env bash
# Phase hybrid-wiring v4 — N-best + SMART verifier.
#
# Same N-best paraphrase generation as v3, but the verifier
# distinguishes the «asked-ABOUT» noun (topic, usually
# genitive-marked) from the «asked-FOR» noun (predicate,
# the possessed thing).  Paraphrase passes if EITHER the
# topic stem OR the predicate stem survives — both are
# legitimate substitution targets.  Drift hallucinations
# («Жауап: Ақтөбе») still get blocked because they preserve
# neither.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PACK="${PACK:-experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json}"
OUT="${OUT:-experiments/hybrid_qlm/paraphrase_eval/hybrid_smart_results.md}"
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
        cases.append({'subject': 'paraphrase', 'topic': item['canonical'],
                      'input': variant, 'expected_response': item['expected_substring'],
                      'was_accepted': True, 'notes': 'pass=1'})
json.dump({'version': 'p1', 'name': 'p1', 'description': '',
           'sample_count': len(cases), 'cases': cases},
          open(sys.argv[2], 'w'), ensure_ascii=False)
PYEOF
echo "[hybrid-smart] pass 1 — deterministic cascade"
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
        misses.append({'idx': idx, 'variant': c['input'], 'canonical': c['topic'],
                       'expected_substring': c['expected_response'],
                       'deterministic_predicted': pred})
json.dump(misses, open(sys.argv[3], 'w'), ensure_ascii=False)
print(f'pass 1 misses: {len(misses)}')
PYEOF

MISS_COUNT=$(python3 -c "import json; print(len(json.load(open('$MISSES_FILE'))))")
echo "[hybrid-smart] pass 1 misses: $MISS_COUNT"

# --- pass 2: 3 paraphrases per miss + SMART verifier ---
echo "[hybrid-smart] pass 2 — N-best + smart verifier"

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

# Smart verifier ------------------------------------------------
# Extract BOTH the asked-ABOUT noun (topic) and the asked-FOR
# noun (predicate).  Kazakh genitive shape «X-NIŇ Y-I»:
#   X = topic (the thing being asked about)
#   Y = predicate (the thing being asked for about X)
# Verifier passes iff paraphrase contains the stem of EITHER.
# Drift hallucinations preserve neither and still get blocked.

GENITIVE_SUFFIXES = ('ның', 'нің', 'дың', 'дің', 'тың', 'тің')
FUNCTION_WORDS = {
    'деген', 'дегеніміз', 'қандай', 'қандайды', 'қанша',
    'қалай', 'қайда', 'қай', 'неге', 'нені', 'кім',
    'мен', 'сен', 'сіз', 'осы', 'сол', 'бұл', 'мынау',
    'үшін', 'үшінде', 'туралы', 'арқылы', 'бірге',
    'болса', 'болғанда', 'болсын', 'еді', 'екен',
    'айтшы', 'айтыңыз', 'түсіндір', 'түсіндіріңіз',
    'жауап', 'сұрақ', 'нұсқа', 'не', 'жоқ', 'иә',
}

def extract_topic_and_predicate(text):
    """
    Returns (topic_stem, predicate_stem) — both may be None.

    topic_stem: first 4 chars of the genitive-marked noun,
                lowercased.  «Темірдің» → «темі».
    predicate_stem: first 4 chars of the noun IMMEDIATELY
                following the genitive-marked one, OR the
                longest substantive non-function noun if no
                genitive shape is present.  «Темірдің
                символын» → «симв».
    """
    tokens = re.findall(r'[А-Яа-яҚқҒғҢңӨөҰұҮүҺһІі]+', text)
    topic_stem = None
    predicate_stem = None
    # Pass 1 — genitive shape detection.
    for i, t in enumerate(tokens):
        low = t.lower()
        for suf in GENITIVE_SUFFIXES:
            if low.endswith(suf) and len(low) > len(suf) + 1:
                stem = low[:-len(suf)]
                if len(stem) >= 3:
                    topic_stem = stem[:4]
                # Following token is the predicate.
                if i + 1 < len(tokens):
                    follow = tokens[i + 1].lower()
                    if len(follow) >= 4 and follow not in FUNCTION_WORDS:
                        predicate_stem = follow[:4]
                break
        if topic_stem:
            break
    # Pass 2 — fallback: longest substantive noun = topic.
    if not topic_stem:
        cands = [
            t.lower() for t in tokens
            if len(t) >= 4 and t.lower() not in FUNCTION_WORDS
        ]
        if cands:
            best = max(cands, key=len)
            topic_stem = best[:4]
    return topic_stem, predicate_stem

def verify(original, paraphrase):
    topic, predicate = extract_topic_and_predicate(original)
    para_lower = paraphrase.lower()
    if topic and topic in para_lower:
        return True, f'topic «{topic}» preserved'
    if predicate and predicate in para_lower:
        return True, f'predicate «{predicate}» preserved (topic «{topic}» substituted)'
    return False, f'NEITHER topic «{topic}» NOR predicate «{predicate}» in paraphrase'

# Run -----------------------------------------------------------
for m in misses:
    candidates = []
    for name, temp, tmpl in PROMPTS:
        paraphrase = call(tmpl.format(q=m['variant']), temp)
        first = next((ln for ln in paraphrase.splitlines() if ln.strip()), '')
        verified, reason = verify(m['variant'], first)
        candidates.append({'prompt': name, 'paraphrase': first,
                           'verified': verified, 'verify_reason': reason})
        mark = '✓' if verified else '✗'
        print(f"  [{name}] {mark} {m['variant']!r} → {first!r}  [{reason}]")
    m['candidates'] = candidates
json.dump(misses, open(out_path, 'w'), ensure_ascii=False)
PYEOF

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
        cases.append({'subject': 'paraphrase-smart', 'topic': m['canonical'],
                      'input': cand['paraphrase'],
                      'expected_response': m['expected_substring'],
                      'was_accepted': True,
                      'notes': f"miss_idx={m['idx']} prompt={cand['prompt']}"})
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

prompt_wins = defaultdict(int)
for m in misses:
    for c in m['candidates']:
        if c['hit']:
            prompt_wins[c['prompt']] += 1
            break

lines = [
    '# Phase hybrid-wiring v4 — N-best + SMART verifier',
    '',
    f'**Pack:** `{pack_path}`',
    f'**Verifier:** topic+predicate noun preservation (asked-ABOUT vs asked-FOR distinction)',
    '',
    '## Summary',
    '',
    f'- Deterministic baseline:        **{baseline_hits} / {total_para}** ({100*baseline_hits//total_para}%)',
    f'- Hybrid N-best + smart verifier:**{hybrid_hits} / {total_para}** ({100*hybrid_hits//total_para}%)',
    f'- Recoveries:                    **{len(recoveries)} / {miss_count}** misses ({100*len(recoveries)//miss_count if miss_count else 0}% recovery rate)',
    f'- Verifier rejections:           **{rejected_count}** candidates blocked at the gate',
    f'- Lift:                          **+{100*len(recoveries)//total_para} pp**',
    '',
    '### Comparison across hybrid versions',
    '',
    '| Variant                     | Recoveries | Recovery rate | Lift  | Rejections |',
    '| --------------------------- | ---------- | ------------- | ----- | ---------- |',
    '| v1 single-shot (no verifier)| 4 / 19     | 21 %          | +4 pp | 0          |',
    '| v2 N-best (no verifier)     | 9 / 19     | 47 %          | +9 pp | 0          |',
    '| v3 N-best + naive verifier  | 5 / 19     | 26 %          | +5 pp | 17         |',
    f'| v4 N-best + smart verifier  | {len(recoveries)} / {miss_count}     | {100*len(recoveries)//miss_count if miss_count else 0} %         | +{100*len(recoveries)//total_para} pp | {rejected_count}         |',
    '',
    '### Per-prompt wins',
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
            lines.append(f"- {mark} [{c['prompt']}] `{c['paraphrase']}` → «{c['predicted'][:50]}»  [{c['verify_reason']}]")
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
print(f'BASELINE: {baseline_hits}/{total_para} | HYBRID+SMART-VERIFIER: {hybrid_hits}/{total_para} | LIFT: +{len(recoveries)} ({100*len(recoveries)//total_para}pp) | REJECTED: {rejected_count}')
PYEOF
