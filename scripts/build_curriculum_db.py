#!/usr/bin/env python3
# build_curriculum_db.py — create + populate the Kazakh curriculum
# SQLite database (data/curriculum/curriculum.db).
#
# Why SQLite + this schema:
#   - Pedagogical ordering is first-class (PK enforces grade →
#     subject → topic_order → subtopic_order → qa_order monotone
#     traversal; full index supports range queries by grade/subject).
#   - Binary format. ~5× smaller than equivalent JSON for the same
#     content, single file to version + ship.
#   - Queryable from sqlite3 CLI for spot-checks without writing code.
#   - Trivial to export into training pack format.
#
# Schema lives here as code (single source of truth). Topics are
# inserted by domain-specific functions (one per subject) appended
# below; each generates Q&A pairs in curriculum order.
#
# Invariants enforced at write time:
#   - qa_id MUST encode the full path: kz_<subject>_g<grade>_t<topic>_s<sub>_q<qa>
#   - (grade, subject, topic_order, subtopic_order, qa_order) UNIQUE
#   - Q, A non-empty Kazakh text; loanwords ok if they're Latin-letter
#     chemistry symbols (h2o, ag, NaCl, …) — these are domain artefacts
#   - source is 'curated' for hand-authored, or 'wikipedia_kk:<page>'
#     for grounded facts

from __future__ import annotations

import argparse
import os
import sqlite3
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DB_PATH = REPO_ROOT / "data" / "curriculum" / "curriculum.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS curriculum (
    qa_id            TEXT    PRIMARY KEY,
    grade            INTEGER NOT NULL,
    subject          TEXT    NOT NULL,
    topic_order      INTEGER NOT NULL,
    topic_name       TEXT    NOT NULL,
    subtopic_order   INTEGER NOT NULL,
    subtopic_name    TEXT,
    qa_order         INTEGER NOT NULL,
    question         TEXT    NOT NULL,
    answer           TEXT    NOT NULL,
    difficulty       INTEGER DEFAULT 1,
    question_type    TEXT    DEFAULT 'definition',
    source           TEXT    DEFAULT 'curated',
    created_at       TEXT    DEFAULT (datetime('now')),
    UNIQUE (grade, subject, topic_order, subtopic_order, qa_order)
);

CREATE INDEX IF NOT EXISTS idx_seq
    ON curriculum(grade, subject, topic_order, subtopic_order, qa_order);

CREATE INDEX IF NOT EXISTS idx_subject_grade
    ON curriculum(subject, grade);
"""


def open_db(path: Path = DB_PATH) -> sqlite3.Connection:
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(path)
    conn.executescript(SCHEMA)
    return conn


def make_qa_id(subject: str, grade: int, topic: int, sub: int, qa: int) -> str:
    return f"kz_{subject}_g{grade:02d}_t{topic:02d}_s{sub:02d}_q{qa:03d}"


def insert_qa(
    conn: sqlite3.Connection,
    *,
    grade: int,
    subject: str,
    topic_order: int,
    topic_name: str,
    subtopic_order: int,
    subtopic_name: str | None,
    qa_order: int,
    question: str,
    answer: str,
    difficulty: int = 1,
    question_type: str = "definition",
    source: str = "curated",
) -> str:
    qa_id = make_qa_id(subject, grade, topic_order, subtopic_order, qa_order)
    conn.execute(
        """
        INSERT OR REPLACE INTO curriculum
            (qa_id, grade, subject, topic_order, topic_name,
             subtopic_order, subtopic_name, qa_order,
             question, answer, difficulty, question_type, source)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (qa_id, grade, subject, topic_order, topic_name,
         subtopic_order, subtopic_name, qa_order,
         question, answer, difficulty, question_type, source),
    )
    return qa_id


# ────────────────────────────────────────────────────────────────────
# Chemistry — Grade 8 — Topic 1: Атом және молекула
# ────────────────────────────────────────────────────────────────────

