//! User-facing French copy for licence outcomes.

use chrono::{DateTime, Utc};
use rgaa_license::{Denial, LicenseError, LicenseStatus, Tier};

/// Command used to install a replacement licence key.
pub const ACTIVATE_HINT: &str = "`rgaa licence activate <clé>`";

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
        Some(url) => format!("Mettez à niveau : {url} — puis {ACTIVATE_HINT}."),
        None => format!("Mettez à niveau votre licence, puis {ACTIVATE_HINT}."),
    }
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
            format!("Quota journalier de {limit} atteint. Réessayez dans {}.", duration(wait))
        }
        LicenseError::Malformed(_)
        | LicenseError::InvalidSignature
        | LicenseError::NoVerificationKey
        | LicenseError::NotYetValid { .. }
        | LicenseError::Cache(_)
        | LicenseError::Sync(_) => format!(
            "Licence invalide. Réactivez-la avec {ACTIVATE_HINT}, ou supprimez-la pour revenir à l'offre Free."
        ),
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
