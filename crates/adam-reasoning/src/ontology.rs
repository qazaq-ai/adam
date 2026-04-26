//! Lightweight ontology contract for admissible symbolic facts.
//!
//! This module does not try to solve open-world semantics. Its job is
//! narrower and harder: encode the small set of type constraints the
//! repository already relies on, so graph/reasoner consumers can
//! reject structurally invalid facts before verbalisation.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::{Fact, FactSource, Predicate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OntologyIssue {
    RulePredicateMismatch {
        rule_id: String,
        predicate: Predicate,
    },
    PlaceObjectRequired {
        predicate: Predicate,
        object: String,
    },
    TimeLikeRequired {
        subject: String,
        object: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedFactValidationIssue {
    Head(OntologyIssue),
    EmptySupportChain,
    SupportPatternMismatch {
        rule_id: String,
    },
    MissingSupportSource {
        pack: String,
        sample_id: String,
    },
    SupportingFactInvalid {
        pack: String,
        sample_id: String,
        issue: OntologyIssue,
    },
}

pub fn validate_fact(
    subject: &str,
    predicate: Predicate,
    object: &str,
) -> Result<(), OntologyIssue> {
    if predicate_requires_place_object(predicate) && !is_place_like_root(object) {
        return Err(OntologyIssue::PlaceObjectRequired {
            predicate,
            object: object.to_string(),
        });
    }
    if predicate == Predicate::After && !(is_time_like_root(subject) && is_time_like_root(object)) {
        return Err(OntologyIssue::TimeLikeRequired {
            subject: subject.to_string(),
            object: object.to_string(),
        });
    }
    Ok(())
}

pub fn validate_derived_fact(
    rule_id: &str,
    subject: &str,
    predicate: Predicate,
    object: &str,
) -> Result<(), OntologyIssue> {
    if !rule_supports_predicate(rule_id, predicate) {
        return Err(OntologyIssue::RulePredicateMismatch {
            rule_id: rule_id.to_string(),
            predicate,
        });
    }
    validate_fact(subject, predicate, object)
}

pub fn validate_derived_fact_with_supports(
    derived: &crate::reasoner::DerivedFact,
    support_facts: &[Fact],
) -> Result<(), DerivedFactValidationIssue> {
    validate_derived_fact(
        &derived.rule_id,
        &derived.subject.root,
        derived.predicate,
        &derived.object.root,
    )
    .map_err(DerivedFactValidationIssue::Head)?;

    if derived.source_chain.is_empty() {
        return Err(DerivedFactValidationIssue::EmptySupportChain);
    }

    for source in &derived.source_chain {
        let fact = find_support_fact(source, support_facts).ok_or_else(|| {
            DerivedFactValidationIssue::MissingSupportSource {
                pack: source.pack.clone(),
                sample_id: source.sample_id.clone(),
            }
        })?;
        validate_fact(&fact.subject.root, fact.predicate, &fact.object.root).map_err(|issue| {
            DerivedFactValidationIssue::SupportingFactInvalid {
                pack: source.pack.clone(),
                sample_id: source.sample_id.clone(),
                issue,
            }
        })?;
    }

    let resolved_facts: Vec<&Fact> = derived
        .source_chain
        .iter()
        .filter_map(|source| find_support_fact(source, support_facts))
        .collect();
    if !support_pattern_matches(derived, &resolved_facts) {
        return Err(DerivedFactValidationIssue::SupportPatternMismatch {
            rule_id: derived.rule_id.clone(),
        });
    }

    Ok(())
}

pub fn predicate_requires_place_object(predicate: Predicate) -> bool {
    matches!(predicate, Predicate::LivesIn | Predicate::GoesTo)
}

pub fn is_place_like_root(root: &str) -> bool {
    let lower = root.trim().to_lowercase();
    geography_roots().contains(lower.as_str())
        || matches!(
            lower.as_str(),
            "қала"
                | "ауыл"
                | "кент"
                | "аудан"
                | "облыс"
                | "өңір"
                | "ел"
                | "мемлекет"
                | "өзен"
                | "көл"
                | "теңіз"
                | "тау"
                | "жота"
        )
}

pub fn is_time_like_root(root: &str) -> bool {
    matches!(
        root.trim().to_lowercase().as_str(),
        "күн"
            | "түн"
            | "таң"
            | "түс"
            | "кеш"
            | "таңертең"
            | "сәске"
            | "бесін"
            | "ымырт"
            | "көктем"
            | "жаз"
            | "күз"
            | "қыс"
            | "апта"
            | "ай"
            | "жыл"
            | "ғасыр"
            | "минут"
            | "сағат"
    )
}

pub fn rule_supports_predicate(rule_id: &str, predicate: Predicate) -> bool {
    matches!(
        (rule_id, predicate),
        ("R1_is_a_transitivity", Predicate::IsA)
            | ("R2_has_inheritance", Predicate::Has)
            | ("R3_has_inheritance_via_part_of", Predicate::Has)
            | ("R5_shared_is_a_target", Predicate::RelatedTo)
            | ("R6_lives_in_via_part_of", Predicate::LivesIn)
            | ("R7_goes_to_via_part_of", Predicate::GoesTo)
            | ("R8_after_transitivity", Predicate::After)
            | ("R9_part_of_transitivity", Predicate::PartOf)
            | ("R10_in_domain_inheritance", Predicate::InDomain)
            | ("R11_in_domain_shared_target", Predicate::RelatedTo)
    )
}

fn geography_roots() -> &'static BTreeSet<String> {
    static ROOTS: OnceLock<BTreeSet<String>> = OnceLock::new();
    ROOTS.get_or_init(build_geography_roots)
}

