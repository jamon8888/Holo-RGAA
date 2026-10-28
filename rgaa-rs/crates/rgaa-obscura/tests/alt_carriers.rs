//! Les sept porteurs d'alternative textuelle du critère RGAA 1.3 (tests 1.3.1 à 1.3.8).
//!
//! Prérequis de la calibration par famille de preuve (#354) : l'extraction et le rendu
//! diffèrent selon l'élément porteur, et c'est là que vivent les pannes silencieuses.
//! Un test à fixtures les attrape à chaque commit, là où une campagne d'étiquetage ne les
//! verrait qu'une fois — cf. le défaut des 64 octets de `rgaa-holo` (#353).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use rgaa_obscura::ObscuraBridge;
/// Sert le répertoire de fixtures en HTTP sur la boucle locale : Obscura refuse `file://`.
fn serve(dir: PathBuf) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
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
        }
    });
    Ok(port)
}

/// (test RGAA, porteur, alternative attendue dans la fixture)
const CARRIERS: &[(&str, &str, &str)] = &[
    ("1.3.1", "img", "ALT-IMG"),
    ("1.3.2", "area (dans une map)", "ALT-AREA"),
    ("1.3.3", "input type=image", "ALT-INPUT"),
    ("1.3.4", "object type=image/*", "ALT-OBJECT"),
    ("1.3.5", "embed type=image/*", "ALT-EMBED"),
    ("1.3.6", "svg (title/desc)", "ALT-SVG"),
    ("1.3.7", "canvas", "ALT-CANVAS"),
];

#[tokio::test]
async fn page_context_extracts_every_alt_carrier_of_criterion_1_3() {
    // Même porte que `fixture_classification` : ce test a besoin du binaire Obscura.
    if !rgaa_test_corpus::e2e::enabled("alt_carriers") {
        return;
    }
    // Les fixtures sont servies sur 127.0.0.1, qu'Obscura bloque par défaut (garde SSRF) ;
    // sans cette variable la page se charge en document vide de 39 octets et le test
    // passerait pour de mauvaises raisons.
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");
    let url = format!("http://127.0.0.1:{port}/alt-carriers.html");

    let bridge = ObscuraBridge::new();
    let mut by_url = ObscuraBridge::extract_page_context_batch(
        bridge.binary_path().to_string(),
        vec![url.clone()],
        1,
    )
    .await
    .expect("extract_page_context_batch");
    let ctx = by_url.remove(&url).expect("context for the fixture URL");

    // Garde-fou : si la page n'a pas vraiment été chargée, tout le reste est sans valeur.
    assert_eq!(
        ctx.get("title").and_then(|t| t.as_str()),
        Some("Porteurs d'alternative — fixture 1.3.x"),
        "la fixture n'a pas été chargée ; vérifier le garde SSRF et le serveur local"
    );

    let haystack = serde_json::to_string(&ctx).expect("serialise context");
    let missing: Vec<String> = CARRIERS
        .iter()
        .filter(|(_, _, alt)| !haystack.contains(alt))
        .map(|(test, carrier, _)| format!("{test} ({carrier})"))
        .collect();

    assert!(
        missing.is_empty(),
        "porteurs d'alternative absents du contexte extrait : {missing:?}\n\
         contexte extrait : {haystack}"
    );
}
