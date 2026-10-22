//! 1.1 est déclaré en couverture *complète* (`mechanisms.toml`) et ses huit tests routés vers
//! `axe-1-1` + `gapfix-1-1`. Mais aucune règle axe citée ne cible 1.1.4 (image réactive côté
//! serveur), 1.1.7 (`<embed>`) ni 1.1.8 (`<canvas>`), et la sonde `gapfix-1-1` ne regarde que
//! `img:not([alt])`. Si le silence d'axe vaut `Pass`, une page dont seul un canvas ou un embed
//! manque d'alternative ressortirait conforme sur 1.1.
//!
//! Ce test pose la question à l'audit réel. Deux pages : un **témoin** (img sans alt, que
//! l'audit doit voir — sans lui, un résultat « conforme » ne prouverait rien) et la page à
//! canvas + embed.
//!
//! Non câblé dans le job E2E de la CI : il énonce le comportement **attendu**, et tant que la
//! couverture de 1.1 est surdéclarée il échoue. Lancer à la main avec `RUN_E2E=1`.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rgaa_core::CriterionStatus;
use rgaa_obscura::ObscuraBridge;
use rgaa_rules::{AxeMapper, GapFixRules};

fn serve(dir: PathBuf) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
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
        }
    });
    Ok(port)
}

/// Ce que l'audit conclut pour 1.1 sur une page, **tel que le pipeline le matérialise** :
/// le statut posé par axe (via `AxeMapper`) et celui posé par la sonde `gapfix-1-1` (via
/// `GapFixRules::parse_results`). `None` = aucun verdict : le critère est laissé au repli du
/// pipeline (estimation du modèle), ce qui n'est pas une conformité.
async fn audit_1_1(url: &str) -> (Option<CriterionStatus>, Option<CriterionStatus>) {
    // `run_axe_batch` pilote une session CDP par URL : le serveur doit être démarré. Sans lui
    // la fonction rend une carte vide (l'échec par URL est journalisé, jamais remonté), ce
    // qui ressemblerait à « axe n'a rien trouvé ».
    let mut server = ObscuraBridge::new().with_port(9341);
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");
    let bridge = Arc::new(server);
    let binary = bridge.binary_path().to_string();

    let mut axe = bridge
        .run_axe_batch(vec![url.to_string()], 1)
        .await
        .expect("run_axe_batch");
    let keys: Vec<String> = axe.keys().cloned().collect();
    let violations = axe
        .remove(url)
        .unwrap_or_else(|| panic!("pas de résultat axe pour {url} ; clés reçues : {keys:?}"));
    let mapped = AxeMapper::map(&violations).expect("mapping axe");
    let axe_status = mapped.get("1.1").map(|r| r.status.clone());

    let snippets: HashMap<String, String> = GapFixRules::snippets()
        .iter()
        .filter(|(id, _)| id.as_str() == "1.1")
        .map(|(id, s)| (id.clone(), (*s).to_string()))
        .collect();
    let mut probes = ObscuraBridge::run_gap_fix_batch(binary, vec![url.to_string()], snippets, 1)
        .await
        .expect("run_gap_fix_batch");
    let probes_keys: Vec<String> = probes.keys().cloned().collect();
    let raw = probes.remove(url).unwrap_or_else(|| {
        panic!("pas de résultat de sonde pour {url} ; clés reçues : {probes_keys:?}")
    });
    let probe_status = GapFixRules::parse_results(&raw)
        .get("1.1")
        .map(|r| r.status.clone());
    (axe_status, probe_status)
}

#[tokio::test]
async fn criterion_1_1_does_not_pass_a_page_whose_canvas_and_embed_lack_an_alternative() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_1_1_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    // Témoin : sans lui, « pas de Pass » sur la page à canvas ne prouverait rien.
    let control = format!("http://127.0.0.1:{port}/coverage-1-1-img-control.html");
    let (axe_control, probe_control) = audit_1_1(&control).await;
    assert_eq!(
        axe_control,
        Some(CriterionStatus::Fail),
        "le témoin (img sans alt) doit échouer 1.1 : sinon le banc ne voit rien"
    );
    assert_eq!(
        probe_control,
        Some(CriterionStatus::Fail),
        "la sonde doit voir l'img sans alt"
    );

    let page = format!("http://127.0.0.1:{port}/coverage-1-1-embed-canvas.html");
    let (axe_status, probe_status) = audit_1_1(&page).await;
    eprintln!("page canvas+embed : axe -> {axe_status:?}, sonde gapfix-1-1 -> {probe_status:?}");

    // Aucun des deux mécanismes ne doit conclure `Pass` : ni l'un ni l'autre ne décide
    // 1.1.7 ni 1.1.8. L'absence de verdict est le bon résultat — le critère retombe sur le
    // repli du pipeline (estimation du modèle, jamais un `Pass`).
    assert_ne!(
        axe_status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 1.1 alors qu'un <canvas> et un <embed> sont sans alternative"
    );
    assert_ne!(
        probe_status,
        Some(CriterionStatus::Pass),
        "la sonde rend Pass pour 1.1 alors qu'un <canvas> et un <embed> sont sans alternative"
    );
}
