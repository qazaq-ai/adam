#!/usr/bin/env python3
# build_speech_defect_eval.py — synthesise text-level speech-defect
# eval cases per Codex consultation #3 (2026-06-16).
#
# Strategy: take **clean** Kazakh school-eval queries that production
# already answers correctly, apply a deterministic defect transform
# (NLU-level, not audio), and assert the same expected_response.
# A pass means production's routing survives the corruption — i.e.
# FST + morpheme retrieval + lookup_possessive_property substring
# matching are robust to that defect pattern. A fail surfaces a
# specific corruption the system mistakes for a different intent.
#
# Defect categories (Codex categorisation):
#   rhotacism  — р → л / й / dropped
#   sigmatism  — с → ф / т ; ш → с / ф ; ж → з
#   lambdacism — л → й / ў / dropped
#   kappacism  — к → т ; қ → х / к ; ғ → г / х
#   nasalization — vowel drift / deletion (STT surrogate)
#   stuttering — repeated prefix syllables
#   elderly    — vowel/consonant deletion, compressed words
#   whisper    — known Whisper STT drift patterns from rc25/rc28 audit
#
# Each case carries was_accepted=true: production SHOULD survive the
# corruption. Cases that fail are real router gaps to document.

from __future__ import annotations

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
OUT = REPO_ROOT / "data" / "eval" / "speech_defect_eval.json"

