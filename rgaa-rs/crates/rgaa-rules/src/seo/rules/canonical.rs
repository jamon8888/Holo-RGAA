use super::{violation, Ctx, Outcome};
use crate::seo::catalog::SeoRule;

/// `scheme://host[:port]` of an absolute URL, lower-cased. `None` for relative or malformed URLs.
fn origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next()?;
    if host.is_empty() {
        return None;
    }
    Some(format!(
        "{}://{}",
        scheme.to_ascii_lowercase(),
        host.to_ascii_lowercase()
    ))
}

fn normalize_url(url: &str) -> String {
    let no_fragment = url.split('#').next().unwrap_or(url);
    no_fragment.trim_end_matches('/').to_ascii_lowercase()
}

pub(super) fn present(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    Outcome::fail_if(
        ctx.snapshot.canonicals.is_empty(),
        rule,
        "aucun link rel=canonical",
        1,
    )
}

pub(super) fn same_origin(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if ctx.snapshot.canonicals.is_empty() {
        return Outcome::NotApplicable;
    }
    let Some(page_origin) = origin(&ctx.snapshot.url) else {
        return Outcome::NotApplicable;
    };
    let foreign: Vec<&String> = ctx
        .snapshot
        .canonicals
        .iter()
        .filter(|c| origin(c).is_some_and(|o| o != page_origin))
        .collect();
    match foreign.first() {
        None => Outcome::Pass,
        Some(first) => Outcome::Fail(vec![violation(
            rule,
            format!("canonical vers une autre origine : {first}"),
            foreign.len(),
        )]),
    }
}

pub(super) fn single(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let n = ctx.snapshot.canonicals.len();
    if n == 0 {
        return Outcome::NotApplicable;
    }
    let mut distinct: Vec<String> = ctx
        .snapshot
        .canonicals
        .iter()
        .map(|c| normalize_url(c))
        .collect();
    distinct.sort();
    distinct.dedup();
    Outcome::fail_if(
        distinct.len() > 1,
        rule,
        format!(
            "{n} balises canonical, {} valeurs distinctes",
            distinct.len()
        ),
        n,
    )
}

pub(super) fn hreflang_self_reference(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if ctx.snapshot.hreflangs.is_empty() {
        return Outcome::NotApplicable;
    }
    let page = normalize_url(&ctx.snapshot.url);
    let canonical = ctx.snapshot.canonicals.first().map(|c| normalize_url(c));
    let self_ref = ctx.snapshot.hreflangs.iter().any(|h| {
        let href = normalize_url(&h.href);
        href == page || canonical.as_deref() == Some(href.as_str())
    });
    Outcome::fail_if(
        !self_ref,
        rule,
        "aucun hreflang ne référence la page elle-même",
        ctx.snapshot.hreflangs.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::origin;
    use crate::seo::rules::test_support::*;
    use crate::seo::snapshot::{Hreflang, PageSnapshot};

    fn snap() -> PageSnapshot {
        PageSnapshot {
            url: "https://Example.test/fr/page/".into(),
            canonicals: vec!["https://example.test/fr/page".into()],
            ..Default::default()
        }
    }

    #[test]
    fn origin_parsing() {
        assert_eq!(
            origin("https://A.test:8443/x?y#z").as_deref(),
            Some("https://a.test:8443")
        );
        assert_eq!(origin("/relative"), None);
        assert_eq!(origin("https://"), None);
    }

    #[test]
    fn single_same_origin_canonical_passes() {
        let s = snap();
        assert!(is_pass(&run("SEO-CANON-01", &s)));
        assert!(is_pass(&run("SEO-CANON-02", &s)));
        assert!(is_pass(&run("SEO-CANON-03", &s)));
    }

    #[test]
    fn no_canonical_fails_presence_and_makes_others_na() {
        let s = PageSnapshot {
            canonicals: vec![],
            ..snap()
        };
        assert!(is_fail(&run("SEO-CANON-01", &s)));
        assert!(is_na(&run("SEO-CANON-02", &s)));
        assert!(is_na(&run("SEO-CANON-03", &s)));
    }

    #[test]
    fn foreign_origin_canonical_fails() {
        let s = PageSnapshot {
            canonicals: vec!["https://other.test/page".into()],
            ..snap()
        };
        assert!(is_fail(&run("SEO-CANON-02", &s)));
    }

    #[test]
    fn duplicate_identical_canonicals_pass_but_conflicting_fail() {
        let same = PageSnapshot {
            canonicals: vec![
                "https://example.test/fr/page".into(),
                "https://example.test/fr/page/".into(),
            ],
            ..snap()
        };
        assert!(is_pass(&run("SEO-CANON-03", &same)));
        let conflict = PageSnapshot {
            canonicals: vec![
                "https://example.test/a".into(),
                "https://example.test/b".into(),
            ],
            ..snap()
        };
        assert!(is_fail(&run("SEO-CANON-03", &conflict)));
    }

    #[test]
    fn hreflang_self_reference() {
        let hl = |href: &str| Hreflang {
            lang: "fr".into(),
            href: href.into(),
        };
        let s = PageSnapshot {
            hreflangs: vec![hl("https://example.test/en/page")],
            ..snap()
        };
        assert!(is_fail(&run("SEO-HREF-01", &s)));
        let s = PageSnapshot {
            hreflangs: vec![hl("https://example.test/fr/page")],
            ..snap()
        };
        assert!(is_pass(&run("SEO-HREF-01", &s)));
        let s = PageSnapshot {
            hreflangs: vec![],
            ..snap()
        };
        assert!(is_na(&run("SEO-HREF-01", &s)));
    }
}
