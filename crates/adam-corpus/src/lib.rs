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
pub struct SourceRegistryEntry {
    pub id: String,
    pub stage: CorpusStage,
    pub language: String,
    pub script: String,
    pub source_type: String,
    pub provenance: String,
    pub allowed_for_training: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRegistry {
    pub version: String,
    pub entries: Vec<SourceRegistryEntry>,
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
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CorpusError, CorpusManifest, CorpusStage, SourcePolicy, SourceRegistry, SourceRegistryEntry,
    };

    #[test]
    fn rejects_non_kazakh_manifests() {
        let manifest = CorpusManifest {
            version: "0.0.1".to_string(),
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
            version: "0.0.1".to_string(),
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
            version: "0.0.1".to_string(),
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
            version: "0.0.1".to_string(),
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
            version: "0.0.1".to_string(),
            entries: Vec::new(),
        };

        assert_eq!(registry.validate(), Err(CorpusError::EmptySourceRegistry));
    }

    #[test]
    fn accepts_kazakh_cyrillic_source_registry() {
        let registry = SourceRegistry {
            version: "0.0.1".to_string(),
            entries: vec![SourceRegistryEntry {
                id: "source_01".to_string(),
                stage: CorpusStage::Raw,
                language: "kazakh".to_string(),
                script: "cyrillic".to_string(),
                source_type: "public_text".to_string(),
                provenance: "seed".to_string(),
                allowed_for_training: false,
            }],
        };

        assert!(registry.validate().is_ok());
    }
}
