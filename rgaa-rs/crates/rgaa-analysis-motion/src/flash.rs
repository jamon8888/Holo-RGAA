//! Comptage des flashs généraux et rouges (spec §3, WCAG 2.3.1).
//!
//! La transition n'est comptée qu'à une **inversion d'extremum** d'au moins
//! 0,10 de luminance (ou 20 de mesure rouge) : un fondu lent n'est pas un
//! flash, même s'il parcourt toute la plage de luminance. Un flash est une
//! paire changement sombre → clair précédée d'un changement clair → sombre.
//! Le comptage se fait dans une fenêtre spatiale glissante de 341×256 px (pas
//! de 64 px) sur une image normalisée à 1024×768, et dans une fenêtre
//! temporelle d'une seconde.

use crate::frames::Frame;
use crate::luminance::{red_metric, relative_luminance};
use crate::{
    DELTA_LUMINANCE, DELTA_ROUGE, FENETRE_HAUTEUR, FENETRE_LARGEUR, HAUTEUR_REFERENCE,
    LARGEUR_REFERENCE, LUMINANCE_SOMBRE_MAX, PAS_FENETRE,
};
use image::imageops::FilterType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Type de flash, tel qu'il figure dans la preuve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum FlashType {
    General,
    Rouge,
}

/// Résultat pour un type de flash : pic par seconde et fenêtre concernée.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FlashResult {
    pub type_: FlashType,
    pub max_par_s: u32,
    pub a_s: f64,
    /// `[x, y, largeur, hauteur]` dans l'image normalisée.
    pub fenetre: [u32; 4],
    /// Fraction de la fenêtre qui dépasse 3 flashs/s.
    pub surface: f64,
}

impl Default for FlashResult {
    fn default() -> Self {
        Self {
            type_: FlashType::General,
            max_par_s: 0,
            a_s: 0.0,
            fenetre: [0, 0, FENETRE_LARGEUR, FENETRE_HAUTEUR],
            surface: 0.0,
        }
    }
}

/// Durée de la fenêtre temporelle, en centièmes de seconde.
const FENETRE_MS: u16 = 100;
/// Un pixel « touché » a strictement plus de trois flashs dans la seconde.
const FLASHORS_SEUIL: u32 = 4;
/// Bornes de l'historique par pixel ; 30 i/s donnent au plus 15 flashs/s.
const HISTORIQUE: usize = 16;

#[derive(Clone, Copy)]
struct EtatPixel {
    extreme: f32,
    direction: i8,
    a_un_changement_sombre: bool,
    flashs: [u16; HISTORIQUE],
    nombre: u8,
}

impl Default for EtatPixel {
    fn default() -> Self {
        Self {
            extreme: 0.0,
            direction: 0,
            a_un_changement_sombre: false,
            flashs: [0; HISTORIQUE],
            nombre: 0,
        }
    }
}

impl EtatPixel {
    /// Met à jour l'extremum et enregistre un flash sur une inversion claire.
    fn met_a_jour(&mut self, valeur: f32, maintenant: u16, seuil: f32, sombre_max: f32) {
        if self.direction == 0 {
            if (valeur - self.extreme).abs() < f32::EPSILON {
                return;
            }
            self.direction = if valeur > self.extreme { 1 } else { -1 };
            self.extreme = valeur;
            return;
        }

        if self.direction > 0 {
            if valeur >= self.extreme {
                self.extreme = valeur;
                return;
            }
            let delta = self.extreme - valeur;
            if delta >= seuil && valeur.min(self.extreme) < sombre_max {
                self.direction = -1;
                self.extreme = valeur;
                self.a_un_changement_sombre = true;
            }
            return;
        }

        if valeur <= self.extreme {
            self.extreme = valeur;
            return;
        }
        let delta = valeur - self.extreme;
        if delta >= seuil && valeur.min(self.extreme) < sombre_max {
            self.direction = 1;
            self.extreme = valeur;
            if self.a_un_changement_sombre {
                self.a_un_changement_sombre = false;
                self.enregistre_flash(maintenant);
            }
        }
    }

    fn enregistre_flash(&mut self, maintenant: u16) {
        if self.nombre as usize == HISTORIQUE {
            self.flashs.rotate_left(1);
            self.flashs[HISTORIQUE - 1] = maintenant;
        } else {
            self.flashs[usize::from(self.nombre)] = maintenant;
            self.nombre += 1;
        }
    }

    /// Nombre de flashs dans `[maintenant - 1 s, maintenant[`.
    fn compte(&self, maintenant: u16) -> u32 {
        let debut = maintenant.saturating_sub(FENETRE_MS);
        self.flashs[..usize::from(self.nombre)]
            .iter()
            .filter(|&&instant| instant >= debut && instant < maintenant)
            .count() as u32
    }
}

/// Détecte les deux types de flash sur une séquence de frames horodatées.
#[must_use]
pub fn detect_flashs(frames: &[Frame]) -> (FlashResult, FlashResult) {
    (
        detect(frames, FlashType::General),
        detect(frames, FlashType::Rouge),
    )
}

