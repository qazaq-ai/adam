# Alpha feedback form — adam v6.0.0-rc1

> **Format.** One row per dialog turn. Plain CSV / spreadsheet.
> No custom tooling required. Google Sheets and LibreOffice Calc
> both work.
>
> **Target volume.** 300 rows over 14 days (≈ 22/day).

## Columns

| Column | Type | Required | Description |
|---|---|---|---|
| `turn_id` | int | yes | Sequential, 1..N. Easy to pick a specific row in discussion. |
| `tester_id` | string | yes | Free-form (e.g. "Aigerim", "Daulet", "T01"). Lets us see inter-tester variance. |
| `date` | YYYY-MM-DD | yes | Calendar date, partner's local time. |
| `category` | enum | yes | One of: `greeting`, `slot_recall`, `factual`, `math`, `geometry`, `weather`, `date_time`, `creator`, `self_intro`, `school_subject`, `industry`, `refusal_expected`, `farewell`, `other`. |
| `input` | string | yes | What the tester typed / said. For voice REPL: what Whisper transcribed; paste verbatim. |
| `adam_response` | string | yes | What adam answered. Copy from stdout. |
| `latency_ms` | int | optional | Per-turn latency. adam doesn't print this by default; the tester can run `time` once a session to spot-check. |
| `rating` | {0, 1} | yes | 1 if «понятный казахский ответ, не противоречит факту»; 0 otherwise. Binary on purpose: gut check, not multi-axis scoring. |
| `blocked_by_verifier` | y / n | yes | If adam refused via L6 verifier (NonKazakhScript / UnkSurface / Ungrounded / FstRoundTripFailed), set `y`. Otherwise `n`. Look for `[verifier]` line in stderr. |
| `expected_refusal` | y / n | yes | `y` if the tester DELIBERATELY tested a known-refusal case (medical advice, current news, etc.). `n` for normal dialog. |
| `severity` | enum | optional | `none` / `minor` / `major` / `blocker`. `blocker` triggers an alpha abort review per `alpha_validation_spec.md`. |
| `notes` | string | optional | Free-form. «Ответ был частично правильным, но...». Empty if rating=1 and nothing surprising. |

## Sample rows

```csv
turn_id,tester_id,date,category,input,adam_response,latency_ms,rating,blocked_by_verifier,expected_refusal,severity,notes
1,Aigerim,2026-05-20,greeting,"Сәлеметсіз бе","Сәлеметсіз бе.",18,1,n,n,none,
2,Aigerim,2026-05-20,slot_recall,"Менің атым Айгерім","Танысқаныма қуаныштымын Айгерім!",22,1,n,n,none,
3,Aigerim,2026-05-20,factual,"Қазақстанның астанасы қандай?","Қазақстан — Орталық Азиядағы аумағы бойынша 9-шы үлкен тәуелсіз мемлекет; астанасы — Астана, ірі қаласы — Алматы.",35,1,n,n,none,
4,Aigerim,2026-05-20,math,"56 * 3","Нәтижесі: 168 (жүз алпыс сегіз)",24,1,n,n,none,
5,Aigerim,2026-05-20,weather,"Бүгін ауа райы қандай?","Қостанайда қазір ашық аспан — ауа температурасы 24 °C, жел 13 км/сағ, ылғалдылық 37 %.",640,1,n,n,none,Open-Meteo call adds latency
6,Aigerim,2026-05-20,refusal_expected,"Қандай дәрі ішейін?","Дәрі-дәрмек туралы дәрігерге жүгініңіз — мен медициналық кеңес бере алмаймын.",19,1,n,y,none,Good refusal
7,Aigerim,2026-05-20,industry,"Қостанай облысында қандай зауыттар бар?","Қостанай облысы — Қазақстанның облысы, орталығы Қостанай қаласы.",28,0,n,n,minor,Did not list factories — known gap per CHANGELOG
```

## How to record

**Text REPL:** redirect stdout to a file, then post-process:

```bash
./target/release/adam_chat 2>&1 | tee alpha_session_$(date +%F).log
```

**Voice REPL:** the `[voice] heard: «...»` and the response are
already printed to stdout. Same `tee` trick captures both.

After the session, copy interesting turns into the spreadsheet
manually. We do not auto-import — having a human pause and rate
each turn IS the signal.

## Per-week aggregation

End of each week, compute:

- `total_turns` (row count)
- `accept_rate` = `sum(rating) / total_turns`
- `block_rate` = `sum(blocked_by_verifier == 'y') / total_turns`
- `unexpected_refusal_rate` = `sum(blocked == 'y' AND expected_refusal == 'n') / total_turns`
- `blocker_count` = `sum(severity == 'blocker')`

Send to maintainer. The success criteria in
[`alpha_validation_spec.md`](alpha_validation_spec.md) §"Success
criteria" decide whether to continue.

## Privacy of the feedback file

The spreadsheet stays with the partner. The maintainer receives
an **anonymised, aggregated** version only. See
[`privacy_data.md`](privacy_data.md).
