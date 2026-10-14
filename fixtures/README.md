# Fixtures

Recorded responses from real addons, used by conformance tests (Stremio v1 from Phase 1, D15). The first recordings come from spike S3 (T0.S3.7).

| Folder | Contents |
|---|---|
| `stremio/<addon>/` | Responses from one Stremio addon: manifest, catalogs, metas, streams, subtitles |
| `samples/` | Synthetic examples that exercise the scrubber's check in CI |

Layout inside an addon folder (`manifest.json`, `<resource>/<type>/<id>[/<extra>].json`, `index.json`) is in the [stremio-client spec](../docs/specs/stremio-client.md#fixtures-d61-d62). Every recording is checked by `cargo test -p app-addons-stremio --test conformance`.

## Recording an addon

Use `stremio-cli record`: it trims, scrubs and anonymizes **before** writing anything, re-checks what it wrote (contents and file names), and only then replaces the addon's folder; a failed run leaves the old recording alone (D61, D62).

```sh
# Public addon
cargo run -p app-stremio-cli -- record --name cinemeta https://v3-cinemeta.strem.io/manifest.json

# Addon whose URL holds your debrid key: keep the URL in a file outside the repo (or in
# STREMIO_ADDONS) and pass the raw key in FIXTURE_SECRETS so it's caught even where the
# heuristics can't recognise it.
FIXTURE_SECRETS="your-torbox-key"   cargo run -p app-stremio-cli -- record --name aiostreams --addons ~/aiostreams-url.txt
```

PowerShell:

```powershell
$env:FIXTURE_SECRETS = "your-torbox-key"
cargo run -p app-stremio-cli -- record --name aiostreams --addons $HOMEiostreams-url.txt
Remove-Item Env:FIXTURE_SECRETS
```

Without explicit paths it picks catalogs, metas, streams and subtitles from the manifest. To record specific requests, add them after `--`: `-- stream/movie/tt1375666 catalog/movie/top/skip=100`. Re-running replaces the addon's folder once the new recording has passed its checks.

## No secrets, ever (D57), no release names (D62)

The repo is public. Configurable addons (AIOStreams, Comet, Torrentio) put your debrid key **inside the addon URL path**, and debrid stream links carry tokens. Before committing a recording, run the scrubber:

```sh
# Keys from an env var so they don't end up in shell history files you might share
FIXTURE_SECRETS="your-torbox-key" \
  cargo run -p app-fixtures -- scrub --addon "<your addon manifest URL>" fixtures/stremio/aiostreams/*.json
```

It replaces, with `REDACTED`:
1. every `--secret` / `FIXTURE_SECRETS` value, anywhere, including percent-encoded (a string that still contains one after decoding is replaced whole);
2. every `--addon` base URL, which removes its config segment (`https://host/<config>/manifest.json` → `https://host/REDACTED/manifest.json`), in `https://`, `http://` and `stremio://` forms;
3. values (strings or numbers) under secret-looking keys (`apiKey`, `token`, `Authorization`, `torbox`, …), `user:pass@` in URLs, secret-looking query parameters, token-like URL path segments (long letter+digit strings, UUIDs, base64 configs), the path segment after a debrid service name (`/resolve/torbox/<key>/`), and all of the above inside URL-valued query parameters (proxies like MediaFlow's `?d=<url>`).

It also anonymizes stream and subtitle content: technical tokens stay, other words become `Title`, hashes become counters and stream URLs point at `stream.invalid` (rules in [`anonymize.rs`](../crates/fixtures/src/anonymize.rs)).

CI runs `app-fixtures check fixtures`, which fails if scrubbing or anonymizing would change anything. Run it locally with `FIXTURE_SECRETS` set to your real keys before pushing, which also catches keys the heuristics can't recognise.

The heuristics also redact some harmless values (e.g. info hashes inside URLs). That's intended: fixtures need the response shape, not those values.
