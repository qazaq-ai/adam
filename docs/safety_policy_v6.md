# Safety policy v6 — informational over refusal

**Status.** Active from 2026-05-21. Supersedes the pre-v6
refusal-only policy on medical / legal / financial queries.
Self-harm and current-data paths are **unchanged** by this
revision and remain governed by their previous policies.

## Why we changed

Pre-v6, every high-stakes query (`SafetyCategory::{Medical,
Legal, Financial}`) reached the user as a refusal: "I am not a
doctor / lawyer / financial advisor; consult a qualified
specialist." For the Kazakh-speaking user — especially in rural
and remote regions where a qualified specialist may be hours
away — a bare "consult a specialist" is a **non-answer**. The
user learned nothing about their situation, got no triage
guidance, and was no better positioned to make a safe decision.

The policy revision: **deliver health / legal / financial
literacy, with explicit emergency triage on the first line and a
specialist-referral disclaimer on the last**. The user gets
information they can act on; safety is preserved by triage
guidance and the disclaimer; the deterministic-kernel guarantees
(no hallucination, audit logged) remain intact.

## What we deliver

For every medical / legal / financial query, the response is
**three sections**:

1. **Emergency triage — first line.** The phone number or
   immediate action goes at the very top of the response. Even
   a reader who stops after one sentence knows where to call:
   - Medical: `103` (жедел жәрдем — emergency medical).
   - Legal — active offence: `102` (police).
   - Legal — civil matter: "контакт адвоката / нотариуса".
   - Financial — suspected fraud: банкіңіздің клиент-сервисі +
     `102`.
2. **Informational content — middle.** General facts the user
   can act on responsibly:
   - Medical: symptoms, conditions, basic first aid, prevention,
     widely-known emergency interventions (e.g. chew 300 mg
     aspirin on suspected MI if no allergy).
   - Legal: applicable code articles (ҚР АК / ҚК), contract
     reqs, citizen rights, court procedure overview.
   - Financial: instrument types, risk profiles, diversification
     principles, tax basics, retirement saving structure.
3. **Specialist-referral disclaimer — last line.** A
   one-sentence reminder that the specific case requires a
   qualified human:
   - Medical: «нақты диагноз және ем — дәрігердің құзіреті».
   - Legal: «нақты іс үшін заңгер / адвокатпен ақылдасыңыз».
   - Financial: «нақты қаржылық шешім — білікті кеңесшінің ісі».

## What we do NOT deliver

The line between **information** (delivered) and **prescription**
(refused) is documented to prevent drift in future template
edits and seed-data work:

| ✅ informational                                    | ❌ prescriptive |
|----------------------------------------------------|------------------|
| "Симптомдар: қысып жатқандай ауыру, сол қолға таралу, ентігу." | "Сізде инфаркт." |
| "Жедел жәрдем 103. Аспирин 300 мг шайнауға болады." | "Нитроглицерин дозасы — 5 мг 6 сағат сайын." |
| "Инвестиция тәуекелдері: волатильность, ликвидность, регуляция жоқтығы." | "Bitcoin сатып алыңыз." |
| "Шарт реквизиттері: тараптар, нысана, мерзім, қол қою." | "Сіздің шартыңыз жарамсыз / Сіз сот шығасыз." |
| "ҚР АК 380-бабы шарт еркіндігін көрсетеді." | "Сіз бұл істі ұтасыз." |

**Принцип:** «учим думать», а не «решаем за пользователя».
Information lets the user prepare for the specialist appointment
(or recognise that one is needed). Prescription pretends to be
the specialist — which is exactly the failure mode the pre-v6
refusal policy was meant to prevent.

## Self-harm — separate policy

Self-harm queries (`SafetyCategory::SelfHarm`) **stay on the
crisis path**, not the informational path. Crisis intervention
has different ethics from health-literacy delivery:

- The first-line response is **always the 1415 Republican Trust
  Line + a human-care note**, not factual information about
  depression or suicide.
- Educational content about mental-health conditions
  (informational) is **only delivered if the user follow-up
  signals they have moved past the immediate crisis** — e.g.
  "айтшы депрессия дегеніміз не?" in a subsequent turn.
- The 1415 number, an invitation to contact a close person,
  and the reminder that adam is a language model (humans are
  ready to help) are non-negotiable.

This is the only category where the refusal-style template
remains the right answer.

## Current-data — separate policy

Queries about live data (`SafetyCategory::CurrentData` —
weather, exchange rates, news, prices) remain on the refusal
path because **adam has no live data feed** for most of these.
Open-Meteo gives us weather; everything else is honest
"I don't have live data; check an official source."

When a live feed lands (e.g. NBK exchange rate API), the
current-data path migrates to the informational policy at that
domain's pace. Until then, refusal remains the truthful answer.

## Defence / military — full information

`data/world_core/military_kz.jsonl` ships factual information
about Kazakhstan's defence structure, doctrine, equipment, and
international cooperation. No refusal path; the informational
policy applies even more directly because the audience for
defence questions (МО РК, КУС им. Байтұрсынұлы, students,
researchers) needs factual literacy, not refusals.

## Audit log

Every safety-class turn is **trace-logged**. The trace carries:

- The `SafetyCategory` slug that fired.
- The template family key picked (`safety_info.medical` vs
  `safety_refusal.self_harm` etc.).
- The rendered output.

This audit record is what we hand to a compliance reviewer
(AI Law 18.01.2026 high-risk-domain audit; МО РК pilot
contract; КУС partnership disclosure). The deterministic
trace + the policy doc above + the seed examples are the
"how the system handles X" evidence.

## Implementation notes

- Template files: `data/dialog/templates/v1.toml` — see the
  `safety_info.medical / legal / financial` and
  `safety_refusal.self_harm / current_data` family blocks.
- Planner routing: `crates/adam-dialog/src/planner.rs` — the
  `__safety_refusal__` slot still drives selection; the
  category slug maps to `safety_info.*` for medical / legal /
  financial and `safety_refusal.*` for self_harm / current_data.
- Detector: `crates/adam-dialog/src/discourse.rs`
  `detect_safety_topic` still labels the input; no detector
  changes were needed for the policy revision.
- Test corpora: `data/eval/adversarial_dialog_v1.json` srf_02
  through srf_05 expect substring `["дәрігер", "жедел жәрдем",
  "заңгер", "юрист", "қаржы", "кеңесші"]` — all of these
  appear in the new templates (triage line + disclaimer line)
  so the integration tests stay green.

## What changes in subsequent sprints

The infrastructure (template families, planner routing, policy
doc) is in place after this commit. The **data work** is
iterative:

- **Sprint +1:** expand `data/world_core/medical_kz.jsonl`
  with ~ 50 curated symptom / condition / first-aid facts.
  Same shape as `food_*` and `geography_*` JSONL files.
- **Sprint +2:** `data/world_core/legal_kz.jsonl` with ~ 50
  facts covering ҚР АК / ҚК / Конституция high-traffic
  articles.
- **Sprint +3:** `data/world_core/financial_kz.jsonl` with
  ~ 50 facts on banking, taxes, retirement, basic investment.
- **Sprint +4:** `data/world_core/defense_kz.jsonl` —
  rename / extend `military_kz.jsonl` with formal structure.

Each sprint ships with a `reviewed_at` date + reviewer name
in the JSONL row schema, so the audit log knows who signed off
on the factual content.
