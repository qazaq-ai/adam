//! Iteration harness for long-running extract / reasoning binaries.
//!
//! v3.1.0 introduced the 3h-budget iteration discipline: every binary
//! that walks corpus data must be interruptible, budget-bounded, and
//! commit a useful artifact even when it stops short.
//!
//! ## Invariants enforced here
//!
//! 1. **Time budget** — caller sets `--time-budget <SEC>`; the harness
//!    hands back a deadline `Instant`. Work loops call
//!    [`IterationBudget::should_stop`] between chunks and fall through
//!    to commit on `true`.
//! 2. **Progress ticks** — `--progress-interval <SEC>` (default 30s).
//!    A monitor thread reads [`ProgressCounter`] atomics and prints a
//!    structured `[mm:ss] …` line to stderr at most every interval.
//!    No per-sample prints.
//! 3. **Graceful signal handling** — SIGINT (Ctrl-C) and SIGTERM flip
//!    [`IterationBudget::interrupted`]. The caller is responsible for
//!    committing the partial artifact before exit.
//!
//! ## Partial-artifact contract
//!
//! When a binary stops on `StopReason::TimeBudget` or `::Interrupted`,
//! the artifact it writes **must** include a `status` field set to
//! `"timed_out"` or `"interrupted"`, plus `elapsed_s` and whatever
//! completion progress it can report (`packs_completed/packs_total`,
//! `samples_scanned`, etc.). Downstream bins must treat these as
//! first-class input — a partial `facts.json` is still a valid
//! `facts.json`, just smaller.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Why a work loop stopped. Callers use this to pick the `status` value
/// they write into the output artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// All work completed inside the budget, no signal.
    Completed,
    /// `--time-budget` elapsed mid-run.
    TimeBudget,
    /// SIGINT / SIGTERM fired mid-run.
    Interrupted,
}

impl StopReason {
    /// Stable JSON form. Downstream bins match on these strings.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::TimeBudget => "timed_out",
            Self::Interrupted => "interrupted",
        }
    }
}

/// Lightweight deadline + interrupt flag. Cheap to clone (all state is
/// behind `Arc`). Clone it into rayon closures and check
/// [`IterationBudget::should_stop`] between work chunks.
#[derive(Clone)]
pub struct IterationBudget {
    started: Instant,
    deadline: Option<Instant>,
    interrupted: Arc<AtomicBool>,
}

