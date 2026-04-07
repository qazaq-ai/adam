use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerProfile {
    pub language: String,
    pub script: String,
    pub strategy: String,
}

impl Default for TokenizerProfile {
    fn default() -> Self {
        Self {
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            strategy: "train_kazakh_first_tokenizer".to_string(),
        }
    }
}

pub fn normalize_text(text: &str) -> String {
    text.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{TokenizerProfile, normalize_text};

    #[test]
    fn normalizes_basic_input() {
        assert_eq!(normalize_text(" Сәлем "), "сәлем");
    }

    #[test]
    fn default_profile_is_kazakh_cyrillic() {
        let profile = TokenizerProfile::default();

        assert_eq!(profile.language, "kazakh");
        assert_eq!(profile.script, "cyrillic");
    }
}