# (category, clean_input, corrupted_input, expected_response, notes)
CASES: list[tuple[str, str, str, str, str]] = [
    # ─────────────────────────────────────────────────────────────
    # Rhotacism — «р» dropped or substituted by «л» / «й» / ∅
    # ─────────────────────────────────────────────────────────────
    ("rhotacism", "Рақмет.", "Лақмет.", "оқасы жоқ", "р→л at word start"),
    ("rhotacism", "Рақмет.", "Ақмет.", "оқасы жоқ", "р dropped at word start"),
    ("rhotacism", "Жүрек не үшін керек?", "Жүйек не үшін керек?", "жүрек", "р→й mid-word + missing organ name"),
    ("rhotacism", "Жүрек не үшін керек?", "Жүлек не үшін керек?", "жүрек", "р→л mid-word"),
    ("rhotacism", "Терінің қызметі.", "Теінің қызметі.", "тері", "р dropped"),
    ("rhotacism", "Дұрыс па?", "Дұыс па?", "дұрыс", "р dropped"),
    ("rhotacism", "Бір байтта неше бит бар?", "Біл байтта неше бит бал?", "8", "two р→л substitutions"),
    ("rhotacism", "Күміс қандай зат?", "Күміс қандай зат?", "металл", "control — no р, should still pass"),
    ("rhotacism", "Қазақстанның мемлекеттік тілі.", "Қазақстанның мемлекеттік тілі.", "қазақ", "control — passes through unchanged"),
    ("rhotacism", "Төрттің түбірі.", "Төйттің түбійі.", "2", "р→й on number word + sqrt marker"),

    # ─────────────────────────────────────────────────────────────
    # Sigmatism / shepelyavost' — с/ш/ж drift
    # ─────────────────────────────────────────────────────────────
    ("sigmatism", "Сәлем", "Фәлем", "сәлем", "с→ф at word start"),
    ("sigmatism", "Судың формуласы.", "Фудың формуласы.", "h2o", "су→фу — chemistry lookup should still fire on formula marker"),
    ("sigmatism", "Бір байтта неше бит бар?", "Біл байтта неше біт бал?", "8", "rhotacism + lowercase drift"),
    ("sigmatism", "Шаш ұзындығы.", "Сас ұзындығы.", "<probe>", "ш→с — probe"),
    ("sigmatism", "Жасым отыз екі.", "Зафым отыз екі.", "<probe>", "ж→з + ш→ф stack"),
    ("sigmatism", "Сегізді жетіге көбейт.", "Фегізді жетіге көбейт.", "56", "math survives с→ф"),
    ("sigmatism", "Күкірт қышқылының формуласы.", "Күкіт қышқылының формулафы.", "H₂SO₄", "two-substitution chemistry"),
    ("sigmatism", "Қанның қызметі қандай?", "Қанның қызметі қандай?", "қан", "control"),
    ("sigmatism", "Бесті жетіге қос.", "Бефті жетіге қоф.", "12", "math с→ф"),
    ("sigmatism", "Қазақтың ұлттық тағамы.", "Қазақтың ұлттық тағамы.", "бесбармақ", "control"),

    # ─────────────────────────────────────────────────────────────
    # Lambdacism — «л» → «й» / dropped
    # ─────────────────────────────────────────────────────────────
    ("lambdacism", "Алматы дегеніміз не?", "Айматы дегеніміз не?", "қала", "л→й Almaty"),
    ("lambdacism", "Балқаш көлі туралы.", "Байқаш көйі туралы.", "Балқаш", "two-place л→й"),
    ("lambdacism", "Қазақстанда қандай тіл мемлекеттік?", "Қазақстанда қандай тій мемйекеттік?", "қазақ", "language word inflected"),
    ("lambdacism", "Жасуша деген не?", "Жасуша деген не?", "Жасуша", "control"),
    ("lambdacism", "Бала деген не?", "Бая деген не?", "<probe>", "probe — word with no л context"),
    ("lambdacism", "Қазақтың ұлттық тағамы.", "Қазақтың ұйттық тағамы.", "бесбармақ", "ұлттық→ұйттық"),
    ("lambdacism", "Электр тогы деген не?", "Эектр тогы деген не?", "электр", "э+ле lost"),
    ("lambdacism", "Алгоритм деген не?", "Айгоритм деген не?", "тізбе", "ал→ай"),
    ("lambdacism", "Көздің қызметі.", "Көздің қызметі.", "көру", "control — л not in this case"),
    ("lambdacism", "Тілдің қызметі.", "Тійдің қызметі.", "<probe>", "tongue/lang ambiguity + л drift"),

    # ─────────────────────────────────────────────────────────────
    # Kappacism / uvular weakness — к/қ/ғ drift
    # ─────────────────────────────────────────────────────────────
    ("kappacism", "Қазақстанның астанасы қай қала?", "Хазахстанның астанасы хай хала?", "Астана", "qa→ha + multiple"),
    ("kappacism", "Көмірқышқыл газының формуласы.", "Төмірқышқыл газының формуласы.", "CO₂", "к→т"),
    ("kappacism", "Күміс деген зат қандай?", "Түміс деген зат хандай?", "металл", "к→т + қ→х"),
    ("kappacism", "Қазақ деген не?", "Хазах деген не?", "<probe>", "qa→ha"),
    ("kappacism", "Қышқыл деген не?", "Хышхыл деген не?", "<probe>", "all-қ word"),
    ("kappacism", "Кітап деген не?", "Тітап деген не?", "<probe>", "к→т probe"),
    ("kappacism", "Қанша уақыт қазір?", "Ханша уаххыт хазір?", "<probe>", "voice REPL clock query"),
    ("kappacism", "Кальцийдің химиялық таңбасы.", "Тальцийдің химиялық таңбасы.", "Ca", "к→т on element"),
    ("kappacism", "Қазақстанның мемлекеттік тілі.", "Хазахстанның мемлекеттік тілі.", "қазақ", "control passes? — country name should still resolve"),
    ("kappacism", "Бір байтта неше бит бар?", "Бір байтта неше біт бал?", "8", "no к but mild drift"),

    # ─────────────────────────────────────────────────────────────
    # Nasalization (STT-surrogate via vowel drift/deletion)
    # ─────────────────────────────────────────────────────────────
    ("nasalization", "Қазақстан туралы.", "Казхстан туралы.", "Қазақстан", "ә→а + drop"),
    ("nasalization", "Сәлеметсіз бе.", "Сәлмесіз бе.", "сәлем", "compressed"),
    ("nasalization", "Жүректің қызметі.", "Жүректң қызметі.", "жүрек", "short vowel dropped"),
    ("nasalization", "Бесбармақ туралы.", "Беспармақ туралы.", "бесбармақ", "voicing"),
    ("nasalization", "Қазақстанда қандай тіл мемлекеттік?", "Қазақстнда қандай тіл мемлекетк?", "қазақ", "multi-drop"),
    ("nasalization", "Атом дегеніміз не?", "Атм дегеніміз не?", "Атом", "о dropped"),
    ("nasalization", "Қызыл түс.", "Қзл түс.", "<probe>", "extreme drop"),
    ("nasalization", "Алгоритм.", "Алгрітм.", "тізбе", "vowels collapsed"),
    ("nasalization", "Жер — қандай аспан денесі?", "Жер — қандай аспан денесі?", "ғаламшар", "control"),
    ("nasalization", "Темірдің химиялық таңбасы.", "Тмірдің химиялық таңбасы.", "Fe", "vowel dropped"),

    # ─────────────────────────────────────────────────────────────
    # Stuttering — repeated prefix syllables
    # ─────────────────────────────────────────────────────────────
    ("stuttering", "Сәлем.", "Са-сә-сәлем.", "сәлем", "tri-repeat onset"),
    ("stuttering", "Менің атым Дәулет.", "Ме-мен-менің атым Дә-Дәулет.", "Дәулет", "session name capture under stutter"),
    ("stuttering", "Жүрек не үшін керек?", "Жү-жү-жүрек не үшін керек?", "қан айналымы", "Codex consultation #4 audit: heart pumps blood, not «ойлау». Expected corrected 2026-06-16."),
    ("stuttering", "Қазақстанның астанасы.", "Қа-қа-қазақстанның астанасы.", "Астана", "country stutter"),
    ("stuttering", "Алты түбірі.", "Ал-ал-алты түбірі.", "2.449489742783178", "math survives stutter"),
    ("stuttering", "Бір байтта неше бит бар?", "Бі-бі-бір байтта неше бит бар?", "8", "stutter on numeral"),
    ("stuttering", "Күмістің формуласы.", "Кү-кү-күмістің формуласы.", "Ag", "chemistry under stutter"),
    ("stuttering", "Атом дегеніміз не?", "А-а-атом дегеніміз не?", "Атом", "definition under stutter"),
    ("stuttering", "Қазақтың ұлттық тағамы.", "Қа-қазақтың ұлттық тағамы.", "бесбармақ", "double-onset stutter"),
    ("stuttering", "Бесті жетіге қос.", "Бе-бе-бесті жетіге қос.", "12", "math single-step stutter"),

    # ─────────────────────────────────────────────────────────────
    # Elderly diction — vowel/consonant deletion, compressed words
    # ─────────────────────────────────────────────────────────────
    ("elderly", "Қазақстан Республикасы.", "Казхстан Республикасы.", "Қазақстан", "ә→а + drop"),
    ("elderly", "Сәлеметсіз бе, Ағай.", "Сәлмесіз бе.", "сәлем", "compressed greeting"),
    ("elderly", "Қазақстанның бұрынғы астанасы.", "Казхстанның бұрынғы астанасы.", "Алматы", "elderly drift on country"),
    ("elderly", "Бесбармақ туралы.", "Бесбармақ туралы.", "бесбармақ", "control"),
    ("elderly", "Бір байтта неше бит бар?", "Бі байтта неше бит бар?", "8", "vowel dropped from numeral"),
    ("elderly", "Күмістің химиялық таңбасы.", "Кмістің химиялық таңбасы.", "Ag", "vowel dropped"),
    ("elderly", "Алгоритм деген не?", "Алгрітм деген не?", "тізбе", "vowels lost"),
    ("elderly", "Жер ғаламшар ма?", "Жер ғаламшар ма?", "ғаламшар", "control"),
    ("elderly", "Қазақстанның мемлекеттік тілі.", "Казхстнның мемлекеттік тілі.", "қазақ", "extreme drop"),
    ("elderly", "Темірдің химиялық таңбасы.", "Тмірдің химиялық таңбасы.", "Fe", "vowel drop"),

    # ─────────────────────────────────────────────────────────────
    # Whisper drift — known patterns from rc25/rc28 audit
    # ─────────────────────────────────────────────────────────────
    ("whisper", "Сәлеметсіз бе?", "Сәлімі түсбе.", "сәлем", "rc28 known drift"),
    ("whisper", "Қалыңыз қалай.", "Қалыңғыз қалай.", "жақсы", "rc28 known drift"),
    ("whisper", "Менің атым — Дәулет.", "Алды мен таныс айық.", "Дәулет", "rc28 — accepted as intro probe"),
    ("whisper", "Күмістің формуласы.", "Күмістің формулысы.", "Ag", "stable drift суффикс"),
    ("whisper", "Он алты түбірі.", "Оналты түбірі.", "4", "STT-fold known case"),
    ("whisper", "Жасым отыз екіде.", "Жасым, отыз екіде.", "32", "comma insertion"),
    ("whisper", "Менің атым кім?", "Менімі атым кім.", "Дәулет", "comma/typo drift"),
    ("whisper", "Менің есімім Дәулет.", "Менің есімім даулет.", "Дәулет", "case drop"),
    ("whisper", "Менің жасым нешеде?", "Менің жасым нише де.", "32", "phoneme drift in question word"),
    ("whisper", "Менің атым — Дәулет.", "Менім атым - дәулет.", "Дәулет", "double-drift"),
]


