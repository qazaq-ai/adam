use std::{env, fs, process::ExitCode};

use adam_train::{
    CleanTrainingCorpusManifest, CleanTrainingCorpusPack, build_clean_training_corpus_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("usage: report_clean_corpus <clean-corpus-manifest> <clean-corpus-pack>");
        return ExitCode::FAILURE;
    };
    let Some(pack_path) = args.next() else {
        eprintln!("usage: report_clean_corpus <clean-corpus-manifest> <clean-corpus-pack>");
        return ExitCode::FAILURE;
    };

    let manifest: CleanTrainingCorpusManifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read clean corpus manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let pack: CleanTrainingCorpusPack = match read_json(&pack_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read clean corpus pack: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_clean_training_corpus_report(&manifest, &pack) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("clean corpus report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build clean corpus report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