impl IterationBudget {
    /// Build from process argv. Parses:
    ///   `--time-budget <SEC>`       — absolute cap, default none.
    ///   `--time-budget-mins <MIN>`  — convenience alias.
    /// Unrecognised values parse to `None` and log a warning on stderr.
    pub fn from_args(args: &[String]) -> Self {
        let seconds = parse_flag_u64(args, "--time-budget").or_else(|| {
            parse_flag_u64(args, "--time-budget-mins").map(|m| m.saturating_mul(60))
        });
        let started = Instant::now();
        let deadline = seconds.map(|s| started + Duration::from_secs(s));
        Self {
            started,
            deadline,
            interrupted: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Build with no budget and no installed signal handler — for unit
    /// tests that don't want to touch process-global state.
    pub fn unbounded_for_tests() -> Self {
        Self {
            started: Instant::now(),
            deadline: None,
            interrupted: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Install a process-global Ctrl-C / SIGTERM handler that flips the
    /// interrupted flag on this budget. Safe to call once per process.
    /// Subsequent calls are no-ops (ctrlc returns an error we swallow
    /// for idempotence — tests may install + reinstall during runs).
    pub fn install_signal_handler(&self) {
        let flag = Arc::clone(&self.interrupted);
        let _ = ctrlc::set_handler(move || {
            flag.store(true, Ordering::Relaxed);
        });
    }

    /// Non-blocking check. Call between work chunks (every few seconds
    /// of work, not every sample — this is intentionally cheap but not
    /// free).
    pub fn should_stop(&self) -> bool {
        if self.interrupted.load(Ordering::Relaxed) {
            return true;
        }
        if let Some(d) = self.deadline {
            if Instant::now() >= d {
                return true;
            }
        }
        false
    }

    /// Why the loop should stop (or `Completed` if it shouldn't). Call
    /// exactly once at the end of the work loop to pick `status` for
    /// the artifact.
    pub fn stop_reason(&self) -> StopReason {
        if self.interrupted.load(Ordering::Relaxed) {
            StopReason::Interrupted
        } else if self.deadline.is_some_and(|d| Instant::now() >= d) {
            StopReason::TimeBudget
        } else {
            StopReason::Completed
        }
    }

    /// Wall-clock elapsed since the budget was constructed. Safe to
    /// call any number of times.
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Full seconds elapsed, rounded toward zero. Convenience for the
    /// `elapsed_s` field in partial artifacts.
    pub fn elapsed_secs(&self) -> u64 {
        self.elapsed().as_secs()
    }

    /// Seconds remaining in the budget, or `None` if unbounded.
    /// Saturates at zero once the deadline has passed.
    pub fn remaining_secs(&self) -> Option<u64> {
        let d = self.deadline?;
        let now = Instant::now();
        if now >= d {
            Some(0)
        } else {
            Some((d - now).as_secs())
        }
    }

    /// The raw interrupted flag — rayon closures may want to hold a
    /// clone and check it without going through `should_stop` (avoiding
    /// the deadline load).
    pub fn interrupted_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.interrupted)
    }
}

/// Atomic counters shared between worker threads and the progress
/// monitor. The monitor reads; workers `fetch_add`. All fields are
/// public so binaries can add their own without a builder.
#[derive(Default)]
pub struct ProgressCounter {
    pub samples_scanned: AtomicUsize,
    pub items_produced: AtomicUsize,
    /// Free-form extra counter — binary decides what this means.
    /// `extract_facts` uses it for "words parsed".
    pub extra: AtomicU64,
}

impl ProgressCounter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn inc_sample(&self) {
        self.samples_scanned.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_items(&self, n: usize) {
        self.items_produced.fetch_add(n, Ordering::Relaxed);
    }

    pub fn add_extra(&self, n: u64) {
        self.extra.fetch_add(n, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (usize, usize, u64) {
        (
            self.samples_scanned.load(Ordering::Relaxed),
            self.items_produced.load(Ordering::Relaxed),
            self.extra.load(Ordering::Relaxed),
        )
    }
}

/// Monitor-thread handle. Drops do NOT automatically stop the monitor —
/// call [`ProgressMonitor::join`] from the caller after the work loop
/// exits.
pub struct ProgressMonitor {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ProgressMonitor {
    /// Spawn a monitor thread that prints `[mm:ss] samples=N items=M
    /// extra=E elapsed=S rem=R` to stderr every `interval` seconds until
    /// [`ProgressMonitor::join`] is called. `label` is prepended so
    /// concurrent bins don't scramble lines.
    pub fn spawn(
        budget: IterationBudget,
        counters: Arc<ProgressCounter>,
        interval: Duration,
        label: &'static str,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let tick = Duration::from_millis(250);
            let mut next_print = Instant::now() + interval;
            while !stop_t.load(Ordering::Relaxed) {
                thread::sleep(tick);
                if Instant::now() < next_print {
                    continue;
                }
                next_print = Instant::now() + interval;
                let (samples, items, extra) = counters.snapshot();
                let elapsed = budget.elapsed_secs();
                let rem = budget
                    .remaining_secs()
                    .map(|s| format!(" rem={s}s"))
                    .unwrap_or_default();
                let interrupted = budget
                    .interrupted
                    .load(Ordering::Relaxed)
                    .then_some(" INT")
                    .unwrap_or("");
                eprintln!(
                    "[{}] {label} samples={samples} items={items} extra={extra} elapsed={elapsed}s{rem}{interrupted}",
                    hhmmss(elapsed)
                );
            }
            // Final tick — flush the last counters so the transition
            // from "running" to "done" is always visible.
            let (samples, items, extra) = counters.snapshot();
            let elapsed = budget.elapsed_secs();
            eprintln!(
                "[{}] {label} DONE samples={samples} items={items} extra={extra} elapsed={elapsed}s",
                hhmmss(elapsed)
            );
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }

    /// Stop the monitor and wait for its final tick to flush. Call once
    /// after the work loop returns.
    pub fn join(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
    }
}

fn parse_flag_u64(args: &[String], name: &str) -> Option<u64> {
    let idx = args.iter().position(|a| a == name)?;
    let raw = args.get(idx + 1)?;
    match raw.parse::<u64>() {
        Ok(n) => Some(n),
        Err(_) => {
            eprintln!(
                "harness: warning — {name} value `{raw}` is not a non-negative integer; ignoring"
            );
            None
        }
    }
}

fn hhmmss(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_unbounded_never_stops() {
        let b = IterationBudget::unbounded_for_tests();
        assert!(!b.should_stop());
        assert_eq!(b.stop_reason(), StopReason::Completed);
        assert_eq!(b.remaining_secs(), None);
    }

    #[test]
    fn budget_from_args_parses_seconds() {
        let args: Vec<String> = vec!["prog".into(), "--time-budget".into(), "42".into()];
        let b = IterationBudget::from_args(&args);
        assert_eq!(b.remaining_secs().unwrap() <= 42, true);
        assert_eq!(b.stop_reason(), StopReason::Completed);
    }

    #[test]
    fn budget_from_args_mins_alias() {
        let args: Vec<String> = vec!["prog".into(), "--time-budget-mins".into(), "2".into()];
        let b = IterationBudget::from_args(&args);
        assert!(b.remaining_secs().unwrap() <= 120);
        assert!(b.remaining_secs().unwrap() > 115);
    }

    #[test]
    fn interrupted_flag_triggers_stop() {
        let b = IterationBudget::unbounded_for_tests();
        assert!(!b.should_stop());
        b.interrupted.store(true, Ordering::Relaxed);
        assert!(b.should_stop());
        assert_eq!(b.stop_reason(), StopReason::Interrupted);
    }

    #[test]
    fn deadline_in_the_past_triggers_stop() {
        let past = Instant::now() - Duration::from_secs(1);
        let b = IterationBudget {
            started: past,
            deadline: Some(past),
            interrupted: Arc::new(AtomicBool::new(false)),
        };
        assert!(b.should_stop());
        assert_eq!(b.stop_reason(), StopReason::TimeBudget);
    }

    #[test]
    fn stop_reason_prioritises_interrupt_over_budget() {
        // Both conditions true — interrupted wins (user intent beats
        // auto-timeout). Useful signal for post-mortem diagnostics.
        let past = Instant::now() - Duration::from_secs(1);
        let b = IterationBudget {
            started: past,
            deadline: Some(past),
            interrupted: Arc::new(AtomicBool::new(true)),
        };
        assert_eq!(b.stop_reason(), StopReason::Interrupted);
    }

    #[test]
    fn progress_counter_increments_across_threads() {
        let c = ProgressCounter::new();
        let mut handles = Vec::new();
        for _ in 0..4 {
            let c = Arc::clone(&c);
            handles.push(std::thread::spawn(move || {
                for _ in 0..1000 {
                    c.inc_sample();
                    c.add_items(2);
                    c.add_extra(3);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let (s, i, e) = c.snapshot();
        assert_eq!(s, 4000);
        assert_eq!(i, 8000);
        assert_eq!(e, 12_000);
    }

    #[test]
    fn stop_reason_as_str_stable() {
        assert_eq!(StopReason::Completed.as_str(), "completed");
        assert_eq!(StopReason::TimeBudget.as_str(), "timed_out");
        assert_eq!(StopReason::Interrupted.as_str(), "interrupted");
    }

    #[test]
    fn parse_flag_u64_tolerates_bad_value() {
        let args: Vec<String> = vec!["prog".into(), "--time-budget".into(), "abc".into()];
        let b = IterationBudget::from_args(&args);
        assert_eq!(b.remaining_secs(), None);
    }

    #[test]
    fn progress_monitor_prints_final_tick_on_join() {
        let budget = IterationBudget::unbounded_for_tests();
        let counters = ProgressCounter::new();
        let mon = ProgressMonitor::spawn(
            budget.clone(),
            Arc::clone(&counters),
            Duration::from_secs(60),
            "test",
        );
        counters.add_items(7);
        mon.join();
        // If the thread's final tick is lost, join would hang rather
        // than failing visibly — so the test primarily verifies that
        // join returns within a reasonable time without a deadlock.
    }
}
