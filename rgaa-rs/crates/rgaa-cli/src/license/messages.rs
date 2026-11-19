//! User-facing licence messages, in French (#87).
//!
//! Every string a licence refusal or warning shows to a person lives here, so
//! the CLI, `rgaa licence status` and (later) the MCP transports say the same
//! thing. The module depends only on `rgaa_license` and `chrono`: it can move
//! verbatim into `rgaa-license` once the MCP crates need it (they cannot
//! depend on `rgaa-cli`, which itself depends on `rgaa-mcp-http`).
//!
//! The machine-readable side (`code`, HTTP status, JSON-RPC code) is
//! `rgaa_license::Denial`'s job; nothing here should be parsed.

use chrono::{DateTime, Utc};
use rgaa_license::{Denial, LicenseError, LicenseStatus, Tier};

/// The command that installs a licence key, quoted in every upgrade hint.
pub const ACTIVATE_HINT: &str = "`rgaa licence activate <clé>`";

/// Commercial tier name as shown to people (the offer names are not translated).
pub fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Free => "Free",
        Tier::Professional => "Professional",
        Tier::Team => "Team",
    }
}

/// Short status label for `rgaa licence status` and `rgaa config show`.
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

/// `2027-01-01 00:00 UTC`.
pub fn date(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// `3 h 12 min`, `45 min`, `moins d'une minute`. Rounded up to the minute,
/// so a wait is never under-announced.
pub fn duration(secs: u64) -> String {
    let minutes = secs.div_ceil(60);
    let (h, m) = (minutes / 60, minutes % 60);
    match (h, m) {
        (0, 0) => "moins d'une minute".to_string(),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

fn days(n: i64) -> String {
    if n <= 1 {
        format!("{n} jour")
    } else {
        format!("{n} jours")
    }
}

/// Why a token is unusable, in French.
pub fn invalid_reason(error: &LicenseError) -> String {
    match error {
        LicenseError::Malformed(detail) => {
            format!("la clé n'est pas un jeton de licence valide ({detail})")
        }
        LicenseError::InvalidSignature => {
            "la signature ne correspond à aucune clé de vérification connue".to_string()
        }
        LicenseError::NoVerificationKey => format!(
            "aucune clé de vérification n'est configurée (définissez {})",
            rgaa_license::token::PUBLIC_KEYS_ENV
        ),
        LicenseError::NotYetValid { at } => {
            format!("la licence n'est pas encore valide (début : {})", date(*at))
        }
        LicenseError::Expired { at } => format!("la licence a expiré le {}", date(*at)),
        LicenseError::GraceExceeded { since } => format!(
            "la licence n'a pas été vérifiée en ligne depuis le {}",
            date(*since)
        ),
        LicenseError::FeatureNotInTier { required, .. } => {
            format!("offre {} requise", tier_name(*required))
        }
        LicenseError::QuotaExceeded { limit, .. } => format!("quota journalier de {limit} atteint"),
        LicenseError::Cache(detail) => {
            format!("le fichier de licence est illisible ({detail})")
        }
        LicenseError::Sync(detail) => format!("synchronisation impossible ({detail})"),
    }
}

fn upgrade_tail(upgrade_url: Option<&str>) -> String {
    match upgrade_url {
        Some(url) => format!("Mettez à niveau : {url} — puis {ACTIVATE_HINT}."),
        None => format!("Mettez à niveau votre licence, puis {ACTIVATE_HINT}."),
    }
}

/// The message for a refused command.
pub fn denial(denial: &Denial, now: DateTime<Utc>) -> String {
    let url = denial.upgrade_url.as_deref();
    match &denial.error {
        LicenseError::FeatureNotInTier { tier, required, .. } => format!(
            "Fonctionnalité réservée à l'offre {} (offre actuelle : {}). {}",
            tier_name(*required),
            tier_name(*tier),
            upgrade_tail(url)
        ),
        LicenseError::Expired { at } => format!(
            "Licence expirée le {} : fonctionnalités réduites à l'offre Free. {}",
            date(*at),
            upgrade_tail(url)
        ),
        LicenseError::GraceExceeded { .. } => grace_exceeded(),
        LicenseError::QuotaExceeded { limit, reset_at } => {
            let used = denial
                .rate
                .map(|r| r.limit.saturating_sub(r.remaining))
                .unwrap_or(*limit);
            let wait = denial
                .retry_after_secs()
                .unwrap_or_else(|| (*reset_at - now).num_seconds().max(0) as u64);
            format!(
                "Quota journalier atteint ({used}/{limit}). Réessayez dans {}.",
                duration(wait)
            )
        }
        other => format!(
            "Licence invalide : {}. Réactivez-la avec {ACTIVATE_HINT}, ou supprimez-la avec \
             `rgaa licence deactivate` pour revenir à l'offre Free.",
            invalid_reason(other)
        ),
    }
}

/// Warning printed on an admitted command while offline within the grace period.
pub fn grace_warning(
    last_validated: DateTime<Utc>,
    until: DateTime<Utc>,
    now: DateTime<Utc>,
) -> String {
    let offline = (now - last_validated).num_days().max(0);
    let left_secs = (until - now).num_seconds().max(0);
    let left = match left_secs / 86_400 {
        0 => "moins d'un jour restant".to_string(),
        1 => "1 jour restant".to_string(),
        n => format!("{n} jours restants"),
    };
    format!(
        "Licence non vérifiée en ligne depuis {} — mode hors ligne, {left}.",
        days(offline)
    )
}

/// The grace period ran out: the installation runs as Free.
pub fn grace_exceeded() -> String {
    "Délai hors ligne expiré : fonctionnalités réduites à l'offre Free.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_round_up_to_the_minute() {
        assert_eq!(duration(0), "moins d'une minute");
        assert_eq!(duration(1), "1 min");
        assert_eq!(duration(60), "1 min");
        assert_eq!(duration(3 * 3600), "3 h");
        assert_eq!(duration(3 * 3600 + 12 * 60), "3 h 12 min");
        assert_eq!(duration(3 * 3600 + 11 * 60 + 1), "3 h 12 min");
    }

    #[test]
    fn grace_warning_counts_whole_days() {
        let t0 = DateTime::parse_from_rfc3339("2026-11-10T08:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let now = t0 + chrono::Duration::days(3);
        let until = t0 + chrono::Duration::days(7);
        assert_eq!(
            grace_warning(t0, until, now),
            "Licence non vérifiée en ligne depuis 3 jours — mode hors ligne, 4 jours restants."
        );
    }
}
