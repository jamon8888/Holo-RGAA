//! Parseur pur du journal Speech Logger (recherche §1.2).
//!
//! Format documenté par l'add-on : un enregistrement par séquence de parole,
//! séparateur configurable (deux espaces par défaut), marqueurs de début/fin
//! de session `### Log started on … ###` / `### Log ended on … ###`, UTF-8.
//! Le parseur ne touche ni au système de fichiers ni à l'horloge.

/// Journal analysé : énoncés dans l'ordre d'apparition et environnement
/// éventuellement déclaré dans un en-tête `# environnement: …`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    pub enonces: Vec<String>,
    pub environnement: Option<String>,
}

/// Analyse un contenu de journal. `separateur` est la chaîne insérée par
/// Speech Logger entre les éléments d'une séquence (deux espaces par défaut).
#[must_use]
pub fn analyser(contenu: &str, separateur: &str) -> Journal {
    let separateur = if separateur.is_empty() {
        "  "
    } else {
        separateur
    };
    let mut enonces = Vec::new();
    let mut environnement = None;

    // `lines()` rend aussi la dernière ligne d'un fichier tronqué sans `\n`
    // final : c'est précisément le cas d'un journal encore ouvert.
    for ligne in contenu.lines() {
        let ligne = ligne.trim_end_matches('\r').trim();
        if ligne.is_empty() {
            continue;
        }
        if let Some(reste) = ligne.strip_prefix("# environnement:") {
            if environnement.is_none() {
                environnement = Some(reste.trim().to_owned());
            }
            continue;
        }
        // Marqueurs `### Log started/ended on … ###` : métadonnées de session,
        // jamais des paroles (recherche §1.2).
        if ligne.starts_with("###") {
            continue;
        }
        let morceaux: Vec<&str> = ligne
            .split(separateur)
            .map(str::trim)
            .filter(|morceau| !morceau.is_empty())
            .collect();
        if !morceaux.is_empty() {
            enonces.push(morceaux.join(" "));
        }
    }

    Journal {
        enonces,
        environnement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Valeurs attendues : recherche §1.2, exemple de sortie observé dans le
    // code de l'add-on (S5) — pas recalculées depuis notre implémentation.
    const SESSION: &str = "### Log started on 11/22/26 at 09:14:02 ###\n\
recycle bin  1 of 55\n\
Menu   sous-menu Ouvrir\n\
### Log ended on 11/22/26 at 09:14:31 ###\n";

    #[test]
    fn parse_le_format_documente_avec_le_separateur_par_defaut() {
        let journal = analyser(SESSION, "  ");
        assert_eq!(
            journal.enonces,
            vec!["recycle bin 1 of 55", "Menu sous-menu Ouvrir"]
        );
        assert_eq!(journal.environnement, None);
    }

    #[test]
    fn les_marqueurs_de_session_ne_sont_pas_des_enonces() {
        let journal = analyser("### Log started on 11/22/26 at 09:14:02 ###\n### Log ended on 11/22/26 at 09:14:31 ###\n", "  ");
        assert!(journal.enonces.is_empty());
    }

    #[test]
    fn un_separateur_personnalise_est_utilise() {
        // Recherche §1.2 : le séparateur est configurable.
        let contenu = "### Log started on 11/22/26 at 09:14:02 ###\nMenu|bouton\n";
        let journal = analyser(contenu, "|");
        assert_eq!(journal.enonces, vec!["Menu bouton"]);
    }

    #[test]
    fn un_journal_tronque_reste_lisible() {
        // Pas de `Log ended`, pas de retour à la ligne final : on garde ce
        // qui a déjà été écrit, sans paniquer.
        let journal = analyser(
            "### Log started on 11/22/26 at 09:14:02 ###\nmenu, bouton",
            "  ",
        );
        assert_eq!(journal.enonces, vec!["menu, bouton"]);
    }

    #[test]
    fn un_en_tete_peut_declarer_l_environnement() {
        let journal = analyser(
            "# environnement: NVDA 2026.3.1 + Firefox 141.0\nmenu, bouton\n",
            "  ",
        );
        assert_eq!(journal.enonces, vec!["menu, bouton"]);
        assert_eq!(
            journal.environnement.as_deref(),
            Some("NVDA 2026.3.1 + Firefox 141.0")
        );
    }
}
