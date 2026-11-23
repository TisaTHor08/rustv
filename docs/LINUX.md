# RusTV on Linux

RusTV runs on Linux (x86_64) as a Tauri 2 application on WebKitGTK 4.1. This page explains how to install it,
what has to be set up on the system, what works, what does not, and what was actually verified.

**Status: first Linux release, tested only in Docker (virtual display) and with unit tests. It was never run on a real
desktop, with a real TV, a real phone, a real gamepad or a Wayland session.** Expect rough edges and please report them.

## Install

Download one of the packages from the release page (or build them yourself, see [Build](#build)).

### Debian, Ubuntu, Linux Mint, Pop!_OS (.deb)

```
sudo apt install ./rustv_<version>_amd64.deb
```

`apt` pulls the dependencies (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`) and suggests
`mpv`, `wireplumber` or `pulseaudio-utils`, `xdg-utils` and `xdotool`. The package also installs:

- `/usr/lib/udev/rules.d/60-rustv-uinput.rules` and `/usr/lib/modules-load.d/rustv-uinput.conf`: they let the phone
  send keys and mouse movements (see [Phone keys and mouse](#phone-keys-and-mouse-devuinput));
- a menu entry (category Video, shown under Sound & Video) and the RusTV icons.

Log out and in once (or reboot) after the first install so the new permissions apply to your session.

### Other distributions (AppImage)

```
chmod +x RusTV_<version>_amd64.AppImage
./RusTV_<version>_amd64.AppImage
```

The AppImage does not install the udev rule: copy the two small files from this repository
(`crates/rustv-shell/linux/60-rustv-uinput.rules` to `/etc/udev/rules.d/`,
`crates/rustv-shell/linux/rustv-uinput.conf` to `/etc/modules-load.d/`), then reboot. The system libraries
`libwebkit2gtk-4.1` and `libayatana-appindicator3` are bundled or must be installed by your distribution
(see the package names in your distribution).

### Programs RusTV uses when they are installed

| Program | Used for | Install (Debian/Ubuntu) |
| --- | --- | --- |
| `mpv` | Playback, live TV, recordings | `sudo apt install mpv` |
| `wpctl` (WirePlumber) or `pactl` | System volume and audio outputs | already there on most desktops |
| `xdg-utils` | Opening files, folders and links | `sudo apt install xdg-utils` |
| `xdotool` (X11) or `wtype` (wlroots Wayland) | Typing text from the phone with the right keyboard layout | `sudo apt install xdotool` |
| Google Chrome, Brave, Vivaldi or Chromium | Streaming apps with DRM, YouTube without ads | see below |
| `vlc`, `steam` | The VLC and Steam tiles | your package manager |

RusTV never downloads mpv or Hyperion on Linux: the in-app "install" buttons show the package manager command for
your distribution (detected from `/etc/os-release`), for example `use_package_manager:sudo apt install mpv`.

## Phone keys and mouse (/dev/uinput)

The phone sends arrows, Enter, Back, volume keys, media keys, text and mouse movements. RusTV injects them through a
virtual keyboard and a virtual mouse created with `/dev/uinput`, which works on both X11 and Wayland.

Requirements:

1. The `uinput` kernel module must be loaded (`lsmod | grep uinput`). The package loads it at boot through
   `/usr/lib/modules-load.d/rustv-uinput.conf`; right now you can run `sudo modprobe uinput`.
2. Your user must be allowed to write to `/dev/uinput`. The shipped udev rule is:

   ```
   KERNEL=="uinput", SUBSYSTEM=="misc", GROUP="input", MODE="0660", TAG+="uaccess", OPTIONS+="static_node=uinput"
   ```

   `TAG+="uaccess"` gives access to the user logged in on the local seat; `GROUP="input"` also allows members of the
   `input` group (`sudo usermod -aG input $USER`, then log out and in). Reload with
   `sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=misc --sysname-match=uinput`.

If `/dev/uinput` cannot be opened, the error code is `uinput_denied`. RusTV shows it as a red banner in the pairing
window and `platform_features` reports it for the Settings pages; the rest of RusTV keeps working but the phone cannot
drive the PC.

Notes and limits:

- Security: any program running as your user (or in the `input` group) can then create virtual keyboards. That is the
  normal trade-off of `uinput`; do not add users to `input` that you do not trust.
- Text typing: `wtype` is used on Wayland and `xdotool type` on X11 when installed, because they follow your keyboard
  layout. Without them RusTV falls back to uinput key codes for plain ASCII, which is correct only on a US QWERTY
  layout (it is skipped when `setxkbmap -query` reports another layout). Otherwise the text is dropped and
  `input tool_missing` is printed once on stderr. GNOME on Wayland does not support `wtype`; install `xdotool` for
  Xwayland applications or use a US layout.
- Media keys (play/pause, next, previous, stop) first try MPRIS on the active player and only send the real key when
  no player is found.
- Window pinning, "bring to front" and the foreground window title are not available (no portable API): the Alt+Tab
  overlay of the phone sends Alt+Tab to your window manager as it is.

## Volume and audio outputs

Volume and mute use `wpctl` (PipeWire) and fall back to `pactl`. The phone's volume bar follows the PC (poll every
500 ms while a phone is connected). The list of outputs, the default output and the per-output volume (Settings >
Screen and sound) use `pactl list sinks`, or `wpctl status` when `pactl` is missing. Without either tool the volume bar
is absent and the audio pages report `tool_missing`.

## Apps, icons and launching

Installed apps come from the XDG `.desktop` files in `~/.local/share/applications`, `/usr/share/applications`,
`/usr/local/share/applications`, the Flatpak exports and the snap desktop directory. Entries that are hidden
(`NoDisplay`, `Hidden`) or terminal-only are skipped. Apps start through their `Exec` line (field codes removed, no
shell, detached), `xdg-open` opens files and links, Steam starts with `steam -bigpicture`, VLC with `vlc`.
Icons come from the `hicolor` and other icon themes and `/usr/share/pixmaps` (PNG only, SVG icons are skipped, so apps
that ship only an SVG show no icon on the phone).

## Streaming apps and DRM

The embedded web view on Linux is WebKitGTK. It has no Widevine module, so Netflix, Prime Video, Disney+, Max,
Apple TV+, Canal+ and other DRM services do not play in it. RusTV therefore opens these tiles in a Chromium-family
browser in app mode on the TV screen, with its own profile (`~/.config/RusTV/browser-profiles/<tile>`), when one is
installed:

| Browser | Detected as | DRM |
| --- | --- | --- |
| Google Chrome (`google-chrome-stable`, `google-chrome`) | `chrome` | Widevine included, preferred |
| Brave, Vivaldi | `brave`, `vivaldi` | depends on their Widevine setup (Brave asks to enable it) |
| Chromium (`chromium`, `chromium-browser`) | `chromium` | usually no Widevine on Linux |

YouTube keeps using Brave or Vivaldi for their ad blocker (Chrome and Chromium are only used for YouTube when you pick
them explicitly in Settings > Apps).

Control: the setting "Open streaming apps in a browser" is on by default and has no effect when no browser is found
(the tile then opens in the built-in view). It is stored in `~/.config/RusTV/linux.json`
(`streaming_browser`, `streaming_browser_kind`), exposed by the `linux_get` / `linux_set` commands and by the Linux
settings page. A catalog or custom tile can set `"drm": true` or `"drm": false` to force or exclude it; the
default list is `netflix`, `prime`, `disney`, `max`, `appletv`, `canal`, `crunchyroll`, `paramount`, `ocs`, `rakuten`,
`molotov`, `spotify`, `applemusic`, `tidal`.

On X11 the window is placed on the TV monitor with `--window-position` and `--window-size`. On Wayland the browser is
started with `--ozone-platform=wayland` and `--start-fullscreen`; Wayland compositors decide where the window opens,
so move it to the TV once if needed. Flatpak and snap browsers are detected but their sandbox may refuse the
`~/.config/RusTV` profile folder: prefer the `.deb` builds.

## Secrets

The Home Assistant (MQTT) password, the TMDB, Plex and Jellyfin tokens and the Hyperion token are stored in the desktop
keyring through the Secret Service D-Bus API (GNOME Keyring, KWallet, KeePassXC). Only a random identifier is written in
RusTV's JSON files. If no keyring is running or it stays locked, saving a secret fails with `keyring_unavailable`: RusTV
never falls back to plain text on disk. Install and unlock `gnome-keyring` (or enable the Secret Service in KeePassXC or
KWallet) to use these features.

## Autostart

Settings > Start with the PC writes `~/.config/autostart/rustv.desktop` (`Exec=<rustv> --boot` or
`--tv --boot`, `X-GNOME-Autostart-enabled=true`) and removes it when turned off. For an AppImage the `.AppImage` file
path is used.

## Now playing and play/pause

Other players are read through MPRIS (D-Bus): Firefox, Chromium, Spotify, VLC and any player exposing
`org.mpris.MediaPlayer2.*`. The RusTV player (mpv) keeps priority through its own state. Title, artist, position,
duration and state appear on the phone; play/pause, next and previous go to the active player.

## Tray icon and GNOME

The tray uses `libayatana-appindicator3`. GNOME hides tray icons unless the "AppIndicator and KStatusNotifierItem
Support" extension is enabled. When no tray host is registered on the session bus (or the icon cannot be created),
RusTV does not exit: it opens the pairing window at start so you always have a way in. Close it and use the phone,
or enable the extension.

## Monitors and TV mode

TV mode places a fullscreen window on the chosen monitor (position, size, fullscreen through Tauri/tao). Monitors are
ordered left to right as on Windows. On X11 this works as on Windows in principle; on Wayland the compositor ignores
client-side positioning, so the window may open on the monitor where the mouse or focus is. Moving the window to the
TV once and choosing that monitor in the picker is the workaround. Fullscreen on a specific Wayland output
(`fullscreen_on_monitor`) is not wired in this release.

## Updates

The in-app updater only checks for a newer release and shows the link; it never installs on Linux (`manual_update`).
Update with the new `.deb` or AppImage.

## What is not supported on Linux (yet)

| Feature | State |
| --- | --- |
| Game controller | Not implemented (the XInput backend is Windows only; `gilrs` is not wired). The Settings page must show it as unavailable. |
| TV guard (keep other windows off the TV, taskbar, notifications) | No-op. Keep-awake (inhibit sleep) is not implemented either: disable screen blanking in your desktop settings for TV use. |
| Resolution, refresh rate, HDR | Not implemented (stubs). Use your desktop display settings. |
| Bluetooth RFCOMM remote | Skipped: use Wi-Fi. |
| Text field detection for other applications | Not implemented (reports "no text field"): the phone keyboard opens only for RusTV's own pages. |
| Self-signed HTTPS certificates on the LAN (`allow_insecure_lan`) | Ignored by WebKitGTK in this release; use plain HTTP, a trusted certificate, or open the service in Chromium. |
| App icons that are SVG only | No icon. |

`platform_features` (Tauri command) returns the list as booleans: `guard`, `display_modes`, `hdr`, `gamepad` and
`bluetooth` are `false` on Linux.

## Troubleshooting

- **The window is blank or black (virtual machines, some NVIDIA setups)**: start with
  `WEBKIT_DISABLE_DMABUF_RENDERER=1 rustv` or `WEBKIT_DISABLE_COMPOSITING_MODE=1 rustv`.
- **Phone connects but nothing happens when you press keys**: check `uinput_denied` (see above), then
  `ls -l /dev/uinput` and `getfacl /dev/uinput`.
- **The phone cannot find the PC**: open TCP 47820 and UDP discovery in your firewall (`sudo ufw allow 47820/tcp`).
- **No tray icon on GNOME**: install the AppIndicator extension; RusTV opens its pairing window meanwhile.
- **Netflix says it cannot play**: install Google Chrome (Widevine) and keep "Open streaming apps in a browser" on.
- **Saving a password fails with `keyring_unavailable`**: start and unlock a Secret Service keyring.

## Build

Everything is built in Docker, no Linux machine is needed:

```
cd ui && npm install && npm run build && cd ..
.\scripts\linux-docker.ps1 build-image      # once
.\scripts\build-linux.ps1                   # dist-linux/: .deb, .AppImage, SHA256SUMS, build-report.json
```

On Linux, `scripts/build-linux.sh` does the same inside any machine with the build prerequisites. See
[PORTING.md](PORTING.md) and [DEVELOPMENT.md](DEVELOPMENT.md).

## What was verified

See the end of this page; it is updated with every release candidate.

### Verified (Docker, Ubuntu 24.04, no GPU, virtual display)

(filled in after the smoke test)

### Never verified

- A real desktop session (GNOME, KDE, Wayland or X11 with real hardware), a TV, a phone connecting from the network
  to a Linux PC, a gamepad, real audio output, real `/dev/uinput` injection into a focused application.
- Wayland (any compositor), multi-monitor placement, the tray on a desktop that shows it.
- Netflix and other DRM playback in Chrome, Brave or Vivaldi.
- AppImage execution on a distribution other than the build container, flatpak and snap browsers.
