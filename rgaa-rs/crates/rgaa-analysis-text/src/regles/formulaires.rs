//! 11.2, 11.13, 11.9, 11.10, 11.11, 11.12 et 11.7.

use crate::llm::{ContexteReduit, ElementLlm};
use crate::mesures::{
    BoutonMesures, ChampMesures, FormulaireMesures, LegendeMesures, Mesure, Mesures,
};
use crate::similarite::{
    autocomplete_ecart, est_message_generique, est_symbole_seul, finalite_attendue, similarite,
};
use crate::snapshot::{Formulaire, TextSnapshot};
use rgaa_analysis::{action_interdite, Result};
use serde_json::json;

use super::sujet_element;

/// Observation sans effet de bord d'une sonde de formulaire.
///
/// P-08 **ne soumet jamais** : la sonde ne peut que rapporter ce qui a déjà été
/// observé (messages d'erreur, présence d'une page récapitulative). L'action
/// interdite est détectée par `rgaa_analysis::actions`, liste partagée avec
/// P-07 et la règle F-SEC.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormObservation {
    pub messages_erreur: Vec<String>,
    pub recapitulatif_detecte: Option<bool>,
}

/// Point d'extension pour une future sonde sans effet de bord (aucune dans ce ticket).
pub trait FormProbe: Send + Sync {
    /// Observe un formulaire sans rien exécuter.
    ///
    /// # Errors
    ///
    /// [`rgaa_analysis::RgaaError::Analysis`] si l'observation est illisible ;
    /// l'audit continue avec les seules données capturées.
    fn observe(&self, formulaire: &Formulaire) -> Result<FormObservation>;
}

/// Sonde par défaut : ne rapporte rien, ne touche à rien.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopFormProbe;

impl FormProbe for NoopFormProbe {
    fn observe(&self, _formulaire: &Formulaire) -> Result<FormObservation> {
        Ok(FormObservation::default())
    }
}

/// 11.2 (étiquette) et 11.13 (finalité), puis signaux de 11.10/11.11.
pub(super) fn champs(snapshot: &TextSnapshot) -> Vec<Mesure> {
    let mut mesures = Vec::new();
    for champ in &snapshot.page.champs {
        let etiquette_vide = champ.etiquette.trim().is_empty();
        let etiquette_egale_placeholder =
            !champ.placeholder.is_empty() && champ.etiquette.trim() == champ.placeholder.trim();
        let symbole_seul = est_symbole_seul(champ.etiquette.trim());
        let deterministe_11_2 = etiquette_vide || etiquette_egale_placeholder || symbole_seul;
        let attendu = finalite_attendue(champ);
        let ecart = autocomplete_ecart(champ);
        let ecart_finalite = ecart.is_some();
        let requis_sans_indication =
            champ.requis && !champ.indication_requise_visible && !champ.indication_requise_ax;
        let sans_exemple = champ.format_attendu.is_some() && champ.exemple_format.is_none();

        let communes = || ChampMesures {
            identifiant: champ.id.clone(),
            etiquette: champ.etiquette.clone(),
            placeholder: champ.placeholder.clone(),
            autocomplete: champ.autocomplete.clone(),
            autocomplete_attendu: attendu.map(str::to_owned),
            ecart_finalite,
            etiquette_vide,
            etiquette_egale_placeholder,
            symbole_seul,
            format_attendu: champ.format_attendu.clone(),
            exemple_format: champ.exemple_format.clone(),
            requis_sans_indication,
            avis_modele: None,
        };

        if deterministe_11_2 {
            mesures.push(Mesure {
                test: "11.2.1",
                sujet: sujet_element(&champ.id),
                portee: rgaa_analysis::Portee::Mesuree,
                indice: Some(rgaa_analysis::Indice::Suspect {
                    raison: "étiquette vide, égale au placeholder ou réduite à un symbole"
                        .to_owned(),
                }),
                donnees: Mesures::Champ(communes()),
                element: None,
            });
        }
        if ecart_finalite || attendu.is_some() {
            let element = ElementLlm::new(
                format!("finalite:{}", champ.id),
                champ.etiquette.clone(),
                ContexteReduit::new(champ.titre_section.clone(), &champ.extrait, None),
                json!({
                    "autocomplete": champ.autocomplete,
                    "autocomplete_attendu": attendu,
                    "ecart_finalite": ecart_finalite,
                }),
            );
            mesures.push(Mesure {
                test: "11.13.1",
                sujet: sujet_element(&champ.id),
                portee: rgaa_analysis::Portee::Estimee,
                indice: Some(if ecart_finalite {
                    rgaa_analysis::Indice::Suspect {
                        raison: format!(
                            "finalité attendue {} mais autocomplete {:?}",
                            ecart.unwrap_or_default(),
                            champ.autocomplete
                        ),
                    }
                } else {
                    rgaa_analysis::Indice::Neutre
                }),
                donnees: Mesures::Champ(communes()),
                element: Some(element),
            });
        }
        if requis_sans_indication {
            mesures.push(Mesure {
                test: "11.10.1",
                sujet: sujet_element(&champ.id),
                portee: rgaa_analysis::Portee::Mesuree,
                indice: Some(rgaa_analysis::Indice::Suspect {
                    raison: "champ requis sans indication visible ni accessibilité".to_owned(),
                }),
                donnees: Mesures::Champ(communes()),
                element: None,
            });
        }
        if sans_exemple {
            mesures.push(Mesure {
                test: "11.11.1",
                sujet: sujet_element(&champ.id),
                portee: rgaa_analysis::Portee::Mesuree,
                indice: Some(rgaa_analysis::Indice::Suspect {
                    raison: "format attendu sans exemple".to_owned(),
                }),
                donnees: Mesures::Champ(communes()),
                element: None,
            });
        }
    }
    mesures
}

