//! Repérage statique des limites de temps (spec §5, critère 13.1).
//!
//! `DomSnapshot::json` est le même document que celui du plugin P-04 : un
//! objet avec un tableau `nodes` en ordre du document, chaque nœud ayant un
//! `tag`, des `attrs` et un `text` optionnel. L'observation dynamique (CDP
//! `Debugger` et recherche d'une boîte « prolonger la session ») est une
//! seconde PR.

use rgaa_analysis::{analysis_error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Une limite de temps relevée dans le document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LimiteTemps {
    #[serde(rename = "type")]
    pub type_: String,
    pub delai_s: f64,
    pub mecanisme_prolongation: bool,
}

#[derive(Debug, Deserialize)]
struct DomJson {
    #[serde(default)]
    nodes: Vec<NodeJson>,
}

#[derive(Debug, Deserialize)]
struct NodeJson {
    tag: String,
    #[serde(default)]
    attrs: BTreeMap<String, String>,
    #[serde(default)]
    text: String,
}

/// Analyse le DOM à la recherche d'un `meta refresh` ou d'un timer qui
/// redirige (`location`, `form.submit`).
///
/// # Errors
/// Renvoie `RgaaError::Analysis` si le snapshot n'a pas la forme documentée.
pub fn detecte_limites(dom_json: &str) -> Result<Vec<LimiteTemps>> {
    let dom: DomJson = serde_json::from_str(dom_json)
        .map_err(|erreur| analysis_error("motion", format!("DOM snapshot illisible : {erreur}")))?;
    let mut limites = Vec::new();
    for noeud in &dom.nodes {
        if noeud.tag.eq_ignore_ascii_case("meta") {
            let refresh = noeud.attrs.iter().any(|(cle, valeur)| {
                cle.eq_ignore_ascii_case("http-equiv") && valeur.eq_ignore_ascii_case("refresh")
            });
            if refresh {
                if let Some(contenu) = noeud
                    .attrs
                    .iter()
                    .find(|(cle, _)| cle.eq_ignore_ascii_case("content"))
                    .map(|(_, valeur)| valeur.as_str())
                {
                    if let Some(delai) = delai_meta(contenu).filter(|delai| *delai > 0.0) {
                        limites.push(LimiteTemps {
                            type_: "meta-refresh".to_owned(),
                            delai_s: delai,
                            mecanisme_prolongation: false,
                        });
                    }
                }
            }
        }
        if noeud.tag.eq_ignore_ascii_case("script") {
            let texte = noeud.text.to_lowercase();
            for (mot_cle, nom) in [
                ("settimeout", "timer-redirection"),
                ("setinterval", "timer-redirection"),
            ] {
                if !texte.contains(mot_cle) {
                    continue;
                }
                if !(texte.contains("location") || texte.contains(".submit")) {
                    continue;
                }
                if let Some(ms) = delai_timer(&noeud.text, mot_cle) {
                    if ms > 0.0 {
                        limites.push(LimiteTemps {
                            type_: nom.to_owned(),
                            delai_s: ms / 1000.0,
                            mecanisme_prolongation: false,
                        });
                    }
                }
            }
        }
    }
    Ok(limites)
}

fn delai_meta(contenu: &str) -> Option<f64> {
    contenu
        .split(';')
        .next()
        .map(str::trim)
        .and_then(|valeur| valeur.parse::<f64>().ok())
        .filter(|valeur| valeur.is_finite() && *valeur >= 0.0)
}

/// Prend le dernier argument numérique de `setTimeout(...)` (millisecondes).
/// La parenthèse est appariée par profondeur : un appel imbriqué comme
/// `setTimeout(function(){location.assign('x')}, 5000)` ne doit pas être coupé
/// à la première `)`.
fn delai_timer(texte: &str, mot_cle: &str) -> Option<f64> {
    let debut = texte.to_lowercase().find(mot_cle)? + mot_cle.len();
    let reste = texte.get(debut..)?.trim_start();
    let apres_parenthere = reste.strip_prefix('(')?;
    let mut profondeur = 1u32;
    let mut fin = None;
    for (decalage, caractere) in apres_parenthere.char_indices() {
        match caractere {
            '(' => profondeur += 1,
            ')' => {
                profondeur -= 1;
                if profondeur == 0 {
                    fin = Some(decalage);
                    break;
                }
            }
            _ => {}
        }
    }
    let args = apres_parenthere.get(..fin?)?;
    args.split(',')
        .filter_map(|argument| argument.trim().parse::<f64>().ok())
        .next_back()
}
