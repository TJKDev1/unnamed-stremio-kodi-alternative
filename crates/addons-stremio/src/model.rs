//! Normalized protocol models (spec: Models). Every object keeps fields we don't model in
//! `other`, so nothing an addon sends is lost when we store or re-serve it.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::de;

/// Fields we don't model, preserved as sent.
pub type Other = Map<String, Value>;

// ---------------------------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(deserialize_with = "de::vec")]
    pub resources: Vec<ManifestResource>,
    #[serde(default, deserialize_with = "de::strings")]
    pub types: Vec<String>,
    /// `None` means any id; `Some([])` means none.
    #[serde(
        default,
        deserialize_with = "de::opt_strings",
        skip_serializing_if = "Option::is_none"
    )]
    pub id_prefixes: Option<Vec<String>>,
    #[serde(default, deserialize_with = "de::vec")]
    pub catalogs: Vec<ManifestCatalog>,
    #[serde(
        default,
        deserialize_with = "de::vec",
        skip_serializing_if = "Vec::is_empty"
    )]
    /// Same shape as `catalogs` (stremio-core uses one type for both).
    pub addon_catalogs: Vec<ManifestCatalog>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub logo: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub background: Option<String>,
    #[serde(default)]
    pub behavior_hints: ManifestBehaviorHints,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestBehaviorHints {
    #[serde(default, deserialize_with = "de::bool")]
    pub adult: bool,
    #[serde(default, deserialize_with = "de::bool")]
    pub p2p: bool,
    #[serde(default, deserialize_with = "de::bool")]
    pub configurable: bool,
    #[serde(default, deserialize_with = "de::bool")]
    pub configuration_required: bool,
    #[serde(flatten)]
    pub other: Other,
}

/// A manifest resource: either just its name (`"stream"`, using the manifest's `types` and
/// `idPrefixes`) or an object with its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ManifestResource {
    Short(String),
    Full(FullResource),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullResource {
    pub name: String,
    #[serde(
        default,
        deserialize_with = "de::opt_strings",
        skip_serializing_if = "Option::is_none"
    )]
    pub types: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "de::opt_strings",
        skip_serializing_if = "Option::is_none"
    )]
    pub id_prefixes: Option<Vec<String>>,
    #[serde(flatten)]
    pub other: Other,
}

impl ManifestResource {
    pub fn name(&self) -> &str {
        match self {
            Self::Short(name) => name,
            Self::Full(r) => &r.name,
        }
    }
    /// The resource's own types, if it overrides the manifest's.
    pub fn types(&self) -> Option<&[String]> {
        match self {
            Self::Short(_) => None,
            Self::Full(r) => r.types.as_deref(),
        }
    }
    /// The resource's own id prefixes, if it overrides the manifest's.
    pub fn id_prefixes(&self) -> Option<&[String]> {
        match self {
            Self::Short(_) => None,
            Self::Full(r) => r.id_prefixes.as_deref(),
        }
    }
}

