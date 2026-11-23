# Porting RusTV to Linux and macOS

RusTV is Windows-only today. This document is the inventory of every Windows-specific piece, the portable
fallback that exists now (step 1: the whole workspace compiles and its tests pass on Linux, Windows behaviour is
unchanged), and what the real Linux and macOS backends should use (step 2 and later).

Status in one table:

| Target | `cargo check --workspace --all-targets` | `cargo test --workspace` | Notes |
| --- | --- | --- | --- |
| Windows (x86_64-pc-windows-msvc) | yes | yes | Reference behaviour, unchanged. |
| Linux (x86_64-unknown-linux-gnu), Docker | yes | yes | Compiles and tests run, but most OS features are stubs (listed below). |
| macOS | not verified for the shell | not verified | `rustv-protocol` and `rustv-core` pass `cargo check --target x86_64-apple-darwin` from Linux. `rustv-shell` (Tauri) needs the Apple SDK: it must be built on a Mac. |

## Convention

Two mechanisms, used consistently:

1. **Whole-module replacement** (`rustv-core`). The Windows implementation stays in its original file
   (`input.rs`, `audio.rs`...). On other systems the module is replaced by a stub file with exactly the same public
   API, selected in `lib.rs`:

   ```rust
   #[cfg_attr(not(windows), path = "platform/stub/audio.rs")]
   pub mod audio;
   ```

   Stub files live in `crates/rustv-core/src/platform/stub/`. They are shared by Linux and macOS. A real backend
   adds `platform/linux/<module>.rs` / `platform/macos/<module>.rs` and selects it with another `cfg_attr`
   (`target_os = "linux"`, `target_os = "macos"`) before the `not(windows)` one, or replaces the stub in place.
2. **Inner `sys` module replacement** (`rustv-shell`). `display.rs` and `guard.rs` keep their pure, tested logic
   and put every OS call in `mod sys`. On Windows that is the original inline module, elsewhere it is
   `crates/rustv-shell/src/platform/stub/<module>_sys.rs`. Smaller modules (`gamepad.rs`, `autostart.rs`,
   `usearch/secret.rs`, `ha/dpapi.rs`...) use item-level `#[cfg(windows)]` / `#[cfg(not(windows))]` pairs
   next to the Windows code.

Rules for every stub:

- Same public signatures as the Windows code, so callers never change.
- Never a fake success: `Err("unsupported_platform")` (constant `rustv_core::platform::UNSUPPORTED`), `None`,
  `false`, or an empty list. Functions whose historical signature returns `()` (key injection) print one line
  to stderr the first time they are called and expose `check()` returning the same error.
- Secrets never fall back to plain text: without a protector the sealing functions fail.
- Nothing sensitive is logged.

Portable helpers (`crates/rustv-core/src/platform/mod.rs`, unit-tested on every OS):

| Function | Windows | Linux | macOS |
| --- | --- | --- | --- |
| `config_dir()` | `%APPDATA%\RusTV` | `$XDG_CONFIG_HOME/RusTV` or `~/.config/RusTV` | `~/Library/Application Support/RusTV` |
| `data_dir()` | same as config | `$XDG_DATA_HOME/RusTV` or `~/.local/share/RusTV` | same as config |
| `video_dir()` | `%USERPROFILE%\Videos` | `$XDG_VIDEOS_DIR` or `~/Videos` | `~/Movies` |
| `home_dir()` | `%USERPROFILE%` | `$HOME` | `$HOME` |
| `exe_name("mpv")` | `mpv.exe` | `mpv` | `mpv` |
| `which(name)` | PATH, `.exe` added | PATH then `/usr/bin`, `/usr/local/bin`, `/snap/bin`, flatpak exports | PATH then `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/Applications` |
| `ipc::path(name)` | `\\.\pipe\<name>` | `$XDG_RUNTIME_DIR/<name>.sock` (temp dir if unset) | same as Linux |

