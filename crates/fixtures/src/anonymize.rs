//! Anonymizes stream and subtitle content in recorded fixtures (D62).
//!
//! The repo is public, and scraper responses name pirated releases. Tests need the *shape* of a
//! response and the tokens ranking cares about (resolution, codec, HDR, size, languages), not
//! the work or the release. So inside every `streams[]` and `subtitles[]` item:
//!
//! - text keeps whitelisted technical tokens, numbers, emoji and punctuation; every other word
//!   becomes `Title` (`Inception.2010.2160p.UHD.BluRay.x265-GROUP` →
//!   `Title.2010.2160p.UHD.BluRay.x265-Title`);
//! - info hashes and video hashes become zero-padded counters of the same length;
//! - URLs become `<scheme>://stream.invalid/<n>[.ext]`, magnet links a magnet for a fake hash.
//!
//! Subtitle `lang`/encoding codes, `ytId` and `countryWhitelist` stay. Everything else in an
//! item goes through the rules, at any depth (a `videoHash` nested in `behaviorHints` is a
//! hash; a `tracker:<url>` source is a URL). The rules are idempotent: anonymizing anonymized output changes nothing, which is
//! what [`check`] tests in CI.

use serde_json::{Map, Value};

use crate::Finding;

const PLACEHOLDER: &str = "Title";
const FAKE_HOST: &str = "stream.invalid";

/// Technical words kept as they are (compared lower-case).
const KEEP_WORDS: &[&str] = &[
    "title",    // the placeholder itself, so the rules are idempotent
    "redacted", // the scrubber's marker
    // quality and source
    "uhd",
    "fhd",
    "qhd",
    "hd",
    "sd",
    "bluray",
    "blu",
    "ray",
    "bd",
    "bdrip",
    "brrip",
    "bdremux",
    "remux",
    "web",
    "webdl",
    "webrip",
    "dl",
    "rip",
    "hdtv",
    "hdrip",
    "dvdrip",
    "dvd",
    "dvdscr",
    "cam",
    "hdcam",
    "ts",
    "tc",
    "telesync",
    "telecine",
    "scr",
    "screener",
    "proper",
    "repack",
    "rerip",
    "extended",
    "unrated",
    "uncut",
    "directors",
    "cut",
    "theatrical",
    "imax",
    "remastered",
    "nf",
    "amzn",
    "dsnp",
    "hmax",
    "atvp",
    "hulu",
    "pcok",
    "max",
    // video
    "hdr",
    "hdr10plus",
    "plus",
    "dv",
    "dovi",
    "dolby",
    "vision",
    "hlg",
    "sdr",
    "hevc",
    "avc",
    "xvid",
    "divx",
    "10bit",
    "8bit",
    // audio
    "aac",
    "eac",
    "ddp",
    "dd",
    "dts",
    "ma",
    "hra",
    "x",
    "truehd",
    "atmos",
    "flac",
    "opus",
    "lpcm",
    "pcm",
    "ch",
    "audio",
    "dual",
    "multi",
    // containers and sizes
    "mkv",
    "mp",
    "avi",
    "m2ts",
    "webm",
    "iso",
    "gb",
    "mb",
    "tb",
    "kb",
    "gib",
    "mib",
    // languages
    "sub",
    "subs",
    "subbed",
    "dubbed",
    "dub",
    "eng",
    "en",
    "ita",
    "it",
    "fr",
    "fre",
    "fra",
    "es",
    "spa",
    "esp",
    "de",
    "ger",
    "deu",
    "ja",
    "jp",
    "jpn",
    "ru",
    "rus",
    "pt",
    "por",
    "br",
    "ko",
    "kor",
    "zh",
    "chi",
    "hin",
    "english",
    "italian",
    "french",
    "spanish",
    "german",
    "japanese",
    "russian",
    "portuguese",
    "korean",
    "chinese",
    "hindi",
    "latino",
    "castellano",
    "vostfr",
    "vf",
    "vo",
    // debrid / addon vocabulary
    "cached",
    "uncached",
    "download",
    "torrent",
    "debrid",
    "torbox",
    "rd",
    "ad",
    "pm",
    "p2p",
    "usenet",
    "seeders",
    "peers",
    "size",
    "season",
    "seasons",
    "episode",
    "episodes",
    "complete",
    "pack",
    "batch",
    "part",
    "vol",
    "movie",
    "series",
    "anime",
    "sample",
    "unknown",
    "aiostreams",
    "torrentio",
    "comet",
    "mediafusion",
    "jackettio",
    "zilean",
    "via",
    "from",
    "in",
    "s",
    "e",
];

