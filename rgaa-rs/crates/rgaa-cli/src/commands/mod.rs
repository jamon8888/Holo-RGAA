#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    Analyze,
    Igt,
    Verify,
    Report,
    Policy,
}