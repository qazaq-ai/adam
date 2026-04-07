use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusManifest {
    pub name: String,
    pub language: String,
    pub policy: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CorpusError {
    #[error("corpus language must be kazakh")]
    NonKazakhLanguage,
}

impl CorpusManifest {
    pub fn validate(&self) -> Result<(), CorpusError> {
        if self.language != "kazakh" {
            return Err(CorpusError::NonKazakhLanguage);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CorpusError, CorpusManifest};

    #[test]
    fn rejects_non_kazakh_manifests() {
        let manifest = CorpusManifest {
            name: "mixed".to_string(),
            language: "mixed".to_string(),
            policy: "forbidden".to_string(),
        };

        assert_eq!(manifest.validate(), Err(CorpusError::NonKazakhLanguage));
    }
}
