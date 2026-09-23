#!/usr/bin/env bash
set -euo pipefail

rm -rf dist
wasm-pack build --target nodejs --no-typescript --release -d dist/node --out-name wasm-sha1-node
wasm-pack build --target no-modules --no-typescript --release -d dist/web --out-name wasm-sha1

# wasm-pack writes a "*" gitignore into each package. Drop those so this repo can
# keep tracking the generated artifacts.
rm -f dist/node/.gitignore dist/web/.gitignore

# Current no-modules glue keeps wasm_bindgen in script scope. Publish it on the
# global object so existing WebGME script tags can still call wasm_bindgen.hash.
printf '\nglobalThis.wasm_bindgen = wasm_bindgen;\n' >> dist/web/wasm-sha1.js
