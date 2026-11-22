//! Binaire `rgaa-sidecar-screenreader` : JSON-RPC 2.0 ligne par ligne sur
//! stdin/stdout (P-00 §7 bis). Les journaux vont dans stderr, jamais dans
//! stdout qui reste réservé au protocole.

use std::io::{self, BufRead, Write};

use chrono::{DateTime, Utc};
use rgaa_sidecar_screenreader::protocole::{
    Analyseur, Configuration, Erreur, HorlogeSimulee, ID, TESTS, VERSION,
};
use rgaa_sidecar_screenreader::{Horloge, HorlogeSysteme};
use serde_json::{json, Value};

fn envoyer(valeur: &Value) {
    let mut stdout = io::stdout().lock();
    if let Err(erreur) = serde_json::to_writer(&mut stdout, valeur) {
        tracing::error!(%erreur, "sérialisation JSON-RPC impossible");
        return;
    }
    if let Err(erreur) = stdout.write_all(b"\n").and_then(|()| stdout.flush()) {
        tracing::error!(%erreur, "écriture JSON-RPC impossible");
    }
}

fn reponse_erreur(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    })
}

fn journal(niveau: &str, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "journal",
        "params": {"niveau": niveau, "message": message},
    })
}

/// Horloge de rejeu, disponible seulement en debug pour les tests du binaire.
fn construire_horloge(_options: &Value) -> Box<dyn Horloge> {
    #[cfg(debug_assertions)]
    {
        if let Some(instants) = _options.get("horodatage_simule").and_then(Value::as_array) {
            let parses: Option<Vec<DateTime<Utc>>> = instants
                .iter()
                .map(|valeur| {
                    valeur
                        .as_str()
                        .and_then(|texte| DateTime::parse_from_rfc3339(texte).ok())
                        .map(|instant| instant.with_timezone(&Utc))
                })
                .collect();
            if let Some(instants) = parses {
                return Box::new(HorlogeSimulee::new(instants));
            }
            tracing::warn!("horodatage_simule invalide : horloge réelle utilisée");
        }
    }
    Box::new(HorlogeSysteme)
}

fn main() {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_ansi(false)
        .init();
    tracing::info!(
        plugin = ID,
        version = VERSION,
        "sidecar lecteur d'écran prêt"
    );

    let mut analyseur: Option<Analyseur<Box<dyn Horloge>>> = None;

    for ligne in io::stdin().lock().lines() {
        let ligne = match ligne {
            Ok(ligne) => ligne,
            Err(erreur) => {
                tracing::error!(%erreur, "lecture stdin");
                break;
            }
        };
        if ligne.trim().is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(&ligne) {
            Ok(message) => message,
            Err(erreur) => {
                envoyer(&reponse_erreur(
                    None,
                    -32700,
                    &format!("JSON invalide : {erreur}"),
                ));
                continue;
            }
        };
        let id = message.get("id").cloned();
        match message.get("method").and_then(Value::as_str) {
            Some("initialiser") => {
                let options = message
                    .get("params")
                    .and_then(|params| params.get("options"))
                    .unwrap_or(&Value::Null);
                let configuration = Configuration::depuis_options(options);
                let horloge = construire_horloge(options);
                if cfg!(debug_assertions) && options.get("horodatage_simule").is_some() {
                    envoyer(&journal(
                        "warn",
                        "horloge simulée active (mode test, jamais en audit)",
                    ));
                }
                analyseur = Some(Analyseur::new(configuration, horloge));
                envoyer(&json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "id": ID,
                        "version": VERSION,
                        "tests": TESTS,
                        "version_modele": Value::Null,
                    },
                }));
            }
            Some("analyser") => {
                let params = message.get("params").unwrap_or(&Value::Null);
                if analyseur.is_none() {
                    envoyer(&reponse_erreur(
                        id,
                        -32000,
                        "preuve manquante (sidecar screenreader-nvda : initialiser non appelé)",
                    ));
                    continue;
                }
                // En debug, l'horloge de rejeu peut aussi être demandée par
                // `analyser` (le client sidecar n'envoie les options qu'ici).
                #[cfg(debug_assertions)]
                if let Some(options) = params.get("options") {
                    if options.get("horodatage_simule").is_some() {
                        analyseur = Some(Analyseur::new(
                            Configuration::depuis_options(options),
                            construire_horloge(options),
                        ));
                    }
                }
                let Some(sidecar) = analyseur.as_mut() else {
                    envoyer(&reponse_erreur(
                        id,
                        -32000,
                        "preuve manquante (sidecar screenreader-nvda : initialiser indisponible)",
                    ));
                    continue;
                };
                match sidecar.analyser(params) {
                    Ok(preuves) => envoyer(&json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"preuves": preuves},
                    })),
                    Err(erreur) => {
                        let raison = match &erreur {
                            Erreur::NvdaNonDetecte => {
                                "preuve manquante (sidecar screenreader-nvda : NVDA non détecté)"
                                    .into()
                            }
                            _ => format!("preuve manquante (sidecar screenreader-nvda : {erreur})"),
                        };
                        tracing::warn!(%raison, "analyse screenreader interrompue");
                        envoyer(&journal("warn", &raison));
                        envoyer(&reponse_erreur(id, -32000, &raison));
                    }
                }
            }
            Some("arreter") => break,
            Some(methode) => envoyer(&reponse_erreur(
                id,
                -32601,
                &format!("méthode inconnue : {methode}"),
            )),
            None => envoyer(&reponse_erreur(id, -32600, "méthode absente")),
        }
    }
}
