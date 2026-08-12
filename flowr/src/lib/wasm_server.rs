//! Minimal HTTP file server for serving WASM implementations to remote executors.
//!
//! When the coordinator dispatches jobs with `file://` implementation URLs,
//! remote executors cannot access those files. This server makes them available
//! over HTTP so executors can fetch WASM modules using the existing `HttpProvider`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;

use log::{debug, error, info, trace};

/// A background HTTP server that serves files from a root directory.
pub struct WasmServer {
    /// The base URL where files are served (e.g., `http://192.168.1.1:12345`)
    base_url: String,
    /// The root directory from which files are served
    _root: PathBuf,
}

impl WasmServer {
    /// Start a new WASM file server on a random port, serving files from `root`.
    ///
    /// Returns the server handle with the base URL for constructing file URLs.
    /// The server runs in a background thread and stops when the process exits.
    ///
    /// # Errors
    ///
    /// Returns an error if the TCP listener cannot be bound.
    pub fn start(root: &Path) -> Result<Self, String> {
        let listener = TcpListener::bind("0.0.0.0:0")
            .map_err(|e| format!("Could not bind WASM server: {e}"))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| format!("Could not get WASM server address: {e}"))?;

        // Use the machine's actual IP for remote access, not 0.0.0.0
        let ip = local_ip().unwrap_or_else(|| local_addr.ip());
        let base_url = format!("http://{ip}:{}", local_addr.port());

        info!(
            "WASM server listening on {base_url}, serving from {}",
            root.display()
        );

        let serve_root = root.to_path_buf();
        thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    // Set timeouts to prevent hung connections
                    let timeout = Some(std::time::Duration::from_secs(30));
                    let _ = stream.set_read_timeout(timeout);
                    let _ = stream.set_write_timeout(timeout);
                    let root = serve_root.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_request(stream, &root) {
                            debug!("WASM server request error: {e}");
                        }
                    });
                } else {
                    error!("WASM server accept error");
                    break;
                }
            }
        });

        Ok(WasmServer {
            base_url,
            _root: root.to_path_buf(),
        })
    }

    /// Get the base URL of this server (e.g., `http://192.168.1.1:12345`)
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Send an HTTP error response with the given status code and reason.
fn send_error(stream: &mut std::net::TcpStream, code: u16, reason: &str) -> Result<(), String> {
    let response = format!("HTTP/1.1 {code} {reason}\r\nContent-Length: 0\r\n\r\n");
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("Could not write {code} response: {e}"))
}

/// Handle a single HTTP request, serving a file from `root`.
fn handle_request(mut stream: std::net::TcpStream, root: &Path) -> Result<(), String> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| format!("Could not read request: {e}"))?;

    // Parse "GET /path HTTP/1.x"
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let Some(path) = parts.next() else {
        return send_error(&mut stream, 400, "Bad Request");
    };

    let is_head = method == "HEAD";
    if method != "GET" && !is_head {
        return send_error(&mut stream, 405, "Method Not Allowed");
    }

    // Consume remaining headers (read until empty line)
    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|e| format!("Could not read header: {e}"))?;
        if header.trim().is_empty() {
            break;
        }
    }

    // Resolve URL path against root (strip leading slash)
    let relative_path = path.trim_start_matches('/');
    let file_path = root.join(relative_path);

    // Security: only serve .wasm files
    if file_path.extension().is_none_or(|ext| ext != "wasm") {
        return send_error(&mut stream, 403, "Forbidden");
    }

    // Security: ensure the resolved path is under root
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Could not canonicalize root: {e}"))?;

    let Ok(canonical_file) = file_path.canonicalize() else {
        return send_error(&mut stream, 404, "Not Found");
    };

    if !canonical_file.starts_with(&canonical_root) {
        return send_error(&mut stream, 403, "Forbidden");
    }

    // Read and serve the file
    if let Ok(mut file) = std::fs::File::open(&canonical_file) {
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| format!("Could not read file: {e}"))?;

        trace!(
            "WASM server: serving {} ({} bytes, HEAD={is_head})",
            canonical_file.display(),
            contents.len()
        );

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/wasm\r\nContent-Length: {}\r\n\r\n",
            contents.len()
        );
        stream
            .write_all(header.as_bytes())
            .map_err(|e| format!("Could not write header: {e}"))?;
        if !is_head {
            stream
                .write_all(&contents)
                .map_err(|e| format!("Could not write body: {e}"))?;
        }
    } else {
        send_error(&mut stream, 404, "Not Found")?;
    }

    Ok(())
}

/// Get the machine's local (non-loopback) IPv4 address.
fn local_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    /// Make a raw HTTP GET request and return (status code, body).
    fn http_get(url: &str) -> (u16, Vec<u8>) {
        let url = url::Url::parse(url).unwrap();
        let host = url.host_str().unwrap();
        let port = url.port().unwrap();
        let path = url.path();

        let mut stream = TcpStream::connect(format!("{host}:{port}")).unwrap();
        let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response);

        // Parse status code from first line
        let status_code: u16 = response_str
            .lines()
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        // Find body after \r\n\r\n
        let body = response
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map_or_else(Vec::new, |pos| {
                response.get(pos + 4..).unwrap_or(&[]).to_vec()
            });

        (status_code, body)
    }

    /// Build a localhost URL for the server, replacing the network IP
    /// with 127.0.0.1. The server binds to 0.0.0.0 so it accepts localhost
    /// connections. Using localhost avoids flakiness when `local_ip()`
    /// returns a network IP that is unreachable during parallel test runs.
    fn localhost_url(server: &WasmServer) -> String {
        let url = url::Url::parse(server.base_url()).unwrap();
        format!("http://127.0.0.1:{}", url.port().unwrap())
    }

    #[test]
    fn serves_wasm_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let wasm_content = b"\x00asm\x01\x00\x00\x00";
        let wasm_path = dir.path().join("test.wasm");
        std::fs::File::create(&wasm_path)
            .unwrap()
            .write_all(wasm_content)
            .unwrap();

        let server = WasmServer::start(dir.path()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let (status, body) = http_get(&format!("{}/test.wasm", localhost_url(&server)));
        assert_eq!(status, 200);
        assert_eq!(body, wasm_content);
    }

    #[test]
    fn returns_404_for_missing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let server = WasmServer::start(dir.path()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let (status, _) = http_get(&format!("{}/nonexistent.wasm", localhost_url(&server)));
        assert_eq!(status, 404);
    }
}
