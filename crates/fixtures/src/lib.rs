//! Secret scrubbing for recorded addon fixtures (T0.1.7, D57).
//!
//! Stremio addons carry credentials in places that are easy to miss: configurable addons
//! (AIOStreams, Comet, Torrentio) encode the user's debrid key into the *path* of the addon
//! URL, debrid resolve links put tokens in stream URLs, and proxies (MediaFlow) wrap whole
//! upstream URLs in a query parameter. A [`Scrubber`] removes them in three layers:
//!
//! 1. **Known secrets** (e.g. the user's TorBox key) are replaced wherever they appear,
//!    including percent-encoded. A string that still contains one after decoding is
//!    replaced whole.
//! 2. **Addon base URLs** (everything before `/manifest.json`) are replaced with
//!    `<scheme>://<host>/REDACTED`, which drops the whole config segment. `https://`,
//!    `http://` and `stremio://` (install links) forms are all covered.
//! 3. **Heuristics** on JSON: values (strings or numbers) under secret-looking keys, URL
//!    user-info, secret-looking query parameters, token-like path segments, keys after
//!    debrid service names, and the same rules inside URL-valued query parameters.
//!
//! A fixture is clean when scrubbing it changes nothing; [`Scrubber::check`] reports what
//! would change.

use serde_json::Value;
use url::Url;

pub mod anonymize;

/// Replacement for every removed secret.
pub const REDACTED: &str = "REDACTED";

/// How deep nested URLs (a proxy's `?d=<url>`) and repeated percent-encoding are followed.
const MAX_DEPTH: usize = 4;

/// Key names (lower-cased, `_`/`-` removed) whose values are always secret.
const SECRET_KEY_PARTS: &[&str] = &[
    "apikey",
    "token",
    "secret",
    "password",
    "passwd",
    "authorization",
    "cookie",
    "session",
    "signature",
    "credential",
];
const SECRET_KEYS_EXACT: &[&str] = &["key", "sig", "pass", "auth", "pw"];
/// Debrid/Usenet services: in addon configs (`torbox=<key>`) the value is the API key.
const SERVICE_KEYS: &[&str] = &[
    "torbox",
    "realdebrid",
    "alldebrid",
    "premiumize",
    "debridlink",
    "offcloud",
    "putio",
    "easydebrid",
    "easynews",
    "pikpak",
    "seedr",
];

/// One change the scrubber made (or would make).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// JSON pointer to the value, e.g. `/streams/0/url`.
    pub pointer: String,
    /// Why it was scrubbed.
    pub reason: &'static str,
}

/// Removes secrets from fixture JSON. See the [crate docs](crate) for the rules.
#[derive(Debug, Clone, Default)]
pub struct Scrubber {
    secrets: Vec<String>,
    addon_bases: Vec<String>,
}

