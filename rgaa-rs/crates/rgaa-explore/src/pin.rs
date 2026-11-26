//! Version pins for the exploratory substrate.
//!
//! These are duplicated from `install.sh` and `rgaa-obscura/src/lib.rs` on
//! purpose: this dev-only crate must not pull in the browser substrate as a
//! dependency. `tests/pin_sync.rs` keeps the duplicates honest.

/// Pinned Obscura binary version, including the program name token.
pub const OBSCURA_PINNED_VERSION: &str = "obscura 0.2.4";

/// Pinned agent-browser CLI version.
pub const AGENT_BROWSER_PINNED_VERSION: &str = "0.45.0";

/// Exact npm package spec used for every agent-browser invocation.
pub const AGENT_BROWSER_PACKAGE: &str = "agent-browser@0.45.0";

/// Compare the first two whitespace tokens exactly.
///
/// A prefix match would accept `obscura 0.2.40` as `0.2.4`, so this mirrors
/// the gate in `install.sh` and `ObscuraBridge::start_server`.
#[must_use]
pub fn obscura_version_matches(output: &str) -> bool {
    let mut reported = output.split_whitespace();
    let mut pinned = OBSCURA_PINNED_VERSION.split_whitespace();
    (reported.next(), reported.next()) == (pinned.next(), pinned.next())
}

/// Accept only `agent-browser <pinned version>` from `--version`.
#[must_use]
pub fn agent_browser_version_matches(output: &str) -> bool {
    let mut reported = output.split_whitespace();
    match (reported.next(), reported.next()) {
        (Some(name), Some(version)) => {
            name == "agent-browser" && version == AGENT_BROWSER_PINNED_VERSION
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obscura_pin_is_exact() {
        assert!(obscura_version_matches("obscura 0.2.4"));
        assert!(obscura_version_matches("obscura 0.2.4 extra tokens"));
        assert!(!obscura_version_matches("obscura 0.2.40"));
        assert!(!obscura_version_matches("stealth 0.2.4"));
        assert!(!obscura_version_matches(""));
    }

    #[test]
    fn cli_pin_is_exact() {
        assert!(agent_browser_version_matches("agent-browser 0.45.0"));
        assert!(!agent_browser_version_matches("agent-browser 0.46.0"));
        assert!(!agent_browser_version_matches("chrome 0.45.0"));
        assert!(!agent_browser_version_matches("agent-browser"));
    }
}
