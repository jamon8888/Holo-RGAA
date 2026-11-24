//! Point d'extension du sidecar IRIS : oracle indépendant, non installé ici.
//!
//! Le dépôt IRIS (Electronic Arts, BSD-3) n'est pas compilé dans ce plugin et
//! n'a pas reçu de push depuis 2025-01 : il reste un **oracle optionnel** des
//! tests, jamais le moteur principal. Lorsqu'une commande est configurée
//! (`RGAA_IRIS_BIN`), on lui envoie les frames sérialisées sur stdin et on
//! attend un `FlashResult` JSON (ou `null`) sur stdout. Sans commande, le
//! résultat est `None` et le plugin continue avec son propre comptage.

use crate::flash::FlashResult;
use crate::frames::Frame;
use crate::{missing_proof, Result, PLUGIN_ID};
use serde::Serialize;
use std::io::Write;
use std::process::{Command, Stdio};

/// Oracle de flashs interchangeable (IRIS aujourd'hui, autre outil demain).
pub trait OracleFlash {
    fn nom(&self) -> &'static str;
    fn disponible(&self) -> bool;
    fn compte_flashs(&self, frames: &[Frame]) -> Result<Option<FlashResult>>;
}

#[derive(Debug, Serialize)]
struct EntreeOracle<'a> {
    frames: &'a [Frame],
}

/// Oracle IRIS, configuré ou non.
pub struct IrisOracle {
    pub commande: Option<String>,
}

impl IrisOracle {
    #[must_use]
    pub fn avec_commande(commande: Option<String>) -> Self {
        Self { commande }
    }

    /// Lit `RGAA_IRIS_BIN` sans jamais télécharger ni installer quoi que ce soit.
    #[must_use]
    pub fn depuis_environnement() -> Self {
        Self::avec_commande(
            std::env::var("RGAA_IRIS_BIN")
                .ok()
                .filter(|c| !c.is_empty()),
        )
    }

    fn commande(&self) -> Option<&str> {
        self.commande.as_deref()
    }
}

impl Default for IrisOracle {
    fn default() -> Self {
        Self::depuis_environnement()
    }
}

impl OracleFlash for IrisOracle {
    fn nom(&self) -> &'static str {
        "iris"
    }

    fn disponible(&self) -> bool {
        self.commande().is_some()
    }

    fn compte_flashs(&self, frames: &[Frame]) -> Result<Option<FlashResult>> {
        let Some(commande) = self.commande() else {
            return Ok(None);
        };
        let entre = serde_json::to_vec(&EntreeOracle { frames }).map_err(|erreur| {
            crate::analysis_error(PLUGIN_ID, format!("entrée oracle IRIS invalide : {erreur}"))
        })?;
        let mut enfant = Command::new(commande)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|erreur| {
                missing_proof(
                    PLUGIN_ID,
                    &format!("oracle IRIS ({commande}) absent : {erreur}"),
                )
            })?;
        if let Some(stdin) = enfant.stdin.as_mut() {
            stdin.write_all(&entre).map_err(|erreur| {
                missing_proof(PLUGIN_ID, &format!("oracle IRIS injoignable : {erreur}"))
            })?;
        }
        let sortie = enfant.wait_with_output().map_err(|erreur| {
            missing_proof(PLUGIN_ID, &format!("oracle IRIS interrompu : {erreur}"))
        })?;
        if !sortie.status.success() {
            return Err(missing_proof(
                PLUGIN_ID,
                &format!("oracle IRIS a échoué (code {})", sortie.status),
            ));
        }
        serde_json::from_slice(&sortie.stdout).map_err(|erreur| {
            missing_proof(
                PLUGIN_ID,
                &format!("réponse oracle IRIS invalide : {erreur}"),
            )
        })
    }
}