fn build_geography_roots() -> BTreeSet<String> {
    let mut roots = BTreeSet::new();
    let raw = include_str!("../../../data/world_core/geography_kz.jsonl");
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(entry) = serde_json::from_str::<GeoLine>(line) else {
            continue;
        };
        if entry.review_status.as_deref() == Some("rejected") {
            continue;
        }
        for fact in entry.facts {
            roots.insert(fact.subject.trim().to_lowercase());
        }
    }
    roots
}

pub fn find_support_fact<'a>(source: &FactSource, facts: &'a [Fact]) -> Option<&'a Fact> {
    facts.iter().find(|fact| fact.source == *source)
}

fn support_pattern_matches(derived: &crate::reasoner::DerivedFact, supports: &[&Fact]) -> bool {
    if supports.len() == 1 {
        return true;
    }
    let [first, second] = supports else {
        return false;
    };
    match derived.rule_id.as_str() {
        "R1_is_a_transitivity" => {
            first.predicate == Predicate::IsA
                && second.predicate == Predicate::IsA
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R2_has_inheritance" => {
            first.predicate == Predicate::IsA
                && second.predicate == Predicate::Has
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R3_has_inheritance_via_part_of" => {
            first.predicate == Predicate::Has
                && second.predicate == Predicate::PartOf
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R5_shared_is_a_target" => {
            first.predicate == Predicate::IsA
                && second.predicate == Predicate::IsA
                && first.object.root == second.object.root
                && canonical_pair(&first.subject.root, &second.subject.root)
                    == (derived.subject.root.clone(), derived.object.root.clone())
        }
        "R6_lives_in_via_part_of" => {
            first.predicate == Predicate::LivesIn
                && second.predicate == Predicate::PartOf
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R7_goes_to_via_part_of" => {
            first.predicate == Predicate::GoesTo
                && second.predicate == Predicate::PartOf
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R8_after_transitivity" => {
            first.predicate == Predicate::After
                && second.predicate == Predicate::After
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R9_part_of_transitivity" => {
            first.predicate == Predicate::PartOf
                && second.predicate == Predicate::PartOf
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R10_in_domain_inheritance" => {
            first.predicate == Predicate::IsA
                && second.predicate == Predicate::InDomain
                && first.object.root == second.subject.root
                && derived.subject.root == first.subject.root
                && derived.object.root == second.object.root
        }
        "R11_in_domain_shared_target" => {
            first.predicate == Predicate::InDomain
                && second.predicate == Predicate::InDomain
                && first.object.root == second.object.root
                && canonical_pair(&first.subject.root, &second.subject.root)
                    == (derived.subject.root.clone(), derived.object.root.clone())
        }
        _ => false,
    }
}

fn canonical_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct GeoLine {
    #[serde(default)]
    review_status: Option<String>,
    facts: Vec<GeoFact>,
}

#[derive(Debug, Deserialize)]
struct GeoFact {
    subject: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geography_catalog_marks_known_places() {
        assert!(is_place_like_root("алматы"));
        assert!(is_place_like_root("қостанай"));
        assert!(is_place_like_root("қала"));
        assert!(!is_place_like_root("ақын"));
    }

    #[test]
    fn after_requires_time_like_roots() {
        assert!(validate_fact("күз", Predicate::After, "жаз").is_ok());
        assert!(matches!(
            validate_fact("алматы", Predicate::After, "астана"),
            Err(OntologyIssue::TimeLikeRequired { .. })
        ));
    }

    #[test]
    fn place_predicates_require_place_object() {
        assert!(validate_fact("адам", Predicate::LivesIn, "алматы").is_ok());
        assert!(matches!(
            validate_fact("адам", Predicate::GoesTo, "ақын"),
            Err(OntologyIssue::PlaceObjectRequired { .. })
        ));
    }

