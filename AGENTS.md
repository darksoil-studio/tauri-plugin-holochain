# tauri-plugin-holochain — Agent Instructions

> **This repo follows the workshop root's patterns — it does not define its own.** Development workflow, process, changelog conventions, and spec/feature-doc discipline live in the workshop: [`CLAUDE.md`](../CLAUDE.md), [`AGENTS.md`](../AGENTS.md), [`documentation/DEVELOPMENT_WORKFLOW.md`](../documentation/DEVELOPMENT_WORKFLOW.md). Below is only what's specific to THIS repo.

## Purpose

`library` — Tauri 2 plugin that embeds a Holochain conductor (`holochain_runtime` crate) into a desktop / Android app, providing Tauri commands for launching the runtime, installing apps, and exposing `AppWebsocket`s to the JS side. Consumed by [`unyt-sandbox/unyt`](../unyt-sandbox/unyt/) and other downstream Tauri apps; not itself a deployable.

## Stack

- **Rust workspace** at root ([`Cargo.toml`](Cargo.toml),
  [`crates/`](crates/)): the `tauri-plugin-holochain` crate plus the
  extracted `holochain_runtime` crate.
- **JavaScript** API package — see [`package.json`](package.json)
  and [`src/`](src/) (the JS bindings consumed by Tauri apps via
  npm).
- Optional `hc-auth` feature for authenticated bootstrap / relay
  (see existing CHANGELOG entry).
- **Requires `nix develop -c …`** — see
  [`flake.nix`](flake.nix). The workshop's
  [Nix discipline section](../AGENTS.md#nix-discipline) lists this
  repo.

## Build

```bash
nix develop -c cargo build --release         # Rust workspace
npm install                                  # JS bindings
npm run start                                # JS dev (if working on bindings)
```

## Format

Apply, then verify, both Rust and JS:

```bash
nix develop -c cargo fmt
nix develop -c cargo fmt --check
npx prettier --write "src/**/*.{ts,js,json}"
npx prettier --check "src/**/*.{ts,js,json}"
```

If a `format` / `format:check` script is later wired into
`package.json`, prefer the script over `npx`.

## Test

```bash
nix develop -c cargo test                    # Rust
```

Rust tests cover the runtime / plugin surface. JS bindings are
exercised in downstream apps' integration tests
([`unyt-sandbox/unyt`](../unyt-sandbox/unyt/) is the canonical
consumer).

## Deploy

n/a — library. Consumers depend either via `git` rev pin in
`Cargo.toml` or via npm package version.

## Repo-specific rules

- **`HolochainPluginConfig` is a public contract** for downstream
  apps. Renames or removed fields are breaking; go through deprecate
  → remove across two minor versions when possible.
- **`hc-auth` feature flag** gates authenticated bootstrap / relay.
  Don't make features that depend on it always-on; downstream apps
  opt in via the feature flag.
- **Holochain runtime version bumps** must be coordinated with
  [`ham`](../ham/) and the canonical consumer
  [`unyt-sandbox/unyt`](../unyt-sandbox/unyt/) — open a coordinated
  plan before bumping.