def populate_himiya_g08_t01_atom(conn: sqlite3.Connection) -> int:
    """Chemistry 8 / Topic 1: Atom and molecule. Foundational."""
    G, SUBJ, T, TNAME = 8, "himiya", 1, "Атом және молекула"

    s1 = "Атом ұғымы"
    pairs_s1 = [
        ("Атом дегеніміз не?",
         "Атом — заттың химиялық бөлінбейтін ең кіші бөлшегі.",
         1, "definition"),
        ("Атомның өлшемі қанша?",
         "Атомның диаметрі шамамен 10-10 м (0,1 нм).",
         2, "definition"),
        ("Атом қандай бөлшектерден тұрады?",
         "Атом ядродан және ядро айналасында қозғалатын электрондардан тұрады.",
         2, "definition"),
        ("Атом ядросында қандай бөлшектер бар?",
         "Атом ядросында оң зарядты протондар және зарядсыз нейтрондар бар.",
         2, "definition"),
        ("Атомның қандай заряды бар?",
         "Тұтас атом электрлік бейтарап — протондар санының оң заряды электрондар санының теріс зарядына тең.",
         3, "reasoning"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s1, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=1, subtopic_name=s1, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    s2 = "Молекула ұғымы"
    pairs_s2 = [
        ("Молекула дегеніміз не?",
         "Молекула — заттың химиялық қасиеттерін сақтап тұратын ең кіші бөлшегі.",
         1, "definition"),
        ("Молекула атомдардан қалай тұрады?",
         "Молекула бірдей немесе әр түрлі екі немесе одан да көп атомнан тұрады.",
         2, "definition"),
        ("Қарапайым зат пен күрделі заттың молекулалары қалай ажыратылады?",
         "Қарапайым зат молекуласы бір түрлі атомнан, ал күрделі заттыкі әр түрлі атомдардан тұрады.",
         3, "reasoning"),
        ("Молекула мысалдары келтіріңіз.",
         "Мысалы: O2 (оттегі), h2o (су), co2 (көмірқышқыл газы).",
         2, "definition"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s2, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=2, subtopic_name=s2, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    s3 = "Заттардың құрылысы"
    pairs_s3 = [
        ("Зат қандай күйлерде бола алады?",
         "Зат үш агрегаттық күйде болады: қатты, сұйық және газ.",
         1, "definition"),
        ("Қатты заттарда молекулалар қалай орналасады?",
         "Қатты заттарда молекулалар бір-біріне тығыз орналасып, тек тербелмелі қозғалыс жасайды.",
         2, "definition"),
        ("Газда молекулалар қалай орналасады?",
         "Газда молекулалар бір-бірінен алыс орналасып, барлық бағытта еркін қозғалады.",
         2, "definition"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s3, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=3, subtopic_name=s3, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    return len(pairs_s1) + len(pairs_s2) + len(pairs_s3)


# ────────────────────────────────────────────────────────────────────
# Chemistry — Grade 8 — Topic 2: Химиялық элементтер
# ────────────────────────────────────────────────────────────────────

def populate_himiya_g08_t02_elements(conn: sqlite3.Connection) -> int:
    """Chemistry 8 / Topic 2: Chemical elements + symbols."""
    G, SUBJ, T, TNAME = 8, "himiya", 2, "Химиялық элементтер"

    s1 = "Элемент ұғымы"
    pairs_s1 = [
        ("Химиялық элемент дегеніміз не?",
         "Химиялық элемент — бір типті атомдардың жиынтығы.",
         1, "definition"),
        ("Қазіргі уақытта неше химиялық элемент белгілі?",
         "Бүгінгі күні 118-ден астам химиялық элемент белгілі.",
         1, "definition"),
        ("Элемент атомын немен ажыратады?",
         "Элемент атомын ядродағы протондар санымен (атомдық нөмірмен) ажыратады.",
         3, "reasoning"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s1, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=1, subtopic_name=s1, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    # Symbol lookups — these are the "factual slot" pairs that fix
    # audit cases like #15 (formulas) and #16 (ag).
    #
    # Genitive forms are hand-authored to preserve Kazakh vowel
    # harmony + voicing assimilation. Programmatic suffix selection
    # by last-letter alone gets cases like "Азот" wrong (back-vowel
    # → -тың, not -тің) and "Натрий" wrong (after -й → -дің, not
    # -нің). Authoring the genitive explicitly is cheaper and more
    # reliable than implementing the full FST rule here.
    s2 = "Элементтердің таңбалары"
    # (name, symbol, genitive_of_name, vocative_lowercase_genitive)
    symbols = [
        ("Сутегі",   "h",  "Сутегінің",   "сутегінің"),
        ("Оттегі",   "o",  "Оттегінің",   "оттегінің"),
        ("Азот",     "n",  "Азоттың",     "азоттың"),
        ("Көміртек", "c",  "Көміртектің", "көміртектің"),
        ("Күкірт",   "s",  "Күкірттің",   "күкірттің"),
        ("Фосфор",   "p",  "Фосфордың",   "фосфордың"),
        ("Натрий",   "na", "Натрийдің",   "натрийдің"),
        ("Калий",    "k",  "Калийдің",    "калийдің"),
        ("Кальций",  "ca", "Кальцийдің",  "кальцийдің"),
        ("Магний",   "mg", "Магнийдің",   "магнийдің"),
        ("Алюминий", "al", "Алюминийдің", "алюминийдің"),
        ("Темір",    "fe", "Темірдің",    "темірдің"),
        ("Мыс",      "cu", "Мыстың",      "мыстың"),
        ("Мырыш",    "zn", "Мырыштың",    "мырыштың"),
        ("Күміс",    "ag", "Күмістің",    "күмістің"),
        ("Алтын",    "au", "Алтынның",    "алтынның"),
        ("Сынап",    "hg", "Сынаптың",    "сынаптың"),
        ("Қорғасын", "pb", "Қорғасынның", "қорғасынның"),
        ("Хлор",     "cl", "Хлордың",     "хлордың"),
        ("Бром",     "br", "Бромның",     "бромның"),
    ]
    qa_idx = 0
    for kaz_name, symbol, gen_name, gen_lower in symbols:
        qa_idx += 1
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=2, subtopic_name=s2, qa_order=qa_idx,
                  question=f"{gen_name} химиялық таңбасы қандай?",
                  answer=f"{gen_name} химиялық таңбасы — {symbol}.",
                  difficulty=1, question_type="formula",
                  source="curated")
        qa_idx += 1
        # Reverse direction: given symbol, ask which element
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=2, subtopic_name=s2, qa_order=qa_idx,
                  question=f"{symbol} — қандай элементтің таңбасы?",
                  answer=f"{symbol} — {kaz_name.lower()} элементінің таңбасы.",
                  difficulty=1, question_type="formula",
                  source="curated")

    s3 = "Қарапайым заттар"
    pairs_s3 = [
        ("Сутегі газының формуласы қандай?",
         "Сутегі газының формуласы — H2.",
         1, "formula"),
        ("Оттегі газының формуласы қандай?",
         "Оттегі газының формуласы — O2.",
         1, "formula"),
        ("Азот газының формуласы қандай?",
         "Азот газының формуласы — N2.",
         1, "formula"),
        ("Хлор газының формуласы қандай?",
         "Хлор газының формуласы — Cl2.",
         1, "formula"),
        ("Озонның формуласы қандай?",
         "Озонның формуласы — O3.",
         2, "formula"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s3, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=3, subtopic_name=s3, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    return len(pairs_s1) + qa_idx + len(pairs_s3)


# ────────────────────────────────────────────────────────────────────
# Chemistry — Grade 8 — Topic 7: Су (Water)
# ────────────────────────────────────────────────────────────────────
#
# Curriculum order follows Atamura «Химия 8» (Усманова и др., 2018):
#   t7.s1  Судың құрамы              (composition)
#   t7.s2  Физикалық қасиеттер       (physical properties)
#   t7.s3  Химиялық қасиеттер        (chemical properties)
#   t7.s4  Су реакциялары            (reactions)
#   t7.s5  Табиғаттағы рөлі          (role in nature)
#
# Each subtopic generates 6-10 Q&A pairs with progression:
#   difficulty=1: definition / formula recall
#   difficulty=2: simple application
#   difficulty=3: reasoning / multi-step

def populate_himiya_g08_t07_water(conn: sqlite3.Connection) -> int:
    """Chemistry 8 / Topic 7: Su (Water). Returns count of pairs inserted."""
    G, SUBJ, T, TNAME = 8, "himiya", 7, "Су"

    # ──────────────────────────────────────────
    # Subtopic 1 — Композиция (composition)
    # ──────────────────────────────────────────
    s1 = "Судың құрамы"
    pairs_s1 = [
        ("Судың химиялық формуласы қандай?",
         "Судың химиялық формуласы — h2o.",
         1, "formula"),
        ("Су қандай атомдардан тұрады?",
         "Су молекуласы екі сутегі атомынан және бір оттегі атомынан тұрады.",
         1, "definition"),
        ("Бір су молекуласында неше атом бар?",
         "Бір су молекуласында үш атом бар: екі сутегі (h) және бір оттегі (o).",
         1, "definition"),
        ("Судың молекулалық массасы қанша?",
         "Судың молекулалық массасы — 18 (2 × 1 + 16 = 18).",
         2, "calculation"),
        ("Судағы сутегі мен оттегінің массалық қатынасы қандай?",
         "Судағы сутегі мен оттегінің массалық қатынасы 1:8 (2 г сутегі — 16 г оттегі).",
         2, "calculation"),
        ("Су — қарапайым зат па, әлде күрделі зат па?",
         "Су — күрделі зат, өйткені екі түрлі элементтен (сутегі мен оттегі) тұрады.",
         2, "reasoning"),
        ("h2o формуласындағы «2» саны нені білдіреді?",
         "h2o формуласындағы «2» индексі — молекулада сутегі атомының саны екеу екенін білдіреді.",
         2, "definition"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s1, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=1, subtopic_name=s1, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    # ──────────────────────────────────────────
    # Subtopic 2 — Физикалық қасиеттер
    # ──────────────────────────────────────────
    s2 = "Судың физикалық қасиеттері"
    pairs_s2 = [
        ("Таза судың түсі қандай?",
         "Таза су — түссіз сұйықтық.",
         1, "definition"),
        ("Судың иісі бар ма?",
         "Таза судың иісі де, дәмі де жоқ.",
         1, "definition"),
        ("Су қандай температурада қайнайды?",
         "Қалыпты атмосфералық қысымда су 100 °c-та қайнайды.",
         1, "formula"),
        ("Су қандай температурада қатады?",
         "Қалыпты атмосфералық қысымда су 0 °c-та қатады.",
         1, "formula"),
        ("Судың тығыздығы қанша?",
         "4 °c температурада судың тығыздығы 1 г/см3 (1000 кг/м3).",
         1, "formula"),
        ("Су үш агрегаттық күйде кездесе ала ма?",
         "Иә, су үш агрегаттық күйде кездеседі: қатты (мұз), сұйық (су) және газ тәрізді (бу).",
         2, "definition"),
        ("Мұз неліктен судың бетінде қалқиды?",
         "Мұздың тығыздығы судан төмен (0,92 г/см3), сондықтан мұз судың бетінде қалқиды.",
         3, "reasoning"),
        ("Жылы судың салқын суға қарағанда тығыздығы қандай?",
         "Жылы судың тығыздығы салқын судан төмен (4 °c-тан жоғары температурада).",
         3, "reasoning"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s2, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=2, subtopic_name=s2, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    # ──────────────────────────────────────────
    # Subtopic 3 — Химиялық қасиеттер
    # ──────────────────────────────────────────
    s3 = "Судың химиялық қасиеттері"
    pairs_s3 = [
        ("Су қандай металдармен реакцияға түседі?",
         "Су активті металдармен (натрий na, калий k, кальций ca) реакцияға түседі.",
         2, "definition"),
        ("Натрий сумен реакцияға түскенде не пайда болады?",
         "Натрий сумен реакцияға түскенде сутегі (H2) және натрий гидроксиді (NaOH) пайда болады.",
         2, "formula"),
        ("Натрийдің сумен реакциясының теңдеуі қандай?",
         "2 na + 2 h2o → 2 NaOH + H2",
         3, "formula"),
        ("Су негіздік оксидтермен реакцияға түсе ме?",
         "Иә, су негіздік оксидтермен реакцияға түсіп негіздер түзеді: CaO + h2o → ca(OH)2.",
         3, "formula"),
        ("Су қышқылдық оксидтермен реакцияға түсе ме?",
         "Иә, су қышқылдық оксидтермен реакцияға түсіп қышқылдар түзеді: SO3 + h2o → H2SO4.",
         3, "formula"),
        ("Көмірқышқыл газы сумен қалай әрекеттеседі?",
         "Көмірқышқыл газы сумен әрекеттесіп көмір қышқылын түзеді: co2 + h2o → H2CO3.",
         3, "formula"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s3, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=3, subtopic_name=s3, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    # ──────────────────────────────────────────
    # Subtopic 4 — Су реакциялары (electrolysis etc.)
    # ──────────────────────────────────────────
    s4 = "Су реакциялары"
    pairs_s4 = [
        ("Судың электролизі дегеніміз не?",
         "Судың электролизі — электр тогының әсерімен суды сутегі мен оттегіге ыдырату үрдісі.",
         2, "definition"),
        ("Судың электролиз реакциясының теңдеуі қандай?",
         "2 h2o → 2 H2 + O2",
         3, "formula"),
        ("Судың электролизінде қандай газдар бөлінеді?",
         "Судың электролизінде сутегі (H2) және оттегі (O2) газдары бөлінеді.",
         2, "definition"),
        ("Электролизде сутегі мен оттегінің көлемдік қатынасы қандай?",
         "Электролизде сутегі мен оттегінің көлемдік қатынасы 2:1.",
         3, "calculation"),
        ("Гидролиз дегеніміз не?",
         "Гидролиз — заттың сумен әрекеттесіп ыдырау реакциясы.",
         2, "definition"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s4, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=4, subtopic_name=s4, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    # ──────────────────────────────────────────
    # Subtopic 5 — Табиғаттағы рөлі
    # ──────────────────────────────────────────
    s5 = "Судың табиғаттағы рөлі"
    pairs_s5 = [
        ("Жер бетінің қанша пайызын су алып жатыр?",
         "Жер бетінің шамамен 71 пайызын су алып жатыр.",
         1, "definition"),
        ("Адам ағзасы неше пайыз судан тұрады?",
         "Адам ағзасы шамамен 60–70 пайыз судан тұрады.",
         1, "definition"),
        ("Су айналымы дегеніміз не?",
         "Су айналымы — судың табиғатта булану, конденсация, жауын-шашын түрінде үздіксіз ауысуы.",
         2, "definition"),
        ("Тұщы су қайдан табылады?",
         "Тұщы су өзендерде, көлдерде, мұздықтарда және жер асты суларында болады.",
         1, "definition"),
        ("Тірі организмдер үшін судың маңызы қандай?",
         "Су — тіршіліктің негізі: ол ағзада еріткіш қызметін атқарады, температураны реттейді, заттарды тасымалдайды.",
         3, "reasoning"),
    ]
    for i, (q, a, diff, qtype) in enumerate(pairs_s5, start=1):
        insert_qa(conn, grade=G, subject=SUBJ, topic_order=T, topic_name=TNAME,
                  subtopic_order=5, subtopic_name=s5, qa_order=i,
                  question=q, answer=a, difficulty=diff, question_type=qtype,
                  source="curated")

    return (len(pairs_s1) + len(pairs_s2) + len(pairs_s3)
            + len(pairs_s4) + len(pairs_s5))


# ────────────────────────────────────────────────────────────────────
# CLI entry-point
# ────────────────────────────────────────────────────────────────────

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", default=str(DB_PATH))
    parser.add_argument("--stats-only", action="store_true",
                        help="Just print DB stats, don't insert.")
    args = parser.parse_args()

    db_path = Path(args.db)
    conn = open_db(db_path)

    if not args.stats_only:
        n1 = populate_himiya_g08_t01_atom(conn)
        print(f"[curriculum] inserted {n1} pairs into himiya/g08/t01 (Атом)")
        n2 = populate_himiya_g08_t02_elements(conn)
        print(f"[curriculum] inserted {n2} pairs into himiya/g08/t02 (Элементтер)")
        n7 = populate_himiya_g08_t07_water(conn)
        print(f"[curriculum] inserted {n7} pairs into himiya/g08/t07 (Су)")
        conn.commit()

    # Stats
    cur = conn.cursor()
    total = cur.execute("SELECT COUNT(*) FROM curriculum").fetchone()[0]
    by_subj = cur.execute(
        "SELECT subject, grade, COUNT(*) FROM curriculum "
        "GROUP BY subject, grade ORDER BY subject, grade"
    ).fetchall()
    db_size = db_path.stat().st_size if db_path.exists() else 0
    print(f"[curriculum] db: {db_path}")
    print(f"[curriculum] size: {db_size} bytes ({db_size/1024:.1f} KB)")
    print(f"[curriculum] total pairs: {total}")
    print("[curriculum] by subject/grade:")
    for subj, grade, n in by_subj:
        print(f"    {subj:12s} grade={grade:2d}: {n} pairs")

    conn.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
