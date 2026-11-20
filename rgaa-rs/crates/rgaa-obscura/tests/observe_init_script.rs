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

#[tokio::test]
async fn scripted_redirects_are_seen_and_carousels_are_not() {
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
    let window = Duration::from_millis(900);

    let redirect = server
        .observe_scripted_redirects(
            &format!("http://127.0.0.1:{port}/init-script-redirect.html"),
            window,
        )
        .await
        .expect("observation de la page redirigeante");
    assert_eq!(redirect.calls.len(), 1, "reçu : {redirect:?}");
    assert_eq!(redirect.calls[0].method, "replace");
    assert!(redirect.calls[0].target.contains("redirected"));
    assert!(
        redirect.calls[0].at_ms >= 200,
        "délai attendu ~300 ms : {redirect:?}"
    );
    eprintln!(
        "points d'entrée non interceptables : {:?}",
        redirect.unhooked
    );

    let carousel = server
        .observe_scripted_redirects(
            &format!("http://127.0.0.1:{port}/init-script-carousel.html"),
            window,
        )
        .await
        .expect("observation du carrousel");
    assert!(
        carousel.calls.is_empty(),
        "un carrousel setInterval n'est pas une redirection : {carousel:?}"
    );

    let href = server
        .observe_scripted_redirects(
            &format!("http://127.0.0.1:{port}/init-script-href.html"),
            window,
        )
        .await
        .expect("observation de l'affectation de href");
    assert_eq!(href.calls.len(), 1, "reçu : {href:?}");
    assert_eq!(href.calls[0].method, "href");
}
