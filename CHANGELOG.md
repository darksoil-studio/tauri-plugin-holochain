# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- upgrade Holochain ecosystem to 0.6.1 stable; kitsune2 to 0.4.1; lair-keystore-api to 0.6.3
- Refactored the `tauri-plugin-holochain` crate to extract the `HolochainRuntime` functionality as the `holochain_runtime` crate.
- Gossip arc clamp is not a setting part of `HolochainPluginConfig`.
- Added optional `hc-auth` feature for authenticated bootstrap/relay network support with conductor restart capability.
