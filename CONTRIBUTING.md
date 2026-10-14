# Contributing

The project is built by a solo developer with AI help (D16), so the process is written down so that people and AI agents can follow the same rules.

## Where things are

| Path | What |
|---|---|
| [`docs/design/`](docs/design/) | The *why*: vision, architecture, [decision log](docs/design/01-vision.md#decision-log) (D#), [open questions](docs/design/08-open-questions.md) |
| [`docs/plan/`](docs/plan/README.md) | The *what and when*: phases → milestones → tasks, with exit criteria |
| [`docs/specs/`](docs/specs/README.md) | The *exact contracts* code is written against |
| [`docs/spikes/`](docs/spikes/README.md) | Spike write-ups (Phase 0) |
| `crates/` | Rust workspace (packages are named `app-*` until the app has a name) |
| `ui/` | pnpm workspace: app shell, default skin, SDK |
| `fixtures/` | Recorded addon responses for conformance tests (secrets scrubbed) |
| `xtask/` | Repo maintenance tasks: `cargo xtask <task>` |

## Workflow

1. **Pick a task** from the [plan](docs/plan/README.md). Every piece of work has a task ID (`T1.3.2`, spikes `T0.S3.2`).
2. **Spec first.** If the task implements a spec that is still a `stub`, fill in the contract first, in its own commit, and get it agreed before writing code.
3. **One branch + PR per milestone or spike** (e.g. `m0.1-repo-bootstrap`, `s3-stremio-client`), **one commit per task** inside it (D58).
4. **Commit messages start with the task ID**: `T1.3.2: mpv under webview on Windows`. Work that isn't a task (typo fixes, CI tweaks) says so plainly.
5. **Review.** Each PR is reviewed by GPT-6.1-Sol (D58). It merges once review findings are resolved and CI is green.
6. **Tick the box** in the plan and update the [status table](docs/plan/README.md#status) when a milestone closes.

## Definition of done (every task)

1. Code merged to `main`, behind no flags unless the task says so.
2. Tests for the behaviour: unit tests for logic, conformance tests for protocols, perf tests where a [budget](docs/design/03-architecture.md#performance-budget) applies.
3. If it implements a spec: the spec is updated to match what was actually built, and its status bumped.
4. If it made a decision: logged as a D# in the [decision log](docs/design/01-vision.md#decision-log) and removed from [open questions](docs/design/08-open-questions.md).

## Local setup

Requirements: Rust (the version in `rust-toolchain.toml` installs automatically via rustup), Node (version in `.node-version`), pnpm (version in `package.json` → `packageManager`).

```sh
pnpm install

# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-licenses

# UI
pnpm typecheck
pnpm lint
pnpm -r run test
pnpm build

# Fixtures
cargo run -p app-fixtures -- check fixtures
```

CI runs all of the above on Windows, macOS and Linux.

## Rules that CI enforces

- **Formatting and lints:** rustfmt; clippy with warnings as errors; ESLint (typescript-eslint strict).
- **Chromium 94 baseline (D14):** UI code is ES2021, and `eslint-plugin-compat` rejects web APIs that Chrome 94 lacks (the LG C3's browser engine). Don't raise the baseline without a decision.
- **Licenses (D37, D59):** the app is GPL-3.0-or-later, server/sync crates AGPL-3.0-or-later, SDKs MIT. `cargo xtask check-licenses` checks every crate and `package.json`; the policy lives in `xtask/src/licenses.rs`.
- **No secrets in fixtures:** recorded addon responses go through the scrubber, and CI rejects fixtures that still look like they contain keys. See [`fixtures/README.md`](fixtures/README.md).
- **No telemetry (D38).** Crash reports are opt-in only.

## Licensing of contributions

Contributions are accepted under the license of the part of the repo they touch (see above). By opening a PR you agree to license your contribution that way.
