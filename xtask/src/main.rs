//! Repo maintenance tasks. Run with `cargo xtask <task>`.

mod licenses;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <task>

tasks:
  check-licenses   every crate/package declares the license D37/D59 require";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("check-licenses") => licenses::run(),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
