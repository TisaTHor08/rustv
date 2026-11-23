#!/usr/bin/env bash
# Builds RusTV for macOS and writes everything a maintainer needs to review the result into dist-macos/.
#
#   scripts/build-macos.sh              native architecture (Apple silicon or Intel)
#   scripts/build-macos.sh --universal  one app for both (needs both Rust targets, installed automatically)
#   scripts/build-macos.sh --skip-tests skip `cargo test` (not recommended)
#
# Output (dist-macos/): RusTV_<version>_<arch>.dmg, RusTV_<version>_<arch>.app.zip, SHA256SUMS,
# build-report.json (machine readable), build.log (full log). No signing identity is used and nothing is uploaded.
# Exit code: 0 build OK, 2 build failed, 3 prerequisites missing.
set -uo pipefail

UNIVERSAL=0
SKIP_TESTS=0
for a in "$@"; do
  case "$a" in
    --universal) UNIVERSAL=1 ;;
    --skip-tests) SKIP_TESTS=1 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "unknown option: $a"; exit 3 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 3
OUT="$ROOT/dist-macos"
rm -rf "$OUT"; mkdir -p "$OUT"
LOG="$OUT/build.log"
: > "$LOG"

say() { printf '\n== %s\n' "$*" | tee -a "$LOG"; }
run() { echo "+ $*" >> "$LOG"; "$@" >> "$LOG" 2>&1; }
json_escape() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr '\n' ' '; }

STATUS_PREREQ="ok"; STATUS_UI="skipped"; STATUS_TESTS="skipped"; STATUS_BUILD="skipped"; NOTES=""
note() { NOTES="$NOTES$1; "; }

# ---------------------------------------------------------------- prerequisites
say "Checking prerequisites"
if [ "$(uname -s)" != "Darwin" ]; then echo "This script must run on macOS."; exit 3; fi
ARCH="$(uname -m)"                     # arm64 or x86_64
MACOS_VERSION="$(sw_vers -productVersion)"
if ! xcode-select -p >/dev/null 2>&1; then
  echo "Xcode Command Line Tools are missing. Run: xcode-select --install   (a window opens; accept, wait, then re-run)"; exit 3
fi
command -v git >/dev/null || { echo "git is missing"; exit 3; }
if ! command -v node >/dev/null; then echo "Node.js 20+ is missing. Install it with: brew install node"; exit 3; fi
NODE_MAJOR="$(node -p 'process.versions.node.split(".")[0]')"
if [ "$NODE_MAJOR" -lt 20 ]; then echo "Node.js 20+ is required (found $(node -v))"; exit 3; fi
if ! command -v cargo >/dev/null; then
  if [ -x "$HOME/.cargo/bin/cargo" ]; then export PATH="$HOME/.cargo/bin:$PATH"; else
    echo "Rust is missing. Install it from https://rustup.rs (official installer), then open a new terminal."; exit 3
  fi
fi
run rustup update stable
if [ "$UNIVERSAL" = 1 ]; then run rustup target add aarch64-apple-darwin x86_64-apple-darwin; fi
if ! cargo tauri --version >/dev/null 2>&1; then
  say "Installing the Tauri CLI (cargo install tauri-cli, a few minutes)"
  run cargo install tauri-cli --version "^2" --locked || { echo "Could not install tauri-cli, see $LOG"; exit 3; }
fi
GIT_SHA="$(git rev-parse HEAD)"
GIT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
GIT_DIRTY="clean"; [ -n "$(git status --porcelain)" ] && GIT_DIRTY="modified"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' crates/rustv-shell/Cargo.toml | head -1)"
LOCK_SHA="$(shasum -a 256 Cargo.lock | cut -d' ' -f1)"
echo "RusTV $VERSION on macOS $MACOS_VERSION ($ARCH), commit $GIT_SHA ($GIT_DIRTY)" | tee -a "$LOG"

# ---------------------------------------------------------------- UI
say "Building the TV interface (ui/)"
if (cd ui && npm ci && npm run build) >> "$LOG" 2>&1; then STATUS_UI="ok"; else STATUS_UI="failed"; note "ui build failed"; fi

# ---------------------------------------------------------------- tests
if [ "$SKIP_TESTS" = 0 ] && [ "$STATUS_UI" = ok ]; then
  say "Running cargo test --workspace"
  if cargo test --workspace >> "$LOG" 2>&1; then STATUS_TESTS="ok"; else STATUS_TESTS="failed"; note "cargo test failed (see build.log)"; fi
  TESTS_SUMMARY="$(grep -E '^test result' "$LOG" | tail -5 | tr '\n' ' ')"
