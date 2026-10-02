//! Lint profiles: which framework a finding is reported against.
//!
//! The four rule families this crate implements are defects under all three
//! frameworks, so a profile does **not** change what is detected. What it
//! changes is which references a finding cites, and that is not cosmetic: an
//! auditor filing an RGAA 4.1 report needs the RGAA criterion number, a team
//! working to WCAG 2.1 AA needs the success criterion, and a Section 508 filing
//! needs the WCAG 2.0 subset the Revised 508 standard incorporates — citing a
//! WCAG 2.1 addition there would cite a requirement Section 508 does not make.
//!
//! Criterion numbers and their WCAG references are read from the `rgaa-core`
//! catalog, never retyped here, so a catalog correction reaches this crate.

use rgaa_core::RgaaCriteria;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::rules::RuleId;

/// The reference framework a lint run reports against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub enum Profile {
    /// RGAA 4.1.2, the French public-sector framework.
    #[default]
    #[serde(rename = "rgaa-4.1", alias = "rgaa-4.1.2", alias = "rgaa")]
    Rgaa41,
    /// WCAG 2.1 Level AA.
    #[serde(rename = "wcag-2.1-aa", alias = "wcag", alias = "wcag-2.1")]
    Wcag21Aa,
    /// Revised Section 508 (36 CFR 1194 Appendix A), which incorporates WCAG 2.0
    /// Level AA by reference at E205.4.
    #[serde(rename = "section-508", alias = "508", alias = "section508")]
    Section508,
}

impl Profile {
    /// Every profile, for tests and for listing them to an agent.
    pub const ALL: [Profile; 3] = [Profile::Rgaa41, Profile::Wcag21Aa, Profile::Section508];

    /// The identifier accepted in `lint-rules.toml` and in the MCP request.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rgaa41 => "rgaa-4.1",
            Self::Wcag21Aa => "wcag-2.1-aa",
            Self::Section508 => "section-508",
        }
    }

    /// References to cite for a rule under this profile.
    ///
    /// Empty is impossible for the four shipped rules and is asserted in tests:
    /// a finding with no reference is a finding no one can act on.
    pub fn references(self, rule: RuleId) -> Vec<Reference> {
        let criterion_id = rule.rgaa_criterion();
        let Some(criterion) = RgaaCriteria::find(criterion_id) else {
            // Unreachable while `rgaa_criterion()` returns a catalog id, which a
            // test pins. Degrading to an empty list rather than panicking keeps a
            // catalog edit from taking the MCP server down.
            return Vec::new();
        };
        let wcag: Vec<&str> = criterion
            .wcag_refs
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        match self {
            Self::Rgaa41 => vec![Reference {
                framework: "RGAA 4.1.2".into(),
                id: criterion.id.to_string(),
                detail: Some(criterion.title.clone()),
            }],
            Self::Wcag21Aa => wcag
                .iter()
                .map(|sc| Reference {
                    framework: "WCAG 2.1 AA".into(),
                    id: (*sc).to_string(),
                    detail: None,
                })
                .collect(),
            Self::Section508 => {
                let mut refs = vec![Reference {
                    framework: "Section 508 (Revised)".into(),
                    id: "E205.4".into(),
                    detail: Some("Incorporates WCAG 2.0 Level A and AA by reference".into()),
                }];
                refs.extend(
                    wcag.iter()
                        .filter(|sc| !WCAG_21_ADDITIONS.contains(sc))
                        .map(|sc| Reference {
                            framework: "WCAG 2.0 AA".into(),
                            id: (*sc).to_string(),
                            detail: None,
                        }),
                );
                refs
            }
        }
    }
}

/// Success criteria WCAG 2.1 added over WCAG 2.0.
///
/// Section 508 incorporates WCAG 2.0, so these must not appear in a Section 508
/// citation even when the RGAA catalog lists them for the same criterion.
const WCAG_21_ADDITIONS: &[&str] = &[
    "1.3.4", "1.3.5", "1.3.6", "1.4.10", "1.4.11", "1.4.12", "1.4.13", "2.1.4", "2.2.6", "2.5.1",
    "2.5.2", "2.5.3", "2.5.4", "2.5.5", "2.5.6", "4.1.3",
];

/// One citation attached to a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Reference {
    /// e.g. `"RGAA 4.1.2"`, `"WCAG 2.1 AA"`.
    pub framework: String,
    /// The criterion or success-criterion number.
    pub id: String,
    /// Human-readable title, when the catalog carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