`config_dir()` is used by `settings::dir()` (and therefore `store.rs`, tuner, mods, profiles, DVR state...), by
`rustv-core` (`config.json`, `vault.json`). Everything currently lives under `config_dir()`; `data_dir()` exists for
the Linux step (large data such as per-app web profiles, search index, recordings metadata may move there).
Paths are always built with `join`, never by concatenating separators.

Windows-only crates (`windows`, `windows-core`, `png` for icon extraction) sit under
`[target.'cfg(windows)'.dependencies]` in both crates. No dependency was added in step 1.

## Build for Linux with Docker

Prerequisites: Docker Desktop (or any Docker), and `ui/dist` built (the Tauri context embeds it):

```
cd ui && npm install && npm run build && cd ..
.\scripts\linux-docker.ps1 build-image     # once (ubuntu:24.04, Rust stable, Node 20, WebKitGTK 4.1 dev packages)
.\scripts\linux-docker.ps1 check           # cargo check --workspace --all-targets
.\scripts\linux-docker.ps1 test            # cargo test --workspace
.\scripts\linux-docker.ps1 shell           # interactive shell in the container
.\scripts\linux-docker.ps1 clean           # removes the rustv-* image and volumes
```

On Linux/macOS use `scripts/linux-docker.sh` with the same sub-commands. The worktree is mounted read-write on
`/src`; the cargo registry and the target directory live in the named volumes `rustv-cargo-registry`,
`rustv-cargo-git` and `rustv-target` (never on the bind mount: it is very slow). Extra arguments after the
sub-command are passed to cargo. The image name is `rustv-linux-build`.

System packages needed to build the Tauri 2 shell on Debian/Ubuntu: `build-essential pkg-config libssl-dev
libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`.

macOS check (best effort, protocol and core only):
`rustup target add x86_64-apple-darwin` then `cargo check --target x86_64-apple-darwin -p rustv-protocol -p rustv-core`.
The shell cannot be checked from Linux because Tauri's macOS crates compile Objective-C against the Apple SDK.

## Inventory

### rustv-core

| Module | Windows implementation | Fallback now (non-Windows) | Linux backend | macOS backend |
| --- | --- | --- | --- | --- |
| `input.rs` | `SendInput` (keyboard, Unicode text, mouse), `VkKeyScanExW`, `SetWindowPos`, `SetForegroundWindow` | `platform/stub/input.rs`: logical `VIRTUAL_KEY` codes with the Windows values and a `keys` module, `HWND` look-alike. Every injection function does nothing and prints `input unsupported_platform` once. `input::check()` returns `Err("unsupported_platform")` (always `Ok` on Windows). Window functions return neutral values. | `uinput` (`/dev/uinput`, works on X11 and Wayland, needs a udev rule) or XTest on X11 (`libxdo`); `libei` is the Wayland-native future. Text: keysym mapping or `xdotool type`. | `CGEventCreateKeyboardEvent` / `CGEventPost` (needs the Accessibility permission). Text: `CGEventKeyboardSetUnicodeString`. |
| `audio.rs` | Core Audio `IAudioEndpointVolume` | `get() -> None`, `set()` no-op, `run()` returns at once | `wpctl get-volume/set-volume @DEFAULT_AUDIO_SINK@` (PipeWire), fallback `pactl` | CoreAudio (`AudioObjectGetPropertyData`, `kAudioHardwareServiceDeviceProperty_VirtualMainVolume`) |
| `smtc.rs` | `GlobalSystemMediaTransportControlsSessionManager` (now playing) | `run()` returns at once, no now-playing | MPRIS over D-Bus (`org.mpris.MediaPlayer2.*`, `zbus`) | MediaRemote (private framework) or AppleScript for Music/Spotify |
| `uia.rs` | UI Automation focused-element polling (text field detection) | `run()` returns; text focus stays `false` | AT-SPI over D-Bus (`org.a11y.atspi`) | Accessibility API (`AXUIElement`, `AXFocusedUIElement`) |
| `apps.rs` | Start Menu `.lnk` scan, `ShellExecuteW` | `list()` returns only the custom apps, `launch()` does nothing | `.desktop` files in `XDG_DATA_DIRS/applications`, launch with `gtk-launch` or `Command`, URLs with `xdg-open` | `/Applications/*.app`, launch with `open` (`open -a`, `open <url>`) |
| `icons.rs` | `SHGetFileInfoW` + GDI to PNG | `icon_png_base64() -> None` | Icon theme lookup (hicolor, `Icon=` of the `.desktop`) | `.icns` of the app bundle |
| `bluetooth.rs` | WinRT RFCOMM service provider | `adapter_address() -> None`, `run()` returns `Err("unsupported_platform")` (and is not even called without an adapter) | BlueZ over D-Bus (`ProfileManager1`, RFCOMM profile) | Skip (CoreBluetooth has no RFCOMM server) |
| `vault.rs` | Mostly portable. `bw.exe`/`bw.cmd` discovery, `CREATE_NO_WINDOW`, `GetWindowTextW` | Discovery on Linux/macOS: `bw` on PATH, `/usr/local/bin`, `/opt/homebrew/bin`, `/usr/bin`, `/snap/bin`, `~/.local/bin`. Window title goes through `input::foreground_title()` (empty). Autofill typing fails visibly because input is a stub. | Same code; needs the input backend | Same code; needs the input backend |
| `config.rs`, `core.rs` | Paths and key codes | Use `platform::config_dir()` and `input::keys` | Nothing more | Nothing more |
| `server.rs` | Portable (TCP, UDP beacon) | None | Check firewall (ufw/firewalld) docs | Check macOS firewall prompt |

