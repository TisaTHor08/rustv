# RusTV

**A real TV system for Windows. Free, open source, fully customizable, controlled from your phone.**

RusTV turns a Windows PC into a smart TV: a Google-TV-style home screen, your streaming apps
(YouTube, Netflix, Prime Video, Plex, Jellyfin, UGREEN Theater, …), VLC, live TV channels, and a
phone app that works as the remote. Everything can be themed and extended with **mods**.

> Status: early development (v0.x). The remote-control foundation works; the TV shell is next.
> See the [roadmap](docs/ROADMAP.md) and the [architecture](docs/ARCHITECTURE.md).

## What works today

- **Phone remote** (Svelte + Capacitor, Android): QR pairing, D-pad, OK/Back, tab switching (Alt+Tab),
  pin window on top, app launcher with icons, touchpad mode, keyboard that pops up when a text field
  is focused on the PC.
- **Windows service** (Rust): tray icon, input injection (`SendInput`), app launcher, text-field
  detection (UI Automation), TCP + Bluetooth RFCOMM transports, LAN discovery beacon.
- **Security**: HMAC-SHA256 challenge-response (the secret never travels), AES-256-GCM encrypted
  frames with anti-replay counters, per-phone revocable pairing, no Internet needed.
  See [docs/PROTOCOL.md](docs/PROTOCOL.md).

## Coming next

TV shell (Tauri + WebView2, DRM playback verified: Widevine and PlayReady are available), multi-screen
mode that blacks out other displays, customization (wallpapers, tiles, themes), the mod system, and
live TV (M3U + XMLTV, bring your own playlist).

## Repository layout

| Path | Content |
| --- | --- |
| `crates/rustv-protocol` | Wire protocol: framing, handshake, encryption, messages (Rust) |
| `crates/rustv-service` | Windows background service (Rust) |
| `remote/` | Phone app (Svelte + Capacitor); protocol re-implemented in TypeScript |
| `docs/` | Architecture, protocol, roadmap |

## Build

Requirements: Rust (stable, MSVC), Node 22+, and Android Studio for the APK.

```
cargo test -p rustv-protocol
cargo build --release -p rustv-service        # target/release/rustv-service.exe

cd remote
npm install && npm run build && npx cap sync android
npx cap open android                          # Build > Build APK
```

Interoperability test (TypeScript client against the Rust service):
`node remote/scripts/e2e.ts <ip:port> <deviceId> <keyBase64>`.

## Contributing

Issues and pull requests are welcome. RusTV is, and will stay, free and open source.

## License

[GPL-3.0-or-later](LICENSE). Forks must remain open source too.
