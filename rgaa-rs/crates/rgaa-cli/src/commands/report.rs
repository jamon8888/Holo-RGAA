use clap::Parser;

#[derive(Debug, Parser)]
pub struct ReportCommand {
    #[clap(value_name = "AUDIT")]
    pub audit: std::path::PathBuf,
    #[clap(long)]
    pub format: Option<String>,
    #[clap(long)]
    pub output: Option<std::path::PathBuf>,
    #[clap(long)]
    pub audit_id: Option<String>,
}

pub async fn handle(_audit: std::path::PathBuf, _config: &crate::Config, _format: Option<String>, _output: Option<std::path::PathBuf>, _audit_id: Option<String>) -> Result<(), anyhow::Error> {
    Ok(())
}