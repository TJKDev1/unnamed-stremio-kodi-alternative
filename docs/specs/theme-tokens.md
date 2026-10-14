# Spec: Theme tokens & theme files

**Status:** stub · **Implemented by:** M1.9 · **Decisions:** D6, D23, D49

## Purpose
The design-token system that drives the default skin and every well-behaved skin, and the `theme.toml` format. Design: [tier 0/1](../design/05-theming-and-modding.md#tier-01-design-tokens-and-themes).

## Scope
- **In:** token names and types, mode/variant overrides, `theme.toml` format, scoped CSS rules and the sanitiser, versioning.
- **Out:** skins (structure, [skin-format](skin-format.md)).

## Contract (to fill in)
- Token groups: `color.*`, `type.*`, `space.*`, `radius.*`, `elevation.*`, `motion.*`, `focus.*`, `card.*`, sounds.
- Variants: light / dark / OLED; per-mode overrides (desktop / tv).
- `theme.toml`: `[meta]` (id, name, `api` semver, modes) + `[tokens.*]`.
- Compiled output: CSS custom properties.

## Behaviour (known constraints)
- Live preview with no rebuild.
- Sanitiser: no remote `@import`, no external `url()` unless permitted, no fixed overlays outside the theme's slots.
- Blur auto-disables on low-end devices and in TV lite mode.
- Must work on Chromium 94.

## Conformance tests
- Every token used by the default skin is defined in the default theme.
- Sanitiser test corpus (allowed/blocked CSS).

## Open items
- Final token list (start from the default skin's needs).