/// A catalog. Legacy `extraSupported`/`extraRequired` are normalized into `extra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawCatalog")]
pub struct ManifestCatalog {
    #[serde(rename = "type")]
    pub r#type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub extra: Vec<ExtraProp>,
    /// True when the manifest used the legacy extra fields (kept for validation warnings).
    #[serde(skip)]
    pub legacy_extra: bool,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCatalog {
    #[serde(rename = "type")]
    r#type: String,
    id: String,
    #[serde(default, deserialize_with = "de::opt_string")]
    name: Option<String>,
    #[serde(default, deserialize_with = "opt_extra")]
    extra: Option<Vec<ExtraProp>>,
    #[serde(default, deserialize_with = "de::opt_strings")]
    extra_supported: Option<Vec<String>>,
    #[serde(default, deserialize_with = "de::opt_strings")]
    extra_required: Option<Vec<String>>,
    #[serde(flatten)]
    other: Other,
}

fn opt_extra<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Vec<ExtraProp>>, D::Error> {
    let value = Value::deserialize(d)?;
    if value.is_null() {
        return Ok(None);
    }
    de::vec(value).map(Some).map_err(serde::de::Error::custom)
}

impl From<RawCatalog> for ManifestCatalog {
    fn from(raw: RawCatalog) -> Self {
        let legacy =
            raw.extra.is_none() && (raw.extra_supported.is_some() || raw.extra_required.is_some());
        let extra = match raw.extra {
            Some(extra) => extra,
            None => {
                let required = raw.extra_required.unwrap_or_default();
                let mut names = raw.extra_supported.unwrap_or_default();
                // A required extra is supported even if the manifest forgot to say so.
                for name in &required {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                }
                names
                    .into_iter()
                    .map(|name| ExtraProp {
                        is_required: required.contains(&name),
                        name,
                        options: None,
                        options_limit: None,
                        other: Other::new(),
                    })
                    .collect()
            }
        };
        Self {
            r#type: raw.r#type,
            id: raw.id,
            name: raw.name,
            extra,
            legacy_extra: legacy,
            other: raw.other,
        }
    }
}

impl ManifestCatalog {
    /// Whether this catalog can be requested with exactly these extra names: every required
    /// extra is given and every given extra is supported.
    pub fn is_extra_supported(&self, given: &[&str]) -> bool {
        let all_required_given = self
            .extra
            .iter()
            .filter(|e| e.is_required)
            .all(|e| given.contains(&e.name.as_str()));
        let all_given_supported = given
            .iter()
            .all(|g| self.extra.iter().any(|e| e.name == *g));
        all_required_given && all_given_supported
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraProp {
    pub name: String,
    #[serde(default, deserialize_with = "de::bool")]
    pub is_required: bool,
    #[serde(
        default,
        deserialize_with = "de::opt_strings",
        skip_serializing_if = "Option::is_none"
    )]
    pub options: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub options_limit: Option<u64>,
    #[serde(flatten)]
    pub other: Other,
}

/// An installed (or listed) addon: where it lives and what it says it does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub transport_url: String,
    pub manifest: Manifest,
    #[serde(flatten)]
    pub other: Other,
}

// ---------------------------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------------------------

/// A catalog item. Also the common part of [`Meta`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaPreview {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub poster: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub poster_shape: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub background: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub logo: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub release_info: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub imdb_rating: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub runtime: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub released: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::vec",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub links: Vec<MetaLink>,
    #[serde(default)]
    pub behavior_hints: MetaBehaviorHints,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaLink {
    #[serde(deserialize_with = "string")]
    pub name: String,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub category: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub url: Option<String>,
    #[serde(flatten)]
    pub other: Other,
}

fn nullable<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Meta>, D::Error> {
    Option::<Meta>::deserialize(d)
}

fn string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    de::opt_string(d)?.ok_or_else(|| serde::de::Error::custom("expected a string"))
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaBehaviorHints {
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_video_id: Option<String>,
    #[serde(default, deserialize_with = "de::bool")]
    pub has_scheduled_videos: bool,
    #[serde(flatten)]
    pub other: Other,
}

/// A full meta item: the preview fields plus `videos`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(flatten)]
    pub preview: MetaPreview,
    #[serde(
        default,
        deserialize_with = "de::vec",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub videos: Vec<Video>,
}

