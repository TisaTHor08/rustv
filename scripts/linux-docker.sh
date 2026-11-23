#!/usr/bin/env bash
# Verifications Linux dans Docker. Usage : scripts/linux-docker.sh {build-image|check|test|shell|clean} [args cargo]
# Images/volumes prefixes "rustv-". Le dossier target est un volume nomme (pas de bind mount : trop lent).
set -euo pipefail
cd "$(dirname "$0")/.."
IMAGE=rustv-linux-build
run() {
  docker run --rm -it -v "$(pwd -W 2>/dev/null || pwd):/src" \
    -v rustv-cargo-registry:/opt/cargo/registry -v rustv-cargo-git:/opt/cargo/git \
    -v rustv-target:/target -e CARGO_TARGET_DIR=/target \
    --name rustv-linux-run "$IMAGE" "$@"
}
cmd="${1:-check}"; shift || true
case "$cmd" in
  build-image) docker build -t "$IMAGE" -f docker/linux/Dockerfile docker/linux ;;
  check) run cargo check --workspace --all-targets "$@" ;;
  test)  run cargo test --workspace "$@" ;;
  shell) run bash ;;
  clean) docker volume rm rustv-cargo-registry rustv-cargo-git rustv-target 2>/dev/null || true; docker rmi "$IMAGE" 2>/dev/null || true ;;
  *) echo "usage: $0 {build-image|check|test|shell|clean}"; exit 2 ;;
esac
