use super::{Ctx, Outcome};
use crate::seo::catalog::SeoRule;

pub(super) fn single_h1(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let n = ctx
        .snapshot
        .headings
        .iter()
        .filter(|h| h.level == 1)
        .count();
    Outcome::fail_if(n != 1, rule, format!("{n} élément(s) <h1>"), n.max(1))
}

pub(super) fn no_level_skip(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if ctx.snapshot.headings.is_empty() {
        return Outcome::NotApplicable;
    }
    let mut skips = 0usize;
    let mut previous: Option<u8> = None;
    for h in &ctx.snapshot.headings {
        if let Some(prev) = previous {
            if h.level > prev + 1 {
                skips += 1;
            }
        }
        previous = Some(h.level);
    }
    Outcome::fail_if(
        skips > 0,
        rule,
        format!("{skips} saut(s) de niveau de titre"),
        skips,
    )
}

pub(super) fn images_have_alt(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if ctx.snapshot.images_total == 0 {
        return Outcome::NotApplicable;
    }
    let missing = ctx.snapshot.images_missing_alt;
    Outcome::fail_if(
        missing > 0,
        rule,
        format!("{missing} image(s) sans attribut alt"),
        missing,
    )
}

#[cfg(test)]
mod tests {
    use crate::seo::rules::test_support::*;
    use crate::seo::snapshot::{Heading, PageSnapshot};

    fn h(level: u8) -> Heading {
        Heading {
            level,
            text: format!("h{level}"),
            next_paragraph: None,
        }
    }

    #[test]
    fn one_h1_and_ordered_levels_pass() {
        let s = PageSnapshot {
            headings: vec![h(1), h(2), h(3), h(2)],
            ..Default::default()
        };
        assert!(is_pass(&run("SEO-HEAD-01", &s)));
        assert!(is_pass(&run("SEO-HEAD-02", &s)));
    }

    #[test]
    fn zero_or_two_h1_fail() {
        let none = PageSnapshot {
            headings: vec![h(2)],
            ..Default::default()
        };
        assert!(is_fail(&run("SEO-HEAD-01", &none)));
        let two = PageSnapshot {
            headings: vec![h(1), h(1)],
            ..Default::default()
        };
        assert!(is_fail(&run("SEO-HEAD-01", &two)));
    }

    #[test]
    fn level_skip_fails_but_going_up_is_fine() {
        let skip = PageSnapshot {
            headings: vec![h(1), h(3)],
            ..Default::default()
        };
        assert!(is_fail(&run("SEO-HEAD-02", &skip)));
        let up = PageSnapshot {
            headings: vec![h(1), h(2), h(3), h(1)],
            ..Default::default()
        };
        assert!(is_pass(&run("SEO-HEAD-02", &up)));
        assert!(is_na(&run("SEO-HEAD-02", &PageSnapshot::default())));
    }

    #[test]
    fn images_alt() {
        assert!(is_na(&run("SEO-IMG-01", &PageSnapshot::default())));
        let ok = PageSnapshot {
            images_total: 3,
            images_missing_alt: 0,
            ..Default::default()
        };
        assert!(is_pass(&run("SEO-IMG-01", &ok)));
        let bad = PageSnapshot {
            images_total: 3,
            images_missing_alt: 2,
            ..Default::default()
        };
        assert!(is_fail(&run("SEO-IMG-01", &bad)));
    }
}
