#!/usr/bin/env bash
# Builds the web version and serves it.
#
# Two steps, and the second is the one people forget: `cargo build --target
# wasm32-unknown-unknown` produces a .wasm that a browser cannot load on its
# own. `wasm-bindgen` writes the JavaScript glue that imports it and wires up
# the canvas, the event loop and the allocator.
#
#   ./web.sh          build and serve on :8080
#   ./web.sh build    build only
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "wasm-bindgen is not installed. Once:"
  echo "    cargo install wasm-bindgen-cli"
  exit 1
fi

echo "==> building for wasm32-unknown-unknown"
cargo build -p canvas --release --target wasm32-unknown-unknown

echo "==> writing the JavaScript glue"
wasm-bindgen \
  --out-dir web/pkg \
  --target web \
  --no-typescript \
  target/wasm32-unknown-unknown/release/canvas.wasm

if [ "${1:-serve}" = "build" ]; then
  echo "==> built. Serve web/ with any static server."
  exit 0
fi

echo "==> http://localhost:8080"
# Python rather than a dependency: it is on every machine this runs on, and a
# static server is not worth a package.
cd web && python3 -m http.server 8080