/// Prefixes that may be followed by digits (`x265`, `hdr10`, `s01`, `ddp5`).
const KEEP_PREFIXES: &[&str] = &[
    "x", "h", "hdr", "ac", "dd", "ddp", "eac", "av", "vp", "mp", "aac", "dts", "s", "e", "ep",
    "mpeg", "pcm", "lpcm", "m2ts", "hevc", "opus", "flac", "truehd", "atmos", "ch",
];

/// Suffixes that may follow digits (`2160p`, `4k`, `10bit`, `8gb`, `3d`, `60fps`).
const KEEP_SUFFIXES: &[&str] = &[
    "p", "i", "k", "bit", "bits", "gb", "mb", "kb", "tb", "gib", "mib", "fps", "ch", "hz", "d",
    "th", "st", "nd", "rd", "plus",
];

/// Keys whose string values are kept verbatim in any item: YouTube ids are public trailers,
/// country codes drive geo-filtering logic.
const KEEP_KEYS: &[&str] = &["ytId", "countryWhitelist"];
/// Extra verbatim keys inside a subtitle item (language and encoding codes).
const SUBTITLE_KEEP_KEYS: &[&str] = &["lang", "SubEncoding", "m", "g"];
/// Keys holding content hashes, at any depth.
const HASH_KEYS: &[&str] = &["infoHash", "videoHash"];

/// Anonymizes `value` in place and returns what changed.
pub fn anonymize(value: &mut Value) -> Vec<Finding> {
    let mut state = State::default();
    walk(value, &mut String::new(), &mut state);
    state.findings
}

/// What [`anonymize`] would change; empty means the fixture is already anonymized.
pub fn check(value: &Value) -> Vec<Finding> {
    anonymize(&mut value.clone())
}

#[derive(Default)]
struct State {
    counter: u64,
    findings: Vec<Finding>,
}

impl State {
    fn next(&mut self) -> u64 {
        self.counter += 1;
        self.counter
    }

    fn set(&mut self, slot: &mut Value, new: String, pointer: &str, reason: &'static str) {
        if slot.as_str() != Some(new.as_str()) {
            *slot = Value::String(new);
            self.findings.push(Finding {
                pointer: pointer.to_owned(),
                reason,
            });
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Stream,
    Subtitle,
}

/// Finds `streams` / `subtitles` lists anywhere (responses, `meta.videos[].streams`).
fn walk(value: &mut Value, pointer: &mut String, state: &mut State) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                let len = pointer.len();
                push(pointer, key);
                match (key.as_str(), child) {
                    ("streams", Value::Array(items)) => {
                        items_of(items, Item::Stream, pointer, state)
                    }
                    ("subtitles", Value::Array(items)) => {
                        items_of(items, Item::Subtitle, pointer, state)
                    }
                    (_, child) => walk(child, pointer, state),
                }
                pointer.truncate(len);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter_mut().enumerate() {
                let len = pointer.len();
                push(pointer, &i.to_string());
                walk(child, pointer, state);
                pointer.truncate(len);
            }
        }
        _ => {}
    }
}

fn items_of(items: &mut [Value], kind: Item, pointer: &mut String, state: &mut State) {
    for (i, item) in items.iter_mut().enumerate() {
        let len = pointer.len();
        push(pointer, &i.to_string());
        if let Value::Object(map) = item {
            item_fields(map, kind, pointer, state);
        }
        pointer.truncate(len);
    }
}