### rustv-shell

| Module | Windows implementation | Fallback now (non-Windows) | Linux backend | macOS backend |
| --- | --- | --- | --- | --- |
| `autostart.rs` | `HKCU\...\Run` via `reg.exe` | `get()` returns `"off"`, `set()` does nothing. `run_value()` stays portable and tested. | `~/.config/autostart/rustv.desktop` (`Exec=... --boot`) | LaunchAgent `~/Library/LaunchAgents/org.rustv.app.plist` |
| `guard.rs` (TV guard) | WinEventHook, taskbar, `reg.exe` toast setting, `SetThreadExecutionState`, window moves | `platform/stub/guard_sys.rs`: every `sys` function is a no-op, `keep_awake_start()` returns a dead channel. All decision logic (Rect, Policy, Trust, Backoff, recovery plans, boot decision) stays compiled and tested. The settings page still shows the guard but it does nothing. | Not applicable at first (no taskbar to hide). Later: `org.freedesktop.ScreenSaver.Inhibit` or `systemd-inhibit` for keep-awake, `gsettings` for banners, EWMH on X11 | `IOPMAssertionCreate` for keep-awake; guard not planned |
| `display.rs` | Core Audio endpoint list, `IPolicyConfig`, GDI `EnumDisplaySettings`/`ChangeDisplaySettingsEx`, DisplayConfig HDR | `platform/stub/display_sys.rs`: audio outputs and mode changes return `Err("unsupported_platform")`, monitors and modes are empty, HDR unsupported. Pure logic (mode filtering, revert state machine, output cycling) stays tested. | Outputs: `wpctl status` / `pactl list sinks`; modes: `xrandr` (X11) or `wlr-randr` (wlroots) or `gnome-randr`; HDR: none | CoreAudio; CoreGraphics (`CGDisplaySetDisplayMode`); HDR via CoreGraphics |
| `gamepad.rs` | XInput loaded dynamically, ToolHelp snapshot for the foreground process | XInput section is `#[cfg(windows)]`; `start()` does not spawn the poll thread, `gamepad_get` reports `available: false`, `gamepad_test_vibration` returns `Err("unsupported_platform")`. Mapping, repeat, combo and foreground policy logic stays tested. | `gilrs` crate (evdev) or direct evdev; rumble through force feedback | GameController framework (`GCController`) |
| `browser.rs` | Brave/Vivaldi install paths (LOCALAPPDATA, Program Files, registry App Paths), `taskkill` | Candidate paths for Linux (`/usr/bin/brave-browser`, `/opt/brave.com/brave/brave`, `/snap/bin/brave`, flatpak exports, `/usr/bin/vivaldi`, `/opt/vivaldi/vivaldi`) and macOS (`/Applications/Brave Browser.app/Contents/MacOS/Brave Browser`, Vivaldi) plus PATH; no registry; `close()` sends `kill -TERM`. App mode flags (`--app=`, `--user-data-dir=`...) are Chromium flags and are identical on every OS. Chrome and Chromium are not yet detected. | Add Chrome (`google-chrome`, `/opt/google/chrome/chrome`) and Chromium (`chromium`, `chromium-browser`, snap) as new `Kind`s if wanted; process group kill | Same, plus Chrome in `/Applications/Google Chrome.app` |
| `dvr.rs` | `GetDiskFreeSpaceExW`, mpv named pipe, `CREATE_NO_WINDOW` | Default folder from `platform::video_dir()`. mpv IPC through `platform::ipc` (Unix socket). `free_space()` returns `None` (DVR then never blocks on disk space). | `statvfs` for free space (the `libc` crate or `fs2`/`fs4`) | Same |
| `player.rs` | Downloads the Windows build of mpv (7z), `mpv.exe`, named pipe IPC | `find_mpv()` looks for `mpv` on PATH and common prefixes, `install()` returns `Err("unsupported_platform")` (never downloads a binary of another OS), IPC over a Unix socket through `platform::ipc`. mpv itself must be installed by the user (`apt install mpv`, `brew install mpv`). | Optionally install a Flatpak or AppImage; `mpv` options such as `--hwdec=auto-safe` and `--screen-name` need testing under Wayland | `brew install mpv`, `/Applications/mpv.app` |
| `hyperion.rs` | `hyperiond.exe` download/unzip, Job Object kill-on-close, `taskkill`, DPAPI for the API token | Process lookup: `hyperiond` on PATH and `/usr/bin`, `/usr/local/bin`, `/opt/homebrew/bin`, `/snap/bin`. `install()` returns `Err("unsupported_platform")`. Stop uses `kill -TERM`. Token sealing fails (`dpapi_failed`) so a token cannot be saved. | Package manager install; API token in the Secret Service | Same with Keychain |
| `ha/dpapi.rs` | DPAPI (`CryptProtectData`) for the MQTT password | `Dpapi` fails with `Err("unsupported_platform")` for protect and unprotect (no plain-text fallback) | libsecret / Secret Service (`secret-service` or `keyring` crate) | Keychain (`security-framework` or `keyring`) |
| `usearch/secret.rs` | DPAPI for TMDB/Plex/Jellyfin tokens | Same as `ha/dpapi.rs` | Same | Same |
| `update.rs` | Downloads `-setup.exe` and runs it | `install()` returns `Err("unsupported_platform")`; `check()` still works (no asset is offered) | Ship `.deb` / AppImage / Flatpak and update through them | `.dmg`, Sparkle or Homebrew |
| `tv.rs` | `WebviewWindow::hwnd()` + `force_foreground`, VLC in Program Files, WebView2 `additional_browser_args` for self-signed LAN certificates | Focus helper compiled only on Windows (`set_focus()` remains). VLC: `vlc` on PATH or `/Applications/VLC.app`. The self-signed certificate option is ignored (see below). Per-app profiles keep using `data_directory`, which WebKitGTK and WKWebView support. | `with_webview` on WebKitGTK to set `WebContext::set_tls_errors_policy` | `WKNavigationDelegate` challenge handling via `with_webview` |
| `fs.rs` | `USERPROFILE`, `GetLogicalDrives`, hidden attribute | Home from `platform::home_dir()` (Videos is Movies on macOS); drive list replaced by a `/` root; dotfiles hidden | Add mounted volumes (`/media`, `/run/media/$USER`, `/mnt`) | `/Volumes` |
| `settings.rs`, `store.rs` | `%APPDATA%\RusTV` | `platform::config_dir()` | Nothing more | Nothing more |
| `commands.rs` | `windows_apps` (Start Menu list), `open_path` through `apps::launch` | Follows `rustv_core::apps` stubs | Follows the `apps.rs` backend | Same |
| `main.rs` | `windows_subsystem = "windows"` attribute (harmless elsewhere), tray icon | Tray uses Tauri `tray-icon` (needs `libayatana-appindicator3` at runtime on Linux) | Check tray visibility on GNOME (extension needed) | Menu bar icon |
| `cast/*`, `lan.rs`, `live.rs`, `tuner.rs`, `library.rs`, `mods.rs`, `media.rs`, `ha/*` (MQTT), `usearch/*` | Portable (network, files, JSON) | None | Check `socket2` multicast on Linux interfaces | Same |

