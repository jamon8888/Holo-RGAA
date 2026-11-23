//! 13.1 — redirections par script (#265), vérifiées sous le vrai Obscura.
//!
//! La capacité `Page.addScriptToEvaluateOnNewDocument` (hook posé avant les scripts de la
//! page) est celle que la spec marque « (v) » : ce test est la preuve qu'elle fonctionne
//! sur le binaire épinglé, et il classe les fixtures du mécanisme `probe-13-1-script-redirect`.
//!
//! Câblé dans le job CI `E2E (navigateur)` (`RUN_E2E=1`) ; en local, lancer à la main avec
//! `RUN_E2E=1` et le binaire Obscura épinglé (`RGAA_OBSCURA_BIN`).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

use rgaa_core::CriterionStatus;
use rgaa_obscura::ObscuraBridge;
use rgaa_rules::script_hooks::{
    parse_redirect_observation, PartialProbeStatus, REDIRECT_HOOK, REDIRECT_OBSERVATION_MS,
    REDIRECT_READ,
};

/// Serveur de fixtures : un fil par connexion, pour qu'une connexion ouverte et muette
/// (Obscura en garde) ne bloque jamais la suivante.
fn serve(dir: std::path::PathBuf) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let dir = dir.clone();
            std::thread::spawn(move || {
                let mut stream = stream;
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                let name = request
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .trim_start_matches('/')
                    .to_string();
                let body = if name.contains("..") || name.contains('/') {
                    None
                } else {
                    std::fs::read(dir.join(&name)).ok()
                };
                let _ = match body {
                    Some(b) => {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
                             Content-Length: {}\r\nConnection: close\r\n\r\n",
                            b.len()
                        );
                        stream
                            .write_all(head.as_bytes())
                            .and_then(|()| stream.write_all(&b))
                    }
                    None => stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    ),
                };
            });
        }
    });
    Ok(port)
}

#[tokio::test]
async fn script_redirects_fail_and_carousels_do_not() {
    if !rgaa_test_corpus::e2e::enabled("criterion_13_1_script_redirect") {
        return;
    }
    rgaa_test_corpus::e2e::allow_private_network();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rgaa-test-corpus/criteria");
    let port = serve(dir).expect("fixture server");

    let mut bridge = ObscuraBridge::from_env()
        .with_ephemeral_port()
        .expect("port CDP libre");
    bridge
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");

    let mut verdicts = Vec::new();
    for name in [
        "13.1-script-redirect-fail",
        "13.1-script-redirect-href-fail",
        "13.1-script-redirect-pass",
    ] {
        let url = format!("http://127.0.0.1:{port}/{name}.html");
        let observation = bridge
            .observe_with_hook(
                &url,
                REDIRECT_HOOK,
                REDIRECT_READ,
                Duration::from_millis(REDIRECT_OBSERVATION_MS),
            )
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        if name == "13.1-script-redirect-pass" {
            assert!(
                observation["at_load"]["title"]
                    .as_str()
                    .is_some_and(|title| title.contains("Carrousel automatique")),
                "carousel document was not observed: {observation}"
            );
        }
        let finding = parse_redirect_observation(&observation);
        eprintln!("{name}: {finding:?}");
        verdicts.push((name, finding.map(|f| f.status)));
    }
    bridge.stop_server().await;

    assert_eq!(
        verdicts[0].1,
        Some(PartialProbeStatus::NeedsReview),
        "{verdicts:?}"
    );
    assert_eq!(
        verdicts[1].1,
        Some(PartialProbeStatus::NeedsReview),
        "{verdicts:?}"
    );
    assert!(verdicts[2].1.is_none(), "{verdicts:?}");
    assert_ne!(
        verdicts[2].1,
        Some(CriterionStatus::Fail),
        "un carrousel n'est pas une redirection: {verdicts:?}"
    );
}
