//! **v4.93.5** — Codex 2026-05-07 audit P2: curated pedagogical
//! content for the four new tutor intents (AskExercise / CodeRequest
//! / ExplainCompilerError / AskPurpose).
//!
//! Content is **curated**, not generated. Per Codex directive: "не
//! пытаться свободно генерировать код без проверки" — adam never
//! free-generates code; it surfaces vetted snippets and exercises
//! or honestly responds "I don't have a curated example for this
//! topic yet".
//!
//! Coverage starts narrow: ~10-15 topics each, focused on Rust
//! basics (ownership / borrow / lifetimes / traits / Result / Option
//! / Future / async / Stream / pin / hello world). Subsequent
//! releases extend the maps in lockstep with the Rust + Async Book
//! corpus already covered (rust_001…693).

/// Return a curated exercise prompt for the given technical topic
/// in canonical lowercase form. None when the topic isn't covered
/// — the planner falls back to a generic "which topic would you
/// like to practice?" template in that case.
pub fn exercise_for(topic: &str) -> Option<&'static str> {
    match topic {
        "ownership" => Some(
            "Мына кодта қате бар: `let s = String::from(\"hello\"); let s2 = s; println!(\"{s}\");` — \
            компилятор не білдіреді? Қалай түзетесіз? (Иеленудің бір ережесін еске түсіріңіз: \
            бір мән — бір ғана иеленуші.)",
        ),
        "borrow" | "borrowing" | "қарызға алу" => Some(
            "Мына функция қандай қате береді? Түзетіп беріңіз:\n\
            ```rust\nfn main() {{ let s = String::from(\"hi\"); change(&s); }}\nfn change(s: &String) {{ s.push_str(\" world\"); }}\n```\n\
            Кеңес: `&s` тек оқу үшін; жазу үшін қандай белгі керек?",
        ),
        "lifetime" | "lifetimes" | "тіршілік мерзімі" => Some(
            "Мына функцияға дұрыс lifetime annotation қойыңыз:\n\
            ```rust\nfn longer(a: &str, b: &str) -> &str {{ if a.len() > b.len() {{ a }} else {{ b }} }}\n```\n\
            Кеңес: екі параметр де қайтарылатын мәннің өмір сүруі шегін шектейді.",
        ),
        "trait" | "traits" | "trait composition" => Some(
            "`Summary` trait жариялаңыз: `summarize(&self) -> String` әдісі бар. \
            Кейін оны `Article { title, body }` структурасына іске асырыңыз. \
            (Кеңес: `impl Trait for Type` синтаксисі.)",
        ),
        "result" => Some(
            "Мына функция екі санды бөледі. Қате болса `Result<f64, String>` қайтарсын:\n\
            ```rust\nfn divide(a: f64, b: f64) -> ??? {{ /* нөлге бөлуге қарсы тексеру */ }}\n```\n\
            Шакыру кезінде `?` операторы көмектесе алады.",
        ),
        "option" => Some(
            "`first_word(&str) -> Option<&str>` функция жазыңыз — жолдағы алғашқы сөзді қайтарсын. \
            Бос жол `None` болсын. (`split_whitespace().next()` пайдалы.)",
        ),
        "match" | "match өрнегі" => Some(
            "`enum Coin { Penny, Nickel, Dime, Quarter }` берілген. \
            `value(coin: Coin) -> u32` функциясын `match` арқылы жазыңыз: \
            Penny=1, Nickel=5, Dime=10, Quarter=25.",
        ),
        "iterator" => Some(
            "Бір санның квадратын есептейтін iterator пайдаланып, 1..=5-тің квадраттарын жинаңыз:\n\
            ```rust\nlet squares: Vec<i32> = (1..=5).map(|x| ???).collect();\n```\n\
            Күтілетін нәтиже: `[1, 4, 9, 16, 25]`.",
        ),
        "closure" => Some(
            "`apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32` функциясын жазыңыз. \
            Шакыру: `apply(|n| n * 2, 5)` — 10 қайтарсын.",
        ),
        "future" => Some(
            "Мына `async fn` ішіне `Future` күтуді қосыңыз:\n\
            ```rust\nasync fn fetch_and_print(url: &str) {{ /* http_get(url).await ... */ }}\n```\n\
            Қандай типті `http_get` қайтарады?",
        ),
        "async" | "async fn" | "await" | ".await оператор" => Some(
            "`async fn double(n: u32) -> u32 { n * 2 }` функциясын `main`-нен қалай шакырамыз? \
            (Кеңес: `#[tokio::main]` атрибуті + `.await`.)",
        ),
        "tokio" | "tokio runtime" => Some(
            "Tokio-да 1 секунд күтіп, «hello» басып шығаратын қарапайым программа жазыңыз. \
            (`tokio::time::sleep(Duration::from_secs(1)).await` пайдалы.)",
        ),
        "stream" | "stream трейт сигнатурасы" => Some(
            "Vec<u32>-тен Stream жасап, әр элементті 2-ге көбейтіп collect жасайтын pipeline жазыңыз. \
            (Кеңес: `futures::stream::iter` + `.map(...)` + `.collect::<Vec<_>>().await`.)",
        ),
        "pin" | "pin шешімі" => Some(
            "`Pin<Box<T>>` мен `Pin<&mut T>` арасындағы айырмашылық не? \
            Қашан heap-pin (Box::pin) керек, қашан stack-pin (`pin!` макросы) ыңғайлы?",
        ),
        "hello world" => Some(
            "Cargo жобасы жасаңыз: `cargo new my_first` — кейін `src/main.rs`-те «Сәлем, әлем!» басып шығарыңыз. \
            `cargo run` арқылы орындау.",
        ),
        // **v4.94.5** — extension batch: more Rust core topics.
        "vec" | "vector" => Some(
            "`Vec<i32>` жасаңыз: `vec![1, 2, 3, 4, 5]`. `iter()` арқылы әр элементтің квадратын есептеп жаңа Vec-қа жинаңыз. \
            (Кеңес: `.map(|x| x * x).collect::<Vec<_>>()`.)",
        ),
        "string" => Some(
            "`String::new()` арқылы бос жол жасап, `push_str` мен `push` арқылы «Сәлем, ` + ат + `!» құрастырыңыз. \
            (Кеңес: `let mut s = String::new(); s.push_str(\"...\");`.)",
        ),
        "struct" => Some(
            "`Person { name: String, age: u32 }` структурасын жариялаңыз. \
            `new(name: &str, age: u32) -> Self` constructor-ын `impl Person {{ }}` блокта жазыңыз. \
            Қалай шакырамыз?",
        ),
        "enum" => Some(
            "`enum TrafficLight { Red, Yellow, Green }` жариялаңыз. \
            `time_seconds(&self) -> u32` әдісі: Red=30, Yellow=5, Green=25. \
            `match` арқылы іске асырыңыз.",
        ),
        "error handling" => Some(
            "Файлды оқып, оның байт-санын қайтаратын функция жазыңыз: \
            `fn count_bytes(path: &str) -> Result<usize, std::io::Error>`. \
            Қате болса `?` арқылы тарату. Үлгі: `fs::read(path).map(|b| b.len())`.",
        ),
        "hashmap" => Some(
            "Vec-тегі сөздер тізімінен `HashMap<String, u32>` жасаңыз — әр сөздің пайда болу санын есептейді. \
            (Кеңес: `*counter.entry(word).or_insert(0) += 1`.)",
        ),
        "module" | "module жолы" => Some(
            "`src/lib.rs` ішінде `mod math { pub fn add(a: i32, b: i32) -> i32 { a + b } }` модулін жазып, \
            `tests/integration.rs`-тен `use mycrate::math; math::add(2, 3)` арқылы шакырыңыз.",
        ),
        "test" | "тест" => Some(
            "`#[cfg(test)] mod tests { use super::*; #[test] fn it_works() { assert_eq!(add(2, 2), 4); } }` \
            пайдаланып, `add` функциясын тексеретін тест жазыңыз. `cargo test` арқылы жүгірту.",
        ),
        "box" | "box<t>" => Some(
            "Recursive enum жариялаңыз: `enum List { Cons(i32, Box<List>), Nil }`. \
            Бір тізім құрастырыңыз: `Cons(1, Box::new(Cons(2, Box::new(Nil))))`. \
            Box не үшін керек?",
        ),
        "rc" | "rc<t>" => Some(
            "`Rc<String>` жасап, екі жерде clone-мен бөлісіңіз. \
            `Rc::strong_count(&rc)` арқылы санды тексеріңіз. (Кеңес: `Rc::clone(&rc)`).",
        ),
        "arc" | "arc<t>" => Some(
            "`Arc<Mutex<Vec<i32>>>` жасап, екі ағында оған push жасаңыз. \
            `thread::spawn` + `Arc::clone` + `lock().unwrap().push(...)` пайдалану.",
        ),
        "thread" | "ос ағыны" => Some(
            "`thread::spawn(|| { println!(\"hi from thread\"); })` arқылы екі ағын ашып, әрқайсысында 1..=5 басып шығарыңыз. \
            Әр ағынды `join()` арқылы аяқтап күтіңіз.",
        ),
        "channel" | "mpsc::channel" => Some(
            "`mpsc::channel()` жасап, бөлек ағыннан 5 хабар жіберіңіз. \
            Негізгі ағында `for received in rx` арқылы оларды басып шығарыңыз.",
        ),
        "join!" | "join! макросы" => Some(
            "Екі async функция параллель шакырыңыз: `let (a, b) = tokio::join!(fetch_a(), fetch_b());`. \
            Sequential `.await`-пен айырмашылығы — қанша уақыт үнемдейді?",
        ),
        "select!" | "select! макросы" => Some(
            "`tokio::select!` арқылы 5 секунд timeout мен фактілі жұмыс арасында race жасаңыз. \
            Қайсысы тез келсе — соның нәтижесі. (Кеңес: `tokio::time::sleep` + `_ = sleep(Duration::from_secs(5)) => ...`).",
        ),
        "smart pointer" | "ақылды сілтеме" => Some(
            "`Box<T>` / `Rc<T>` / `RefCell<T>` арасындағы айырмашылықты түсіндіріп, әрқайсысына бір қарапайым мысал жазыңыз. \
            (Box — heap-те бекіту; Rc — shared ownership; RefCell — interior mutability.)",
        ),
        _ => None,
    }
}