fi

# ---------------------------------------------------------------- bundle
if [ "$STATUS_UI" = ok ]; then
  say "Building the app bundle (cargo tauri build, 5 to 15 minutes the first time)"
  if [ "$UNIVERSAL" = 1 ]; then TARGET_ARG="--target universal-apple-darwin"; BUNDLE_DIR="target/universal-apple-darwin/release/bundle"; ARCH_LABEL="universal"
  else TARGET_ARG=""; BUNDLE_DIR="target/release/bundle"; ARCH_LABEL="$ARCH"; fi
  # shellcheck disable=SC2086
  if cargo tauri build --bundles app,dmg $TARGET_ARG >> "$LOG" 2>&1; then STATUS_BUILD="ok"; else STATUS_BUILD="failed"; note "cargo tauri build failed"; fi
fi

# ---------------------------------------------------------------- collect
DMG=""; APPZIP=""
if [ "$STATUS_BUILD" = ok ]; then
  SRC_DMG="$(ls "$BUNDLE_DIR"/dmg/*.dmg 2>/dev/null | head -1)"
  SRC_APP="$(ls -d "$BUNDLE_DIR"/macos/*.app 2>/dev/null | head -1)"
  if [ -n "$SRC_DMG" ]; then DMG="RusTV_${VERSION}_${ARCH_LABEL}.dmg"; cp "$SRC_DMG" "$OUT/$DMG"; else note "no dmg produced"; fi
  if [ -n "$SRC_APP" ]; then APPZIP="RusTV_${VERSION}_${ARCH_LABEL}.app.zip"; ditto -c -k --keepParent "$SRC_APP" "$OUT/$APPZIP"; else note "no .app produced"; fi
  # Smoke test: the app must start and stay alive for 8 seconds (no window is needed; it lives in the menu bar).
  if [ -n "$SRC_APP" ]; then
    BIN="$(ls "$SRC_APP"/Contents/MacOS/* | head -1)"
    "$BIN" >> "$LOG" 2>&1 & PID=$!
    sleep 8
    if kill -0 "$PID" 2>/dev/null; then SMOKE="started_and_stayed_alive"; kill "$PID" 2>/dev/null; else SMOKE="exited_early"; note "app exited within 8 s"; fi
  else SMOKE="not_run"; fi
else SMOKE="not_run"; fi
( cd "$OUT" && ls ./*.dmg ./*.zip >/dev/null 2>&1 && shasum -a 256 ./*.dmg ./*.zip 2>/dev/null > SHA256SUMS ) || true

# ---------------------------------------------------------------- report
RUSTC_V="$(rustc --version 2>/dev/null)"; CARGO_V="$(cargo --version 2>/dev/null)"; NODE_V="$(node -v)"; TAURI_V="$(cargo tauri --version 2>/dev/null)"
cat > "$OUT/build-report.json" <<EOF
{
  "project": "RusTV",
  "version": "$VERSION",
  "commit": "$GIT_SHA",
  "branch": "$GIT_BRANCH",
  "working_tree": "$GIT_DIRTY",
  "cargo_lock_sha256": "$LOCK_SHA",
  "macos": "$MACOS_VERSION",
  "arch": "$ARCH",
  "bundle_arch": "${ARCH_LABEL:-n/a}",
  "toolchain": { "rustc": "$(json_escape "$RUSTC_V")", "cargo": "$(json_escape "$CARGO_V")", "node": "$NODE_V", "tauri_cli": "$(json_escape "$TAURI_V")" },
  "steps": { "prerequisites": "$STATUS_PREREQ", "ui": "$STATUS_UI", "tests": "$STATUS_TESTS", "bundle": "$STATUS_BUILD", "smoke_test": "$SMOKE" },
  "tests_summary": "$(json_escape "${TESTS_SUMMARY:-}")",
  "artifacts": { "dmg": "$DMG", "app_zip": "$APPZIP" },
  "signed": false,
  "notes": "$(json_escape "$NOTES")"
}
EOF

say "Done"
cat "$OUT/build-report.json"
[ -f "$OUT/SHA256SUMS" ] && cat "$OUT/SHA256SUMS"
echo
if [ "$STATUS_BUILD" = ok ]; then echo "Build OK. Results are in dist-macos/."; exit 0; fi
echo "Build FAILED. Read dist-macos/build.log (last lines below)."; tail -40 "$LOG"; exit 2
