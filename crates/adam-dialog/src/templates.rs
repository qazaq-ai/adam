//! Template repository — loads `data/dialog/templates/v1.toml` at runtime.
//!
//! Structure:
//! ```toml
//! version = "0.7.5"
//!
//! [[families]]
//! key = "greeting.casual"
//! templates = ["сәлем", "сәлем, қалыңыз қалай"]
//! ```
//!
//! The repository is a flat `key → Vec<String>` map. Keys use dotted
//! paths (`intent.sub_kind`) for readability but are opaque strings to
//! the loader. The planner knows which key to look up for each intent.
//!
//! Missing keys are fatal at load time — we want configuration errors
//! to surface on startup, not during a conversation.

use std::{collections::HashMap, fs, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawFile {
    #[allow(dead_code)]
    version: String,
    families: Vec<RawFamily>,
}

#[derive(Debug, Deserialize)]
struct RawFamily {
    key: String,
    templates: Vec<String>,
}

/// Loaded template repository. Held by value; cheap to clone.
#[derive(Debug, Clone)]
pub struct TemplateRepository {
    by_key: HashMap<String, Vec<String>>,
}

impl TemplateRepository {
    /// Load from the canonical repository path. Searches in parent
    /// directories if the default path doesn't exist — useful for tests
    /// run from various working directories.
    pub fn load_default() -> Result<Self, TemplateError> {
        for candidate in [
            "data/dialog/templates/v1.toml",
            "../data/dialog/templates/v1.toml",
            "../../data/dialog/templates/v1.toml",
        ] {
            if Path::new(candidate).exists() {
                return Self::load(candidate);
            }
        }
        Err(TemplateError::NotFound)
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, TemplateError> {
        let raw_text = fs::read_to_string(&path).map_err(|e| TemplateError::Io {
            path: path.as_ref().display().to_string(),
            source: e,
        })?;
        let raw: RawFile = toml::from_str(&raw_text).map_err(|e| TemplateError::Parse {
            path: path.as_ref().display().to_string(),
            message: e.to_string(),
        })?;
        let mut by_key = HashMap::new();
        for family in raw.families {
            if family.templates.is_empty() {
                return Err(TemplateError::EmptyFamily { key: family.key });
            }
            by_key.insert(family.key, family.templates);
        }
        Ok(Self { by_key })
    }

    /// Minimal hardcoded fallback used when the TOML file is absent
    /// (e.g., in isolated unit tests). One template per intent-key
    /// covering the full v0.8.0 intent surface so dialog keeps working
    /// even if `data/dialog/templates/v1.toml` is missing.
    pub fn hardcoded_fallback() -> Self {
        let mut by_key = HashMap::new();
        // v0.7.0 intents
        by_key.insert("greeting.casual".into(), vec!["сәлем".into()]);
        by_key.insert("greeting.polite".into(), vec!["сәлеметсіз бе".into()]);
        by_key.insert("greeting.morning".into(), vec!["қайырлы таң".into()]);
        by_key.insert("greeting.day".into(), vec!["қайырлы күн".into()]);
        by_key.insert("greeting.evening".into(), vec!["қайырлы кеш".into()]);
        by_key.insert("farewell".into(), vec!["сау бол".into()]);
        by_key.insert("affirmation".into(), vec!["иә".into()]);
        by_key.insert("negation".into(), vec!["жоқ".into()]);
        // v0.7.5 intents
        by_key.insert("thanks".into(), vec!["оқасы жоқ".into()]);
        by_key.insert("apology".into(), vec!["ештеңе емес".into()]);
        by_key.insert("ask_how_are_you".into(), vec!["жақсымын, рахмет".into()]);
        by_key.insert("statement_of_wellbeing".into(), vec!["жақсы екен".into()]);
        by_key.insert("ask_name".into(), vec!["менің атым адам".into()]);
        // v0.8.0 intents
        by_key.insert(
            "statement_of_name".into(),
            vec!["қош келдіңіз {name}".into()],
        );
        by_key.insert("ask_age".into(), vec!["мен әлі жаспын".into()]);
        by_key.insert("statement_of_age".into(), vec!["түсіндім".into()]);
        by_key.insert(
            "ask_location".into(),
            vec!["мен сандық әлемде тұрамын".into()],
        );
        by_key.insert("statement_of_location".into(), vec!["түсіндім".into()]);
        by_key.insert(
            "ask_location.with_known_user.geo_feature".into(),
            vec!["менің білуімше, сіз {city} жақтансыз".into()],
        );
        by_key.insert(
            "statement_of_location.geo_feature".into(),
            vec!["{city} жақтан екеніңізді ұқтым".into()],
        );
        by_key.insert(
            "ask_occupation".into(),
            vec!["мен сөйлесуге жаралғанмын".into()],
        );
        by_key.insert("statement_of_occupation".into(), vec!["түсіндім".into()]);
        by_key.insert("ask_family".into(), vec!["мен жалғызбын".into()]);
        by_key.insert("statement_of_family".into(), vec!["қуаныштымын".into()]);
        by_key.insert("ask_weather".into(), vec!["менде терезе жоқ".into()]);
        by_key.insert("statement_of_weather".into(), vec!["түсіндім".into()]);
        by_key.insert("ask_time".into(), vec!["уақытты білмеймін".into()]);
        by_key.insert("compliment".into(), vec!["рахмет".into()]);
        by_key.insert("request".into(), vec!["әрине, айтыңыз".into()]);
        by_key.insert("well_wishes".into(), vec!["сізге де".into()]);
        // v1.1.0 intents
        by_key.insert("insult".into(), vec!["сізге ренжімеймін".into()]);
        by_key.insert(
            "unknown.with_noun".into(),
            vec!["ах, {noun} туралы айтасыз ба".into()],
        );
        by_key.insert(
            "unknown.with_grounded_fact".into(),
            // v5.23.0 — drop the «{noun} туралы қысқаша айтсам:» meta-opener
            // (live-feedback anti-meta-opener pass). The body already mentions
            // the topic; the wrapper sounded like a Wikipedia citation.
            vec!["{fact}".into()],
        );
        // fallback
        by_key.insert("unknown".into(), vec!["түсінбедім".into()]);
        Self { by_key }
    }

    /// Look up the template list for `key`. Returns the fallback
    /// `["түсінбедім"]` if the key is unknown — this keeps the dialog
    /// loop resilient to planner/repository drift.
    pub fn get(&self, key: &str) -> &[String] {
        self.by_key
            .get(key)
            .map(|v| v.as_slice())
            .unwrap_or_else(|| {
                self.by_key
                    .get("unknown")
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
            })
    }

    /// Count of registered keys, for diagnostics.
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// **v5.10.0 — Codex follow-up review (B3).** Direct key-presence
    /// check that bypasses the `unknown`-family fallback in `get`.
    /// Required by override branches that try a per-error-code
    /// specialised family first (e.g. `ask_fix_previous_error.E0107`)
    /// and need to know whether to fall back to a generic family
    /// without accidentally rendering the `unknown` clarification
    /// templates.
    pub fn has_key(&self, key: &str) -> bool {
        self.by_key.contains_key(key)
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("no template file found in the canonical search paths")]
    NotFound,
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error in {path}: {message}")]
    Parse { path: String, message: String },
    #[error("family '{key}' has no templates")]
    EmptyFamily { key: String },
}
