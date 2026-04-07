use serde::{Deserialize, Serialize};

pub const MODEL_NAME: &str = "adam";
pub const MODEL_SCOPE: &str = "kazakh-first text model foundation";
pub const SUPPORTED_LANGUAGE: &str = "kazakh";
pub const SUPPORTED_SCRIPT: &str = "cyrillic";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelIdentity {
    pub name: String,
    pub scope: String,
    pub language: String,
    pub script: String,
    pub phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationPrinciples {
    pub kazakh_only: bool,
    pub cyrillic_only: bool,
    pub corpus_first: bool,
    pub eval_first: bool,
    pub no_multilingual_scope: bool,
}

impl Default for ModelIdentity {
    fn default() -> Self {
        Self {
            name: MODEL_NAME.to_string(),
            scope: MODEL_SCOPE.to_string(),
            language: SUPPORTED_LANGUAGE.to_string(),
            script: SUPPORTED_SCRIPT.to_string(),
            phase: "foundation".to_string(),
        }
    }
}

impl Default for FoundationPrinciples {
    fn default() -> Self {
        Self {
            kazakh_only: true,
            cyrillic_only: true,
            corpus_first: true,
            eval_first: true,
            no_multilingual_scope: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FoundationPrinciples, ModelIdentity};

    #[test]
    fn exposes_kazakh_first_identity() {
        let identity = ModelIdentity::default();

        assert_eq!(identity.name, "adam");
        assert_eq!(identity.language, "kazakh");
        assert_eq!(identity.script, "cyrillic");
        assert_eq!(identity.phase, "foundation");
    }

    #[test]
    fn foundation_principles_are_strict() {
        let principles = FoundationPrinciples::default();

        assert!(principles.kazakh_only);
        assert!(principles.cyrillic_only);
        assert!(principles.corpus_first);
        assert!(principles.eval_first);
    }
}