/// **v4.99.5** — adaptive-difficulty wrapper around [`exercise_for`].
/// When the curriculum-stage progress signal is [`Easy`] or [`Hard`],
/// returns a stage-tailored variant; falls back to the canonical
/// [`exercise_for`] content when the signal is [`Normal`] or no
/// difficulty-tuned variant exists for the topic.
///
/// Coverage: the 5 canonical curriculum stages — `ownership`,
/// `borrow`, `lifetime`, `traits`, `async`. Other topics (closure,
/// thread, etc.) ignore the hint and surface the existing
/// `exercise_for` content.
///
/// [`Easy`]: crate::curriculum::DifficultyHint::Easy
/// [`Hard`]: crate::curriculum::DifficultyHint::Hard
/// [`Normal`]: crate::curriculum::DifficultyHint::Normal
pub fn exercise_for_with_hint(
    topic: &str,
    hint: crate::curriculum::DifficultyHint,
) -> Option<&'static str> {
    use crate::curriculum::DifficultyHint;

    // Canonicalise the topic to its curriculum-stage id (the stage
    // tables in v1 are keyed off these short names).
    // **v5.2.5** — Codex round-3 audit Bug 3. Kazakh aliases for all
    // 5 canonical stages added so Kazakh-first input («иелік
    // бойынша жаттығу беріңізші») resolves to the curriculum stage.
    let stage = match topic {
        "ownership" | "иелік" => Some("ownership"),
        "borrow" | "borrowing" | "қарыз алу" | "қарызға алу" | "қарыз" => {
            Some("borrow")
        }
        "lifetime" | "lifetimes" | "тіршілік мерзімі" | "өмір кезеңі" | "өмір сүру кезеңі" => {
            Some("lifetime")
        }
        "trait" | "traits" | "trait composition" | "қасиеттер" | "қасиет" => {
            Some("traits")
        }
        "async" | "async fn" | "await" | ".await оператор" | "асинхронды" | "асинхрон" => {
            Some("async")
        }
        _ => None,
    };

    let tailored: Option<&'static str> = match (stage, hint) {
        (Some("ownership"), DifficultyHint::Easy) => Some(
            "Қарапайым иелік мысалы: `let s = String::from(\"hi\");` деп жариялаңыз да, \
            `println!(\"{s}\");` арқылы экранға шығарыңыз. Бұл — иелік ережесін бұзбайтын ең қарапайым ағын. \
            (Кеңес: `s` иесі осы блоктың соңына дейін.)",
        ),
        (Some("ownership"), DifficultyHint::Hard) => Some(
            "Иелік пен функция тіркесімі. Мына кодты түзетіңіз:\n\
            ```rust\nfn main() {{ let s = String::from(\"hi\"); take(s); take(s); }}\nfn take(s: String) {{ println!(\"{}\", s); }}\n```\n\
            Қандай қателер шығады? `take` функциясы ішкі көшірмемен жұмыс істесе қалай түзу керек? \
            Екі шешімді ұсыныңыз: біріншісі — `Clone`, екіншісі — reference.",
        ),
        (Some("borrow"), DifficultyHint::Easy) => Some(
            "Reference арқылы оқу: бір функция жазыңыз — `print_len(s: &String)` — \
            String-нің ұзындығын экранға шығарсын. Шакыру кезінде `print_len(&my_string)` беріңіз. \
            (Кеңес: `&` тек оқу үшін, иелік алынбайды.)",
        ),
        (Some("borrow"), DifficultyHint::Hard) => Some(
            "Бірнеше reference: мына функцияны жазыңыз:\n\
            ```rust\nfn first_two_words(s: &String) -> (Option<&str>, Option<&str>) {{ /* ... */ }}\n```\n\
            `&str` slices қайтарсын — alma бір рет, екінші рет; екеуі де `s`-тен қарызға алынады. \
            Бір сәтте бір ғана `&mut` болатынын қалай тексересіз?",
        ),
        (Some("lifetime"), DifficultyHint::Easy) => Some(
            "Static lifetime: `fn forever() -> &'static str` жазыңыз — \
            кез келген литерал-жолды қайтарсын. `'static` нені білдіреді? \
            (Кеңес: бағдарламаның бүкіл өмірі бойы жарамды.)",
        ),
        (Some("lifetime"), DifficultyHint::Hard) => Some(
            "Struct + lifetime parameter:\n\
            ```rust\nstruct Excerpt<'a> {{ part: &'a str }}\nimpl<'a> Excerpt<'a> {{ fn announce(&self) -> &str {{ /* ... */ }} }}\n```\n\
            `announce` дұрыс жұмыс істеуі үшін қандай lifetime annotations қажет? \
            Lifetime elision rule қашан жұмыс істейді?",
        ),
        (Some("traits"), DifficultyHint::Easy) => Some(
            "Бір trait + бір impl: `trait Greet {{ fn hello(&self); }}` жариялаңыз. \
            Кейін `struct Person {{ name: String }}` үшін `hello` әдісін іске асырыңыз — \
            ол `Сәлем, {{name}}!` басып шығарсын.",
        ),
        (Some("traits"), DifficultyHint::Hard) => Some(
            "Trait bounds + generic функция:\n\
            ```rust\nfn longest<T: ???>(items: &[T]) -> &T {{ /* ең үлкенін қайтарсын */ }}\n```\n\
            Қандай trait bound керек? `PartialOrd` мен `Ord` арасындағы айырмашылық не? \
            `T: Display` тағы керек пе, әлде жоқ па — неге?",
        ),
        (Some("async"), DifficultyHint::Easy) => Some(
            "Қарапайым async fn: `async fn greet() -> String { String::from(\"Сәлем!\") }` жазыңыз. \
            `main`-да `#[tokio::main]` атрибутымен `let g = greet().await;` шакырыңыз. \
            `.await` нені білдіреді?",
        ),
        (Some("async"), DifficultyHint::Hard) => Some(
            "Tokio + spawn + select. Екі async фон-міндетін параллель іске қосыңыз: \
            бірі — 1 секунд күтіп «А» басады, екіншісі — 2 секунд күтіп «Б». \
            `tokio::select!` арқылы біреуі бітсе бірден шығыңыз. \
            `tokio::spawn` мен `tokio::join!`-нің рөлі неде?",
        ),
        _ => None,
    };

    tailored.or_else(|| exercise_for(topic))
}

