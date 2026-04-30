# 20-тарау. Соңғы жоба: көп ағынды веб-сервер

`Rust` Book-тың соңғы тарауы. Бұған дейін біз ұсақ-түйекті ұғымдарды
үйрендік: иелік, типтер, трейттер, ағындар, каналдар, ақылды сілтемелер,
макростар. Енді — бәрін бір нақты жобада біріктіреміз. Біз кішкене,
бірақ толық — **көп ағынды веб-сервер** жасаймыз.

Жоба үш кезеңде өтеді:

1. Бір ағынды HTTP-сервер құру.
2. Оны көп ағынды етіп қайта құрастыру (ағын-жинағы арқылы).
3. Жайлап тоқтауды (graceful shutdown) жүзеге асыру.

Соңында сізде болатыны: TCP қосылымдарды қабылдайтын, HTTP
сұраныстарды талдайтын, HTML қайтаратын, көптеген клиентке параллельді
қызмет көрсететін, дұрыс түрде тоқтайтын бағдарлама. Күрделі жоқ —
бар оқыған ұғымдардың практикалық қолданысы.

## 20.1. Бір ағынды веб-сервер құру

`Rust`-тың стандартты кітапханасы TCP пен UDP-дан жұмыс жасайтын
типтерді ұсынады. HTTP — TCP үстіндегі мәтіндік протокол. Бізге
HTTP-кітапхана керек емес — өзіміз талдайтын боламыз. Шектелген, бірақ
жұмыс жасайтын нұсқа.

`cargo new hello` командасымен жобаны бастайық.

### TCP қосылымды тыңдау

`std::net::TcpListener` арқылы порт ашамыз:

```rust
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("Қосылым орнатылды!");
    }
}
```

`bind` — порт пен мекенжайды ашады. `127.0.0.1:7878` — жергілікті
машина, 7878 порт. `7878` — себебі ASCII-да «rust» сөзінің сандары
(R=18, U=21, S=19, T=20 — ауысты).

`listener.incoming()` итератор қайтарады; әр итерация — жаңа қосылым.
`stream` — `TcpStream` түрінде. Бірақ ол `Result` ішінде, өйткені
қосылым кей кезде сәтсіз болуы мүмкін.

`cargo run` шакырғанда — бағдарлама порт ашады және кезекте тұрады.
Браузерден `http://127.0.0.1:7878` кіріп көрсеңіз — терминалда
«Қосылым орнатылды!» деген жол шығады, бірақ браузерде ешбір нәрсе
көрінбейді — біз әлі жауап жібермейміз.

### Сұранысты оқу

HTTP сұранысы — мәтін. Әр жол `\r\n` арқылы аяқталады. Мысал:

```text
GET / HTTP/1.1
Host: 127.0.0.1:7878
User-Agent: Mozilla/5.0 ...
Accept: text/html,application/xhtml+xml,...

```

Бірінші жол — **сұраныс-жол** (request line): метод, жол, HTTP
нұсқасы. Қалғаны — **тақырыптар**. Бос жол сұраныстың аяқталғанын
білдіреді.

Біз сұранысты оқып, экранда көрсетейік:

```rust
use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Сұраныс: {:#?}", http_request);
}
```

`BufReader::new(&mut stream)` — TcpStream-ді буферлеп оқуға кесеміз.
`.lines()` — итератор, `Result<String>` қайтарады. `.map(|r| r.unwrap())`
қателерді `unwrap`-пен ашамыз. `.take_while(|line| !line.is_empty())`
— бос жол кездескенше алады.

Шығатыны:

```text
Сұраныс: [
    "GET / HTTP/1.1",
    "Host: 127.0.0.1:7878",
    "User-Agent: Mozilla/5.0 ...",
    ...
]
```

Енді біз сұранысты «оқыдық». Бірақ браузер әлі күтіп тұр — біз
жауап жіберуіміз керек.

### Жауап жазу

HTTP жауабы — солай мәтін. Формат:

```text
HTTP/1.1 200 OK\r\n
Content-Length: 13\r\n
\r\n
Hello, world!
```

