#!/usr/bin/env bash
set -euo pipefail

root=${1:-dist}

rm -rf "$root"
wasm-pack build --target nodejs --no-typescript --release -d "$root/node" --out-name wasm-sha1-node
wasm-pack build --target no-modules --no-typescript --release -d "$root/web" --out-name wasm-sha1

# wasm-pack writes a "*" gitignore into each package. Drop those so this repo can
# keep tracking the generated artifacts.
rm -f "$root/node/.gitignore" "$root/web/.gitignore"

# Current no-modules glue keeps wasm_bindgen in script scope. Publish it on the
# global object so existing WebGME script tags can still call wasm_bindgen.hash.
printf '\nglobalThis.wasm_bindgen = wasm_bindgen;\n' >> "$root/web/wasm-sha1.js"
