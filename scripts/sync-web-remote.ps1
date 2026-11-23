# Construit la télécommande web (remote/) et la copie dans crates\rustv-shell\remote-web (ressource Tauri).
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location "$root\remote"
try { npm ci; npm run build } finally { Pop-Location }
$dest = "$root\crates\rustv-shell\remote-web"
Get-ChildItem $dest -Force | Where-Object { $_.Name -notin 'PLACEHOLDER.txt', '.gitignore' } | Remove-Item -Recurse -Force
Copy-Item "$root\remote\dist\*" $dest -Recurse
Write-Host "Télécommande web copiée dans $dest"