impl Scrubber {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a literal secret (an API key, a password). Values shorter than 4 characters are
    /// ignored, since replacing them would mangle unrelated text.
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        let secret = secret.into();
        if secret.len() >= 4 && !self.secrets.contains(&secret) {
            self.secrets.push(secret);
            // Longest first, so a secret that contains another is replaced whole.
            self.secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
        }
        self
    }

    /// Adds an addon URL (manifest URL or base URL). Its base, which holds the addon's config,
    /// is replaced everywhere with `<scheme>://<host>/REDACTED`, in its `https://`, `http://`
    /// and `stremio://` (install link) forms. Query parameter values on the URL are secrets
    /// too (some addons authenticate with `manifest.json?key=…`).
    pub fn addon_url(mut self, url: &str) -> Self {
        if let Ok(parsed) = Url::parse(&url.replacen("stremio://", "https://", 1)) {
            for (_, value) in parsed.query_pairs() {
                self = self.secret(value.into_owned());
            }
        }
        let base = addon_base(url);
        let Some((_, rest)) = base.split_once("://") else {
            return self;
        };
        for scheme in ["https", "http", "stremio"] {
            let base = format!("{scheme}://{rest}");
            // A base without a path (`https://host`) has no config to remove.
            if redacted_base(&base) != base && !self.addon_bases.contains(&base) {
                self.addon_bases.push(base);
            }
        }
        self.addon_bases.sort_by_key(|s| std::cmp::Reverse(s.len()));
        self
    }

    /// Scrubs `value` in place and returns what was changed.
    pub fn scrub(&self, value: &mut Value) -> Vec<Finding> {
        let mut findings = Vec::new();
        self.scrub_value(value, &mut String::new(), false, &mut findings);
        findings
    }

    /// Reports what [`scrub`](Self::scrub) would change, without changing `value`.
    pub fn check(&self, value: &Value) -> Vec<Finding> {
        self.scrub(&mut value.clone())
    }

    /// Scrubs a single string that is not inside a secret key.
    pub fn scrub_str(&self, text: &str) -> (String, Option<&'static str>) {
        let (out, reason) = self.scrub_text(text, 0);
        // Last line of defence: a known secret or addon config that survives in some encoding
        // we could not rewrite in place takes the whole string with it.
        if self.contains_known(&out) {
            return (REDACTED.to_owned(), Some("known secret (encoded)"));
        }
        (out, reason)
    }

    /// Byte strings that must never survive: each secret, and each addon's decoded config path
    /// (e.g. `/stremio/<uuid>/<blob>`), whatever host, port or scheme it appears with. Very
    /// short paths are anchored to the host to avoid redacting unrelated text.
    fn needles(&self) -> Vec<Vec<u8>> {
        let mut needles: Vec<Vec<u8>> =
            self.secrets.iter().map(|s| s.as_bytes().to_vec()).collect();
        for base in &self.addon_bases {
            if let Some((host, segments)) = base_parts(base) {
                let path = format!("/{}", segments.join("/"));
                needles.push(if path.len() >= 8 {
                    path.into_bytes()
                } else {
                    format!("{host}{path}").into_bytes()
                });
            }
        }
        needles
    }

    /// True if a needle appears in `text` as written or after up to [`MAX_DEPTH`] rounds of
    /// percent-decoding. Works on bytes, so invalid UTF-8 in the decoded form can't hide one.
    fn contains_known(&self, text: &str) -> bool {
        let needles = self.needles();
        if needles.is_empty() {
            return false;
        }
        let mut bytes = text.as_bytes().to_vec();
        for _ in 0..=MAX_DEPTH {
            if needles
                .iter()
                .any(|n| bytes.windows(n.len()).any(|w| w == n.as_slice()))
            {
                return true;
            }
            let next = percent_decode_bytes(&bytes);
            if next == bytes {
                break;
            }
            bytes = next;
        }
        false
    }

    /// Scrubs a query value that may be a (possibly re-encoded) URL. Nested URLs past the
    /// inspection budget are redacted whole rather than passed through unchecked.
    fn scrub_nested(&self, value: &str, depth: usize) -> Option<String> {
        let mut candidate = value.to_owned();
        for level in 0..MAX_DEPTH {
            if Url::parse(&candidate)
                .is_ok_and(|u| matches!(u.scheme(), "http" | "https" | "stremio"))
            {
                if depth + 1 + level >= MAX_DEPTH {
                    return Some(REDACTED.to_owned());
                }
                return match self.scrub_text(&candidate, depth + 1 + level) {
                    (scrubbed, Some(_)) => Some(scrubbed),
                    (_, None) => None,
                };
            }
            let next = percent_decode(&candidate);
            if next == candidate {
                break;
            }
            candidate = next;
        }
        None
    }

    fn scrub_text(&self, text: &str, depth: usize) -> (String, Option<&'static str>) {
        let (mut out, mut reason) = self.replace_known(text);
        if let Some(scrubbed) = self.scrub_url(&out, depth)
            && scrubbed != out
        {
            out = scrubbed;
            reason = reason.or(Some("secret in URL"));
        }
        (out, reason)
    }

    /// Replaces known addon bases and secrets, literally and in their percent-encoded forms.
    fn replace_known(&self, text: &str) -> (String, Option<&'static str>) {
        let mut out = text.to_owned();
        let mut reason = None;
        for base in &self.addon_bases {
            let redacted = redacted_base(base);
            for form in encoded_forms(base) {
                if out.contains(form.as_str()) {
                    out = out.replace(form.as_str(), &redacted);
                    reason = Some("addon config in URL");
                }
            }
        }
        for secret in &self.secrets {
            for form in encoded_forms(secret) {
                if out.contains(form.as_str()) {
                    out = out.replace(form.as_str(), REDACTED);
                    reason = Some("known secret");
                }
            }
        }
        (out, reason)
    }

    /// Scrubs an absolute `http(s)://` or `stremio://` URL; `None` if `text` is not one.
    /// URL-valued query parameters (a proxy's `?d=<url>`) are scrubbed recursively.
    fn scrub_url(&self, text: &str, depth: usize) -> Option<String> {
        if depth >= MAX_DEPTH {
            return None;
        }
        let mut url = Url::parse(text).ok()?;
        if !matches!(url.scheme(), "http" | "https" | "stremio") {
            return None;
        }
        let mut changed = false;

        if !url.username().is_empty() || url.password().is_some() {
            url.set_username(REDACTED).ok()?;
            url.set_password(None).ok()?;
            changed = true;
        }

        if url.query().is_some() {
            let mut any = false;
            let pairs: Vec<(String, String)> = url
                .query_pairs()
                .map(|(k, v)| {
                    let (k, v) = (k.into_owned(), v.into_owned());
                    if v.is_empty() || v == REDACTED {
                        (k, v)
                    } else if is_secret_key(&k) || is_token_like(&v) || is_uuid(&v) {
                        any = true;
                        (k, REDACTED.to_owned())
                    } else if let Some(nested) = self.scrub_nested(&v, depth) {
                        any = true;
                        (k, nested)
                    } else {
                        (k, v)
                    }
                })
                .collect();
            if any {
                url.query_pairs_mut().clear().extend_pairs(pairs);
                changed = true;
            }
        }

        let mut segments: Vec<String> = url
            .path_segments()
            .map(|s| s.map(percent_decode).collect())
            .unwrap_or_default();
        let mut path_changed = false;
        // A registered addon base, however its path is encoded: compare host + decoded segments.
        for base in &self.addon_bases {
            if let Some((host, base_segments)) = base_parts(base)
                && url.host_str() == Some(host.as_str())
                && segments.len() >= base_segments.len()
                && segments[..base_segments.len()] == base_segments[..]
            {
                segments.splice(..base_segments.len(), [REDACTED.to_owned()]);
                path_changed = true;
                break;
            }
        }
        let mut new_segments = Vec::with_capacity(segments.len());
        let mut after_service = false;
        for decoded in &segments {
            let (known, _) = self.replace_known(decoded);
            // Debrid resolve links put the key right after the service name: `/resolve/torbox/<key>/`.
            let scrubbed = if after_service && !decoded.is_empty() {
                REDACTED.to_owned()
            } else {
                scrub_segment(&known)
            };
            after_service = is_service_key(decoded);
            path_changed |= scrubbed != *decoded;
            new_segments.push(scrubbed);
        }
        if path_changed {
            url.path_segments_mut().ok()?.clear().extend(&new_segments);
            changed = true;
        }

        Some(if changed {
            url.to_string()
        } else {
            text.to_owned()
        })
    }

    fn scrub_value(
        &self,
        value: &mut Value,
        pointer: &mut String,
        secret_key: bool,
        findings: &mut Vec<Finding>,
    ) {
        match value {
            Value::String(s) => {
                if secret_key {
                    if !s.is_empty() && s != REDACTED {
                        *s = REDACTED.to_owned();
                        findings.push(Finding {
                            pointer: pointer.clone(),
                            reason: "secret-looking key",
                        });
                    }
                } else if let (scrubbed, Some(reason)) = self.scrub_str(s) {
                    *s = scrubbed;
                    findings.push(Finding {
                        pointer: pointer.clone(),
                        reason,
                    });
                }
            }
            Value::Number(_) if secret_key => {
                *value = Value::String(REDACTED.to_owned());
                findings.push(Finding {
                    pointer: pointer.clone(),
                    reason: "secret-looking key",
                });
            }
            Value::Array(items) => {
                for (i, item) in items.iter_mut().enumerate() {
                    let len = pointer.len();
                    pointer.push('/');
                    pointer.push_str(&i.to_string());
                    self.scrub_value(item, pointer, secret_key, findings);
                    pointer.truncate(len);
                }
            }
            Value::Object(map) => {
                for (key, item) in map.iter_mut() {
                    let len = pointer.len();
                    pointer.push('/');
                    pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                    let secret = secret_key || is_secret_key(key);
                    self.scrub_value(item, pointer, secret, findings);
                    pointer.truncate(len);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
}

/// Scrubs an absolute `http(s)://` or `stremio://` URL with the built-in heuristics only;
/// `None` if `text` is not one.
pub fn scrub_url(text: &str) -> Option<String> {
    Scrubber::new().scrub_url(text, 0)
}

/// True for key names that hold credentials (`apiKey`, `debrid_token`, `Authorization`, …).
pub fn is_secret_key(key: &str) -> bool {
    let norm = normalize_key(key);
    SECRET_KEYS_EXACT.contains(&norm.as_str())
        || SERVICE_KEYS.contains(&norm.as_str())
        || SECRET_KEY_PARTS.iter().any(|part| norm.contains(part))
}

fn is_service_key(key: &str) -> bool {
    SERVICE_KEYS.contains(&normalize_key(key).as_str())
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|c| *c != '_' && *c != '-')
        .collect::<String>()
        .to_ascii_lowercase()
}

/// The addon base for a manifest or resource URL: everything before `/manifest.json` or the
/// resource path (`/catalog/…`, `/meta/…`, `/stream/…`, `/subtitles/…`, `/addon_catalog/…`).
pub fn addon_base(url: &str) -> String {
    let no_query = url.split(['?', '#']).next().unwrap_or(url);
    for marker in [
        "/manifest.json",
        "/catalog/",
        "/meta/",
        "/stream/",
        "/subtitles/",
        "/addon_catalog/",
    ] {
        if let Some(i) = no_query.find(marker) {
            return no_query[..i].to_owned();
        }
    }
    no_query.trim_end_matches('/').to_owned()
}

/// `https://host/secret/config` → `https://host/REDACTED`; a base with no path is unchanged.
fn redacted_base(base: &str) -> String {
    match Url::parse(base) {
        Ok(url) if url.path().trim_matches('/').is_empty() => base.to_owned(),
        Ok(url) => format!(
            "{}://{}/{REDACTED}",
            url.scheme(),
            url.host_str().unwrap_or(REDACTED)
        ),
        Err(_) => REDACTED.to_owned(),
    }
}

/// `s` as written, and percent-encoded the ways URLs and forms encode it.
fn encoded_forms(s: &str) -> Vec<String> {
    let mut forms = vec![s.to_owned()];
    for (upper, plus) in [(true, false), (false, false), (true, true)] {
        let mut out = String::with_capacity(s.len() * 3);
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    out.push(b as char)
                }
                b' ' if plus => out.push('+'),
                _ if upper => out.push_str(&format!("%{b:02X}")),
                _ => out.push_str(&format!("%{b:02x}")),
            }
        }
        if !forms.contains(&out) {
            forms.push(out);
        }
    }
    forms
}

