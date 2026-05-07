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
        "trait" | "trait composition" => Some(
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
        _ => None,
    }
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
        "trait" => Some(
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
        "trait" => Some(
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
        _ => None,
    }
}
