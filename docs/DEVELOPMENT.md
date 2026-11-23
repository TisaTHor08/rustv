# Development guide

## Prerequisites

- Windows 10/11 (the TV shell and input injection are Windows-only).
- Rust stable with the **MSVC** toolchain (and the Visual Studio Build Tools / C++ workload).
- Node.js 22 or newer.
- WebView2 runtime (included in Windows 11).
- Android Studio, only if you build or modify the phone app APK.
- Optional, to check the Linux build: Docker (see below and [PORTING.md](PORTING.md)).

## Repository map

| Path | Purpose | Key files |
| --- | --- | --- |
| `crates/rustv-protocol` | Wire protocol: frames, mutual handshake, AES-256-GCM sessions, JSON messages | `frame.rs`, `pairing.rs`, `session.rs`, `crypto.rs`, `messages.rs` |
| `crates/rustv-core` | Phone server (TCP, Bluetooth, UDP beacon), input injection, pairing config, UI Automation, now-playing | `server.rs`, `core.rs`, `input.rs`, `uia.rs`, `smtc.rs`, `config.rs` |
| `crates/rustv-shell` | The `rustv.exe` Tauri app: windows, tray, tiles, mods, live TV, library, player, update check | `main.rs`, `commands.rs`, `catalog.json`, `mods.rs`, `settings.rs`, `prefs.rs`, `live.rs`, `library.rs`, `player.rs`, `update.rs`, `sdk.js`, `spatnav.js`, `tauri.conf.json`, `capabilities/` |
| `ui/` | TV interface (Svelte 5 + Vite): home, settings, picker, pairing, live TV, library, mod host | `src/lib/Home.svelte`, `Settings.svelte`, `i18n.ts`, `api.ts`, `modHost.ts` |
| `remote/` | Phone app (Svelte 5 + Capacitor, Android); protocol re-implemented in TypeScript | `src/lib/protocol/`, `connection.svelte.ts`, `Remote.svelte`, `i18n.ts`, `scripts/`, `android/` |
| `mods/` | Example mods (theme, widget, page, tiles, adapter) | each folder has a `mod.json` |
| `registry/` | Community mod catalog | `index.json` |
| `assets/` | Logo | `logo.svg` |
| `docs/` | Documentation | see [README.md](README.md) |
| `bluetooth.rs` | Empty stray file at the repository root, not part of the build (the Bluetooth code is `crates/rustv-core/src/bluetooth.rs`) | |

## Build, run, test

```
# Protocol and shell unit tests
cargo test -p rustv-protocol -p rustv-shell

# TV interface (the shell embeds ui/dist)
cd ui
npm install
npm run check          # svelte-check + tsc
npm run build
cd ..

# The application
cargo build --release -p rustv-shell        # target/release/rustv.exe

# Phone app
cd remote
npm install
npm run check
npm run build
npx cap sync android
npx cap open android                         # then Build > Build APK in Android Studio
```

Run `rustv.exe` for tray mode, or `rustv.exe --tv --screen N` to start directly in TV mode on screen number `N`.
`npm run dev` in `ui/` or `remote/` starts a Vite dev server for the web part only.

### Linux packages and smoke test in Docker

`.\scripts\build-linux.ps1` builds the `.deb` and the `.AppImage` inside the build image and copies them to `dist-linux/`
(git-ignored) with `SHA256SUMS` and `build-report.json`. `.\scripts\smoke-linux.ps1` installs the `.deb` in a clean Ubuntu 24.04
container with a virtual display (Xvfb), D-Bus, PulseAudio (dummy sink) and gnome-keyring, starts `rustv --tv` for 20 s and
reports (process alive, port 47820, config files, handshake with the `fake_phone` example, a screenshot). See [LINUX.md](LINUX.md).

### Linux build check in Docker

The workspace compiles and its unit tests pass on Linux (OS features are stubs, see [PORTING.md](PORTING.md)).
With Docker Desktop and `ui/dist` built (`cd ui && npm run build`):

```
.\scripts\linux-docker.ps1 build-image   # once
.\scripts\linux-docker.ps1 check         # cargo check --workspace --all-targets
.\scripts\linux-docker.ps1 test          # cargo test --workspace
```

