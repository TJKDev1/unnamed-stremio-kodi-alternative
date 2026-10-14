# Spec: Focus & input

**Status:** stub · **Implemented by:** S2, M1.4, M2.3 · **Decisions:** D12, D36

## Purpose
One focus engine and one command/binding system for every input device. Design: [two input modes](../design/06-ux-input-player.md#two-input-modes-both-first-class).

## Scope
- **In:** focus model (groups, memory, traps, overrides), command registry, binding format, device profiles and glyphs, mode switching.
- **Out:** platform key-code mapping details (per-platform adapters).

## Contract (to fill in)
- Focus attributes for skins: `focus-group`, `focus-trap`, explicit neighbours, default focus.
- Command: `{ id, title, context, default bindings }`; mods can register commands (Phase 4).
- Binding: input (key / button / axis / remote key) + modifiers (chord, long-press, hold-repeat) → command, per context.
- Keymap file format for import/export.

## Behaviour (known constraints)
- Focus changes are synchronous and local; never wait on data; placeholders are focusable.
- Focus memory per group; restore on Back.
- Hold-to-repeat with acceleration.
- Focus move < 1 frame on desktop, < 2 frames on the LG C3.

## Conformance tests
- Spatial navigation golden tests over fixed layouts.
- Perf test for focus move.

## Open items
- Build on Norigin Spatial Navigation / BBC LRUD, or write our own (S2 decides).