/// 11.7 — groupe de champs sans légende : cas restant confié au LLM.
pub(super) fn legendes(snapshot: &TextSnapshot) -> Vec<Mesure> {
    snapshot
        .page
        .champs
        .iter()
        .filter_map(|champ| {
            let groupe = champ.groupe.as_deref()?;
            if groupe.is_empty() || champ.legende.as_deref().is_some_and(|l| !l.is_empty()) {
                return None;
            }
            let element = ElementLlm::new(
                format!("legende:{}", champ.id),
                groupe.to_owned(),
                ContexteReduit::new(champ.titre_section.clone(), &champ.extrait, None),
                json!({ "champ": champ.etiquette }),
            );
            Some(Mesure {
                test: "11.7.1",
                sujet: sujet_element(&champ.id),
                portee: rgaa_analysis::Portee::Estimee,
                indice: Some(rgaa_analysis::Indice::Suspect {
                    raison: "groupe de champs sans légende".to_owned(),
                }),
                donnees: Mesures::Legende(LegendeMesures {
                    identifiant: champ.id.clone(),
                    groupe: groupe.to_owned(),
                    legende: String::new(),
                    avis_modele: None,
                }),
                element: Some(element),
            })
        })
        .collect()
}

/// 11.9 — bouton sans nom ou nom réduit à une icône.
pub(super) fn boutons(snapshot: &TextSnapshot) -> Vec<Mesure> {
    snapshot
        .page
        .boutons
        .iter()
        .map(|bouton| {
            let nom_vide = bouton.nom.trim().is_empty();
            let icone_seule = bouton.icone_seule || est_symbole_seul(bouton.nom.trim());
            let mesuree = nom_vide || icone_seule;
            let action = if bouton.texte_proche.is_empty() {
                bouton.formulaire.as_deref().unwrap_or_default()
            } else {
                &bouton.texte_proche
            };
            let similarite_action = similarite(&bouton.nom, action);
            let element = (!mesuree).then(|| {
                ElementLlm::new(
                    bouton.id.clone(),
                    bouton.nom.clone(),
                    ContexteReduit::new(bouton.titre_section.clone(), &bouton.extrait, None),
                    json!({
                        "action_attendue": action,
                        "similarite_action": similarite_action,
                    }),
                )
            });
            Mesure {
                test: "11.9.1",
                sujet: sujet_element(&bouton.id),
                portee: if mesuree {
                    rgaa_analysis::Portee::Mesuree
                } else {
                    rgaa_analysis::Portee::Estimee
                },
                indice: Some(if mesuree || similarite_action < 0.3 {
                    rgaa_analysis::Indice::Suspect {
                        raison: "intitulé de bouton absent ou sans rapport avec l'action"
                            .to_owned(),
                    }
                } else {
                    rgaa_analysis::Indice::Neutre
                }),
                donnees: Mesures::Bouton(BoutonMesures {
                    identifiant: bouton.id.clone(),
                    nom: bouton.nom.clone(),
                    nom_vide,
                    icone_seule,
                    action_attendue: action.to_owned(),
                    similarite_action,
                    avis_modele: None,
                }),
                element,
            }
        })
        .collect()
}