Бірінші жол — **жағдай-жол**: HTTP-нұсқа, мәртебе кодт (200 — OK),
түсіндірме. Тақырыптар. Бос жол. Содан кейін — мазмұн.

Кодты қосайық:

```rust
use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|r| r.unwrap())
        .take_while(|l| !l.is_empty())
        .collect();

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).unwrap();
}
```

Жауапты `stream.write_all` арқылы жібереміз. `as_bytes()` — жолды
байттарға айналдырады (TCP байттармен жұмыс істейді).

Жауапта мазмұн жоқ — бос. Браузерде ақ парақ көрінеді.

### Шынайы HTML қайтару

Жобаға `hello.html` файлын қосайық:

```html
<!DOCTYPE html>
<html lang="kk">
<head>
    <meta charset="UTF-8">
    <title>Сәлемдесу!</title>
</head>
<body>
    <h1>Сәлем!</h1>
    <p>`Rust`-тағы менің бірінші HTTP-серверім.</p>
</body>
</html>
```

Кодта файлды оқып, жауапқа қосамыз:

```rust
use std::fs;

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|r| r.unwrap())
        .take_while(|l| !l.is_empty())
        .collect();

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("hello.html").unwrap();
    let length = contents.len();

    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
    );

    stream.write_all(response.as_bytes()).unwrap();
}
```

`Content-Length` тақырыбы — браузерге мазмұнның мөлшерін айтады.
Браузер сонша байт оқиды.

`cargo run` шакырғанда және браузерден кіргенде — HTML беті көрінеді.

### Сұранысты тексеріп, түрлі жауап беру

Әзірге біз барлық сұранысқа бір жауап беріп жатырмыз. Бірақ нақты
сервер: `/` жолына — басты бет; қалған жолдарға — 404. Сұраныс-жолды
тексерейік:

```rust
fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filename) = if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();
    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
    );

    stream.write_all(response.as_bytes()).unwrap();
}
```

`404.html` файлын да дайындаймыз:

```html
<!DOCTYPE html>
<html lang="kk">
<head><meta charset="UTF-8"><title>404</title></head>
<body>
    <h1>Айып!</h1>
    <p>Іздегеніңіз табылмады.</p>
</body>
</html>
```

`cargo run` — қазір `/` бойынша `hello.html`, басқа жолдарда
`404.html` қайтарылады.

## 20.2. Бір ағынды серверді көп ағындыға айналдыру

Біздің сервер бір ағында жұмыс істеп жатыр. Қандай мәселе? Әр сұраныс
кезек күтуі керек. Егер бір сұраныс баяу — қалғандары да баяу.
Демонстрация үшін «`/sleep`» жолын қосайық: ол 5 секунд күтеді,
содан кейін жауап беретін болады.

```rust
use std::thread;
use std::time::Duration;

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();
    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
    );

    stream.write_all(response.as_bytes()).unwrap();
}
```

Енді тестілеу: екі браузер-табын ашыңыз. Біріншісінде `/sleep`-ке
кіріңіз — ол 5 секунд күтеді. Екіншісінде сол кезде `/`-ке кіріңіз —
ол да 5 секунд күтеді! Себебі сервер бір сұранысты әлі бітірмей,
екіншісін бастамайды. Кез келген клиент бір баяу сұраныстың кесірінен
зардап шегеді.

Шешім — әр сұранысты бөлек ағында өңдеу. Бірақ әр сұранысқа жаңа
ағын ашу — алуан түрлі шектелген проблемаға келеді: жүйе мың
ағын құра алмауы мүмкін, мындаған ағын — ресурс үнемдеу емес.

Дұрыс шешім — **ағын-жинағы** (thread pool). Алдын ала тіркелген
шектелген ағындар жинағы. Әр сұранысты бар бір ағынға тапсырып,
ол өңдейді. Барлық ағын банс болса — сұраныстар кезекте күтеді,
бірақ серверге зиян келмейді.

### `ThreadPool` құрылымын жоспарлау

Біздің мақсат — `ThreadPool` атты типке мынадай интерфейс жасау:

```rust
let pool = ThreadPool::new(4); // 4 жұмысшы ағын
pool.execute(|| { /* код */ });
```

