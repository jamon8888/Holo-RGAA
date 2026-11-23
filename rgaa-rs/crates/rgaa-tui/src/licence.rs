//! `rgaa licence activate | status | deactivate` (#87).
//!
//! Every function that decides or renders something takes its paths, clock
//! time and gate as arguments, so tests run against a tempdir and a
//! generated key without touching `HOME`, `XDG_CONFIG_HOME` or any other
//! process-global environment. Only the `cmd_*` wrappers read the
//! environment. User-facing copy is French, from
//! [`rgaa_cli::license::messages`].

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use chrono::{DateTime, Utc};
use rgaa_cli::license::messages as fr;
use rgaa_license::cache::{LICENSE_FILE, LICENSE_KEY_ENV};
use rgaa_license::gate::LICENSE_SERVER_ENV;
use rgaa_license::{
    LicenseCache, LicenseClaims, LicenseError, LicenseFile, LicenseGate, LicenseSnapshot,
    LicenseStatus,
};

/// Where the licence in effect comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenceSource {
    /// `RGAA_LICENSE_KEY` is set: it overrides `license.toml`.
    Env,
    /// `license.toml` at this path (it may not exist).
    File(PathBuf),
    /// Neither `XDG_CONFIG_HOME` nor `HOME` is set.
    NoConfigDir,
}

/// `license.toml` path from an explicit config dir (see
/// [`LicenseCache::dir_from`] for the environment-free resolution).
pub fn licence_path(config_dir: &Path) -> PathBuf {
    config_dir.join(LICENSE_FILE)
}

/// Pure: which source the gate will use, given the env token and the path.
pub fn source_from(env_token: Option<&str>, path: Option<PathBuf>) -> LicenceSource {
    match (env_token.filter(|t| !t.trim().is_empty()), path) {
        (Some(_), _) => LicenceSource::Env,
        (None, Some(p)) => LicenceSource::File(p),
        (None, None) => LicenceSource::NoConfigDir,
    }
}

/// The token from the command line, or from a file (trimmed).
pub fn read_token(token: Option<String>, file: Option<&Path>) -> anyhow::Result<String> {
    let raw = match (token, file) {
        (Some(t), None) => t,
        (None, Some(p)) => std::fs::read_to_string(p)
            .with_context(|| format!("Lecture de la clé impossible : {}", p.display()))?,
        (Some(_), Some(_)) => bail!("Indiquez la clé ou --file, pas les deux."),
        (None, None) => bail!("Clé manquante : rgaa licence activate <clé> | --file <chemin>"),
    };
    let token = raw.trim().to_string();
    if token.is_empty() {
        bail!("La clé de licence est vide.");
    }
    Ok(token)
}

/// Verifies `token` and, only if it is usable, writes it to `path` (mode
/// 0600). Re-activating the same token preserves its grace-period baseline.
pub fn activate(
    gate: &LicenseGate,
    token: &str,
    path: &Path,
    now: DateTime<Utc>,
) -> anyhow::Result<LicenseClaims> {
    let claims = gate.verify(token).map_err(|e| {
        anyhow!(
            "Clé refusée : {}. Aucun fichier n'a été écrit.",
            fr::invalid_reason(&e)
        )
    })?;
    if now >= claims.exp {
        bail!(
            "Clé refusée : la licence a expiré le {}. Aucun fichier n'a été écrit.",
            fr::date(claims.exp)
        );
    }
    if let Some(at) = claims.nbf.filter(|at| now < *at) {
        bail!(
            "Clé refusée : {}. Aucun fichier n'a été écrit.",
            fr::invalid_reason(&LicenseError::NotYetValid { at })
        );
    }
    // An unreadable file must not pass for "no baseline": that would reset grace offline.
    let previous = LicenseCache::load_license(path).map_err(|e| {
        anyhow!(
            "Licence existante illisible : {}. Aucun fichier n'a été écrit.",
            fr::invalid_reason(&e)
        )
    })?;
    let last_validated = activation_baseline(token, previous, now);
    let file = LicenseFile {
        token: token.to_string(),
        last_validated,
    };
    LicenseCache::store_license(path, &file).map_err(|e| {
        anyhow!(
            "Écriture de la licence impossible : {}",
            fr::invalid_reason(&e)
        )
    })?;
    Ok(claims)
}

/// The confirmation printed after a successful activation.
pub fn activation_message(claims: &LicenseClaims, path: &Path, source: &LicenceSource) -> String {
    let mut out = format!(
        "Licence activée : offre {}, valable jusqu'au {}.\nEnregistrée dans {}.",
        fr::tier_name(claims.tier),
        fr::date(claims.exp),
        path.display()
    );
    if *source == LicenceSource::Env {
        out.push_str(&format!(
            "\nAttention : {LICENSE_KEY_ENV} est défini et a priorité sur ce fichier."
        ));
    }
    out
}

/// Removes `license.toml`. Removing an absent licence is not an error.
pub fn deactivate(path: &Path, env_set: bool) -> anyhow::Result<String> {
    let existed = path.exists();
    LicenseCache::remove_license(path).map_err(|e| anyhow!("{}", fr::invalid_reason(&e)))?;
    let mut out = if existed {
        format!(
            "Licence supprimée ({}). Offre en vigueur : Free.",
            path.display()
        )
    } else {
        format!("Aucune licence à supprimer ({}).", path.display())
    };
    if env_set {
        out.push_str(&format!(
            "\nAttention : {LICENSE_KEY_ENV} est encore défini ; retirez-le pour revenir à l'offre Free."
        ));
    }
    Ok(out)
}

