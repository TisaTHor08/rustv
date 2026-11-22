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

### `crates/rustv-core`
Library embedded by the shell: accepts phone connections (TCP port 47820, Bluetooth RFCOMM,
UDP discovery beacon on 47821), injects keyboard/mouse input with `SendInput`, launches apps, detects focused
text fields with UI Automation, handles Alt+Tab and "pin on top", extracts app icons. It reports phone
commands that concern the TV (Home, start/stop TV mode, device changes) to the shell through an event channel.

Design note: the phone is "just a keyboard" for the TV interface. Arrow keys sent by the remote are real key
events, so the home screen, web apps and native apps all receive them the same way.

### `remote/`
Svelte + Capacitor Android app. The protocol is re-implemented in TypeScript (WebCrypto). A small native
plugin (`RusTVTransportPlugin.java`) exposes raw TCP / RFCOMM / UDP to the WebView. Connection paths are
raced in parallel (last IP, discovery beacon, QR IPs, Bluetooth) and the first authenticated one wins.

### `crates/rustv-shell` + `ui/`
The `rustv.exe` Tauri application: the "TV mode".
- Screen picker at launch (one window per monitor); other displays are blacked out (toggle in settings).
- 10-foot home screen (Svelte) with hero + rows of tiles driven by the remote.
- App tiles come from `src/catalog.json`: **web** tiles open in a WebView2 window with its own profile
  (`%APPDATA%\RusTV\profiles\<id>`), no Tauri IPC, optional user-agent override and spatial-navigation script;
  **native** tiles launch an executable or URI (VLC, Steam).
- Web app windows have no capability, hence no access to RusTV's commands.

### Mods (`crates/rustv-shell/src/mods.rs`, `ui/src/lib/mods.svelte.ts`)
A mod is a folder with a `mod.json` manifest plus JS/CSS/HTML assets, documented in [MODS.md](MODS.md). It can contribute themes, tiles,
pages, top-bar widgets, site adapters and translations. Execution model:

- Widgets and pages run in a sandboxed iframe served by the local media server with a strict CSP (no network). The only way out
  is a small host API (storage, `fetch` to approved origins, launch, toast) over `postMessage`; every call is checked against the
  permissions the user accepted (stored in `mods-state.json`).
- Site adapters are injected into web app windows as initialization scripts, only if the user approved the matching `inject:` permission.
- The catalog installs a *full commit* of a GitHub repository (content-addressed), previews the real manifest, then asks for approval.

### Ad-blocking browser for YouTube (`crates/rustv-shell/src/browser.rs`)
Tiles flagged `"adblock_browser": true` in `catalog.json` (YouTube only for now) open in an installed Chromium browser instead of WebView2.
The UI sends the `youtubeBrowser` pref (`auto` | `builtin` | `brave` | `vivaldi`) with `launch_tile`; `auto` picks Brave, then Vivaldi, else WebView2.
Detection checks the standard install paths (`%LOCALAPPDATA%`, `%ProgramFiles%`, `%ProgramFiles(x86)%`) and read-only `reg query` of App Paths (`browsers_detected` command).
The browser runs as `--app=<url> --start-fullscreen` positioned on the TV monitor, with `--user-data-dir=%APPDATA%\RusTVrowser-profiles\<id>` (never the user's profile) and the tile user agent.
RusTV keeps the spawned PID; `go_home`, `exit` and opening another app kill that process tree (`taskkill /PID <pid> /T /F`), so the phone's Home button returns to the TV home.
If the browser exits by itself, RusTV returns home (or falls back to WebView2 if it died within 2 s). Limits: Brave Shields are on by default; Vivaldi's built-in blocker
is configured in its own settings (RusTV does not edit the profile), so the user must enable it once in the dedicated profile.

### Player, live TV and library (`crates/rustv-shell/src/{player,live,library}.rs`)
- **Player**: RusTV drives `mpv.exe` (full screen on the TV monitor) over its JSON IPC pipe. mpv is found if already installed, otherwise
  downloaded **only after the user agrees** from the latest `shinchiro/mpv-winbuild-cmake` release (the build mpv.io links to) and checked against
  the SHA-256 digest published by GitHub. A generated `input.conf` maps the remote's keys: in live mode Up/Down zap through the channel list (mpv sends
  a `script-message` that RusTV answers with `loadfile`), in video mode Left/Right seek; Back quits and returns to the home screen.
- **Live TV**: user-supplied M3U playlists and XMLTV guides (URL or file, `.gz` accepted, streamed). Only `http(s)`, `rtmp(s)`, `rtsp`, `udp`, `rtp`, `srt` and
  `mms` stream URLs are accepted (a playlist cannot point mpv at local files). RusTV ships no channel list.
- **Library**: scans the folders you pick (no network). Titles come from file names, series are grouped by `SxxExx`, posters are `poster.jpg`/`folder.jpg`
  next to the media. Photos open in a built-in viewer with slideshow; music and video play in mpv; the file explorer opens anything else with its default app.
- **Now playing**: the core reads Windows' media session (SMTC) and the mpv state and sends `nowPlaying` to the phone.
## Security model

- The pairing secret is shared only through the QR code shown on the PC.
- Authentication is challenge-response (HMAC-SHA256); the secret never crosses the wire.
- Frames are AES-256-GCM encrypted with HKDF-derived per-direction keys and an implicit counter nonce.
- Unpaired devices are refused; pairings are revocable individually.
- Known limits: `SendInput` cannot reach elevated windows unless RusTV is elevated; the lock screen,
  UAC and Windows Hello are unreachable; Chrome exposes text fields to UI Automation only with
  accessibility enabled.
