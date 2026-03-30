#!/bin/bash

set -euo pipefail

cargo install maturin --locked
cargo install wasm-pack --locked
cargo install just
rustup component add rustfmt
rustup component add clippy
cd presentation/deck
npm ci
cd ../..
cd web/jsoncompatdotcom
pnpm i
just check