### Things deliberately unchanged

- The Tauri command names (`windows_apps`, ...) and JSON shapes: the UI is untouched.
- The Windows code paths: the Windows implementation was moved only where a stub had to be selected, never
  rewritten. Behaviour differences on Windows are limited to `exe_name()` (identical `.exe` suffix) and the
  default DVR folder (`USERPROFILE\Videos\RusTV Recordings`, with the previous temp-dir fallback).

## Known limitations of the stubs

- No key, mouse or text injection: a paired phone connects and sees the state, but remote key presses do nothing
  until the input backend exists. `rustv_core::input::check()` tells callers.
- No volume sync, no now-playing, no text-field detection, no app list from the system, no icons.
- No autostart, no TV guard, no display or audio output switching, no gamepad, no HDR.
- Secrets (MQTT password, TMDB/Plex/Jellyfin tokens, Hyperion token) cannot be saved: the settings return
  `unsupported_platform`. Do not add a plain-text fallback.
- Self-signed HTTPS certificates on the LAN (`allow_insecure_lan`) are not accepted by the embedded browser.
- `mpv` and the optional programs (Brave, Vivaldi, VLC, Bitwarden CLI, Hyperion) are found but never downloaded.
- The Linux webview is WebKitGTK 4.1: sites that rely on Chromium-only features (DRM streaming such as Netflix,
  some YouTube TV behaviour) need testing; the Brave/Vivaldi app-mode path exists for that reason.

