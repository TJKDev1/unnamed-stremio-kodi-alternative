//! Resource routes: `{base}/{resource}/{type}/{id}[/{extra}].json` (spec: Routes).

use crate::MANIFEST_PATH;

/// One resource request, independent of which addon it goes to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourcePath {
    pub resource: String,
    pub r#type: String,
    pub id: String,
    /// `name=value` pairs, in order; a name may repeat (multi-select genres).
    pub extra: Vec<(String, String)>,
}

impl ResourcePath {
    pub fn new(resource: &str, r#type: &str, id: &str) -> Self {
        Self {
            resource: resource.to_owned(),
            r#type: r#type.to_owned(),
            id: id.to_owned(),
            extra: Vec::new(),
        }
    }

    pub fn with_extra(mut self, name: &str, value: &str) -> Self {
        self.extra.push((name.to_owned(), value.to_owned()));
        self
    }

    /// The extra names, for checking against a catalog's supported extras.
    pub fn extra_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for (name, _) in &self.extra {
            if !names.contains(&name.as_str()) {
                names.push(name);
            }
        }
        names
    }

    /// The URL path, starting with `/` and ending in `.json`.
    pub fn to_url_path(&self) -> String {
        let mut path = format!(
            "/{}/{}/{}",
            encode_component(&self.resource),
            encode_component(&self.r#type),
            encode_component(&self.id)
        );
        if !self.extra.is_empty() {
            path.push('/');
            path.push_str(&encode_extra(&self.extra));
        }
        path.push_str(".json");
        path
    }

    /// The full URL for this path on the addon at `transport_url` (which ends in
    /// `/manifest.json`, possibly followed by a query string). The query string is carried over,
    /// as stremio-core does.
    pub fn url(&self, transport_url: &str) -> String {
        let (path, query) = match transport_url.find(['?', '#']) {
            Some(i) => transport_url.split_at(i),
            None => (transport_url, ""),
        };
        let base = path.strip_suffix(MANIFEST_PATH).unwrap_or(path);
        format!("{base}{}{query}", self.to_url_path())
    }

    /// Where a recording of this path lives inside an addon's fixture folder: the same
    /// segments, each made safe for every filesystem (spec: Fixtures).
    pub fn fixture_path(&self) -> String {
        let mut parts = vec![
            encode_fs(&self.resource),
            encode_fs(&self.r#type),
            encode_fs(&self.id),
        ];
        if !self.extra.is_empty() {
            parts.push(encode_fs(&encode_extra(&self.extra)));
        }
        parts.join("/") + ".json"
    }
}

/// `name=value&…` with both sides encoded like `encodeURIComponent`.
pub fn encode_extra(extra: &[(String, String)]) -> String {
    extra
        .iter()
        .map(|(name, value)| format!("{}={}", encode_component(name), encode_component(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Percent-encodes like JS `encodeURIComponent`: everything except `A-Z a-z 0-9 - _ . ! ~ * ' ( )`.
/// This is the set stremio-core uses for every route segment.
pub fn encode_component(s: &str) -> String {
    encode(s, |b| {
        b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b)
    })
}

/// Percent-encodes everything except `A-Z a-z 0-9 - _ .`, which is safe as a file name on
/// Windows, macOS and Linux (`:` and `*` are not on Windows).
pub fn encode_fs(s: &str) -> String {
    let encoded = encode(s, |b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
    // `.` and `..` are not file names.
    match encoded.as_str() {
        "." => "%2E".into(),
        ".." => "%2E%2E".into(),
        _ => encoded,
    }
}

fn encode(s: &str, keep: impl Fn(u8) -> bool) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if keep(b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_routes() {
        let p = ResourcePath::new("stream", "movie", "tt1375666");
        assert_eq!(p.to_url_path(), "/stream/movie/tt1375666.json");
        assert_eq!(
            p.url("https://a.b/cfg/manifest.json"),
            "https://a.b/cfg/stream/movie/tt1375666.json"
        );
    }

    #[test]
    fn ids_are_encoded_like_encode_uri_component() {
        let p = ResourcePath::new("stream", "series", "tt0944947:1:1");
        assert_eq!(p.to_url_path(), "/stream/series/tt0944947%3A1%3A1.json");
        let p = ResourcePath::new("meta", "anime", "kitsu:1/ä b");
        assert_eq!(p.to_url_path(), "/meta/anime/kitsu%3A1%2F%C3%A4%20b.json");
        assert_eq!(encode_component("-_.!~*'()"), "-_.!~*'()");
        assert_eq!(encode_component("&=+?#"), "%26%3D%2B%3F%23");
    }

    #[test]
    fn extras_keep_order_and_repeats() {
        let p = ResourcePath::new("catalog", "movie", "top")
            .with_extra("genre", "Sci-Fi & Fantasy")
            .with_extra("genre", "Drama")
            .with_extra("skip", "100");
        assert_eq!(
            p.to_url_path(),
            "/catalog/movie/top/genre=Sci-Fi%20%26%20Fantasy&genre=Drama&skip=100.json"
        );
        assert_eq!(p.extra_names(), ["genre", "skip"]);
    }

    #[test]
    fn only_the_trailing_manifest_path_is_replaced_and_query_is_kept() {
        let p = ResourcePath::new("stream", "movie", "tt1");
        assert_eq!(
            p.url("https://a.b/manifest.json/x/manifest.json?k=v"),
            "https://a.b/manifest.json/x/stream/movie/tt1.json?k=v"
        );
    }

    #[test]
    fn fixture_paths_are_filesystem_safe() {
        let p = ResourcePath::new("stream", "series", "tt0944947:1:1");
        assert_eq!(p.fixture_path(), "stream/series/tt0944947%3A1%3A1.json");
        let p = ResourcePath::new("catalog", "movie", "top").with_extra("search", "a*b");
        assert_eq!(p.fixture_path(), "catalog/movie/top/search%3Da%2Ab.json");
        assert_eq!(encode_fs(".."), "%2E%2E");
    }
}
