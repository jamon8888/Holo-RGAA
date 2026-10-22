//! 9.3 est déclaré en couverture *complète* (`mechanisms.toml`), routé vers une seule règle
//! axe (`axe-9-3` : `list`, `listitem`, `definition-list`, `dlitem`). Ces règles valident la
//! *structure* d'un élément déjà balisé comme liste — elles ne regardent jamais un élément qui
//! n'est pas balisé comme tel. Or 9.3.1 et 9.3.2 posent la question sur « les informations
//! regroupées **visuellement** sous forme de liste », balisage ou non : une liste à puces faite
//! en CSS sur des `<p>`, sans aucun `<ul>`/`<li>`, est exactement le cas que le test vise, et
//! axe n'a rien à y mordre puisqu'aucun élément n'y porte un rôle de liste.
//!
//! Ce test pose la question à l'audit réel. Deux pages : un **témoin** (un vrai `<ul>` mal
//! balisé, qu'axe doit voir — sans lui, un résultat « conforme » sur l'autre page ne prouverait
//! rien) et la page à liste visuelle sans balisage.
//!
//! Non câblé dans le job E2E de la CI : il énonce le comportement **attendu**, et tant que la
//! couverture de 9.3 est surdéclarée il échoue. Lancer à la main avec `RUN_E2E=1`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rgaa_core::CriterionStatus;
use rgaa_obscura::ObscuraBridge;
use rgaa_rules::AxeMapper;

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

/// Le statut posé par axe pour 9.3 sur une page, **tel que le pipeline le matérialise**.
async fn audit_9_3(bridge: &Arc<ObscuraBridge>, url: &str) -> Option<CriterionStatus> {
    let mut axe = Arc::clone(bridge)
        .run_axe_batch(vec![url.to_string()], 1)
        .await
        .expect("run_axe_batch");
    let keys: Vec<String> = axe.keys().cloned().collect();
    let violations = axe
        .remove(url)
        .unwrap_or_else(|| panic!("pas de résultat axe pour {url} ; clés reçues : {keys:?}"));
    let mapped = AxeMapper::map(&violations).expect("mapping axe");
    mapped.get("9.3").map(|r| r.status.clone())
}

#[tokio::test]
async fn criterion_9_3_does_not_pass_a_visually_styled_list_with_no_list_markup() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_9_3_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    let mut server = ObscuraBridge::new().with_port(9342);
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");
    let bridge = Arc::new(server);

    // Témoin : un vrai `<ul>` dont les enfants ne sont pas des `<li>`. Sans ce témoin en
    // échec, une page « conforme » sur l'autre ne prouverait rien — le banc pourrait être
    // simplement aveugle.
    let control = format!("http://127.0.0.1:{port}/coverage-9-3-list-control.html");
    let control_status = audit_9_3(&bridge, &control).await;
    assert_eq!(
        control_status,
        Some(CriterionStatus::Fail),
        "le témoin (ul mal balisé) doit échouer 9.3 : sinon le banc ne voit rien"
    );

    let page = format!("http://127.0.0.1:{port}/coverage-9-3-visual-fake-list.html");
    let status = audit_9_3(&bridge, &page).await;
    eprintln!("liste visuelle sans balisage : axe -> {status:?}");

    // axe ne doit pas rendre `Pass` : aucune règle du mécanisme ne regarde un contenu qui
    // n'est pas balisé comme liste, donc son silence ne décide rien sur 9.3.1/9.3.2.
    assert_ne!(
        status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 9.3 alors que la page ne contient aucune balise de liste à juger : \
         son silence ne prouve pas que les informations regroupées visuellement sont bien codées"
    );
}
