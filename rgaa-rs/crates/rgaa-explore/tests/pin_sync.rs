//! Keep the duplicated version pins in sync without depending on the browser
//! substrate crate (which would pull the whole Obscura tree into a dev tool).

use rgaa_explore::pin::{AGENT_BROWSER_PINNED_VERSION, OBSCURA_PINNED_VERSION};

const OBSCURA_LIB: &str = include_str!("../../rgaa-obscura/src/lib.rs");
const INSTALL_SH: &str = include_str!("../../../../install.sh");
const SUBSTRATE_DOC: &str = include_str!("../../../../docs/obscura-substrate.md");

#[test]
fn obscura_pin_matches_the_browser_substrate() {
    assert_eq!(OBSCURA_PINNED_VERSION, "obscura 0.2.4");
    assert!(
        OBSCURA_LIB.contains("const OBSCURA_PINNED_VERSION: &str = \"obscura 0.2.4\";"),
        "rgaa-obscura pin drifted from rgaa-explore"
    );
}

#[test]
fn obscura_pin_matches_the_installer_and_docs() {
    assert!(
        INSTALL_SH.contains("OBSCURA_VERSION=\"0.2.4\""),
        "install.sh pin drifted"
    );
    assert!(
        SUBSTRATE_DOC.contains("obscura 0.2.4"),
        "substrate doc pin drifted"
    );
}

#[test]
fn agent_browser_pin_is_a_single_exact_version() {
    assert_eq!(AGENT_BROWSER_PINNED_VERSION, "0.45.0");
    assert_eq!(
        rgaa_explore::pin::AGENT_BROWSER_PACKAGE,
        format!("agent-browser@{AGENT_BROWSER_PINNED_VERSION}")
    );
}
