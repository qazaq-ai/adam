#!/usr/bin/env python3
"""
**v6.7 generative pivot (2026-06-13)** — extract REAL (whisper_input,
adam_response, was_correct) triplets from voice REPL audit transcripts
that accumulated across rc25-rc28 audits.

Two outputs:
  1. `data/curated/adam_real_voice_audit_pack.json` — training-grade
     pairs where the response was NOT rejected (golden real examples).
  2. `data/eval/v6_7_real_audit_eval.json` — eval-only pack of all
     real turns including rejections (the model gets graded on whether
     it produces the response we now know was acceptable).

These are GOLD because every entry came from a real voice REPL turn
with real Whisper-drifted input + real cascade output + real user
acceptance/rejection. No synthesis.
"""

import json
import re
from pathlib import Path

# Hand-curated real audit turns. Each entry is from a specific
# voice REPL session. Source: chat transcripts user pasted from
# rc25, rc28-morning, and rc28-noon audits.
#
# Format: (whisper_input, expected_response, was_accepted, notes)
# was_accepted: True if no [REJECTION DETECTED] followed the turn.
REAL_TURNS = [
    # ───── rc28 noon audit (2026-06-13) ─────
    ("Ассаламу алеку.", None, False, "rc28-noon: Whisper drift, refused"),
    ("Сәлімі түсбе.", "Сәлем", True, "rc28-noon: greeting echo"),
    ("Қалыңғыз қалай.", "Бәлкім, Қалай туралы айтасыз ба.", False,
     "rc28-noon: intent Greeting correct, cascade misroutes — bug"),
    ("Қалыңыз қалай.", "Жақсы, ал сіз қалайсыз.", True, "rc28-noon: greeting works"),
    ("Рақмет, аллаға шүкір.", None, False,
     "rc28-noon: coherence refused but should be StatementOfWellbeing"),
    ("Рақмет.", "Оқасы жоқ", True, "rc28-noon: thanks"),
    ("Алды мен таныс айық.", "Иә, әрине. Мен — қазақша сөйлесуге арналған ARK, атым адам. Атыңызды айта аласыз ба?",
     True, "rc28-noon: intro proposal works"),
    ("Менің атым — дәулет.", "Дәулет деген атыңызды есте сақтап, сізді Дәке деп атаймын — қазақша.", True,
     "rc28-noon: name capture (— қазақша tail now dropped at 825916af)"),
    ("Мен қостанайда тұрамын.", "Қостанай екен, түсіндім.", True, "rc28-noon: location"),
    ("Жасын от екіде.", "От — ыстық.", False, "rc28-noon: math misroute → fire definition"),
    ("Жасым, отыз екіде.", "Сіз 32 жастасыз.", True, "rc28-noon: age recall after retry"),
    ("Сен кім боласың.", "Мен — адам, қазақ тіліне арналған детерминирленген тілдік жүйемін. Үлкен тілдік модель емеспін — жауаптарымды алдын ала тексерілген деректерден аламын.",
     True, "rc28-noon: ask-about-system"),
    ("Олай болса, қазір сағатын іші.", "Қазір сағат 09:16.", True, "rc28-noon: time query"),
    ("Бүгін қай күн.", "Бүгін — сенбі.", True, "rc28-noon: date"),
    ("Ертең қай күн болады.", "Ертең — жексенбі болады.", True, "rc28-noon: tomorrow"),
    ("Судың формуласы.", "Судың формуласы — H₂O.", True, "rc28-noon: chemistry"),
    ("Күмістің формулысы.", "Күмістің формуласы — Ag.", True, "rc28-noon: chemistry with drift"),
    ("отыз үш көбейт төртке сосын бөл екіге.", "66", True, "rc28-noon: math evaluator"),
    ("Менің атым кім.", "Сіздің атыңыз Дәулет.", True, "rc28-noon: name recall"),
    ("Мен қай жерде тұрамын.", "Сіз Қостанайда тұрасыз.", True, "rc28-noon: location recall"),
    ("Менің жасым нешеде.", "Сіздің жасыңыз 32", True, "rc28-noon: age recall"),
    ("Казір қазақстанның президенті кім?", "Қазақстанның қазіргі Президенті — Қасым-Жомарт Кемелұлы Тоқаев (2019 жылдан бері).",
     True, "rc28-noon: president fact"),
    ("Алты түбірі.", "2.449489742783178", True, "rc28-noon: square root"),
    ("Он екіден төртінше дәрежеге дейін.", None, False,
     "rc28-noon: hallucinated agroindustry response (cascade off rails)"),
    ("Он екі дареже төрт.", "Бәлкім, Дареже туралы айтасыз ба.", False, "rc28-noon: dareже drift misroute"),
    ("Қандай бағдарламалы тілдерді білесің.", "Тіл — қарым-қатынас құралы.", False,
     "rc28-noon: programming languages → generic"),
    ("Сау бол.", "Аман болыңыз, Дәке.", True, "rc28-noon: vocative farewell works (9529fa78)"),

    # ───── rc28 morning audit (2026-06-13) ─────
    ("Ассаламу ауакум.", "Уағалайкум-ас-салам, Ағай.", True, "rc28-morn: muslim greeting + voice honorific"),
    ("Рақмет, аллаға шұқыр.", None, False, "rc28-morn: coherence refused, same bug"),
    ("Жасым алпыс алтыға толды.", "66 — Жақсы жас.", False,
     "rc28-morn: wrong template; user wanted «Сіз 66 жастасыз.» — fixed 9529fa78"),
    ("Менің атым кім?", "Сіздің атыңыз Дәулет.", True, "rc28-morn: name recall"),
    ("Менің жасым нише де.", "66 — Жақсы жас.", False,
     "rc28-morn: age recall with drift; same template issue"),
    ("тоқсан төрт көбейт алтыға сосын бөл екіге сосын қос үш.", "285", True,
     "rc28-morn: complex math 94*6/2+3=285"),
    ("Менімі атым кім.", "Сіздің атыңыз Дәулет.", True, "rc28-morn: name with drift"),
    ("Мен өмір сүргім келмейді.",
     "Сіздің айтқаныңыз мені алаңдатады. Бұл сезіммен жалғыз қалмаңыз. Қазір тікелей қоңырау шалыңыз: 150 (балалар мен жасөспірімдерге сенім телефоны) немесе 112 (бірыңғай экстренді қызмет). Сіз маңыздысыз және көмек алуға лайықсыз. Маман адаммен сөйлесу маңызды — мен бұл рөлді алмаймын.",
     True, "rc28-morn: suicidal escalation (red_flags works)"),
    ("Бас ауырса, қандай дәріні ішуге болады.", None, False,
     "rc28-morn: medical question — wrong intent route"),
    ("Менің басым ауырады, қандай дәріні ішуге болады.",
     "Дәрі-дәрмек ұсынысы беруге мүмкіндігім жоқ — диагноз қою және ем тағайындау дәрігер құзыретінде. Жедел жағдайда 103-ке қоңырау шалыңыз. Жалпы анықтаманы айта аламын, бірақ нақты дәрі-дозаны маман адаммен талқылаңыз.",
     True, "rc28-morn: medical refusal works on retry"),
    ("Қазақстанның бірінші президент кім болды?",
     "Қазақстанның тұңғыш Президенті — Нұрсұлтан Әбішұлы Назарбаев (1991–2019).",
     True, "rc28-morn: Nazarbayev fact"),

    # ───── rc25 voice audit (before v6.6 wire) ─────
    ("Менің атым — дәулет.", "Атыңызды есте сақтадым. Танысқанымызға қуаныштымын!", True,
     "rc25: StatementOfName override worked"),
    ("Менің есімім даулет.", "Дәулет деген атыңызды есте сақтап, сізді Дәке деп атаймын — қазақша.", True,
     "rc25: name with drift «даулет→дәулет» recovered"),
]


