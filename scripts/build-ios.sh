#!/usr/bin/env bash
# Builds the RusTV phone remote for the iOS simulator and writes a report into dist-ios/.
#
#   scripts/build-ios.sh                      builds, boots a simulator, installs, launches, screenshots
#   scripts/build-ios.sh --no-run             build only
#   scripts/build-ios.sh --device "iPhone 16" choose the simulator by name
#
# No signing identity is used (CODE_SIGNING_ALLOWED=NO), nothing is uploaded.
# Prerequisite: the iOS platform exists (remote/ios, created by `npx cap add ios`, see docs/AGENT-BUILD-IOS.md).
# Exit code: 0 OK, 2 build failed, 3 prerequisites missing.
set -uo pipefail

RUN=1
DEVICE_NAME=""
while [ $# -gt 0 ]; do
  case "$1" in
    --no-run) RUN=0 ;;
    --device) shift; DEVICE_NAME="${1:-}" ;;
    -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
    *) echo "unknown option: $1"; exit 3 ;;
  esac
  shift
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 3
OUT="$ROOT/dist-ios"
rm -rf "$OUT"; mkdir -p "$OUT"
LOG="$OUT/build.log"; : > "$LOG"
say() { printf '\n== %s\n' "$*" | tee -a "$LOG"; }
json_escape() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr '\n' ' '; }
S_WEB="skipped"; S_SYNC="skipped"; S_BUILD="skipped"; S_RUN="skipped"; NOTES=""
note() { NOTES="$NOTES$1; "; }

say "Checking prerequisites"
[ "$(uname -s)" = "Darwin" ] || { echo "This script must run on macOS."; exit 3; }
command -v xcodebuild >/dev/null || { echo "Xcode is missing. Install Xcode from the App Store, open it once to accept the licence, then run: sudo xcode-select -s /Applications/Xcode.app"; exit 3; }
xcodebuild -version >> "$LOG" 2>&1 || { echo "xcodebuild does not work (accept the Xcode licence: sudo xcodebuild -license)"; exit 3; }
command -v node >/dev/null || { echo "Node.js 20+ is missing (brew install node)"; exit 3; }
[ -d remote/ios ] || { echo "remote/ios is missing: run 'cd remote && npm install @capacitor/ios@^8 && npx cap add ios' first (docs/AGENT-BUILD-IOS.md)"; exit 3; }
GIT_SHA="$(git rev-parse HEAD)"; GIT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
XCODE_V="$(xcodebuild -version | tr '\n' ' ')"

say "Building the web app and syncing Capacitor"
if (cd remote && npm ci && npm run build) >> "$LOG" 2>&1; then S_WEB="ok"; else S_WEB="failed"; note "web build failed"; fi
if [ "$S_WEB" = ok ]; then
  if (cd remote && npx cap sync ios) >> "$LOG" 2>&1; then S_SYNC="ok"; else S_SYNC="failed"; note "cap sync failed"; fi
fi

# Find the Xcode project or workspace Capacitor generated.
cd remote/ios/App 2>/dev/null || { echo "remote/ios/App not found"; exit 3; }
if [ -d App.xcworkspace ]; then CONTAINER=(-workspace App.xcworkspace); else CONTAINER=(-project App.xcodeproj); fi
DERIVED="$OUT/DerivedData"

if [ "$S_SYNC" = ok ]; then
  say "Building for the iOS simulator (no code signing)"
  if xcodebuild "${CONTAINER[@]}" -scheme App -configuration Debug -sdk iphonesimulator \
       -destination 'generic/platform=iOS Simulator' -derivedDataPath "$DERIVED" \
       CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO build >> "$LOG" 2>&1; then S_BUILD="ok"; else S_BUILD="failed"; note "xcodebuild failed"; fi
fi
cd "$ROOT" || exit 3

APP_PATH="$(ls -d "$DERIVED"/Build/Products/Debug-iphonesimulator/*.app 2>/dev/null | head -1)"
if [ "$S_BUILD" = ok ] && [ -n "$APP_PATH" ]; then
  ditto -c -k --keepParent "$APP_PATH" "$OUT/RusTV-Remote-simulator.app.zip"
  BUNDLE_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP_PATH/Info.plist" 2>/dev/null)"
  if [ "$RUN" = 1 ]; then
    say "Running in the simulator"
    if [ -n "$DEVICE_NAME" ]; then UDID="$(xcrun simctl list devices available | grep -F "$DEVICE_NAME (" | head -1 | sed -E 's/.*\(([0-9A-F-]{36})\).*/\1/')"
    else UDID="$(xcrun simctl list devices available | grep -E 'iPhone' | head -1 | sed -E 's/.*\(([0-9A-F-]{36})\).*/\1/')"; fi
    if [ -z "$UDID" ]; then note "no iPhone simulator found (install one in Xcode > Settings > Platforms)"; S_RUN="no_simulator"
    else
      { xcrun simctl boot "$UDID"; xcrun simctl bootstatus "$UDID" -b; xcrun simctl install "$UDID" "$APP_PATH"; xcrun simctl launch "$UDID" "$BUNDLE_ID"; } >> "$LOG" 2>&1 \
        && { sleep 6; xcrun simctl io "$UDID" screenshot "$OUT/launch.png" >> "$LOG" 2>&1; S_RUN="ok"; } \
        || { S_RUN="failed"; note "simulator run failed"; }
      xcrun simctl shutdown "$UDID" >> "$LOG" 2>&1
    fi
  fi
elif [ "$S_BUILD" = ok ]; then note "no .app found in DerivedData"; fi
rm -rf "$DERIVED"
( cd "$OUT" && ls ./*.zip ./*.png >/dev/null 2>&1 && shasum -a 256 ./*.zip ./*.png > SHA256SUMS ) || true

cat > "$OUT/build-report.json" <<EOF
{
  "project": "RusTV phone remote (iOS)",
  "commit": "$GIT_SHA",
  "branch": "$GIT_BRANCH",
  "xcode": "$(json_escape "$XCODE_V")",
  "steps": { "web_build": "$S_WEB", "cap_sync": "$S_SYNC", "xcodebuild_simulator": "$S_BUILD", "simulator_run": "$S_RUN" },
  "signed": false,
  "notes": "$(json_escape "$NOTES")"
}
EOF
say "Done"; cat "$OUT/build-report.json"
if [ "$S_BUILD" = ok ]; then echo "Build OK. Results are in dist-ios/."; exit 0; fi
echo "Build FAILED. Last lines of the log:"; tail -40 "$LOG"; exit 2
