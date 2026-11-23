# Changelog

All notable changes to RusTV are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **Phone remote models** ([docs/REMOTE-MODELS.md](docs/REMOTE-MODELS.md)): the phone remote can take the shape of the remote that ships with the chosen TV style (Google TV, Samsung, LG, Hisense, Philips, Fire TV, Apple TV, Roku).
  The PC announces its TV style in `state.osStyle`; the phone follows it automatically, or you pick a model in the phone settings.
- **Tizen and webOS interface styles** ([docs/OS-STYLES.md](docs/OS-STYLES.md)): a Samsung-like launcher bar with previews and an ambient clock mode, and an LG-like rotating banner with a wide-card launcher (Inter and Noto Sans, OFL, bundled).
- **Interface styles** ([docs/OS-STYLES.md](docs/OS-STYLES.md)): Settings > Appearance > Interface style switches the home screen live between the original RusTV look and recreations of the Google TV and Roku home screens (layout, motion, palette, radius and font also re-skin the other screens), plus a framework to add more styles with two files, and a Widget position setting (auto, four corners, hidden).
- **VIDAA and Titan interface styles** ([docs/OS-STYLES.md](docs/OS-STYLES.md)): two more home screens, a bright shortcut-strip-and-banner layout inspired by Hisense VIDAA and an icon bar with hero carousel and square app tiles inspired by Philips Titan OS (Poppins font added, OFL).
- **Interface styles** ([docs/OS-STYLES.md](docs/OS-STYLES.md)): Settings > Appearance > Interface style switches the home screen live between the original RusTV look and recreations of the Google TV, Roku, Fire TV and Apple TV home screens (layout, motion, palette, radius and font also re-skin the other screens), plus a framework to add more styles with two files, and a Widget position setting (auto, four corners, hidden).
- **Linux release (experimental)** ([docs/LINUX.md](docs/LINUX.md)): real Linux backends for phone key and mouse injection
  (`/dev/uinput`, X11 and Wayland), system volume and audio outputs (`wpctl`/`pactl`), installed apps and icons (XDG `.desktop`),
  now playing and play/pause (MPRIS), secrets in the desktop keyring (Secret Service, never plain text), autostart,
  free disk space, mounted drives, a `.deb` and an AppImage built in Docker (`scripts/build-linux.ps1`), a Docker smoke test
  on a virtual display, and a `platform_features` command so settings pages can hide features that are stubs.
  Netflix, Prime Video, Disney+ and other DRM tiles open in Chrome, Brave, Vivaldi or Chromium in app mode (WebKitGTK has no Widevine).
  Not available on Linux yet: game controller, TV guard, display modes, HDR, Bluetooth remote. Never tested on a real desktop.
- **Universal search** ([docs/SEARCH.md](docs/SEARCH.md)): type a title and see which streaming services have it (with a free TMDB key, data from JustWatch) and open them
  directly, plus matches in your private libraries (UGREEN Theater, Plex, Jellyfin, the local library) and in mapped network drives and folders you choose.
  Mods can add their own search sources with `contributes.search`.
- **Launch animation for web apps**: the app window stays hidden until its page has loaded, the home screen shows an animation, ignores clicks and keys while it waits (Back cancels), and a slow site gets a hint after 8 s and a 25 s limit. This also removes the empty window through which the home screen used to show.
- **Automatic TV guide** ([docs/LIVE-TV.md](docs/LIVE-TV.md)): an optional, off-by-default programme guide (XML-TV-Fr, French TNT) that needs no source or account, with day navigation and a "Tonight" jump in Live TV, an "On TV" group in universal search, reminders, and playing or recording when the channel matches one of your tuner or M3U channels. Third-party data with no explicit licence; one download per day at most.
- **Web tiles accept several addresses** (for example the NAS local address followed by its UGREEN Link address): the first one that answers is used.

## [1.2.0] - 2026-11-23

### Added

