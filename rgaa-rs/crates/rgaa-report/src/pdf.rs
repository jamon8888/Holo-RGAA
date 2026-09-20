//! PDF output through the piloted browser engine.
//!
//! The crate produces printable semantic HTML; conversion itself is one
//! headless Chromium call. No PDF library, no frozen WebKit: the browser
//! already driven for audits renders the tagged PDF.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::ReportError;

/// Renders `html` to `pdf` with headless Chromium.
///
/// Chromium takes the output path as the value of `--print-to-pdf`; the
/// HTML file stays the sole positional target. The binary path is explicit
/// so tests inject a missing one.
pub fn print_to_pdf_chromium(
    chromium_bin: &str,
    html: &Path,
    pdf: &Path,
) -> Result<(), ReportError> {
    let mut print_to_pdf = OsString::from("--print-to-pdf=");
    print_to_pdf.push(pdf);
    let output = Command::new(chromium_bin)
        .args(["--headless", "--disable-gpu", "--no-pdf-header-footer"])
        .arg(print_to_pdf)
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
