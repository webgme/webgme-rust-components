# webgme-rust

Rust components compiled to WebAssembly for use with WebGME.

The published artifacts live in `dist/`: `dist/web` for browsers and `dist/node` for Node.js.
Rebuild them when the Rust sources change. Browser support follows whatever `wasm-bindgen` emits;
check the generated glue after a toolchain bump.

## Environment

- Rustup, on the stable channel. `rust-toolchain.toml` selects stable and the `wasm32-unknown-unknown` target.
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install wasm-pack
```

`wasm32-unknown-unknown` is a stable target. Nightly and `wasm-gc` are not required; wasm-pack runs the bindgen post-processing itself.

## Build

```bash
./build.sh
```

or `npm run build`.

## Usage

Depend on the files under `dist` rather than building Rust in the consuming project.

- Browser (`dist/web`): a classic script. After load, `wasm_bindgen` is on the global object.

```javascript
wasm_bindgen({ module_or_path: "./wasm-sha1_bg.wasm" }).then(function () {
  wasm_bindgen.hash("hello");
});
```

- Node (`dist/node`): `require` the package and call `hash(data)`. The loader reads the `.wasm` file next to the script; there is no separate `_bg.js`.

`hash` returns the SHA-1 digest of the UTF-8 input as a lowercase hex string.
