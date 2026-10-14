//! License policy check (T0.1.4, D37, D59).
//!
//! Every Cargo package and every `package.json` must declare the license its location
//! requires. Packages whose license differs from the root `LICENSE` (GPL) must have a
//! `LICENSE` file with the matching text, in their own directory or a parent below the root.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

pub const GPL: &str = "GPL-3.0-or-later";
pub const AGPL: &str = "AGPL-3.0-or-later";
pub const MIT: &str = "MIT";

/// The license a package at `rel` (path relative to the repo root, `/`-separated) must use.
///
/// - Server and sync code: AGPL, so hosted forks must share their changes.
/// - SDKs and the schema that generates SDK bindings: MIT, so addon/theme authors license freely.
/// - Everything else (the app): GPL.
pub fn required_license(rel: &str) -> &'static str {
    let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        ["crates", "server", ..] => AGPL,
        ["crates", "sdk-schema", ..] | ["ui", "sdk", ..] | ["sdk", ..] => MIT,
        _ if parts.iter().any(|p| p.contains("sync")) => AGPL,
        _ => GPL,
    }
}

/// FNV-1a 64 of the whitespace-normalised GNU texts (as published at gnu.org/licenses).
const GPL_TEXT_HASH: u64 = 0x2ee7eb2f1d9cbd2e;
const AGPL_TEXT_HASH: u64 = 0x7dae94fddee25626;

/// The MIT license after its copyright line(s), whitespace-normalised.
const MIT_BODY: &str = "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions: The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software. THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.";

