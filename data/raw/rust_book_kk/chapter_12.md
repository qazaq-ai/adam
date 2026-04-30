# 12-тарау. Кіріс-шығыс жобасы: команда жолы бағдарламасын құру

Бұрын біз жекелеген ұғымдарды үйрендік. Енді оларды бір нақты бағдарламада
біріктірейік. Жоба — `grep` атты әйгілі Unix құралының кіші көшірмесі.
`grep` («Globally search a Regular Expression and Print») — файлдан
немесе енгізуден белгілі бір сөздерді табатын құрал. Біздің нұсқамыз
оны жеңілдетіп, бір файлдан жай ғана жолдарды іздейді.

Қандай ұғымдар бір жобаға жиналады:

- Кодты модульдерге бөлу (7-тарау).
- `Vec` пен `String` қолдану (8-тарау).
- Қателерді өңдеу (9-тарау).
- Трейттер мен тіршілік мерзімі (10-тарау).
- Сынақтар жазу (11-тарау).
- Орта айнымалылары мен стандартты шығаруларды басқару — жаңа.

Жолдан-жолдан өсіп шығатын мысал арқылы біз: жаман кодтан жақсы кодқа
қарай қалай дамуға болатынын; не үшін кодты бөлектеп, кітапханаға
шығаратынын; тестке негізделген әзірлеудің (TDD) қалай жұмыс
істейтінін көреміз.

## 12.1. Командалық жол аргументтерін қабылдау

Жаңа жоба — `cargo new minigrep` — жасайық:

```sh
$ cargo new minigrep
$ cd minigrep
```

`grep`-ке ұқсас, бағдарламаға екі аргумент керек: ізделетін сөз және
файл атауы. Командалық жол аргументтерін оқу үшін стандартты кітапхана
ұсынатын `std::env::args` функциясын қолданамыз:

```rust
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(&args);
}
```

`env::args()` итератор қайтарады; `.collect()` оны векторға айналдырады.
Векторды нақты `Vec<String>` ретінде жариялау — типті анық беру үшін.

Бағдарламаны екі аргументпен іске қосайық:

```sh
$ cargo run -- needle haystack
[src/main.rs:5] &args = [
    "target/debug/minigrep",
    "needle",
    "haystack",
]
```

Біріншісі — бағдарламаның өзінің атауы. Қалғандары — біз берген
аргументтер.

Аргументтерді нақты айнымалыларға сақтайық:

```rust
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let query = &args[1];
    let file_path = &args[2];

    println!("Іздеу: {}", query);
    println!("Файл: {}", file_path);
}
```

`&args[1]` — `String`-ке сілтеме. Бұл нұсқа — әзірге жұмыс істейді,
бірақ қатерлі: егер пайдаланушы аргументтер бермесе — `args[1]`
panic-қа барады. Кейінірек жөндейміз.

## 12.2. Файлды оқу

Іздеу үшін файлдың мазмұны керек. `std::fs::read_to_string` функциясы
файлды оқып, оның мазмұнын `String` ретінде қайтарады:

```rust
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let query = &args[1];
    let file_path = &args[2];

    println!("Іздеу: {}", query);
    println!("Файл: {}", file_path);

    let contents = fs::read_to_string(file_path)
        .expect("Файлды оқу мүмкін болмады");

    println!("Мазмұн:\n{}", contents);
}
```

Жоба түбіріне `poem.txt` атты файл жасайық:

```text
I'm nobody! Who are you?
Are you nobody, too?
Then there's a pair of us — don't tell!
They'd banish us, you know.

How dreary to be somebody!
How public, like a frog
To tell your name the livelong day
To an admiring bog!
```

`cargo run -- the poem.txt` орындағанда:

```text
Іздеу: the
Файл: poem.txt
Мазмұн:
I'm nobody! Who are you?
...
```

Жұмыс істейді. Бірақ код әлі бір функцияда — оны қайта ұйымдастыру
керек.

## 12.3. Модульдік және қате өңдеуді жақсартуға қайта құрастыру

Біздің `main` бүкіл логиканы орындап тұр: аргумент талдау, файл оқу, көп
жайт. Бұл бірнеше мәселеге апарады:

1. **Жауапкершіліктерді бөлмеу.** Жұмыс үстелі үлкейген сайын — бір
   функция көп нәрсеге жауапты болғанда — кодты қолдау қиынырақ.
2. **Қате өңдеу нашар.** `expect` дөрекі: пайдаланушыға техникалық
   хабарлама шығады, бар адам оның мағынасын түсінбейді.