Шакырғанда — `execute` ішіндегі жабу банс ағынға тапсырылады.

Бұл жасырын мысал — Rust қауымдастығы оны «компилятормен бағытталған
әзірлеу» (compiler-driven development) деп атайды: біз мысалды жазамыз,
компилятор қателер арқылы бізге қажетін айтады.

`main`-ді қазірден бастап жаңарта береміз:

```rust
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }
}
```

Енді `ThreadPool`-ды тиісінше жасайық.

### `ThreadPool` структы

`src/lib.rs` файлында (жаңа кітапхана сандығы):

```rust
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}
```

`ThreadPool` — жұмысшыларды (`Worker`) және `mpsc::Sender`-ді ұстайды.
`Job` — кез келген орындалатын жабу. `Worker` — id және ағынды
ұстайды.

`new` функциясы — `n` жұмысшыны жасайды. Әр жұмысшы — бір ағын. Әр
ағын каналды тыңдайды.

```rust
impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}
```

`mpsc::channel` — біздің тапсырмалар каналы. **Бір** жіберуші
(`sender`); **көп** қабылдаушы (барлық жұмысшылар бірге қабылдайды).
`mpsc` mpsc — multiple producer, single consumer — бірақ біз керісінше
қажет етеміз. Мысал арқылы шешім: receiver-ды `Arc<Mutex<...>>`
ішінде қойсақ — бар жұмысшылар оған бөлек қол жеткізе алады, бірақ
бір сәтте бір ғана `recv`-ке қол жеткізеді.

```rust
impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Жұмысшы {id} жұмысты алды; орындап жатыр.");
                    job();
                }
                Err(_) => {
                    println!("Жұмысшы {id} өшіп жатыр.");
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
```

Әр жұмысшы — өзінің ағынында. Циклда `recv` шакырады — каналдан жұмыс
келгенше блок жасайды. Жұмыс келгенде орындайды; қайтадан тыңдайды.
Канал жабылғанда (`Err`) — цикл тоқтайды.

`receiver.lock().unwrap().recv()` — мутекс алып, мәнді алу. Lock
бөлектен босайды әр итерация соңында (мутекс-күзетшінің `Drop`-ы
арқылы).

Қазір бізде жұмыс жасайтын ағын-жинағы бар. `cargo run` шакырғанда:

```sh
$ cargo run
$ # браузерден /sleep кіру — 5 секунд күтеді
$ # параллельді /-ге кіру — мезетте жауап
```

Тестке енді бірнеше ағын. `ThreadPool` ішінде 4 ағын — бір уақытта
4 баяу сұранысты ұстай алады. Бесінші ғана кезекте күтеді.

## 20.3. Жайлап тоқтау мен тазалау

Сервер `Ctrl-C` арқылы тоқтатылса — ағындар амалсыз тоқтатылады.
Бар жұмыс жоғалуы мүмкін. Сондай-ақ ресурстарды дұрыс тазалаудың
кепілдігі жоқ.

Шешім — `Drop` трейтін `ThreadPool`-да бекіту. `Drop` ішінде:

1. Каналды жабу (sender-ды `drop` ету).
2. Бар жұмысшы ағындарының аяқталуын күту (`join`).

```rust
impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Жұмысшы {}-нің аяқталуын күтемін.", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
```

`self.sender.take()` — `Option`-нан мәнді алу, `Option`-да `None`
қалдырады. Соның `drop` шакырылғанда — канал жабылады. Барлық
жұмысшылар `recv` `Err` алады — олар циклден шығады.

`worker.thread.take()` — солай. `JoinHandle`-ды алу, нысан
бос қалдыру. `thread.join()` — ағынның аяқталуын күтеді.

Соларды қалай тестілеу? `main`-да тек алғашқы 2 сұранысты қабылдау
шектеуін қойсақ — `for stream in listener.incoming().take(2)`. Содан
кейін бағдарлама `main`-нен шығады, `pool` `Drop`-қа барады, сервер
жайлап тоқтайды.

```rust
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming().take(2) {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Тоқтап жатырмын.");
}
```