/// A path segment is a token (`REDACTED`), a `key=value|key=value` config with secret values
/// redacted, or left alone.
fn scrub_segment(seg: &str) -> String {
    if is_token_like(seg) || is_uuid(seg) {
        return REDACTED.to_owned();
    }
    if seg.contains('=') {
        return seg
            .split('|')
            .map(|part| match part.split_once('=') {
                Some((k, v))
                    if v != REDACTED
                        && !v.is_empty()
                        && (is_secret_key(k) || is_token_like(v) || is_uuid(v)) =>
                {
                    format!("{k}={REDACTED}")
                }
                _ => part.to_owned(),
            })
            .collect::<Vec<_>>()
            .join("|");
    }
    seg.to_owned()
}

/// Long opaque strings: API keys, base64/base64url blobs, hex tokens. At least 20 characters
/// from the token alphabet, with both letters and digits. Ordinary words, IMDb IDs and
/// dotted file names don't match.
pub fn is_token_like(s: &str) -> bool {
    s.len() >= 20
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '/' | '='))
        && s.chars().any(|c| c.is_ascii_digit())
        && s.chars().any(|c| c.is_ascii_alphabetic())
}

fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Host and decoded path segments of an addon base (`https://host/a/b` → `("host", ["a", "b"])`).
fn base_parts(base: &str) -> Option<(String, Vec<String>)> {
    let url = Url::parse(base).ok()?;
    let host = url.host_str()?.to_owned();
    let segments: Vec<String> = url
        .path_segments()?
        .filter(|s| !s.is_empty())
        .map(percent_decode)
        .collect();
    (!segments.is_empty()).then_some((host, segments))
}