/// Return a curated code snippet for the given topic. Snippets are
/// minimal-correct Rust that compiles + runs. None when the topic
/// isn't covered.
pub fn code_snippet_for(topic: &str) -> Option<&'static str> {
    match topic {
        "hello world" => Some(
            "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```\n\
            Cargo жобасы үшін: `cargo new hello && cd hello && cargo run`.",
        ),
        "ownership" => Some(
            "```rust\nfn main() {\n    let s = String::from(\"hello\");\n    takes_ownership(s); // s енді жарамсыз\n    let x = 5;\n    makes_copy(x); // x әлі жарамды (i32 — Copy)\n}\nfn takes_ownership(s: String) { println!(\"{s}\"); }\nfn makes_copy(x: i32) { println!(\"{x}\"); }\n```",
        ),
        "borrow" | "borrowing" => Some(
            "```rust\nfn main() {\n    let s = String::from(\"hello\");\n    let len = calculate_length(&s);\n    println!(\"{s} ({len})\");\n}\nfn calculate_length(s: &String) -> usize { s.len() }\n```",
        ),
        "result" => Some(
            "```rust\nuse std::fs::File;\nuse std::io::{self, Read};\n\nfn read_file(path: &str) -> Result<String, io::Error> {\n    let mut f = File::open(path)?;\n    let mut s = String::new();\n    f.read_to_string(&mut s)?;\n    Ok(s)\n}\n```",
        ),
        "option" => Some(
            "```rust\nfn first_word(s: &str) -> Option<&str> {\n    s.split_whitespace().next()\n}\nfn main() {\n    match first_word(\"сәлем әлем\") {\n        Some(w) => println!(\"first: {w}\"),\n        None => println!(\"empty\"),\n    }\n}\n```",
        ),
        "match" | "match өрнегі" => Some(
            "```rust\nenum Direction { North, South, East, West }\nfn describe(d: Direction) -> &'static str {\n    match d {\n        Direction::North => \"солтүстік\",\n        Direction::South => \"оңтүстік\",\n        Direction::East => \"шығыс\",\n        Direction::West => \"батыс\",\n    }\n}\n```",
        ),
        "iterator" => Some(
            "```rust\nfn main() {\n    let nums = vec![1, 2, 3, 4, 5];\n    let sum: i32 = nums.iter().filter(|&&x| x % 2 == 0).sum();\n    println!(\"жұп қосынды: {sum}\"); // 6\n}\n```",
        ),
        "trait" | "traits" => Some(
            "```rust\ntrait Greet { fn greet(&self) -> String; }\nstruct Kazakh;\nimpl Greet for Kazakh { fn greet(&self) -> String { \"Сәлем!\".into() } }\nfn main() { println!(\"{}\", Kazakh.greet()); }\n```",
        ),
        "async" | "async fn" => Some(
            "```rust\n#[tokio::main]\nasync fn main() {\n    let result = double(5).await;\n    println!(\"{result}\"); // 10\n}\nasync fn double(n: u32) -> u32 { n * 2 }\n```",
        ),
        "future" => Some(
            "```rust\nuse std::future::Future;\nuse std::pin::Pin;\nuse std::task::{Context, Poll};\n\nstruct Ready<T>(Option<T>);\nimpl<T: Unpin> Future for Ready<T> {\n    type Output = T;\n    fn poll(mut self: Pin<&mut Self>, _: &mut Context) -> Poll<T> {\n        Poll::Ready(self.0.take().unwrap())\n    }\n}\n```",
        ),
        "tokio" | "tokio runtime" => Some(
            "```rust\nuse tokio::time::{sleep, Duration};\n#[tokio::main]\nasync fn main() {\n    sleep(Duration::from_secs(1)).await;\n    println!(\"бір секунд өтті\");\n}\n```",
        ),
        "stream" => Some(
            "```rust\nuse futures::stream::{self, StreamExt};\n#[tokio::main]\nasync fn main() {\n    let s = stream::iter(1..=5);\n    let doubled: Vec<i32> = s.map(|x| x * 2).collect().await;\n    println!(\"{doubled:?}\"); // [2, 4, 6, 8, 10]\n}\n```",
        ),
        "vec" | "vector" => Some(
            "```rust\nfn main() {\n    let mut v: Vec<i32> = Vec::new();\n    v.push(1); v.push(2); v.push(3);\n    for x in &v { println!(\"{x}\"); }\n}\n```",
        ),
        "string" => Some(
            "```rust\nfn main() {\n    let mut s = String::from(\"Сәлем, \");\n    s.push_str(\"әлем!\");\n    println!(\"{s}\"); // Сәлем, әлем!\n}\n```",
        ),
        "struct" => Some(
            "```rust\nstruct Person { name: String, age: u32 }\nfn main() {\n    let p = Person { name: \"Дәулет\".into(), age: 30 };\n    println!(\"{} — {} жаста\", p.name, p.age);\n}\n```",
        ),
        // **v4.94.5** — extension batch.
        "enum" => Some(
            "```rust\nenum TrafficLight { Red, Yellow, Green }\n\nfn time_seconds(light: TrafficLight) -> u32 {\n    match light {\n        TrafficLight::Red => 30,\n        TrafficLight::Yellow => 5,\n        TrafficLight::Green => 25,\n    }\n}\n\nfn main() { println!(\"{}\", time_seconds(TrafficLight::Red)); }\n```",
        ),
        "error handling" => Some(
            "```rust\nuse std::fs;\n\nfn count_bytes(path: &str) -> Result<usize, std::io::Error> {\n    let bytes = fs::read(path)?;\n    Ok(bytes.len())\n}\n\nfn main() {\n    match count_bytes(\"Cargo.toml\") {\n        Ok(n) => println!(\"{n} байт\"),\n        Err(e) => eprintln!(\"қате: {e}\"),\n    }\n}\n```",
        ),
        "hashmap" => Some(
            "```rust\nuse std::collections::HashMap;\n\nfn main() {\n    let words = vec![\"бір\", \"екі\", \"бір\", \"үш\", \"екі\", \"бір\"];\n    let mut counts: HashMap<&str, u32> = HashMap::new();\n    for w in &words {\n        *counts.entry(w).or_insert(0) += 1;\n    }\n    println!(\"{counts:?}\"); // {\"бір\": 3, \"екі\": 2, \"үш\": 1}\n}\n```",
        ),
        "module" | "module жолы" => Some(
            "```rust\n// src/lib.rs\npub mod math {\n    pub fn add(a: i32, b: i32) -> i32 { a + b }\n    pub fn mul(a: i32, b: i32) -> i32 { a * b }\n}\n\n// tests/integration.rs\nuse mycrate::math;\n#[test] fn it_adds() { assert_eq!(math::add(2, 3), 5); }\n```",
        ),
        "test" | "тест" => Some(
            "```rust\nfn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test] fn it_works() { assert_eq!(add(2, 2), 4); }\n    #[test] #[should_panic] fn overflow() { let _: i32 = i32::MAX + 1; }\n}\n```",
        ),
        "box" | "box<t>" => Some(
            "```rust\nenum List { Cons(i32, Box<List>), Nil }\nuse List::*;\n\nfn main() {\n    let list = Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))));\n    fn print(l: &List) {\n        match l {\n            Cons(v, next) => { print!(\"{v} \"); print(next) }\n            Nil => println!(\"-\"),\n        }\n    }\n    print(&list); // 1 2 3 -\n}\n```",
        ),
        "rc" | "rc<t>" => Some(
            "```rust\nuse std::rc::Rc;\n\nfn main() {\n    let a = Rc::new(String::from(\"shared\"));\n    let b = Rc::clone(&a);\n    let c = Rc::clone(&a);\n    println!(\"count: {}\", Rc::strong_count(&a)); // 3\n    println!(\"{a} / {b} / {c}\");\n}\n```",
        ),
        "arc" | "arc<t>" => Some(
            "```rust\nuse std::sync::{Arc, Mutex};\nuse std::thread;\n\nfn main() {\n    let v = Arc::new(Mutex::new(Vec::new()));\n    let mut handles = vec![];\n    for i in 0..3 {\n        let v = Arc::clone(&v);\n        handles.push(thread::spawn(move || v.lock().unwrap().push(i)));\n    }\n    for h in handles { h.join().unwrap(); }\n    println!(\"{:?}\", v.lock().unwrap());\n}\n```",
        ),
        "thread" | "ос ағыны" => Some(
            "```rust\nuse std::thread;\n\nfn main() {\n    let h1 = thread::spawn(|| {\n        for i in 1..=3 { println!(\"thread1: {i}\"); }\n    });\n    let h2 = thread::spawn(|| {\n        for i in 1..=3 { println!(\"thread2: {i}\"); }\n    });\n    h1.join().unwrap();\n    h2.join().unwrap();\n}\n```",
        ),
        "channel" | "mpsc::channel" => Some(
            "```rust\nuse std::sync::mpsc;\nuse std::thread;\n\nfn main() {\n    let (tx, rx) = mpsc::channel();\n    thread::spawn(move || {\n        for msg in [\"бір\", \"екі\", \"үш\"] { tx.send(msg).unwrap(); }\n    });\n    for received in rx { println!(\"{received}\"); }\n}\n```",
        ),
        "join!" | "join! макросы" => Some(
            "```rust\nuse tokio::time::{sleep, Duration};\n\nasync fn fetch_a() -> u32 { sleep(Duration::from_secs(1)).await; 1 }\nasync fn fetch_b() -> u32 { sleep(Duration::from_secs(1)).await; 2 }\n\n#[tokio::main]\nasync fn main() {\n    let (a, b) = tokio::join!(fetch_a(), fetch_b()); // ~1с, не 2с\n    println!(\"{a} {b}\");\n}\n```",
        ),
        "select!" | "select! макросы" => Some(
            "```rust\nuse tokio::time::{sleep, Duration};\n\n#[tokio::main]\nasync fn main() {\n    tokio::select! {\n        result = work() => println!(\"done: {result}\"),\n        _ = sleep(Duration::from_secs(5)) => println!(\"timeout!\"),\n    }\n}\nasync fn work() -> u32 { sleep(Duration::from_secs(2)).await; 42 }\n```",
        ),
        "smart pointer" | "ақылды сілтеме" => Some(
            "```rust\nuse std::rc::Rc;\nuse std::cell::RefCell;\n\nfn main() {\n    let shared: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(vec![1, 2, 3]));\n    let a = Rc::clone(&shared);\n    a.borrow_mut().push(4);\n    println!(\"{:?}\", shared.borrow()); // [1, 2, 3, 4]\n}\n```",
        ),
        "lifetime" | "lifetimes" => Some(
            "```rust\nfn longer<'a>(a: &'a str, b: &'a str) -> &'a str {\n    if a.len() > b.len() { a } else { b }\n}\n\nfn main() {\n    let s1 = String::from(\"кіші\");\n    let s2 = String::from(\"үлкенірек\");\n    println!(\"{}\", longer(&s1, &s2));\n}\n```",
        ),
        _ => None,
    }
}

