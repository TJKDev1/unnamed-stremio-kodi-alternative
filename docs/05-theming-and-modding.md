# 05 — Theming & modding

Goal: **every part of the UI is replaceable**, there are easy on-ramps for non-coders, and nothing breaks every release (Kodi's biggest skin problem).

## Four tiers

| Tier | Who | What it can change | Format | Risk |
|---|---|---|---|---|
| 0. Settings | Everyone | Accent/colors, fonts, density, poster size and shape, corner radius, animation speed (or off), blur on/off, rail behavior | In-app settings → writes tokens | None |
| 1. Themes | Non-coders, designers | All design tokens + scoped CSS | `theme.toml` + optional `.css` | None (sanitized CSS) |
| 2. Skins | Kodi-style skinners | Layout and structure of any screen or component, per-mode variants | Declarative component templates | Low (no arbitrary code) |
| 3. Mods | Developers | New screens, widgets, buttons, commands, keybinds, data views | JS/TS or WASM, sandboxed, permissioned | Medium (sandbox + permissions) |

All four tiers stack: a mod can add a widget, a skin decides where it goes, a theme decides how it looks, and the user's settings override colors on top.

---

## Tier 0/1: Design tokens and themes

- **A single token system** drives the default skin and all well-behaved skins:
  - `color.*` (surface, on-surface, accent, focus-ring, rating, state colors…)
  - `type.*` (families, scale, weights)
  - `space.*`, `radius.*`, `elevation.*`, `motion.*` (durations, easings; `motion.scale = 0` disables)
  - `focus.*` (ring style, scale-on-focus, glow, sound)
  - `card.*` (poster ratio, title position, badge style)
- Tokens compile to CSS custom properties, so **live preview is instant** and there's no rebuild.
- Themes can supply **light/dark/OLED variants** and **per-mode overrides** (desktop vs TV, since TV needs larger type and stronger focus).
- **Dynamic color**: optional accent extracted from the focused item's artwork, computed in the core, not in JS.
- **In-app theme editor**: tweak tokens with live preview, then export/share as a file, a QR code or a link.
- **Scoped CSS** in themes is allowed but sanitized: no `@import` of remote URLs, no `url()` to external hosts unless permitted, no `position: fixed` overlays outside the theme's own slots.

```toml
# theme.toml
[meta]
id = "com.example.midnight"
name = "Midnight"
api = "1.x"            # semver of the theming API
modes = ["desktop", "tv"]

[tokens.color]
surface = "#0b0d12"
accent  = "#7c5cff"
focus-ring = "#ffffff"

[tokens.tv.type]
scale = 1.35            # bigger text in TV mode
```

---

## Tier 2: Skins

Skins replace **structure**, not just colors. This is like Kodi skins, without Kodi's problems.

| Kodi skins | Our skins |
|---|---|
| Magic window/control IDs | Named screens and **named slots** (`home.rows`, `detail.actions`, `player.osd.bottom`) |
| No state model, logic spread across XML + Python helpers | Templates bind to **typed view-models** (with TS types and JSON Schema) published by the core |
| Absolute 1080p coordinates | Responsive layout (flex/grid) + per-mode variants |
| Break every `xbmc.gui` bump | **Semver'd skin API**, compatibility shims for N−2 majors, a linter that flags deprecated usage |
| Must reimplement every window | **Partial skins**: override only the screens you care about; the rest fall back to the default skin |
| Focus declared per control (`onup`/`ondown`) | Automatic spatial focus, with optional overrides (`focus-group`, `focus-trap`, explicit neighbors) |

**Format (proposal):** single-file components with a restricted template syntax (HTML-like plus bindings, loops, conditionals, slots) and scoped CSS, **compiled to SolidJS** at install time. No `<script>`: skins are data. Logic belongs in mods, which keeps skins safe to share.

```html
<!-- skins/neon/home.row.skin -->
<template for="home.row" vm="CatalogRow">
  <section class="row" focus-group="row">
    <h2>{vm.title} <small if={vm.addonName}>· {vm.addonName}</small></h2>
    <rail virtualized items={vm.items} as="item">
      <slot name="card" item={item} />  <!-- uses the skin's or default card -->
    </rail>
  </section>
</template>
<style scoped>
  .row h2 { font: var(--type-row-title); color: var(--color-on-surface); }
</style>
```

- **Hot reload** while editing. Skin dev mode includes a focus debugger, slot inspector and per-component performance overlay.
- **Perf gate**: skins declare a target device class. A skin whose components blow the frame budget on TV gets a warning (and auto-fallback per component if the user wants).
- **Layout presets** ship as skins: "Rail + rows" (default), "Top tabs + hero", "Grid browser", "Minimal TV".

---

## Tier 3: Mods (code)

Mods are code, so they're sandboxed from day one (lesson from the Obsidian/BetterDiscord malware incidents).

- **Runtime:** a dedicated Web Worker (JS/TS) or WASM. **Never on the UI thread, never raw DOM access.**
- **What a mod can do** (via the `mod` API):
  - Register **components** into named slots (declarative templates, like skins, with data from the mod).
  - Add **screens/routes**, **settings pages**, **commands** (command palette + bindable to keys/buttons), **context-menu actions**, **player OSD widgets**.
  - Read view-models it has permission for; dispatch intents it has permission for.
  - Use network (domain allowlist), KV storage, notifications.
- **Permissions** are declared in the manifest and shown at install, e.g. "read your watch history", "control playback", "access api.trakt.tv".
- **Budgets:** per-mod CPU time per frame and memory cap. A mod that stalls gets suspended with a toast.
- **"Unsafe developer mode"** (opt-in, red banner): allows raw CSS/DOM injection for experiments, like Vencord's dev mode. It is never on by default and never via the catalog.

### Example mod ideas (to validate the API)
- A "Random pick" button on Home.
- Trakt/Letterboxd ratings badge on posters.
- A clock and weather widget for TV mode.
- A Jellyfin-style "Because you watched…" row.
- Custom stream formatters (how stream names render).

---

## Distribution

- **Community catalogs** for themes/skins/mods (same mechanism as addon catalogs). We host no official catalog (D46).
- Import by file, URL or QR code. One-click install from a website (deep link `app://install?…`).
- Per-profile themes/skins (the kids profile gets a different skin).
- **Safe mode**: holding a key/button at startup (or a crash loop detected) boots the default skin with mods disabled. Users can always recover.
