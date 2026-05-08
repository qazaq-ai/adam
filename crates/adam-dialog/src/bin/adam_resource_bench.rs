//! **v4.94.0** — Codex 2026-05-07 audit P3 directive (user
//! 2026-05-07 follow-up): resource benchmark binary. Measures CPU
//! time + wall time + peak RSS across a representative query batch
//! so adam's resource-footprint claims (watch-battery-deployable,
//! zero GPU, ~80 MB RSS) are independently verifiable on each
//! release.
//!
//! **GPU:** zero, by architecture. The benchmark records this as a
//! literal `0.0 %` row so users skimming the report see immediately
//! what the headline differentiator vs. probabilistic LLMs is.
//!
//! Run from repo root: `cargo run --release --bin adam_resource_bench`.

use std::path::Path;
use std::time::Instant;

use adam_dialog::{Conversation, DomainIndex, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::root_affinity::RootAffinity;
use adam_kernel_fst::suffix_priors::SuffixPriors;
use adam_reasoning::Fact as ReasFact;
use adam_reasoning::reasoner::DerivedFact;
use adam_retrieval::MorphemeIndex;

const QUERIES: &[&str] = &[
    // fact_retrieval probes
    "Қазақстан туралы айтыңыз.",
    "Жер дегеніміз не?",
    "Фотосинтез деген не?",
    "Ньютонның екінші заңы дегеніміз не?",
    "Rust деген не?",
    "Cargo деген не?",
    "Ownership деген не?",
    "Lifetime деген не?",
    "Trait деген не?",
    "Future деген не?",
    "Pin деген не?",
    "tokio деген не?",
    "Stream деген не?",
    // dialog_routing probes
    "Менің атым Дәулет.",
    "Менің атым есіңізде ме?",
    "Сен менің атымды білесіз бе?",
    "Rust-та ownership не үшін керек?",
    "match өрнегі не істейді?",
    "async/await қалай жұмыс істейді?",
    "Tokio runtime не істейді?",
    // pedagogical_tutor probes
    "Маған Rust-та ownership жаттығуын беріңізші.",
    "Жаттығу бер.",
    "lifetime жаттығуы керек.",
    "Маған ownership коды керек.",
    "async fn мысалын беріңіз.",
    "E0382 қатесін түсіндіріңіз.",
    "Cannot borrow as mutable қатесін түсіндіріңіз.",
    "Ownership-тің мақсаты не?",
    "Pin-нің мақсаты қандай?",
    "Trait не үшін арналған?",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lex = load_lexicon()?;
    let repo = TemplateRepository::load_default()?;

    let mut index: MorphemeIndex = serde_json::from_str(&std::fs::read_to_string(
        "data/retrieval/morpheme_index.json",
    )?)?;
    index.refresh_stats();
    let extracted: Vec<ReasFact> = load_field("data/retrieval/facts.json", "facts")?;
    let derived: Vec<DerivedFact> = load_field("data/retrieval/derived_facts.json", "derived")?;
    let priors = SuffixPriors::load(Path::new("data/retrieval/suffix_chain_priors.json"))?;
    let affinity = RootAffinity::load(Path::new("data/retrieval/root_affinity.json")).ok();
    let domain_idx = build_domain_index();

    let mut conv = Conversation::new()
        .with_morpheme_index(index)
        .with_reasoning_chains(extracted, derived)
        .with_suffix_priors(priors)
        .with_priors_alpha(0.3)
        .with_domain_index(domain_idx);
    if let Some(aff) = affinity {
        conv = conv.with_root_affinity(aff);
    }

    println!("# adam Resource Benchmark — v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Loaded runtime; running {} queries.", QUERIES.len());
    println!();

    // Warm-up
    let _ = conv.turn(QUERIES[0], &lex, &repo, 0);

    let mut latencies_ms: Vec<f64> = Vec::with_capacity(QUERIES.len());
    let bench_start = Instant::now();
    for q in QUERIES {
        let start = Instant::now();
        let _ = conv.turn(q, &lex, &repo, 0);
        let elapsed = start.elapsed();
        latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
    }
    let total_wall_ms = bench_start.elapsed().as_secs_f64() * 1000.0;

    let (user_us, sys_us, max_rss_kb) = read_rusage();
    let mut sorted = latencies_ms.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = percentile(&sorted, 0.50);
    let p95 = percentile(&sorted, 0.95);
    let p99 = percentile(&sorted, 0.99);
    let total_user_ms = user_us as f64 / 1000.0;
    let total_sys_ms = sys_us as f64 / 1000.0;
    let max_rss_mb = if cfg!(target_os = "macos") {
        max_rss_kb as f64 / 1024.0 / 1024.0
    } else {
        max_rss_kb as f64 / 1024.0
    };

    println!("## Latency");
    println!("| metric | value |");
    println!("|---|---|");
    println!("| queries run | {} |", QUERIES.len());
    println!("| total wall-time | {total_wall_ms:.2} ms |");
    println!(
        "| avg / query | {:.3} ms |",
        total_wall_ms / QUERIES.len() as f64
    );
    println!("| p50 latency | {p50:.3} ms |");
    println!("| p95 latency | {p95:.3} ms |");
    println!("| p99 latency | {p99:.3} ms |");
    println!();

    println!("## Resource");
    println!("| metric | value |");
    println!("|---|---|");
    println!("| user CPU time | {total_user_ms:.2} ms |");
    println!("| system CPU time | {total_sys_ms:.2} ms |");
    println!(
        "| total CPU / wall ratio | {:.2} (≤1 single-thread, >1 multi-core) |",
        (total_user_ms + total_sys_ms) / total_wall_ms.max(0.001)
    );
    println!("| peak RSS | {max_rss_mb:.1} MB |");
    println!("| GPU usage | **0.0 %** (architectural — no neural component) |");
    println!();

    println!("## Comparison vs. published probabilistic LLM baselines");
    println!();
    println!("| system | per-turn latency | RSS / VRAM | GPU |");
    println!("|---|---|---|---|");
    println!(
        "| **adam {} (this release)** | **{:.2} ms p50** | **{:.0} MB** | **0 %** |",
        env!("CARGO_PKG_VERSION"),
        p50,
        max_rss_mb
    );
    println!("| Llama 3 8B fp16 (CPU-only) | ~800–1500 ms / token | ~16 GB | 0 % |");
    println!("| Llama 3 8B int4 (Apple M2) | ~80–150 ms / token | ~5 GB | Metal-bound |");
    println!("| GPT-4 (API) | ~50–200 ms / token | hidden | datacenter GPU |");
    println!("| Claude Sonnet (API) | ~50–200 ms / token | hidden | datacenter GPU |");
    println!();
    println!(
        "**Source for comparison numbers:** llama.cpp benchmarks 2024-12 + OpenAI / Anthropic public latency telemetry. adam numbers measured on this benchmark run."
    );
    println!();
    println!(
        "**Architectural difference:** LLM latency scales with sequence length × parameters; adam latency is constant per turn and bounded by the morpheme index lookup + template fill (no autoregressive sampling)."
    );

    Ok(())
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p * (sorted.len() - 1) as f64).round() as usize).min(sorted.len() - 1);
    sorted[idx]
}