def main() -> None:
    cases_out = []
    for cat, clean, corrupted, expected, notes in CASES:
        cases_out.append({
            "subject": "speech_defect",
            "topic": cat,
            "input": corrupted,
            "expected_response": expected,
            "was_accepted": "<probe>" not in expected,
            "notes": f"clean='{clean}' | {notes}",
        })

    pack = {
        "version": "v6.8-speech-defect-eval-2026-06-16",
        "name": "adam-speech-defect-eval",
        "description": (
            "Text-level NLU-stage speech-defect eval. Each case takes a "
            "clean Kazakh school-eval query that production answers "
            "correctly, applies a deterministic defect transform (per "
            "Codex consultation #3: rhotacism, sigmatism, lambdacism, "
            "kappacism, nasalisation, stuttering, elderly, whisper "
            "drift), and asserts the same expected_response. Pass = "
            "router/FST/lookup survives the corruption. Fail = router "
            "gap to document. Probe cases (expected='<probe>') are "
            "documented gaps that don't count toward score."
        ),
        "sample_count": len(cases_out),
        "cases": cases_out,
    }
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(pack, f, ensure_ascii=False, indent=2)

    accepted = sum(1 for c in cases_out if c["was_accepted"])
    probes = sum(1 for c in cases_out if not c["was_accepted"])
    print(f"wrote {len(cases_out)} cases to {OUT}")
    print(f"  accepted (counted): {accepted}")
    print(f"  probes (documented gaps): {probes}")

    # Per-category breakdown
    from collections import Counter
    by_cat = Counter(c["topic"] for c in cases_out)
    by_cat_acc = Counter(c["topic"] for c in cases_out if c["was_accepted"])
    print("\n  by category:")
    for cat in sorted(by_cat.keys()):
        print(f"    {cat:<14} {by_cat[cat]:>3} total, {by_cat_acc[cat]:>3} accepted")


if __name__ == "__main__":
    main()