Шакырылғанда: 2 сұраныс өңделеді, содан кейін бағдарлама шығу
процессін бастайды. Шығатыны:

```text
Тоқтап жатырмын.
Жұмысшы 0-нің аяқталуын күтемін.
Жұмысшы 0 өшіп жатыр.
Жұмысшы 1-нің аяқталуын күтемін.
Жұмысшы 1 өшіп жатыр.
Жұмысшы 2-нің аяқталуын күтемін.
Жұмысшы 2 өшіп жатыр.
Жұмысшы 3-нің аяқталуын күтемін.
Жұмысшы 3 өшіп жатыр.
```

Барлығы дұрыс — әр жұмысшы дұрыс тазаланды, ешбір жұмыс жоғалмады.

## 20.4. Финал қорытындысы

Сіз `Rust`-тың Book-нің соңғы тарауын тіндеп шықтыңыз. Бір жобада
бар біз үйренгенімізді қолдандық:

- **TCP программалау** — `TcpListener`, `TcpStream` (16-тарауда айтылған
  ағындармен бірге).
- **Қателерді өңдеу** — `unwrap`, `Result` (9-тарау).
- **Файл оқу** — `fs::read_to_string` (12-тараудан minigrep).
- **Жабулар мен `Box<dyn FnOnce>`** — `execute(F: FnOnce + Send +
  'static)` (13 + 19-тараулар).
- **Каналдар** — `mpsc::channel` (16-тарау).
- **`Mutex<T>` пен `Arc<T>`** — receiver-ды бөлу (15 + 16-тараулар).
- **`Drop` трейті** — жайлап тоқтату (15-тарау).
- **`Option::take`** — `Option`-да жасырылған мәнді шығарып алу (6 +
  15-тараулар).

Бұл — **сіздің** жасаған `Rust` бағдарламаңыз. Шынайы жұмыс жасайды,
шынайы инвариантты ұстайды, шынайы жүйелік жоба түрде жүреді.
Кеңеюі: tokio немесе async-std негізінде асинхронды нұсқасына
ауыстыру; HTTP-ді hyper кітапханасына; маршрутизация мен middleware
қосу — Rocket немесе Actix-web фреймворктерін зерттеу.

`Rust`-та сізге толық экожүйе ашылды. Жүйе бағдарламалау, веб,
ендірме жүйелер, ойындар, машиналық оқыту — әрбір саланың Rust-та
қызықты кітапханалары мен қауымдастығы бар.

`Rust` Book — бар Rust оқу жолының бір бөлігі. Кейінгі қадамдар:

- **Rust by Example** — мысалдармен қысқа сабақтар.
- **Rustlings** — кішкене жаттығулар.
- **The Rustonomicon** — `unsafe` Rust-тың тереңірек қарастырылуы.
- **Rust Reference** — тілдің ресми сипаттамасы.

Қауымдастық: [users.rust-lang.org](https://users.rust-lang.org),
[r/rust](https://reddit.com/r/rust), [rust-lang/rust](https://github.com/rust-lang/rust)
GitHub-та.

`Rust`-та ойдағыдай жобалар жасап шығарыңыз!

---

## Қазақ тіліндегі аударманың бір қорытындысы

Бұл — `Rust` Book-тің 20-тарауының соңы. Қазақ тіліне аударылған
толық кітап — Тарау 1-ден 20-ға дейін, ~80 000 сөз — енді сіздің
қолыңызда. Бұл — `Rust`-ты қазақ тілінде үйренудің бастапқы
ресурсы. Терминология `data/world_core/programming_rust.jsonl`
файлында тіркелген; пайдалы анықтамалар.

Аударма — ашық, MIT/Apache-2.0 лицензиясы бойынша. Кез келген
оқушы, мұғалім, жобалаушы — оны қолдана алады, өзгерте алады, өз
жобасына қоса алады.

Қазақ тілінде сапалы технологиялық оқу-материалдарының жетіспеуі —
бар проблеманың бір бөлігі. Бұл аударма сол жетіспеушілікті, ең
болмаса, бір бөлікте — Rust бағдарламалау тілін үйрену үшін —
жабатын шамамыз.
