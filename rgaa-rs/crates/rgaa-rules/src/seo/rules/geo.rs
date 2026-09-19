use super::schema_org::{has_prop, has_type};
use super::{violation, Ctx, Outcome};
use crate::seo::catalog::SeoRule;
use serde_json::Value;

const LEAD_MIN_CHARS: usize = 200;
const ARTICLE_TYPES: &[&str] = &["Article", "NewsArticle", "BlogPosting"];

fn entities<'a>(ctx: &'a Ctx<'_>) -> impl Iterator<Item = &'a Value> {
    ctx.nodes.iter().filter(|n| {
        has_type(n, "Organization") || has_type(n, "WebSite") || has_type(n, "LocalBusiness")
    })
}

fn articles<'a>(ctx: &'a Ctx<'_>) -> impl Iterator<Item = &'a Value> {
    ctx.nodes
        .iter()
        .filter(|n| ARTICLE_TYPES.iter().any(|t| has_type(n, t)))
}

pub(super) fn entity_declared(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let ok = entities(ctx).any(|n| has_prop(n, "name") && has_prop(n, "url"));
    Outcome::fail_if(
        !ok,
        rule,
        "aucune entité Organization/WebSite avec name et url",
        1,
    )
}

pub(super) fn entity_same_as(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let mut any = false;
    let mut with_same_as = false;
    for n in entities(ctx) {
        any = true;
        with_same_as |= has_prop(n, "sameAs");
    }
    if !any {
        return Outcome::NotApplicable;
    }
    Outcome::fail_if(!with_same_as, rule, "entité sans liens sameAs", 1)
}

pub(super) fn article_author_and_date(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let mut violations = Vec::new();
    let mut any = false;
    for n in articles(ctx) {
        any = true;
        let missing: Vec<&str> = ["author", "datePublished"]
            .into_iter()
            .filter(|k| !has_prop(n, k))
            .collect();
        if !missing.is_empty() {
            violations.push(violation(
                rule,
                format!("Article sans {}", missing.join(", ")),
                1,
            ));
        }
    }
    if !any {
        Outcome::NotApplicable
    } else if violations.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(violations)
    }
}

pub(super) fn article_date_modified(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let mut any = false;
    let mut missing = 0usize;
    for n in articles(ctx) {
        any = true;
        if !has_prop(n, "dateModified") {
            missing += 1;
        }
    }
    if !any {
        return Outcome::NotApplicable;
    }
    Outcome::fail_if(
        missing > 0,
        rule,
        format!("{missing} Article sans dateModified"),
        missing,
    )
}

pub(super) fn lead_paragraph(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if !ctx.snapshot.headings.iter().any(|h| h.level == 1) {
        return Outcome::NotApplicable;
    }
    let n = ctx
        .snapshot
        .lead_paragraph
        .as_deref()
        .map_or(0, |p| p.trim().chars().count());
    Outcome::fail_if(
        n < LEAD_MIN_CHARS,
        rule,
        format!("paragraphe d'accroche de {n} caractères après le <h1>"),
        1,
    )
}

#[cfg(test)]
mod tests {
    use crate::seo::rules::test_support::*;
    use crate::seo::snapshot::{Heading, PageSnapshot};

    fn with(json_ld: &[&str]) -> PageSnapshot {
        PageSnapshot {
            json_ld: json_ld.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn entity_rules() {
        assert!(is_fail(&run("GEO-01", &PageSnapshot::default())));
        assert!(is_na(&run("GEO-02", &PageSnapshot::default())));
        let s = with(&[
            r#"{"@context":"https://schema.org","@type":"Organization","name":"A","url":"u"}"#,
        ]);
        assert!(is_pass(&run("GEO-01", &s)));
        assert!(is_fail(&run("GEO-02", &s)));
        let s = with(&[
            r#"{"@context":"https://schema.org","@type":"Organization","name":"A","url":"u","sameAs":["https://wikidata.org/x"]}"#,
        ]);
        assert!(is_pass(&run("GEO-02", &s)));
    }

    #[test]
    fn article_rules() {
        assert!(is_na(&run("GEO-03", &PageSnapshot::default())));
        assert!(is_na(&run("GEO-04", &PageSnapshot::default())));
        let s = with(&[
            r#"{"@context":"https://schema.org","@type":"BlogPosting","headline":"h","author":{"@type":"Person","name":"p"},"datePublished":"2026-01-01"}"#,
        ]);
        assert!(is_pass(&run("GEO-03", &s)));
        assert!(is_fail(&run("GEO-04", &s)));
        let s = with(&[
            r#"{"@context":"https://schema.org","@type":"Article","headline":"h","dateModified":"2026-01-02"}"#,
        ]);
        assert!(is_fail(&run("GEO-03", &s)));
        assert!(is_pass(&run("GEO-04", &s)));
    }

    #[test]
    fn lead_paragraph_rule() {
        assert!(is_na(&run("GEO-05", &PageSnapshot::default())));
        let h1 = Heading {
            level: 1,
            text: "Titre".into(),
            next_paragraph: None,
        };
        let short = PageSnapshot {
            headings: vec![h1.clone()],
            lead_paragraph: Some("Trop court.".into()),
            ..Default::default()
        };
        assert!(is_fail(&run("GEO-05", &short)));
        let long = PageSnapshot {
            headings: vec![h1],
            lead_paragraph: Some("x".repeat(200)),
            ..Default::default()
        };
        assert!(is_pass(&run("GEO-05", &long)));
    }
}