/// One round of percent-decoding. Invalid UTF-8 in the result is replaced (U+FFFD); use
/// [`percent_decode_bytes`] where that could hide something.
fn percent_decode(s: &str) -> String {
    String::from_utf8_lossy(&percent_decode_bytes(s.as_bytes())).into_owned()
}

fn percent_decode_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const KEY: &str = "tb_9f8e7d6c5b4a39281706f5e4d3c2b1a0";

    #[test]
    fn known_secret_is_replaced_everywhere() {
        let s = Scrubber::new().secret(KEY);
        let mut v = json!({ "streams": [{ "title": format!("cached via {KEY}") }] });
        let findings = s.scrub(&mut v);
        assert_eq!(v["streams"][0]["title"], "cached via REDACTED");
        assert_eq!(findings[0].pointer, "/streams/0/title");
    }

    #[test]
    fn addon_url_query_values_are_secrets() {
        let s = Scrubber::new().addon_url("https://host.example/manifest.json?c=shortkey");
        let mut v = json!({ "addon": "https://host.example/manifest.json?c=shortkey", "x": "see shortkey" });
        s.scrub(&mut v);
        assert!(!v.to_string().contains("shortkey"), "{v}");
    }

    #[test]
    fn percent_encoded_known_secret_is_caught() {
        let s = Scrubber::new().secret("pass!word");
        let v = json!({ "url": "https://cdn.example/download/pass%21word/movie.mkv" });
        assert!(!s.check(&v).is_empty());
        let mut v = v;
        s.scrub(&mut v);
        assert_eq!(v["url"], "https://cdn.example/download/REDACTED/movie.mkv");
    }

    #[test]
    fn double_encoded_known_secret_is_caught() {
        let s = Scrubber::new().secret("pass!word");
        let mut v = json!({ "title": "x pass%2521word y" });
        s.scrub(&mut v);
        assert_eq!(v["title"], REDACTED);
    }

    #[test]
    fn addon_config_segment_is_dropped() {
        let manifest = "https://aiostreams.example.com/stremio/0d4c6e2a-1b3f-4a5e-9c7d-8e6f5a4b3c2d/eyJ0b3Jib3giOiJrZXkifQ/manifest.json";
        let s = Scrubber::new().addon_url(manifest);
        let mut v = json!({ "transportUrl": manifest });
        s.scrub(&mut v);
        assert_eq!(
            v["transportUrl"],
            "https://aiostreams.example.com/REDACTED/manifest.json"
        );
    }

    #[test]
    fn addon_base_in_stremio_and_encoded_forms_is_dropped() {
        let s = Scrubber::new().addon_url("https://addon.example/my config/manifest.json");
        let mut v = json!({
            "install": "stremio://addon.example/my config/manifest.json",
            "proxied": "https://proxy.example/p?d=https%3A%2F%2Faddon.example%2Fmy%20config%2Fstream%2Fmovie%2Ftt1.json",
        });
        s.scrub(&mut v);
        assert_eq!(
            v["install"],
            "stremio://addon.example/REDACTED/manifest.json"
        );
        assert!(
            !v["proxied"].as_str().unwrap().contains("config"),
            "{}",
            v["proxied"]
        );
    }

    #[test]
    fn addon_without_config_is_untouched() {
        let manifest = "https://v3-cinemeta.strem.io/manifest.json";
        let s = Scrubber::new().addon_url(manifest);
        let v = json!({ "transportUrl": manifest });
        assert!(s.check(&v).is_empty());
    }

    #[test]
    fn torrentio_style_config_values_are_redacted() {
        let url = format!(
            "https://torrentio.example/providers=yts,eztv|sort=quality|torbox={KEY}/stream/movie/tt0111161.json"
        );
        let out = scrub_url(&url).unwrap();
        assert!(!out.contains(KEY), "{out}");
        assert!(out.contains("providers=yts,eztv"), "{out}");
        assert!(out.contains("sort=quality"), "{out}");
        assert!(out.contains("tt0111161.json"), "{out}");
    }

    #[test]
    fn stremio_install_links_are_scrubbed() {
        let out = scrub_url(
            "stremio://torrentio.strem.fun/realdebrid=ABC123DEFG456HIJK789/manifest.json",
        )
        .unwrap();
        assert!(out.starts_with("stremio://torrentio.strem.fun/"), "{out}");
        assert!(out.ends_with("/manifest.json"), "{out}");
        assert!(!out.contains("ABC123DEFG456HIJK789"), "{out}");
    }

    #[test]
    fn short_service_key_in_config_is_redacted() {
        let out = scrub_url("https://torrentio.example/realdebrid=abc123/manifest.json").unwrap();
        assert!(out.contains("realdebrid=REDACTED"), "{out}");
    }

    #[test]
    fn debrid_resolve_link_tokens_are_redacted() {
        let url = format!(
            "https://torrentio.example/resolve/torbox/{KEY}/2b1f8e0c9d7a6b5c4d3e2f1a0b9c8d7e6f5a4b3c/null/0/The.Movie.2023.2160p.mkv"
        );
        let out = scrub_url(&url).unwrap();
        assert!(!out.contains(KEY), "{out}");
        assert!(out.contains("The.Movie.2023.2160p.mkv"), "{out}");
    }

    #[test]
    fn short_key_after_service_segment_is_redacted() {
        let out =
            scrub_url("https://torrentio.example/resolve/realdebrid/abc123/hash/null/0/x.mkv")
                .unwrap();
        assert_eq!(
            out,
            "https://torrentio.example/resolve/realdebrid/REDACTED/hash/null/0/x.mkv"
        );
    }

    #[test]
    fn query_and_userinfo_are_redacted() {
        let out = scrub_url("https://user:hunter22@example.com/a?apikey=abc&page=2").unwrap();
        assert!(!out.contains("hunter22"), "{out}");
        assert!(!out.contains("apikey=abc"), "{out}");
        assert!(out.contains("page=2"), "{out}");
    }

    #[test]
    fn nested_proxy_urls_are_scrubbed() {
        let v = json!({
            "url": "https://proxy.example/proxy/stream?d=https%3A%2F%2Fcdn.example%2Ffile.mkv%3Ftoken%3Dabc123&api_password=REDACTED"
        });
        let s = Scrubber::new();
        assert!(!s.check(&v).is_empty());
        let mut v = v;
        s.scrub(&mut v);
        let out = v["url"].as_str().unwrap();
        assert!(!out.contains("abc123"), "{out}");
        assert!(s.check(&v).is_empty(), "not idempotent: {out}");
    }

    #[test]
    fn addon_base_with_differently_encoded_path_is_dropped() {
        let s = Scrubber::new().addon_url("https://addon.example/abc!def/manifest.json");
        let mut v = json!({ "url": "https://addon.example/abc%21def/manifest.json" });
        assert!(!s.check(&v).is_empty());
        s.scrub(&mut v);
        assert_eq!(v["url"], "https://addon.example/REDACTED/manifest.json");
        assert!(s.check(&v).is_empty());
    }

    #[test]
    fn addon_config_in_prose_with_port_and_other_encoding_is_caught() {
        let s = Scrubber::new()
            .addon_url("http://localhost:3000/eyJ0b3Jib3giOiJteWtleTEyMyJ9==/manifest.json");
        let mut v = json!({
            "error": "Addon request failed: http://localhost:3000/eyJ0b3Jib3giOiJteWtleTEyMyJ9%3D%3D/manifest.json (timeout)"
        });
        assert!(!s.check(&v).is_empty());
        s.scrub(&mut v);
        assert!(!v.to_string().contains("eyJ0b3Jib3gi"), "{v}");
        assert!(s.check(&v).is_empty());
    }

    #[test]
    fn invalid_utf8_does_not_hide_known_secret() {
        let s = Scrubber::new().secret("pass!word");
        let v = json!({ "url": "https://cdn.example/%FFpa%73s%21word/file.mkv" });
        let mut v2 = v.clone();
        assert!(!s.check(&v).is_empty());
        s.scrub(&mut v2);
        assert!(s.check(&v2).is_empty(), "{v2}");
        assert!(!v2.to_string().contains("word"), "{v2}");
    }

    #[test]
    fn re_encoded_proxy_urls_are_scrubbed() {
        let s = Scrubber::new();
        let mut v = json!({
            "url": "https://proxy.example/p?d=https%253A%252F%252Fcdn.example%252Ffile.mkv%253Ftoken%253Dabc123"
        });
        assert!(!s.check(&v).is_empty());
        s.scrub(&mut v);
        assert!(!v.to_string().contains("abc123"), "{v}");
        assert!(s.check(&v).is_empty(), "not idempotent: {v}");
    }

    #[test]
    fn deeply_nested_proxy_urls_fail_closed() {
        let mut inner = "https://cdn.example/file.mkv?token=abc123".to_owned();
        for _ in 0..5 {
            inner = format!(
                "https://proxy.example/p?d={}",
                url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>()
            );
        }
        let s = Scrubber::new();
        let mut v = json!({ "url": inner });
        s.scrub(&mut v);
        assert!(!v.to_string().contains("abc123"), "{v}");
        assert!(s.check(&v).is_empty(), "not idempotent: {v}");
    }

    #[test]
    fn secret_keys_in_json_are_redacted() {
        let mut v = json!({
            "behaviorHints": { "proxyHeaders": { "request": { "Authorization": "Bearer xyz" } } },
            "config": { "debrid_api_key": "abc", "TorBox": "def", "name": "Comet" },
        });
        Scrubber::new().scrub(&mut v);
        assert_eq!(
            v["behaviorHints"]["proxyHeaders"]["request"]["Authorization"],
            REDACTED
        );
        assert_eq!(v["config"]["debrid_api_key"], REDACTED);
        assert_eq!(v["config"]["TorBox"], REDACTED);
        assert_eq!(v["config"]["name"], "Comet");
    }

    #[test]
    fn numeric_values_under_secret_keys_are_redacted() {
        let mut v = json!({ "token": 123456789012345678_u64, "password": 123456, "page": 2 });
        let findings = Scrubber::new().scrub(&mut v);
        assert_eq!(findings.len(), 2);
        assert_eq!(v["token"], REDACTED);
        assert_eq!(v["password"], REDACTED);
        assert_eq!(v["page"], 2);
    }

    #[test]
    fn ordinary_stremio_data_is_untouched() {
        let v = json!({
            "metas": [{
                "id": "tt0111161",
                "type": "movie",
                "name": "The Shawshank Redemption",
                "poster": "https://images.metahub.space/poster/medium/tt0111161/img",
            }],
            "streams": [{
                "infoHash": "2b1f8e0c9d7a6b5c4d3e2f1a0b9c8d7e6f5a4b3c",
                "fileIdx": 0,
                "title": "The.Shawshank.Redemption.1994.2160p.UHD.BluRay.x265-GROUP\n👤 42 💾 18.3 GB",
                "externalUrl": "https://www.imdb.com/title/tt0111161/?ref_=nv_sr_1&page=2",
            }],
            "cacheMaxAge": 3600,
        });
        assert_eq!(Scrubber::new().check(&v), vec![]);
    }

    #[test]
    fn scrubbing_is_idempotent() {
        let s = Scrubber::new().secret(KEY);
        let mut v = json!({ "url": format!("https://cdn.example/dl/{KEY}/file.mkv?token=abc") });
        assert!(!s.scrub(&mut v).is_empty());
        assert!(s.check(&v).is_empty(), "{v}");
    }

    #[test]
    fn token_heuristic() {
        assert!(is_token_like(KEY));
        assert!(is_token_like(
            "eyJ0b3Jib3giOiJrZXkiLCJzb3J0IjoicXVhbGl0eSJ9"
        ));
        assert!(!is_token_like("tt0111161"));
        assert!(!is_token_like("The.Movie.2023.2160p.mkv"));
        assert!(!is_token_like("supercalifragilisticexpialidocious"));
        assert!(is_uuid("0d4c6e2a-1b3f-4a5e-9c7d-8e6f5a4b3c2d"));
    }
}
