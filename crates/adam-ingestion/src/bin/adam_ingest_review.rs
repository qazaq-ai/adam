// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! TTY review wrapper for the ingestion queue.
//!
//! Thin shim around [`adam_ingestion::review::run_review_session`]:
//! reads candidates from a [`CandidateStore`] under
//! `--root <DIR>`, prints each `NeedsReview` fact to stdout
//! via [`render_fact_for_review`], and parses single-letter
//! decisions from stdin:
//!
//!   * `a` / `A` / `approve`  → ApprovedByHuman
//!   * `r` / `R` / `reject`   → RejectedByHuman
//!   * `s` / `S` / `skip`     → leave NeedsReview
//!   * `q` / `Q` / `quit`     → end session
//!   * empty input            → treated as Skip
//!   * EOF (Ctrl-D)           → treated as Quit
//!   * anything else          → reprompt
//!
//! Prints a one-line summary on exit.  Returns non-zero
//! when the store path is missing or unreadable; returns
//! zero on a clean session (including Quit).

use std::io::{BufRead, Write};

use adam_ingestion::candidate::CandidateFact;
use adam_ingestion::review::{
    ReviewDecision, Reviewer, render_fact_for_review, run_review_session,
};
use adam_ingestion::store::CandidateStore;

/// CLI argument shape — kept minimal on purpose.  `--root`
/// is required; future flags (filter-by-domain, filter-by-
/// confidence-range, batch-approve) land here as they're
/// needed.
struct Args {
    root: String,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let mut root: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                root = args.next();
            }
            "-h" | "--help" => {
                return Err("usage: adam_ingest_review --root <DIR>".into());
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
    }
    let root = root.ok_or_else(|| "missing required --root <DIR>".to_string())?;
    Ok(Args { root })
}

/// Stdin-backed reviewer.  Reprompts on invalid input;
/// treats EOF as Quit so a piped script with fewer
/// decisions than candidates terminates cleanly.
struct StdinReviewer<R: BufRead, W: Write> {
    input: R,
    output: W,
}

impl<R: BufRead, W: Write> Reviewer for StdinReviewer<R, W> {
    fn review_fact(&mut self, fact: &CandidateFact) -> ReviewDecision {
        loop {
            let _ = writeln!(self.output);
            let _ = self
                .output
                .write_all(render_fact_for_review(fact).as_bytes());
            let _ = self
                .output
                .write_all(b"[a]pprove / [r]eject / [s]kip / [q]uit > ");
            let _ = self.output.flush();
            let mut buf = String::new();
            match self.input.read_line(&mut buf) {
                Ok(0) => return ReviewDecision::Quit,
                Ok(_) => {}
                Err(_) => return ReviewDecision::Quit,
            }
            let trimmed = buf.trim().to_ascii_lowercase();
            match trimmed.as_str() {
                "" | "s" | "skip" => return ReviewDecision::Skip,
                "a" | "approve" => return ReviewDecision::Approve,
                "r" | "reject" => return ReviewDecision::Reject,
                "q" | "quit" | "exit" => return ReviewDecision::Quit,
                _ => {
                    let _ = writeln!(
                        self.output,
                        "  ↳ unrecognised input: `{trimmed}` — please answer a / r / s / q"
                    );
                    continue;
                }
            }
        }
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    let store = match CandidateStore::open(&args.root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to open candidate store at `{}`: {e}", args.root);
            std::process::exit(1);
        }
    };
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reviewer = StdinReviewer {
        input: stdin.lock(),
        output: stdout.lock(),
    };
    let summary = match run_review_session(&store, &mut reviewer) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("review session aborted: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "\nreview session done: examined {} | approved {} | rejected {} | skipped {} | quit {}",
        summary.examined, summary.approved, summary.rejected, summary.skipped, summary.quit
    );
}
