//! Licence gating for `rgaa audit …`.

pub mod messages;

use chrono::{DateTime, Utc};
use rgaa_license::{Feature, LicenseGate, LicenseStatus};

use crate::commands::AuditCommand;
use crate::CliError;

/// Exit code for a command refused by the licence gate.
pub const LICENSE_EXIT_CODE: i32 = 4;

/// Optional upgrade page quoted in refusal messages.
const UPGRADE_URL_ENV: &str = "RGAA_LICENSE_UPGRADE_URL";

/// Each command's gated feature.
pub fn feature_for(command: &AuditCommand) -> Feature {
    match command {
        AuditCommand::Analyze(_) => Feature::FullAudit,
        AuditCommand::Igt(_) => Feature::Igt,
        AuditCommand::Verify(_)
        | AuditCommand::Report(_)
        | AuditCommand::Review(_)
        | AuditCommand::Policy(_) => Feature::Remediation,
    }
}

/// Builds the local tier-only gate from the installed licence.
pub fn gate_from_env() -> LicenseGate {
    let upgrade_url = std::env::var(UPGRADE_URL_ENV)
        .ok()
        .filter(|url| !url.trim().is_empty());
    LicenseGate::from_env(false, upgrade_url)
}

/// Checks a command feature, returning an offline-grace warning when needed.
pub fn admit(
    gate: &LicenseGate,
    feature: Feature,
    now: DateTime<Utc>,
) -> Result<Option<String>, CliError> {
    match gate.check(feature, 1) {
        Ok(admission) => Ok(match admission.status {
            LicenseStatus::Grace { until } => {
                let last = gate.snapshot().last_validated.unwrap_or(now);
                Some(messages::grace_warning(last, until, now))
            }
            _ => None,
        }),
        Err(denial) => Err(CliError::License(messages::denial(&denial, now))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rgaa_license::{GateConfig, LicenseVerifier, SystemClock, UsageCounter};

    use super::*;

    /// The CLI's own gate shape: tier gating only, no quota.
    fn unlicensed_gate() -> LicenseGate {
        LicenseGate::new(
            LicenseVerifier::new(vec![]),
            Arc::new(UsageCounter::in_memory()),
            Arc::new(SystemClock),
            GateConfig {
                enforce_quota: false,
                ..GateConfig::default()
            },
        )
    }

    #[test]
    fn an_unlicensed_cli_is_refused_paid_features_with_the_licence_exit_code() {
        let gate = unlicensed_gate();
        for feature in [Feature::FullAudit, Feature::Igt, Feature::Remediation] {
            let err =
                admit(&gate, feature, Utc::now()).expect_err("Free must not run a paid feature");
            assert!(matches!(err, CliError::License(_)), "{feature:?}: {err:?}");
        }
    }

    #[test]
    fn an_unlicensed_cli_still_runs_free_features() {
        let gate = unlicensed_gate();
        assert_eq!(
            admit(&gate, Feature::StaticLint, Utc::now()).ok(),
            Some(None)
        );
    }
}
