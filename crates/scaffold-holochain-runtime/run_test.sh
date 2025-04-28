#!/usr/bin/bash
set -e

DIR=$(pwd)

rm -rf /tmp/test-scaffold-holochain-runtime

nix run --accept-flake-config .#scaffold-holochain-runtime -- --name test-scaffold-holochain-runtime --path /tmp --bundle-identifier org.myorg.testscaffoldholochainruntime
cd /tmp/test-scaffold-holochain-runtime

nix flake update --override-input p2p-shipyard $DIR
nix develop --override-input p2p-shipyard $DIR --command bash -c "
set -e
npm i
npm run tauri icon $DIR/examples/end-user-happ/src-tauri/icons/icon.png
cd src-tauri
cargo add -p test-scaffold-holochain-runtime --path $DIR/crates/tauri-plugin-holochain
cd ..
npm run tauri build -- --no-bundle
"

nix develop --override-input p2p-shipyard $DIR .#androidDev --command bash -c "
set -e

npm i
npm run tauri android init -- --skip-targets-install
npm run tauri android build -- --target aarch64
"
