//! User impact and quick-win metadata for non-conformities.
//!
//! This mirrors the `NotCompliantItem` of DISIC/Ara: one non-conformity carries
//! a user impact (`mineur` / `majeur` / `bloquant`) and a quick-win flag, and
//! several of them can exist for one criterion on one page. The impact and the
//! quick-win flag are a human *review decision* (R5); a machine may only
//! *propose* an impact, which is displayed as a proposition and never used as
//! an authoritative impact (R3).

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::RgaaError;

/// User impact of a non-conformity, from the least to the most severe.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum UserImpact {
    /// Minor impact: the user is inconvenienced but can complete the task.
    Mineur,
    /// Major impact: the task is difficult or requires a workaround.
    Majeur,
    /// Blocking impact: the task cannot be completed at all.
    Bloquant,
}

impl UserImpact {
    /// French label used in reports and in the glossary.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mineur => "mineur",
            Self::Majeur => "majeur",
            Self::Bloquant => "bloquant",
        }
    }
}

impl std::fmt::Display for UserImpact {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Identity and timestamp of the human who set the impact and quick-win flag.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactReview {
    /// Authenticated account of the reviewer.
    pub author: String,
    /// RFC 3339 timestamp of the decision.
    pub reviewed_at: String,
}

/// One non-conformity, as in Ara's `NotCompliantItem`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NonConformity {
    pub id: String,
    pub criterion_id: String,
    pub page_url: String,
    pub description: String,
    /// Authoritative impact, set only by a human review (R5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_impact: Option<UserImpact>,
    /// Authoritative quick-win flag, set only by a human review (R5).
    #[serde(default)]
    pub quick_win: bool,
    /// Machine proposal for the impact. Displayed as a proposition (R3); it
    /// never populates [`Self::user_impact`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_user_impact: Option<UserImpact>,
    /// Human decision that set `user_impact` and `quick_win` (R5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_review: Option<ImpactReview>,
}

impl NonConformity {
    /// Creates a non-conformity with no impact and no quick-win flag yet.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        criterion_id: impl Into<String>,
        page_url: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            criterion_id: criterion_id.into(),
            page_url: page_url.into(),
            description: description.into(),
            user_impact: None,
            quick_win: false,
            proposed_user_impact: None,
            impact_review: None,
        }
    }

    /// Records a machine impact proposal. Stored as a proposal only (R3).
    pub fn set_proposed_impact(&mut self, impact: UserImpact) {
        self.proposed_user_impact = Some(impact);
    }

    /// Impact hint for ordering a *review queue* while the decision is still
    /// pending. A proposal is never presented as a decision (R3). Published
    /// report and remediation order use [`Self::authoritative_impact`] only.
    #[must_use]
    pub fn review_queue_impact(&self) -> Option<UserImpact> {
        self.user_impact.or(self.proposed_user_impact)
    }

    /// Authoritative impact: only a human review may set it (R3/R5).
    #[must_use]
    pub fn authoritative_impact(&self) -> Option<UserImpact> {
        self.user_impact
    }

    /// Records a human review setting the impact and the quick-win flag.
    ///
    /// Requires a non-empty authenticated reviewer and a timestamp (R5).
    ///
    /// # Errors
    /// Returns [`RgaaError::MissingField`] when the author or the timestamp is
    /// empty, so an unattributed decision can never be persisted.
    pub fn record_review(
        &mut self,
        impact: UserImpact,
        quick_win: bool,
        author: &str,
        reviewed_at: &str,
    ) -> Result<(), RgaaError> {
        let author = author.trim();
        if author.is_empty() {
            return Err(RgaaError::MissingField("impact review author".into()));
        }
        let reviewed_at = reviewed_at.trim();
        if reviewed_at.is_empty() {
            return Err(RgaaError::MissingField("impact review timestamp".into()));
        }
        self.user_impact = Some(impact);
        self.quick_win = quick_win;
        self.impact_review = Some(ImpactReview {
            author: author.to_owned(),
            reviewed_at: reviewed_at.to_owned(),
        });
        Ok(())
    }
}

/// Severity rank used by report and remediation priority ordering: bloquant
/// (3) → majeur (2) → mineur (1) → none (0).
#[must_use]
pub fn impact_rank(impact: Option<UserImpact>) -> u8 {
    match impact {
        Some(UserImpact::Bloquant) => 3,
        Some(UserImpact::Majeur) => 2,
        Some(UserImpact::Mineur) => 1,
        None => 0,
    }
}

/// Priority comparator for published artifacts: reviewed impact (bloquant →
/// majeur → mineur → none), then quick win first, then id for determinism.
/// Machine proposals never drive this order (R3).
#[must_use]
pub fn compare_non_conformities(left: &NonConformity, right: &NonConformity) -> Ordering {
    impact_rank(right.authoritative_impact())
        .cmp(&impact_rank(left.authoritative_impact()))
        .then_with(|| right.quick_win.cmp(&left.quick_win))
        .then_with(|| left.id.cmp(&right.id))
}

/// Sorts non-conformities by priority, in place.
pub fn sort_non_conformities(items: &mut [NonConformity]) {
    items.sort_by(compare_non_conformities);
}

