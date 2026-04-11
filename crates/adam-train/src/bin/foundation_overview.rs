use std::{env, fs, process::ExitCode};

use adam_corpus::{SourceAcceptanceDeltaReport, SourceAcceptanceSummaryReport};
use adam_eval::{EvalBenchmarkDeltaReport, EvalBenchmarkReport};
use adam_tokenizer::{TokenizerExperimentDeltaReport, TokenizerExperimentReport};
use adam_train::{
    BaselineTrainingConsistencyReport, BaselineTrainingDeltaReport, FoundationOverviewReport,
    TinyCleanTrainingProfileBaselineDeltaReport, TinyCleanTrainingProfileBaselineReport,
    TinyCleanTrainingProfileStrategyDeltaReport, TinyCleanTrainingProfileStrategyReport,
    TinyCleanTrainingReport, build_foundation_overview_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(corpus_summary_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(corpus_delta_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(tokenizer_report_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(tokenizer_delta_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(eval_report_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(eval_delta_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(training_consistency_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(training_delta_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(tiny_training_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(tiny_profile_policy_report_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(tiny_profile_policy_delta_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(tiny_profile_strategy_report_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };
    let Some(tiny_profile_strategy_delta_path) = args.next() else {
        eprintln!(
            "usage: foundation_overview <corpus-summary> <corpus-delta> <tokenizer-report> <tokenizer-delta> <eval-report> <eval-delta> <training-consistency> <training-delta> <tiny-training-report> <tiny-profile-policy-report> <tiny-profile-policy-delta> <tiny-profile-strategy-report> <tiny-profile-strategy-delta>"
        );
        return ExitCode::FAILURE;
    };

    let corpus_summary: SourceAcceptanceSummaryReport = match load(&corpus_summary_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let corpus_delta: SourceAcceptanceDeltaReport = match load(&corpus_delta_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let tokenizer_report: TokenizerExperimentReport = match load(&tokenizer_report_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let tokenizer_delta: TokenizerExperimentDeltaReport = match load(&tokenizer_delta_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let eval_report: EvalBenchmarkReport = match load(&eval_report_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let eval_delta: EvalBenchmarkDeltaReport = match load(&eval_delta_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let training_consistency: BaselineTrainingConsistencyReport =
        match load(&training_consistency_path) {
            Ok(value) => value,
            Err(code) => return code,
        };
    let training_delta: BaselineTrainingDeltaReport = match load(&training_delta_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let tiny_training: TinyCleanTrainingReport = match load(&tiny_training_path) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let tiny_profile_policy: TinyCleanTrainingProfileBaselineReport =
        match load(&tiny_profile_policy_report_path) {
            Ok(value) => value,
            Err(code) => return code,
        };
    let tiny_profile_policy_delta: TinyCleanTrainingProfileBaselineDeltaReport =
        match load(&tiny_profile_policy_delta_path) {
            Ok(value) => value,
            Err(code) => return code,
        };
    let tiny_profile_strategy: TinyCleanTrainingProfileStrategyReport =
        match load(&tiny_profile_strategy_report_path) {
            Ok(value) => value,
            Err(code) => return code,
        };
    let tiny_profile_strategy_delta: TinyCleanTrainingProfileStrategyDeltaReport =
        match load(&tiny_profile_strategy_delta_path) {
            Ok(value) => value,
            Err(code) => return code,
        };

    let report: FoundationOverviewReport = build_foundation_overview_report(
        &corpus_summary,
        &corpus_delta,
        &tokenizer_report,
        &tokenizer_delta,
        &eval_report,
        &eval_delta,
        &training_consistency,
        &training_delta,
        &tiny_training,
        &tiny_profile_policy,
        &tiny_profile_policy_delta,
        &tiny_profile_strategy,
        &tiny_profile_strategy_delta,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("foundation overview serializes")
    );
    ExitCode::SUCCESS
}

fn load<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, ExitCode> {
    match read_json(path) {
        Ok(value) => Ok(value),
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            Err(ExitCode::FAILURE)
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
