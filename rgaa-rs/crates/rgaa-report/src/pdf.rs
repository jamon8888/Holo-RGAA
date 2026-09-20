//! PDF output through the piloted browser engine.
//!
//! The crate produces printable semantic HTML; conversion itself is one
//! headless Chromium call. No PDF library, no frozen WebKit: the browser
//! already driven for audits renders the tagged PDF.

use std::path::Path;
use std::process::Command;

use crate::ReportError;

/// Renders `html` to `pdf` with headless Chromium (`--print-to-pdf`).
/// The binary path is explicit so tests inject a missing one.
pub fn print_to_pdf_chromium(
    chromium_bin: &str,
    html: &Path,
    pdf: &Path,
) -> Result<(), ReportError> {
    let output = Command::new(chromium_bin)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-pdf-header-footer",
            "--print-to-pdf",
        ])
        .arg(pdf)
        .arg(html)
        .output()
        .map_err(|error| {
            ReportError::execution(format!("impossible de lancer {chromium_bin} : {error}"))
        })?;
    if !output.status.success() {
        return Err(ReportError::execution(format!(
            "{} a échoué : {}",
            chromium_bin,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binaire_manquant_signale_clairement() {
        let error = print_to_pdf_chromium(
            "chromium-inexistant-xyz",
            Path::new("a.html"),
            Path::new("a.pdf"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("chromium-inexistant-xyz"));
    }
}
