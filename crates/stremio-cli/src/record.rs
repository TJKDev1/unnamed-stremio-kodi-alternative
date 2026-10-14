//! `stremio-cli record`: save an addon's responses as conformance fixtures (T0.S3.7, D61).
//!
//! Every body is scrubbed (secrets, D57) and anonymized (D62) **before** it is written, so a
//! recording of an addon whose URL holds a debrid key never puts the key on disk. Keys the
//! heuristics can't spot are passed in `FIXTURE_SECRETS`, as for `app-fixtures scrub`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use app_addons_stremio::{
    Descriptor, ExtraProp, Manifest, ResourcePath, ResourceResponse, Transport, Validated,
    manifest, parse_resource,
};
use app_fixtures::{Scrubber, anonymize};
use serde_json::{Value, json};

/// Ids that most stream/subtitle/meta addons know: Inception, Game of Thrones S1E1, Cowboy
/// Bebop E1 (Kitsu).
const KNOWN_IDS: &[(&str, &str)] = &[
    ("movie", "tt1375666"),
    ("series", "tt0944947:1:1"),
    ("series", "kitsu:1:1"),
    ("anime", "kitsu:1:1"),
];
const KNOWN_METAS: &[(&str, &str)] = &[
    ("movie", "tt1375666"),
    ("series", "tt0944947"),
    ("series", "kitsu:1"),
    ("anime", "kitsu:1"),
];
const MAX_CATALOGS: usize = 8;
const MAX_METAS: usize = 3;
const MAX_STREAMS: usize = 5;
/// Item lists are cut to this many entries: order and shape survive, megabytes don't.
const MAX_ITEMS: usize = 10;
const ITEM_LISTS: &[&str] = &[
    "metas",
    "metasDetailed",
    "videos",
    "streams",
    "subtitles",
    "addons",
];

pub struct Recorder<'a, T: Transport> {
    transport: &'a T,
    url: String,
    /// Staging folder; moved to `final_dir` only after everything verified clean.
    dir: PathBuf,
    final_dir: PathBuf,
    scrubber: Scrubber,
    index: Vec<Value>,
    redacted_files: usize,
}

impl<'a, T: Transport> Recorder<'a, T> {
    /// Records into `out/name`. Work happens in `out/.name.recording`, so a failed run never
    /// touches an existing recording.
    pub fn new(transport: &'a T, url: &str, out: &Path, name: &str) -> Result<Self, String> {
        let url = manifest::transport_url(url).map_err(|e| e.to_string())?;
        let dir = out.join(format!(".{name}.recording"));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let mut scrubber = Scrubber::new().addon_url(&url);
        if let Ok(env) = std::env::var("FIXTURE_SECRETS") {
            for secret in env.split([',', '\n']).map(str::trim) {
                scrubber = scrubber.secret(secret);
            }
        }
        Ok(Self {
            transport,
            url,
            dir,
            final_dir: out.join(name),
            scrubber,
            index: Vec::new(),
            redacted_files: 0,
        })
    }

    /// `text` with anything secret-looking removed. Everything printed goes through this:
    /// parse errors and addon-supplied strings can quote URLs and keys.
    fn safe(&self, text: &str) -> String {
        self.scrubber.scrub_str(text).0
    }

    fn say(&self, text: &str) {
        println!("{}", self.safe(text));
    }