(`scripts/linux-docker.sh` offers the same sub-commands for bash.) Images and volumes are prefixed `rustv-`;
`clean` removes them.

### Integration tests against a real PC

With RusTV running and a phone paired (the pairing gives the device id and key):

```
node remote/scripts/e2e.ts <ip:port> <deviceId> <keyBase64>
node remote/scripts/send.ts <ip:port> <deviceId> <keyBase64> '<json message>' ['<json message>' ...]
```

`e2e.ts` runs the TypeScript protocol client against the Rust core. `send.ts` sends phone messages by hand (700 ms
pause between messages, replies are printed). Message shapes are in [PROTOCOL.md](PROTOCOL.md).

## Debugging the TV interface

The TV UI is a WebView2 page, so Chrome DevTools Protocol works. Start RusTV with the debugging port enabled:

```
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
.\target\release\rustv.exe --tv --screen 1
```

Then drive it from `remote/scripts/cdp.mjs` (run from the `remote/` folder, set `CDP_PORT` if you use another port):

```
node scripts/cdp.mjs - list                         # list pages (first argument is ignored here)
node scripts/cdp.mjs <urlContains> shot out.png     # screenshot
node scripts/cdp.mjs <urlContains> key ArrowDown Enter
node scripts/cdp.mjs <urlContains> type "some text" # insert text in the focused field
node scripts/cdp.mjs <urlContains> eval "document.title"
node scripts/cdp.mjs <urlContains> logs 4000        # console + exceptions for 4000 ms
```

Caveats:

- **CDP only sees the first WebView2 environment.** The TV interface is visible; web app windows (YouTube, Netflix,
  ...) use their own separate profiles under `%APPDATA%\RusTV\profiles\<id>` and do not show up in `list`.
- Run `logs` in one terminal while sending `key` commands from another.
- The debugging port is a development tool; never leave it enabled for normal use.

## Where user data lives

Everything is under `%APPDATA%\RusTV\`:

| Path | Content |
| --- | --- |
| `config.json` | Pairing: paired devices and keys (secret, never share or log it) |
| `prefs.json` | Interface preferences: theme, wallpaper, tiles, language, weather, ... |
| `tv.json` | TV mode settings: chosen screen, black-out, URLs typed for tiles that need your own server |
| `mods/` | Installed mods (`mods/<id>/mod.json`); related state in `mods-data/` and `mods-state.json` |
| `profiles/` | One WebView2 profile per web app (sessions never mix) |
| `player/` | The mpv player, installed on demand |

To start fresh, quit RusTV and move the folder aside rather than deleting it.

## Security rules for contributors

- **No telemetry**, no analytics, no phoning home. The only network calls are the ones the user asked for (apps they
  open, optional weather, update check against GitHub Releases, catalog, mpv download with checksum).
- **Never add Tauri capabilities to web app windows.** They must stay unable to call RusTV commands
  (`crates/rustv-shell/capabilities/`). Only the TV interface window gets commands.
- **Mod permission model**: `storage`, `launch`, `network:<origin>`, `inject:<pattern>`. Pages and widgets are
  sandboxed with a strict CSP. Do not add implicit powers; a new permission needs an issue and a docs update.
- **No secrets in logs**: never print pairing keys, device secrets, tokens, playlist URLs with credentials.
- Keep the protocol guarantees: the secret never travels, frames are authenticated and encrypted, counters stop replay.
  Changes under `crates/rustv-protocol` must update [PROTOCOL.md](PROTOCOL.md) and the tests.

## Testing safely

- Do **not** press Enter on the *Start with Windows* row in settings while testing: it writes a `Run` entry in your
  user registry. Toggle it back if you did.
- Clean up test files in `%APPDATA%\RusTV` afterwards (test mods, playlists, extra profiles), or use a throwaway
  Windows user.
- On first launch Windows Firewall asks to allow RusTV on your network: accept for private networks, otherwise the
  phone cannot connect over Wi-Fi.
- Use a second screen or a windowed setup when testing TV mode; black-out mode covers the other monitors.
- Do not test with real credentials or private playlists in screenshots and logs you plan to share.
