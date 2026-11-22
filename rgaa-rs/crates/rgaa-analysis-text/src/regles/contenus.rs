//! 5.2/5.5 (tableaux), 12.3 (plan), 13.4 (version alternative) et
//! 1.5/7.2/13.6 (alternatives absentes ou à regrouper).

use crate::llm::{ContexteReduit, ElementLlm};
use crate::mesures::{
    AlternativeMesures, Mesure, Mesures, PlanSiteMesures, TableauMesures, VersionAlternativeMesures,
};
use crate::similarite::{couverture, normaliser, similarite};
use crate::snapshot::TextSnapshot;
use rgaa_analysis::{Indice, Portee};
use serde_json::json;

use super::{sujet_element, sujet_page};

/// 5.2 (résumé d'un tableau complexe) et 5.5 (titre du tableau).
pub(super) fn tableaux(snapshot: &TextSnapshot) -> Vec<Mesure> {
    let mut mesures = Vec::new();
    for tableau in &snapshot.page.tableaux {
        let resume_present = !tableau.resume.trim().is_empty();
        let caption_present = !tableau.caption.trim().is_empty();
        let contenu = if tableau.en_tetes.is_empty() {
            tableau.contenu.clone()
        } else {
            format!("{} {}", tableau.en_tetes.join(" "), tableau.contenu)
        };
        let similarite_caption = similarite(&tableau.caption, &contenu);
        let commune = || TableauMesures {
            identifiant: tableau.id.clone(),
            complexe: tableau.complexe,
            resume_present,
            caption_present,
            similarite_caption,
            avis_modele: None,
        };

        if tableau.complexe && !resume_present {
            mesures.push(Mesure {
                test: "5.2.1",
                sujet: sujet_element(&tableau.id),
                portee: Portee::Mesuree,
                indice: Some(Indice::Suspect {
                    raison: "tableau complexe sans résumé".to_owned(),
                }),
                donnees: Mesures::Tableau(commune()),
                element: None,
            });
        }
        if !caption_present {
            mesures.push(Mesure {
                test: "5.5.1",
                sujet: sujet_element(&tableau.id),
                portee: Portee::Mesuree,
                indice: Some(Indice::Suspect {
                    raison: "tableau sans titre (caption)".to_owned(),
                }),
                donnees: Mesures::Tableau(commune()),
                element: None,
            });
        }
        if caption_present && similarite_caption < 0.3 {
            let element = ElementLlm::new(
                format!("caption:{}", tableau.id),
                tableau.caption.clone(),
                ContexteReduit::new(tableau.titre_section.clone(), &tableau.extrait, None),
                json!({ "similarite_caption": similarite_caption }),
            );
            mesures.push(Mesure {
                test: "5.5.1",
                sujet: sujet_element(&tableau.id),
                portee: Portee::Estimee,
                indice: Some(Indice::Suspect {
                    raison: "titre sans rapport avec le contenu du tableau".to_owned(),
                }),
                donnees: Mesures::Tableau(commune()),
                element: Some(element),
            });
        }
    }
    mesures
}

/// 12.3 — plan absent (mesuré) puis couverture par les URL du spider (estimée).
pub(super) fn plan_du_site(snapshot: &TextSnapshot) -> Vec<Mesure> {
    let Some(plan) = &snapshot.plan_du_site else {
        return vec![Mesure {
            test: "12.3.1",
            sujet: sujet_page(&snapshot.page.url),
            portee: Portee::Mesuree,
            indice: Some(Indice::Suspect {
                raison: "plan du site absent".to_owned(),
            }),
            donnees: Mesures::PlanSite(PlanSiteMesures {
                present: false,
                couverture: 0.0,
                urls_manquantes: snapshot.urls_spider.clone(),
                avis_modele: None,
            }),
            element: None,
        }];
    };

    let liens: Vec<String> = plan.liens.iter().map(|lien| normaliser(lien)).collect();
    let urls: Vec<String> = snapshot
        .urls_spider
        .iter()
        .map(|url| normaliser(url))
        .collect();
    let manquantes: Vec<String> = snapshot
        .urls_spider
        .iter()
        .zip(urls.iter())
        .filter(|(_, url)| !liens.contains(url))
        .map(|(url, _)| url.clone())
        .collect();
    let couverture = if urls.is_empty() {
        0.0
    } else {
        (urls.len() - manquantes.len()) as f64 / urls.len() as f64
    };
    let element = ElementLlm::new(
        "plan-du-site".to_owned(),
        plan.url.clone(),
        ContexteReduit::new(
            "Plan du site".to_owned(),
            &format!(
                "{} URL découvertes, {} présentes",
                urls.len(),
                urls.len() - manquantes.len()
            ),
            Some(plan.url.clone()),
        ),
        json!({ "couverture": couverture, "urls_manquantes": manquantes }),
    );
    vec![Mesure {
        test: "12.3.1",
        sujet: sujet_page(&plan.url),
        portee: Portee::Estimee,
        indice: Some(if couverture < 1.0 {
            Indice::Suspect {
                raison: format!("plan incomplet : {:.0} % de couverture", couverture * 100.0),
            }
        } else {
            Indice::Neutre
        }),
        donnees: Mesures::PlanSite(PlanSiteMesures {
            present: true,
            couverture,
            urls_manquantes: manquantes,
            avis_modele: None,
        }),
        element: Some(element),
    }]
}

