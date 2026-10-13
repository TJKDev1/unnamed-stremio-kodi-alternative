# 09 — Anime support

Owner watches anime "some" but wants **good** anime support (D27). Main reference: [Seanime](https://github.com/5rahim/seanime) (GPL-3.0, Go server + React UI + Electron/libmpv desktop, AniList-centric), researched 2026-10-13.

## Why anime needs special handling
Stremio/Cinemeta's `tt…:S:E` model can't represent anime properly:
- **Cours and split seasons**: AniList/AniDB split by cour or part; TVDB/TMDB group everything under one series with seasons.
- **Absolute numbering**: release files say "ep 38" where TMDB says S3E1.
- **Specials, OVAs, ONAs, movies, recaps, episode 0**, "Part 2" continuations, movies split into parts.
- **Many ID systems**: AniList, MAL, Kitsu, AniDB, Simkl, TVDB, TMDB, IMDb.
- **Messy release names** (fansub groups, batches, dual audio, multi-subs).

## Design

### 1. Anime-aware ID & episode mapping (part of the core ID-mapping service)
- **Canonical anime entry** with a cross-ID document (AniList, MAL, Kitsu, AniDB, Simkl, TVDB, TMDB, IMDb).
- **AniDB episode list is the episode spine.** TVDB/TMDB season/episode and absolute numbers map onto it.
- Data sources (all **offline dumps, cached locally, self-hostable**; no hard dependency on one live API):
  - [Anime-Lists XML](https://github.com/Anime-Lists/anime-lists) for AniDB↔TVDB/TMDB episode offsets (range mappings, specials, double episodes).
  - Fribb/anime-lists and manami anime-offline-database for the ID crosswalk.
  - Seanime's MIT [build-your-own-animap](https://github.com/5rahim/build-your-own-animap) pipeline as a reference, or to reuse.
- A **patch overlay** (community-editable corrections file) fixes bad mappings without code changes.
- **Absolute ↔ relative conversion** through the relation tree (per-entry absolute ranges ordered by air date), with a **per-show manual offset** as an escape hatch.
- Edge cases (episode 0, split movies, "Part 2" numbering) are **modelled explicitly** in the mapping schema, instead of hard-coded hacks like Seanime has.
- Like everything else, the mapper is a **built-in, replaceable provider** (see [04](04-addons.md#built-in-but-replaceable-everything-is-a-provider-d25)).

### 2. Stremio-addon anime content
- Full support for `kitsu:` IDs (Anime Kitsu addon) and other anime ID prefixes; the core translates between ID systems so that streams found under `kitsu:ID:ep` and `tt:S:E` show up for the same episode.
- AIOStreams/Comet anime results run through the same ranking engine.

### 3. Trackers
- AniList, MAL, Kitsu and Simkl (which has anime-specific lists) as **tracker addons** on the shared tracker API ([04](04-addons.md#tracker--scrobbler-addons)). Several can run at once: e.g. Simkl for everything + AniList for anime.
- Seanime breaks without AniList; **we never require any tracker**.

### 4. Anime-specific ranking fields
The ranking engine ([06](06-ux-input-player.md#stream-ranking-engine)) gets anime fields that its presets can use, as Seanime's release profiles do:
- release group (preferred/avoided lists), batch vs single episode, **dual audio**, **multi-subs**, subtitle type (softsub/hardsub), source (BD/WEB), version (v2 repacks).
- "Cached wins only within a quality band" (Seanime promotes a cached release only if it scores ≥ 70% of the best).
- Per-content-type presets: e.g. an **"Anime" preset** (JP audio, EN subs, preferred groups) that applies automatically to anime entries.

### 5. Language rules per content type
- Audio/subtitle preferences can differ by content type: anime → Japanese audio + subtitles; other content → original audio, subtitles off or forced only. Fully configurable.

### 6. Playback niceties
- **OP/ED skip** via the `segments` resource. An anime segments provider (e.g. AniSkip-style data keyed by MAL ID + episode) can be an addon; chapter-name patterns ("Opening", "OP", "Ending") as fallback, like Seanime v3.10.
- **Filler badges** (filler / mixed / canon), with an optional "skip fillers" in binge mode. Data comes from a provider addon.
- High-fidelity **ASS subtitles** (libmpv), font attachments, typesetting.
- NC/OP/ED/PV files in local libraries are tagged as extras, not episodes.

### 7. Local anime files
- Anitomy-style filename parser (study `seanime-parser` / `tanuki`), shared with the release-name parser in the ranking engine.
- **Scored, explainable matching** (title, season, part, format, year) with a minimum score, user regex rules, force-match, and a scan log. Unmatched → "Needs attention" list.

### 8. UI
- Airing schedule in the calendar (by tracker or metadata provider), "new episode" notifications.
- Seasons shown as the user prefers: **AniList-style (each cour is its own entry)** or **TMDB-style (one show, multiple seasons)**, with a toggle per profile.

## Lessons from Seanime: what to avoid
- Hard dependency on one tracker or metadata API (AniList rate limits, account required).
- A single-maintainer, obfuscated mapping endpoint as a dependency. Use open dumps plus our patch overlay.
- Extension permissions that are declared but not enforced for content providers.
- Missing multi-user/profiles, reverse-proxy base URL, thin-client mode. We plan these from the start.
