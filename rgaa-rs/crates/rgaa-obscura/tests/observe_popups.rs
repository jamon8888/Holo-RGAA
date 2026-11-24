//! Fenêtres ouvertes sans action de l'utilisateur (#266, 13.2).
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
async fn only_automatic_popups_are_reported() {
    if !rgaa_test_corpus::e2e::enabled("observe_popups") {
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

    for (page, expected) in [
        ("popup-onload.html", 1),
        ("popup-timer.html", 1),
        ("popup-link.html", 0),
        ("popup-click.html", 0),
    ] {
        let popups = server
            .observe_auto_popups(
                &format!("http://127.0.0.1:{port}/{page}"),
                Duration::from_millis(900),
            )
            .await
            .unwrap_or_else(|e| panic!("{page} : {e}"));
        assert_eq!(popups.len(), expected, "{page} : {popups:?}");
    }
}