- **Boot to TV and TV guard** ([docs/TV-GUARD.md](docs/TV-GUARD.md)): RusTV can start at sign-in straight into TV mode on the right screen
  (it waits up to 60 s for the TV to wake), keeps the PC awake, and stops other windows from appearing over the TV
  (minimize or move, with an allow list). Hiding the taskbar and muting Windows notifications are optional and always restored,
  including after a crash. A panic row in Settings undoes everything.
- **Complete live TV** ([docs/LIVE-TV.md](docs/LIVE-TV.md)): a program guide grid, reminders, favourites, a DVR (record now, record a
  programme, record a series, retention and free-space rules), HDHomeRun discovery and TVHeadend sources, and timeshift in the player.
- **Complete player** ([docs/PLAYER.md](docs/PLAYER.md)): audio and subtitle track menu, resume where you stopped, a Continue watching row
  on the home screen, a queue with automatic next episode, and a live-TV pause and rewind buffer.
- **Cast receiver** ([docs/CAST.md](docs/CAST.md)): receive YouTube (DIAL) and media (DLNA/UPnP) from a phone or another PC. Off by default.
- **Ambilight with Hyperion** ([docs/AMBILIGHT.md](docs/AMBILIGHT.md)): RusTV can install and run Hyperion, light the LEDs with the focused tile,
  and hand over to screen capture during video. Off by default.
- **Screen and sound** ([docs/DISPLAY-AUDIO.md](docs/DISPLAY-AUDIO.md)): choose the audio output and its volume, the resolution, the refresh rate and HDR,
  with a 15 second automatic revert.
- **Game controller** ([docs/GAMEPAD.md](docs/GAMEPAD.md)): XInput pads drive the home screen, apps and the player, with haptic feedback,
  Alt+Tab on the bumpers and a pointer mode. It only acts on RusTV and the programs it launched.
- **Home Assistant** ([docs/HOME-ASSISTANT.md](docs/HOME-ASSISTANT.md)): MQTT discovery exposes RusTV as a device (state, now playing, volume, current app,
  buttons, app launcher) and accepts commands (launch, volume, open an app, key presses). A ready-made package and Lovelace card are included.
- **Settings, Integrations and hardware** groups the new pages. They are all usable with the remote.

### Changed

- The interface scales with the smaller of the screen height and width, so it fits ultrawide, 16:10, 4:3 and portrait screens as well as 16:9.

### Notes

- Hyperion and Home Assistant are built-in modules, not mods: mods cannot start processes or open sockets.
- The new code that touches hardware or the network (display and audio APIs, XInput, SSDP, MQTT, Hyperion, tuners) is covered by unit tests of the
  logic only. Each documentation page lists what has not been verified on real hardware.

## [1.1.0] - 2026-11-23

### Added

- **Phone remote redesigned in the Google TV style**: a four-way ring with an OK button, round Back, Home, Search and Tabs
  buttons, a volume bar and haptic feedback on every press. The ring is the default; the swipe surface is one tap away.
- **PC volume sync**: the phone's volume bar follows the Windows volume (and mute), and the phone's hardware volume keys
  control the PC. New protocol messages `setVolume` (phone to PC) and `volume` (PC to phone), backed by a new `audio` module in
  `rustv-core` (Windows Core Audio).
- **Navigation sounds** in the TV interface: short synthesised sounds for move, select and back, with a switch and a volume
  setting in Settings.
- **Mod store redesign** (Settings, Mods, Online catalog): categories, search (name, publisher, tag, description),
  install and uninstall, publisher, dependencies (other mods or external programs), an icon and a banner per mod,
  a personal star rating stored only on your PC, and 5 cards per row.
- **Catalog format v2** in `registry/index.json`, generated by the new `scripts/mods-tool.py` (`check`, `art`, `registry`)
  from the metadata in each `mod.json`. See [docs/MODS.md](docs/MODS.md).
- **42 mods**, all listed in the online catalog. They cover themes, widgets, pages, games,
  tile packs, enhancements (site adapters), icons and the phone remote.
