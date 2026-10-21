//! 5.7 est déclaré en couverture *complète* (`mechanisms.toml`), routé vers trois règles
//! axe (`td-headers-attr`, `th-has-data-cells`, `td-has-header`). Deux raisons distinctes,
//! chacune suffisante seule, lui interdisent de rendre `Pass` sur son silence :
//!
//! 1. Aucune des trois ne valide ce que 5.7.2 demande : qu'un attribut `scope` porte la
//!    valeur `"row"` pour un en-tête de ligne et `"col"` pour un en-tête de colonne.
//!    `td-headers-attr` valide seulement qu'un `headers` référence bien des `<th>` du même
//!    tableau (5.7.4, côté référence) ; `th-has-data-cells` valide qu'un `<th>` décrit au
//!    moins une cellule de données ; `td-has-header` (expérimentale) valide qu'une cellule
//!    de donnée a au moins un en-tête. Confirmé contre axe-core lui-même, pas seulement
//!    contre ce pipeline : la même page, chargée dans un vrai Chrome, laisse aussi passer
//!    un `scope="col"` posé sur un `<th>` qui est, sans ambiguïté, un en-tête de ligne.
//!    Ce n'est pas un défaut de ce pipeline (contrairement au point 2) : c'est une limite
//!    réelle d'axe-core.
//! 2. En instrumentant l'exécution réelle, `td-headers-attr` et `th-has-data-cells`
//!    tombent en **`incomplete`** via ce pipeline sur un témoin qu'un vrai Chrome classe
//!    pourtant sans ambiguïté en `violations` (un `headers` pointant vers un id
//!    inexistant). C'est le même défaut d'infrastructure que #371 (confirmé jusqu'ici sur
//!    `color-contrast` et `link-in-text-block`), mais il touche ici une règle qui ne
//!    dépend d'aucun rendu visuel — seulement d'une mise en relation entre deux nœuds du
//!    DOM (un `headers` et l'`id` qu'il référence). Le défaut est donc plus large qu'un
//!    problème de couleur/contraste : toute règle qui compare ou relie plusieurs nœuds
//!    semble à risque.
//!
//! Ce test pose la question à l'audit réel. Deux pages : un **témoin** (un `headers`
//! pointant vers un id inexistant — une violation flagrante, qu'un vrai Chrome classe en
//! `violations`) et une page où chaque `<th>` de ligne porte `scope="col"` au lieu de
//! `scope="row"`.
//!
//! Non câblé dans le job E2E de la CI. Lancer à la main avec `RUN_E2E=1`.

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

async fn audit_5_7(bridge: &Arc<ObscuraBridge>, url: &str) -> Option<CriterionStatus> {
    let mut axe = Arc::clone(bridge)
        .run_axe_batch(vec![url.to_string()], 1)
        .await
        .expect("run_axe_batch");
    let keys: Vec<String> = axe.keys().cloned().collect();
    let violations = axe
        .remove(url)
        .unwrap_or_else(|| panic!("pas de résultat axe pour {url} ; clés reçues : {keys:?}"));
    let mapped = AxeMapper::map(&violations).expect("mapping axe");
    mapped.get("5.7").map(|r| r.status.clone())
}

#[tokio::test]
async fn criterion_5_7_never_passes_on_silence_alone() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_5_7_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    let mut server = ObscuraBridge::new().with_port(9347);
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");
    let bridge = Arc::new(server);

    // Violation flagrante (headers vers un id inexistant), qu'un vrai Chrome classe sans
    // ambiguïté en violation. Note #371 : ce témoin ne rend pas `Fail` aujourd'hui via ce
    // pipeline — `td-headers-attr` tombe en `incomplete` plutôt qu'en violation. Ce qui
    // compte ici, et ce que ce correctif garantit, c'est qu'il ne rend pas non plus
    // `Pass` : son absence de verdict retombe honnêtement sur le repli du pipeline.
    let control = format!("http://127.0.0.1:{port}/coverage-5-7-headers-dangling-control.html");
    let control_status = audit_5_7(&bridge, &control).await;
    eprintln!("témoin (headers vers un id inexistant) : axe -> {control_status:?}");
    assert_ne!(
        control_status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 5.7 sur un headers pointant vers un id inexistant"
    );

    let page = format!("http://127.0.0.1:{port}/coverage-5-7-wrong-scope-orientation.html");
    let status = audit_5_7(&bridge, &page).await;
    eprintln!("scope inversé : axe -> {status:?}");

    // Aucune des trois règles mappées ne vérifie la cohérence orientation/valeur de
    // `scope`, même dans un environnement sans le défaut #371 (vérifié contre un vrai
    // Chrome) : axe ne doit jamais rendre `Pass` sur son silence ici.
    assert_ne!(
        status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 5.7 alors que chaque <th> de ligne porte scope=\"col\" au lieu \
         de scope=\"row\" : aucune des trois règles mappées ne vérifie cette cohérence"
    );
}
