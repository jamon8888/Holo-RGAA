//! Active offer/mode reporting (`rgaa status` and the `rgaa audit` header).
//!
//! The mode is derived from [`ModeStatus`] in `rgaa-core`, itself resolved
//! from the existing LLM provider configuration. This is read-only: no tool
//! writes a review status here (PRD R7).

use rgaa_core::provider::ModeStatus;

use crate::CliError;

/// Human-readable, deterministic status block.
#[must_use]
pub fn report(status: &ModeStatus) -> String {
    format!(
        "Mode : {}\nIA : {}\nFournisseur : {}\nHôte : {}\nRaison : {}",
        status.mode.display_name(),
        status.mode.as_str(),
        status.provider.unwrap_or("—"),
        status.host.as_deref().unwrap_or("—"),
        status.reason,
    )
}

/// `rgaa status` handler.
///
/// # Errors
/// Returns [`CliError::Execution`] when the mode or route cannot be resolved
/// — for example a Souverain route pointing at an agent publisher.
pub fn run() -> Result<i32, CliError> {
    match ModeStatus::from_env() {
        Ok(status) => {
            println!("{}", report(&status));
            Ok(0)
        }
        Err(error) => Err(CliError::execution(error.to_string())),
    }
}

/// Best-effort header printed at the start of every `rgaa audit` command.
///
/// A resolution error is reported here but not swallowed by the audit: the
/// audit's own configuration resolution fails closed afterwards.
pub fn print_audit_header() {
    match ModeStatus::from_env() {
        Ok(status) => eprintln!("{}", report(&status)),
        Err(error) => eprintln!("Mode : indéterminé — {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::provider::LlmMode;

    #[test]
    fn status_report_names_the_offer_and_its_reason() {
        let status = ModeStatus {
            mode: LlmMode::Off,
            provider: None,
            host: None,
            reason: "aucun fournisseur LLM configuré".into(),
        };
        let text = report(&status);
        assert!(text.contains("Mode : Souverain"), "{text}");
        assert!(text.contains("IA : off"), "{text}");
        assert!(text.contains("Fournisseur : —"), "{text}");
        assert!(text.contains("Hôte : —"), "{text}");
        assert!(text.contains("aucun fournisseur LLM configuré"), "{text}");
    }

    #[test]
    fn managed_reports_confort_and_gateway_host() {
        let status = ModeStatus {
            mode: LlmMode::Managed,
            provider: Some("mistral"),
            host: Some("gateway.rgaa.example".into()),
            reason: "passerelle managed".into(),
        };
        let text = report(&status);
        assert!(text.contains("Mode : Confort"), "{text}");
        assert!(text.contains("IA : managed"), "{text}");
        assert!(text.contains("Fournisseur : mistral"), "{text}");
        assert!(text.contains("Hôte : gateway.rgaa.example"), "{text}");
    }

    #[test]
    fn byok_is_displayed_as_souverain() {
        let status = ModeStatus {
            mode: LlmMode::Byok,
            provider: Some("mistral"),
            host: Some("api.mistral.ai".into()),
            reason: "clé du client".into(),
        };
        let text = report(&status);
        assert!(text.starts_with("Mode : Souverain\nIA : byok\n"), "{text}");
    }
}