fn detect(frames: &[Frame], type_: FlashType) -> FlashResult {
    let (seuil, sombre_max) = match type_ {
        FlashType::General => (DELTA_LUMINANCE, LUMINANCE_SOMBRE_MAX),
        FlashType::Rouge => (DELTA_ROUGE, f32::INFINITY),
    };
    let nombre_pixels = (LARGEUR_REFERENCE * HAUTEUR_REFERENCE) as usize;
    let mut etats: Vec<EtatPixel> = Vec::with_capacity(nombre_pixels);
    let mut resultat = FlashResult {
        type_,
        ..FlashResult::default()
    };

    for frame in frames {
        let pixels = normalise(frame);
        let maintenant = (frame.t_s * 100.0).round().max(0.0) as u16;
        if etats.is_empty() {
            etats.resize(nombre_pixels, EtatPixel::default());
        }
        let mut pic = 0u32;
        for (etat, pixel) in etats.iter_mut().zip(pixels.chunks(4)) {
            let valeur = match type_ {
                FlashType::General => relative_luminance(pixel[0], pixel[1], pixel[2]),
                FlashType::Rouge => red_metric(pixel[0], pixel[1], pixel[2]),
            };
            etat.met_a_jour(valeur, maintenant, seuil, sombre_max);
            let compte = etat.compte(maintenant);
            if compte > pic {
                pic = compte;
            }
        }
        if pic > resultat.max_par_s {
            resultat.max_par_s = pic;
            resultat.a_s = frame.t_s;
        }
        if pic < FLASHORS_SEUIL {
            continue;
        }

        let (surface, fenetre) = surface_touchee(&etats, maintenant);
        if surface > resultat.surface {
            resultat.surface = surface;
            resultat.fenetre = fenetre;
        }
    }
    resultat
}

/// Surface maximale, sur toutes les fenêtres 341×256, des pixels qui
/// franchissent le seuil de trois flashs par seconde.
fn surface_touchee(etats: &[EtatPixel], maintenant: u16) -> (f64, [u32; 4]) {
    const LARGE: u32 = LARGEUR_REFERENCE + 1;
    let mut integral = vec![0u32; LARGE as usize * (HAUTEUR_REFERENCE as usize + 1)];
    for y in 0..HAUTEUR_REFERENCE {
        let mut somme_ligne = 0u32;
        let ligne = (y + 1) as usize * LARGE as usize;
        let ligne_precedente = y as usize * LARGE as usize;
        for x in 0..LARGEUR_REFERENCE {
            let touche = u32::from(
                etats[(y * LARGEUR_REFERENCE + x) as usize].compte(maintenant) >= FLASHORS_SEUIL,
            );
            somme_ligne += touche;
            integral[ligne + x as usize + 1] =
                integral[ligne_precedente + x as usize + 1] + somme_ligne;
        }
    }

    let aire = FENETRE_LARGEUR * FENETRE_HAUTEUR;
    let mut meilleure = 0.0;
    let mut fenetre = [0, 0, FENETRE_LARGEUR, FENETRE_HAUTEUR];
    for y in decalages(HAUTEUR_REFERENCE, FENETRE_HAUTEUR) {
        for x in decalages(LARGEUR_REFERENCE, FENETRE_LARGEUR) {
            let bas = (y + FENETRE_HAUTEUR) as usize * LARGE as usize;
            let haut = y as usize * LARGE as usize;
            let droite = (x + FENETRE_LARGEUR) as usize;
            let gauche = x as usize;
            let colonne_droite = integral[bas + droite] - integral[haut + droite];
            let colonne_gauche = integral[bas + gauche] - integral[haut + gauche];
            let touche = colonne_droite - colonne_gauche;
            let fraction = touche as f64 / aire as f64;
            if fraction > meilleure {
                meilleure = fraction;
                fenetre = [x, y, FENETRE_LARGEUR, FENETRE_HAUTEUR];
            }
        }
    }
    (meilleure, fenetre)
}

fn decalages(etendue: u32, fenetre: u32) -> Vec<u32> {
    let dernier = etendue - fenetre;
    let mut decalages: Vec<u32> = (0..=dernier).step_by(PAS_FENETRE as usize).collect();
    if decalages.last() != Some(&dernier) {
        decalages.push(dernier);
    }
    decalages
}

/// Normalise une frame à la taille de référence (plus proche voisin : on ne
/// fabrique pas de luminance intermédiaire).
fn normalise(frame: &Frame) -> Vec<u8> {
    let image = match image::RgbaImage::from_raw(frame.largeur, frame.hauteur, frame.rgba.clone()) {
        Some(image) => image,
        None => return Vec::new(),
    };
    image::imageops::resize(
        &image,
        LARGEUR_REFERENCE,
        HAUTEUR_REFERENCE,
        FilterType::Nearest,
    )
    .into_raw()
}
