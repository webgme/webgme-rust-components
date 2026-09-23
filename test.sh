#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v node >/dev/null 2>&1; then
  echo "node 18+ is required to run the wasm contract tests" >&2
  exit 1
fi

cargo test

echo "Checking the committed dist artifacts"
node --test tests/contract.test.js

fresh="$(mktemp -d)"
trap 'rm -rf "$fresh"' EXIT
./build.sh "$fresh"

echo "Checking a fresh build of the current sources"
COMPAT_DIST="$fresh" node --test tests/contract.test.js