/// True if `text` is the complete, unmodified text of `license` (whitespace aside).
/// MIT may have any copyright line(s) between the title and the body.
fn is_license_text(text: &str, license: &str) -> bool {
    let norm = normalize_whitespace(text);
    match license {
        GPL => fnv1a64(&norm) == GPL_TEXT_HASH,
        AGPL => fnv1a64(&norm) == AGPL_TEXT_HASH,
        MIT => {
            norm.starts_with("MIT License Copyright")
                && norm.ends_with(MIT_BODY)
                && !norm[..norm.len() - MIT_BODY.len()].contains("Permission")
        }
        _ => false,
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fnv1a64(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |hash, b| {
        (hash ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

struct Package {
    kind: &'static str,
    name: String,
    dir: PathBuf,
    license: Option<String>,
}

pub fn run() -> ExitCode {
    match check() {
        Ok(count) => {
            println!("check-licenses: {count} packages ok");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for err in &errors {
                eprintln!("error: {err}");
            }
            eprintln!(
                "check-licenses: {} problem(s); the policy is in xtask/src/licenses.rs (D37, D59)",
                errors.len()
            );
            ExitCode::FAILURE
        }
    }
}

fn check() -> Result<usize, Vec<String>> {
    let root = repo_root().map_err(|e| vec![e])?;
    let mut packages = cargo_packages(&root).map_err(|e| vec![e])?;
    npm_packages(&root, &mut packages).map_err(|e| vec![e])?;

    let mut errors = Vec::new();
    if let Err(err) = check_license_file(&root, &root, GPL) {
        errors.push(format!("repo root: {err}"));
    }
    for pkg in &mut packages {
        // Canonical paths, so prefix checks hold on Windows (`\\?\`, drive-letter case).
        match std::fs::canonicalize(&pkg.dir) {
            Ok(dir) if dir.starts_with(&root) => pkg.dir = dir,
            _ => errors.push(format!(
                "{} {}: {} is outside the repo",
                pkg.kind,
                pkg.name,
                pkg.dir.display()
            )),
        }
    }
    for pkg in packages.iter().filter(|p| p.dir.starts_with(&root)) {
        let rel = relative(&root, &pkg.dir);
        let want = required_license(&rel);
        let label = format!(
            "{} {} ({})",
            pkg.kind,
            pkg.name,
            if rel.is_empty() { "." } else { &rel }
        );
        match pkg.license.as_deref() {
            Some(have) if have == want => {}
            Some(have) => errors.push(format!(
                "{label}: license is \"{have}\", must be \"{want}\""
            )),
            None => errors.push(format!("{label}: no license declared, must be \"{want}\"")),
        }
        if let Err(err) = check_license_file(&root, &pkg.dir, want) {
            errors.push(format!("{label}: {err}"));
        }
    }
    if errors.is_empty() {
        Ok(packages.len())
    } else {
        Err(errors)
    }
}

fn repo_root() -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate the repo root")?;
    std::fs::canonicalize(root).map_err(|e| format!("resolving {}: {e}", root.display()))
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Looks for the nearest `LICENSE` from `dir` upwards. Non-GPL packages must find theirs
/// below the root (the root `LICENSE` is the GPL); the root itself must have the GPL.
fn check_license_file(root: &Path, dir: &Path, license: &str) -> Result<(), String> {
    let mut current = Some(dir);
    while let Some(d) = current.filter(|d| d.starts_with(root)) {
        let file = d.join("LICENSE");
        if file.is_file() {
            let text = std::fs::read_to_string(&file)
                .map_err(|e| format!("reading {}: {e}", file.display()))?;
            return if is_license_text(&text, license) {
                Ok(())
            } else {
                Err(format!(
                    "nearest license file {} is not the complete {license} text",
                    relative(root, &file)
                ))
            };
        }
        current = d.parent();
    }
    Err(format!("needs a LICENSE file with the {license} text"))
}

fn cargo_packages(root: &Path) -> Result<Vec<Package>, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .map_err(|e| format!("running cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("parsing cargo metadata: {e}"))?;
    let packages = meta["packages"]
        .as_array()
        .ok_or("cargo metadata has no packages")?;
    packages
        .iter()
        .map(|p| {
            let manifest = p["manifest_path"]
                .as_str()
                .ok_or("package without manifest_path")?;
            Ok(Package {
                kind: "crate",
                name: p["name"].as_str().unwrap_or("?").to_owned(),
                dir: Path::new(manifest)
                    .parent()
                    .ok_or("bad manifest path")?
                    .to_path_buf(),
                license: p["license"].as_str().map(str::to_owned),
            })
        })
        .collect()
}

/// Every `package.json` in the repo, skipping dependencies, build output and dot-directories.
fn npm_packages(root: &Path, out: &mut Vec<Package>) -> Result<(), String> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let manifest = dir.join("package.json");
        if manifest.is_file() {
            let text = std::fs::read_to_string(&manifest)
                .map_err(|e| format!("reading {}: {e}", manifest.display()))?;
            let json: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("parsing {}: {e}", manifest.display()))?;
            out.push(Package {
                kind: "package",
                name: json["name"].as_str().unwrap_or("?").to_owned(),
                dir: dir.clone(),
                license: json["license"].as_str().map(str::to_owned),
            });
        }
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("reading {}: {e}", dir.display()))?;
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let skip = name.starts_with('.')
                || matches!(
                    name.as_ref(),
                    "node_modules" | "target" | "dist" | "fixtures"
                );
            if !skip && entry.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push(entry.path());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_code_is_gpl() {
        assert_eq!(required_license(""), GPL);
        assert_eq!(required_license("crates/core"), GPL);
        assert_eq!(required_license("ui/app"), GPL);
        assert_eq!(required_license("ui/skin-default"), GPL);
        assert_eq!(required_license("xtask"), GPL);
    }

    #[test]
    fn server_and_sync_are_agpl() {
        assert_eq!(required_license("crates/server"), AGPL);
        assert_eq!(required_license("crates/sync"), AGPL);
        assert_eq!(required_license("crates/sync-relay"), AGPL);
    }

    fn repo_file(rel: &str) -> String {
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel)).unwrap()
    }

    #[test]
    fn committed_license_texts_are_recognised() {
        assert!(is_license_text(&repo_file("LICENSE"), GPL));
        assert!(is_license_text(&repo_file("crates/server/LICENSE"), AGPL));
        assert!(is_license_text(
            &repo_file("crates/sdk-schema/LICENSE"),
            MIT
        ));
        assert!(!is_license_text(&repo_file("LICENSE"), AGPL));
        assert!(!is_license_text(&repo_file("crates/server/LICENSE"), GPL));
    }

    #[test]
    fn altered_or_wrong_version_texts_are_rejected() {
        let gpl = repo_file("LICENSE");
        let v2 = gpl.replace("Version 3, 29 June 2007", "Version 2, June 1991");
        assert!(!is_license_text(&v2, GPL));
        assert!(!is_license_text(
            "GNU GENERAL PUBLIC LICENSE\nVersion 3",
            GPL
        ));
        let mit = repo_file("ui/sdk/LICENSE");
        assert!(!is_license_text(&mit.replace("NONINFRINGEMENT", ""), MIT));
        assert!(!is_license_text(&format!("{mit}\nExtra terms."), MIT));
    }

    #[test]
    fn sdks_are_mit() {
        assert_eq!(required_license("crates/sdk-schema"), MIT);
        assert_eq!(required_license("ui/sdk"), MIT);
        assert_eq!(required_license("sdk/addon-sdk-ts"), MIT);
    }
}
