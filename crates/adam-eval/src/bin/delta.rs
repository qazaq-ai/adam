use std::{env, fs, process::ExitCode};

use adam_eval::{EvalBenchmarkReport, EvalSuite, build_eval_benchmark_delta_report};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(suite_path) = args.next() else {
        eprintln!("usage: delta <benchmark-manifest> <expected-benchmark-report>");
        return ExitCode::FAILURE;
    };
    let Some(expected_path) = args.next() else {
        eprintln!("usage: delta <benchmark-manifest> <expected-benchmark-report>");
        return ExitCode::FAILURE;
    };

    let suite: EvalSuite = match read_json(&suite_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read benchmark manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let expected: EvalBenchmarkReport = match read_json(&expected_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read expected benchmark report: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_eval_benchmark_delta_report(&suite, &expected) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("benchmark delta serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build eval benchmark delta report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