/// The full `rgaa licence status` report.
pub fn render_status(
    snap: &LicenseSnapshot,
    now: DateTime<Utc>,
    source: &LicenceSource,
    grace_enabled: bool,
) -> String {
    let mut lines = vec![
        format!("Offre :      {}", fr::tier_name(snap.tier)),
        format!("Statut :     {}", fr::status_label(&snap.status)),
    ];
    match &snap.status {
        LicenseStatus::Unlicensed => {
            lines.push(format!(
                "            Activez une licence avec {}.",
                fr::ACTIVATE_HINT
            ));
        }
        LicenseStatus::Grace { until } => {
            let last = snap.last_validated.unwrap_or(now);
            lines.push(format!(
                "            {}",
                fr::grace_warning(last, *until, now)
            ));
        }
        LicenseStatus::GraceExceeded { .. } => {
            lines.push(format!("            {}", fr::grace_exceeded()));
        }
        LicenseStatus::Expired { at } => {
            lines.push(format!(
                "            Expirée le {} : fonctionnalités réduites à l'offre Free.",
                fr::date(*at)
            ));
        }
        LicenseStatus::Invalid(e) => {
            lines.push(format!("            Raison : {}.", fr::invalid_reason(e)));
        }
        LicenseStatus::Valid => {}
    }
    if let Some(claims) = &snap.claims {
        lines.push(format!("Titulaire :  {}", claims.sub));
        lines.push(format!("Expiration : {}", fr::date(claims.exp)));
    }
    let baseline = snap
        .last_validated
        .map(fr::date)
        .unwrap_or_else(|| "jamais".to_string());
    let grace_note = if grace_enabled {
        String::new()
    } else {
        format!(" (grâce désactivée : {LICENSE_SERVER_ENV} non défini)")
    };
    lines.push(format!(
        "Référence de la période de grâce : {baseline}{grace_note}"
    ));

    let rate = &snap.rate;
    let used = rate.limit.saturating_sub(rate.remaining);
    let wait = rate.retry_after_secs(now);
    lines.push(format!(
        "Quota :      {used}/{} unités aujourd'hui ; réinitialisation le {} (dans {})",
        rate.limit,
        fr::date(rate.reset_at),
        fr::duration(wait)
    ));
    lines.push(
        "            Non décompté en local (CLI, stdio) : seuls les serveurs HTTP l'appliquent."
            .to_string(),
    );
    lines.push(format!(
        "Source :     {}",
        match source {
            LicenceSource::Env => format!("variable {LICENSE_KEY_ENV}"),
            LicenceSource::File(p) => p.display().to_string(),
            LicenceSource::NoConfigDir =>
                "aucun répertoire de configuration (HOME non défini)".into(),
        }
    ));
    lines.join("\n")
}

/// One-line summary for `rgaa config show`.
pub fn config_row(snap: &LicenseSnapshot) -> String {
    let mut row = format!(
        "Licence: {} — {}",
        fr::tier_name(snap.tier),
        fr::status_label(&snap.status)
    );
    if let Some(claims) = &snap.claims {
        row.push_str(&format!(" (expiration : {})", fr::date(claims.exp)));
    }
    row
}

fn env_token() -> Option<String> {
    std::env::var(LICENSE_KEY_ENV)
        .ok()
        .filter(|t| !t.trim().is_empty())
}

fn activation_baseline(
    token: &str,
    previous: Option<LicenseFile>,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    previous
        .filter(|file| file.token == token)
        .map_or(now, |file| file.last_validated)
}

fn default_path() -> anyhow::Result<PathBuf> {
    LicenseCache::default_dir()
        .map(|d| licence_path(&d))
        .ok_or_else(|| {
            anyhow!("Ni XDG_CONFIG_HOME ni HOME n'est défini : impossible de situer license.toml.")
        })
}

pub fn cmd_activate(token: Option<String>, file: Option<PathBuf>) -> anyhow::Result<()> {
    let token = read_token(token, file.as_deref())?;
    let path = default_path()?;
    let gate = rgaa_cli::license::gate_from_env();
    let claims = activate(&gate, &token, &path, Utc::now())?;
    let source = source_from(env_token().as_deref(), Some(path.clone()));
    println!("{}", activation_message(&claims, &path, &source));
    Ok(())
}

pub fn cmd_status() -> anyhow::Result<()> {
    let gate = rgaa_cli::license::gate_from_env();
    let source = source_from(
        env_token().as_deref(),
        LicenseCache::default_dir().map(|d| licence_path(&d)),
    );
    let grace_enabled = gate.config().grace.is_some();
    println!(
        "{}",
        render_status(&gate.snapshot(), Utc::now(), &source, grace_enabled)
    );
    Ok(())
}

pub fn cmd_deactivate() -> anyhow::Result<()> {
    let path = default_path()?;
    println!("{}", deactivate(&path, env_token().is_some())?);
    Ok(())
}

/// The `config show` row from the environment.
pub fn config_row_from_env() -> String {
    config_row(&rgaa_cli::license::gate_from_env().snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn reactivating_the_same_token_keeps_its_grace_baseline() {
        let saved_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let reactivated_at = Utc.with_ymd_and_hms(2026, 1, 9, 0, 0, 0).unwrap();
        let file = LicenseFile {
            token: "same-token".into(),
            last_validated: saved_at,
        };
        assert_eq!(
            activation_baseline("same-token", Some(file), reactivated_at),
            saved_at
        );
    }
}
