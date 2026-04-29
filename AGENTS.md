# tauri-plugin-holochain — Agent Instructions

## Purpose

Tauri 2 plugin that embeds a Holochain conductor (`holochain_runtime`
crate) into a desktop / Android app. Provides Tauri commands for
launching the runtime, installing apps, and exposing
`AppWebsocket`s to the JS side. Used by
[`unyt-sandbox/unyt`](../unyt-sandbox/unyt/) (and other downstream
Tauri apps) to ship Holochain inside an installable binary.

## Classification

`library` — consumed by Tauri apps; not itself a deployable.

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

## Related repos in workshop

- Consumed by [`unyt-sandbox/unyt`](../unyt-sandbox/unyt/) — the
  Unyt app embeds this plugin.
- Coordinates closely with [`ham`](../ham/) on Holochain client
  version pinning (both must be compatible with the same
  `holochain_client` rc).

## Changelog

File: [`./CHANGELOG.md`](./CHANGELOG.md). Format: [Keep a Changelog
1.1.0](https://keepachangelog.com/en/1.1.0/) with `## [Unreleased]`
at the top and standard subsections. One bullet per agent change,
≤120 chars, present-tense imperative. Branch-type → section mapping
per workshop
[`branch-and-pr-workflow.mdc`](../.cursor/rules/branch-and-pr-workflow.mdc).

This is a **library** consumed by Tauri apps via `git rev` pin or
npm version. Breaking changes to the public Tauri command surface,
the JS bindings API, or the `HolochainPluginConfig` schema MUST
appear under `### Changed` (or `### Removed`) and call out the
required consumer-side migration.

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

## Lessons learned

_Append entries here whenever an agent (or human) loses time to
something a guardrail would have prevented. Keep each entry: date,
short symptom, concrete fix._
