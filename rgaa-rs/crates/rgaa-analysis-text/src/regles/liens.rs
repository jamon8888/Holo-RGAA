//! 6.1 (lien explicite), 8.6 (titre de page) et 2.2 (titre de cadre).

use std::collections::{HashMap, HashSet};

use crate::llm::{ContexteReduit, ElementLlm};
use crate::mesures::{CadreMesures, LienMesures, Mesure, Mesures, TitrePageMesures};
use crate::similarite::{
    est_generique, est_nom_de_fichier, jaro_winkler, normaliser, similarite, INTITULE_GENERIQUE,
    TITRE_CADRE_GENERIQUE,
};
use crate::snapshot::TextSnapshot;
use indexmap::IndexMap;
use rgaa_analysis::{Indice, Portee};
use serde_json::json;

use super::{sujet_element, sujet_page};

/// 6.1 — intitulé générique sans contexte AX, et signaux de regroupement.
pub(super) fn liens(snapshot: &TextSnapshot) -> Vec<Mesure> {
    let tous = &snapshot.page.liens;
    let mut par_libelle: HashMap<String, HashSet<&str>> = HashMap::new();
    for lien in tous {
        par_libelle
            .entry(normaliser(&lien.intitule))
            .or_default()
            .insert(lien.cible.as_str());
    }

    tous.iter()
        .map(|lien| {
            let libelle = normaliser(&lien.intitule);
            let cibles_differentes = par_libelle.get(&libelle).map_or(0, HashSet::len);
            let generique = est_generique(&lien.intitule, INTITULE_GENERIQUE);
            let contexte_ax = !lien.nom_accessible.is_empty()
                || !lien.description_accessible.is_empty()
                || !lien.titre_liste.is_empty();
            let mesuree = generique && !contexte_ax;
            let similarite_cible = lien
                .titre_cible
                .as_deref()
                .map_or(0.0, |titre| similarite(&lien.intitule, titre));
            let element = ElementLlm::new(
                lien.id.clone(),
                lien.intitule.clone(),
                ContexteReduit::new(
                    lien.titre_liste.clone(),
                    if lien.extrait.is_empty() {
                        &lien.titre_section
                    } else {
                        &lien.extrait
                    },
                    Some(lien.cible.clone()),
                ),
                json!({
                    "generique": generique,
                    "meme_intitule_cibles_differentes": cibles_differentes,
                    "similarite_cible": similarite_cible,
                    "contexte_ax": contexte_ax,
                }),
            );
            let indice = if mesuree || cibles_differentes > 1 || similarite_cible < 0.3 {
                Indice::Suspect {
                    raison: "intitulé générique ou cible non étayée".to_owned(),
                }
            } else {
                Indice::Neutre
            };
            Mesure {
                test: "6.1.1",
                sujet: sujet_element(&lien.id),
                portee: if mesuree {
                    Portee::Mesuree
                } else {
                    Portee::Estimee
                },
                indice: Some(indice),
                donnees: Mesures::Lien(LienMesures {
                    intitule: lien.intitule.clone(),
                    nom_accessible: lien.nom_accessible.clone(),
                    cible: lien.cible.clone(),
                    generique,
                    meme_intitule_cibles_differentes: cibles_differentes,
                    similarite_cible,
                    avis_modele: None,
                }),
                element: Some(element),
            }
        })
        .collect()
}

/// 8.6 — titre vide, répété sur plus de 3 pages, ou nom de fichier.
pub(super) fn titres(snapshot: &TextSnapshot) -> Vec<Mesure> {
    let mut pages: IndexMap<String, &crate::snapshot::PageText> = IndexMap::new();
    if !snapshot.page.url.is_empty() {
        pages.insert(snapshot.page.url.clone(), &snapshot.page);
    }
    for page in &snapshot.pages {
        let cle = if page.url.is_empty() {
            format!("{}#{}", page.titre, pages.len())
        } else {
            page.url.clone()
        };
        pages.entry(cle).or_insert(page);
    }

    let mut occurrences: HashMap<String, usize> = HashMap::new();
    for page in pages.values() {
        *occurrences.entry(normaliser(&page.titre)).or_default() += 1;
    }

    pages
        .iter()
        .map(|(url, page)| {
            let vide = page.titre.trim().is_empty();
            let nom_de_fichier = est_nom_de_fichier(&page.titre);
            let nombre_pages_identiques = occurrences
                .get(&normaliser(&page.titre))
                .copied()
                .unwrap_or(0);
            let mesuree = vide || nom_de_fichier || nombre_pages_identiques > 3;
            let corps = if page.h1.is_empty() {
                page.contenu_principal.as_str()
            } else {
                page.h1.as_str()
            };
            let similarite_contenu = similarite(&page.titre, corps);
            let jaro = jaro_winkler(&page.titre, &page.h1);
            let indice = if mesuree || similarite_contenu < 0.3 {
                Indice::Suspect {
                    raison: "titre vide, répété ou sans rapport avec le contenu".to_owned(),
                }
            } else {
                Indice::Neutre
            };
            let element = (!mesuree).then(|| {
                ElementLlm::new(
                    format!("titre:{url}"),
                    page.titre.clone(),
                    ContexteReduit::new(
                        page.h1.clone(),
                        &page.contenu_principal,
                        Some(url.clone()),
                    ),
                    json!({
                        "similarite_contenu": similarite_contenu,
                        "jaro_h1": jaro,
                    }),
                )
            });
            Mesure {
                test: "8.6.1",
                sujet: sujet_page(url),
                portee: if mesuree {
                    Portee::Mesuree
                } else {
                    Portee::Estimee
                },
                indice: Some(indice),
                donnees: Mesures::TitrePage(TitrePageMesures {
                    url: url.clone(),
                    titre: page.titre.clone(),
                    h1: page.h1.clone(),
                    nombre_pages_identiques,
                    nom_de_fichier,
                    vide,
                    similarite_contenu,
                    jaro_h1: jaro,
                    avis_modele: None,
                }),
                element,
            }
        })
        .collect()
}

/// 2.2 — cadre sans titre ou avec un titre générique.
pub(super) fn cadres(snapshot: &TextSnapshot) -> Vec<Mesure> {
    snapshot
        .page
        .cadres
        .iter()
        .map(|cadre| {
            let titre_generique =
                cadre.titre.trim().is_empty() || est_generique(&cadre.titre, TITRE_CADRE_GENERIQUE);
            let similarite_titre = similarite(&cadre.titre, &cadre.texte_document);
            let element = (!titre_generique).then(|| {
                ElementLlm::new(
                    cadre.id.clone(),
                    cadre.titre.clone(),
                    ContexteReduit::new(
                        cadre.titre_section.clone(),
                        if cadre.extrait.is_empty() {
                            &cadre.texte_document
                        } else {
                            &cadre.extrait
                        },
                        None,
                    ),
                    json!({ "similarite_titre": similarite_titre }),
                )
            });
            Mesure {
                test: "2.2.1",
                sujet: sujet_element(&cadre.id),
                portee: if titre_generique {
                    Portee::Mesuree
                } else {
                    Portee::Estimee
                },
                indice: Some(if titre_generique || similarite_titre < 0.3 {
                    Indice::Suspect {
                        raison: "titre de cadre générique ou sans rapport".to_owned(),
                    }
                } else {
                    Indice::Neutre
                }),
                donnees: Mesures::Cadre(CadreMesures {
                    titre: cadre.titre.clone(),
                    titre_generique,
                    texte_document: cadre.texte_document.clone(),
                    similarite_titre,
                    avis_modele: None,
                }),
                element,
            }
        })
        .collect()
}
