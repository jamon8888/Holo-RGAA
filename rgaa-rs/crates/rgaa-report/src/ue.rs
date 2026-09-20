//! UE declaration: the 2018/1523 core sections as semantic HTML.
//!
//! The reviewer sets the qualitative status; the engine never invents it
//! (see `taux_juridique`). The pack supplies language and enforcement.

use std::fmt::Write;

use crate::guard::{Contact, Derogation};
use crate::packs::{pack, Pays};
use crate::ReportError;

/// Everything the UE template needs, nothing it doesn't.
pub struct DeclarationUeInput<'a> {
    pub pays: Pays,
    pub service: &'a str,
    pub organisme: &'a str,
    /// Reviewer-set legal status in the pack language, never empty.
    pub statut: &'a str,
    pub non_conformites: &'a [String],
    pub derogations: &'a [Derogation],
    pub contact: &'a Contact,
    pub date_declaration: &'a str,
    pub methode_evaluation: &'a str,
}

fn echappe(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Fixed headings in the rendered content language. Only languages with a
/// validated template exist here (English default, German from the validated
/// prototype); every other pack renders English and is honestly labeled
/// `lang="en"` until its translation lands (i18n backlog, not this ticket).
struct Gabarit {
    langue: &'static str,
    titre: &'static str,
    engagement: &'static str,
    contenu: &'static str,
    charge: &'static str,
    preparation: &'static str,
    retour: &'static str,
    recours: &'static str,
    aucun_nc: &'static str,
    aucune_derogation: &'static str,
}

const GABARIT_EN: Gabarit = Gabarit {
    langue: "en",
    titre: "Accessibility statement",
    engagement: "Commitment",
    contenu: "Non-accessible content",
    charge: "Disproportionate burden",
    preparation: "Preparation",
    retour: "Feedback",
    recours: "Enforcement",
    aucun_nc: "None reported.",
    aucune_derogation: "None claimed.",
};

const GABARIT_DE: Gabarit = Gabarit {
    langue: "de",
    titre: "Erklärung zur Barrierefreiheit",
    engagement: "Verpflichtung",
    contenu: "Nicht barrierefreie Inhalte",
    charge: "Unverhältnismäßige Belastung",
    preparation: "Erstellung",
    retour: "Feedback und Kontakt",
    recours: "Durchsetzungsverfahren",
    aucun_nc: "Keine gemeldet.",
    aucune_derogation: "Keine geltend gemacht.",
};

fn gabarit(pays: Pays) -> &'static Gabarit {
    match pays {
        Pays::De => &GABARIT_DE,
        _ => &GABARIT_EN,
    }
}

