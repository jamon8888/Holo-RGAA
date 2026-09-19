use super::{Ctx, Outcome};
use crate::seo::catalog::SeoRule;

const TITLE_RANGE: std::ops::RangeInclusive<usize> = 10..=60;
const DESCRIPTION_RANGE: std::ops::RangeInclusive<usize> = 50..=160;

fn trimmed(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

pub(super) fn title_present(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    Outcome::fail_if(
        trimmed(&ctx.snapshot.title).is_none(),
        rule,
        "<title> absent ou vide",
        1,
    )
}

pub(super) fn title_length(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    match trimmed(&ctx.snapshot.title) {
        None => Outcome::NotApplicable,
        Some(t) => {
            let n = t.chars().count();
            Outcome::fail_if(
                !TITLE_RANGE.contains(&n),
                rule,
                format!("<title> fait {n} caractères"),
                1,
            )
        }
    }
}

pub(super) fn description_present(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    Outcome::fail_if(
        trimmed(&ctx.snapshot.meta_description).is_none(),
        rule,
        "meta description absente ou vide",
        1,
    )
}

pub(super) fn description_length(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    match trimmed(&ctx.snapshot.meta_description) {
        None => Outcome::NotApplicable,
        Some(d) => {
            let n = d.chars().count();
            Outcome::fail_if(
                !DESCRIPTION_RANGE.contains(&n),
                rule,
                format!("meta description fait {n} caractères"),
                1,
            )
        }
    }
}

pub(super) fn not_noindex(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let noindex = ctx
        .snapshot
        .meta_robots
        .iter()
        .any(|d| d == "noindex" || d == "none");
    Outcome::fail_if(noindex, rule, "directive robots noindex présente", 1)
}

pub(super) fn lang_present(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    Outcome::fail_if(
        trimmed(&ctx.snapshot.lang).is_none(),
        rule,
        "attribut lang absent sur <html>",
        1,
    )
}

#[cfg(test)]
mod tests {
    use crate::seo::rules::test_support::*;
    use crate::seo::snapshot::PageSnapshot;

    fn snap() -> PageSnapshot {
        PageSnapshot {
            title: Some("Plombier à Lyon — Dépannage 24h/24".into()),
            meta_description: Some("Intervention rapide sur Lyon et sa métropole pour toute fuite, chauffe-eau ou canalisation bouchée.".into()),
            lang: Some("fr".into()),
            ..Default::default()
        }
    }

    #[test]
    fn well_formed_meta_passes() {
        let s = snap();
        for id in [
            "SEO-META-01",
            "SEO-META-02",
            "SEO-META-03",
            "SEO-META-04",
            "SEO-META-05",
            "SEO-LANG-01",
        ] {
            assert!(is_pass(&run(id, &s)), "{id}");
        }
    }

    #[test]
    fn missing_title_fails_presence_and_makes_length_na() {
        let s = PageSnapshot {
            title: Some("   ".into()),
            ..snap()
        };
        assert!(is_fail(&run("SEO-META-01", &s)));
        assert!(is_na(&run("SEO-META-02", &s)));
    }

    #[test]
    fn title_too_long_fails() {
        let s = PageSnapshot {
            title: Some("x".repeat(61)),
            ..snap()
        };
        assert!(is_fail(&run("SEO-META-02", &s)));
    }

    #[test]
    fn description_too_short_fails() {
        let s = PageSnapshot {
            meta_description: Some("Trop court.".into()),
            ..snap()
        };
        assert!(is_fail(&run("SEO-META-04", &s)));
    }

    #[test]
    fn noindex_and_none_fail() {
        for d in ["noindex", "none"] {
            let s = PageSnapshot {
                meta_robots: vec![d.into()],
                ..snap()
            };
            assert!(is_fail(&run("SEO-META-05", &s)), "{d}");
        }
        let s = PageSnapshot {
            meta_robots: vec!["nofollow".into()],
            ..snap()
        };
        assert!(is_pass(&run("SEO-META-05", &s)));
    }

    #[test]
    fn missing_lang_fails() {
        let s = PageSnapshot {
            lang: None,
            ..snap()
        };
        assert!(is_fail(&run("SEO-LANG-01", &s)));
    }
}
