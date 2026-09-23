const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const vm = require("node:vm");

// Locks the interactions WebGME already uses. New exports may be added.
// These calls must keep returning the same SHA-1 hex digests.
const VECTORS = [
  ["", "da39a3ee5e6b4b0d3255bfef95601890afd80709"],
  ["hello", "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"],
  ["abc", "a9993e364706816aba3e25717850c26c9cd0d89d"],
  ["café", "f424452a9673918c6f09b0cdd35b20be8e6ae7d7"],
  [
    "The quick brown fox jumps over the lazy dog",
    "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12",
  ],
];

const distRoot = process.env.COMPAT_DIST
  ? path.resolve(process.env.COMPAT_DIST)
  : path.join(__dirname, "..", "dist");

function assertVectors(hash) {
  assert.equal(typeof hash, "function");
  for (const [input, expected] of VECTORS) {
    assert.equal(hash(input), expected, JSON.stringify(input));
    assert.equal(hash(input), expected);
  }
}

test("node package keeps require(...).hash(string)", () => {
  const nodeDir = path.join(distRoot, "node");
  const pkg = JSON.parse(fs.readFileSync(path.join(nodeDir, "package.json"), "utf8"));
  assert.equal(pkg.main, "wasm-sha1-node.js");
  for (const file of pkg.files) {
    assert.ok(fs.existsSync(path.join(nodeDir, file)), file);
  }

  const loaded = require(path.join(nodeDir, pkg.main));
  assertVectors(loaded.hash);
});

test("browser script keeps global wasm_bindgen.hash after init", async () => {
  const webDir = path.join(distRoot, "web");
  const pkg = JSON.parse(fs.readFileSync(path.join(webDir, "package.json"), "utf8"));
  assert.equal(pkg.browser, "wasm-sha1.js");
  for (const file of pkg.files) {
    assert.ok(fs.existsSync(path.join(webDir, file)), file);
  }

  const script = fs.readFileSync(path.join(webDir, pkg.browser), "utf8");
  const context = {
    console,
    WebAssembly,
    TextDecoder,
    TextEncoder,
    URL,
    Response,
    Uint8Array,
    DataView,
    Object,
    fetch() {
      throw new Error("init with module bytes must not fetch");
    },
  };
  context.globalThis = context;
  vm.createContext(context);
  vm.runInContext(script, context);

  assert.equal(typeof context.wasm_bindgen, "function");
  const wasm = fs.readFileSync(path.join(webDir, "wasm-sha1_bg.wasm"));
  await context.wasm_bindgen({ module_or_path: wasm });
  await context.wasm_bindgen({ module_or_path: wasm });
  assertVectors(context.wasm_bindgen.hash);
});
