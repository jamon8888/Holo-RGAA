use std::path::PathBuf;

pub async fn audit(url: Option<String>, output: Option<PathBuf>) -> anyhow::Result<()> {
    let url = url.ok_or_else(|| anyhow::anyhow!("URL required: rgaa audit <URL>"))?;

    println!("Starting RGAA audit for {}", url);
    println!("(Browser CDP + agentic evaluation — this may take a few minutes)\n");

    let config = rgaa_core::CrawlConfig {
        max_pages: 10,
        max_depth: 3,
        respect_robots: true,
        sample_mode: false,
    };

    let orchestrator = rgaa_orchestrator::pipeline::Orchestrator::new();

    let result = orchestrator.run(&url, &config).await.map_err(|e| anyhow::anyhow!(e))?;

    if let Some(path) = output {
        crate::tui::export::export(&result, &path)
            .map_err(|e| anyhow::anyhow!("export failed: {e}"))?;
        println!("Exported to {}", path.display());
    } else {
        print_result_summary(&result);
    }

    Ok(())
}

pub async fn config_show() -> anyhow::Result<()> {
    let api_key = crate::keyring::get_api_key().ok();
    let base_url = crate::keyring::get_base_url()
        .unwrap_or_else(|| "https://api.hcompany.ai/v1/chat/completions".to_string());

    println!(
        "API Key: {}",
        if api_key.is_some() {
            "*** (set)"
        } else {
            "(not set)"
        }
    );
    println!("Base URL: {}", base_url);
    println!("Storage: ~/.rgaa/audits.db");

    Ok(())
}

pub async fn config_set_api_key(key: String) -> anyhow::Result<()> {
    crate::keyring::store_api_key(&key)?;
    println!("API key stored securely.");
    Ok(())
}

pub async fn config_set_base_url(url: String) -> anyhow::Result<()> {
    crate::keyring::store_base_url(&url)?;
    println!("Base URL saved.");
    Ok(())
}

pub async fn install() -> anyhow::Result<()> {
    crate::tui::run_install_wizard();
    Ok(())
}

pub async fn history(limit: usize) -> anyhow::Result<()> {
    let storage = crate::storage::storage()
        .await
        .map_err(|e| anyhow::anyhow!("failed to open database: {e}"))?;
    let audits = storage
        .list_audits(limit)
        .map_err(|e| anyhow::anyhow!("failed to list audits: {e}"))?;

    if audits.is_empty() {
        println!("No audits found. Run `rgaa audit <URL>` first.");
        return Ok(());
    }

    println!("{:<38} {:>8}  {}", "URL", "Score", "Date");
    println!("{}", "-".repeat(60));
    for audit in audits {
        println!(
            "{:<38} {:>7.1}%  {}",
            &audit.url[..audit.url.len().min(38)],
            audit.taux_global,
            audit.id.split('-').next().unwrap_or(&audit.id)
        );
    }

    Ok(())
}

fn print_result_summary(result: &rgaa_core::AuditResult) {
    let taux = result.taux_global;
    let label = if taux >= 80.0 {
        "Conforme"
    } else if taux >= 50.0 {
        "Partiellement conforme"
    } else {
        "Non conforme"
    };
    let color = if taux >= 80.0 { "32" } else if taux >= 50.0 { "33" } else { "31" };

    println!();
    println!("  ╔══════════════════════════════════════╗");
    println!("  ║         RGAA AUDIT RESULTS           ║");
    println!("  ╚══════════════════════════════════════╝");
    println!();
    println!("  URL:     {}", result.url);
    println!("  Score:   \x1b[{}m{:.1}%\x1b[0m ({})", color, taux, label);
    println!(
        "  Passed:  {}  Failed: {}  N/A: {}",
        result.passed, result.failed, result.na
    );
    println!();

    let ia_criteria = rgaa_core::RgaaCriteria::ia_assiste();
    let partial = rgaa_core::RgaaCriteria::partiellement_automatique();
    let det_count = rgaa_core::RgaaCriteria::all()
        .len()
        .saturating_sub(ia_criteria.len() + partial.len());
    println!("  {} deterministic criteria", det_count);
    println!(
        "  {} IA-Assistee criteria (agentic)",
        ia_criteria.len()
    );
    println!("  {} partially automatable criteria", partial.len());

    if let Some(page) = result.pages.first() {
        let fail_count = page
            .criteria
            .iter()
            .filter(|r| r.status == rgaa_core::CriterionStatus::Fail)
            .count();
        if fail_count > 0 {
            println!();
            println!("  Top failures:");
            for r in page
                .criteria
                .iter()
                .filter(|r| r.status == rgaa_core::CriterionStatus::Fail)
                .take(5)
            {
                println!("    [{}] {}", r.criterion_id, r.title);
            }
        }
    }

    println!();
    println!("  Full report: use --export report.html");
}
