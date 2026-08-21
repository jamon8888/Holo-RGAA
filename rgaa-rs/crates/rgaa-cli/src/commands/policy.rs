use rgaa_core::CriterionStatus;

use crate::commands::report::load_bundle;
use crate::commands::CommonArgs;
use crate::config::Config;
use crate::CliError;

/// CLI arguments for the policy command.
///
/// Checks if an audit bundle complies with the configured policy thresholds.
#[derive(Debug, clap::Args)]
pub struct PolicyArgs {
    /// Common CLI arguments shared across all commands.
    #[clap(flatten)]
    pub common: CommonArgs,
    /// Path to the audit bundle JSON file.
    #[clap(long, value_name = "BUNDLE")]
    pub input: std::path::PathBuf,
}

/// Executes the policy check command.
///
/// Loads the audit bundle, validates it, and checks compliance against
/// the configured policy thresholds (min_compliance, required_criteria).
///
/// # Returns
///
/// Exit code 0 if compliant, 1 if non-compliant.
///
/// # Errors
///
/// Returns `CliError` if configuration or bundle loading fails.
pub fn run(args: PolicyArgs) -> Result<i32, CliError> {
    let config = Config::load(args.common.config.as_deref())
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    let bundle = load_bundle(&args.input)?;
    bundle
        .validate()
        .map_err(|error| CliError::invalid_input(error.to_string()))?;

    let passed = bundle.summary.passed;
    let failed = bundle.summary.failed;
    let evaluated = passed + failed;
    let compliance = if evaluated == 0 {
        100.0
    } else {
        passed as f64 / evaluated as f64 * 100.0
    };

    let required_satisfied = config
        .policy
        .required_criteria
        .iter()
        .all(|required| criterion_passes(&bundle, required));

    let compliant = compliance >= config.policy.min_compliance
        && required_satisfied
        && bundle.summary.errors == 0;

    if compliant {
        println!(
            "compliance {compliance:.2}% (minimum {:.2}%): PASS",
            config.policy.min_compliance
        );
        Ok(0)
    } else {
        println!(
            "compliance {compliance:.2}% (minimum {:.2}%): FAIL",
            config.policy.min_compliance
        );
        Ok(1)
    }
}

fn criterion_passes(bundle: &rgaa_core::AuditBundle, criterion_id: &str) -> bool {
    bundle
        .pages
        .iter()
        .flat_map(|page| page.criteria.iter())
        .any(|criterion| {
            criterion.criterion_id == criterion_id && criterion.status == CriterionStatus::Pass
        })
}
