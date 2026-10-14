# Spec: Stremio addon client (protocol v1)

**Status:** draft · **Implemented by:** S3, M1.2, M1.3 · **Decisions:** D15, D44, D48, D60, D61, D62, D63 · **Code:** `crates/addons-stremio` (`app-addons-stremio`), dev CLI `crates/stremio-cli`

## Purpose
100% compatible client for the Stremio addon protocol. Design: [Stremio addon protocol](../design/04-addons.md#stremio-addon-protocol-must-be-100-compatible). stremio-core is a reference for behaviour and test cases only (D44).

## Scope
- **In:** manifest, all resources (`catalog`, `meta`, `stream`, `subtitles`, `addon_catalog`), extras, IDs, caching hints, configure flow, account import, EPG.
- **Out:** `ipfs://`/`ipns://` transport and the legacy `/stremio/v1` JSON-RPC transport (low priority); playing `infoHash`/`nzbUrl`/archive streams (plugins).
- **S3 covers:** models, manifest validation, routes, router + fan-out, cache policy, recording. Configure flow, account import and EPG date routes come in M1.2.

## Contract

### Layering (D60)
The crate is **sans-IO**: models, validation, routes, routing and cache policy are plain functions over data. All network access goes through one trait:

```rust
pub trait Transport: Send + Sync {
    /// GET `url`. Implementations apply their own timeout and return the body whatever the status.
    fn get(&self, url: &str) -> impl Future<Output = Result<HttpResponse, TransportError>> + Send;
}
pub struct HttpResponse { pub status: u16, pub cache_control: Option<String>, pub body: Vec<u8> }
```

The default implementation (`ReqwestTransport`, feature `reqwest`, on by default) uses reqwest + rustls on tokio. Fan-out uses only `futures` combinators, so the crate never names a runtime; per-request timeouts live in the transport. Tests use a fixture-backed fake transport.

### Models
`Manifest`, `ManifestResource`, `ManifestCatalog`, `ExtraProp`, `AddonCatalogDef`, `MetaPreview`, `Meta`, `Video`, `Stream` (+ `StreamSource`), `Subtitle`, `Descriptor { transport_url, manifest }`, and one response type per resource.

- Field names follow the protocol (`camelCase` on the wire, `snake_case` in Rust).
- **Unknown fields are preserved** in an `other: Map<String, Value>` on every object, and re-serialized unchanged.
- **Lists are lenient:** an item that fails to parse is dropped and reported in `ParseReport.dropped` (index + error); the rest of the response still loads. Required-shape errors on the top level (no `metas`/`meta`/`streams`/`subtitles`/`addons` key) fail the response.
- `ManifestResource` accepts a string (`"stream"`) or an object (`{name, types, idPrefixes}`).
- Legacy catalog extras (`extraSupported: [names]`, `extraRequired: [names]`) are normalized into `extra: [ExtraProp]` (`isRequired` set for names in `extraRequired`). The original fields are not re-emitted.
- `Stream` keeps all protocol fields; `Stream::source()` classifies it as `Url`, `YouTube`, `Torrent { info_hash, file_idx, sources, file_must_include }`, `External`, `Nzb`, `Archive { kind, urls }`, or `Unknown`. A stream with no playable source is kept as `Unknown`: the UI shows "requires plugin" instead of hiding it.
- Numbers that addons send as strings (`fileIdx: "3"`, `videoSize: "123"`) are accepted.

### Manifest validation
`Manifest::from_json` fails when `id`, `name` or `version` is missing or empty, or when no `resources` entry is usable. It returns warnings (not errors) when:
- a list item was dropped (bad resource, catalog, extra);
- a resource object or catalog uses a type not listed in `types`;
- a catalog uses the legacy `extraSupported`/`extraRequired` (names only in `extraRequired` are treated as supported);
- `idPrefixes` (manifest or resource) is an empty array (treated as "no ids match");
- there are `catalogs` but no `catalog` resource.

`transport_url` accepts `https://`, `http://` and `stremio://` (→ `https://`) URLs ending in `/manifest.json` (a query string is allowed); legacy `/stremio/v1` and `ipfs://` are `Unsupported`.

### Routes
`ResourcePath { resource, type, id, extra: Vec<(String, String)> }` → `{base}/{resource}/{type}/{id}.json` or `{base}/{resource}/{type}/{id}/{extra}.json`, where:
- `base` is the transport URL with the **trailing** `/manifest.json` removed; a query string on the transport URL is carried over after `.json`, as stremio-core does (some addons authenticate that way);
- `resource`, `type` and `id` are each encoded like JS `encodeURIComponent` (`tt1:1:2` → `tt1%3A1%3A2`), as stremio-core does;
- `extra` is `name=value` pairs, each side `encodeURIComponent`-encoded, joined with `&`, in the given order. Repeated names are allowed (multi-select genres).

### Router
`plan(addons, request) -> Vec<(addon index, ResourcePath)>`, in install order:
- `catalog`: one entry per manifest catalog matching the type, whose required extras are all present and whose extras are all supported (`catalog.is_extra_supported`).
- `meta`/`stream`/`subtitles`: addons whose resource list has the resource, whose types (resource-level if given, else manifest-level) include the type, and whose idPrefixes (same rule) match the id prefix. No `idPrefixes` ⇒ any id matches.
- `addon_catalog`: addons whose `addonCatalogs` contain the `(type, id)`.

`fetch_all(transport, plan)` returns a `Stream` of `(addon index, Result<Response>)` items **in completion order**: one slow addon never delays another.

### Caching (`cacheMaxAge` / `staleRevalidate` / `staleError`)
Values are seconds, read from the response body; if the body has none, `Cache-Control: max-age`, `stale-while-revalidate`, `stale-if-error` are used. For a cached entry of age `a`:

| Age | Result |
|---|---|
| `a < cacheMaxAge` | `Fresh`: serve cached, no request |
| `a < cacheMaxAge + staleRevalidate` | `Stale`: serve cached now, revalidate in the background |
| otherwise | `Expired`: fetch; if the fetch fails and `a < cacheMaxAge + staleError`, serve cached |

No hints ⇒ not cached. The cache key is the full resource URL. The clock is injected (`Clock` trait) so tests use a fake.

### Fixtures (D61, D62)
`fixtures/stremio/<addon>/` holds:
- `manifest.json`: the manifest response body;
- `<resource>/<type>/<id>[/<extra>].json`: response bodies, with every path segment percent-encoded for the filesystem (everything except `A-Z a-z 0-9 - _ .`, so `:` becomes `%3A`, which is legal on Windows);
- `index.json`: `{ "addon": <scrubbed transport URL>, "recorded": "YYYY-MM-DD", "requests": [{ "path", "status", "cacheControl", "file" }] }`.

Recordings are written by `stremio-cli record`, which scrubs secrets (T0.1.7 scrubber) **and** anonymizes stream and subtitle content before anything touches disk. Anonymizing keeps the shape and the technical tokens (resolution, codec, HDR, size, languages, emoji) and replaces work titles, release groups, info hashes and stream URLs with synthetic values. It's idempotent, and CI checks every committed fixture is already scrubbed and anonymized.

## Behaviour (known constraints)
- Movie without `videos` ⇒ video id = meta id.
- `videos[].streams` embedded in meta suppresses other stream addons for that video.
- Streams are delivered per addon as each answers; never wait for all.
- Unplayable stream kinds show as "requires plugin X", not hidden.
- AIOStreams: trust its order by default (see [stream-ranking](stream-ranking.md)).
- Secrets: the installed URL is a secret; UI and exports show a redacted form.
- A non-2xx status or a body that isn't JSON is an error for that addon only.

## Conformance tests
- Every committed fixture parses with **zero dropped items** and its manifest validates without errors (`cargo test -p app-addons-stremio`).
- Legacy manifest forms (`extraSupported/extraRequired`, resource strings vs objects): unit tests on synthetic manifests.
- Route encoding against stremio-core's `encodeURIComponent` set.
- Cache table above with a fake clock.
- Router selection by type / idPrefix / resource-object overrides.

### Conformance addons (D63)
| Addon | Why | Recorded by |
|---|---|---|
| AIOStreams (owner's config) | Target #1, configurable, pre-sorted debrid streams | owner (`record`, local) |
| Comet (owner's config) | Debrid scraper, 403 without config | owner |
| MediaFusion (owner's config) | Scraper + live TV, empty without config | owner |
| Cinemeta | Official catalogs/meta, `addon_catalog` | CI-safe, recorded in S3 |
| OpenSubtitles v3 | Subtitles | S3 |
| Anime Kitsu | `kitsu:` ids, anime meta | S3 |
| TMDB (tmdb.elfhosted.com) | `tmdb:` ids, genre/skip extras | S3 |
| M3U/EPG TV Addon (iptv-org config) | `tv` type, base64 config in path | S3 |
| Torrentio (unconfigured) | `infoHash` streams | S3 |
| WatchHub | `externalUrl` streams | S3 |

## Open items
- Grow to ~20 addons in M1.2.
- EPG (`catalog/tv/{id}/date=…` → `metasDetailed`): no reachable legal addon with `epgProvider` found during S3; synthetic fixture until one is.
- Stremio datastore API shapes for import (undocumented; capture during T1.2.9).
