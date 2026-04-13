use std::{env, fs, process::ExitCode};

use adam_corpus::{SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_train::{
    BaselineTrainingManifest, CleanTrainingCorpusManifest, CleanTrainingCorpusPack,
    MiniCleanTrainingManifest, assemble_mini_clean_training_pack,
    build_mini_clean_training_miss_audit_report,
};

const USAGE: &str = "usage: mini_training_miss_audit <training-manifest> <mini-manifest> <clean-corpus-manifest> <clean-corpus-pack>";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(training_manifest_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let Some(mini_manifest_path) = args.next() else {
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

    let manifest: BaselineTrainingManifest = match read_json(&training_manifest_path) {
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
    let acceptance_report: SourceAcceptanceReport =
        match read_json(&manifest.acceptance_report_manifest) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read acceptance report: {error}");
                return ExitCode::FAILURE;
            }
        };
    let mini_manifest: MiniCleanTrainingManifest = match read_json(&mini_manifest_path) {
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

    let pack = match assemble_mini_clean_training_pack(
        &mini_manifest,
        &clean_corpus_manifest,
        &clean_corpus_pack,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to assemble mini clean training pack: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_mini_clean_training_miss_audit_report(
        &manifest,
        &registry,
        &rules,
        &acceptance_report,
        &pack,
    ) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("mini clean training miss audit serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build mini clean training miss audit report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
