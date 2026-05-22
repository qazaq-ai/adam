# Alpha-partner outreach candidates — v6.0.0-rc1

> **Source.** Maintainer's existing contacts + Codex 2026-05-18
> peer-review recommendations.
> **Sorted by**: probability of "yes" within 2 weeks of outreach,
> based on existing relationship and fit.
> **Send order**: top to bottom; pause after a positive response.

## Tier 1 — active relationship

### 1. КРУ им. А. Байтұрсынұлы (Костанай)

- **Why first.** Maintainer has an active contact (outreach
  письма sent 2026-05-18 per `project_kru_baitursynov_partnership`).
  Strong fit: Институт Байтұрсынұлы / Kazakh corpus tradition.
- **Contact.** Проф. Әбсадық Алмасбек Ахметұлы — проректор по
  учебной работе + зав.кафедрой филологии и практической
  лингвистики. Тройная роль = одна точка входа во все три двери
  (наука / ректорат / филфак).
- **Backup channel.** `info@ksu.edu.kz` с пометкой «Үшін: проф.
  Әбсадық А. А.»; AI SANA programme contact via
  https://ksu.edu.kz/ru/ai-sana/.
- **Hook.** Heritage angle — «Байтұрсынұлы formalised Kazakh
  grammar for school in 1914; adam formalises it for AI in 2026».
  2028 = 155-летие, окно подготовки 2026-2027.
- **Risk.** Academic timeline (2-4 weeks for formal response).

### 2. КГУ Smart Center / Qostanai IT Hub

- **Why second.** Maintainer is geographically co-located in
  Kostanay; same university as #1 but a different vertical
  (technical R&D / startup pilot environment instead of
  philological review).
- **Contact.** https://ksu.edu.kz/science-and-innovation/regionalnyj-smart-cent/ — public-facing inquiry email.
- **Hook.** "Local IT product, world-first deterministic Kazakh
  AI, deploy in your hub for 2 weeks free of charge".
- **Risk.** Possible overlap with #1 if the same administrator
  routes both queries to the same desk.

## Tier 2 — strong fit, cold outreach

### 3. А. Байтұрсынұлы Institute of Linguistics (Алматы)

- **Why.** Codex called this out as the strongest *linguistic*
  alpha (vs *educational* alpha for КРУ). The Institute runs the
  Kazakh National Corpus (https://qazcorpus.kz/?lang=en).
- **Contact.** https://tbi.kz/ — institutional inquiry; National
  Corpus team email TBD.
- **Hook.** "We need 30 minutes of a corpus linguist's review on
  2 000 lexicon-gap candidates that block our v6.0 GA. In
  exchange: cite the Institute in our preprint as the review
  authority."
- **Note.** This is more #3 (Lexicon V2 review) than #7 (alpha
  validation) — different criterion close.

### 4. РНПЦ «Дарын» (Astana)

- **Why.** Republican gifted-children school network.
  Codex-recommended. School-tutor positioning aligns with the
  «Qazaq AI Ұстаз» product spec.
- **Contact.** https://daryn.kz/ — administrative inquiry.
- **Hook.** "Russian / Kazakh / English-fluent students; we have
  a Kazakh-only deterministic tutor; want to test on a 5-tester
  cohort?"
- **Risk.** Federal-network bureaucracy — may take 4-6 weeks for
  formal sign-off.

### 5. Bilim Innovation Lyceums (BIL, ≈ 30 schools nationwide)

- **Why.** Codex-recommended. STEM / tri-lingual school
  network with a strong pilot culture. Schools have their own
  IT departments and can deploy quickly if a principal says yes.
- **Contact.** https://bil.edu.kz/?lang=en — central office.
- **Hook.** "Free 2-week Kazakh-language tutor pilot. We supply
  the deploy guide; you supply 5 testers across 5 schools and
  one feedback spreadsheet."
- **Risk.** Schools may want kk + ru bilingual; adam refuses
  ru-mode honestly, which may surprise testers.

## Tier 3 — research-flavoured fallbacks

### 6. Nazarbayev University Kazakh NLP team (Astana)

- **Why.** Codex-recommended. Authors of multiple Kazakh corpus /
  Kazakh BERT papers. Likely interested in adam as a deterministic
  baseline.
- **Risk.** Academic publication pressure may pull them to
  publish-then-collaborate rather than collaborate-then-publish.

### 7. ҚазҰУ кафедра қазақ тілі / филологии (Алматы)

- **Why.** Codex-recommended. Standard Kazakh-philology audience.
- **Risk.** Slower turnaround than Tier 1-2.

### 8. ENU филология / корпусная лингвистика (Astana)

### 9. IITU NLP researchers (Almaty)

### 10. Abai KazNPU филология (Almaty)

### 11. Shokan University corpus projects (Kokshetau)

## Outreach template

Send `pitch_kk.md` (Kazakh-speaking institution) or `pitch_ru.md`
(other) as the email body. Attach nothing. Wait 7 days for a
response; if nothing, send a one-line follow-up:

> «Сәлеметсіз бе! Өткен апта осы хатты жіберген едім (тіркемеде).
> Қызығу болса, бір жолмен жауап беріңіз. Жоқ болса, қайта
> мазаламаймын.»

## When you get a "yes"

1. Send `deploy_single_machine.md`, `alpha_validation_spec.md`,
   `feedback_form.md`, `privacy_data.md`, `risk_disclaimer.md`
   in one email.
2. Schedule a 30-minute video call (or in-person if Kostanay) to
   walk through the kit with their technical lead.
3. Sign `privacy_data.md` jointly.
4. Set kick-off date. The protocol is 14 days from there.
5. Weekly check-in (Friday) on the feedback spreadsheet.
6. End of week 2: PASS / FAIL review per
   `alpha_validation_spec.md`.

## Tracking

Add a row to `docs/alpha_reports/_outreach_log.csv`:
`date, candidate, status, response_date, outcome, notes`.
Status: `sent` / `acknowledged` / `accepted` / `declined` /
`stale (no response)`.

Aim: send to Tier-1 + Tier-2 (5 candidates) in week 1; expect
1-2 acceptances by week 3.