/// 13.4 — similarité et couverture de la version alternative (P-04 §3.4).
pub(super) fn versions_alternatives(snapshot: &TextSnapshot) -> Vec<Mesure> {
    snapshot
        .page
        .versions
        .iter()
        .map(|version| {
            let similarite_texte = similarite(&version.original, &version.alternative);
            let taux_couverture = couverture(&version.original, &version.alternative);
            let element = ElementLlm::new(
                version.id.clone(),
                version.titre.clone(),
                ContexteReduit::new("Version alternative".to_owned(), &version.alternative, None),
                json!({
                    "similarite": similarite_texte,
                    "couverture": taux_couverture,
                }),
            );
            Mesure {
                test: "13.4.1",
                sujet: sujet_element(&version.id),
                portee: Portee::Estimee,
                indice: Some(if similarite_texte < 0.8 || taux_couverture < 0.8 {
                    Indice::Suspect {
                        raison: "version alternative incomplète ou dissemblable".to_owned(),
                    }
                } else {
                    Indice::Rassurant {
                        raison: "version alternative couvrant le texte original".to_owned(),
                    }
                }),
                donnees: Mesures::VersionAlternative(VersionAlternativeMesures {
                    identifiant: version.id.clone(),
                    titre: version.titre.clone(),
                    similarite: similarite_texte,
                    couverture: taux_couverture,
                    avis_modele: None,
                }),
                element: Some(element),
            }
        })
        .collect()
}

/// 1.5, 7.2, 13.6 — alternative absente (mesuré), sinon regroupement avec
/// contexte pour le LLM.
pub(super) fn alternatives(snapshot: &TextSnapshot) -> Vec<Mesure> {
    crate::llm::CRITERES_ALTERNATIVES
        .iter()
        .flat_map(|critere| {
            snapshot
                .page
                .alternatives
                .iter()
                .filter(|alternative| alternative.critere == *critere)
                .map(|alternative| {
                    let test = match *critere {
                        "1.5" => "1.5.1",
                        "7.2" => "7.2.1",
                        _ => "13.6.1",
                    };
                    let element = (!alternative.present).then(|| {
                        ElementLlm::new(
                            alternative.id.clone(),
                            alternative.intitule.clone(),
                            ContexteReduit::new(
                                "Alternative".to_owned(),
                                &alternative.contexte,
                                None,
                            ),
                            json!({ "present": alternative.present }),
                        )
                    });
                    Mesure {
                        test,
                        sujet: sujet_element(&alternative.id),
                        portee: if alternative.present {
                            Portee::Estimee
                        } else {
                            Portee::Mesuree
                        },
                        indice: Some(if alternative.present {
                            Indice::Neutre
                        } else {
                            Indice::Suspect {
                                raison: "alternative absente".to_owned(),
                            }
                        }),
                        donnees: Mesures::Alternative(AlternativeMesures {
                            identifiant: alternative.id.clone(),
                            critere: (*critere).to_owned(),
                            present: alternative.present,
                            intitule: alternative.intitule.clone(),
                            contexte: alternative.contexte.clone(),
                            avis_modele: None,
                        }),
                        element,
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