/// **v4.96.0** — Codex round-2 audit Bug 7: cross-language contrast
/// content. Returns a curated explanation of how a Rust concept
/// differs from / corresponds to its analogue in another language.
/// Lookup key is `(other_language_lowercase, rust_concept_lowercase)`.
/// None when the pair isn't curated — the planner falls back to a
/// generic «менде нақты салыстыру жоқ» template.
pub fn cross_language_contrast(other: &str, concept: &str) -> Option<&'static str> {
    match (other, concept) {
        ("python", "ownership") => Some(
            "Python-да Rust-тағыдай ownership жоқ. Python — garbage-collected тіл: айнымалылар \
            reference count + cycle detector арқылы тазаланады. Rust-тың иеленуі — статикалық, \
            компиляцияда тексеріледі; Python-дікі — runtime-да. Performance / safety айырмашылығы: \
            Rust-та use-after-free / data race компиляция қатесі, ал Python-да runtime-да жасырын.",
        ),
        ("python", "borrow") | ("python", "borrowing") => Some(
            "Python-да Rust-тағы borrow checker жоқ. Python барлық айнымалыны reference түрінде \
            ұстайды (immutable / mutable айырмашылығы тек тип бойынша); алу-беруге шек жоқ. \
            Rust-тың `&T` / `&mut T` аралас-аяқ ережесі (бір mutable XOR көп immutable) Python-да \
            жоқ — concurrent-mutation қаупі бар.",
        ),
        ("python", "lifetime") | ("python", "lifetimes") => Some(
            "Python-да lifetime annotation жоқ. Refcounted object-тер scope-тан шыққанша «тірі»; \
            scope аяқталысымен GC оларды босатады. Rust-тың lifetime — static type-system бөлігі, \
            dangling pointer қаупін компиляцияда жояды.",
        ),
        ("java", "ownership") => Some(
            "Java-да Rust-тағыдай ownership жоқ. Java — JVM garbage-collected: барлық object heap-те, \
            тазалау runtime-да. Rust-тың ownership — статикалық, GC-сіз. Java-да тек «final» reference \
            бар, ол тек қайта-байланыстыруды (rebinding) шектейді — иеленуді тасымайды.",
        ),
        ("java", "lifetime") | ("java", "lifetimes") => Some(
            "Java-да lifetime annotation жоқ. JVM GC объектінің өмір сүруін runtime-да басқарады. \
            Rust-тың lifetime — статика; «GC жоқ + dangling pointer-сіз» нәтижесін береді.",
        ),
        ("javascript", "async") | ("js", "async") => Some(
            "JavaScript-те де `async` / `await` бар, бірақ механика айырықша. JS — single-threaded \
            event loop; Promise-ты «hot» — жасалғанда жұмыс басталады. Rust Future — «cold» / lazy: \
            тек poll-ден кейін жұмыс істейді. JS Promise execution VM-нің ішінде; Rust Future-ге \
            executor (tokio) керек.",
        ),
        ("javascript", "future") | ("js", "future") => Some(
            "JS-те `Future` емес, `Promise` — eager (жасалғанда жұмыс басталады), Rust Future — \
            lazy (тек poll-ден кейін). JS Promise тек single-threaded event loop ішінде, Rust \
            Future кез-келген executor-да (single немесе multi-thread) орындалады.",
        ),
        ("go", "ownership") | ("golang", "ownership") => Some(
            "Go-да Rust-тағыдай ownership жоқ. Go — GC-language: айнымалылар heap-те, GC оларды \
            тазалайды. Goroutine-дар бір mutex / channel арқылы синхрондалады, бірақ data race \
            компиляцияда тексерілмейді (тек race detector runtime-да).",
        ),
        ("go", "async") | ("golang", "async") => Some(
            "Go-да `async / await` синтаксисі жоқ — оның орнына goroutine + channel. `go func()` \
            жаңа goroutine spawn-дайды (M:N scheduler), ал `chan T` арқылы хабар жіберіледі. \
            Rust async/await — explicit Future + executor, Go-нікі — implicit goroutine + scheduler. \
            Семантика жақын, синтаксис өзгеше.",
        ),
        ("c++", "ownership") | ("cpp", "ownership") => Some(
            "C++-та `unique_ptr` / `shared_ptr` бар — Rust-тың `Box<T>` / `Rc<T>`-нің idiomic \
            аналогтары. Бірақ C++-та compiler ownership-ті ENFORCE етпейді: `std::move` тыс \
            көшірмелеу әлі мүмкін, dangling reference — runtime UB. Rust-та сондай қателер \
            компиляцияда табылады.",
        ),
        ("c", "ownership") => Some(
            "C-да ownership концепциясы тілде жоқ. Жадты қолмен `malloc` / `free` арқылы басқару \
            керек, double-free / use-after-free қателері — runtime UB. Rust ownership дәл осы \
            мәселелерді шешуге арналған.",
        ),
        // Default: empty match → falls back to generic template.
        _ => None,
    }
}

