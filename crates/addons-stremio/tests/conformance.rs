//! Stremio v1 conformance on recorded fixtures (spec: Conformance tests). Every addon folder in
//! `fixtures/stremio/` must have a manifest that validates and responses that parse with zero
//! dropped items.

use std::path::{Path, PathBuf};

use app_addons_stremio::{Manifest, parse_resource};
use serde_json::Value;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/stremio")
}

#[test]
fn every_recorded_response_parses_cleanly() {
    let dir = fixtures_dir();
    let mut addons: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures/stremio exists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    addons.sort();
    assert!(!addons.is_empty(), "no recordings in {}", dir.display());

    let mut failures = Vec::new();
    let mut responses = 0;
    for addon in &addons {
        let name = addon.file_name().unwrap().to_string_lossy();
        let manifest = std::fs::read(addon.join("manifest.json")).unwrap();
        if let Err(e) = Manifest::from_json(&manifest) {
            failures.push(format!("{name}/manifest.json: {e}"));
        }

        let index: Value =
            serde_json::from_slice(&std::fs::read(addon.join("index.json")).unwrap()).unwrap();
        for request in index["requests"].as_array().unwrap() {
            let Some(file) = request["file"].as_str() else {
                continue;
            };
            let resource = file.split('/').next().unwrap();
            let body = std::fs::read(addon.join(file)).unwrap();
            match parse_resource(resource, &body) {
                Ok(parsed) if parsed.dropped.is_empty() => responses += 1,
                Ok(parsed) => failures.push(format!(
                    "{name}/{file}: {} dropped, first: {:?}",
                    parsed.dropped.len(),
                    parsed.dropped[0]
                )),
                Err(e) => failures.push(format!("{name}/{file}: {e}")),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("{} addons, {responses} responses", addons.len());
}
