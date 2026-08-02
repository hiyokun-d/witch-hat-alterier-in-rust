#!/usr/bin/env bash
# Builds `canvas` and launches it as a real macOS .app, then cleans up.
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

# Fires on normal exit, on Ctrl-C, and on kill — so quitting the app or
# interrupting the script never leaves a stale bundle behind. Only the bundle
# goes; `target/` itself is the build cache and rebuilding it costs minutes.
cleanup() {
	rm -rf "$APP"
}
trap cleanup EXIT INT TERM

cargo build -p canvas

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp apps/canvas/macos/Info.plist "$APP/Contents/Info.plist"
cp target/debug/canvas "$APP/Contents/MacOS/canvas"

# `-n` forces a new instance rather than re-focusing a stale one; without it a
# previously launched copy just comes forward and you test the old build.
# `-W` blocks until the app quits, which is what lets the trap above clean up
# at the right moment instead of deleting the bundle out from under a running
# process.
open -n -W "$APP"
