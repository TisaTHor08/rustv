# Verifications Linux dans Docker (Docker Desktop, WSL2).
# Usage : .\scripts\linux-docker.ps1 build-image | check | test | shell | clean  [arguments cargo]
# Images/volumes prefixes "rustv-". Le dossier target est un volume nomme (pas de bind mount : trop lent).
# ui/dist doit exister (npm run build dans ui/) : tauri::generate_context! l'embarque.
param([Parameter(Position = 0)][string]$Cmd = "check", [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest)
$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent $PSScriptRoot
$Image = "rustv-linux-build"
function Invoke-Linux([string[]]$ContainerArgs) {
    docker run --rm -v "${Root}:/src" -v rustv-cargo-registry:/opt/cargo/registry -v rustv-cargo-git:/opt/cargo/git `
        -v rustv-target:/target -e CARGO_TARGET_DIR=/target --name rustv-linux-run $Image @ContainerArgs
}
switch ($Cmd) {
    "build-image" { docker build -t $Image -f "$Root/docker/linux/Dockerfile" "$Root/docker/linux" }
    "check" { Invoke-Linux (@("cargo", "check", "--workspace", "--all-targets") + $Rest) }
    "test" { Invoke-Linux (@("cargo", "test", "--workspace") + $Rest) }
    "shell" { docker run --rm -it -v "${Root}:/src" -v rustv-cargo-registry:/opt/cargo/registry -v rustv-cargo-git:/opt/cargo/git -v rustv-target:/target -e CARGO_TARGET_DIR=/target --name rustv-linux-shell $Image bash }
    "clean" {
        docker volume rm rustv-cargo-registry rustv-cargo-git rustv-target 2>$null
        docker rmi $Image 2>$null
    }
    default { Write-Error "usage: linux-docker.ps1 build-image|check|test|shell|clean" }
}
