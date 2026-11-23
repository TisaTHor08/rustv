# Builds the RusTV .deb and .AppImage inside Docker and copies them to dist-linux\ (git-ignored).
# Usage: .\scripts\build-linux.ps1 [-RebuildUi] [-SkipTests]
# Requires Docker and the image built once with: .\scripts\linux-docker.ps1 build-image
# ui\dist must exist (cd ui; npm install; npm run build) unless -RebuildUi is used (slower: npm ci in the container).
param([switch]$RebuildUi, [switch]$SkipTests)
$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent $PSScriptRoot
$Image = "rustv-linux-build"
$args2 = @()
if ($RebuildUi) { $args2 += "--rebuild-ui" }
if ($SkipTests) { $args2 += "--skip-tests" }
docker run --rm -v "${Root}:/src" -v rustv-cargo-registry:/opt/cargo/registry -v rustv-cargo-git:/opt/cargo/git `
    -v rustv-target:/target -e CARGO_TARGET_DIR=/target --name rustv-linux-bundle $Image bash scripts/build-linux.sh @args2
exit $LASTEXITCODE