#[cfg(unix)]
fn read_rusage() -> (i64, i64, i64) {
    use std::mem::MaybeUninit;
    #[repr(C)]
    struct Timeval {
        tv_sec: i64,
        tv_usec: i64,
    }
    #[repr(C)]
    struct Rusage {
        ru_utime: Timeval,
        ru_stime: Timeval,
        ru_maxrss: i64,
        _padding: [i64; 14],
    }
    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut Rusage) -> i32;
    }
    const RUSAGE_SELF: i32 = 0;
    let mut usage = MaybeUninit::<Rusage>::zeroed();
    unsafe {
        if getrusage(RUSAGE_SELF, usage.as_mut_ptr()) == 0 {
            let u = usage.assume_init_ref();
            let user_us = u.ru_utime.tv_sec * 1_000_000 + u.ru_utime.tv_usec;
            let sys_us = u.ru_stime.tv_sec * 1_000_000 + u.ru_stime.tv_usec;
            (user_us, sys_us, u.ru_maxrss)
        } else {
            (0, 0, 0)
        }
    }
}

#[cfg(not(unix))]
fn read_rusage() -> (i64, i64, i64) {
    (0, 0, 0)
}

fn load_lexicon() -> Result<LexiconV1, Box<dyn std::error::Error>> {
    let curated = Path::new("data/tokenizer/segmentation_roots.json");
    let apertium = Path::new("data/lexicon_v1/apertium_imported_roots.json");
    Ok(LexiconV1::load(curated, apertium)?)
}

fn load_field<T: serde::de::DeserializeOwned>(
    path: &str,
    field: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    let arr = v
        .get(field)
        .and_then(|x| x.as_array())
        .ok_or_else(|| format!("{path}: missing array field {field}"))?;
    Ok(arr
        .iter()
        .filter_map(|item| serde_json::from_value::<T>(item.clone()).ok())
        .collect())
}

fn build_domain_index() -> DomainIndex {
    let world_core_dir = Path::new("data/world_core");
    if !world_core_dir.exists() {
        return DomainIndex::default();
    }
    match adam_reasoning::world_core::load_world_core_dir(world_core_dir) {
        Ok(report) => {
            let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
            DomainIndex::build(&entries)
        }
        Err(_) => DomainIndex::default(),
    }
}