/// Return a curated explanation for a Rust compiler error code
/// (canonical `E0xxx` uppercase form). None when the error isn't
/// covered.
pub fn explain_error_code(code: &str) -> Option<&'static str> {
    match code {
        "E0382" => Some(
            "**E0382 — use of moved value.** Сіз бір мәнді басқа функцияға (немесе айнымалыға) \
            берген болсаңыз — оның иеленуі (ownership) ауысты, ескі айнымалы енді жарамсыз. \
            Шешім: (1) `clone()` жасау — егер мәнді көшіруге болатын тип болса; (2) сілтеме `&value` \
            беру — иеленуді тасымау үшін; (3) кодтың логикасын — мәнді алдымен пайдаланып, кейін \
            тасу.",
        ),
        "E0277" => Some(
            "**E0277 — trait bound not satisfied.** Қолданылған тип қажетті трейтті іске асырмаған. \
            Мысалы: `println!(\"{}\", value)` талап етеді `Display` трейтін; `value`-ң типі оны \
            іске асырмаса — қате. Шешім: (1) `#[derive(Debug)]` + `{value:?}` пайдалану; (2) типке \
            қажетті трейтті қолмен `impl Display for Type` іске асыру.",
        ),
        "E0308" => Some(
            "**E0308 — mismatched types.** Күтілген типпен берілген тип сай емес. Compiler хабары \
            «expected X, found Y» түрінде нақты типтерді көрсетеді. Шешім: типтерді сәйкестендіру — \
            `as` cast (қарапайым типтер үшін), `From::from` түрлендіру, немесе функция қолтаңбасын \
            өзгерту.",
        ),
        "E0506" => Some(
            "**E0506 — cannot assign to X because it is borrowed.** Бір мәнді кейбір код қарызға \
            алып тұрғанда оны өзгертуге болмайды. Шешім: borrow scope-ын қысқарту — `{ }` блок \
            ішіне қою немесе `drop(borrow)` ашық шакыру; немесе мутацияны borrow аяқталғаннан \
            кейін жасау.",
        ),
        "E0596" => Some(
            "**E0596 — cannot borrow X as mutable.** Айнымалы `mut` емес, бірақ оны өзгертуге \
            тырысып жатырсыз. Шешім: айнымалыны `let mut x = ...` түрінде жариялау; функция \
            параметрі болса — `&mut X` түрінде алу.",
        ),
        "E0521" => Some(
            "**E0521 — borrowed data escapes outside of function.** Функция ішіндегі сілтеме \
            функциядан тыс жерде пайдаланылмақ — бұл lifetime ережесін бұзады. Шешім: оwned типті \
            қайтару (`String` орнына `&str`-ден); немесе lifetime annotation қосу.",
        ),
        "E0499" => Some(
            "**E0499 — cannot borrow X as mutable more than once.** Бір айнымалыға `&mut` сілтемесі \
            бір уақытта тек біреу болуы керек (Rust borrow checker ережесі). Шешім: бір mutable \
            borrow аяқталғанша екіншісін бастамау — scope-ты `{ }` блокпен ажырату немесе бірінші \
            borrow-ты ашық `drop` жасау.",
        ),
        "E0507" => Some(
            "**E0507 — cannot move out of borrowed content.** `&T` сілтеме арқылы алынған мәнді \
            move ету мүмкін емес — иеленуді тасу keruek екен, ал сілтеме иеленуді бермейді. \
            Шешім: (1) `clone()` жасау; (2) сілтеме арқылы пайдалану; (3) `Option::take()` /  \
            `mem::replace` сияқты ауыстыру әдістері.",
        ),
        "E0597" => Some(
            "**E0597 — X does not live long enough.** Сілтеме өзі сілтеген мәннен ұзақ өмір сүруге \
            тырысып жатыр. Мысалы: ішкі scope-та жасалған String-ке alyngan сілтеменi сыртқы scope-та \
            пайдалану. Шешім: (1) owned тип қайтару; (2) lifetime annotation қосу; (3) мәнді `static` \
            өмір сүруі бар жерге жылжыту.",
        ),
        // **v4.94.5** — extension batch.
        "E0599" => Some(
            "**E0599 — no method named X found for type Y.** Тип Y-да X деген әдіс жоқ. \
            Себептері: (1) типтың дұрыс емес — `Vec<i32>` орнына `[i32; 5]` алып отырмыз; (2) trait \
            импорт етілмеген — `use std::io::Read` керек; (3) generic constraint жетіспейді. \
            Шешім: compiler хабарының «help: items from traits can only be used if the trait is in scope» \
            бөліміне қарау — қандай `use` керек жазылған.",
        ),
        "E0432" => Some(
            "**E0432 — unresolved import.** Импорт жасалған модуль / тип / функция жоқ. \
            Себептері: (1) ат қате жазылған; (2) crate dependency Cargo.toml-да жоқ; (3) модуль `pub` емес. \
            Шешім: `cargo tree` арқылы тәуелділікті тексеру; `cargo check --message-format=json` арқылы дұрыс жолды табу.",
        ),
        "E0658" => Some(
            "**E0658 — use of unstable library feature.** Бұл функция тек nightly-да қолжетімді. \
            Шешім: (1) stable Rust-та бар балама іздеу; (2) `rustup default nightly` арқылы nightly-ге өту \
            + `#![feature(feature_name)]` атрибуті; (3) Rust release notes-те функция қашан тұрақтанатынын тексеру.",
        ),
        "E0463" => Some(
            "**E0463 — can't find crate for X.** Cargo dependencies-те crate жоқ немесе атау қате. \
            Шешім: `Cargo.toml`-дегі `[dependencies]` бөлімін тексеру; `cargo update` жүгірту; \
            crate атауындағы дефис вертuс underscore айырмашылығы (`tokio_util` vs `tokio-util`).",
        ),
        "E0061" => Some(
            "**E0061 — wrong number of arguments.** Функция X аргумент күтеді, бірақ Y берілді. \
            Шешім: функция қолтаңбасын тексеру (`cargo doc --open` арқылы); тупл / array-ды бөлектеп беру \
            мүмкін емес — әр аргументті жеке жазу керек.",
        ),
        _ => None,
    }
}

