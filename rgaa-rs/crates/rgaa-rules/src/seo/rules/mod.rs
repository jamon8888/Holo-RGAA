mod aeo;
mod canonical;
mod geo;
mod headings;
mod meta;
mod nap;
mod schema_org;

use super::catalog::SeoRule;
use super::snapshot::{BusinessProfile, PageSnapshot};
use rgaa_core::Violation;
use serde_json::Value;

pub(crate) enum Outcome {
    Pass,
    NotApplicable,
    Fail(Vec<Violation>),
}

impl Outcome {
    pub(crate) fn fail_if(
        failed: bool,
        rule: &SeoRule,
        description: impl Into<String>,
        nodes: usize,
    ) -> Self {
        if failed {
            Self::Fail(vec![violation(rule, description, nodes)])
        } else {
            Self::Pass
        }
    }
}

pub(crate) fn violation(rule: &SeoRule, description: impl Into<String>, nodes: usize) -> Violation {
    Violation {
        rule_id: rule.id.clone(),
        impact: rule.criticality.as_str().to_string(),
        description: description.into(),
        nodes_affected: nodes,
    }
}

pub(crate) struct Ctx<'a> {
    pub snapshot: &'a PageSnapshot,
    pub profile: Option<&'a BusinessProfile>,
    /// One entry per `<script type="application/ld+json">`; `Err` holds the parse error.
    pub json_ld: Vec<Result<Value, String>>,
    /// Every schema.org node (with `@graph` flattened) that parsed successfully.
    pub nodes: Vec<Value>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(snapshot: &'a PageSnapshot, profile: Option<&'a BusinessProfile>) -> Self {
        let json_ld: Vec<Result<Value, String>> = snapshot
            .json_ld
            .iter()
            .map(|raw| serde_json::from_str::<Value>(raw).map_err(|e| e.to_string()))
            .collect();
        let nodes = json_ld
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .flat_map(schema_org::flatten)
            .collect();
        Self {
            snapshot,
            profile,
            json_ld,
            nodes,
        }
    }

    pub(crate) fn nodes_of_type<'s>(&'s self, ty: &'s str) -> impl Iterator<Item = &'s Value> + 's {
        self.nodes
            .iter()
            .filter(move |n| schema_org::has_type(n, ty))
    }
}

pub(crate) fn evaluate(rule: &SeoRule, ctx: &Ctx<'_>) -> Option<Outcome> {
    let out = match rule.id.as_str() {
        "SEO-META-01" => meta::title_present(rule, ctx),
        "SEO-META-02" => meta::title_length(rule, ctx),
        "SEO-META-03" => meta::description_present(rule, ctx),
        "SEO-META-04" => meta::description_length(rule, ctx),
        "SEO-META-05" => meta::not_noindex(rule, ctx),
        "SEO-LANG-01" => meta::lang_present(rule, ctx),
        "SEO-CANON-01" => canonical::present(rule, ctx),
        "SEO-CANON-02" => canonical::same_origin(rule, ctx),
        "SEO-CANON-03" => canonical::single(rule, ctx),
        "SEO-HREF-01" => canonical::hreflang_self_reference(rule, ctx),
        "SEO-HEAD-01" => headings::single_h1(rule, ctx),
        "SEO-HEAD-02" => headings::no_level_skip(rule, ctx),
        "SEO-IMG-01" => headings::images_have_alt(rule, ctx),
        "SEO-SCHEMA-01" => schema_org::valid_json(rule, ctx),
        "SEO-SCHEMA-02" => schema_org::context_and_type(rule, ctx),
        "SEO-SCHEMA-03" => schema_org::required_properties(rule, ctx),
        "SEO-NAP-01" => nap::name_present(rule, ctx),
        "SEO-NAP-02" => nap::phone_present(rule, ctx),
        "SEO-NAP-03" => nap::address_present(rule, ctx),
        "SEO-NAP-04" => nap::local_business_schema(rule, ctx),
        "GEO-01" => geo::entity_declared(rule, ctx),
        "GEO-02" => geo::entity_same_as(rule, ctx),
        "GEO-03" => geo::article_author_and_date(rule, ctx),
        "GEO-04" => geo::article_date_modified(rule, ctx),
        "GEO-05" => geo::lead_paragraph(rule, ctx),
        "AEO-01" => aeo::questions_have_faq_schema(rule, ctx),
        "AEO-02" => aeo::faq_answers_complete(rule, ctx),
        "AEO-03" => aeo::howto_steps_complete(rule, ctx),
        "AEO-04" => aeo::questions_have_concise_answer(rule, ctx),
        "AEO-05" => aeo::speakable_declared(rule, ctx),
        "AEO-06" => aeo::breadcrumb_declared(rule, ctx),
        _ => return None,
    };
    Some(out)
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::super::catalog::SeoCatalog;
    use super::super::snapshot::PageSnapshot;
    use super::*;

    pub(crate) fn rule(id: &str) -> &'static SeoRule {
        SeoCatalog::get().find(id).expect("rule in catalog")
    }

    pub(crate) fn run(id: &str, snapshot: &PageSnapshot) -> Outcome {
        run_with(id, snapshot, None)
    }

    pub(crate) fn run_with(
        id: &str,
        snapshot: &PageSnapshot,
        profile: Option<&BusinessProfile>,
    ) -> Outcome {
        let ctx = Ctx::new(snapshot, profile);
        evaluate(rule(id), &ctx).expect("rule is dispatched")
    }

    pub(crate) fn is_pass(o: &Outcome) -> bool {
        matches!(o, Outcome::Pass)
    }
    pub(crate) fn is_na(o: &Outcome) -> bool {
        matches!(o, Outcome::NotApplicable)
    }
    pub(crate) fn is_fail(o: &Outcome) -> bool {
        matches!(o, Outcome::Fail(_))
    }
}

#[cfg(test)]
mod tests {
    use super::super::catalog::SeoCatalog;
    use super::*;

    #[test]
    fn every_catalog_rule_is_dispatched() {
        let snapshot = PageSnapshot::default();
        let ctx = Ctx::new(&snapshot, None);
        for rule in SeoCatalog::get().rules().values() {
            assert!(
                evaluate(rule, &ctx).is_some(),
                "{} has no evaluator",
                rule.id
            );
        }
    }

    #[test]
    fn violation_impact_carries_criticality() {
        let rule = SeoCatalog::get().find("SEO-META-05").unwrap();
        let v = violation(rule, "x", 1);
        assert_eq!(v.impact, "P0");
        assert_eq!(v.rule_id, "SEO-META-05");
    }
}
