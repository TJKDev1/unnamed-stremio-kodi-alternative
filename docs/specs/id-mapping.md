# Spec: ID & episode mapping

**Status:** stub · **Implemented by:** M1.5 (T1.5.9, basic), M1.11, M5.5 · **Decisions:** D27, D40

## Purpose
Resolve between IMDb, TMDb, TVDB, Kitsu, MAL, AniList, AniDB and Simkl IDs, and between episode numbering schemes, so streams, trackers and local files all agree on "which episode is this". Design: [anime ID & episode mapping](../design/09-anime.md#1-anime-aware-id--episode-mapping-part-of-the-core-id-mapping-service).

## Scope
- **In:** cross-ID document, episode spine, absolute ↔ season/episode conversion, data sources, patch overlay, per-show offsets.
- **Out:** metadata itself (metadata slot).

## Contract (to fill in)
- `map(id, episode?) → { ids: {imdb, tmdb, tvdb, kitsu, mal, anilist, anidb, simkl}, episode: {absolute, season, number, anidbEpisode} }`.
- Edge cases modelled explicitly: episode 0, specials/OVAs, split cours, "Part 2", split movies, double episodes.
- Patch overlay file format.

## Behaviour (known constraints)
- Offline dumps cached locally and self-hostable: Anime-Lists XML, Fribb/anime-lists, manami; TMDb external IDs for non-anime.
- AniDB episode list is the anime episode spine.
- Single-slot built-in provider; replaceable.

## Conformance tests
- Golden set of tricky shows with expected mappings (to be collected).

## Open items
- Update cadence for dumps and where they're fetched from.