3. **Тестілеу қиын.** `main` функциясы тек байт-байт жазылғанда жұмыс
   істейді. Оны бөлектеген кезде ғана сынақ жазу мүмкін болады.

Қайта құрастырудың жоспары:

1. Аргумент талдауды бөлек функцияға шығару.
2. Конфигурация мәндерін бір структқа жинау.
3. `Config::build` атты «конструкторды» жасау.
4. Қате өңдеуді — `Result` арқылы.
5. Логиканы `main`-нен `lib.rs`-ке шығару.

### Аргумент талдауды бөлектеу

```rust
fn main() {
    let args: Vec<String> = env::args().collect();
    let (query, file_path) = parse_config(&args);
    // ... қалған бөлік
}

fn parse_config(args: &[String]) -> (&str, &str) {
    let query = &args[1];
    let file_path = &args[2];
    (query, file_path)
}
```

Қазір аргумент талдау бөлек функцияда. Бірақ кортежпен қайтару — ыңғайсыз:
қандай мән — `query`, қандай мән — `file_path`? Атаулар жоғалды.

### Конфигурацияны структқа жинау

Кортеждің орнына — структ:

```rust
struct Config {
    query: String,
    file_path: String,
}

fn parse_config(args: &[String]) -> Config {
    let query = args[1].clone();
    let file_path = args[2].clone();
    Config { query, file_path }
}
```

`clone` арқылы біз `String`-тің көшірмесін жасап жатырмыз. Бұл — ұнамсыз
өнімділік-айыппұл. Бірақ `args`-тан қарызға алу әлдеқайда күрделі болар
еді — әзірге `clone` дұрыс таңдау. Қарапайымдылық — бірінші кезекте.

`main` енді мынадай:

```rust
fn main() {
    let args: Vec<String> = env::args().collect();
    let config = parse_config(&args);

    println!("Іздеу: {}", config.query);
    println!("Файл: {}", config.file_path);

    let contents = fs::read_to_string(&config.file_path)
        .expect("Файлды оқу мүмкін болмады");
    // ...
}
```

### Конструктор

`parse_config` функциясын `Config::build` әдісіне айналдырамыз —
конструктор стилі:

```rust
impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("аргументтер жетіспейді");
        }
        let query = args[1].clone();
        let file_path = args[2].clone();
        Ok(Config { query, file_path })
    }
}
```

Енді ол `Result` қайтарады. Аргументтер жетіспесе — анық қате;
жеткілікті болса — `Config`. `&'static str` — қате типі ретінде
тұрақты жол.

### Қателерді жөнді өңдеу

`main` ішінде:

```rust
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Аргумент қатесі: {err}");
        process::exit(1);
    });

    // ... қалған бөлік
}
```

`unwrap_or_else` — `Result`-та бар әдіс. `Ok` болса — мәнді шығарады.
`Err` болса — берген жабуды (closure) шақырады. Жабу қатені аргумент
ретінде алады, экранға шығарады, әрі бағдарламаны 1-кодпен тоқтатады.

`eprintln!` — `println!`-ге ұқсас, бірақ стандартты қатеге (stderr)
жазады, стандартты шығаруға (stdout) емес. Бұл — Unix дәстүрі: қалыпты
шығу — нәтиже; қателер — бөлек ағында.

### Логиканы `main`-нен бөлектеу

Бағдарлама үлкейіп бара жатса — `main` өте қарапайым болуы керек: тек
аргументтерді алу, қатені қайтару. Барлық басты логиканы `run` атты
функцияға шығарамыз:

```rust
fn main() {
    // ... аргумент талдау

    if let Err(e) = run(config) {
        eprintln!("Қолданба қатесі: {e}");
        process::exit(1);
    }
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.file_path)?;
    println!("Мазмұн:\n{contents}");
    Ok(())
}
```

`Box<dyn Error>` — кез келген қате типін қабылдайтын **трейт-нысан**.
Бұл — функция қайтаратын қате типін шектемей, кез келген «қателі
нәрсе»-ні қабылдай алатын тәсіл.

`run` нәтижесін `if let Err(e) = ...` арқылы өңдейміз.

### Кодты кітапхана сандығына шығару

Соңғы қадам — `Config` пен `run`-ды `lib.rs`-ке шығару, `main.rs`-ті
қарапайым ету.

`src/lib.rs`:

