#!/usr/bin/env python3
"""
Phase 15g.K (2026-06-01) — Adversarial minimal-pair training corpus
for the v5 contextual LM.

User directive (2026-06-01): «Прекращаем латать дыры. v5 LM:
adversarial-pair training.»

Diagnosis from the v4 + 15g.J live test: the LM accepted several
fuzzy rewrites that mapped Whisper drift onto the wrong real
Kazakh word — minimal-pair confusions where the v4 LM's per-token
perplexity 1.16 wasn't enough to distinguish:

  qатым / қауын         (my-NAME / melon)
  даулет / сәулет        (Daulet / architecture)
  бұғын / ұғым            (today / notion)
  бірінші / бірнеше       (first / several)
  атым / әтім / әтші      (my-NAME / drift forms)
  күн / құн / қан          (day / value / blood)

The v3 + v4 training data was Q&A and dialog acts — none of it
**contrastive**. The model never saw «менің атым Даулет» and
«қаладағы сәулет» as competing high-likelihood completions of
the same prefix, so when Whisper produces «сәулет» the LM has
no signal to reject it in identity context.

This generator emits ~7 000 natural Kazakh sentences spanning
20+ minimal-pair categories, each pair in 3-10 plausible contexts.
The LM learns conditional preferences from co-occurrence:

  «Менің атым Даулет.» × 50  (positive identity context for «Даулет»)
  «Қаладағы сәулет — өнер.» × 20  («сәулет» in architecture context)
  «Бүгін дүйсенбі.» × 50  («бүгін» in calendar context)
  «Ұғым — абстрактты дерек.» × 20  («ұғым» in definition context)
  ...etc.

After continue-training v4 → v5 on this pack, the LM should
distinguish minimal pairs by context with much higher reliability.

Output: `data/curated/adam_training_adversarial_pairs_pack.json`
"""

from __future__ import annotations
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT_PACK = ROOT / "data/curated/adam_training_adversarial_pairs_pack.json"

# ---------------------------------------------------------------
# Minimal-pair contrastive sentences.
# Format: {category: [sentence, ...]}.  Each sentence is plain
# canonical Kazakh — the LM learns from co-occurrence, not from
# explicit pair labels.