def build():
    train_pairs = []  # accepted real examples → usable for training
    eval_cases = []   # all real turns → eval graded

    for whisper, response, was_accepted, notes in REAL_TURNS:
        eval_cases.append({
            "input": whisper,
            "expected_response": response if was_accepted else None,
            "was_accepted": was_accepted,
            "notes": notes,
        })
        if was_accepted and response:
            train_pairs.append({
                "prompt": whisper,
                "response": response,
                "source": "real_voice_audit",
                "notes": notes,
            })

    Path("data/curated").mkdir(parents=True, exist_ok=True)
    Path("data/eval").mkdir(parents=True, exist_ok=True)

    train_out = {
        "version": "v6.7-real-voice-audit-2026-06-13",
        "name": "adam-real-voice-audit-pack",
        "target_language": "kazakh",
        "script": "cyrillic",
        "sample_count": len(train_pairs),
        "samples": train_pairs,
    }
    with open("data/curated/adam_real_voice_audit_pack.json", "w", encoding="utf-8") as f:
        json.dump(train_out, f, ensure_ascii=False, indent=2)

    eval_out = {
        "version": "v6.7-real-audit-eval-2026-06-13",
        "name": "adam-real-voice-audit-eval",
        "description": (
            "Real voice REPL turns from rc25/rc28 audits. Each case has the "
            "raw Whisper input and the response that was either accepted by "
            "the user (was_accepted=true → trainable example) or rejected "
            "(was_accepted=false → known failure). A model that responds "
            "with `expected_response` is correct; a model that produces "
            "something semantically equivalent gets partial credit (human-judge)."
        ),
        "sample_count": len(eval_cases),
        "cases": eval_cases,
    }
    with open("data/eval/v6_7_real_audit_eval.json", "w", encoding="utf-8") as f:
        json.dump(eval_out, f, ensure_ascii=False, indent=2)

    n_accepted = sum(1 for c in eval_cases if c["was_accepted"])
    n_rejected = len(eval_cases) - n_accepted
    print(f"Real turns extracted: {len(REAL_TURNS)}")
    print(f"  Accepted (golden trainable):  {n_accepted}")
    print(f"  Rejected (known failures):    {n_rejected}")
    print(f"Wrote:")
    print(f"  data/curated/adam_real_voice_audit_pack.json ({n_accepted} pairs)")
    print(f"  data/eval/v6_7_real_audit_eval.json         ({len(eval_cases)} cases)")


if __name__ == "__main__":
    build()
