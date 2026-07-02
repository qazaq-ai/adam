// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam_briefing` — interactive REPL for the v6.9.2 safety-briefing
//! session engine.  Runs one OT/TB procedure end-to-end:
//! инструктаж → устный опрос → оценка → протокол.
//!
//! ## Usage
//!
//! ```sh
//! # List available procedure ids:
//! cargo run -p adam-dialog --bin adam_briefing -- --list
//!
//! # Run a session for one procedure:
//! cargo run -p adam-dialog --bin adam_briefing -- kk_metallurgy_loto_003
//! ```
//!
//! At each prompt type the worker's spoken answer (Kazakh); during
//! the instruction phase type any acknowledgement («түсінікті») to
//! advance.  The final protocol prints the pass/fail verdict for the
//! ОТ/ТБ ИТР to sign.

use std::io::{self, Write};

use adam_dialog::briefing_session::BriefingSession;
use adam_dialog::procedure_loader::shared_procedures;

fn main() {
    let arg = std::env::args().nth(1);

    match arg.as_deref() {
        None | Some("--help") | Some("-h") => {
            eprintln!(
                "usage: adam_briefing <procedure_id> | --list\n\
                 example: adam_briefing kk_metallurgy_loto_003"
            );
        }
        Some("--list") => {
            let procs = shared_procedures();
            if procs.is_empty() {
                eprintln!("(no procedures loaded — is data/procedures present?)");
                return;
            }
            eprintln!("{} procedures:", procs.len());
            for p in procs {
                println!("{:34} {}", p.id, p.title_kk);
            }
        }
        Some(id) => run_session(id),
    }
}

fn run_session(id: &str) {
    let Some(mut session) = BriefingSession::from_id(id) else {
        eprintln!("procedure `{id}` not found. Run with --list to see available ids.");
        return;
    };

    println!("{}\n", session.begin());

    let stdin = io::stdin();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap_or(0) == 0 {
            eprintln!("\n(input closed — session aborted before completion)");
            return;
        }
        let reply = session.advance(line.trim());
        println!("\n{}\n", reply.text);
        if reply.done {
            break;
        }
    }
}
