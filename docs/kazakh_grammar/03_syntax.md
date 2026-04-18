# Kazakh Syntax — SOV, Case Government, Clause Structure

Status: **Week 1 stub** — scope-scoping only. Syntax matters for the LM over roots (section 4 of `00_architecture_v1.md`) but is not on the FST critical path. Detailed study deferred to Week 5+.

## 1. Basic word order

Kazakh is **SOV** (Subject-Object-Verb): `Мен кітапты оқыдым` = "I read the book" (literally: I book-ACC read-PAST-1SG).

Key ordering principles:

- Verb is normally final.
- Modifiers precede their head: `үлкен кітап` ("big book"), not `*кітап үлкен`.
- Postpositions (not prepositions): `мектеп+ке дейін` ("until school"), not `*дейін мектеп+ке`.
- Genitive phrases: `баланың кітабы` (POSSESSOR-GEN POSSESSED-POSS.3) = "the child's book".

Word order is **looser in practice** than in strictly configurational languages because of rich case marking. Topic and focus can reorder constituents without loss of grammaticality. For our LM, we encode the **canonical SOV order** and accept that real corpus data will show variation.

## 2. Case government

### 2.1 Verbs governing case

- Transitive verb → direct object is **accusative** (when definite) or **nominative** (when indefinite): `кітап оқыдым` vs `кітапты оқыдым`.
- Motion verbs → destination in **dative** or **allative** (дейін): `үйге барамын` = "I go home".
- Location verbs → location in **locative**: `үйде тұрамын` = "I live at home".
- Separation verbs → source in **ablative**: `үйден шықтым` = "I left the house".

### 2.2 Postpositions governing case

| postposition | meaning | governs |
|---|---|---|
| үшін | for | nominative / genitive |
| туралы | about | nominative / genitive |
| арқылы | through | nominative |
| кейін | after | ablative |
| бұрын | before | ablative |
| дейін | until | dative |
| сайын | every | nominative |

The LM must learn which postposition requires which case. Since this is categorical (not statistical), we could actually **encode it as a constraint** in the root-LM's lexicon entry for each postposition. That moves yet more predictability out of the stochastic layer.

## 3. Clause combining

- **Coordinating conjunctions**: `және` (and), `бірақ` (but), `немесе` (or), `әрі` (also).
- **Subordinating**: via converb (гerund-like) forms of verbs, not with overt subordinators. `Мен келгенде...` = "when I came...", literally "I come-CONVERB-LOC".
- **Embedded clauses** use participles (`-ған/-ген`) that nominalise a clause.

For v1.0.0 we scope to **simple clauses and two-clause compounds with overt conjunctions**. Full participial embedding is v2.0+.

## 4. Predicate types

Kazakh predicates can be:

- Verbal (default, most cases)
- Nominal with copula `-дa/-де` (existential-locative) or predicative suffixes (`-мын/-мін` etc.)
- Adjectival with same predicative endings

Example: `Мен мұғаліммін` = "I am a teacher" (mug̅alim-1SG.PRED), no overt copula verb.

Predicative endings **mirror** personal-ending paradigms of verbs. This is convenient: the same set of suffixes serves both "I write" (`жазамын`) and "I am a teacher" (`мұғаліммін`). Our FST can reuse the same state machine with a different stem category.

## 5. What the root-LM needs to learn

Given the above, the small-transformer root-LM has to predict:

1. **Next root** (from the ~10-50k root vocabulary)
2. **Feature bundle** attaching to that root (from ~500 possible bundles after pruning impossible combinations)
3. **Syntactic role** (subject/object/oblique) — implicit in case choice
4. **Clause boundaries** — sentence-final punctuation + optional conjunctions

No need to learn:
- Which suffix goes where (fixed by morphology)
- Which allomorph to use (fixed by phonology)
- That "үшін" needs its governee in genitive (fixed by lexicon)

This is where the **massive compute savings** come from.

## 6. Source references

- Ысқақов А., «Қазіргі қазақ тілі: синтаксис» — primary
- Бaйғыт Н.Қ., "Qazaq syntax morphosyntaxisi" (modern treatment)
- Apertium-kaz disambiguation CG-3 rules (show which syntactic patterns they treat as reliable)

## 7. Out of scope for v1.0.0

- Free word order variations
- Discourse / information structure (topic, focus)
- Pragmatics
- Dialects
- Historical forms
