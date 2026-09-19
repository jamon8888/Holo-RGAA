use super::schema_org::{as_list, has_prop, has_type};
use super::{Ctx, Outcome};
use crate::seo::catalog::SeoRule;
use crate::seo::snapshot::Heading;

const ANSWER_RANGE: std::ops::RangeInclusive<usize> = 40..=300;

fn is_question(h: &Heading) -> bool {
    h.text.trim_end().ends_with('?')
}

fn question_headings<'a>(ctx: &'a Ctx<'_>) -> impl Iterator<Item = &'a Heading> {
    ctx.snapshot.headings.iter().filter(|h| is_question(h))
}

pub(super) fn questions_have_faq_schema(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let questions = question_headings(ctx).count();
    if questions == 0 {
        return Outcome::NotApplicable;
    }
    let has_faq = ctx.nodes_of_type("FAQPage").next().is_some();
    Outcome::fail_if(
        !has_faq,
        rule,
        format!("{questions} titre(s)-question sans schéma FAQPage"),
        questions,
    )
}

pub(super) fn faq_answers_complete(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let mut any = false;
    let mut bad = 0usize;
    for faq in ctx.nodes_of_type("FAQPage") {
        any = true;
        for q in as_list(faq.get("mainEntity")) {
            let answer_ok = as_list(q.get("acceptedAnswer"))
                .iter()
                .any(|a| has_prop(a, "text"));
            if !has_type(q, "Question") || !has_prop(q, "name") || !answer_ok {
                bad += 1;
            }
        }
    }
    if !any {
        return Outcome::NotApplicable;
    }
    Outcome::fail_if(
        bad > 0,
        rule,
        format!("{bad} Question sans name ou acceptedAnswer.text"),
        bad,
    )
}

pub(super) fn howto_steps_complete(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let mut any = false;
    let mut bad = 0usize;
    for howto in ctx.nodes_of_type("HowTo") {
        any = true;
        let steps = as_list(howto.get("step"));
        if steps.is_empty() {
            bad += 1;
        }
        bad += steps
            .iter()
            .filter(|s| !has_prop(s, "text") && !has_prop(s, "name"))
            .count();
    }
    if !any {
        return Outcome::NotApplicable;
    }
    Outcome::fail_if(
        bad > 0,
        rule,
        format!("{bad} step HowTo sans text ni name"),
        bad,
    )
}

pub(super) fn questions_have_concise_answer(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let mut any = false;
    let mut bad = 0usize;
    for h in question_headings(ctx) {
        any = true;
        let n = h
            .next_paragraph
            .as_deref()
            .map_or(0, |p| p.trim().chars().count());
        if !ANSWER_RANGE.contains(&n) {
            bad += 1;
        }
    }
    if !any {
        return Outcome::NotApplicable;
    }
    Outcome::fail_if(
        bad > 0,
        rule,
        format!("{bad} question(s) sans réponse concise (40-300 caractères)"),
        bad,
    )
}

pub(super) fn speakable_declared(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let present = ctx.nodes.iter().any(|n| has_prop(n, "speakable"));
    Outcome::fail_if(!present, rule, "aucun balisage speakable", 1)
}

pub(super) fn breadcrumb_declared(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let present = ctx.nodes_of_type("BreadcrumbList").next().is_some();
    Outcome::fail_if(!present, rule, "aucun schéma BreadcrumbList", 1)
}

#[cfg(test)]
mod tests {
    use crate::seo::rules::test_support::*;
    use crate::seo::snapshot::{Heading, PageSnapshot};

    fn q(text: &str, answer: Option<&str>) -> Heading {
        Heading {
            level: 2,
            text: text.into(),
            next_paragraph: answer.map(Into::into),
        }
    }

    fn with(json_ld: &[&str]) -> PageSnapshot {
        PageSnapshot {
            json_ld: json_ld.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    const FAQ_OK: &str = r#"{"@context":"https://schema.org","@type":"FAQPage","mainEntity":[
        {"@type":"Question","name":"Q ?","acceptedAnswer":{"@type":"Answer","text":"R"}}]}"#;
    const FAQ_BAD: &str = r#"{"@context":"https://schema.org","@type":"FAQPage","mainEntity":
        {"@type":"Question","name":"Q ?","acceptedAnswer":{"@type":"Answer"}}}"#;

    #[test]
    fn question_headings_need_faq_schema() {
        assert!(is_na(&run("AEO-01", &PageSnapshot::default())));
        let s = PageSnapshot {
            headings: vec![q("Combien ça coûte ?", None)],
            ..Default::default()
        };
        assert!(is_fail(&run("AEO-01", &s)));
        let s = PageSnapshot {
            headings: vec![q("Combien ça coûte ?", None)],
            ..with(&[FAQ_OK])
        };
        assert!(is_pass(&run("AEO-01", &s)));
    }

    #[test]
    fn faq_completeness() {
        assert!(is_na(&run("AEO-02", &PageSnapshot::default())));
        assert!(is_pass(&run("AEO-02", &with(&[FAQ_OK]))));
        assert!(is_fail(&run("AEO-02", &with(&[FAQ_BAD]))));
    }

    #[test]
    fn howto_completeness() {
        assert!(is_na(&run("AEO-03", &PageSnapshot::default())));
        let ok = with(&[
            r#"{"@context":"https://schema.org","@type":"HowTo","name":"n","step":[{"@type":"HowToStep","text":"t"}]}"#,
        ]);
        assert!(is_pass(&run("AEO-03", &ok)));
        let bad = with(&[
            r#"{"@context":"https://schema.org","@type":"HowTo","name":"n","step":[{"@type":"HowToStep"}]}"#,
        ]);
        assert!(is_fail(&run("AEO-03", &bad)));
        let empty =
            with(&[r#"{"@context":"https://schema.org","@type":"HowTo","name":"n","step":[]}"#]);
        assert!(is_fail(&run("AEO-03", &empty)));
    }

    #[test]
    fn concise_answer_after_question() {
        assert!(is_na(&run("AEO-04", &PageSnapshot::default())));
        let good = PageSnapshot {
            headings: vec![q("Pourquoi ?", Some(&"x".repeat(120)))],
            ..Default::default()
        };
        assert!(is_pass(&run("AEO-04", &good)));
        let none = PageSnapshot {
            headings: vec![q("Pourquoi ?", None)],
            ..Default::default()
        };
        assert!(is_fail(&run("AEO-04", &none)));
        let long = PageSnapshot {
            headings: vec![q("Pourquoi ?", Some(&"x".repeat(301)))],
            ..Default::default()
        };
        assert!(is_fail(&run("AEO-04", &long)));
    }

    #[test]
    fn speakable_and_breadcrumb() {
        assert!(is_fail(&run("AEO-05", &PageSnapshot::default())));
        assert!(is_fail(&run("AEO-06", &PageSnapshot::default())));
        let s = with(&[
            r#"{"@context":"https://schema.org","@type":"WebPage","speakable":{"@type":"SpeakableSpecification","cssSelector":["h1"]}}"#,
            r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[]}"#,
        ]);
        assert!(is_pass(&run("AEO-05", &s)));
        assert!(is_pass(&run("AEO-06", &s)));
    }
}
