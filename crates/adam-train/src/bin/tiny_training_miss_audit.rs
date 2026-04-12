use std::{env, fs, process::ExitCode};

use adam_corpus::{SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_train::{
    BaselineTrainingManifest, CleanTrainingCorpusManifest, CleanTrainingCorpusPack,
    TinyCleanTrainingProfilePromotionReport, TinyCleanTrainingProfileSuiteManifest,
    assemble_tiny_clean_training_pack_from_promotion, build_tiny_clean_training_miss_audit_report,
};

const USAGE: &str = "usage: tiny_training_miss_audit <training-manifest> <tiny-profile-suite-manifest> <tiny-profile-promotion-report>";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let Some(profile_suite_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let Some(promotion_report_path) = args.next() else {
        eprintln!("{USAGE}");
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
    let profile_suite: TinyCleanTrainingProfileSuiteManifest = match read_json(&profile_suite_path)
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read tiny clean training profile suite manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let promotion_report: TinyCleanTrainingProfilePromotionReport =
        match read_json(&promotion_report_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny clean training profile promotion report: {error}");
                return ExitCode::FAILURE;
            }
        };
    let manifest_dir = std::path::Path::new(&profile_suite_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let clean_corpus_manifest_path = manifest_dir.join(&profile_suite.source_clean_corpus_manifest);
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
    let clean_corpus_pack_path = manifest_dir.join(&profile_suite.source_clean_corpus_pack);
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
    let pack = match assemble_tiny_clean_training_pack_from_promotion(
        &profile_suite,
        &promotion_report,
        &clean_corpus_manifest,
        &clean_corpus_pack,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to assemble tiny clean training pack from promoted profile: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_tiny_clean_training_miss_audit_report(&manifest, &registry, &rules, &report, &pack)
    {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("tiny clean training miss audit serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny clean training miss audit report: {error}");
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