```rust
use std::error::Error;
use std::fs;

pub struct Config {
    pub query: String,
    pub file_path: String,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("аргументтер жетіспейді");
        }
        let query = args[1].clone();
        let file_path = args[2].clone();
        Ok(Config { query, file_path })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.file_path)?;
    println!("Мазмұн:\n{contents}");
    Ok(())
}
```

`src/main.rs`:

```rust
use std::env;
use std::process;

use minigrep::Config;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Аргумент қатесі: {err}");
        process::exit(1);
    });

    if let Err(e) = minigrep::run(config) {
        eprintln!("Қолданба қатесі: {e}");
        process::exit(1);
    }
}
```

Енді `main` тек арматура; бар логика — кітапханада. Ол сынауға қол
жетімді, басқа бағдарламаларға қайта пайдалануға болады.

## 12.4. TDD арқылы кітапхананың функцияларын дамыту

Әзірге іздеу логикасы жоқ — біз жай ғана файлдың мазмұнын экранға
шығарамыз. Енді нақты `search` функциясын — берілген сөзді ұстайтын
жолдарды табатынын — жазайық. Бірақ оны бөлек жолмен жазамыз: алдымен
сынақ, содан кейін код. Бұл — **тестке негізделген әзірлеу**
(test-driven development, TDD).

TDD циклі — үш қадам:

1. Жобаланатын мінезге сай тест жазу. Тест бастан кетеді (өйткені код
   әлі жоқ).
2. Сынақты өткізетін минимум кодты жазу.
3. Сынақ өткеннен соң — кодты жұмыс ұстатпай тазарту.

### Кетіп жатқан сынақты жазу

