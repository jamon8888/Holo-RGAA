use rgaa_obscura::{GuidedTest, ObscuraBridge};

use crate::commands::{write_output, CommonArgs};
use crate::config::Config;
use crate::CliError;

#[derive(Debug, clap::Args)]
pub struct IgtArgs {
    #[clap(flatten)]
    pub common: CommonArgs,
    #[clap(long, value_name = "TEST")]
    pub test: String,
}

pub async fn run(args: IgtArgs) -> Result<i32, CliError> {
    let config = Config::load(args.common.config.as_deref())
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    if !config.guided_tests.iter().any(|id| id == &args.test) {
        return Err(CliError::invalid_input(format!(
            "unknown guided test '{}'",
            args.test
        )));
    }

    let test = GuidedTest {
        id: args.test.clone(),
        version: 1,
        preconditions: Vec::new(),
        steps: Vec::new(),
        criterion_mapping: Vec::new(),
        evidence_requirements: Vec::new(),
    };

    let mut bridge = ObscuraBridge::new();
    bridge
        .start_server()
        .await
        .map_err(|error| CliError::execution(format!("browser unavailable: {error}")))?;
    let result = bridge
        .run_guided_test(&test)
        .await
        .map_err(|error| CliError::execution(error.to_string()))?;
    let rendered = serde_json::to_string_pretty(&result)
        .map_err(|error| CliError::execution(error.to_string()))?;
    write_output(&args.common.output, &rendered)?;
    Ok(0)
}