## What is not verified

- Nothing in this document was run on a real Linux or macOS desktop: only compilation and unit tests (in Docker for
  Linux). The GUI was never started on Linux.
- macOS: only `rustv-protocol` and `rustv-core` were type-checked (`x86_64-apple-darwin`, from Linux). The shell
  has not been compiled for macOS.
- The `cfg(not(windows))` branches of `kill -TERM` (browser, Hyperion) and the Unix socket IPC for mpv were not
  exercised against real processes.

## Step 2 checklist (Linux backends)

1. Input injection (`uinput` or XTest) and `input::check()` returning `Ok`.
2. Volume (`wpctl`/`pactl`) for `audio.rs` and `display::sys` audio outputs.
3. App list and launching (`.desktop`, `xdg-open`, `gtk-launch`), icons.
4. Media session (MPRIS), text-field detection (AT-SPI).
5. Secrets (Secret Service) behind the existing `Protector` trait.
6. Autostart `.desktop`, packaging (`.deb`, AppImage), update path.
7. Gamepad (evdev/gilrs), display modes (`xrandr`/`wlr-randr`), free disk space (`statvfs`).
8. TLS policy for self-signed LAN hosts in WebKitGTK; test per-app profiles and the launch animation
   (`on_page_load`) on WebKitGTK.
