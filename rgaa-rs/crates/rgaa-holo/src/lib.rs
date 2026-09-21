pub mod backend;
pub mod cassette;
pub mod client;
pub mod ollama;
pub mod prompts;
pub mod transport;

pub use backend::{BackendConfig, LlmBackend};
pub use cassette::{Cassette, CassetteBackend, CassetteEntry};
pub use client::HoloClient;
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
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let last_request = Arc::new(Mutex::new(String::new()));
        let recorder = Arc::clone(&last_request);

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { break };
                let recorder = Arc::clone(&recorder);
                std::thread::spawn(move || {
                    let mut buf = vec![0u8; 16 * 1024];
                    let n = s.read(&mut buf).unwrap_or(0);
                    *recorder.lock().unwrap() = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
}
