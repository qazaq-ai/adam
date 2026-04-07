use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusStage {
    Raw,
    Curated,
    Eval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePolicy {
    KazakhOnly,
    KazakhPrimary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    PublicText,
    ReferenceText,
    AdministrativeText,
    EducationalText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDomain {
    General,
    Reference,
    Administrative,
    Education,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseClass {
    Open,
    ReviewRequired,
    InternalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityTier {
    Seed,
    Reviewed,
    TrainingReady,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRegistryEntry {
    pub id: String,
    pub stage: CorpusStage,
    pub language: String,
    pub script: String,
    pub source_type: SourceType,
    pub domain: SourceDomain,
    pub license_class: LicenseClass,
    pub quality_tier: QualityTier,
    pub provenance: String,
    pub allowed_for_training: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRegistry {
    pub version: String,
    pub entries: Vec<SourceRegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceScoringRules {
    pub version: String,
    pub minimum_acceptance_score: i32,
    pub open_license_bonus: i32,
    pub reviewed_quality_bonus: i32,
    pub training_ready_bonus: i32,
    pub administrative_domain_bonus: i32,
    pub reference_domain_bonus: i32,
    pub raw_stage_penalty: i32,
    pub review_required_penalty: i32,
    pub internal_only_penalty: i32,
    pub seed_quality_penalty: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAcceptance {
    pub score: i32,
    pub accepted_for_training: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAcceptanceRecord {
    pub source_id: String,
    pub score: i32,
    pub accepted_for_training: bool,
    pub positive_signals: Vec<String>,
    pub negative_signals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAcceptanceReport {
    pub version: String,
    pub name: String,
    pub source_registry_manifest: String,
    pub scoring_rules_manifest: String,
    pub records: Vec<SourceAcceptanceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusManifest {
    pub version: String,
    pub name: String,
    pub language: String,
    pub script: String,
    pub stage: CorpusStage,
    pub source_policy: SourcePolicy,
    pub domains: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CorpusError {
    #[error("corpus language must be kazakh")]
    NonKazakhLanguage,
    #[error("corpus script must be cyrillic")]
    NonCyrillicScript,
    #[error("corpus domains must not be empty")]
    EmptyDomains,
    #[error("raw corpora cannot claim strict kazakh-only curation")]
    RawCorpusCannotBeStrict,
    #[error("source registry entries must not be empty")]
    EmptySourceRegistry,
    #[error("source id must not be empty")]
    EmptySourceId,
    #[error("source provenance must not be empty")]
    EmptyProvenance,
    #[error("raw sources cannot be allowed for training")]
    RawSourceCannotTrain,
    #[error("sources requiring license review cannot be allowed for training")]
    LicenseReviewRequired,
    #[error("seed-quality sources cannot be allowed for training")]
    SeedQualityCannotTrain,
    #[error("source acceptance report entries must not be empty")]
    EmptyAcceptanceReport,
    #[error("source acceptance report record ids must not be empty")]
    EmptyAcceptanceRecordId,
    #[error("source acceptance report references must not be empty")]
    EmptyAcceptanceReportReference,
    #[error("source acceptance report must contain exactly one record per source")]
    AcceptanceReportCoverageMismatch,
    #[error("source acceptance report contains duplicate source ids")]
    DuplicateAcceptanceRecordId,
    #[error("source acceptance report score mismatch")]
    AcceptanceScoreMismatch,
    #[error("source acceptance report training decision mismatch")]
    AcceptanceDecisionMismatch,
    #[error("source acceptance report signals mismatch")]
    AcceptanceSignalsMismatch,
}

impl CorpusManifest {
    pub fn validate(&self) -> Result<(), CorpusError> {
        if self.language != "kazakh" {
            return Err(CorpusError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(CorpusError::NonCyrillicScript);
        }

        if self.domains.is_empty() {
            return Err(CorpusError::EmptyDomains);
        }

        if self.stage == CorpusStage::Raw && self.source_policy == SourcePolicy::KazakhOnly {
            return Err(CorpusError::RawCorpusCannotBeStrict);
        }

        Ok(())
    }
}

impl SourceRegistry {
    pub fn validate(&self) -> Result<(), CorpusError> {
        if self.entries.is_empty() {
            return Err(CorpusError::EmptySourceRegistry);
        }

        for entry in &self.entries {
            if entry.language != "kazakh" {
                return Err(CorpusError::NonKazakhLanguage);
            }

            if entry.script != "cyrillic" {
                return Err(CorpusError::NonCyrillicScript);
            }

            if entry.id.trim().is_empty() {
                return Err(CorpusError::EmptySourceId);
            }

            if entry.provenance.trim().is_empty() {
                return Err(CorpusError::EmptyProvenance);
            }

            if entry.allowed_for_training && entry.stage == CorpusStage::Raw {
                return Err(CorpusError::RawSourceCannotTrain);
            }

            if entry.allowed_for_training && entry.license_class == LicenseClass::ReviewRequired {
                return Err(CorpusError::LicenseReviewRequired);
            }

            if entry.allowed_for_training && entry.quality_tier == QualityTier::Seed {
                return Err(CorpusError::SeedQualityCannotTrain);
            }
        }

        Ok(())
    }
}

impl SourceScoringRules {
    pub fn score(&self, entry: &SourceRegistryEntry) -> SourceAcceptance {
        let mut score = 0;

        match entry.license_class {
            LicenseClass::Open => score += self.open_license_bonus,
            LicenseClass::ReviewRequired => score -= self.review_required_penalty,
            LicenseClass::InternalOnly => score -= self.internal_only_penalty,
        }

        match entry.quality_tier {
            QualityTier::Seed => score -= self.seed_quality_penalty,
            QualityTier::Reviewed => score += self.reviewed_quality_bonus,
            QualityTier::TrainingReady => score += self.training_ready_bonus,
        }

        match entry.domain {
            SourceDomain::Administrative => score += self.administrative_domain_bonus,
            SourceDomain::Reference => score += self.reference_domain_bonus,
            SourceDomain::General | SourceDomain::Education => {}
        }

        if entry.stage == CorpusStage::Raw {
            score -= self.raw_stage_penalty;
        }

        SourceAcceptance {
            score,
            accepted_for_training: entry.allowed_for_training
                && score >= self.minimum_acceptance_score,
        }
    }

    pub fn build_record(&self, entry: &SourceRegistryEntry) -> SourceAcceptanceRecord {
        let mut positive_signals = BTreeSet::new();
        let mut negative_signals = BTreeSet::new();
        let acceptance = self.score(entry);

        match entry.license_class {
            LicenseClass::Open => {
                positive_signals.insert("open_license".to_string());
            }
            LicenseClass::ReviewRequired => {
                negative_signals.insert("review_required_license".to_string());
            }
            LicenseClass::InternalOnly => {
                negative_signals.insert("internal_only_license".to_string());
            }
        }

        match entry.quality_tier {
            QualityTier::Seed => {
                negative_signals.insert("seed_quality".to_string());
            }
            QualityTier::Reviewed => {
                positive_signals.insert("reviewed_quality".to_string());
            }
            QualityTier::TrainingReady => {
                positive_signals.insert("training_ready_quality".to_string());
            }
        }

        match entry.domain {
            SourceDomain::Administrative => {
                positive_signals.insert("administrative_domain".to_string());
            }
            SourceDomain::Reference => {
                positive_signals.insert("reference_domain".to_string());
            }
            SourceDomain::General | SourceDomain::Education => {}
        }

        if entry.stage == CorpusStage::Raw {
            negative_signals.insert("raw_stage".to_string());
        }

        SourceAcceptanceRecord {
            source_id: entry.id.clone(),
            score: acceptance.score,
            accepted_for_training: acceptance.accepted_for_training,
            positive_signals: positive_signals.into_iter().collect(),
            negative_signals: negative_signals.into_iter().collect(),
        }
    }
}

impl SourceAcceptanceReport {
    pub fn validate(
        &self,
        registry: &SourceRegistry,
        rules: &SourceScoringRules,
    ) -> Result<(), CorpusError> {
        if self.records.is_empty() {
            return Err(CorpusError::EmptyAcceptanceReport);
        }

        if self.source_registry_manifest.trim().is_empty()
            || self.scoring_rules_manifest.trim().is_empty()
        {
            return Err(CorpusError::EmptyAcceptanceReportReference);
        }

        if self.records.len() != registry.entries.len() {
            return Err(CorpusError::AcceptanceReportCoverageMismatch);
        }

        let mut seen = HashSet::new();

        for record in &self.records {
            if record.source_id.trim().is_empty() {
                return Err(CorpusError::EmptyAcceptanceRecordId);
            }

            if !seen.insert(record.source_id.clone()) {
                return Err(CorpusError::DuplicateAcceptanceRecordId);
            }

            let Some(entry) = registry
                .entries
                .iter()
                .find(|entry| entry.id == record.source_id)
            else {
                return Err(CorpusError::AcceptanceReportCoverageMismatch);
            };
            let expected = rules.build_record(entry);

            if record.score != expected.score {
                return Err(CorpusError::AcceptanceScoreMismatch);
            }

            if record.accepted_for_training != expected.accepted_for_training {
                return Err(CorpusError::AcceptanceDecisionMismatch);
            }

            if record.positive_signals != expected.positive_signals
                || record.negative_signals != expected.negative_signals
            {
                return Err(CorpusError::AcceptanceSignalsMismatch);
            }
        }

        Ok(())
    }
}

pub fn build_source_acceptance_report(
    name: &str,
    source_registry_manifest: &str,
    scoring_rules_manifest: &str,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
) -> Result<SourceAcceptanceReport, CorpusError> {
    registry.validate()?;

    let mut records = registry
        .entries
        .iter()
        .map(|entry| rules.build_record(entry))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.source_id.cmp(&right.source_id));

    let report = SourceAcceptanceReport {
        version: registry.version.clone(),
        name: name.to_string(),
        source_registry_manifest: source_registry_manifest.to_string(),
        scoring_rules_manifest: scoring_rules_manifest.to_string(),
        records,
    };
    report.validate(registry, rules)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{
        CorpusError, CorpusManifest, CorpusStage, LicenseClass, QualityTier,
        SourceAcceptanceReport, SourceDomain, SourcePolicy, SourceRegistry, SourceRegistryEntry,
        SourceScoringRules, SourceType, build_source_acceptance_report,
    };

    #[test]
    fn rejects_non_kazakh_manifests() {
        let manifest = CorpusManifest {
            version: "0.0.2".to_string(),
            name: "mixed".to_string(),
            language: "mixed".to_string(),
            script: "cyrillic".to_string(),
            stage: CorpusStage::Curated,
            source_policy: SourcePolicy::KazakhOnly,
            domains: vec!["admin".to_string()],
        };

        assert_eq!(manifest.validate(), Err(CorpusError::NonKazakhLanguage));
    }

    #[test]
    fn rejects_non_cyrillic_manifests() {
        let manifest = CorpusManifest {
            version: "0.0.2".to_string(),
            name: "latin".to_string(),
            language: "kazakh".to_string(),
            script: "latin".to_string(),
            stage: CorpusStage::Curated,
            source_policy: SourcePolicy::KazakhOnly,
            domains: vec!["admin".to_string()],
        };

        assert_eq!(manifest.validate(), Err(CorpusError::NonCyrillicScript));
    }

    #[test]
    fn rejects_empty_domains() {
        let manifest = CorpusManifest {
            version: "0.0.2".to_string(),
            name: "empty".to_string(),
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            stage: CorpusStage::Curated,
            source_policy: SourcePolicy::KazakhOnly,
            domains: Vec::new(),
        };

        assert_eq!(manifest.validate(), Err(CorpusError::EmptyDomains));
    }

    #[test]
    fn rejects_strict_policy_for_raw_stage() {
        let manifest = CorpusManifest {
            version: "0.0.2".to_string(),
            name: "raw".to_string(),
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            stage: CorpusStage::Raw,
            source_policy: SourcePolicy::KazakhOnly,
            domains: vec!["web".to_string()],
        };

        assert_eq!(
            manifest.validate(),
            Err(CorpusError::RawCorpusCannotBeStrict)
        );
    }

    #[test]
    fn rejects_empty_source_registry() {
        let registry = SourceRegistry {
            version: "0.0.2".to_string(),
            entries: Vec::new(),
        };

        assert_eq!(registry.validate(), Err(CorpusError::EmptySourceRegistry));
    }

    #[test]
    fn accepts_kazakh_cyrillic_source_registry() {
        let registry = SourceRegistry {
            version: "0.0.2".to_string(),
            entries: vec![SourceRegistryEntry {
                id: "source_01".to_string(),
                stage: CorpusStage::Raw,
                language: "kazakh".to_string(),
                script: "cyrillic".to_string(),
                source_type: SourceType::PublicText,
                domain: SourceDomain::Administrative,
                license_class: LicenseClass::Open,
                quality_tier: QualityTier::Reviewed,
                provenance: "seed".to_string(),
                allowed_for_training: false,
            }],
        };

        assert!(registry.validate().is_ok());
    }

    #[test]
    fn rejects_training_on_raw_sources() {
        let registry = SourceRegistry {
            version: "0.0.2".to_string(),
            entries: vec![SourceRegistryEntry {
                id: "raw_training".to_string(),
                stage: CorpusStage::Raw,
                language: "kazakh".to_string(),
                script: "cyrillic".to_string(),
                source_type: SourceType::PublicText,
                domain: SourceDomain::General,
                license_class: LicenseClass::Open,
                quality_tier: QualityTier::TrainingReady,
                provenance: "seed".to_string(),
                allowed_for_training: true,
            }],
        };

        assert_eq!(registry.validate(), Err(CorpusError::RawSourceCannotTrain));
    }

    #[test]
    fn scores_reviewed_open_sources_higher_than_seed_raw_sources() {
        let rules = SourceScoringRules {
            version: "0.0.2".to_string(),
            minimum_acceptance_score: 3,
            open_license_bonus: 3,
            reviewed_quality_bonus: 2,
            training_ready_bonus: 4,
            administrative_domain_bonus: 1,
            reference_domain_bonus: 1,
            raw_stage_penalty: 3,
            review_required_penalty: 3,
            internal_only_penalty: 5,
            seed_quality_penalty: 2,
        };

        let reviewed = SourceRegistryEntry {
            id: "reviewed".to_string(),
            stage: CorpusStage::Curated,
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            source_type: SourceType::AdministrativeText,
            domain: SourceDomain::Administrative,
            license_class: LicenseClass::Open,
            quality_tier: QualityTier::Reviewed,
            provenance: "seed".to_string(),
            allowed_for_training: true,
        };

        let seed = SourceRegistryEntry {
            id: "seed".to_string(),
            stage: CorpusStage::Raw,
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            source_type: SourceType::AdministrativeText,
            domain: SourceDomain::Administrative,
            license_class: LicenseClass::ReviewRequired,
            quality_tier: QualityTier::Seed,
            provenance: "seed".to_string(),
            allowed_for_training: false,
        };

        let reviewed_score = rules.score(&reviewed);
        let seed_score = rules.score(&seed);

        assert!(reviewed_score.score > seed_score.score);
        assert!(reviewed_score.accepted_for_training);
        assert!(!seed_score.accepted_for_training);
    }

    #[test]
    fn builds_acceptance_report_with_expected_records() {
        let registry = SourceRegistry {
            version: "0.0.2".to_string(),
            entries: vec![
                SourceRegistryEntry {
                    id: "admin_raw_seed".to_string(),
                    stage: CorpusStage::Raw,
                    language: "kazakh".to_string(),
                    script: "cyrillic".to_string(),
                    source_type: SourceType::AdministrativeText,
                    domain: SourceDomain::Administrative,
                    license_class: LicenseClass::ReviewRequired,
                    quality_tier: QualityTier::Seed,
                    provenance: "manual_seed".to_string(),
                    allowed_for_training: false,
                },
                SourceRegistryEntry {
                    id: "reference_curated_training".to_string(),
                    stage: CorpusStage::Curated,
                    language: "kazakh".to_string(),
                    script: "cyrillic".to_string(),
                    source_type: SourceType::ReferenceText,
                    domain: SourceDomain::Reference,
                    license_class: LicenseClass::Open,
                    quality_tier: QualityTier::TrainingReady,
                    provenance: "manual_reference_seed".to_string(),
                    allowed_for_training: true,
                },
            ],
        };
        let rules = SourceScoringRules {
            version: "0.0.2".to_string(),
            minimum_acceptance_score: 3,
            open_license_bonus: 3,
            reviewed_quality_bonus: 2,
            training_ready_bonus: 4,
            administrative_domain_bonus: 1,
            reference_domain_bonus: 1,
            raw_stage_penalty: 3,
            review_required_penalty: 3,
            internal_only_penalty: 5,
            seed_quality_penalty: 2,
        };

        let report = build_source_acceptance_report(
            "training-ready-sources",
            "data/raw/source_registry.json",
            "data/raw/source_scoring_rules.json",
            &registry,
            &rules,
        )
        .expect("acceptance report");

        assert_eq!(report.records.len(), 2);
        assert!(!report.records[0].accepted_for_training);
        assert!(report.records[1].accepted_for_training);
        assert_eq!(
            report.records[1].positive_signals,
            vec![
                "open_license".to_string(),
                "reference_domain".to_string(),
                "training_ready_quality".to_string(),
            ]
        );
    }

    #[test]
    fn rejects_mismatched_acceptance_report() {
        let registry = SourceRegistry {
            version: "0.0.2".to_string(),
            entries: vec![SourceRegistryEntry {
                id: "reference_curated_training".to_string(),
                stage: CorpusStage::Curated,
                language: "kazakh".to_string(),
                script: "cyrillic".to_string(),
                source_type: SourceType::ReferenceText,
                domain: SourceDomain::Reference,
                license_class: LicenseClass::Open,
                quality_tier: QualityTier::TrainingReady,
                provenance: "manual_reference_seed".to_string(),
                allowed_for_training: true,
            }],
        };
        let rules = SourceScoringRules {
            version: "0.0.2".to_string(),
            minimum_acceptance_score: 3,
            open_license_bonus: 3,
            reviewed_quality_bonus: 2,
            training_ready_bonus: 4,
            administrative_domain_bonus: 1,
            reference_domain_bonus: 1,
            raw_stage_penalty: 3,
            review_required_penalty: 3,
            internal_only_penalty: 5,
            seed_quality_penalty: 2,
        };
        let report = SourceAcceptanceReport {
            version: "0.0.2".to_string(),
            name: "bad".to_string(),
            source_registry_manifest: "data/raw/source_registry.json".to_string(),
            scoring_rules_manifest: "data/raw/source_scoring_rules.json".to_string(),
            records: vec![super::SourceAcceptanceRecord {
                source_id: "reference_curated_training".to_string(),
                score: 0,
                accepted_for_training: false,
                positive_signals: Vec::new(),
                negative_signals: Vec::new(),
            }],
        };

        assert_eq!(
            report.validate(&registry, &rules),
            Err(CorpusError::AcceptanceScoreMismatch)
        );
    }
}
