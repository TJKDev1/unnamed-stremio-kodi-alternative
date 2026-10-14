//! Serves the built page and the WebSocket core, for a plain browser or for CI.
//!
//! usage: serve [--web DIR] [--port N] [--browser PATH] [--results FILE]
//!
//! The page is on http://127.0.0.1:N/ and the WebSocket core on port N+1. With `--browser`,
//! starts that browser headless on the page with `?auto=1`, waits for the page to send its
//! results, writes them to FILE, prints a summary and exits non-zero if a run's final view-
//! model didn't match the core's (perf numbers are reported, not enforced: CI runners vary).

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use s4_core::server::{serve_static, serve_ws};

fn main() {
    let mut web = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web/dist");
    let mut port: u16 = 7310;
    let mut browser = None;
    let mut results =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.cache/s4-results/results.json");
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--web" => web = it.next().expect("--web DIR").into(),
            "--port" => port = it.next().and_then(|p| p.parse().ok()).expect("--port N"),
            "--browser" => browser = it.next(),
            "--results" => results = it.next().expect("--results FILE").into(),
            other => panic!("unknown argument {other}"),
        }
    }
    let (tx, rx) = mpsc::channel();
    serve_static(&format!("127.0.0.1:{port}"), web).expect("static server");
    serve_ws(&format!("127.0.0.1:{}", port + 1), tx).expect("websocket server");
    let url = format!("http://127.0.0.1:{port}/?ws=ws://127.0.0.1:{}/", port + 1);
    let Some(browser) = browser else {
        println!("page: {url}  (add &auto=1 to run everything)");
        loop {
            let data = rx.recv().unwrap();
            save(&results, &data);
        }
    };
    let profile = std::env::temp_dir().join(format!("s4-profile-{}", std::process::id()));
    let mut child = Command::new(browser)
        .args([
            "--headless",
            "--no-sandbox",
            "--disable-gpu",
            "--no-first-run",
            "--remote-debugging-port=0",
        ])
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("{url}&auto=1"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start browser");
    let data = rx.recv_timeout(Duration::from_secs(300));
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&profile);
    let data = data.expect("no results from the page within 5 minutes");
    save(&results, &data);
    println!("{}", data["summary"].as_str().unwrap_or("(no summary)"));
    if data["ok"] != true {
        eprintln!(
            "serve: a run's final view-model differs from the core's (see {})",
            results.display()
        );
        std::process::exit(1);
    }
}

fn save(path: &PathBuf, data: &serde_json::Value) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_string_pretty(data).unwrap()).unwrap();
    eprintln!("serve: results written to {}", path.display());
}