`src/lib.rs`-те:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_result() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents));
    }
}
```

Сынақ айтады: `search("duct", "...")` шакыруы `["safe, fast,
productive."]` векторын қайтаруы керек, өйткені бұл жалғыз жол `duct`
сөзін ұстайды.

`search` функциясы әлі жоқ. `cargo test` орындағанда — кетеді:

```text
error[E0425]: cannot find function `search` in this scope
```

Күтілген. Енді `search` функциясын жазамыз.

### Сынақты өткізетін кодты жазу

```rust
pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let mut results = Vec::new();

    for line in contents.lines() {
        if line.contains(query) {
            results.push(line);
        }
    }

    results
}
```

Тіршілік мерзімі `'a` — нәтиже векторы `contents`-ке сілтейтін
тілімдерді ұстайды. Сондықтан нәтиженің тіршілігі — `contents`-тің
тіршілігіне байлы.

Логика: жолдан-жолдан, әрбір жолда `query` бар-жоғын тексереміз; бар
болса — нәтижеге қосамыз.

`cargo test` орындасақ — өтеді!

### `run` ішінде `search` шакыру

Енді `run` функциясын жөндейміз:

```rust
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.file_path)?;

    for line in search(&config.query, &contents) {
        println!("{line}");
    }

    Ok(())
}
```

Іздеу нәтижесі — әр жолды басып шығарамыз. `cargo run -- the poem.txt`
орындасақ:

```text
Then there's a pair of us — don't tell!
To tell your name the livelong day
```

Жұмыс істейді — `the` сөзін ұстайтын жолдар.

## 12.5. Орта айнымалыларымен жұмыс істеу

Енді бір мүмкіндік қосайық: пайдаланушы әріп регистрін ескермей іздегісі
келсе ше? Біз оған **орта айнымалысы** — `IGNORE_CASE` — арқылы
басқартамыз.

### Регистрсіз іздеу үшін сынақ

`#[cfg(test)] mod tests`-те:

```rust
#[test]
fn case_insensitive() {
    let query = "rUsT";
    let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";

    assert_eq!(
        vec!["Rust:", "Trust me."],
        search_case_insensitive(query, contents)
    );
}
```

`Rust` пен `Trust` екеуі де `rUsT`-қа сай (регистрді ескермей).

### `search_case_insensitive` функциясын жазу

```rust
pub fn search_case_insensitive<'a>(
    query: &str,
    contents: &'a str,
) -> Vec<&'a str> {
    let query = query.to_lowercase();
    let mut results = Vec::new();

    for line in contents.lines() {
        if line.to_lowercase().contains(&query) {
            results.push(line);
        }
    }

    results
}
```

`to_lowercase` — әріптерді кіші регистрге айналдырады. Сонда `RuST` пен
`Rust` бір қалыпта тұрады.

Назар аударыңыз: `to_lowercase` жаңа `String` жасайды. Сондықтан `query`
енді `String`, ал `line.to_lowercase()` — әр итерацияда `String`. Бұл —
аздап өнімділікке зиян, бірақ дұрыстық — алдымен.

### `Config`-ке regі-set параметрін қосу

```rust
pub struct Config {
    pub query: String,
    pub file_path: String,
    pub ignore_case: bool,
}
```

`build` ішінде орта айнымалысын тексереміз:

```rust
use std::env;

impl Config {
    pub fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("аргументтер жетіспейді");
        }
        let query = args[1].clone();
        let file_path = args[2].clone();

        let ignore_case = env::var("IGNORE_CASE").is_ok();

        Ok(Config { query, file_path, ignore_case })
    }
}
```

`env::var("IGNORE_CASE")` — `Result` қайтарады. Орта айнымалысы
бар болса — `Ok`, жоқ болса — `Err`. `.is_ok()` — `true` или `false`.

`run` ішінде:

```rust
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.file_path)?;

    let results = if config.ignore_case {
        search_case_insensitive(&config.query, &contents)
    } else {
        search(&config.query, &contents)
    };

    for line in results {
        println!("{line}");
    }

    Ok(())
}
```

Тестілеу:

```sh
$ cargo run -- to poem.txt
Are you nobody, too?
How dreary to be somebody!

$ IGNORE_CASE=1 cargo run -- to poem.txt
Are you nobody, too?
How dreary to be somebody!
To tell your name the livelong day
```

Айырмашылық: екінші шакыруда `To` (бас әріппен) да табылды.

## 12.6. Қате хабарламаларын стандартты шығаруға емес, стандартты қатеге жазу

Қазіргі бағдарламамыз барлық шығаруларды (нәтижелерді *пен* қателерді)
`println!` арқылы шығарып отыр. Бұл — Unix дәстүрімен сай емес: қалыпты
шығу — стандартты шығаруға (stdout); қателер — стандартты қатеге
(stderr). Бөлек ағындар — пайдаланушыға бағдарламаны жоғары деңгейде
басқаруға мүмкіндік береді: нәтижелерді файлға бағыттай аласыз, бірақ
қателер экранда қалады.

### `eprintln!` пен `println!`

`eprintln!` — стандартты қатеге жазады. `println!` — стандартты шығаруға.
Біз бұған дейін `Аргумент қатесі: ...` пен `Қолданба қатесі: ...`
хабарламаларында `eprintln!`-ді қолдандық — олар қателер.

Енді тексерейік: бағдарламаның қалыпты шығаруын файлға жазайық.

```sh
$ cargo run -- to poem.txt > output.txt
```

`> output.txt` — стандартты шығаруды файлға бағыттайды. Файл `output.txt`
ішінде:

```text
Are you nobody, too?
How dreary to be somebody!
```

— табылған жолдар. Қателер де болса (мысалы, аргументтер жоқ болса):

```sh
$ cargo run > output.txt
```

Экранда:

```text
Аргумент қатесі: аргументтер жетіспейді
```

— `eprintln!` арқылы. Файл `output.txt` бос. Сондай дұрыс мінез-құлық
бар: қалыпты шығару бағытталған, қателер — экранда.

## 12.7. Тарау қорытындысы

Бұл тарауда сіз шынайы команда жолы бағдарламасын жасадыңыз. Жолда:

- Аргументтерді `env::args` арқылы оқу;
- Файлды `fs::read_to_string` арқылы оқу;
- Кодты `lib.rs`-ке бөлектеу;
- Қате-түрлерді `Result` арқылы өңдеу;
- `Config::build` атты конструктор стилін;
- TDD циклі арқылы `search` функциясын дамыту;
- Орта айнымалысын `env::var` арқылы оқу;
- `eprintln!` арқылы қателерді стандартты қатеге жазу.

Бұл — `Rust`-тың практикасында өнім жобасын құру керек болғанда сізге
керек болатын негізгі үлгілер. Кейінгі тарауларда осы жобаны функционал
бағдарламалаудың кейбір тәсілдерімен — итераторлар мен жабулармен —
қысқартуға қарастырамыз.

Мини-`grep` өзі — толық бағдарлама. Сіз оны өз есебіңізге жетілдіре
аласыз: реттік өрнектерді қолдау, бірнеше файлда іздеу, түсті шығару,
жол нөмірлерін көрсету. Әр осындай қосым — Rust-та жаттығудың
ыңғайлы әдісі.

Келесі тарауда — функционал стиль ұғымдары: жабулар (closures) мен
итераторлар. Олар сіздің кодыңызды әлдеқайда жинақы әрі тиімді ете
алады.
