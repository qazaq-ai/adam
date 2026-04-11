use std::{env, fs, process::ExitCode};

use adam_corpus::{SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_train::{
    BaselineTrainingManifest, CleanTrainingCorpusManifest, CleanTrainingCorpusPack,
    TinyCleanTrainingSelectionManifest, assemble_tiny_clean_training_pack_from_corpus,
    build_tiny_clean_training_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("usage: tiny_train <training-manifest> <tiny-clean-selection-manifest>");
        return ExitCode::FAILURE;
    };
    let Some(selection_manifest_path) = args.next() else {
        eprintln!("usage: tiny_train <training-manifest> <tiny-clean-selection-manifest>");
        return ExitCode::FAILURE;
    };

    let manifest: BaselineTrainingManifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read training manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let registry: SourceRegistry = match read_json(&manifest.source_registry_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read source registry: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SourceScoringRules = match read_json(&manifest.scoring_rules_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read scoring rules: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report: SourceAcceptanceReport = match read_json(&manifest.acceptance_report_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read acceptance report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let selection_manifest: TinyCleanTrainingSelectionManifest =
        match read_json(&selection_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny clean training selection manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let manifest_dir = std::path::Path::new(&selection_manifest_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let clean_corpus_manifest_path =
        manifest_dir.join(&selection_manifest.source_clean_corpus_manifest);
    let clean_corpus_manifest: CleanTrainingCorpusManifest =
        match read_json_path(&clean_corpus_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "failed to read clean training corpus manifest {}: {error}",
                    clean_corpus_manifest_path.display()
                );
                return ExitCode::FAILURE;
            }
        };
    let clean_corpus_pack_path = manifest_dir.join(&selection_manifest.source_clean_corpus_pack);
    let clean_corpus_pack: CleanTrainingCorpusPack = match read_json_path(&clean_corpus_pack_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "failed to read clean training corpus pack {}: {error}",
                clean_corpus_pack_path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let pack = match assemble_tiny_clean_training_pack_from_corpus(
        &selection_manifest,
        &clean_corpus_manifest,
        &clean_corpus_pack,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to assemble tiny clean training pack from corpus: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_tiny_clean_training_report(&manifest, &registry, &rules, &report, &pack) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("tiny training report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny clean training report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}

fn read_json_path<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
