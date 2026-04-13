use std::{env, fs, path::Path, process::ExitCode};

use adam_train::{
    CleanTrainingCorpusManifest, CleanTrainingCorpusPack, MiniCleanTrainingManifest,
    assemble_mini_clean_training_pack,
};

const USAGE: &str = "usage: assemble_mini_clean_training <mini-manifest> <clean-corpus-manifest> <clean-corpus-pack>";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let Some(clean_corpus_manifest_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let Some(clean_corpus_pack_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };

    let manifest: MiniCleanTrainingManifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read mini clean training manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let clean_corpus_manifest: CleanTrainingCorpusManifest =
        match read_json(&clean_corpus_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read clean corpus manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let clean_corpus_pack: CleanTrainingCorpusPack = match read_json(&clean_corpus_pack_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read clean corpus pack: {error}");
            return ExitCode::FAILURE;
        }
    };

    match assemble_mini_clean_training_pack(&manifest, &clean_corpus_manifest, &clean_corpus_pack) {
        Ok(pack) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pack).expect("mini clean training pack serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to assemble mini clean training pack: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(Path::new(path))?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