/// 11.10 (contrôle de saisie), 11.11 (suggestion) et 11.12 (confirmation).
///
/// Aucune soumission n'est effectuée : `soumission_effectuee` vaut toujours
/// `false`, y compris pour un formulaire sensible.
pub(super) fn formulaires(snapshot: &TextSnapshot) -> Vec<Mesure> {
    let mut mesures = Vec::new();
    for formulaire in &snapshot.page.formulaires {
        let noms_boutons: Vec<&str> = snapshot
            .page
            .boutons
            .iter()
            .filter(|bouton| bouton.formulaire.as_deref() == Some(formulaire.id.as_str()))
            .map(|bouton| bouton.nom.as_str())
            .collect();
        let mut mots_declencheurs = Vec::new();
        for texte in std::iter::once(formulaire.intitule.as_str()).chain(noms_boutons) {
            if let Some(mot) = action_interdite(texte) {
                if !mots_declencheurs.iter().any(|deja: &String| deja == mot) {
                    mots_declencheurs.push(mot.to_owned());
                }
            }
        }
        let sensible = !mots_declencheurs.is_empty();
        let messages_generiques: Vec<String> = formulaire
            .messages_erreur
            .iter()
            .filter(|message| est_message_generique(message))
            .cloned()
            .collect();

        if !messages_generiques.is_empty() {
            let element = ElementLlm::new(
                format!("messages:{}", formulaire.id),
                formulaire.intitule.clone(),
                ContexteReduit::new(formulaire.titre_section.clone(), &formulaire.extrait, None),
                json!({ "messages_generiques": messages_generiques }),
            );
            mesures.push(Mesure {
                test: "11.11.1",
                sujet: sujet_element(&formulaire.id),
                portee: rgaa_analysis::Portee::Estimee,
                indice: Some(rgaa_analysis::Indice::Suspect {
                    raison: "message d'erreur générique ou sans suggestion".to_owned(),
                }),
                donnees: Mesures::Formulaire(communes(
                    formulaire,
                    sensible,
                    mots_declencheurs.clone(),
                    messages_generiques,
                )),
                element: Some(element),
            });
        }

        if sensible {
            let element = ElementLlm::new(
                format!("formulaire:{}", formulaire.id),
                formulaire.intitule.clone(),
                ContexteReduit::new(formulaire.titre_section.clone(), &formulaire.extrait, None),
                json!({
                    "mots_declencheurs": mots_declencheurs,
                    "recapitulatif_observe": formulaire.recapitulatif_detecte,
                }),
            );
            mesures.push(Mesure {
                test: "11.12.1",
                sujet: sujet_element(&formulaire.id),
                portee: rgaa_analysis::Portee::Mesuree,
                indice: Some(rgaa_analysis::Indice::Suspect {
                    raison: format!(
                        "formulaire à finalité sensible : {}",
                        mots_declencheurs.join(", ")
                    ),
                }),
                donnees: Mesures::Formulaire(communes(
                    formulaire,
                    sensible,
                    mots_declencheurs,
                    Vec::new(),
                )),
                element: Some(element),
            });
        }
    }
    mesures
}

fn communes(
    formulaire: &Formulaire,
    sensible: bool,
    mots_declencheurs: Vec<String>,
    messages_generiques: Vec<String>,
) -> FormulaireMesures {
    FormulaireMesures {
        identifiant: formulaire.id.clone(),
        intitule: formulaire.intitule.clone(),
        sensible,
        mots_declencheurs,
        messages_erreur: formulaire.messages_erreur.clone(),
        messages_generiques,
        recapitulatif_observe: formulaire.recapitulatif_detecte,
        soumission_effectuee: false,
        avis_modele: None,
    }
}
