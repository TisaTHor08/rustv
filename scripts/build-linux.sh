#!/usr/bin/env bash
# Builds the RusTV Linux packages INSIDE the build container (docker/linux/Dockerfile) and writes everything a
# maintainer needs into dist-linux/ of the repository (git-ignored). Normally started by scripts/build-linux.ps1
# (or scripts/linux-docker.sh build) which mounts the repository on /src.
#
#   scripts/build-linux.sh                 use ui/dist if present, run tests, build .deb and .AppImage
#   scripts/build-linux.sh --rebuild-ui    run `npm ci && npm run build` in ui/ first
#   scripts/build-linux.sh --skip-tests    skip `cargo test --workspace`
#
# Output (dist-linux/): rustv_<version>_<arch>.deb, RusTV_<version>_<arch>.AppImage, SHA256SUMS, build-report.json,
# build.log. Nothing is signed or uploaded. Exit code: 0 OK, 2 build failed, 3 prerequisites missing.
set -uo pipefail

REBUILD_UI=0
SKIP_TESTS=0
for a in "$@"; do
  case "$a" in
    --rebuild-ui) REBUILD_UI=1 ;;
    --skip-tests) SKIP_TESTS=1 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "unknown option: $a"; exit 3 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 3
OUT="$ROOT/dist-linux"
rm -rf "$OUT"; mkdir -p "$OUT"
LOG="$OUT/build.log"
: > "$LOG"

say() { printf '\n== %s\n' "$*" | tee -a "$LOG"; }
run() { echo "+ $*" >> "$LOG"; "$@" >> "$LOG" 2>&1; }
json_escape() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr '\n' ' '; }
STATUS_UI="skipped"; STATUS_TESTS="skipped"; STATUS_BUILD="skipped"; NOTES=""
note() { NOTES="$NOTES$1; "; }

say "Checking prerequisites"
for t in cargo node dpkg-deb; do command -v "$t" >/dev/null || { echo "$t is missing (run this inside the rustv-linux-build image)"; exit 3; }; done
cargo tauri --version >/dev/null 2>&1 || { echo "tauri-cli is missing (rebuild the image: scripts/linux-docker.ps1 build-image)"; exit 3; }
export APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=true
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
ARCH="$(dpkg --print-architecture)"        # amd64, arm64...
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' crates/rustv-shell/Cargo.toml | head -1 | tr -d '[:space:]')"
GIT_SHA="$(git -c safe.directory='*' rev-parse HEAD 2>/dev/null || echo unknown)"
GIT_BRANCH="$(git -c safe.directory='*' rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
LOCK_SHA="$(sha256sum Cargo.lock | cut -d' ' -f1)"
echo "RusTV $VERSION for Linux $ARCH, commit $GIT_SHA" | tee -a "$LOG"

say "TV interface (ui/)"
if [ "$REBUILD_UI" = 1 ]; then
  if (cd ui && npm ci && npm run build) >> "$LOG" 2>&1; then STATUS_UI="rebuilt"; else STATUS_UI="failed"; note "ui build failed"; fi
elif [ -f ui/dist/index.html ]; then
  STATUS_UI="prebuilt"
else
  echo "ui/dist is missing: build it on the host (cd ui && npm install && npm run build) or pass --rebuild-ui" | tee -a "$LOG"
  STATUS_UI="missing"; note "ui/dist missing"
fi

if [ "$SKIP_TESTS" = 0 ] && [ "$STATUS_UI" != failed ] && [ "$STATUS_UI" != missing ]; then
  say "cargo test --workspace"
  if cargo test --workspace >> "$LOG" 2>&1; then STATUS_TESTS="ok"; else STATUS_TESTS="failed"; note "cargo test failed (see build.log)"; fi
  TESTS_SUMMARY="$(grep -E '^test result' "$LOG" | tr '\n' ' ')"
fi

if [ "$STATUS_UI" != failed ] && [ "$STATUS_UI" != missing ]; then
  say "cargo tauri build --bundles deb,appimage (the first run downloads linuxdeploy: network needed)"
  if (cd crates/rustv-shell && cargo tauri build --bundles deb,appimage) >> "$LOG" 2>&1; then STATUS_BUILD="ok"; else STATUS_BUILD="failed"; note "cargo tauri build failed"; fi
fi

DEB=""; APPIMAGE=""
if [ "$STATUS_BUILD" = ok ]; then
  B="$TARGET_DIR/release/bundle"
  SRC_DEB="$(ls "$B"/deb/*.deb 2>/dev/null | head -1 | tr -d "")"
  SRC_AI="$(ls "$B"/appimage/*.AppImage 2>/dev/null | head -1 | tr -d "")"
  if [ -n "$SRC_DEB" ]; then DEB="$(basename "$SRC_DEB")"; cp "$SRC_DEB" "$OUT/$DEB"; else note "no .deb produced"; fi
  if [ -n "$SRC_AI" ]; then APPIMAGE="$(basename "$SRC_AI")"; cp "$SRC_AI" "$OUT/$APPIMAGE"; else note "no .AppImage produced"; fi
fi
( cd "$OUT" && ls ./*.deb ./*.AppImage >/dev/null 2>&1 && sha256sum ./*.deb ./*.AppImage 2>/dev/null | sed 's#\./##' > SHA256SUMS ) || true
DEB_INFO=""; [ -n "$DEB" ] && DEB_INFO="$(dpkg-deb -f "$OUT/$DEB" Package Version Architecture Depends Recommends 2>/dev/null)"

cat > "$OUT/build-report.json" <<EOF
{
  "project": "RusTV",
  "version": "$VERSION",
  "commit": "$GIT_SHA",
  "branch": "$GIT_BRANCH",
  "cargo_lock_sha256": "$LOCK_SHA",
  "os": "$(. /etc/os-release && echo "$PRETTY_NAME")",
  "arch": "$ARCH",
  "toolchain": { "rustc": "$(json_escape "$(rustc --version)")", "cargo": "$(json_escape "$(cargo --version)")", "node": "$(node -v)", "tauri_cli": "$(json_escape "$(cargo tauri --version)")" },
  "steps": { "ui": "$STATUS_UI", "tests": "$STATUS_TESTS", "bundle": "$STATUS_BUILD" },
  "tests_summary": "$(json_escape "${TESTS_SUMMARY:-}")",
  "artifacts": { "deb": "$DEB", "appimage": "$APPIMAGE" },
  "deb_control": "$(json_escape "$DEB_INFO")",
  "signed": false,
  "notes": "$(json_escape "$NOTES")"
}
EOF

say "Done"
cat "$OUT/build-report.json"
[ -f "$OUT/SHA256SUMS" ] && cat "$OUT/SHA256SUMS"
echo
if [ "$STATUS_BUILD" = ok ]; then echo "Build OK. Results are in dist-linux/."; exit 0; fi
echo "Build FAILED. Read dist-linux/build.log (last lines below)."; tail -40 "$LOG"; exit 2
