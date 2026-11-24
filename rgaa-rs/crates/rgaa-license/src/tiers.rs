use std::fmt;

use serde::{Deserialize, Serialize};

/// Commercial tier carried by a licence. An installation without a licence
/// runs as [`Tier::Free`].
///
/// Deserialisation is strict: an unknown tier name in a token is rejected
/// rather than guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Free,
    Professional,
    Team,
}

/// A gated capability. Every route, MCP tool and CLI command that is not
/// exempt maps to exactly one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    StaticLint,
    FullAudit,
    Remediation,
    Igt,
    Batch,
}

impl Feature {
    pub const ALL: [Feature; 5] = [
        Feature::StaticLint,
        Feature::FullAudit,
        Feature::Remediation,
        Feature::Igt,
        Feature::Batch,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Feature::StaticLint => "static_lint",
            Feature::FullAudit => "full_audit",
            Feature::Remediation => "remediation",
            Feature::Igt => "igt",
            Feature::Batch => "batch",
        }
    }

    /// The cheapest tier that includes this feature.
    pub fn minimum_tier(self) -> Tier {
        match self {
            Feature::StaticLint => Tier::Free,
            Feature::FullAudit | Feature::Remediation | Feature::Igt | Feature::Batch => {
                Tier::Professional
            }
        }
    }
}

impl Tier {
    pub const ALL: [Tier; 3] = [Tier::Free, Tier::Professional, Tier::Team];

    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Free => "free",
            Tier::Professional => "professional",
            Tier::Team => "team",
        }
    }

    /// Metered units per UTC day.
    pub fn daily_quota(self) -> u32 {
        match self {
            Tier::Free => 10,
            Tier::Professional => 500,
            Tier::Team => 5000,
        }
    }

    pub fn allows(self, feature: Feature) -> bool {
        self >= feature.minimum_tier()
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exhaustive tier × feature matrix. Adding a `Feature` or a `Tier`
    /// without a row here fails the length assertion.
    #[test]
    fn tier_feature_matrix() {
        use Feature::*;
        use Tier::*;
        let expected: &[(Tier, Feature, bool)] = &[
            (Free, StaticLint, true),
            (Free, FullAudit, false),
            (Free, Remediation, false),
            (Free, Igt, false),
            (Free, Batch, false),
            (Professional, StaticLint, true),
            (Professional, FullAudit, true),
            (Professional, Remediation, true),
            (Professional, Igt, true),
            (Professional, Batch, true),
            (Team, StaticLint, true),
            (Team, FullAudit, true),
            (Team, Remediation, true),
            (Team, Igt, true),
            (Team, Batch, true),
        ];
        assert_eq!(expected.len(), Tier::ALL.len() * Feature::ALL.len());
        for &(tier, feature, allowed) in expected {
            assert_eq!(tier.allows(feature), allowed, "{tier} × {feature}");
        }
    }

    #[test]
    fn daily_quotas_match_issue_87() {
        assert_eq!(Tier::Free.daily_quota(), 10);
        assert_eq!(Tier::Professional.daily_quota(), 500);
        assert_eq!(Tier::Team.daily_quota(), 5000);
    }

    #[test]
    fn unknown_tier_is_rejected() {
        assert!(serde_json::from_str::<Tier>("\"enterprise\"").is_err());
        assert_eq!(
            serde_json::from_str::<Tier>("\"team\"").ok(),
            Some(Tier::Team)
        );
    }
}
