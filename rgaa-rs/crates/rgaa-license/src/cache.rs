//! On-disk licence and usage state under `$XDG_CONFIG_HOME/rgaa/`
//! (falling back to `$HOME/.config/rgaa/`).
//!
//! Two files, so that the per-request usage write never rewrites the token:
//! - `license.toml`: the token and the time of the last successful online check;
//! - `usage.toml`: the current UTC day and the units consumed on it.
//!
//! Every function takes an explicit path. Only [`LicenseCache::default_dir`]
//! reads the environment, so tests never touch the real home directory or
//! mutate process-global env vars.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
    /// Last time the licence service confirmed this token. The offline grace
    /// period is counted from here.
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
    /// Empty values are ignored, as the XDG spec requires.
    pub fn dir_from(xdg_config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
        let non_empty = |v: Option<OsString>| v.filter(|s| !s.is_empty());
        let base = non_empty(xdg_config_home)
            .map(PathBuf::from)
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
    toml::from_str(&text)
        .map(Some)
        .map_err(|e| LicenseError::Cache(format!("{}: {e}", path.display())))
}

/// Sequence number that makes every temp file name unique within this process.
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Atomic write: a sibling temp file, created `0600` on unix (the token is a
/// credential), then renamed over the target.
///
/// The temp name is unique per write (pid + sequence) and opened with
/// `create_new`, so an existing file is never reused: a stale file left by an
/// earlier run, possibly with broader permissions and an old token, cannot be
/// truncated and renamed into place. The mode is also set explicitly after
/// open, so the result does not depend on the umask. A legacy fixed-name
/// `<path>.tmp` from an earlier version is removed.
fn store_toml<T: Serialize>(path: &Path, value: &T) -> Result<(), LicenseError> {
    let err = |e: &dyn std::fmt::Display| LicenseError::Cache(format!("{}: {e}", path.display()));
    let text = toml::to_string(value).map_err(|e| err(&e))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| err(&e))?;
    }
    let mut legacy_name = path.as_os_str().to_owned();
    legacy_name.push(".tmp");
    match fs::remove_file(PathBuf::from(legacy_name)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(err(&e)),
    }

    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(format!(".{}.{seq}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp_name);

    let written = write_private_tmp(&tmp, text.as_bytes());
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(err(&e));
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        err(&e)
    })
}

/// Creates `tmp` exclusively (never an existing file), makes it `0600`, writes
/// and syncs it.
fn write_private_tmp(tmp: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        let mut f = options.open(tmp)?;
        f.set_permissions(fs::Permissions::from_mode(0o600))?;
        f.write_all(bytes)?;
        f.sync_all()
    }
    #[cfg(not(unix))]
    {
        let mut f = options.open(tmp)?;
        f.write_all(bytes)?;
        f.sync_all()
    }
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
        assert!(!dir.path().join("license.toml.tmp").exists());
    }

    /// A stale `license.toml.tmp` with broad permissions (left by an earlier run)
    /// must not receive the token: the write goes to a fresh private file and the
    /// stale one is removed.
    #[cfg(unix)]
    #[test]
    fn stale_broad_tmp_file_never_receives_the_token() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LICENSE_FILE);
        let stale = dir.path().join("license.toml.tmp");
        fs::write(&stale, "old = 1").unwrap();
        fs::set_permissions(&stale, fs::Permissions::from_mode(0o644)).unwrap();

        let file = LicenseFile {
            token: "v4.public.secret".into(),
            last_validated: Utc::now(),
        };
        LicenseCache::store_license(&path, &file).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert!(!stale.exists(), "legacy tmp file must be removed");
        assert_eq!(LicenseCache::load_license(&path).unwrap(), Some(file));
    }

    /// Unique temp names: after a write, the directory holds only the target.
    #[test]
    fn writes_leave_no_temp_files_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(USAGE_FILE);
        for count in 0..3 {
            let usage = UsageFile {
                day: NaiveDate::from_ymd_opt(2026, 11, 19).unwrap(),
                count,
            };
            LicenseCache::store_usage(&path, &usage).unwrap();
        }
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec![USAGE_FILE.to_string()]);
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
