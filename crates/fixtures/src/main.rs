//! `app-fixtures`: scrub recorded fixtures, or check that committed ones are clean.
//!
//! ```text
//! app-fixtures scrub [--secret VALUE]... [--addon URL]... FILE...   rewrite files in place
//! app-fixtures check [--secret VALUE]... [--addon URL]... PATH...   fail if anything would change
//! ```
//!
//! Both also anonymize stream and subtitle content (D62, [`app_fixtures::anonymize`]).
//!
//! Secrets can also come from the `FIXTURE_SECRETS` environment variable (comma- or
//! newline-separated), so real keys never have to appear on the command line.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use app_fixtures::{Finding, Scrubber, anonymize};

const USAGE: &str = "usage:
  app-fixtures scrub [--secret VALUE]... [--addon URL]... FILE...
  app-fixtures check [--secret VALUE]... [--addon URL]... PATH...

Secrets are also read from FIXTURE_SECRETS (comma- or newline-separated).
check walks directories and looks at every .json file.";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let mut scrubber = Scrubber::new();
    if let Ok(env) = std::env::var("FIXTURE_SECRETS") {
        for secret in env.split([',', '\n']).map(str::trim) {
            scrubber = scrubber.secret(secret);
        }
    }
    let mut paths = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--secret" | "--addon" => {
                let Some(value) = args.next() else {
                    eprintln!("{arg} needs a value\n\n{USAGE}");
                    return ExitCode::FAILURE;
                };
                scrubber = if arg == "--secret" {
                    scrubber.secret(value)
                } else {
                    scrubber.addon_url(&value)
                };
            }
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    if paths.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }

    let result = match command.as_str() {
        "scrub" => scrub(&scrubber, &paths),
        "check" => check(&scrubber, &paths),
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Errors name files, and file names can hold the very secrets we were given.
            eprintln!("{}", scrubber.scrub_str(&err).0);
            ExitCode::FAILURE
        }
    }
}

fn scrub(scrubber: &Scrubber, files: &[PathBuf]) -> Result<(), String> {
    for file in files {
        let mut value = read_json(file)?;
        let mut findings = scrubber.scrub(&mut value);
        findings.extend(anonymize::anonymize(&mut value));
        let shown = scrubber.scrub_str(&file.display().to_string()).0;
        if findings.is_empty() {
            println!("{shown}: clean");
            continue;
        }
        let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())? + "\n";
        std::fs::write(file, text).map_err(|e| format!("writing {}: {e}", file.display()))?;
        println!("{shown}: scrubbed/anonymized {} value(s)", findings.len());
    }
    Ok(())
}

fn check(scrubber: &Scrubber, roots: &[PathBuf]) -> Result<(), String> {
    let mut files = Vec::new();
    for root in roots {
        collect_json(root, &mut files)?;
    }
    let mut dirty = 0;
    for file in &files {
        // Diagnostics name files, so they go through the scrubber too.
        let shown = scrubber.scrub_str(&file.display().to_string()).0;
        if let Some(reason) = scrubber.check_path(&file.to_string_lossy()) {
            eprintln!("{shown}: {reason} in the file name");
            dirty += 1;
        }
        let value = read_json(file)?;
        let findings: Vec<Finding> = scrubber
            .check(&value)
            .into_iter()
            .chain(anonymize::check(&value))
            .collect();
        for finding in findings {
            eprintln!("{shown}: {} at {}", finding.reason, finding.pointer);
            dirty += 1;
        }
    }
    if dirty == 0 {
        println!("fixtures: {} file(s) clean", files.len());
        Ok(())
    } else {
        Err(format!(
            "fixtures: {dirty} value(s) look secret or not anonymized; run `app-fixtures scrub` on the file(s) above"
        ))
    }
}

fn read_json(file: &Path) -> Result<serde_json::Value, String> {
    let text =
        std::fs::read_to_string(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing {}: {e}", file.display()))
}

fn collect_json(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "json") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    let entries =
        std::fs::read_dir(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut children: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    children.sort();
    for child in children {
        collect_json(&child, out)?;
    }
    Ok(())
}
