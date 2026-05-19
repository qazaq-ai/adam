# Alpha validation specification — adam v6.0.0-rc1

> **Purpose.** Concrete success / failure / abort criteria for the
> 2-week external-alpha deployment that closes v6.0 acceptance
> criterion #7 in [`docs/architecture_neural_v6.md`](../architecture_neural_v6.md) §9.
>
> **Audience.** Maintainer + partner technical lead. Read together
> before kick-off.

## Scope of the alpha

**In scope:** adam_chat text or voice REPL, deterministic kernel
default path. Optional L5.5 neural preview through `/neural` slash
command if the partner trained a checkpoint locally.

**Out of scope:**
- Production-traffic SLA (this is a research preview).
- Multi-user concurrency (REPL is single-tenant per process).
- Web / API / mobile UI (CLI only at rc1).
- Non-Kazakh languages.

## Duration & volume

| Parameter | Target |
|---|---|
| Duration | 14 calendar days, kick-off date set jointly |
| Real-user turns | ≥ 300 total (≈ 22/day; one tester × 30 turns × 10 days OR 10 testers × 30 turns) |
| Distinct testers | ≥ 3 (so we see inter-tester variance) |
| Topic categories | ≥ 5 (greetings, slot recall, factual, math, refusals, …) |

300 turns is the **minimum** for statistical signal. 500-1000 is
ideal but not required.

## Success criteria

The alpha is reported as **successful** if and only if all of:

1. **Acceptable-response rate ≥ 85 %** on in-scope turns, where
   "acceptable" = «понятный казахский ответ, не противоречит
   общеизвестному факту». Tester rates per turn on a {0, 1}
   spreadsheet (see [`feedback_form.md`](feedback_form.md)).
2. **Zero unsupported factual claims.** Every confident assertion
   adam makes about a named entity / number / date traces to a
   curated fact in `data/world_core/*.jsonl` or `facts.json`. If
   adam says «Қазақстанда X облыс бар» and X ≠ 17, that's a
   failure.
3. **Zero privacy / network violations.** Outbound network traffic
   limited to: (a) Open-Meteo when AskWeather fires and location
   is configured; (b) Apple TTS (local); (c) nothing else. The
   tester confirms via `Little Snitch` / `lsof -i` / `nettop` that
   no other connections fire.
4. **Latency ≤ 150 ms p50, ≤ 400 ms p99** per dialog turn on
   M2-class hardware (deterministic-path; neural preview turns
   excluded). Voice STT adds 1-3 s — that's whisper-cpp, not
   adam, and is reported separately.
5. **Verifier block rate < 5 %** of dialog turns. Higher block
   rate suggests either a known KG gap (acceptable, must be
   logged) or a verifier mis-calibration (must be fixed). Each
   block has an audit-trail entry in stderr; the tester pastes
   3-5 blocks into the feedback form.

## Abort triggers

Pause / cancel the alpha immediately if any of:

1. **Unsafe factual assertion** — adam says something that, if
   acted on, could harm a user (medical / legal / financial advice
   even when «just an example», or a privacy-violating recall).
2. **Data leak** — adam sends user-stated personal information
   (name / city / occupation) to any external service. Per
   `privacy_data.md` this never happens; if it does, that's a P0.
3. **Crash loop** — the binary segfaults or hangs more than once
   per 100 turns.
4. **Deterministic-path regression** — turns that worked in
   v5.32.x produce worse responses on v6.0.0-rc1. Use
   `assert_response_with_toml` regression tests from
   `crates/adam-dialog/tests/end_to_end.rs` as the baseline.
5. **Sustained p50 > 300 ms** under normal load (not first-cold-
   start; warm runs).
6. Partner explicitly reports: «adam-ды күнделікті жұмыста
   қолдану мүмкін емес» (cannot use adam in daily work) on core
   Kazakh morphology / school-tutor tasks.

If any trigger fires: pause, file an issue, fix or document, then
either resume or convert the alpha into a debugging engagement.

## Feedback collection

Per `feedback_form.md`. One row per turn. Columns:

  `turn_id, input, adam_response, latency_ms, rating_0_or_1,
   category, blocked_by_verifier_y_n, notes`

The tester fills 300 rows over 14 days. The maintainer aggregates
weekly + at end of alpha.

## After the alpha

1. **Acceptance review** — maintainer + partner read the
   feedback together, score each success criterion, declare the
   alpha PASS / FAIL.
2. **PASS** → v6.0.0 GA proceeds (along with Lexicon V2 sign-off
   and preprint acceptance). Partner is acknowledged in the
   release announcement.
3. **FAIL** → adam stays on `v6.0.0-rc1` indefinitely until the
   failing criterion is addressed by a new release candidate. The
   partner's feedback drives the next rc.
4. Either way: feedback CSV gets committed to
   `docs/alpha_reports/<partner>_<date>.md` (with
   anonymisation), so the next alpha has prior art.
