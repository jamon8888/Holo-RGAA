//! 12.7 est déclaré en couverture *complète* (`mechanisms.toml`, `test_routes.json`), ses
//! deux tests routés vers `axe-12-7` (`bypass`, `skip-link`) et `gapfix-12-7` (repérage de
//! texte). Les deux mécanismes n'examinent jamais qu'**une seule page**. Or 12.7.2 porte sur
//! « chaque **ensemble de pages** » : la cohérence du lien d'évitement à travers tout un
//! site. Aucune règle axe ni aucune sonde du registre ne compare deux pages entre elles —
//! `site_comparison.rs` ne traite pas ce critère. Si le silence (ou la réussite) d'une seule
//! page vaut `Pass` pour 12.7, un site audité sur une seule page ressortirait conforme à
//! 12.7.2 sans que la cohérence inter-pages ait jamais été vérifiée.
//!
//! Ce test pose la question à l'audit réel sur une seule page, correcte pour 12.7.1 :
//! ni axe ni la sonde ne doivent conclure `Pass` pour le critère entier, puisqu'aucun des
//! deux ne décide 12.7.2. Un **témoin** (page sans aucun mécanisme d'évitement) prouve que
//! le banc n'est pas aveugle : sans lui, un résultat « non conforme » sur la bonne page ne
//! prouverait rien.
//!
//! Non câblé dans le job E2E de la CI : il énonce le comportement **attendu**, et tant que
//! la couverture de 12.7 est surdéclarée il échoue. Lancer à la main avec `RUN_E2E=1`.

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

/// Ce que l'audit conclut pour 12.7 sur une page, **tel que le pipeline le matérialise** :
/// le statut posé par axe et celui posé par la sonde `gapfix-12-7`.
async fn audit_12_7(
    bridge: &Arc<ObscuraBridge>,
    binary: &str,
    url: &str,
) -> (Option<CriterionStatus>, Option<CriterionStatus>) {
    let mut axe = Arc::clone(bridge)
        .run_axe_batch(vec![url.to_string()], 1)
        .await
        .expect("run_axe_batch");
    let keys: Vec<String> = axe.keys().cloned().collect();
    let violations = axe
        .remove(url)
        .unwrap_or_else(|| panic!("pas de résultat axe pour {url} ; clés reçues : {keys:?}"));
    let mapped = AxeMapper::map(&violations).expect("mapping axe");
    let axe_status = mapped.get("12.7").map(|r| r.status.clone());

    let snippets: HashMap<String, String> = GapFixRules::snippets()
        .iter()
        .filter(|(id, _)| id.as_str() == "12.7")
        .map(|(id, s)| (id.clone(), (*s).to_string()))
        .collect();
    let mut probes =
        ObscuraBridge::run_gap_fix_batch(binary.to_string(), vec![url.to_string()], snippets, 1)
            .await
            .expect("run_gap_fix_batch");
    let probes_keys: Vec<String> = probes.keys().cloned().collect();
    let raw = probes.remove(url).unwrap_or_else(|| {
        panic!("pas de résultat de sonde pour {url} ; clés reçues : {probes_keys:?}")
    });
    let probe_status = GapFixRules::parse_results(&raw)
        .get("12.7")
        .map(|r| r.status.clone());
    (axe_status, probe_status)
}

#[tokio::test]
async fn criterion_12_7_does_not_pass_a_site_from_a_single_page() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("criterion_12_7_coverage ignoré (poser RUN_E2E=1 pour l'activer)");
        return;
    }
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");

    let mut server = ObscuraBridge::new().with_port(9343);
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP d'Obscura");
    let bridge = Arc::new(server);
    let binary = bridge.binary_path().to_string();

    // Témoin : aucun mécanisme d'évitement du tout. La sonde doit le voir — sans elle en
    // échec, un résultat « conforme » sur la bonne page ne prouverait rien. axe, lui, rend
    // déjà `Pass` ici malgré l'absence totale de mécanisme : la preuve la plus directe que
    // sa `Pass` sur 12.7 ne découle d'aucune règle qui ait réellement tranché quoi que ce
    // soit — coverage="complete" l'initialise à `Pass` et rien dans cette page ne porte un
    // nom de règle mappé pour le faire basculer.
    let control = format!("http://127.0.0.1:{port}/coverage-12-7-no-skip-control.html");
    let (axe_control, probe_control) = audit_12_7(&bridge, &binary, &control).await;
    eprintln!("témoin sans évitement : axe -> {axe_control:?}, sonde -> {probe_control:?}");
    assert_ne!(
        probe_control,
        Some(CriterionStatus::Pass),
        "le témoin ne doit pas passer la sonde : aucun texte d'évitement reconnu n'existe"
    );

    // Une page seule, correcte pour 12.7.1 (lien d'évitement présent et fonctionnel).
    let page = format!("http://127.0.0.1:{port}/coverage-12-7-single-page-skip-link.html");
    let (axe_status, probe_status) = audit_12_7(&bridge, &binary, &page).await;
    eprintln!("page seule, 12.7.1 correct : axe -> {axe_status:?}, sonde -> {probe_status:?}");

    // Ni axe ni la sonde ne décident jamais 12.7.2 : aucun des deux ne compare la page à
    // aucune autre. Un `Pass` ici affirmerait une cohérence inter-pages jamais vérifiée.
    assert_ne!(
        axe_status,
        Some(CriterionStatus::Pass),
        "axe rend Pass pour 12.7 depuis une seule page, alors que 12.7.2 porte sur un \
         ensemble de pages qu'aucune règle axe ne compare"
    );
    assert_ne!(
        probe_status,
        Some(CriterionStatus::Pass),
        "la sonde rend Pass pour 12.7 depuis une seule page, alors que 12.7.2 porte sur un \
         ensemble de pages qu'aucune sonde ne compare"
    );
}