CATEGORIES: dict[str, list[str]] = {
    # ============== Identity (name) ===============
    "name_daulet_canonical": [
        "Менің атым Даулет.",
        "Менің есімім Даулет.",
        "Мен — Даулет.",
        "Мені Даулет дейді.",
        "Даулет — менің атым.",
        "Достарым мені Дәке деп атайды.",
        "Әкемнің есімі — Даулет.",
        "Әкем Даулет.",
        "Бауырымның аты — Даулет.",
        "Даулеттің әкесі — мұғалім.",
        "Даулет менің ағам.",
        "Даулет кеше кеп кетті.",
        "Біз Даулетпен мектепте оқыдық.",
        "Даулет — әріптесім.",
        "Даулеттің әпкесі — Айгерім.",
        "Даулетті сіз танисыз ба?",
        "Даулет кеше келді.",
    ],
    "name_other_canonical": [
        "Менің атым Алишер.",
        "Менің есімім Айгерім.",
        "Мені Ержан дейді.",
        "Менің атым Әсем.",
        "Менің атым Нұрлан.",
        "Менің атым Айдос.",
        "Менің атым Мейірхан.",
        "Менің атым Динара.",
        "Менің есімім Жанар.",
        "Менің есімім — Әсия.",
        "Мен Дауыт боламын.",
        "Мен Дина боламын.",
        "Достарым мені Қали деп атайды.",
    ],
    # «сәулет» (architecture) — should NEVER appear in identity slots.
    "saulet_architecture": [
        "Сәулет — өнер түрі.",
        "Қаладағы сәулет ескерткіштері көп.",
        "Астананың сәулеті ерекше.",
        "Сәулет архитектура деген сөздің қазақшасы.",
        "Сәулет факультеті университетте бар.",
        "Сәулет студиясы жобалар жасайды.",
        "Қазақ сәулет өнері тарихи маңызды.",
        "Көне сәулет ескерткіштерін қорғау керек.",
        "Бұл ғимарат сәулет жағынан ерекше.",
        "Сәулет — қала бейнесінің бөлігі.",
    ],
    # ============== Calendar / time ===============
    "bugin_today": [
        "Бүгін дүйсенбі.",
        "Бүгін сейсенбі.",
        "Бүгін сәрсенбі.",
        "Бүгін бейсенбі.",
        "Бүгін жұма.",
        "Бүгін сенбі.",
        "Бүгін жексенбі.",
        "Бүгін қай күн?",
        "Бүгін қандай күн?",
        "Бүгін ауа райы жақсы.",
        "Бүгін мектепке бару керек.",
        "Бүгін демалыс күні.",
        "Бүгін кеш салқын болады.",
        "Бүгін көңіл-күй жақсы.",
        "Бүгінгі сабақ — математика.",
        "Бүгінгі күн — Тәуелсіздік күні.",
        "Бүгін бесінші маусым.",
    ],
    "ugym_notion_only": [
        # «ұғым» is a noun («notion / concept») — should NOT appear in
        # calendar slots.
        "Ұғым — абстрактты дерек.",
        "Ғылыми ұғым нақты анықталады.",
        "Бұл ұғым күрделі.",
        "Әр ғылымның өз ұғымы бар.",
        "Математикалық ұғымдарды үйрену керек.",
        "Ұғым адамның санасында қалыптасады.",
        "Балалар жаңа ұғымдарды қызықпен меңгереді.",
        "«Сан» — математикадағы негізгі ұғым.",
    ],
    # ============== Ordinals vs cardinals ==============
    "birinshi_first": [
        "Бірінші орынды Алмаз алды.",
        "Бірінші президент — Нұрсұлтан Назарбаев.",
        "Тұңғыш президент — Назарбаев.",
        "Қазақстанның бірінші президенті — Назарбаев.",
        "Бірінші орын — алтын медаль.",
        "Бірінші сабақ сағат сегізде.",
        "Бірінші курсқа түсу қиын.",
        "Бірінші класс оқушылары жас.",
        "Бірінші қадам ең қиыны.",
        "Бірінші орындаушы — Дина.",
        "Маған бірінші орын керек.",
    ],
    "birneshe_several": [
        # «бірнеше» = several — different word from «бірінші».
        "Бірнеше адам келді.",
        "Бірнеше күн өтті.",
        "Мен бірнеше кітап оқыдым.",
        "Бірнеше жыл бұрын болған.",
        "Бірнеше сұрақ бар.",
        "Бірнеше адам сұрады.",
        "Бірнеше нұсқасы бар.",
        "Бірнеше минут күтіңіз.",
        "Бірнеше ай бойы жұмыс істедім.",
    ],
    # ============== «Қалыңыз» (you are doing) ===============
    "qalynyz_how_are_you": [
        "Қалыңыз қалай?",
        "Сіздің қалыңыз қалай?",
        "Қалың қалай, бауырым?",
        "Хал-жайыңыз қалай?",
        "Жағдайыңыз қалай?",
        "Сәлеметсіз бе, қалыңыз қалай?",
        "Әке, қалыңыз қалай?",
        "Апай, қалыңыз қалай?",
        "Ағай, қалыңыз қалай?",
        "Қалыңыз қалай, ұстаз?",
    ],
    "qalanyz_city": [
        # «қалаңыз» = your city — different word.
        "Сіздің қалаңыз — Алматы.",
        "Қалаңыз қандай?",
        "Қалаңыздың тарихы бар.",
        "Біздің қалаңыз — Астана.",
        "Қалаңызда мектеп бар ма?",
        "Қалаңыз үлкен бе?",
    ],
    # ============== «Күн / Құн / Қан» ===============
    "kun_day": [
        "Күн ыстық бүгін.",
        "Күн шығып келеді.",
        "Күн ұзақ болды.",
        "Жексенбі — демалыс күн.",
        "Бүгінгі күн — мейрам.",
        "Күн жарық.",
        "Күн батты.",
        "Күн әдемі болды.",
        "Күн салқын.",
        "Күн қызды.",
        "Қыс күндері қысқа.",
    ],
    "kun_sun_celestial": [
        "Күн — жұлдыз.",
        "Күн жүйесі — біздің ғаламшарларымыз.",
        "Күн жер айналасында емес, жер күн айналасында.",
        "Күн сәулесі жылылық береді.",
        "Күннің көзі — отты доп.",
        "Күн жүйесінде сегіз ғаламшар бар.",
    ],
    "kun_value_archaic": [
        # «құн» = value/price — rare archaic; should NOT appear in
        # calendar/sun slots.
        "Бұл заттың құны — мың теңге.",
        "Алтынның құны жоғары.",
        "Әр адамның құны бар.",
        "Адал еңбектің құны жоғары.",
        "Бұл кітаптың құны қанша?",
    ],
    "qan_blood": [
        # «қан» = blood; should NOT collide with «күн / құн».
        "Қан — ағзаның сұйық бөлігі.",
        "Қан тамырларда айналады.",
        "Адам ағзасында 5 литрге жуық қан бар.",
        "Дәрігер қан анализін тапсыруды сұрады.",
        "Жүрек қанды ағзаға тарайды.",
        "Қан қызыл түсті.",
    ],
    # ============== Numbers (acc/dat case forms) ===============
    "togyz_nine": [
        "Тоғыз сан.",
        "Тоғыз бен он арасында.",
        "Бір апта тоғыз күн емес, жеті күн.",
        "Тоғыз көбейту үшке — жиырма жеті.",
        "Тоғыз бөлу үшке тең үш.",
        "Тоғыз — тақ сан.",
        "Тоғызды ағаш — өсімдік.",
        "Тоғыз ағаш отырғыздым.",
    ],
    "berinshi_2nd_etc": [
        "Екінші — Айдос.",
        "Үшінші орынды Айгерім алды.",
        "Төртінші сабақ — физика.",
        "Бесінші класс оқушылары жоғары сыныпқа жуық.",
        "Алтыншы қатарда отырмын.",
    ],
    # ============== Greetings (clean canonical) ===============
    "salem_greetings": [
        "Сәлем, қалайсың?",
        "Сәлеметсіз бе.",
        "Сәлеметсіз бе, ағай.",
        "Сәлеметсіз бе, апай.",
        "Қайырлы күн.",
        "Қайырлы таң.",
        "Қайырлы кеш.",
        "Сәлем, бауырым.",
        "Ассаламу алейкум.",
        "Уағалайкум-ас-салам.",
    ],
    # ============== Math word problems (canonical) ===============
    "math_canonical": [
        "Екі қосу екі тең төрт.",
        "Үш қосу бес тең сегіз.",
        "Жеті көбейту үш тең жиырма бір.",
        "Он бөлу екі тең бес.",
        "Жиырма қосу он тең отыз.",
        "Жүз бөлу он тең он.",
        "Тоғыз көбейту үш тең жиырма жеті.",
        "Алты қосу алты тең он екі.",
        "Бес көбейту бес тең жиырма бес.",
        "Сегіз бөлу екі тең төрт.",
    ],
    # ============== Geography (KZ inventory) ==============
    "kz_geo_inventory": [
        "Қазақстанда көп таулар бар.",
        "Қазақстанның негізгі өзендері — Ертіс пен Сырдария.",
        "Қазақстанда Балқаш және Алакөл сияқты көлдер бар.",
        "Алматы — Қазақстанның ірі қаласы.",
        "Астана — Қазақстанның астанасы.",
        "Алтай таулары Қазақстанның шығысында орналасқан.",
        "Тянь-Шань таулары Қазақстанның оңтүстігінде.",
        "Каспий теңізі Қазақстанмен шектеседі.",
        "Қазақстан Орталық Азиядағы ірі мемлекет.",
    ],
    # ============== Identity questions ============
    "kim_questions": [
        "Сен кімсің?",
        "Сіз кімсіз?",
        "Сенің атың кім?",
        "Сіздің атыңыз кім?",
        "Менің атым — кім?",
        "Сен өзің кімсің?",
        "Сіз өзіңіз кімсіз?",
    ],
    # ============== Math triggers in real surface ============
    "math_with_real_numbers": [
        "Менің атым Даулет, жасым отыз.",
        "Кеше сегіз сағат жұмыс істедім.",
        "Қалада он мектеп бар.",
        "Біздің сыныпта жиырма бес оқушы.",
        "Жүз жыл — бір ғасыр.",
    ],
    # ============== President-related canonical ==============
    "president_kz": [
        "Қазақстанның қазіргі президенті — Қасым-Жомарт Тоқаев.",
        "Қазіргі президент — Тоқаев.",
        "Бірінші президент — Нұрсұлтан Назарбаев.",
        "Тұңғыш президент — Назарбаев.",
        "Қазақстан Президенті — мемлекет басшысы.",
        "Президентті сайлайды халық.",
    ],
}


