#!/usr/bin/env bash
# Runs in the build image: builds the helper binaries used by the smoke test (see scripts/smoke-linux.ps1) and copies
# them to dist-linux/tools: the fake phone (example of rustv-protocol) and the unit-test executables of rustv-core and
# rustv-shell (their `#[ignore]` tests talk to real PulseAudio, the keyring and /dev/uinput).
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=dist-linux/tools
rm -rf "$OUT"; mkdir -p "$OUT"
cargo build -p rustv-protocol --example fake_phone
cp "${CARGO_TARGET_DIR:-target}/debug/examples/fake_phone" "$OUT/fake_phone"
for pair in "rustv-core:core" "rustv-shell:shell"; do
  crate="${pair%%:*}"; name="${pair##*:}"
  exe="$(cargo test -p "$crate" --no-run 2>&1 | sed -n 's/.*Executable .* (\(.*\))/\1/p' | head -1)"
  [ -n "$exe" ] && cp "$exe" "$OUT/test-$name"
done
ls -la "$OUT"
