# Spec: Skin format

**Status:** stub · **Implemented by:** S7, M4.7 · **Decisions:** D6, D45

## Purpose
The `.skin` single-file component format: HTML-like templates bound to view-models, with no scripts, compiled to the UI framework at install time. Design: [tier 2 skins](../design/05-theming-and-modding.md#tier-2-skins).

## Scope
- **In:** template grammar, bindings, control flow, slots, focus attributes, scoped styles, named screens/slots, API versioning, perf gate metadata.
- **Out:** tokens ([theme-tokens](theme-tokens.md)); mod code (`mod-api`).

## Contract (to fill in after S7)
- `<template for="<slot>" vm="<ViewModel>">` + restricted syntax (`{expr}`, `if`, loops, `<slot>`, `virtualized` rails).
- `<style scoped>`.
- Skin manifest: id, `api` semver, target device class, minimum engine (e.g. `webos>=23`).

## Behaviour (known constraints)
- No `<script>`; skins are data.
- Partial skins fall back to the default skin per slot.
- Compiler down-levels or rejects CSS beyond Chromium 94 when targeting `webos>=23`.
- Compatibility shims for N−2 skin API majors; linter flags deprecated usage.

## Conformance tests
- Compiler golden tests; rejected-feature tests; slot fallback test.

## Open items
- Expression language limits (pure, no side effects; which operators?).
