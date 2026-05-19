# adam v6.0.0-rc1 — Alpha onboarding kit

> **Purpose.** Materials a partner organisation needs to evaluate
> adam's v6.0.0-rc1 preview release in 2 weeks, on a single
> M2-class machine, without prior NLP / Rust expertise.
>
> **Target partner.** A Kazakh-speaking organisation interested
> in a deterministic, offline-capable Kazakh tutor / morphology
> assistant. Schools, language institutes, IT-hubs, university
> philology departments, regional research centres. See
> [`alpha_outreach_candidates.md`](alpha_outreach_candidates.md).
>
> **Maintainer.** Daulet Baimurza · Qazna Technologies LLP ·
> `baimurza.daulet@gmail.com` · Kostanay, Kazakhstan.
>
> **Branch under test.** `experimental/agglutinative-neural`
> (will be tagged `v6.0.0-rc1` upon kit publication).

## What's in this kit

| File | Audience | Purpose |
|---|---|---|
| [`pitch_kk.md`](pitch_kk.md) | Decision-maker (Kazakh) | One-page partnership proposal |
| [`pitch_ru.md`](pitch_ru.md) | Decision-maker (Russian) | One-page partnership proposal |
| [`deploy_single_machine.md`](deploy_single_machine.md) | Technical lead | Copy-paste deploy guide, M2-class hardware |
| [`alpha_validation_spec.md`](alpha_validation_spec.md) | Both | 2-week protocol — what to test, success criteria, abort triggers |
| [`feedback_form.md`](feedback_form.md) | All testers | Per-turn rating template, plain spreadsheet |
| [`privacy_data.md`](privacy_data.md) | Compliance / IT | Data-handling agreement: zero phone-home, audit trail location |
| [`risk_disclaimer.md`](risk_disclaimer.md) | Decision-maker | Honest known-limitations list |
| [`alpha_outreach_candidates.md`](alpha_outreach_candidates.md) | Maintainer | List of 5 candidate organisations to approach first |

## How to use this kit (maintainer)

1. Send `pitch_kk.md` (or `pitch_ru.md`) by email to a candidate
   contact from `alpha_outreach_candidates.md`.
2. If they express interest, attach `deploy_single_machine.md`
   and `alpha_validation_spec.md` for their technical lead's review.
3. Sign the privacy agreement (`privacy_data.md`) together.
4. Share `risk_disclaimer.md` openly — there's no point starting
   an alpha that doesn't acknowledge the limitations.
5. Set a kick-off date; the protocol is 2 weeks from there.
6. Collect the feedback spreadsheet weekly; the success/abort
   criteria in `alpha_validation_spec.md` decide whether to
   continue.

## How to use this kit (partner)

Read `pitch_kk.md` (or `pitch_ru.md`) first — 5 minutes. If the
proposition is interesting, jump to `risk_disclaimer.md` (also
5 minutes) — this is what we can and cannot promise. If the
risk profile is acceptable, your technical lead reads
`deploy_single_machine.md` (~30 minutes including the actual
deploy) and `alpha_validation_spec.md` (15 minutes). Sign
`privacy_data.md` with the maintainer. Set a kick-off date.
