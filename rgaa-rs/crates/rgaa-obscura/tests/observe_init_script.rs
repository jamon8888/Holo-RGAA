//! `observe_with_init_script` : le script d'initialisation s'exécute avant les scripts de
//! la page, et la page est observée sans interaction. Primitive de #265 (13.1).
//!
//! Câblé sous `RUN_E2E=1` (binaire Obscura sur le PATH).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rgaa_obscura::ObscuraBridge;

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

// Records every `location.replace` / `assign` call before any page script runs. Obscura
// exposes no `Location` constructor: the methods are own, writable properties of the
// `location` object (measured on 0.2.4), so the hook patches them there.
const HOOK: &str = r#"
window.__nav = [];
for (const m of ['replace', 'assign']) {
  try {
    location[m] = function (u) { window.__nav.push(m + ':' + u); };
  } catch (e) { window.__nav.push('unhookable:' + m); }
}
"#;

#[tokio::test]
async fn init_script_runs_before_page_scripts_and_observes_without_interaction() {
    if !rgaa_test_corpus::e2e::enabled("observe_init_script") {
        return;
    }
    rgaa_test_corpus::e2e::allow_private_network();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let port = serve(dir).expect("fixture server");
    let mut server = ObscuraBridge::new()
        .with_ephemeral_port()
        .expect("port CDP libre");
    server
        .start_server()
        .await
        .expect("démarrage du serveur CDP");

    let read = "JSON.stringify(window.__nav || null)";
    let redirect = server
        .observe_with_init_script(
            &format!("http://127.0.0.1:{port}/init-script-redirect.html"),
            HOOK,
            Duration::from_millis(900),
            read,
        )
        .await
        .expect("observation de la page redirigeante");
    let seen = redirect["result"]["value"].as_str().unwrap_or("");
    assert!(
        seen.contains("replace:about:blank#redirected"),
        "le hook doit voir la redirection par script ; reçu : {redirect}"
    );

    let carousel = server
        .observe_with_init_script(
            &format!("http://127.0.0.1:{port}/init-script-carousel.html"),
            HOOK,
            Duration::from_millis(900),
            read,
        )
        .await
        .expect("observation du carrousel");
    assert_eq!(
        carousel["result"]["value"].as_str(),
        Some("[]"),
        "un carrousel setInterval n'est pas une redirection ; reçu : {carousel}"
    );
}
