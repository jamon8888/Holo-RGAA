//! On-disk licence and usage state under `$XDG_CONFIG_HOME/rgaa/`
//! (falling back to `$HOME/.config/rgaa/`).
//!
//! Two files, so that the per-request usage write never rewrites the token:
//! - `license.toml`: the token and local activation baseline for offline grace;
//! - `usage.toml`: the current UTC day and the units consumed on it.
//!
//! Every function takes an explicit path. Only [`LicenseCache::default_dir`]
//! reads the environment, so tests never touch the real home directory or
//! mutate process-global env vars.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::LicenseError;

pub const LICENSE_FILE: &str = "license.toml";
pub const USAGE_FILE: &str = "usage.toml";

/// Overrides the token in `license.toml`; for CI and containers.
pub const LICENSE_KEY_ENV: &str = "RGAA_LICENSE_KEY";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseFile {
    pub token: String,
    /// Activation baseline for offline grace; local reactivation does not renew it.
    pub last_validated: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageFile {
    pub day: NaiveDate,
    pub count: u32,
}

pub struct LicenseCache;

impl LicenseCache {
    /// Pure path resolution, testable without touching the environment.
    /// Empty and relative `XDG_CONFIG_HOME` values are ignored, as the XDG spec requires.
    pub fn dir_from(xdg_config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
        let non_empty = |v: Option<OsString>| v.filter(|s| !s.is_empty());
        let base = non_empty(xdg_config_home)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| non_empty(home).map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("rgaa"))
    }

    /// `None` when neither variable is set (sandboxes, some CI): callers then
    /// run unlicensed rather than failing.
    pub fn default_dir() -> Option<PathBuf> {
        Self::dir_from(
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        )
    }

    /// `Ok(None)` when the file does not exist; an error when it exists but
    /// cannot be read or parsed. Never panics on bad content.
    pub fn load_license(path: &Path) -> Result<Option<LicenseFile>, LicenseError> {
        load_toml(path)
    }

    pub fn store_license(path: &Path, file: &LicenseFile) -> Result<(), LicenseError> {
        store_toml(path, file)
    }

    /// Removes the licence. Removing a licence that is not there is not an error.
    pub fn remove_license(path: &Path) -> Result<(), LicenseError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(LicenseError::Cache(format!("{}: {e}", path.display()))),
        }
    }

    pub fn load_usage(path: &Path) -> Result<Option<UsageFile>, LicenseError> {
        load_toml(path)
    }

    pub fn store_usage(path: &Path, file: &UsageFile) -> Result<(), LicenseError> {
        store_toml(path, file)
    }
}

fn load_toml<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, LicenseError> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(LicenseError::Cache(format!("{}: {e}", path.display()))),
    };
    // `message()` only: the full `Display` quotes the source line, which may be the token.
    toml::from_str(&text)
        .map(Some)
        .map_err(|e| LicenseError::Cache(format!("{}: {}", path.display(), e.message())))
}

/// Atomic write: a sibling temp file, created `0600` on unix (the token is a
/// credential), then renamed over the target.
fn store_toml<T: Serialize>(path: &Path, value: &T) -> Result<(), LicenseError> {
    let err = |e: &dyn std::fmt::Display| LicenseError::Cache(format!("{}: {e}", path.display()));
    let text = toml::to_string(value).map_err(|e| err(&e))?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| err(&e))?;
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| err(&e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|e| err(&e))?;
    }
    let f = tmp.as_file_mut();
    f.write_all(text.as_bytes()).map_err(|e| err(&e))?;
    f.sync_all().map_err(|e| err(&e))?;
    tmp.persist(path).map_err(|e| err(&e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn dir_prefers_xdg_then_home_and_ignores_empty() {
        let d = LicenseCache::dir_from(Some("/x".into()), Some("/h".into()));
        assert_eq!(d, Some(PathBuf::from("/x/rgaa")));
        let d = LicenseCache::dir_from(Some("".into()), Some("/h".into()));
        assert_eq!(d, Some(PathBuf::from("/h/.config/rgaa")));
        assert_eq!(LicenseCache::dir_from(None, None), None);
        let d = LicenseCache::dir_from(Some("local-config".into()), Some("/h".into()));
        assert_eq!(d, Some(PathBuf::from("/h/.config/rgaa")));
        assert_eq!(
            LicenseCache::dir_from(Some("".into()), Some("".into())),
            None
        );
    }

    #[test]
    fn license_round_trip_missing_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(LICENSE_FILE);
        assert_eq!(LicenseCache::load_license(&path).unwrap(), None);

        let file = LicenseFile {
            token: "v4.public.abc".into(),
            last_validated: Utc.with_ymd_and_hms(2026, 11, 19, 8, 0, 0).unwrap(),
        };
        LicenseCache::store_license(&path, &file).unwrap();
        assert_eq!(LicenseCache::load_license(&path).unwrap(), Some(file));

        LicenseCache::remove_license(&path).unwrap();
        LicenseCache::remove_license(&path).unwrap();
        assert_eq!(LicenseCache::load_license(&path).unwrap(), None);
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LICENSE_FILE);
        fs::write(&path, "token = [this is not toml").unwrap();
        assert!(matches!(
            LicenseCache::load_license(&path),
            Err(LicenseError::Cache(_))
        ));
    }

    #[test]
    fn decode_error_does_not_echo_the_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LICENSE_FILE);
        fs::write(&path, "token = \"v4.public.SECRET\" trailing\n").unwrap();
        let Err(LicenseError::Cache(msg)) = LicenseCache::load_license(&path) else {
            panic!("expected a cache error");
        };
        assert!(!msg.contains("SECRET"), "{msg}");
    }

    #[cfg(unix)]
    #[test]
    fn license_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LICENSE_FILE);
        let file = LicenseFile {
            token: "t".into(),
            last_validated: Utc::now(),
        };
        LicenseCache::store_license(&path, &file).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn usage_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(USAGE_FILE);
        let usage = UsageFile {
            day: NaiveDate::from_ymd_opt(2026, 11, 19).unwrap(),
            count: 3,
        };
        LicenseCache::store_usage(&path, &usage).unwrap();
        assert_eq!(LicenseCache::load_usage(&path).unwrap(), Some(usage));
    }
}