impl Meta {
    /// The id to request streams with for this meta when there is no episode to pick: the
    /// `defaultVideoId` hint, or the meta id for a movie without videos (spec: Behaviour).
    pub fn default_video_id(&self) -> Option<&str> {
        if let Some(id) = &self.preview.behavior_hints.default_video_id {
            return Some(id);
        }
        self.videos.is_empty().then_some(self.preview.id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    #[serde(deserialize_with = "string")]
    pub id: String,
    /// `title` in the protocol; see [`Video::display_title`] for the `name` fallback.
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub title: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub released: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub season: Option<u64>,
    /// `episode` in the protocol; see [`Video::episode_number`] for the `number` fallback.
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub episode: Option<u64>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub thumbnail: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub overview: Option<String>,
    /// Embedded streams: when present, other stream addons are not asked for this video.
    #[serde(
        default,
        deserialize_with = "de::vec",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub streams: Vec<Stream>,
    #[serde(flatten)]
    pub other: Other,
}

impl Video {
    /// `title`, or the legacy `name` (Cinemeta and TMDB send both; serde aliases would reject
    /// that as a duplicate field, so the fallback is read from `other`).
    pub fn display_title(&self) -> Option<&str> {
        self.title
            .as_deref()
            .or_else(|| self.other.get("name").and_then(Value::as_str))
    }

    /// `episode`, or the legacy `number`.
    pub fn episode_number(&self) -> Option<u64> {
        self.episode.or_else(|| {
            let n = self.other.get("number")?;
            n.as_u64().or_else(|| n.as_str()?.trim().parse().ok())
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Streams and subtitles
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub title: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub url: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub yt_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub info_hash: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub file_idx: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_must_include: Option<Value>,
    #[serde(
        default,
        deserialize_with = "de::opt_strings",
        skip_serializing_if = "Option::is_none"
    )]
    pub sources: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub external_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub nzb_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::vec",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub subtitles: Vec<Subtitle>,
    #[serde(default)]
    pub behavior_hints: StreamBehaviorHints,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamBehaviorHints {
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub binge_group: Option<String>,
    #[serde(default, deserialize_with = "de::bool")]
    pub not_web_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_headers: Option<Value>,
    #[serde(
        default,
        deserialize_with = "de::opt_strings",
        skip_serializing_if = "Option::is_none"
    )]
    pub country_whitelist: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub video_hash: Option<String>,
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub video_size: Option<u64>,
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub filename: Option<String>,
    #[serde(flatten)]
    pub other: Other,
}

/// Archive kinds a stream can point at (`rarUrls`, `zipUrls`, …).
pub const ARCHIVE_KINDS: &[(&str, &str)] = &[
    ("rarUrls", "rar"),
    ("zipUrls", "zip"),
    ("7zipUrls", "7zip"),
    ("tgzUrls", "tgz"),
    ("tarUrls", "tar"),
];

/// What kind of thing a stream points at, which decides who can play it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamSource<'a> {
    Url(&'a str),
    YouTube(&'a str),
    Torrent {
        info_hash: &'a str,
        file_idx: Option<u64>,
        sources: &'a [String],
    },
    External(&'a str),
    Nzb(&'a str),
    Archive(&'static str),
    /// Nothing we recognise: shown as "requires plugin", never hidden.
    Unknown,
}

impl Stream {
    /// Classifies the stream. Checked in the order players prefer: a direct URL wins over
    /// a torrent that also lists one.
    pub fn source(&self) -> StreamSource<'_> {
        if let Some(url) = &self.url {
            return StreamSource::Url(url);
        }
        if let Some(id) = &self.yt_id {
            return StreamSource::YouTube(id);
        }
        if let Some(info_hash) = &self.info_hash {
            return StreamSource::Torrent {
                info_hash,
                file_idx: self.file_idx,
                sources: self.sources.as_deref().unwrap_or_default(),
            };
        }
        if let Some(url) = &self.nzb_url {
            return StreamSource::Nzb(url);
        }
        if let Some((_, kind)) = ARCHIVE_KINDS
            .iter()
            .find(|(key, _)| self.other.contains_key(*key))
        {
            return StreamSource::Archive(kind);
        }
        if let Some(url) = &self.external_url {
            return StreamSource::External(url);
        }
        StreamSource::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subtitle {
    #[serde(
        default,
        deserialize_with = "de::opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<String>,
    #[serde(deserialize_with = "string")]
    pub url: String,
    #[serde(deserialize_with = "string")]
    pub lang: String,
    #[serde(flatten)]
    pub other: Other,
}

// ---------------------------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------------------------

/// Caching hints an addon puts in a response body (seconds).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheHints {
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_max_age: Option<u64>,
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub stale_revalidate: Option<u64>,
    #[serde(
        default,
        deserialize_with = "de::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub stale_error: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogResponse {
    #[serde(deserialize_with = "de::vec")]
    pub metas: Vec<MetaPreview>,
    #[serde(flatten)]
    pub cache: CacheHints,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaResponse {
    /// `null` when the addon doesn't know the id. The key itself is required.
    #[serde(deserialize_with = "nullable")]
    pub meta: Option<Meta>,
    #[serde(flatten)]
    pub cache: CacheHints,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamsResponse {
    #[serde(deserialize_with = "de::vec")]
    pub streams: Vec<Stream>,
    #[serde(flatten)]
    pub cache: CacheHints,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitlesResponse {
    #[serde(deserialize_with = "de::vec")]
    pub subtitles: Vec<Subtitle>,
    #[serde(flatten)]
    pub cache: CacheHints,
    #[serde(flatten)]
    pub other: Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddonCatalogResponse {
    #[serde(deserialize_with = "de::vec")]
    pub addons: Vec<Descriptor>,
    #[serde(flatten)]
    pub cache: CacheHints,
    #[serde(flatten)]
    pub other: Other,
}

/// A parsed resource response, whichever resource it was.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceResponse {
    Catalog(CatalogResponse),
    Meta(Box<MetaResponse>),
    Streams(StreamsResponse),
    Subtitles(SubtitlesResponse),
    AddonCatalog(AddonCatalogResponse),
}

impl ResourceResponse {
    pub fn cache(&self) -> CacheHints {
        match self {
            Self::Catalog(r) => r.cache,
            Self::Meta(r) => r.cache,
            Self::Streams(r) => r.cache,
            Self::Subtitles(r) => r.cache,
            Self::AddonCatalog(r) => r.cache,
        }
    }
}

/// A parsed value plus the list items that were dropped on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed<T> {
    pub value: T,
    pub dropped: Vec<de::Dropped>,
}

/// Why a response couldn't be used at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parses `bytes` as `T`, collecting dropped list items.
pub fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<Parsed<T>, ParseError> {
    let (result, dropped) = de::collect_dropped(|| serde_json::from_slice::<T>(bytes));
    result
        .map(|value| Parsed { value, dropped })
        .map_err(|e| ParseError(e.to_string()))
}

/// Parses a response body for `resource`.
pub fn parse_resource(
    resource: &str,
    bytes: &[u8],
) -> Result<Parsed<ResourceResponse>, ParseError> {
    fn map<T: serde::de::DeserializeOwned>(
        bytes: &[u8],
        f: fn(T) -> ResourceResponse,
    ) -> Result<Parsed<ResourceResponse>, ParseError> {
        let Parsed { value, dropped } = parse::<T>(bytes)?;
        Ok(Parsed {
            value: f(value),
            dropped,
        })
    }
    match resource {
        "catalog" => map(bytes, ResourceResponse::Catalog),
        "meta" => map(bytes, |r: MetaResponse| ResourceResponse::Meta(Box::new(r))),
        "stream" => map(bytes, ResourceResponse::Streams),
        "subtitles" => map(bytes, ResourceResponse::Subtitles),
        "addon_catalog" => map(bytes, ResourceResponse::AddonCatalog),
        other => Err(ParseError(format!("unknown resource `{other}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_value<T: serde::de::DeserializeOwned>(v: Value) -> Parsed<T> {
        parse(&serde_json::to_vec(&v).unwrap()).unwrap()
    }

    #[test]
    fn unknown_fields_survive_a_round_trip() {
        let input = json!({
            "streams": [{ "url": "https://x/1.mkv", "name": "A", "rokuUrl": null, "x": { "y": 1 },
                          "behaviorHints": { "bingeGroup": "g", "custom": true } }],
            "cacheMaxAge": 60, "foo": "bar"
        });
        let parsed: Parsed<StreamsResponse> = parse_value(input.clone());
        assert!(parsed.dropped.is_empty());
        assert_eq!(parsed.value.cache.cache_max_age, Some(60));
        let back = serde_json::to_value(&parsed.value).unwrap();
        assert_eq!(back["foo"], "bar");
        assert_eq!(back["streams"][0]["x"], json!({ "y": 1 }));
        assert_eq!(back["streams"][0]["behaviorHints"]["custom"], true);
        assert_eq!(back["streams"][0]["rokuUrl"], Value::Null);
    }

    #[test]
    fn bad_items_are_dropped_and_reported() {
        let parsed: Parsed<CatalogResponse> = parse_value(json!({
            "metas": [{ "id": "tt1", "type": "movie" }, { "type": "movie" }, 7]
        }));
        assert_eq!(parsed.value.metas.len(), 1);
        assert_eq!(parsed.dropped.len(), 2);
        assert_eq!(parsed.dropped[0].item, "MetaPreview[1]");
    }

    #[test]
    fn nested_drops_are_reported_too() {
        let parsed: Parsed<MetaResponse> = parse_value(json!({
            "meta": { "id": "tt1", "type": "series", "videos": [{ "id": "tt1:1:1" }, { "title": "no id" }] }
        }));
        assert_eq!(parsed.value.meta.unwrap().videos.len(), 1);
        assert_eq!(parsed.dropped.len(), 1);
    }

    #[test]
    fn missing_top_level_key_is_an_error() {
        assert!(parse::<StreamsResponse>(b"{}").is_err());
        assert!(parse::<StreamsResponse>(b"not json").is_err());
    }

    #[test]
    fn scalars_are_lenient() {
        let parsed: Parsed<StreamsResponse> = parse_value(json!({ "streams": [{
            "infoHash": "abc", "fileIdx": "3", "behaviorHints": { "videoSize": "123", "notWebReady": "true" }
        }]}));
        let s = &parsed.value.streams[0];
        assert_eq!(s.file_idx, Some(3));
        assert_eq!(s.behavior_hints.video_size, Some(123));
        assert!(s.behavior_hints.not_web_ready);
        let meta: Parsed<CatalogResponse> = parse_value(json!({ "metas": [
            { "id": "tt1", "type": "movie", "imdbRating": 7.5, "releaseInfo": 2010 }
        ]}));
        assert_eq!(meta.value.metas[0].imdb_rating.as_deref(), Some("7.5"));
        assert_eq!(meta.value.metas[0].release_info.as_deref(), Some("2010"));
    }

    #[test]
    fn stream_sources() {
        let s = |v: Value| serde_json::from_value::<Stream>(v).unwrap();
        assert_eq!(
            s(json!({ "url": "u", "infoHash": "h" })).source(),
            StreamSource::Url("u")
        );
        assert_eq!(
            s(json!({ "ytId": "y" })).source(),
            StreamSource::YouTube("y")
        );
        assert!(matches!(
            s(json!({ "infoHash": "h", "fileIdx": 1 })).source(),
            StreamSource::Torrent {
                info_hash: "h",
                file_idx: Some(1),
                ..
            }
        ));
        assert_eq!(
            s(json!({ "externalUrl": "e" })).source(),
            StreamSource::External("e")
        );
        assert_eq!(s(json!({ "nzbUrl": "n" })).source(), StreamSource::Nzb("n"));
        assert_eq!(
            s(json!({ "rarUrls": [] })).source(),
            StreamSource::Archive("rar")
        );
        assert_eq!(s(json!({ "name": "?" })).source(), StreamSource::Unknown);
    }

    #[test]
    fn videos_with_both_new_and_legacy_fields() {
        let parsed: Parsed<MetaResponse> = parse_value(json!({ "meta": {
            "id": "tt1", "type": "series", "videos": [
                { "id": "tt1:1:1", "episode": 1, "number": 1, "title": "A", "name": "A" },
                { "id": "tt1:1:2", "number": "2", "name": "B" }
            ]
        }}));
        assert!(parsed.dropped.is_empty(), "{:?}", parsed.dropped);
        let videos = parsed.value.meta.unwrap().videos;
        assert_eq!(videos[0].episode_number(), Some(1));
        assert_eq!(videos[1].episode_number(), Some(2));
        assert_eq!(videos[1].display_title(), Some("B"));
    }

    #[test]
    fn movie_without_videos_uses_meta_id() {
        let meta: Meta = serde_json::from_value(json!({ "id": "tt1", "type": "movie" })).unwrap();
        assert_eq!(meta.default_video_id(), Some("tt1"));
        let series: Meta = serde_json::from_value(json!({
            "id": "tt2", "type": "series", "videos": [{ "id": "tt2:1:1" }]
        }))
        .unwrap();
        assert_eq!(series.default_video_id(), None);
    }
}
