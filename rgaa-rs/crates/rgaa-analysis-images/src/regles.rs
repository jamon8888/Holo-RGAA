//! Règles déterministes P-03 §3 : aucune modèle, preuve `Mesuree`.
//!
//! Les libellés sont ceux de la spec : `alt-nom-fichier`, `alt-generique`,
//! `alt-trop-long`, `alt-redondant`.

use crate::inventory::ImageNode;

/// Un alt de **plus de** 150 caractères suggère une description détaillée
/// manquante (critère 1.7).
pub const ALT_MAX_CARACTERES: usize = 150;

/// Normalise pour comparer un alt et un nom de fichier : casse, séparateurs,
/// extension et suffixe de version (`-v2`).
fn normaliser(texte: &str) -> String {
    let texte = texte.trim().to_lowercase();
    let sans_extension = texte
        .rsplit_once('.')
        .filter(|(_, ext)| ["jpg", "jpeg", "png", "gif", "webp", "svg", "bmp"].contains(ext))
        .map_or(texte.as_str(), |(stem, _)| stem);
    let substitue = sans_extension.replace(['-', '_', '.', '/', '\\'], " ");
    let mots: Vec<&str> = substitue
        .split_whitespace()
        .filter(|mot| !est_version(mot))
        .collect();
    mots.join(" ")
}

fn est_version(mot: &str) -> bool {
    mot.starts_with('v') && mot.len() > 1 && mot[1..].chars().all(|c| c.is_ascii_digit())
}

fn nom_de_fichier(src: &str) -> &str {
    let nom = src.rsplit(['/', '\\']).next().unwrap_or(src);
    nom.rsplit_once('.')
        .filter(|(_, ext)| ["jpg", "jpeg", "png", "gif", "webp", "svg", "bmp"].contains(ext))
        .map_or(nom, |(stem, _)| stem)
}

/// Vrai quand l'alt n'est que le nom de fichier (`IMG_2034.jpg`,
/// `banner-final-v2`).
#[must_use]
pub fn est_alt_nom_fichier(alt: &str, src: &str) -> bool {
    let alt = normaliser(alt);
    let src = normaliser(nom_de_fichier(src));
    !alt.is_empty() && alt == src
}

/// Vrai quand l'alt est composé uniquement de « image », « photo », « logo »,
/// éventuellement précédés d'un article.
#[must_use]
pub fn est_alt_generique(alt: &str) -> bool {
    let alt = normaliser(alt).replace('’', "'");
    let mots: Vec<&str> = alt
        .split_whitespace()
        .filter(|mot| {
            ![
                "un", "une", "le", "la", "les", "l", "l'", "d'", "de", "des", "du",
            ]
            .contains(mot)
        })
        .collect();
    !mots.is_empty()
        && mots
            .iter()
            .all(|mot| ["image", "photo", "logo"].contains(mot))
}

/// Vrai au-delà de 150 caractères (la limite elle-même est acceptable).
#[must_use]
pub fn est_alt_trop_long(alt: &str) -> bool {
    alt.chars().count() > ALT_MAX_CARACTERES
}

/// Vrai quand l'alt reprend le texte du lien qui l'entoure.
#[must_use]
pub fn est_alt_redondant(alt: &str, texte_lien: Option<&str>) -> bool {
    let Some(texte_lien) = texte_lien else {
        return false;
    };
    let alt = normaliser(alt);
    let lien = normaliser(texte_lien);
    !alt.is_empty() && alt == lien
}

/// Applique les quatre règles dans l'ordre de la spec, sans doublon.
#[must_use]
pub fn appliquer(node: &ImageNode) -> Vec<&'static str> {
    let alt = node
        .alt
        .as_deref()
        .unwrap_or_else(|| node.nom_accessible.as_deref().unwrap_or(""));
    let src = node.src.as_deref().unwrap_or("");
    let mut regles = Vec::with_capacity(4);
    if est_alt_nom_fichier(alt, src) {
        regles.push("alt-nom-fichier");
    }
    if est_alt_generique(alt) {
        regles.push("alt-generique");
    }
    if est_alt_trop_long(alt) {
        regles.push("alt-trop-long");
    }
    if est_alt_redondant(alt, node.texte_lien.as_deref()) {
        regles.push("alt-redondant");
    }
    regles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_alt_fires_no_rule() {
        let node = ImageNode::default();
        assert!(appliquer(&node).is_empty());
    }
}
