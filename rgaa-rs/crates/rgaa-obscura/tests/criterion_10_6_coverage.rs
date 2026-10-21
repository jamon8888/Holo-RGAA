//! 10.6 est déclaré en couverture *complète* (`mechanisms.toml`), routé vers une seule règle
//! axe (`axe-10-6` : `link-in-text-block`). En instrumentant l'exécution réelle, cette règle
//! souffre du même défaut d'infrastructure que `color-contrast` (#371) — mais sous une forme
//! plus trompeuse encore : au lieu de se classer `inapplicable` (silencieuse, au moins
//! honnête), elle rend un **faux `passes`** sur une violation flagrante.
//!
//! Preuve croisée : la fixture `coverage-10-6-near-invisible-link.html` pose un lien dont la
//! couleur (`#808080`) est à peine plus claire que le texte environnant (`#666666`, ratio ≈
//! 1.46:1, bien sous le 3:1 exigé par 10.6.1), sans soulignement ni gras ni indication au
//! survol/focus. Dans un **vrai Chrome** (axe-core 4.9.1 via CDN), `link-in-text-block` la
//! classe correctement en `violations`. Via `ObscuraBridge::run_axe_batch` (même règle, même
//! page), elle atterrit en `passes` — axe conclut, à tort, qu'il n'y a rien à redire.
//!
//! Ce test pose la question à l'audit réel : axe ne doit jamais rendre `Pass` pour 10.6 sur
//! cette page. Non câblé dans le job E2E de la CI. Lancer à la main avec `RUN_E2E=1`.

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

#[tokio::test]
async fn criterion_10_6_never_passes_on_a_link_distinguished_by_color_alone() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_10_6_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    let mut server = ObscuraBridge::new().with_port(9345);
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");
    let bridge = Arc::new(server);

    let url = format!("http://127.0.0.1:{port}/coverage-10-6-near-invisible-link.html");
    let mut axe = Arc::clone(&bridge)
        .run_axe_batch(vec![url.clone()], 1)
        .await
        .expect("run_axe_batch");
    let violations = axe.remove(&url).unwrap_or_default();
    let mapped = AxeMapper::map(&violations).expect("mapping axe");
    let status = mapped.get("10.6").map(|r| r.status.clone());
    eprintln!("lien distingué uniquement par une couleur proche : axe -> {status:?}");

    // Note #371 : ce lien ne rend pas `Fail` aujourd'hui — `link-in-text-block` rend un
    // faux `passes` dans cet environnement, alors qu'un vrai Chrome le classe correctement
    // en violation sur la même page. Ce que ce correctif garantit, c'est qu'il ne rend pas
    // non plus `Pass` : son absence de verdict retombe honnêtement sur le repli du
    // pipeline, jamais sur une conformité inventée.
    assert_ne!(
        status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 10.6 sur un lien distingué uniquement par une couleur à peine \
         différente du texte environnant (ratio ≈ 1.46:1, sous le seuil de 3:1)"
    );
}