fn item_fields(map: &mut Map<String, Value>, kind: Item, pointer: &mut String, state: &mut State) {
    for (key, child) in map.iter_mut() {
        let len = pointer.len();
        push(pointer, key);
        match (kind, key.as_str(), child) {
            (Item::Stream, "subtitles", Value::Array(items)) => {
                items_of(items, Item::Subtitle, pointer, state)
            }
            (_, key, child) => field(key, child, kind, pointer, state),
        }
        pointer.truncate(len);
    }
}

/// The rule for one value inside an item, by the key it sits under (array elements inherit
/// their list's key), at any depth: a hash nested in `behaviorHints` is still a hash.
fn field(key: &str, value: &mut Value, kind: Item, pointer: &mut String, state: &mut State) {
    match value {
        Value::String(s) => {
            if KEEP_KEYS.contains(&key)
                || (kind == Item::Subtitle && SUBTITLE_KEEP_KEYS.contains(&key))
            {
                return;
            }
            if HASH_KEYS.contains(&key) {
                if !is_fake_hash(s) {
                    let fake = fake_hash(s.len(), state.next());
                    state.set(value, fake, pointer, "hash");
                }
                return;
            }
            if key == "sources" {
                // `dht:<infohash>`, `tracker:<url>`: keep the prefix, anonymize the rest.
                if let Some(hash) = s.strip_prefix("dht:") {
                    if !is_fake_hash(hash) {
                        let fake = format!("dht:{}", fake_hash(hash.len(), state.next()));
                        state.set(value, fake, pointer, "hash");
                    }
                    return;
                }
                if let Some((prefix, rest)) = s.split_once(':')
                    && !prefix.is_empty()
                    && prefix.chars().all(|c| c.is_ascii_alphanumeric())
                    && rest.contains("://")
                {
                    if let Url::Replace(fake) = fake_url(rest, state) {
                        let fake = format!("{prefix}:{fake}");
                        state.set(value, fake, pointer, "url");
                    }
                    return;
                }
            }
            match fake_url(s, state) {
                Url::Already => {}
                Url::Replace(fake) => state.set(value, fake, pointer, "url"),
                Url::Plain => {
                    let text = anonymize_text(s);
                    state.set(value, text, pointer, "text");
                }
            }
        }
        Value::Object(map) => {
            for (child_key, child) in map.iter_mut() {
                let len = pointer.len();
                push(pointer, child_key);
                field(child_key, child, kind, pointer, state);
                pointer.truncate(len);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter_mut().enumerate() {
                let len = pointer.len();
                push(pointer, &i.to_string());
                field(key, child, kind, pointer, state);
                pointer.truncate(len);
            }
        }
        _ => {}
    }
}

enum Url {
    Plain,
    Already,
    Replace(String),
}

/// A replacement for `s` if it is a URL (or magnet) that isn't already fake.
fn fake_url(s: &str, state: &mut State) -> Url {
    if let Some(rest) = s.strip_prefix("magnet:") {
        let fake_prefix = format!("?xt=urn:btih:{}", "0".repeat(32));
        if rest.starts_with(&fake_prefix) {
            return Url::Already;
        }
        return Url::Replace(format!(
            "magnet:?xt=urn:btih:{}",
            fake_hash(40, state.next())
        ));
    }
    let Some((scheme, rest)) = s.split_once("://") else {
        return Url::Plain;
    };
    let valid_scheme = scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    if !valid_scheme {
        return Url::Plain;
    }
    if rest.starts_with(&format!("{FAKE_HOST}/")) {
        return Url::Already;
    }
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    let ext = path
        .rsplit('/')
        .next()
        .and_then(|last| last.rsplit_once('.'))
        .map(|(_, ext)| ext)
        .filter(|ext| {
            (2..=4).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map(|ext| format!(".{}", ext.to_ascii_lowercase()))
        .unwrap_or_default();
    Url::Replace(format!("{scheme}://{FAKE_HOST}/{}{ext}", state.next()))
}

fn fake_hash(len: usize, n: u64) -> String {
    // At least half the string is leading zeros, so `is_fake_hash` recognizes it.
    let len = len.max(2 * format!("{n:x}").len());
    let hex = format!("{n:x}");
    format!("{}{hex}", "0".repeat(len.saturating_sub(hex.len())))
}

/// A hash we generated (or the scrubber's marker): hex with at least half leading zeros,
/// which [`fake_hash`] guarantees for any length and real hashes practically never have.
fn is_fake_hash(s: &str) -> bool {
    if s == crate::REDACTED {
        return true;
    }
    let zeros = s.chars().take_while(|c| *c == '0').count();
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit()) && zeros * 2 >= s.len()
}

/// Replaces every word that isn't a technical token with `Title`.
pub fn anonymize_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut word = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

fn flush(word: &mut String, out: &mut String) {
    if word.is_empty() {
        return;
    }
    if keep_token(word) {
        out.push_str(word);
    } else {
        out.push_str(PLACEHOLDER);
    }
    word.clear();
}

fn keep_token(token: &str) -> bool {
    let t = token.to_lowercase();
    if t.chars().all(|c| c.is_ascii_digit()) || KEEP_WORDS.contains(&t.as_str()) {
        return true;
    }
    if !t.is_ascii() {
        return false;
    }
    // digits + suffix: 2160p, 4k, 10bit
    let digits_end = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    if digits_end > 0 && KEEP_SUFFIXES.contains(&&t[digits_end..]) {
        return true;
    }
    // prefix + digits: x265, hdr10, s01, ddp5
    let letters_end = t.find(|c: char| c.is_ascii_digit()).unwrap_or(t.len());
    let (prefix, rest) = t.split_at(letters_end);
    if !rest.is_empty() && KEEP_PREFIXES.contains(&prefix) {
        // s01e02, hdr10plus
        let rest_digits_end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let tail = &rest[rest_digits_end..];
        return tail.is_empty()
            || KEEP_SUFFIXES.contains(&tail)
            || (tail.starts_with('e')
                && tail[1..].chars().all(|c| c.is_ascii_digit())
                && tail.len() > 1);
    }
    // 1x02, 1920x1080
    if let Some((a, b)) = t.split_once('x') {
        return !a.is_empty()
            && !b.is_empty()
            && a.chars().all(|c| c.is_ascii_digit())
            && b.chars().all(|c| c.is_ascii_digit());
    }
    false
}

/// RFC 6901 pointer segment.
fn push(pointer: &mut String, segment: &str) {
    pointer.push('/');
    pointer.push_str(&segment.replace('~', "~0").replace('/', "~1"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn text_keeps_technical_tokens_only() {
        assert_eq!(
            anonymize_text("Inception.2010.2160p.UHD.BluRay.x265-GROUP"),
            "Title.2010.2160p.UHD.BluRay.x265-Title"
        );
        assert_eq!(
            anonymize_text(
                "Inception 2010 PROPER Bluray 2160p AV1 HDR10 EN/ITA OPUS 5.1-UH [UserHEVC]\n👤 293 💾 8.91 GB ⚙️ 1337x\n🇬🇧 / 🇮🇹"
            ),
            "Title 2010 PROPER Bluray 2160p AV1 HDR10 EN/ITA OPUS 5.1-Title [Title]\n👤 293 💾 8.91 GB ⚙️ Title\n🇬🇧 / 🇮🇹"
        );
        assert_eq!(
            anonymize_text("Show S01E02 1x02 DDP5.1 10bit 4K DV HDR10Plus"),
            "Title S01E02 1x02 DDP5.1 10bit 4K DV HDR10Plus"
        );
        assert_eq!(anonymize_text("進撃の巨人 1080p"), "Title 1080p");
    }

    #[test]
    fn hashes_stay_stable_across_runs() {
        let mut v = json!({ "streams": [
            { "infoHash": "2849211f150e966ef7924f43aec22727a644e10d",
              "behaviorHints": { "videoHash": "REDACTED" } },
            { "behaviorHints": { "videoHash": "9f8e7d6c" } }
        ]});
        anonymize(&mut v);
        assert_eq!(v["streams"][0]["behaviorHints"]["videoHash"], "REDACTED");
        let short = v["streams"][1]["behaviorHints"]["videoHash"]
            .as_str()
            .unwrap();
        assert!(is_fake_hash(short) && short.len() == 8, "{short}");
        assert!(check(&v).is_empty(), "{:?}", check(&v));
        assert!(!is_fake_hash("9f8e7d6c"));
    }

    #[test]
    fn streams_and_subtitles_are_anonymized_and_idempotent() {
        let mut v = json!({
            "streams": [{
                "name": "Torrentio\n4k HDR",
                "title": "Inception 2010 2160p",
                "infoHash": "2849211f150e966ef7924f43aec22727a644e10d",
                "fileIdx": 0,
                "sources": ["tracker:udp://open.tracker/announce", "dht:2849211f150e966ef7924f43aec22727a644e10d"],
                "behaviorHints": { "bingeGroup": "torrentio|4k|HDR", "filename": "Inception.2010.mkv", "videoSize": 123,
                                   "videoHash": "1234567890123456" }
            }, {
                "url": "https://api.torbox.app/REDACTED/Inception.2010.mkv?x=1",
                "externalUrl": "magnet:?xt=urn:btih:abc",
                "ytId": "dQw4w9WgXcQ"
            }],
            "subtitles": [{ "id": "Inception.2010-3728010", "url": "https://subs/Inception.2010.srt", "lang": "eng",
                            "movieReleaseName": "Inception.2010.CAM.Xvid-LKRG" }],
            "meta": { "name": "Inception", "videos": [{ "id": "v", "streams": [{ "title": "Inception" }] }] }
        });
        let findings = anonymize(&mut v);
        assert!(!findings.is_empty());
        let s = &v["streams"][0];
        assert_eq!(s["name"], "Torrentio\n4k HDR");
        assert_eq!(s["title"], "Title 2010 2160p");
        let fake = |v: &Value| is_fake_hash(v.as_str().unwrap().trim_start_matches("dht:"));
        assert!(fake(&s["infoHash"]));
        // Tracker URLs can carry passkeys; the DHT entry names the torrent.
        let tracker = s["sources"][0].as_str().unwrap();
        assert!(
            tracker.starts_with("tracker:udp://stream.invalid/"),
            "{tracker}"
        );
        assert!(fake(&s["sources"][1]));
        assert_eq!(s["behaviorHints"]["filename"], "Title.2010.mkv");
        assert_eq!(s["behaviorHints"]["videoSize"], 123);
        assert!(fake(&s["behaviorHints"]["videoHash"]));
        assert_eq!(s["behaviorHints"]["videoHash"].as_str().unwrap().len(), 16);
        let url = v["streams"][1]["url"].as_str().unwrap();
        assert!(
            url.starts_with("https://stream.invalid/") && url.ends_with(".mkv"),
            "{url}"
        );
        assert!(
            v["streams"][1]["externalUrl"]
                .as_str()
                .unwrap()
                .starts_with("magnet:?xt=urn:btih:0000")
        );
        assert_eq!(v["streams"][1]["ytId"], "dQw4w9WgXcQ");
        assert_eq!(v["subtitles"][0]["id"], "Title.2010-3728010");
        assert!(
            v["subtitles"][0]["url"]
                .as_str()
                .unwrap()
                .starts_with("https://stream.invalid/")
        );
        assert_eq!(v["subtitles"][0]["lang"], "eng");
        assert_eq!(
            v["subtitles"][0]["movieReleaseName"],
            "Title.2010.CAM.Xvid-Title"
        );
        // Meta text outside streams is real metadata and stays.
        assert_eq!(v["meta"]["name"], "Inception");
        assert_eq!(v["meta"]["videos"][0]["streams"][0]["title"], "Title");
        assert!(check(&v).is_empty(), "{:?}", check(&v));
    }
}
