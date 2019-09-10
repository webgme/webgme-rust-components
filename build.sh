rm -r dist
wasm-pack build --target nodejs --no-typescript --release -d dist/node --out-name wasm-sha1-node
wasm-pack build --target no-modules --release --no-typescript -d dist/web --out-name wasm-sha1
