use std::{env, fs, process};

use adam_tokenizer::{TokenizerDryRunPack, TokenizerExperiment, build_dry_run_report};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: dry_run <experiment-manifest> <sample-pack-manifest>".into());
    }

    let experiment: TokenizerExperiment = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let pack: TokenizerDryRunPack = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
    let report = build_dry_run_report(&experiment, &pack)?;

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
