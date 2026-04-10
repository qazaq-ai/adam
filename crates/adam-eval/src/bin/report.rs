use std::{env, fs, process::ExitCode};

use adam_eval::{EvalSuite, build_eval_benchmark_report};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(suite_path) = args.next() else {
        eprintln!("usage: report <benchmark-manifest>");
        return ExitCode::FAILURE;
    };

    let suite: EvalSuite = match read_json(&suite_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read benchmark manifest: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_eval_benchmark_report(&suite) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("benchmark report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build eval benchmark report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