/// Return a curated purpose statement for a topic. None when not
/// covered — the planner falls back to "no purpose data" template.
pub fn purpose_for(topic: &str) -> Option<&'static str> {
    match topic {
        "ownership" => Some(
            "Иеленудің мақсаты — Rust бағдарламаларында жадыны қауіпсіз басқару. \
            Garbage collector-сыз да жад leak / use-after-free / data race орын алмайды — \
            өйткені әр мәннің бір ғана иеленушісі болады, иеленуші scope-тан шыққанда мән \
            автоматты түрде босатылады.",
        ),
        "borrow" | "borrowing" => Some(
            "Қарызға алудың мақсаты — иеленуді тасымай-ақ деректерге қол жеткізу. \
            Сілтеме (`&T` немесе `&mut T`) ту function-ке арналған параметрге беру кезінде \
            мәннің ескі иеленушіден жойылуын болдырмайды.",
        ),
        "lifetime" | "lifetimes" => Some(
            "Lifetime-тің мақсаты — сілтемелер өздері сілтейтін мәндерден ұзақ өмір сүрмеуі. \
            Compiler әр сілтеменің өмір сүру шегін статистикалық тексеріп, dangling pointer \
            қаупін компиляция кезінде жояды.",
        ),
        "trait" | "traits" => Some(
            "Trait-тің мақсаты — типтер арасында ортақ мінез-құлықты білдіру. Trait арқылы \
            generic функциялар әр түрлі типтермен бірдей жұмыс жасай алады; ал `dyn Trait` \
            арқылы — runtime polymorphism.",
        ),
        "future" => Some(
            "Future-нің мақсаты — асинхронды есептеуді типте білдіру. Future — болашақта \
            аяқталатын мәнді ұсынады; executor оны polls арқылы тоқтатпай-қозғалтпай орындайды.",
        ),
        "pin" | "pin шешімі" => Some(
            "Pin-нің мақсаты — мәнді жадтан жылжытуға тыйым салу. Async fn state machine-да \
            self-referential өрістер пайда болғанда (өріс іштегі басқа өріске сілтейтінде) — \
            move ескі pointer-лерді бұзады. Pin сол move-ке тыйым салып, async-ң қауіпсіз \
            жұмысын қамтамасыз етеді.",
        ),
        // **v4.94.5** — extension batch.
        "result" => Some(
            "Result-тың мақсаты — қателерді тілдік типпен білдіру (exception емес). \
            Compiler әр Result-ті өңдеуді мәжбүрлейді — қате жасырылмайды; \
            `?` операторы дұрыс тарату жолын ыңғайлы жасайды.",
        ),
        "option" => Some(
            "Option-ның мақсаты — null reference-тің орнына тілдік-типтік «бар/жоқ». \
            Tony Hoare null-ды «миллиардтық қате» деп атаған; Option compiler-арқылы None жағдайды \
            өңдеуді мәжбүрлейді — null pointer dereference Rust-та орын алмайды.",
        ),
        "match" | "match өрнегі" => Some(
            "Match-тың мақсаты — толық қамту (exhaustiveness) кепілдігі. Compiler барлық enum нұсқалары \
            өңделгенін статистикалық тексереді — жаңа нұсқа қосылса, сол кодты әртүрлі match-та қолданушы \
            компиляция қатесі арқылы хабардар болады.",
        ),
        "iterator" => Some(
            "Iterator-тың мақсаты — лазылы (lazy) элементтер тізбегі. `.map()`, `.filter()`, `.fold()` \
            chain-нің алғашқы intermediate Vec-тер жасамайды — тек соңғы `.collect()` / `.sum()` \
            болғанда жұмыс орындалады. Zero-cost абстракция: compiler барлығын бір циклге fuse жасайды.",
        ),
        "closure" => Some(
            "Closure-дің мақсаты — функцияны мән ретінде беру + ішкі айнымалыларды captures жасау. \
            Generic параметрлер (`F: Fn(i32) -> i32`) үшін zero-cost (compile-time monomorphisation); \
            динамикалық қажет болса — `Box<dyn Fn>` heap-те.",
        ),
        "async" | "async fn" => Some(
            "Async-тың мақсаты — I/O-bound concurrency бір ағында мыңдаған тапсырманы тиімді ұстау. \
            Әрбір tokio task ~1-2 KB жадын алады, ал OS thread ~2 MB. 100 000 connection бір процеске \
            негізделеді (LLM-сіз, GPU-сіз).",
        ),
        "tokio" | "tokio runtime" => Some(
            "Tokio-нің мақсаты — async runtime: executor (work-stealing scheduler) + reactor (epoll/kqueue/IOCP) \
            + I/O драйверлер + timer wheels біріктірілген пакет. Rust async/await тілдік синтаксис; tokio оны \
            нақты production-ready environment-ке айналдырады.",
        ),
        "stream" => Some(
            "Stream-нің мақсаты — async iterator. Element-тер бір-бірлеп, `next().await` арқылы алынады; \
            network stream / channel receiver / file reader — барлығы Stream-сай modeled. \
            StreamExt комбинаторлары `map / filter / fold / buffered` ыңғайлы chaining береді.",
        ),
        "vec" => Some(
            "Vec-тің мақсаты — өсетін heap-allocated массив. Capacity автоматты екі есе өседі (amortized O(1) push); \
            `iter()` / `iter_mut()` / `into_iter()` арқылы әр түрлі ownership стилінде итерация береді.",
        ),
        "hashmap" => Some(
            "HashMap-тың мақсаты — кілт-мән жұптарын О(1) орташа қолжетімділікпен сақтау. \
            Кілт `Hash + Eq` талап етеді; default hasher SipHash (HashDoS-тен қорғану) — performance \
            маңызды кодта `FxHashMap` (rustc-дағыдай) тиімдірек.",
        ),
        "thread" | "ос ағыны" => Some(
            "OS ағынының мақсаты — CPU-bound параллелизм: бірнеше CPU ядросын біруақытта пайдалану. \
            I/O-bound үшін async тиімдірек (~1 KB / task vs ~2 MB / thread). Ереже: CPU-bound = thread, \
            I/O-bound = async task.",
        ),
        "channel" | "mpsc::channel" => Some(
            "Channel-дің мақсаты — ағындар арасындағы хабар жіберу (без shared memory). \
            «Don't communicate by sharing memory; share memory by communicating» — Rob Pike. \
            mpsc — multiple-producer-single-consumer; broadcast — multiple-consumer.",
        ),
        "mutex" => Some(
            "Mutex-тің мақсаты — shared mutable state-ке бір уақытта тек бір ағынның қол жеткізуін қамтамасыз ету. \
            `lock()` MutexGuard қайтарады (RAII): scope аяқталысымен автоматты unlock. \
            Async-та tokio::sync::Mutex (yields task on lock conflict).",
        ),
        // (lifetime/lifetimes already covered above; v4.94.5 extension
        // for these kept its body but the duplicate arm was removed
        // to satisfy clippy's unreachable_patterns check.)
        _ => None,
    }
}
