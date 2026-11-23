#!/usr/bin/env bash
# Construit la télécommande web (remote/) et la copie dans crates/rustv-shell/remote-web, d'où Tauri l'embarque
# comme ressource. À lancer avant `cargo tauri build` (voir docs/WEB-REMOTE.md).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
(cd "$root/remote" && npm ci && npm run build)
dest="$root/crates/rustv-shell/remote-web"
find "$dest" -mindepth 1 ! -name PLACEHOLDER.txt ! -name .gitignore -delete
cp -r "$root/remote/dist/." "$dest/"
echo "Télécommande web copiée dans $dest"
