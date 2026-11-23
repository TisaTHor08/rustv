# Environnement de build/test Linux pour RusTV (Tauri 2 : WebKitGTK 4.1).
# Usage : voir scripts/linux-docker.ps1 / scripts/linux-docker.sh et docs/PORTING.md.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential pkg-config curl ca-certificates git file xz-utils \
        libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
        librsvg2-dev libssl-dev libxdo-dev libdbus-1-dev \
        dpkg-dev patchelf wget squashfs-tools desktop-file-utils xdg-utils zsync xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Rust stable (rustup) + cible macOS pour les verifications de structure (best effort).
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup target add x86_64-apple-darwin

# Node 20 (nodesource), utile pour construire ui/ dans le conteneur si besoin.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

# Tauri CLI (cargo tauri build --bundles deb,appimage). Couche separee : longue a compiler, mise en cache.
# Les bundles AppImage s'executent sans FUSE (APPIMAGE_EXTRACT_AND_RUN) dans un conteneur.
ENV APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=true CARGO_TARGET_DIR=/target
RUN cargo install tauri-cli --version "^2" --locked && rm -rf /opt/cargo/registry /opt/cargo/git
