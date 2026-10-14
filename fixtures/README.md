# Fixtures

Recorded responses from real addons, used by conformance tests (Stremio v1 from Phase 1, D15). The first recordings come from spike S3 (T0.S3.7).

| Folder | Contents |
|---|---|
| `stremio/<addon>/` | Responses from one Stremio addon: manifest, catalogs, metas, streams, subtitles |
| `samples/` | Synthetic examples that exercise the scrubber's check in CI |

How recordings are named and laid out inside an addon folder is decided in S3 and written into the [stremio-client spec](../docs/specs/stremio-client.md).

## No secrets, ever (D57)

The repo is public. Configurable addons (AIOStreams, Comet, Torrentio) put your debrid key **inside the addon URL path**, and debrid stream links carry tokens. Before committing a recording, run the scrubber:

```sh
# Keys from an env var so they don't end up in shell history files you might share
FIXTURE_SECRETS="your-torbox-key" \
  cargo run -p app-fixtures -- scrub --addon "<your addon manifest URL>" fixtures/stremio/aiostreams/*.json
```

It replaces, with `REDACTED`:
1. every `--secret` / `FIXTURE_SECRETS` value, anywhere;
2. every `--addon` base URL, which removes its config segment (`https://host/<config>/manifest.json` → `https://host/REDACTED/manifest.json`);
3. values under secret-looking keys (`apiKey`, `token`, `Authorization`, `torbox`, …), `user:pass@` in URLs, secret-looking query parameters, and token-like URL path segments (long letter+digit strings, UUIDs, base64 configs).

CI runs `app-fixtures check fixtures`, which fails if scrubbing would change anything. Run it locally with `FIXTURE_SECRETS` set to your real keys before pushing, which also catches keys the heuristics can't recognise.

The heuristics also redact some harmless values (e.g. info hashes inside URLs). That's intended: fixtures need the response shape, not those values.
