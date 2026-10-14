# Docs

Status: **planning**, last updated 2026-10-14. The app is called **"the app"** throughout until we pick a name.

The docs are split into three parts:

| Folder | What it is | Changes when… |
|---|---|---|
| [`design/`](design/) | The *why*: vision, research, architecture, decision log (D#), open questions | A decision is made or revisited |
| [`plan/`](plan/README.md) | The *what and when*: phases → milestones → tasks, with exit criteria and checkboxes | Work starts, finishes or is re-scoped |
| [`specs/`](specs/README.md) | The *exact contracts* code is written against: interfaces, schemas, conformance tests | Before a task that implements it starts; after a spike changes it |

Rule of thumb: **design says why, plan says what's next, specs say exactly how.** A task links to the spec it implements; a spec links to the decisions behind it.

## Design docs

| # | Doc | What's in it |
|---|-----|--------------|
| 01 | [Vision & principles](design/01-vision.md) | What we're building, for whom, core principles, **decision log** |
| 02 | [Competitive research](design/02-competitive-research.md) | Stremio, Kodi, and other apps: flaws, strengths, and how we respond |
| 03 | [Architecture](design/03-architecture.md) | Rust core, web UI, player layer, home server, performance budget, platforms |
| 04 | [Addon systems](design/04-addons.md) | Stremio compatibility, Kodi Python shim, our native protocol (HTTP + WASM) |
| 05 | [Theming & modding](design/05-theming-and-modding.md) | The four tiers: settings → themes → skins → mods |
| 06 | [UX, input & player](design/06-ux-input-player.md) | 10-foot UI, focus engine, remapping, stream ranking, catalogs, player |
| 07 | [Roadmap](design/07-roadmap.md) | High-level phase overview (the detailed version is [`plan/`](plan/README.md)) |
| 08 | [Open questions](design/08-open-questions.md) | Everything still undecided |
| 09 | [Anime](design/09-anime.md) | ID/episode mapping, anime ranking, trackers, Seanime lessons |
| 10 | [Music](design/10-music.md) | Later-phase music support |
