//! Rendering of non-conformities in the report, in priority order.
//!
//! Report order comes from ticket #431: impact first (bloquant → majeur →
//! mineur), then quick win. A machine impact proposal is shown as a
//! proposition and never as a decision (R3).

use std::fmt::Write;

use rgaa_core::{compare_non_conformities, AuditBundle, NonConformity};

/// Non-conformities of a bundle in report order: impact, then quick win, then
/// identifier for deterministic output.
#[must_use]
pub fn sorted_non_conformities(bundle: &AuditBundle) -> Vec<&NonConformity> {
    let mut items: Vec<&NonConformity> = bundle.non_conformities.iter().collect();
    items.sort_by(|left, right| compare_non_conformities(left, right));
    items
}

fn impact_cell(item: &NonConformity) -> String {
    if let Some(impact) = item.authoritative_impact() {
        impact.as_str().to_owned()
    } else if let Some(proposed) = item.proposed_user_impact {
        format!("proposition : {} (non validée)", proposed.as_str())
    } else {
        "non évalué".to_owned()
    }
}

fn quick_win_cell(item: &NonConformity) -> &'static str {
    if item.quick_win {
        "oui"
    } else {
        "non"
    }
}

/// Writes the Markdown "Non-conformités" section, or nothing when the audit
/// carries none.
pub fn write_markdown_non_conformities(out: &mut String, bundle: &AuditBundle) {
    let items = sorted_non_conformities(bundle);
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Non-conformités");
    let _ = writeln!(out);
    for item in items {
        let _ = writeln!(
            out,
            "- **{}** `{}` — {} ({}) — impact utilisateur : {}, quick win : {}",
            item.id,
            item.criterion_id,
            item.description,
            item.page_url,
            impact_cell(item),
            quick_win_cell(item),
        );
        if let Some(review) = &item.impact_review {
            let _ = writeln!(
                out,
                "  - Décision de revue : {}, {}",
                review.author, review.reviewed_at
            );
        }
    }
    let _ = writeln!(out);
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

/// Writes the HTML "Non-conformités" section, or nothing when the audit
/// carries none.
pub fn write_html_non_conformities(html: &mut String, bundle: &AuditBundle) {
    let items = sorted_non_conformities(bundle);
    if items.is_empty() {
        return;
    }
    let _ = writeln!(html, "        <h2>Non-conformités</h2>");
    let _ = writeln!(
        html,
        "        <table><thead><tr><th>Identifiant</th><th>Critère</th><th>Page</th><th>Description</th><th>Impact</th><th>Quick win</th></tr></thead><tbody>"
    );
    for item in items {
        let _ = writeln!(
            html,
            "        <tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&item.id),
            escape_html(&item.criterion_id),
            escape_html(&item.page_url),
            escape_html(&item.description),
            escape_html(&impact_cell(item)),
            quick_win_cell(item),
        );
    }
    let _ = writeln!(html, "        </tbody></table>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{AuditConfig, NonConformity, UserImpact};

    fn bundle_with(items: Vec<NonConformity>) -> AuditBundle {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.non_conformities = items;
        bundle
    }

    fn item(id: &str, impact: Option<UserImpact>, quick_win: bool) -> NonConformity {
        let mut item = NonConformity::new(id, "1.1", "https://example.test", "Défaut constaté");
        if let Some(impact) = impact {
            item.record_review(impact, quick_win, "auditrice", "2026-11-21T10:00:00Z")
                .expect("valid review");
        }
        item
    }

    #[test]
    fn report_orders_by_impact_then_quick_win() {
        let bundle = bundle_with(vec![
            item("minor", Some(UserImpact::Mineur), true),
            item("blocking", Some(UserImpact::Bloquant), false),
            item("major-quick", Some(UserImpact::Majeur), true),
            item("blocking-quick", Some(UserImpact::Bloquant), true),
        ]);
        let order: Vec<&str> = sorted_non_conformities(&bundle)
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(
            order,
            ["blocking-quick", "blocking", "major-quick", "minor"]
        );
    }

    #[test]
    fn markdown_shows_reviewed_impact_and_quick_win() {
        let bundle = bundle_with(vec![item("nc-1", Some(UserImpact::Bloquant), true)]);
        let mut out = String::new();
        write_markdown_non_conformities(&mut out, &bundle);

        assert!(out.contains("impact utilisateur : bloquant, quick win : oui"));
        assert!(out.contains("Décision de revue : auditrice, 2026-11-21T10:00:00Z"));
    }

    /// R3: a machine proposal is displayed as a proposition, not as the
    /// reviewer's decision.
    #[test]
    fn markdown_shows_machine_proposal_as_a_proposition() {
        let mut proposed = item("nc-2", None, false);
        proposed.set_proposed_impact(UserImpact::Majeur);
        let bundle = bundle_with(vec![proposed]);
        let mut out = String::new();
        write_markdown_non_conformities(&mut out, &bundle);

        assert!(out.contains("proposition : majeur (non validée)"));
        assert!(!out.contains("impact utilisateur : majeur"));
        assert!(!out.contains("Décision de revue"));
    }

    #[test]
    fn html_escapes_non_conformity_content() {
        let mut unsafe_item = item("nc-1", Some(UserImpact::Mineur), false);
        unsafe_item.description = "<script>alert('x')</script>".into();
        let bundle = bundle_with(vec![unsafe_item]);
        let mut html = String::new();
        write_html_non_conformities(&mut html, &bundle);

        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn empty_bundle_writes_no_section() {
        let bundle = bundle_with(Vec::new());
        let mut markdown = String::new();
        write_markdown_non_conformities(&mut markdown, &bundle);
        let mut html = String::new();
        write_html_non_conformities(&mut html, &bundle);
        assert!(markdown.is_empty());
        assert!(html.is_empty());
    }
}
