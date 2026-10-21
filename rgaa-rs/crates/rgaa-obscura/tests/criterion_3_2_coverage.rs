//! 3.2 est déclaré en couverture *complète* (`mechanisms.toml`), routé vers une seule règle
//! axe (`axe-3-2` : `color-contrast`) qui audite le DOM **tel qu'il est rendu au
//! chargement**. Deux raisons distinctes, chacune suffisante à elle seule, lui interdisent
//! de rendre `Pass` au silence :
//!
//! 1. 3.2.5 porte sur le [mécanisme qui permet d'afficher un rapport de contraste
//!    conforme](https://accessibilite.numerique.gouv.fr/methode/glossaire/) : « un composant
//!    d'interface dont l'activation permet de modifier l'apparence du site […] pour afficher
//!    les contenus avec un ratio de contraste suffisant ». Un bouton qui, une fois cliqué,
//!    produit un contraste *insuffisant*, est invisible à un scan statique au chargement : la
//!    règle axe ne clique jamais sur rien.
//! 2. En instrumentant l'exécution réelle, `color-contrast` n'est **jamais** rapportée par le
//!    pipeline `rgaa-obscura` — ni en violation, ni en `incomplete` — alors que la même
//!    version d'axe-core (4.9.1) la détecte sans ambiguïté dans un Chrome normal sur une
//!    violation flagrante. axe la classe en `inapplicable`, `nodes: []` : sa fonction
//!    `matches()` ne considère aucun nœud comme candidat dans cet environnement headless.
//!    C'est un défaut d'infrastructure du navigateur piloté par `obscura` (#371), pas un
//!    défaut de mapping ; ce test ne le corrige pas, il en constate l'effet.
//!
//! Témoin inclus pour mémoire de #371 : une page à violation flagrante (texte `#cccccc` sur
//! fond `#ffffff`, ratio ≈ 1.6:1) rend ici aussi `axe -> None` plutôt que `Fail` — la preuve
//! la plus directe que `color-contrast` ne se déclenche pas dans cet environnement. Tant
//! que #371 n'est pas résolu, ce test ne peut prouver que l'absence de `Pass` fabriqué ; il
//! ne peut pas prouver qu'axe détecte correctement une vraie violation de contraste.
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

/// Le statut posé par axe pour 3.2 sur une page, **tel que le pipeline le matérialise**.
async fn audit_3_2(bridge: &Arc<ObscuraBridge>, url: &str) -> Option<CriterionStatus> {
    let mut axe = bridge
        .clone()
        .run_axe_batch(vec![url.to_string()], 1)
        .await
        .expect("run_axe_batch");
    let keys: Vec<String> = axe.keys().cloned().collect();
    let violations = axe
        .remove(url)
        .unwrap_or_else(|| panic!("pas de résultat axe pour {url} ; clés reçues : {keys:?}"));
    let mapped = AxeMapper::map(&violations).expect("mapping axe");
    mapped.get("3.2").map(|r| r.status.clone())
}

#[tokio::test]
async fn criterion_3_2_never_passes_on_silence_alone() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_3_2_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    let mut server = ObscuraBridge::new().with_port(9344);
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");
    let bridge = Arc::new(server);

    // Violation flagrante dès le chargement, sans qu'aucun mécanisme n'ait besoin d'être
    // activé. Note #371 : ce témoin ne rend pas `Fail` aujourd'hui — `color-contrast` ne se
    // déclenche pas du tout dans cet environnement. Ce qui compte ici, et ce que le fix de
    // cette PR garantit, c'est qu'il ne rend pas non plus `Pass` : son absence de verdict
    // retombe honnêtement sur le repli du pipeline, jamais sur une conformité inventée.
    let control = format!("http://127.0.0.1:{port}/coverage-3-2-low-contrast-control.html");
    let control_status = audit_3_2(&bridge, &control).await;
    eprintln!("témoin (violation flagrante) : axe -> {control_status:?}");
    assert_ne!(
        control_status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 3.2 sur une page dont le contraste est manifestement insuffisant \
         dès le chargement"
    );

    // Mécanisme d'affichage de contraste qui, une fois activé, produit un contraste
    // insuffisant — axe ne clique jamais sur rien, donc son silence sur l'état par défaut
    // (correct) ne dit rien de ce que 3.2.5 interroge.
    let page = format!("http://127.0.0.1:{port}/coverage-3-2-broken-contrast-mechanism.html");
    let status = audit_3_2(&bridge, &page).await;
    eprintln!("mécanisme de contraste cassé : axe -> {status:?}");
    assert_ne!(
        status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 3.2 alors que le mécanisme d'affichage de contraste, une fois \
         activé, produit un contraste insuffisant"
    );
}