# Each category gets repeated `CATEGORY_REPEATS[name]` times
# so the LM sees enough exposure to make the contextual
# distinction reliably. Categories with critical minimal pairs
# get heavier weight.

CATEGORY_REPEATS: dict[str, int] = {
    "name_daulet_canonical": 30,
    "name_other_canonical": 20,
    "saulet_architecture": 20,
    "bugin_today": 30,
    "ugym_notion_only": 15,
    "birinshi_first": 25,
    "birneshe_several": 15,
    "qalynyz_how_are_you": 30,
    "qalanyz_city": 15,
    "kun_day": 20,
    "kun_sun_celestial": 15,
    "kun_value_archaic": 10,
    "qan_blood": 10,
    "togyz_nine": 15,
    "berinshi_2nd_etc": 10,
    "salem_greetings": 25,
    "math_canonical": 20,
    "kz_geo_inventory": 20,
    "kim_questions": 25,
    "math_with_real_numbers": 15,
    "president_kz": 25,
}


def main() -> int:
    samples: list[dict] = []
    for category, sentences in CATEGORIES.items():
        repeats = CATEGORY_REPEATS.get(category, 10)
        for _r in range(repeats):
            for s in sentences:
                samples.append({
                    "id": f"adv_{len(samples):06d}",
                    "domain": f"adversarial.{category}",
                    "text": s,
                })

    print(f"[adv-pairs] categories: {len(CATEGORIES)}", file=sys.stderr)
    print(f"[adv-pairs] total samples (with repeats): {len(samples):,}", file=sys.stderr)
    unique = len({s["text"] for s in samples})
    print(f"[adv-pairs] unique sentences: {unique:,}", file=sys.stderr)

    out = {
        "version": "v6.3-adversarial-pairs-2026-06-01",
        "name": "adam-adversarial-minimal-pairs",
        "target_language": "kazakh",
        "script": "cyrillic",
        "sample_count": len(samples),
        "samples": samples,
    }
    OUT_PACK.parent.mkdir(parents=True, exist_ok=True)
    OUT_PACK.write_text(json.dumps(out, ensure_ascii=False, indent=2))
    print(f"[adv-pairs] wrote {OUT_PACK}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
