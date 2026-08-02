#!/usr/bin/env bash
# Builds `canvas` and launches it as a real macOS .app.
#
# Why this exists: `cargo run` produces a bare binary. macOS gives unbundled
# processes no Dock icon, no menu bar, and no way to become the active
# application — the window appears and draws, but every keystroke goes to
# whatever app is actually frontmost. Keyboard shortcuts are dead. Wrapping the
# same binary in a .app is the entire fix.
#
# Note: no `--features dev`. Dynamic linking leaves an `@rpath/libstd`
# dependency that only resolves under `cargo run`, so a bundled dev build won't
# launch. Slower to compile, but self-contained.
#
# Usage: ./run.sh

set -euo pipefail

cd "$(dirname "$0")"

APP="target/Canvas.app"

cargo build -p canvas

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp apps/canvas/macos/Info.plist "$APP/Contents/Info.plist"
cp target/debug/canvas "$APP/Contents/MacOS/canvas"

# `-n` forces a new instance rather than re-focusing a stale one; without it a
# previously launched copy just comes forward and you test the old build.
open -n "$APP"
