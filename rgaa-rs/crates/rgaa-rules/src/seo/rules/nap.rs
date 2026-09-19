use super::{Ctx, Outcome};
use crate::seo::catalog::SeoRule;
use crate::seo::snapshot::BusinessProfile;

/// Lower-case, alphanumeric-only (accents kept) so punctuation and spacing differences don't matter.
fn normalize_text(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// Phone comparison ignores the country prefix: `+33 4 12 34 56 78` and `04 12 34 56 78` both end in `412345678`.
fn phone_matches(page_digits: &str, profile_phone: &str) -> bool {
    let wanted = digits(profile_phone);
    let wanted = wanted.trim_start_matches('0');
    let core = if wanted.len() > 9 {
        &wanted[wanted.len() - 9..]
    } else {
        wanted
    };
    !core.is_empty() && page_digits.contains(core)
}

fn with_profile<'a>(ctx: &'a Ctx<'_>) -> Option<&'a BusinessProfile> {
    ctx.profile
}

pub(super) fn name_present(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let Some(p) = with_profile(ctx) else {
        return Outcome::NotApplicable;
    };
    let found = normalize_text(&ctx.snapshot.body_text).contains(&normalize_text(&p.name));
    Outcome::fail_if(
        !found,
        rule,
        format!("nom « {} » introuvable sur la page", p.name),
        1,
    )
}

pub(super) fn phone_present(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let Some(p) = with_profile(ctx) else {
        return Outcome::NotApplicable;
    };
    let found = phone_matches(&digits(&ctx.snapshot.body_text), &p.phone);
    Outcome::fail_if(
        !found,
        rule,
        format!("téléphone « {} » introuvable sur la page", p.phone),
        1,
    )
}

pub(super) fn address_present(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    let Some(p) = with_profile(ctx) else {
        return Outcome::NotApplicable;
    };
    let found = normalize_text(&ctx.snapshot.body_text).contains(&normalize_text(&p.address));
    Outcome::fail_if(
        !found,
        rule,
        format!("adresse « {} » introuvable sur la page", p.address),
        1,
    )
}

pub(super) fn local_business_schema(rule: &SeoRule, ctx: &Ctx<'_>) -> Outcome {
    if with_profile(ctx).is_none() {
        return Outcome::NotApplicable;
    }
    let present = ctx.nodes_of_type("LocalBusiness").next().is_some();
    Outcome::fail_if(!present, rule, "aucun schéma LocalBusiness", 1)
}

#[cfg(test)]
mod tests {
    use crate::seo::rules::test_support::*;
    use crate::seo::snapshot::{BusinessProfile, PageSnapshot};

    fn profile() -> BusinessProfile {
        BusinessProfile {
            name: "Plomberie Dupont & Fils".into(),
            phone: "+33 4 12 34 56 78".into(),
            address: "12 rue de la République, 69001 Lyon".into(),
        }
    }

    fn page() -> PageSnapshot {
        PageSnapshot {
            body_text: "Bienvenue chez PLOMBERIE DUPONT ET FILS ... Appelez le 04.12.34.56.78 — 12, Rue de la Republique 69001 LYON".into(),
            json_ld: vec![r#"{"@context":"https://schema.org","@type":"LocalBusiness","name":"x","address":"a","telephone":"t"}"#.into()],
            ..Default::default()
        }
    }

    #[test]
    fn without_profile_everything_is_na() {
        let s = page();
        for id in ["SEO-NAP-01", "SEO-NAP-02", "SEO-NAP-03", "SEO-NAP-04"] {
            assert!(is_na(&run(id, &s)), "{id}");
        }
    }

    #[test]
    fn phone_and_schema_match_despite_formatting() {
        let s = page();
        let p = profile();
        assert!(is_pass(&run_with("SEO-NAP-02", &s, Some(&p))));
        assert!(is_pass(&run_with("SEO-NAP-04", &s, Some(&p))));
    }

    #[test]
    fn name_with_ampersand_vs_et_is_a_mismatch() {
        let s = page();
        let p = profile();
        assert!(is_fail(&run_with("SEO-NAP-01", &s, Some(&p))));
        let p2 = BusinessProfile {
            name: "Plomberie Dupont et Fils".into(),
            ..p
        };
        assert!(is_pass(&run_with("SEO-NAP-01", &s, Some(&p2))));
    }

    #[test]
    fn address_accent_difference_is_a_mismatch() {
        let s = page();
        let p = profile();
        assert!(is_fail(&run_with("SEO-NAP-03", &s, Some(&p))));
        let p2 = BusinessProfile {
            address: "12 rue de la Republique 69001 Lyon".into(),
            ..p
        };
        assert!(is_pass(&run_with("SEO-NAP-03", &s, Some(&p2))));
    }

    #[test]
    fn missing_phone_and_schema_fail() {
        let s = PageSnapshot {
            body_text: "rien".into(),
            json_ld: vec![],
            ..page()
        };
        let p = profile();
        assert!(is_fail(&run_with("SEO-NAP-02", &s, Some(&p))));
        assert!(is_fail(&run_with("SEO-NAP-04", &s, Some(&p))));
    }
}
