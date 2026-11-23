# Smoke test of the Linux .deb in Docker (virtual display, D-Bus, PulseAudio dummy sink, keyring).
# Prerequisites: .\scripts\linux-docker.ps1 build-image, then .\scripts\build-linux.ps1 (dist-linux\*.deb).
# Usage: .\scripts\smoke-linux.ps1   -> dist-linux\smoke\ (smoke-report.txt, home.png); delete it when you are done.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Dist = Join-Path $Root "dist-linux"
if (-not (Get-ChildItem $Dist -Filter *.deb -ErrorAction SilentlyContinue)) { Write-Error "no .deb in dist-linux: run scripts\build-linux.ps1 first" }
docker build -t rustv-linux-smoke -f "$Root/docker/linux/smoke.Dockerfile" "$Root/docker/linux"
docker run --rm -v "${Root}:/src" -v rustv-cargo-registry:/opt/cargo/registry -v rustv-cargo-git:/opt/cargo/git `
    -v rustv-target:/target -e CARGO_TARGET_DIR=/target rustv-linux-build bash scripts/smoke-tools.sh
$Out = Join-Path $Dist "smoke"
New-Item -ItemType Directory -Force $Out | Out-Null
docker run --rm -v "${Dist}:/in" -v "${Out}:/out" -v "${Root}/scripts:/scripts:ro" --name rustv-linux-smoke-run rustv-linux-smoke bash /scripts/smoke-linux.sh
