//! Secret scrubbing for recorded addon fixtures (T0.1.7, D57).
//!
//! Stremio addons carry credentials in places that are easy to miss: configurable addons
//! (AIOStreams, Comet, Torrentio) encode the user's debrid key into the *path* of the addon
//! URL, and debrid resolve links put tokens in stream URLs. A [`Scrubber`] removes them in
//! three layers:
//!
//! 1. **Known secrets** (e.g. the user's TorBox key) are replaced wherever they appear.
//! 2. **Addon base URLs** (everything before `/manifest.json`) are replaced with
//!    `<scheme>://<host>/REDACTED`, which drops the whole config segment.
//! 3. **Heuristics** on JSON: values under secret-looking keys, URL user-info, secret-looking
//!    query parameters, and token-like path segments.
//!
//! A fixture is clean when scrubbing it changes nothing; [`Scrubber::check`] reports what
//! would change.

use serde_json::Value;
use url::Url;

/// Replacement for every removed secret.
pub const REDACTED: &str = "REDACTED";

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
    /// is replaced everywhere with `<scheme>://<host>/REDACTED`.
    pub fn addon_url(mut self, url: &str) -> Self {
        let base = addon_base(url);
        // A base without a path (`https://host`) has no config to remove.
        if redacted_base(&base) != base && !self.addon_bases.contains(&base) {
            self.addon_bases.push(base);
            self.addon_bases.sort_by_key(|s| std::cmp::Reverse(s.len()));
        }
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
        let mut out = text.to_owned();
        let mut reason = None;
        for base in &self.addon_bases {
            if out.contains(base.as_str()) {
                out = out.replace(base.as_str(), &redacted_base(base));
                reason = Some("addon config in URL");
            }
        }
        for secret in &self.secrets {
            if out.contains(secret.as_str()) {
                out = out.replace(secret.as_str(), REDACTED);
                reason = Some("known secret");
            }
        }
        if let Some(scrubbed) = scrub_url(&out)
            && scrubbed != out
        {
            out = scrubbed;
            reason = reason.or(Some("secret in URL"));
        }
        (out, reason)
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

/// Scrubs an absolute http(s) URL; returns `None` if `text` is not one.
pub fn scrub_url(text: &str) -> Option<String> {
    let mut url = Url::parse(text).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let mut changed = false;

    if !url.username().is_empty() || url.password().is_some() {
        url.set_username(REDACTED).ok()?;
        url.set_password(None).ok()?;
        changed = true;
    }

    if url.query().is_some() {
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let mut any = false;
        let pairs: Vec<(String, String)> = pairs
            .into_iter()
            .map(|(k, v)| {
                if (is_secret_key(&k) || is_token_like(&v)) && v != REDACTED && !v.is_empty() {
                    any = true;
                    (k, REDACTED.to_owned())
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

    let segments: Vec<String> = url
        .path_segments()
        .map(|s| s.map(str::to_owned).collect())
        .unwrap_or_default();
    let mut new_segments = Vec::with_capacity(segments.len());
    let mut path_changed = false;
    let mut after_service = false;
    for seg in &segments {
        let decoded = percent_decode(seg);
        // Debrid resolve links put the key right after the service name: `/resolve/torbox/<key>/`.
        let scrubbed = if after_service && !decoded.is_empty() {
            REDACTED.to_owned()
        } else {
            scrub_segment(&decoded)
        };
        after_service = is_service_key(&decoded);
        if scrubbed != decoded {
            path_changed = true;
            new_segments.push(scrubbed);
        } else {
            new_segments.push(decoded);
        }
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

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
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
    String::from_utf8(out).unwrap_or_else(|_| s.to_owned())
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