    #[test]
    fn derived_fact_rule_must_match_predicate() {
        assert!(
            validate_derived_fact(
                "R6_lives_in_via_part_of",
                "адам",
                Predicate::LivesIn,
                "алматы"
            )
            .is_ok()
        );
        assert!(matches!(
            validate_derived_fact("R1_is_a_transitivity", "жер", Predicate::LivesIn, "алматы"),
            Err(OntologyIssue::RulePredicateMismatch { .. })
        ));
    }

    #[test]
    fn derived_fact_with_supports_requires_non_empty_chain() {
        let derived = crate::reasoner::DerivedFact {
            subject: crate::SlotRef {
                surface: "жер".into(),
                root: "жер".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: crate::SlotRef {
                surface: "ғаламшар".into(),
                root: "ғаламшар".into(),
                pos: "noun".into(),
            },
            rule_id: "R1_is_a_transitivity".into(),
            source_chain: vec![],
            confidence: crate::ConfidenceKind::RuleInferred,
        };
        assert!(matches!(
            validate_derived_fact_with_supports(&derived, &[]),
            Err(DerivedFactValidationIssue::EmptySupportChain)
        ));
    }

    #[test]
    fn derived_fact_with_supports_requires_resolvable_sources() {
        let derived = crate::reasoner::DerivedFact {
            subject: crate::SlotRef {
                surface: "жер".into(),
                root: "жер".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: crate::SlotRef {
                surface: "ғаламшар".into(),
                root: "ғаламшар".into(),
                pos: "noun".into(),
            },
            rule_id: "R1_is_a_transitivity".into(),
            source_chain: vec![FactSource {
                pack: "world_core/astronomy.jsonl".into(),
                sample_id: "missing".into(),
            }],
            confidence: crate::ConfidenceKind::RuleInferred,
        };
        assert!(matches!(
            validate_derived_fact_with_supports(&derived, &[]),
            Err(DerivedFactValidationIssue::MissingSupportSource { .. })
        ));
    }

    #[test]
    fn derived_fact_with_supports_rejects_invalid_supporting_fact() {
        let support = Fact {
            subject: crate::SlotRef {
                surface: "адам".into(),
                root: "адам".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::LivesIn,
            object: crate::SlotRef {
                surface: "ақын".into(),
                root: "ақын".into(),
                pos: "noun".into(),
            },
            pattern: "test".into(),
            source: FactSource {
                pack: "world_core/test.jsonl".into(),
                sample_id: "t1".into(),
            },
            confidence: crate::ConfidenceKind::HumanApproved,
            raw_text: "Адам ақында тұрады".into(),
        };
        let derived = crate::reasoner::DerivedFact {
            subject: crate::SlotRef {
                surface: "жер".into(),
                root: "жер".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: crate::SlotRef {
                surface: "ғаламшар".into(),
                root: "ғаламшар".into(),
                pos: "noun".into(),
            },
            rule_id: "R1_is_a_transitivity".into(),
            source_chain: vec![support.source.clone()],
            confidence: crate::ConfidenceKind::RuleInferred,
        };
        assert!(matches!(
            validate_derived_fact_with_supports(&derived, &[support]),
            Err(DerivedFactValidationIssue::SupportingFactInvalid { .. })
        ));
    }

    #[test]
    fn derived_fact_with_supports_rejects_wrong_rule_pattern() {
        let support_a = Fact {
            subject: crate::SlotRef {
                surface: "жер".into(),
                root: "жер".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: crate::SlotRef {
                surface: "ғаламшар".into(),
                root: "ғаламшар".into(),
                pos: "noun".into(),
            },
            pattern: "test".into(),
            source: FactSource {
                pack: "world_core/test.jsonl".into(),
                sample_id: "t1".into(),
            },
            confidence: crate::ConfidenceKind::HumanApproved,
            raw_text: "Жер — ғаламшар".into(),
        };
        let support_b = Fact {
            subject: crate::SlotRef {
                surface: "күн".into(),
                root: "күн".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: crate::SlotRef {
                surface: "жұлдыз".into(),
                root: "жұлдыз".into(),
                pos: "noun".into(),
            },
            pattern: "test".into(),
            source: FactSource {
                pack: "world_core/test.jsonl".into(),
                sample_id: "t2".into(),
            },
            confidence: crate::ConfidenceKind::HumanApproved,
            raw_text: "Күн — жұлдыз".into(),
        };
        let derived = crate::reasoner::DerivedFact {
            subject: crate::SlotRef {
                surface: "жер".into(),
                root: "жер".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: crate::SlotRef {
                surface: "жұлдыз".into(),
                root: "жұлдыз".into(),
                pos: "noun".into(),
            },
            rule_id: "R1_is_a_transitivity".into(),
            source_chain: vec![support_a.source.clone(), support_b.source.clone()],
            confidence: crate::ConfidenceKind::RuleInferred,
        };
        assert!(matches!(
            validate_derived_fact_with_supports(&derived, &[support_a, support_b]),
            Err(DerivedFactValidationIssue::SupportPatternMismatch { .. })
        ));
    }
}
