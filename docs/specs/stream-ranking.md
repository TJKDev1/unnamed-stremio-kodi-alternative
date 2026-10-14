# Spec: Stream ranking

**Status:** stub · **Implemented by:** M1.3 (trust-order only), M1.6, M1.11 · **Decisions:** D12, D15, D24, D34

## Purpose
The built-in ranker: turns streams from all addons into one ordered, explained list, and decides what Play does. Design: [stream ranking engine](../design/06-ux-input-player.md#stream-ranking-engine-must-have).

## Scope
- **In:** release-name parser output, rule/preset format, pipeline stages, trust-order mode, device-aware rules, explanations, Smart Play decision.
- **Out:** finding streams (addons only, D24).

## Contract (to fill in)
- `rank(streams, context) → { ordered, explanations }`, where context = profile, device capabilities, content type, per-addon settings.
- Parsed stream fields: resolution, HDR/DV (+ profile), codec, audio (codec, channels, Atmos), languages, size, release group, source (BD/WEB/…), cached/debrid, seeders; anime fields (dual audio, multi-subs, softsub/hardsub, version).
- Preset file (TOML): filters, sort keys in priority order, per-addon caps, trust-order addons.
- Smart Play: `decide(ranked, preset) → autoplay(stream) | showList(focus)`.

## Behaviour (known constraints)
- Pipeline: collect → parse → filter → dedupe → sort → format; progressive (re-rank as streams arrive without jumping focus).
- Trust-order addons keep their order; only device-aware rules apply on top unless the user says otherwise.
- v2 structured fields win over parsed text when present (Phase 4).

## Conformance tests
- Parser fixture corpus from real AIOStreams/Comet/anime names.
- Golden tests: preset + stream set → expected order and explanations.

## Open items
- Rule language: plain TOML keys, or a small expression language (compatible with AIOStreams' SEL?).
