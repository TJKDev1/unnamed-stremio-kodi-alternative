//! Spike S4 shell: the IPC benchmark page in Tauri v2 (WebView2 on Windows). Runs the same
//! sessions over Tauri IPC (commands below) and over WebSocket (the core's server on
//! 127.0.0.1:7311). Throwaway code (D65).
//!
//! usage: s4-shell [--auto]   (--auto: run everything, save results, quit)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::sync::mpsc;

use s4_core::server::serve_ws;
use s4_core::session::{Command, Reply, Session, handle};
use tauri::ipc::Response;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const WS_ADDR: &str = "127.0.0.1:7311";

struct State(Mutex<Option<Session>>);

/// T0.S4.4 (Tauri half): control messages as JSON strings, frames as raw bytes
/// (`tauri::ipc::Response` skips JSON: the page receives an ArrayBuffer).
#[tauri::command]
fn ipc(state: tauri::State<'_, State>, msg: String) -> Response {
    let cmd: Command = match serde_json::from_str(&msg) {
        Ok(c) => c,
        Err(e) => return Response::new(format!("bad command: {e}").into_bytes()),
    };
    let mut session = state.0.lock().unwrap();
    match handle(cmd, &mut session) {
        Reply::None | Reply::Results(_) => Response::new(Vec::new()),
        Reply::Text(t) => Response::new(t.into_bytes()),
        Reply::Binary(b) => Response::new(b),
    }
}

fn main() {
    let auto = std::env::args().any(|a| a == "--auto");
    let results =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.cache/s4-results");
    let (tx, rx) = mpsc::channel();
    serve_ws(WS_ADDR, tx).expect("websocket server");
    tauri::Builder::default()
        .manage(State(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![ipc])
        .setup(move |app| {
            let query = format!(
                "index.html?ws=ws://{WS_ADDR}/&transports=tauri,ws,replay{}",
                if auto { "&auto=1" } else { "" }
            );
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App(query.into()))
                .title("S4 IPC bench")
                .inner_size(1600.0, 1000.0)
                .build()?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                while let Ok(data) = rx.recv() {
                    std::fs::create_dir_all(&results).unwrap();
                    let file = results.join(format!(
                        "{}-webview.json",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    ));
                    std::fs::write(&file, serde_json::to_string_pretty(&data).unwrap()).unwrap();
                    eprintln!("{}", data["summary"].as_str().unwrap_or(""));
                    eprintln!("results: {}", file.display());
                    if auto {
                        handle.exit(if data["ok"] == true { 0 } else { 1 });
                    }
                }
            });
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("tauri");
}
