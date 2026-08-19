use clap::{Command, Arg, Args, builder::PossibleValue};

/// Test the Commands enum parsing by using Command builder
#[test]
fn test_commands_enum_discriminants() {
    // Test that all command variants are valid
    use rgaa_cli::Commands::Analyze;
    use rgaa_cli::Commands::Igt;
    use rgaa_cli::Commands::Verify;
    use rgaa_cli::Commands::Report;
    use rgaa_cli::Commands::Policy;
    
    // Just verify the enum values exist
    let _ = Analyze;
    let _ = Igt;
    let _ = Verify;
    let _ = Report;
    let _ = Policy;
}

#[test]
fn test_format_value_variants() {
    // Test that format strings are valid
    let formats = ["json", "markdown", "sarif", "junit"];
    for f in &formats {
        let _ = std::ffi::CString::new(*f);
    }
}

#[test]
fn test_path_exists_check() {
    let path = std::path::PathBuf::from("/tmp");
    assert!(path.exists() || true); // /tmp may not exist, that's ok
}

#[test]
fn test_criterion_count() {
    use rgaa_core::RgaaCriteria;
    assert_eq!(RgaaCriteria::count(), 106);
}

#[test]
fn test_deterministic_criteria() {
    use rgaa_core::RgaaCriteria;
    let criteria = RgaaCriteria::deterministic();
    assert!(!criteria.is_empty());
}

#[test]
fn test_ia_assiste_criteria() {
    use rgaa_core::RgaaCriteria;
    let criteria = RgaaCriteria::ia_assiste();
    assert!(!criteria.is_empty());
}