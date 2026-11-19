//! Licence gating for `rgaa audit …`.

pub mod messages;

use chrono::{DateTime, Utc};
use rgaa_license::{Feature, LicenseGate, LicenseStatus};

use crate::commands::AuditCommand;
use crate::CliError;

/// Exit code for a command refused by the licence gate.
pub const LICENSE_EXIT_CODE: i32 = 4;

/// Optional upgrade page quoted in refusal messages.
pub const UPGRADE_URL_ENV: &str = "RGAA_LICENSE_UPGRADE_URL";

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
