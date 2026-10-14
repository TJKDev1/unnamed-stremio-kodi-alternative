//! T0.S4.4 (WebSocket half): the remote-core transport. One session per connection;
//! control messages are JSON text, frames go back as binary messages. Plus a static file
//! server for the built page, so a plain browser (or headless Chromium 94 in CI) can run it.

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;

use serde_json::Value;
use tungstenite::Message;

use crate::session::{Command, Reply, handle};

/// Serves WebSocket sessions on `addr` forever, one thread per connection. Results
/// messages from the page are forwarded to `results`.
pub fn serve_ws(addr: &str, results: Sender<Value>) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let results = results.clone();
            thread::spawn(move || {
                let _ = stream.set_nodelay(true);
                let Ok(mut ws) = tungstenite::accept(stream) else {
                    return;
                };
                let mut session = None;
                while let Ok(msg) = ws.read() {
                    let text = match msg {
                        Message::Text(t) => t,
                        Message::Close(_) => break,
                        _ => continue,
                    };
                    let reply = match serde_json::from_str::<Command>(&text) {
                        Ok(cmd) => handle(cmd, &mut session),
                        Err(e) => Reply::Text(format!("bad command: {e}")),
                    };
                    let sent = match reply {
                        Reply::None => Ok(()),
                        Reply::Text(t) => ws.send(Message::text(t)),
                        Reply::Binary(b) => ws.send(Message::binary(b)),
                        Reply::Results(v) => {
                            let _ = results.send(v);
                            ws.send(Message::text("ok"))
                        }
                    };
                    if sent.is_err() {
                        break;
                    }
                }
            });
        }
    });
    Ok(())
}

/// Serves files under `root` on `addr` forever.
pub fn serve_static(addr: &str, root: PathBuf) -> std::io::Result<()> {
    let server = tiny_http::Server::http(addr).map_err(std::io::Error::other)?;
    thread::spawn(move || {
        for req in server.incoming_requests() {
            let path = req
                .url()
                .split('?')
                .next()
                .unwrap_or("/")
                .trim_start_matches('/');
            let file = if path.is_empty() {
                root.join("index.html")
            } else {
                root.join(path)
            };
            let safe = !path.split('/').any(|s| s == "..");
            let response = match fs::read(&file) {
                Ok(body) if safe => {
                    let header =
                        tiny_http::Header::from_bytes("content-type", content_type(&file)).unwrap();
                    tiny_http::Response::from_data(body)
                        .with_header(header)
                        .boxed()
                }
                _ => tiny_http::Response::empty(404).boxed(),
            };
            let _ = req.respond(response);
        }
    });
    Ok(())
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        _ => "application/octet-stream",
    }
}
