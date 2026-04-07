use serde::{Deserialize, Serialize};

pub const MODEL_NAME: &str = "adam";
pub const MODEL_SCOPE: &str = "kazakh-first text model foundation";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelIdentity {
    pub name: String,
    pub scope: String,
    pub language: String,
}

impl Default for ModelIdentity {
    fn default() -> Self {
        Self {
            name: MODEL_NAME.to_string(),
            scope: MODEL_SCOPE.to_string(),
            language: "kazakh".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ModelIdentity;

    #[test]
    fn exposes_kazakh_first_identity() {
        let identity = ModelIdentity::default();

        assert_eq!(identity.name, "adam");
        assert_eq!(identity.language, "kazakh");
    }
}
