# Risk disclaimer — adam v6.0.0-rc1

> **Read this before deploy.** Honest known-limitations list.
> If anything below is a deal-breaker for the partner's use case,
> please pause the alpha until v6.0.0 GA (target: mid-late July 2026).

## What adam is NOT

- **Not a production-ready service.** v6.0.0-rc1 is a release
  candidate / research preview. The maintainer is solo, working
  M-F evenings + weekends, without commercial funding. Response
  time on bug reports is 1-3 days, not 1-3 hours.
- **Not a general-purpose AI assistant.** adam answers within the
  scope of: (a) Kazakh morphology and grammar; (b) the curated
  knowledge graph (3 672 facts across 51 domains); (c) basic
  arithmetic; (d) live date/time/weather from the laptop; (e)
  geographic facts about Kazakhstan. **Outside that scope adam
  refuses honestly** — it does not invent.
- **Not multilingual.** adam speaks and understands Kazakh only.
  Russian / English input is treated as out-of-scope. (Whisper
  may transcribe Russian / English correctly for the STT layer,
  but the dialog kernel does not respond in those languages.)
- **Not certified for any vertical.** Education, healthcare,
  finance, legal — none of these have a certification claim.
  adam is research-grade software. Use accordingly.

## Known limitations at v6.0.0-rc1

### Dialog layer

1. **Multi-turn anaphora is shallow.** adam remembers the user's
   name / city / occupation across turns, but reference resolution
   on previous-turn entities («оны айтып бер») is limited.
2. **No cross-language fallback.** If the user code-switches into
   Russian mid-sentence, adam treats the Russian portion as
   out-of-scope and may refuse the whole turn.
3. **Industry list-queries are partial.** Asking «Қостанай
   облысында қандай зауыттар бар?» surfaces an IsA fact, not
   the curated factory list. The factory list IS in
   `data/world_core/kz_industry.jsonl` but the ranker does not
   yet prefer it over the generic oblast IsA. **Known gap;
   ranker tuning is a v6.0.5 task.**

### Knowledge gaps

4. **Lexicon coverage 86 % (prefix match), ~63 % (true root).**
   ~70 000 surfaces in the committed corpus are not yet
   decomposed. Top-2 000 candidate roots are queued for review;
   Codex (peer reviewer) triaged them into 491 auto-approve,
   687 auto-exclude, 822 needs-native-speaker. We are processing
   the auto-clusters into v6.0.0-rc1; the needs-review pile
   remains a v6.0.0-GA blocker.
5. **No live news / prices / currency / sports.** adam has no
   live-data feed for these and will refuse with the
   `safety_refusal.current_data` template.
6. **Weather requires opt-in.** Without `ADAM_WEATHER_CITY` (or
   env LAT/LON) the weather turn refuses honestly. This is the
   air-gap default; the partner enables weather explicitly.

### Voice layer

7. **Whisper STT is noisy on Kazakh.** Word-final `-ң → -н` is
   the most common error. We've added STT-noise recovery for the
   common cases (self-intro, weather, math operators, school
   subjects), but expect 5-10 % of voice turns to be misheard.
8. **No barge-in.** TTS playback runs to completion before adam
   accepts the next turn (configurable via `--barge-in`).

### Neural preview (L5.5)

9. **The trained PoC model is morpheme-level, not sentence-level.**
   `/neural <root>` produces a single inflected word with verifier
   audit, not a multi-sentence answer. This is the **runnable
   preview** the migration plan requires; full dialog-loop
   integration is v6.0.5+.
10. **No pre-built checkpoint ships.** The partner trains their
    own via `poc_kazakh_train` (10 min smoke / 95 min full on
    M2 CPU). We do not distribute pre-trained weights in
    v6.0.0-rc1 (licensing review pending).

### Safety & honesty

11. **adam can be wrong.** Even with strict verifier-grounding,
    factual claims trace to the curated KG — which has errors
    and omissions. If the KG says «X», adam says «X»; if «X» is
    out of date, adam is out of date.
12. **adam refuses, doesn't invent.** This is the design. If
    adam says «білмеймін» (I don't know), it really does not
    know — there is no fallback retrieval that will guess.
13. **No PII handling certification.** If a tester volunteers a
    real name / phone / address in a dialog turn, adam stores
    it in session-belief for the duration of the process and
    discards on exit. This is not GDPR-grade certified handling;
    do not deploy adam in workflows that require PII protection
    until v6.x.

## What we promise

1. **No phone-home.** Outbound traffic restricted per
   [`privacy_data.md`](privacy_data.md). Verifiable via `lsof`.
2. **Honest refusals.** When adam doesn't know, it says so. No
   fabricated facts within the verifier-covered scope.
3. **Reproducibility.** Every CHANGELOG entry traces to a commit;
   every commit's tests are green; every reported number can be
   reproduced from the source tree.
4. **Open source.** BUSL-1.1 → Apache-2.0 in four years. No
   surprise relicensing.

## Acknowledgement

The partner acknowledges they've read this document and
understand:

- adam is research-grade software with the limitations above.
- Bug reports are welcomed but response time is not SLA-guaranteed.
- adam is NOT a substitute for human expertise in education,
  healthcare, finance, legal, or any regulated domain.

**Partner organisation**: ________________ · Date: ________________
