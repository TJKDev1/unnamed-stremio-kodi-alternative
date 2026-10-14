# Spec: Kernel & modules

**Status:** stub · **Implemented by:** M1.1 (T1.1.1–T1.1.5, T1.1.12) · **Decisions:** D25, D41, D42

## Purpose
Defines the small kernel and the module model that every feature (built-in or third-party) is written against. Design: [kernel + built-in addons](../design/03-architecture.md#kernel--built-in-addons--the-app-d25).

## Scope
- **In:** module manifest, lifecycle, dependency resolution, slot registration, event bus, permission model, trust tiers, Cargo-feature mapping.
- **Out:** individual slot contracts (see [slots](slots.md)), storage details (see [storage](storage.md)), WASM sandbox internals (`wasm-host`, Phase 4).

## Contract (to fill in)
- **Manifest fields:** id, version, provides (slots), consumes (slots), dependencies, permissions, settings schema, trust tier, Cargo feature name.
- **Lifecycle:** `registered → resolved → started (lazy, on first use) → stopped`; `disabled` = never initialised (zero footprint, D41).
- **Event bus:** typed events ("tell"); delivery guarantees TBD (at-most-once in-process?).
- **Slot calls:** "ask"; modules never call each other directly.
- **Permissions:** names and how they're granted (built-ins: implicit; third-party: at install).

## Behaviour (known constraints)
- Kernel API small enough to read in an afternoon; if something can be a module, it is one.
- Required slots (UI shell/skin, player backend, storage) must always have a provider; disabling the last provider is refused with a message.
- A module whose dependency is disabled is disabled too, with a clear message.
- Built-ins import only the public SDK; CI enforces it.
- Lazy start: only what the home screen needs loads at startup.

## Conformance tests
- Disabled module: no allocations, tasks, network or UI traces after startup.
- Dependency cascade: disabling a dependency disables dependents with the right message.
- Required-slot guard.
- Build with `--no-default-features` + a feature subset compiles and runs.

## Open items
- IDL/codegen tool choice (T1.1.2).
- Event delivery semantics and back-pressure.
- How module versions map to interface semver.
