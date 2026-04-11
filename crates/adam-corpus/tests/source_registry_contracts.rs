use std::fs;

use adam_corpus::{
    SourceAcceptanceDeltaReport, SourceAcceptanceReport, SourceAcceptanceSummaryReport,
    SourceRegistry, SourceScoringRules, build_source_acceptance_delta_report,
    build_source_acceptance_report, build_source_acceptance_summary_report,
};

#[test]
fn source_registry_stays_kazakh_cyrillic_and_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(path).expect("source registry file"))
            .expect("valid source registry json");

    registry.validate().expect("source registry contract");
    assert!(!registry.entries.is_empty());
    assert!(
        registry
            .entries
            .iter()
            .all(|entry| entry.language == "kazakh")
    );
    assert!(
        registry
            .entries
            .iter()
            .all(|entry| entry.script == "cyrillic")
    );
}

#[test]
fn source_scoring_rules_manifest_is_present_and_usable() {
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry file"))
            .expect("valid source registry json");

    assert!(rules.minimum_acceptance_score >= 0);

    let seed_entry = registry
        .entries
        .iter()
        .find(|entry| entry.id == "seed_public_admin_text")
        .expect("seed source");
    let seed_score = rules.score(seed_entry);
    assert!(seed_score.score <= rules.minimum_acceptance_score);
    assert!(!seed_score.accepted_for_training);

    let accepted_entries = registry
        .entries
        .iter()
        .filter(|entry| entry.allowed_for_training)
        .collect::<Vec<_>>();
    assert!(accepted_entries.len() >= 2);
    for accepted_entry in accepted_entries {
        let accepted_score = rules.score(accepted_entry);
        assert!(accepted_score.score >= rules.minimum_acceptance_score);
        assert!(accepted_score.accepted_for_training);
    }
}

#[test]
fn source_acceptance_report_matches_registry_and_rules() {
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_report.json"
    );
    let report: SourceAcceptanceReport =
        serde_json::from_str(&fs::read_to_string(report_path).expect("source acceptance report"))
            .expect("valid source acceptance report json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry file"))
            .expect("valid source registry json");

    report
        .validate(&registry, &rules)
        .expect("source acceptance report contract");

    let rebuilt = build_source_acceptance_report(
        &report.name,
        &report.source_registry_manifest,
        &report.scoring_rules_manifest,
        &registry,
        &rules,
    )
    .expect("rebuilt acceptance report");
    assert_eq!(rebuilt, report);
    assert_eq!(report.version, "0.0.4");
}

#[test]
fn source_acceptance_summary_report_matches_expected_artifact() {
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_report.json"
    );
    let report: SourceAcceptanceReport =
        serde_json::from_str(&fs::read_to_string(report_path).expect("source acceptance report"))
            .expect("valid source acceptance report json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry file"))
            .expect("valid source registry json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_summary_report.json"
    );
    let expected: SourceAcceptanceSummaryReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("source acceptance summary report"),
    )
    .expect("valid source acceptance summary report json");

    let actual =
        build_source_acceptance_summary_report(&report, &registry, &rules).expect("summary");

    assert_eq!(actual, expected);
}

#[test]
fn source_acceptance_delta_report_matches_expected_artifact() {
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_report.json"
    );
    let report: SourceAcceptanceReport =
        serde_json::from_str(&fs::read_to_string(report_path).expect("source acceptance report"))
            .expect("valid source acceptance report json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry file"))
            .expect("valid source registry json");
    let expected_summary_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_summary_report.json"
    );
    let expected_summary: SourceAcceptanceSummaryReport = serde_json::from_str(
        &fs::read_to_string(expected_summary_path).expect("source acceptance summary report"),
    )
    .expect("valid source acceptance summary report json");
    let expected_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_delta_report.json"
    );
    let expected_delta: SourceAcceptanceDeltaReport = serde_json::from_str(
        &fs::read_to_string(expected_delta_path).expect("source acceptance delta report"),
    )
    .expect("valid source acceptance delta report json");

    let actual =
        build_source_acceptance_delta_report(&report, &registry, &rules, &expected_summary)
            .expect("delta");

    assert_eq!(actual, expected_delta);
}
