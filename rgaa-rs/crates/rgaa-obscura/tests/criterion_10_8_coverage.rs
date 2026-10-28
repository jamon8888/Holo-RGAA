//! 10.8 est déclaré en couverture *complète* (`mechanisms.toml`), routé vers deux règles axe
//! (`aria-hidden-focus`, `hidden-content`). Aucune des deux ne décide 10.8.1 dans son
//! ensemble :
//!
//! - `aria-hidden-focus` catches une contradiction structurelle précise (un élément
//!   `aria-hidden` contenant un enfant focalisable) — mais ne regarde jamais un contenu
//!   masqué par CSS qui n'est **pas** marqué `aria-hidden`.
//! - `hidden-content` (tags axe : `experimental`, `review-item`, **sans aucun tag RGAAv4
//!   ni wcag**) se contente de signaler l'existence de contenu caché pour revue manuelle —
//!   elle ne tranche jamais sa conformité.
//!
//! 10.8.1 pose une question en deux branches : le contenu caché a-t-il vocation à être
//! ignoré par les technologies d'assistance, OU devient-il restituable suite à une action
//! clavier/pointage sur un élément précédent ? Établir laquelle des deux s'applique est un
//! jugement sur l'**intention** de l'auteur et sur le **comportement** effectif d'un
//! mécanisme de révélation — ni l'un ni l'autre ne se lit dans le DOM statique qu'axe
//! examine.
//!
//! Ce test pose la question à l'audit réel. Deux pages : un **témoin** (contradiction
//! structurelle qu'axe doit voir — sans lui, un résultat « conforme » sur l'autre page ne
//! prouverait rien) et une page à contenu caché par CSS, non marqué `aria-hidden`, dont le
//! seul bouton visible ne le révèle jamais.
//!
//! Câblé dans le job CI `E2E (navigateur)`, qui pose `RUN_E2E=1`. En local, lancer
//! à la main avec `RUN_E2E=1` (nécessite le binaire Obscura épinglé sur le PATH).

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

async fn audit_10_8(bridge: &Arc<ObscuraBridge>, url: &str) -> Option<CriterionStatus> {
    let mut axe = Arc::clone(bridge)
        .run_axe_batch(vec![url.to_string()], 1)
        .await
        .expect("run_axe_batch");
    let keys: Vec<String> = axe.keys().cloned().collect();
    let violations = axe
        .remove(url)
        .unwrap_or_else(|| panic!("pas de résultat axe pour {url} ; clés reçues : {keys:?}"));
    let mapped = AxeMapper::map(&violations).expect("mapping axe");
    mapped.get("10.8").map(|r| r.status.clone())
}

#[tokio::test]
async fn criterion_10_8_does_not_pass_hidden_content_of_undecided_intent() {
    if !rgaa_test_corpus::e2e::enabled("criterion_10_8_coverage") {
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    let mut server = ObscuraBridge::new()
        .with_ephemeral_port()
        .expect("port CDP libre");
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");
    let bridge = Arc::new(server);

    // Témoin : contradiction structurelle précise (aria-hidden contenant un lien
    // focalisable). Sans ce témoin en échec, le banc pourrait être aveugle.
    let control =
        format!("http://127.0.0.1:{port}/coverage-10-8-aria-hidden-focusable-control.html");
    let control_status = audit_10_8(&bridge, &control).await;
    assert_eq!(
        control_status,
        Some(CriterionStatus::Fail),
        "le témoin (aria-hidden contenant un focalisable) doit échouer 10.8 : sinon le banc \
         ne voit rien"
    );

    let page = format!("http://127.0.0.1:{port}/coverage-10-8-broken-reveal.html");
    let status = audit_10_8(&bridge, &page).await;
    eprintln!("contenu caché d'intention non établie : axe -> {status:?}");

    // Ni aria-hidden-focus (pas de contradiction structurelle ici : le contenu n'est pas
    // aria-hidden) ni hidden-content (purement informative, jamais de verdict de
    // conformité) ne peuvent établir que ce contenu a vocation à être ignoré, ou qu'il
    // devient restituable par une action clavier. axe ne doit donc jamais rendre `Pass`.
    assert_ne!(
        status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 10.8 alors que l'intention du contenu caché (ignoré par les AT, \
         ou restituable via une action clavier) n'est établie par aucune règle"
    );
}
