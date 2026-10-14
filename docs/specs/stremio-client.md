# Spec: Stremio addon client (protocol v1)

**Status:** stub · **Implemented by:** S3, M1.2, M1.3 · **Decisions:** D15, D44, D48

## Purpose
100% compatible client for the Stremio addon protocol. Design: [Stremio addon protocol](../design/04-addons.md#stremio-addon-protocol-must-be-100-compatible). stremio-core is a reference for behaviour and test cases only (D44).

## Scope
- **In:** manifest, all resources (`catalog`, `meta`, `stream`, `subtitles`, `addon_catalog`), extras, IDs, caching headers, configure flow, account import, EPG.
- **Out:** `ipfs://`/`ipns://` transport (low priority); playing `infoHash`/`nzbUrl`/archive streams (plugins).

## Contract (to fill in)
- Normalized models: `Manifest`, `MetaPreview`, `Meta`, `Video`, `Stream`, `Subtitle`; unknown fields preserved.
- Router: choose addons by resource, `types` and `idPrefixes`; per-addon priority for catalogs/meta/streams separately.
- Routes and extra encoding (`/{resource}/{type}/{id}/{extra}.json`).
- Secrets: the installed URL is a secret; UI and exports show a redacted form.

## Behaviour (known constraints)
- Movie without `videos` ⇒ video id = meta id.
- `videos[].streams` embedded in meta suppresses other stream addons for that video.
- Streams are delivered per addon as each answers; never wait for all.
- Unplayable stream kinds show as "requires plugin X", not hidden.
- AIOStreams: trust its order by default (see [stream-ranking](stream-ranking.md)).

## Conformance tests
- Recorded fixtures from ~20 addons (AIOStreams first, Cinemeta, OpenSubtitles, Torrentio-style, MediaFusion, Comet, TMDB, Anime Kitsu, IPTV/EPG); secrets scrubbed.
- Legacy manifest forms (`extraSupported/extraRequired`, resource strings vs objects).
- Cache header behaviour with a fake clock.

## Open items
- Exact list of the ~20 conformance addons.
- Stremio datastore API shapes for import (undocumented; capture during T1.2.9).