/// Renders the UE-model declaration of `input.pays`.
pub fn render_declaration_ue(input: &DeclarationUeInput) -> Result<String, ReportError> {
    if input.statut.trim().is_empty() {
        return Err(ReportError::invalid_input(
            "statut juridique fixé par le relecteur exigé".to_string(),
        ));
    }
    if input.contact.canal.trim().is_empty() {
        return Err(ReportError::invalid_input(
            "contact de retour d'information manquant".to_string(),
        ));
    }
    let pack = pack(input.pays);
    let gabarit = gabarit(input.pays);
    let mut out = String::new();
    let _ = writeln!(out, "<!DOCTYPE html>");
    let _ = writeln!(out, "<html lang=\"{}\">", gabarit.langue);
    let _ = writeln!(out, "<head><meta charset=\"utf-8\">");
    let _ = writeln!(
        out,
        "<title>{} — {}</title></head>",
        gabarit.titre,
        echappe(input.service)
    );
    let _ = writeln!(out, "<body><main>");
    let _ = writeln!(
        out,
        "<h1>{} — {}</h1>",
        echappe(input.service),
        echappe(input.statut)
    );
    let _ = writeln!(
        out,
        "<p>{}: {}.</p>",
        gabarit.engagement,
        echappe(input.organisme)
    );
    let _ = writeln!(out, "<h2>{}</h2>", gabarit.contenu);
    if input.non_conformites.is_empty() {
        let _ = writeln!(out, "<p>{}</p>", gabarit.aucun_nc);
    } else {
        let _ = writeln!(out, "<ul>");
        for nc in input.non_conformites {
            let _ = writeln!(out, "<li>{}.</li>", echappe(nc));
        }
        let _ = writeln!(out, "</ul>");
    }
    let _ = writeln!(out, "<h2>{}</h2>", gabarit.charge);
    if input.derogations.is_empty() {
        let _ = writeln!(out, "<p>{}</p>", gabarit.aucune_derogation);
    } else {
        let _ = writeln!(out, "<ul>");
        for derogation in input.derogations {
            let _ = writeln!(
                out,
                "<li>{}: {} ; alternative : {} ; review {}.</li>",
                echappe(&derogation.contenu),
                echappe(&derogation.motif),
                echappe(&derogation.alternative),
                echappe(&derogation.date_reexamen)
            );
        }
        let _ = writeln!(out, "</ul>");
    }
    let _ = writeln!(out, "<h2>{}</h2>", gabarit.preparation);
    let _ = writeln!(
        out,
        "<p>{} {} via {}.</p>",
        gabarit.preparation,
        echappe(input.date_declaration),
        echappe(input.methode_evaluation)
    );
    let _ = writeln!(out, "<h2>{}</h2>", gabarit.retour);
    let destination = crate::guard::destination_contact(input.contact).ok_or_else(|| {
        ReportError::invalid_input(
            "destination du contact de retour d'information manquante".to_string(),
        )
    })?;
    let _ = writeln!(out, "<p>{}.</p>", echappe(destination));
    let _ = writeln!(out, "<h2>{}</h2>", gabarit.recours);
    let _ = writeln!(
        out,
        "<p>{}: {}.</p>",
        echappe(pack.recours_nom),
        echappe(pack.recours_contact)
    );
    let _ = writeln!(out, "</main></body></html>");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contact() -> Contact {
        Contact {
            canal: "email".into(),
            email: Some("barrierefreiheit@musterbehoerde.de".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        }
    }

    #[test]
    fn declaration_de_porte_recours_et_statut() {
        let contact = contact();
        let input = DeclarationUeInput {
            pays: Pays::De,
            service: "Musterportal",
            organisme: "Musterbehörde",
            statut: "teilweise barrierefrei",
            non_conformites: &["Kontrast 3,2:1".into()],
            derogations: &[],
            contact: &contact,
            date_declaration: "2026-09-19",
            methode_evaluation: "Selbstbewertung",
        };
        let html = render_declaration_ue(&input).expect("rendu");
        assert!(html.contains("lang=\"de\""));
        assert!(html.contains("Erklärung zur Barrierefreiheit"));
        assert!(html.contains("teilweise barrierefrei"));
        assert!(html.contains("Schlichtungsstelle nach § 16 BGG"));
        assert!(html.contains("schlichtungsstelle-bgg.de"));
        assert!(html.contains("barrierefreiheit@musterbehoerde.de"));
    }

    #[test]
    fn pack_sans_traduction_rend_honetement_anglais() {
        let contact = contact();
        let input = DeclarationUeInput {
            pays: Pays::Pt,
            service: "Portal",
            organisme: "AMA",
            statut: "parcialmente conforme",
            non_conformites: &[],
            derogations: &[],
            contact: &contact,
            date_declaration: "2026-09-19",
            methode_evaluation: "auto",
        };
        let html = render_declaration_ue(&input).expect("rendu");
        assert!(html.contains("lang=\"en\""));
        assert!(html.contains("Accessibility statement"));
    }

    #[test]
    fn statut_vide_bloque() {
        let contact = contact();
        let input = DeclarationUeInput {
            pays: Pays::Fr,
            service: "S",
            organisme: "O",
            statut: "   ",
            non_conformites: &[],
            derogations: &[],
            contact: &contact,
            date_declaration: "2026-09-19",
            methode_evaluation: "auto",
        };
        assert!(render_declaration_ue(&input).is_err());
    }
}
