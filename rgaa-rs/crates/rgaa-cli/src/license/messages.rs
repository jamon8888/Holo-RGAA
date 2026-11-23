//! User-facing French copy for licence outcomes.

use chrono::{DateTime, Utc};
use rgaa_license::{Denial, LicenseError, LicenseStatus, Tier};

/// Recovery path for a licence key. The CLI has no `licence` subcommand: the only
/// key it reads is `RGAA_LICENSE_KEY`, via `LicenseGate::from_env`.
pub const ACTIVATE_HINT: &str = "`RGAA_LICENSE_KEY=<clé>` (variable d'environnement)";

fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Free => "Free",
        Tier::Professional => "Professional",
        Tier::Team => "Team",
    }
}

fn date(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn duration(secs: u64) -> String {
    let minutes = secs.div_ceil(60);
    match (minutes / 60, minutes % 60) {
        (0, 0) => "moins d'une minute".to_string(),
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

fn upgrade_tail(upgrade_url: Option<&str>) -> String {
    match upgrade_url {
        Some(url) => format!("Mettez à niveau : {url} — définissez {ACTIVATE_HINT}, puis relancez la commande."),
        None => format!("Mettez à niveau votre licence : définissez {ACTIVATE_HINT}, puis relancez la commande."),
    }
}

fn invalid_license_message() -> String {
    format!(
        "Licence invalide. Définissez {ACTIVATE_HINT} pour la remplacer, ou supprimez-la pour revenir à l'offre Free."
    )
}

/// Formats a refusal without exposing token contents or verifier details.
pub fn denial(denial: &Denial, now: DateTime<Utc>) -> String {
    let upgrade_url = denial.upgrade_url.as_deref();
    match &denial.error {
        LicenseError::FeatureNotInTier { tier, required, .. } => format!(
            "Fonctionnalité réservée à l'offre {} (offre actuelle : {}). {}",
            tier_name(*required),
            tier_name(*tier),
            upgrade_tail(upgrade_url)
        ),
        LicenseError::Expired { at } => format!(
            "Licence expirée le {} : fonctionnalités réduites à l'offre Free. {}",
            date(*at),
            upgrade_tail(upgrade_url)
        ),
        LicenseError::GraceExceeded { .. } => {
            "Délai hors ligne expiré : fonctionnalités réduites à l'offre Free.".to_string()
        }
        LicenseError::QuotaExceeded { limit, reset_at } => {
            let wait = denial
                .retry_after_secs()
                .unwrap_or_else(|| (*reset_at - now).num_seconds().max(0) as u64);
            format!(
                "Quota journalier de {limit} atteint. Réessayez dans {}.",
                duration(wait)
            )
        }
        LicenseError::Malformed(_)
        | LicenseError::InvalidSignature
        | LicenseError::NoVerificationKey
        | LicenseError::NotYetValid { .. }
        | LicenseError::Cache(_)
        | LicenseError::Sync(_) => invalid_license_message(),
    }
}

/// Warns without refusing during the allowed offline-grace window.
pub fn grace_warning(
    last_validated: DateTime<Utc>,
    until: DateTime<Utc>,
    now: DateTime<Utc>,
) -> String {
    let offline_days = (now - last_validated).num_days().max(0);
    let remaining_days = ((until - now).num_seconds().max(0) / 86_400).max(1);
    format!(
        "Licence non vérifiée en ligne depuis {offline_days} jours — mode hors ligne, {remaining_days} jours restants."
    )
}

/// Human-readable status for future status/config views.
pub fn status_label(status: &LicenseStatus) -> &'static str {
    match status {
        LicenseStatus::Unlicensed => "aucune licence (offre Free)",
        LicenseStatus::Valid => "valide",
        LicenseStatus::Grace { .. } => "hors ligne (période de grâce)",
        LicenseStatus::Expired { .. } => "expirée",
        LicenseStatus::GraceExceeded { .. } => "délai hors ligne expiré",
        LicenseStatus::Invalid(_) => "invalide",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A refusal used to advertise `rgaa licence activate`, which the binary does
    /// not implement: following it left the user stuck. Every message that offers
    /// a recovery path must now quote the environment variable actually read.
    #[test]
    fn activation_hint_points_to_a_supported_recovery_path() {
        let messages = [
            upgrade_tail(None),
            upgrade_tail(Some("https://example.test/upgrade")),
            invalid_license_message(),
        ];
        for message in messages {
            assert!(
                message.contains("RGAA_LICENSE_KEY"),
                "le message doit citer la variable lue par la CLI : {message}"
            );
            assert!(
                !message.contains("licence activate"),
                "le message ne doit pas proposer une commande inexistante : {message}"
            );
        }
    }
}