- **Mods for the phone remote**: a mod can contribute remote themes (CSS variables and optional CSS) and button panels
  (sequences of allowed actions) through `contributes.remote`. The PC sanitizes the bundle, the phone validates it again,
  and panels appear in a new **Mods** tab. New protocol messages `getRemoteMods`, `remoteMods` and `openTile`.
- **"icons" mod contribution** and the **Real icons** mod, which replaces the letter on tiles with app logos.
- **Generated app list**: [docs/APPS.md](docs/APPS.md), built by `scripts/gen-apps-doc.py` from `catalog.json`, with a test
  status for each app.
- **UGREEN Link remote access** (`https://<id>.<region>.ug.link/`) verified end to end: login with two-factor step, Theater
  library and film playback. See [docs/UGREEN.md](docs/UGREEN.md).
- Repository banner and social preview image (`assets/banner.png`), new screenshots, and a
  [user guide](docs/USER-GUIDE.md).

### Changed

- **Keyboard typing** from the phone now uses the keyboard layout of the focused window, so accented and layout-specific
  characters come out right.
- **Spatial navigation** (arrow keys in web apps) was reworked from real-world pages:
  - Up and Down leave a single-line text field; Left and Right leave it when the caret is at the edge.
  - Disabled elements are skipped, and the next candidate is tried if an element refuses focus.
  - Clickable elements that are not links or buttons (a `div` with a pointer cursor) are reachable and activated with Enter.
  - Hidden checkboxes use their visible label as the focus target.

### Fixed

- The phone app now declares the Android `CAMERA` permission, so the QR scanner can ask for it.

### Verification notes

- The phone remote theme and panels were verified with the real PC protocol and a browser rendering, not yet on a
  physical phone.
- Netflix, Prime Video and most streaming apps need an account and have not been tested. See [docs/APPS.md](docs/APPS.md).

## [1.0.0] - 2026-11-23

First stable release.

### Added

- **TV mode** (Tauri and WebView2): screen picker with optional black-out of other displays, hero and rows home screen,
  clock, phone status, settings, start with Windows.
- **App catalog of 42 tiles** (video, live TV, music, cloud gaming, libraries) opening in their own WebView2 window with
  one isolated profile per app, plus native tiles (VLC, Steam). Arrow-key spatial navigation is injected into sites
  without a TV mode.
- **YouTube in Brave or Vivaldi** when installed (ad blocker), in a dedicated profile, configurable in Settings, Apps.
- **Customization**: wallpapers (aurora, color, image, slideshow, looping video, blur, dim), dark, light or automatic theme,
  accent color, French and English, optional weather, tile editing (rename, recolor, icon, move, hide, add websites and
  Windows apps), interface size and reduced motion.
- **Mods** (themes, tiles, pages, widgets, site adapters, translations) with declared and user-approved permissions,
  sandboxed execution and a catalog that installs pinned GitHub commits.
- **Live TV and library**: your own M3U playlists and XMLTV guides with now and next, local library of movies, shows,
  photos and music, file explorer, Windows settings shortcuts, universal search, mpv player installed on demand with a
  checksum, now playing on the phone.
- **Bitwarden integration** through the official `bw` CLI. See [docs/BITWARDEN.md](docs/BITWARDEN.md).
- **UGREEN Theater tile** with a stored NAS address and `allow_insecure_lan` for private HTTPS hosts.
- **Phone remote** (Android, Svelte and Capacitor): QR pairing, swipe surface, D-pad, Back and Home, tab switching, media
  keys, touchpad, keyboard that appears when a text field has focus, now playing card, French and English.
- **Security**: HMAC-SHA256 mutual authentication, AES-256-GCM encrypted frames, per-phone revocable pairing, no telemetry.
  See [docs/PROTOCOL.md](docs/PROTOCOL.md).
- **Installer** (NSIS, per user), **update check** against GitHub Releases, contributor documentation.
- Text typed from the phone is injected as real key presses (YouTube TV ignores Unicode injection).

[1.1.0]: https://github.com/TisaTHor08/rustv/releases/tag/v1.1.0
[1.0.0]: https://github.com/TisaTHor08/rustv/releases/tag/v1.0.0
