use clap::Parser;

#[derive(Debug, Parser)]
pub struct PolicyCommand {
    #[clap(value_name = "PROJECT")]
    pub project: std::path::PathBuf,
    #[clap(long)]
    pub format: Option<String>,
    #[clap(long)]
    pub output: Option<std::path::PathBuf>,
    #[clap(long)]
    pub audit_id: Option<String>,
}

pub async fn handle(_project: std::path::PathBuf, _config: &crate::Config, _format: Option<String>, _output: Option<std::path::PathBuf>, _audit_id: Option<String>) -> Result<(), anyhow::Error> {
    Ok(())
}