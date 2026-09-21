pub mod backend;
pub mod cassette;
pub mod client;
pub mod fallback;
pub mod ollama;
pub mod prompts;
pub mod transport;

pub use backend::{BackendConfig, LlmBackend};
pub use cassette::{Cassette, CassetteBackend, CassetteEntry};
pub use client::HoloClient;
pub use fallback::{BenchmarkResult, FallbackBackend};
pub use ollama::OllamaClient;
pub use prompts::{format_page_context, PageContext, PromptBuilder};
pub use transport::HoloResponse;

#[cfg(test)]
pub(crate) mod test_util {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    pub(crate) struct MockServer {
        addr: String,
        last_request: Arc<Mutex<String>>,
    }

    impl MockServer {
        pub(crate) fn url(&self) -> String {
            format!("http://{}", self.addr)
        }

        /// Raw bytes of the most recent request (headers + body).
        pub(crate) fn last_request(&self) -> String {
            self.last_request.lock().unwrap().clone()
        }
    }

    /// Minimal HTTP server answering every request with a fixed JSON body and
    /// recording the raw request, so tests can assert headers and payload
    /// without touching a real API.
    pub(crate) fn spawn_mock_server(body: &'static str) -> MockServer {
        spawn_mock_server_with_status(200, body)
    }

    /// As [`spawn_mock_server`], but answers with `status` instead of
    /// always 200 — used to exercise failure/fallback paths without
    /// touching a real API.
    pub(crate) fn spawn_mock_server_with_status(status: u16, body: &'static str) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let last_request = Arc::new(Mutex::new(String::new()));
        let recorder = Arc::clone(&last_request);
        let status_line = status_line_for(status);

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { break };
                let recorder = Arc::clone(&recorder);
                let status_line = status_line.clone();
                std::thread::spawn(move || {
                    let mut buf = vec![0u8; 16 * 1024];
                    let n = s.read(&mut buf).unwrap_or(0);
                    *recorder.lock().unwrap() = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let response = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = s.write_all(response.as_bytes());
                    let _ = s.flush();
                });
            }
        });

        MockServer { addr, last_request }
    }

    fn status_line_for(status: u16) -> String {
        let reason = match status {
            200 => "OK",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            _ => "Error",
        };
        format!("{status} {reason}")
    }
}