    /// Records, verifies, then replaces `out/name`. On any error the staging folder is removed
    /// and an existing recording stays as it was.
    pub async fn run(mut self, explicit: Vec<ResourcePath>) -> Result<(), String> {
        let result = self.record_all(explicit).await.and_then(|()| self.verify());
        let result = result.and_then(|()| self.install());
        match result {
            Ok(()) => {
                self.say(&format!(
                    "recorded {} request(s) into {}",
                    self.index.len(),
                    self.final_dir.display()
                ));
                Ok(())
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&self.dir);
                Err(self.safe(&e))
            }
        }
    }

    /// Swaps the verified staging folder in. The old recording is moved aside first and put
    /// back if the swap fails, so there is always one complete recording on disk.
    fn install(&self) -> Result<(), String> {
        let err = |e: std::io::Error| format!("{}: {e}", self.final_dir.display());
        let backup = self.dir.with_extension("previous");
        if backup.exists() {
            std::fs::remove_dir_all(&backup).map_err(err)?;
        }
        let had_old = self.final_dir.exists();
        if had_old {
            std::fs::rename(&self.final_dir, &backup).map_err(err)?;
        }
        if let Err(e) = std::fs::rename(&self.dir, &self.final_dir) {
            if had_old {
                let _ = std::fs::rename(&backup, &self.final_dir);
            }
            return Err(err(e));
        }
        if had_old {
            let _ = std::fs::remove_dir_all(&backup);
        }
        Ok(())
    }

    /// Records the manifest, then `explicit` paths or, if none, an automatic plan.
    async fn record_all(&mut self, explicit: Vec<ResourcePath>) -> Result<(), String> {
        let body = self
            .transport
            .get(&self.url)
            .await
            .map_err(|e| format!("manifest: {e}"))?;
        if !(200..300).contains(&body.status) {
            return Err(format!("manifest: HTTP {}", body.status));
        }
        let Validated { manifest, warnings } =
            Manifest::from_json(&body.body).map_err(|e| e.to_string())?;
        for w in &warnings {
            self.say(&format!("    warning: {}", w.0));
        }
        self.write("manifest.json", &body.body, &mut false)?;
        self.say(&format!(
            "recorded manifest of {} {}",
            manifest.name, manifest.version
        ));
        let addon = Descriptor {
            transport_url: self.url.clone(),
            manifest,
            other: Default::default(),
        };

        if explicit.is_empty() {
            self.auto(&addon).await?;
        } else {
            for path in explicit {
                if !addon.manifest.supports(&path) {
                    self.say(&format!(
                        "    note: the manifest doesn't claim {}",
                        path.to_url_path()
                    ));
                }
                self.record(&path).await?;
            }
        }
        self.write_index()
    }

    async fn auto(&mut self, addon: &Descriptor) -> Result<(), String> {
        let m = &addon.manifest;
        let mut seen_ids: Vec<(String, String)> = Vec::new();

        // Catalogs: plain ones, then one each with skip, genre and search.
        let mut catalogs = Vec::new();
        for c in m.catalogs.iter().take(MAX_CATALOGS) {
            let required: Vec<_> = c.extra.iter().filter(|e| e.is_required).collect();
            let mut path = ResourcePath::new("catalog", &c.r#type, &c.id);
            for e in required {
                let value = match (&e.options, e.name.as_str()) {
                    (Some(options), _) if !options.is_empty() => options[0].clone(),
                    (_, "search") => "dune".to_owned(),
                    _ => continue,
                };
                path = path.with_extra(&e.name, &value);
            }
            catalogs.push(path);
        }
        let variant = |name: &str, value: &dyn Fn(&ExtraProp) -> Option<String>| {
            m.catalogs.iter().find_map(|c| {
                let e = c.extra.iter().find(|e| e.name == name)?;
                if c.extra.iter().any(|o| o.is_required && o.name != name) {
                    return None;
                }
                Some(ResourcePath::new("catalog", &c.r#type, &c.id).with_extra(name, &value(e)?))
            })
        };
        catalogs.extend(variant("skip", &|_| Some("100".into())));
        catalogs.extend(variant("genre", &|e| e.options.as_ref()?.first().cloned()));
        catalogs.extend(variant("search", &|_| Some("dune".into())));
        let mut unique = Vec::new();
        for path in catalogs {
            if !unique.contains(&path) {
                unique.push(path);
            }
        }
        let catalogs = unique;
        for path in &catalogs {
            if let Some(ResourceResponse::Catalog(r)) = self.record(path).await?
                && let Some(first) = r.metas.first()
            {
                seen_ids.push((first.r#type.clone(), first.id.clone()));
            }
        }
        for c in m.addon_catalogs.iter().take(2) {
            self.record(&ResourcePath::new("addon_catalog", &c.r#type, &c.id))
                .await?;
        }

        // Metas: an id from a catalog first, then well-known ones.
        let mut video_ids: Vec<(String, String)> = Vec::new();
        let meta_candidates = seen_ids
            .iter()
            .cloned()
            .chain(KNOWN_METAS.iter().map(|(t, i)| ((*t).into(), (*i).into())));
        let metas = supported(m, "meta", meta_candidates, MAX_METAS);
        for path in metas {
            if let Some(ResourceResponse::Meta(r)) = self.record(&path).await?
                && let Some(meta) = &r.meta
            {
                let video = meta
                    .videos
                    .first()
                    .map(|v| v.id.clone())
                    .or_else(|| meta.default_video_id().map(str::to_owned));
                if let Some(id) = video {
                    video_ids.push((meta.preview.r#type.clone(), id));
                }
            }
        }

        // Streams and subtitles: well-known ids, then ids seen above (live TV, channels).
        let candidates: Vec<(String, String)> = KNOWN_IDS
            .iter()
            .map(|(t, i)| ((*t).into(), (*i).into()))
            .chain(video_ids)
            .chain(seen_ids)
            .collect();
        for resource in ["stream", "subtitles"] {
            for path in supported(m, resource, candidates.iter().cloned(), MAX_STREAMS) {
                self.record(&path).await?;
            }
        }
        Ok(())
    }

    /// Fetches one path, writes it if it's a usable response, and logs it in the index.
    /// Addon failures are logged and skipped; failing to write is an error for the whole run.
    async fn record(&mut self, path: &ResourcePath) -> Result<Option<ResourceResponse>, String> {
        let shown = path.to_url_path();
        let response = match self.transport.get(&path.url(&self.url)).await {
            Ok(r) => r,
            Err(e) => {
                self.say(&format!("    {shown}: {e}"));
                self.index
                    .push(json!({ "path": shown, "error": e.to_string() }));
                return Ok(None);
            }
        };
        let parsed = (200..300)
            .contains(&response.status)
            .then(|| parse_resource(&path.resource, &response.body));
        let mut trimmed = false;
        let (file, result) = match parsed {
            Some(Ok(p)) => {
                let file = self.file_for(path);
                self.write(&file, &response.body, &mut trimmed)?;
                let note = if p.dropped.is_empty() {
                    String::new()
                } else {
                    format!(", {} item(s) dropped", p.dropped.len())
                };
                self.say(&format!("    {shown}: HTTP {}{note}", response.status));
                (Some(file), Some(p.value))
            }
            Some(Err(e)) => {
                self.say(&format!(
                    "    {shown}: HTTP {}, unusable: {e}",
                    response.status
                ));
                (None, None)
            }
            None => {
                self.say(&format!("    {shown}: HTTP {}", response.status));
                (None, None)
            }
        };
        self.index.push(json!({
            "path": shown,
            "status": response.status,
            "cacheControl": response.cache_control,
            "file": file,
            "trimmed": trimmed,
        }));
        Ok(result)
    }

    /// The fixture file for `path`, unless an id or extra would put something secret into a
    /// file name; then `<resource>/redacted/<n>.json`.
    fn file_for(&mut self, path: &ResourcePath) -> String {
        let raw = format!(
            "{}/{}/{}/{}",
            path.resource,
            path.r#type,
            path.id,
            path.extra
                .iter()
                .map(|(n, v)| format!("{n}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        );
        let file = path.fixture_path();
        if [raw.as_str(), &path.to_url_path(), &file]
            .iter()
            .any(|text| self.safe(text) != *text)
        {
            self.redacted_files += 1;
            return format!("{}/redacted/{}.json", path.resource, self.redacted_files);
        }
        file
    }

    /// Trims, scrubs, anonymizes and writes one JSON body under the addon folder.
    fn write(&self, rel: &str, body: &[u8], trimmed: &mut bool) -> Result<(), String> {
        let mut value: Value =
            serde_json::from_slice(body).map_err(|e| format!("{rel}: not JSON: {e}"))?;
        *trimmed = trim(&mut value);
        self.clean(&mut value);
        write_json(&self.dir.join(rel), &value)
    }

    fn clean(&self, value: &mut Value) {
        self.scrubber.scrub(value);
        anonymize::anonymize(value);
    }

    fn write_index(&self) -> Result<(), String> {
        let mut index = json!({
            "addon": self.url,
            "recorded": today(),
            "requests": self.index,
        });
        self.clean(&mut index);
        write_json(&self.dir.join("index.json"), &index)
    }

    /// Belt and braces: nothing written, contents or file names, may still look secret.
    fn verify(&self) -> Result<(), String> {
        let mut dirty = 0;
        visit_json(&self.dir, &mut |file, value| {
            let rel = file
                .strip_prefix(&self.dir)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/");
            if self.scrubber.check_path(&rel).is_some() {
                eprintln!("a file name looks secret; not writing the recording");
                dirty += 1;
            }
            for f in self
                .scrubber
                .check(value)
                .into_iter()
                .chain(anonymize::check(value))
            {
                eprintln!("{}: {} at {}", self.safe(&rel), f.reason, f.pointer);
                dirty += 1;
            }
        })?;
        if dirty > 0 {
            return Err(format!(
                "{dirty} value(s) still look secret; not safe to commit"
            ));
        }
        Ok(())
    }
}

/// Cuts every item list to [`MAX_ITEMS`]; returns whether anything was cut.
fn trim(value: &mut Value) -> bool {
    let mut cut = false;
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if let Value::Array(items) = child
                    && ITEM_LISTS.contains(&key.as_str())
                    && items.len() > MAX_ITEMS
                {
                    items.truncate(MAX_ITEMS);
                    cut = true;
                }
                cut |= trim(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                cut |= trim(child);
            }
        }
        _ => {}
    }
    cut
}

/// The first `max` candidates the addon claims to serve for `resource`, without repeats.
fn supported(
    m: &Manifest,
    resource: &str,
    candidates: impl Iterator<Item = (String, String)>,
    max: usize,
) -> Vec<ResourcePath> {
    let mut out: Vec<ResourcePath> = Vec::new();
    for (t, id) in candidates {
        let path = ResourcePath::new(resource, &t, &id);
        if out.len() < max && m.supports(&path) && !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// Parses `resource/type/id[/extra]` as typed by a person (not URL-encoded).
pub fn parse_path(text: &str) -> Result<ResourcePath, String> {
    let text = text.trim_start_matches('/').trim_end_matches(".json");
    let mut parts = text.splitn(4, '/');
    let (Some(resource), Some(r#type), Some(id)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("`{text}` is not RESOURCE/TYPE/ID[/NAME=VALUE&...]"));
    };
    let mut path = ResourcePath::new(resource, r#type, id);
    if let Some(extra) = parts.next() {
        for pair in extra.split('&') {
            let (name, value) = pair
                .split_once('=')
                .ok_or(format!("extra `{pair}` is not NAME=VALUE"))?;
            path = path.with_extra(name, value);
        }
    }
    Ok(path)
}

fn write_json(file: &Path, value: &Value) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())? + "\n";
    std::fs::write(file, text).map_err(|e| format!("{}: {e}", file.display()))
}

fn visit_json(dir: &Path, f: &mut dyn FnMut(&Path, &Value)) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            visit_json(&path, f)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            f(&path, &value);
        }
    }
    Ok(())
}

/// Today's UTC date as `YYYY-MM-DD` (Howard Hinnant's days-to-civil).
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_typed_by_hand() {
        let p = parse_path("stream/series/tt0944947:1:1").unwrap();
        assert_eq!(p.to_url_path(), "/stream/series/tt0944947%3A1%3A1.json");
        let p = parse_path("/catalog/movie/top/genre=Sci-Fi&skip=20.json").unwrap();
        assert_eq!(p.extra.len(), 2);
        assert!(parse_path("stream/movie").is_err());
    }

    #[test]
    fn secret_ids_never_become_file_names() {
        struct NoNet;
        impl Transport for NoNet {
            async fn get(
                &self,
                _: &str,
            ) -> Result<app_addons_stremio::HttpResponse, app_addons_stremio::TransportError>
            {
                unreachable!()
            }
        }
        // SAFETY of the test: FIXTURE_SECRETS is not read here; the key comes from the URL query.
        let out = std::env::temp_dir();
        let mut r = Recorder::new(
            &NoNet,
            "https://a.example/manifest.json?key=supersecretkey",
            &out,
            "secret-ids-test",
        )
        .unwrap();
        let path = ResourcePath::new("stream", "movie", "tt1:supersecretkey");
        assert_eq!(r.file_for(&path), "stream/redacted/1.json");
        let ok = ResourcePath::new("stream", "movie", "tt1");
        assert_eq!(r.file_for(&ok), "stream/movie/tt1.json");
        assert!(
            !r.safe("error at https://a.example/manifest.json?key=supersecretkey")
                .contains("supersecretkey")
        );
    }

    #[test]
    fn date_format() {
        let d = today();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"));
    }
}
