# Architecture

RusTV is made of four parts that talk to each other over a small, encrypted protocol.

```
 Phone (remote/)  ──TCP / Bluetooth──▶  rustv-service  ◀──▶  rustv-shell (Tauri, planned)
                                          │  input injection, pairing, UI Automation
                                          ▼
                                  Windows apps / web apps / VLC
```

## Components

### `crates/rustv-protocol`
Transport-independent protocol: length-prefixed frames, mutual HMAC handshake, AES-256-GCM sessions,
JSON messages. Everything goes through `Read + Write`, so TCP and Bluetooth RFCOMM share one code path.
Specified in [PROTOCOL.md](PROTOCOL.md).

### `crates/rustv-service`
Background Windows service with a tray icon: accepts phone connections (TCP port 47820, Bluetooth RFCOMM,
UDP discovery beacon on 47821), injects keyboard/mouse input with `SendInput`, launches apps, detects focused
text fields with UI Automation, handles Alt+Tab and "pin on top".

### `remote/`
Svelte + Capacitor Android app. The protocol is re-implemented in TypeScript (WebCrypto). A small native
plugin (`RusTVTransportPlugin.java`) exposes raw TCP / RFCOMM / UDP to the WebView. Connection paths are
raced in parallel (last IP, discovery beacon, QR IPs, Bluetooth) and the first authenticated one wins.

### `rustv-shell` (planned)
Tauri application: the "TV mode".
- Screen picker at launch; other displays are blacked out (can be disabled in settings).
- 10-foot home screen with rows of tiles, wallpaper, spatial navigation driven by the remote.
- App tiles: **web** (WebView2 window with its own persistent profile, Widevine/PlayReady verified available),
  **native** (VLC, Kodi, any executable), **live channel**.

### `rustv-mods` (planned)
A mod is a folder with a `mod.json` manifest plus JS/CSS assets:

```
mods/my-theme/
  mod.json        { "id", "name", "version", "permissions": [...], "entry": "main.js", "styles": ["theme.css"] }
  main.js
  theme.css
```

Mods can add themes, tiles, pages, channel providers, and inject scripts into web apps (for example
spatial navigation for sites without a TV mode). Every sensitive capability (network, launching programs,
key injection) must be declared in the manifest and accepted by the user at install time.

### `rustv-live` (planned)
Live TV from user-supplied M3U playlists and XMLTV guides. RusTV does not ship any channel list.

## Security model

- The pairing secret is shared only through the QR code shown on the PC.
- Authentication is challenge-response (HMAC-SHA256); the secret never crosses the wire.
- Frames are AES-256-GCM encrypted with HKDF-derived per-direction keys and an implicit counter nonce.
- Unpaired devices are refused; pairings are revocable individually.
- Known limits: `SendInput` cannot reach elevated windows unless RusTV is elevated; the lock screen,
  UAC and Windows Hello are unreachable; Chrome exposes text fields to UI Automation only with
  accessibility enabled.
