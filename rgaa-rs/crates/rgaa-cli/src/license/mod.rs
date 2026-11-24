//! Licence gating for `rgaa audit …` (#87).
//!
//! Local CLI runs check the *tier* only: the daily quota is not enforced here,
//! since a counter in a user-writable file is no limit at all (the gate is
//! built with `enforce_quota = false`). A refusal exits with
//! [`LICENSE_EXIT_CODE`] and a French message from [`messages`].

pub mod messages;

use chrono::{DateTime, Utc};
use rgaa_license::{Feature, LicenseGate, LicenseStatus};

use crate::commands::AuditCommand;
use crate::CliError;

/// Exit code of a command refused by the licence gate (missing tier, expired
/// or invalid licence). Distinct from 1 (policy), 2 (input) and 3 (execution)
/// so scripts can tell "upgrade needed" from "the audit failed".
pub const LICENSE_EXIT_CODE: i32 = 4;

/// Optional upgrade page quoted in refusal messages.
pub const UPGRADE_URL_ENV: &str = "RGAA_LICENSE_UPGRADE_URL";

/// Each command's gated feature. Every non-`analyze`/`igt` command is
/// remediation work: verifying, rendering, reviewing and policy-checking.
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

/// The production gate for local CLI use: licence from `RGAA_LICENSE_KEY` or
/// `license.toml`, tier gating only.
pub fn gate_from_env() -> LicenseGate {
    let upgrade_url = std::env::var(UPGRADE_URL_ENV)
        .ok()
        .filter(|u| !u.trim().is_empty());
    LicenseGate::from_env(false, upgrade_url)
}

/// Checks `feature` against `gate`. On admission returns the warning to print,
/// if any (offline grace); on refusal a [`CliError::License`].
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
