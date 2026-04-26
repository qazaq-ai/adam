use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!(
            "usage: cargo run -p adam-train --bin bump_foundation_version -- <current> <target> <file> [file...]"
        );
        return ExitCode::FAILURE;
    }

    let current = &args[0];
    let target = &args[1];
    let files = &args[2..];

    for file in files {
        let contents = match fs::read_to_string(file) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("cannot read {file}: {err}");
                return ExitCode::FAILURE;
            }
        };
        let updated = contents.replace(current, target);
        if let Err(err) = fs::write(file, updated) {
            eprintln!("cannot write {file}: {err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
