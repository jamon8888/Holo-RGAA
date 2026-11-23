//! Détection de mouvement et recherche d'un contrôle pause (spec §4).
//!
//! La moitié dynamique (screencast CDP et `prefers-reduced-motion`) est une
//! seconde PR : la preuve porte `reduced_motion_respecte: false` et l'indice
//! est calculé sur la seule observation disponible des frames.

use rgaa_analysis::AxTree;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::frames::Frame;

/// Durée au-delà de laquelle une région en mouvement est relevée.
pub const DUREE_MOUVEMENT_S: f64 = 5.0;

/// Une région animée observée, avec son éventuel contrôle pause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Mouvement {
    pub selecteur: String,
    pub duree_s: f64,
    pub controle_pause: bool,
    pub reduced_motion_respecte: bool,
}

/// Un bouton dont le nom invite à pause, arrêt ou masquage est un contrôle.
#[must_use]
pub fn a_controle_pause(ax: Option<&AxTree>) -> bool {
    let Some(ax) = ax else {
        return false;
    };
    ax.nodes.iter().any(|noeud| {
        if !noeud.role.eq_ignore_ascii_case("button") {
            return false;
        }
        let nom = noeud.name.to_lowercase();
        ["pause", "arrêter", "arreter", "arrêt", "arret", "stop"]
            .iter()
            .any(|mot| nom.contains(mot))
    })
}

/// Releve la plus longue séquence de frames consécutives qui diffèrent.
///
/// Les frames sont sous-échantillonnées : un carrousel, un `marquee` ou un GIF
/// qui bouge pendant plus de 5 s produisent un `Mouvement`. La différence
/// absolue moyenne doit dépasser 8/255 pour écarter le bruit de compression.
#[must_use]
pub fn analyse_mouvement(frames: &[Frame], ax: Option<&AxTree>, selecteur: &str) -> Vec<Mouvement> {
    let controle_pause = a_controle_pause(ax);
    let mut debut: Option<usize> = None;
    let mut meilleure: Option<(f64, usize, usize)> = None;

    for index in 1..frames.len() {
        if different(&frames[index - 1], &frames[index]) {
            if debut.is_none() {
                debut = Some(index - 1);
            }
            let (debut_index, fin_index) = (debut.unwrap_or(index - 1), index);
            let duree = frames[fin_index].t_s - frames[debut_index].t_s;
            match meilleure {
                Some((meilleure_duree, _, _)) if meilleure_duree >= duree => {}
                _ => meilleure = Some((duree, debut_index, fin_index)),
            }
        } else if let Some((duree, debut_index, fin_index)) = meilleure {
            if duree > DUREE_MOUVEMENT_S {
                return vec![mouvement(
                    selecteur,
                    duree,
                    controle_pause,
                    debut_index,
                    fin_index,
                )];
            }
            let _ = (debut_index, fin_index);
            debut = None;
            meilleure = None;
        } else {
            debut = None;
        }
    }

    match meilleure {
        Some((duree, debut_index, fin_index)) if duree > DUREE_MOUVEMENT_S => {
            vec![mouvement(
                selecteur,
                duree,
                controle_pause,
                debut_index,
                fin_index,
            )]
        }
        _ => Vec::new(),
    }
}

fn mouvement(
    selecteur: &str,
    duree: f64,
    controle_pause: bool,
    _debut: usize,
    _fin: usize,
) -> Mouvement {
    Mouvement {
        selecteur: selecteur.to_owned(),
        duree_s: duree,
        controle_pause,
        // L'observation dynamique CDP (Emulation.setEmulatedMedia) est hors PR.
        reduced_motion_respecte: false,
    }
}

fn different(a: &Frame, b: &Frame) -> bool {
    let pas = 64usize;
    let octets = a.rgba.len().min(b.rgba.len());
    if octets == 0 {
        return false;
    }
    let mut total: u64 = 0;
    let mut compte = 0u64;
    for index in (0..octets).step_by(pas) {
        total += u64::from(a.rgba[index].abs_diff(b.rgba[index]));
        compte += 1;
    }
    total as f64 / compte as f64 > 8.0
}
