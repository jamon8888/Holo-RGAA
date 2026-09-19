use super::{violation, Ctx, Outcome};
use crate::seo::catalog::SeoRule;
use serde_json::Value;

/// Required properties per schema.org `@type`. Kept in sync with SEO-SCHEMA-03's description.
const REQUIRED: &[(&str, &[&str])] = &[
    ("Organization", &["name", "url"]),
    ("LocalBusiness", &["name", "address", "telephone"]),
    ("Article", &["headline", "author", "datePublished"]),
    ("NewsArticle", &["headline", "author", "datePublished"]),
    ("BlogPosting", &["headline", "author", "datePublished"]),
    ("Product", &["name", "offers"]),
    ("WebSite", &["name", "url"]),
    ("BreadcrumbList", &["itemListElement"]),
    ("FAQPage", &["mainEntity"]),
    ("HowTo", &["name", "step"]),
];

/// Every object node in a JSON-LD document: the root, each `@graph` entry, or each array element.
pub(super) fn flatten(doc: &Value) -> Vec<Value> {
    match doc {
        Value::Array(items) => items.iter().flat_map(flatten).collect(),
        Value::Object(obj) => match obj.get("@graph") {
            Some(Value::Array(graph)) => graph.iter().flat_map(flatten).collect(),
            _ => vec![doc.clone()],
        },
        _ => vec![],
    }
}

pub(super) fn types_of(node: &Value) -> Vec<&str> {
    match node.get("@type") {
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    }
}

pub(super) fn has_type(node: &Value, ty: &str) -> bool {
    types_of(node).contains(&ty)
}

pub(super) fn has_prop(node: &Value, key: &str) -> bool {
    match node.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(_) => true,
    }
}

pub(super) fn as_list(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(v) if !v.is_null() => vec![v],
        _ => vec![],
    }
}

fn is_schema_org_context(node: &Value) -> bool {
    let matches = |s: &str| s.trim_end_matches('/').ends_with("schema.org");
    match node.get("@context") {
        Some(Value::String(s)) => matches(s),
        Some(Value::Object(o)) => o.get("@vocab").and_then(Value::as_str).is_some_and(matches),
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).any(matches),
        _ => false,
    }
}

pub(super) fn valid_json(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if ctx.json_ld.is_empty() {
        return Outcome::NotApplicable;
    }
    let errors: Vec<&String> = ctx
        .json_ld
        .iter()
        .filter_map(|r| r.as_ref().err())
        .collect();
    match errors.first() {
        None => Outcome::Pass,
        Some(first) => Outcome::Fail(vec![violation(
            rule,
            format!("JSON-LD invalide : {first}"),
            errors.len(),
        )]),
    }
}

pub(super) fn context_and_type(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let docs: Vec<&Value> = ctx.json_ld.iter().filter_map(|r| r.as_ref().ok()).collect();
    if docs.is_empty() {
        return Outcome::NotApplicable;
    }
    let mut bad = 0usize;
    for doc in docs {
        let roots = match doc {
            Value::Array(items) => items.iter().collect(),
            other => vec![other],
        };
        for root in roots {
            let typed = flatten(root).iter().all(|n| !types_of(n).is_empty());
            if !is_schema_org_context(root) || !typed {
                bad += 1;
            }
        }
    }
    Outcome::fail_if(
        bad > 0,
        rule,
        format!("{bad} bloc(s) JSON-LD sans @context schema.org ou sans @type"),
        bad,
    )
}

pub(super) fn required_properties(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if ctx.nodes.is_empty() {
        return Outcome::NotApplicable;
    }
    let mut violations = Vec::new();
    for node in &ctx.nodes {
        for (ty, required) in REQUIRED {
            if !has_type(node, ty) {
                continue;
            }
            let missing: Vec<&str> = required
                .iter()
                .copied()
                .filter(|k| !has_prop(node, k))
                .collect();
            if !missing.is_empty() {
                violations.push(violation(
                    rule,
                    format!("{ty} sans {}", missing.join(", ")),
                    1,
                ));
            }
        }
    }
    if violations.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(violations)
    }
}

#[cfg(test)]
mod tests {
    use crate::seo::rules::test_support::*;
    use crate::seo::snapshot::PageSnapshot;

    fn with(json_ld: &[&str]) -> PageSnapshot {
        PageSnapshot {
            json_ld: json_ld.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn no_json_ld_is_na_everywhere() {
        let s = PageSnapshot::default();
        for id in ["SEO-SCHEMA-01", "SEO-SCHEMA-02", "SEO-SCHEMA-03"] {
            assert!(is_na(&run(id, &s)), "{id}");
        }
    }

    #[test]
    fn broken_json_fails_only_syntax_rule() {
        let s = with(&[
            r#"{"@context":"https://schema.org","@type":"Organization","name":"A","url":"https://a.test"}"#,
            "{ not json",
        ]);
        assert!(is_fail(&run("SEO-SCHEMA-01", &s)));
        assert!(is_pass(&run("SEO-SCHEMA-02", &s)));
        assert!(is_pass(&run("SEO-SCHEMA-03", &s)));
    }

    #[test]
    fn missing_context_or_type_fails() {
        let s = with(&[r#"{"@type":"Organization","name":"A","url":"u"}"#]);
        assert!(is_fail(&run("SEO-SCHEMA-02", &s)));
        let s = with(&[r#"{"@context":"https://schema.org","name":"A"}"#]);
        assert!(is_fail(&run("SEO-SCHEMA-02", &s)));
    }

    #[test]
    fn graph_nodes_are_flattened_and_checked() {
        let s = with(&[r#"{"@context":"https://schema.org","@graph":[
            {"@type":"WebSite","name":"A","url":"https://a.test"},
            {"@type":"LocalBusiness","name":"A"}
        ]}"#]);
        assert!(is_pass(&run("SEO-SCHEMA-02", &s)));
        match run("SEO-SCHEMA-03", &s) {
            crate::seo::rules::Outcome::Fail(v) => {
                assert_eq!(v.len(), 1);
                assert!(v[0].description.contains("LocalBusiness"));
                assert!(v[0].description.contains("address"));
            }
            _ => panic!("expected fail"),
        }
    }

    #[test]
    fn type_arrays_and_empty_strings() {
        let s = with(&[
            r#"{"@context":"http://schema.org/","@type":["Organization","Brand"],"name":"","url":"u"}"#,
        ]);
        assert!(is_pass(&run("SEO-SCHEMA-02", &s)));
        assert!(is_fail(&run("SEO-SCHEMA-03", &s)));
    }
}
