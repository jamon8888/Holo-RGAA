//! Inventaire des écouteurs (#272) sous le vrai Obscura : 12.10, 13.10, 13.11 et 13.12.
//!
//! Chaque fixture `-fail` doit produire un indice `review` pour son critère et chaque
//! fixture `-pass` aucun. Le harnais de classification des fixtures ne vérifie pas cela :
//! pour un mécanisme `review` seul, il ignore les fixtures `-fail`.
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
    listener_results, LISTENER_HOOK, LISTENER_OBSERVATION_MS, LISTENER_READ,
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
async fn listener_inventory_hints_on_fail_fixtures_and_stays_silent_on_pass_fixtures() {
    if !rgaa_test_corpus::e2e::enabled("criterion_listener_inventory") {
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

    let mut wrong = Vec::new();
    for criterion in ["12.10", "13.10", "13.11", "13.12"] {
        for (suffix, expect_hint) in [("fail", true), ("pass", false)] {
            let url =
                format!("http://127.0.0.1:{port}/{criterion}-listener-inventory-{suffix}.html");
            let observation = bridge
                .observe_with_hook(
                    &url,
                    LISTENER_HOOK,
                    LISTENER_READ,
                    Duration::from_millis(LISTENER_OBSERVATION_MS),
                )
                .await
                .unwrap_or_else(|e| panic!("{criterion}/{suffix}: {e}"));
            let results = listener_results(&observation);
            eprintln!(
                "{criterion}/{suffix}: {:?}",
                results.get(criterion).map(|r| &r.justification)
            );
            let hinted = results.get(criterion);
            if hinted.is_some() != expect_hint {
                wrong.push(format!("{criterion}-{suffix}: hint={}", hinted.is_some()));
            }
            if let Some(result) = hinted {
                assert_eq!(result.status, CriterionStatus::NeedsReview, "{criterion}");
            }
        }
    }
    bridge.stop_server().await;
    assert!(wrong.is_empty(), "inventaire mal classé: {wrong:?}");
}