/// Machine impact proposals, keyed by canonical RGAA criterion id. This is
/// data shown to the reviewer as a proposition (R3), never a decision. The
/// `12.9` keyboard-trap proposal comes from ticket #431.
static IMPACT_PROPOSALS: &[(&str, UserImpact)] = &[("12.9", UserImpact::Bloquant)];

/// Machine proposal of an impact for a criterion, when one is known.
#[must_use]
pub fn propose_impact_for_criterion(criterion_id: &str) -> Option<UserImpact> {
    IMPACT_PROPOSALS
        .iter()
        .find(|(id, _)| *id == criterion_id)
        .map(|(_, impact)| *impact)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, impact: Option<UserImpact>, quick_win: bool) -> NonConformity {
        let mut non_conformity = NonConformity::new(id, "1.1", "https://example.test", "desc");
        if let Some(impact) = impact {
            non_conformity
                .record_review(impact, quick_win, "auditrice", "2026-11-21T10:00:00Z")
                .expect("valid review");
        }
        non_conformity
    }

    #[test]
    fn user_impact_serializes_with_french_labels() {
        assert_eq!(
            serde_json::to_string(&UserImpact::Mineur).expect("serialize"),
            "\"mineur\""
        );
        assert_eq!(
            serde_json::to_string(&UserImpact::Majeur).expect("serialize"),
            "\"majeur\""
        );
        assert_eq!(
            serde_json::to_string(&UserImpact::Bloquant).expect("serialize"),
            "\"bloquant\""
        );
    }

    /// R3: a machine proposal must never become the authoritative impact.
    #[test]
    fn machine_proposal_is_not_an_authoritative_impact() {
        let mut non_conformity =
            NonConformity::new("nc-1", "12.9", "https://example.test", "piège");
        non_conformity.set_proposed_impact(UserImpact::Bloquant);

        assert_eq!(non_conformity.authoritative_impact(), None);
        assert_eq!(
            non_conformity.review_queue_impact(),
            Some(UserImpact::Bloquant)
        );
        assert!(non_conformity.impact_review.is_none());
    }

    /// R5: an unattributed decision is refused.
    #[test]
    fn review_requires_an_authenticated_author_and_a_timestamp() {
        let mut non_conformity = NonConformity::new("nc-1", "1.1", "https://example.test", "desc");
        assert!(non_conformity
            .record_review(UserImpact::Majeur, false, "  ", "2026-11-21T10:00:00Z")
            .is_err());
        assert!(non_conformity
            .record_review(UserImpact::Majeur, false, "auditrice", "")
            .is_err());
        assert!(non_conformity.user_impact.is_none());
    }

    #[test]
    fn review_sets_impact_quick_win_and_reviewer_identity() {
        let mut non_conformity = NonConformity::new("nc-1", "1.1", "https://example.test", "desc");
        non_conformity
            .record_review(
                UserImpact::Majeur,
                true,
                "auditrice",
                "2026-11-21T10:00:00Z",
            )
            .expect("valid review");

        assert_eq!(
            non_conformity.authoritative_impact(),
            Some(UserImpact::Majeur)
        );
        assert!(non_conformity.quick_win);
        let review = non_conformity
            .impact_review
            .as_ref()
            .expect("review recorded");
        assert_eq!(review.author, "auditrice");
        assert_eq!(review.reviewed_at, "2026-11-21T10:00:00Z");
    }

    /// Ordering comes from ticket #431: impact first, quick win second.
    #[test]
    fn priority_sort_orders_by_impact_then_quick_win() {
        let mut items = vec![
            item("minor", Some(UserImpact::Mineur), true),
            item("blocking-no", Some(UserImpact::Bloquant), false),
            item("none", None, true),
            item("major-quick", Some(UserImpact::Majeur), true),
            item("blocking-quick", Some(UserImpact::Bloquant), true),
            item("major-no", Some(UserImpact::Majeur), false),
        ];
        sort_non_conformities(&mut items);

        let order: Vec<&str> = items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            order,
            [
                "blocking-quick",
                "blocking-no",
                "major-quick",
                "major-no",
                "minor",
                "none",
            ]
        );
    }

    #[test]
    fn priority_sort_is_deterministic_on_equal_keys() {
        let mut items = vec![
            item("z", Some(UserImpact::Majeur), true),
            item("a", Some(UserImpact::Majeur), true),
            item("m", Some(UserImpact::Majeur), true),
        ];
        sort_non_conformities(&mut items);
        let order: Vec<&str> = items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(order, ["a", "m", "z"]);
    }

    #[test]
    fn proposal_for_keyboard_trap_is_bloquant() {
        assert_eq!(
            propose_impact_for_criterion("12.9"),
            Some(UserImpact::Bloquant)
        );
        assert_eq!(propose_impact_for_criterion("1.1"), None);
    }

    #[test]
    fn non_conformity_round_trips_and_defaults_on_deserialize() {
        let raw = r#"{"id":"nc-1","criterion_id":"1.1","page_url":"https://example.test","description":"desc"}"#;
        let decoded: NonConformity = serde_json::from_str(raw).expect("old JSON still loads");
        assert_eq!(decoded.user_impact, None);
        assert!(!decoded.quick_win);
        assert_eq!(decoded.proposed_user_impact, None);
        assert_eq!(decoded.impact_review, None);
    }
}
