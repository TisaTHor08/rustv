# Roadmap

- [x] **0. Remote foundation**: protocol, service, phone app, pairing, Bluetooth, text-field detection, icons
- [x] **1. Repository**: license, docs, logo
- [x] **v0.2 TV mode**: Tauri shell, screen picker + black-out, home (hero + rows), web apps with per-app profiles and spatial navigation, native apps (VLC, Steam), autostart, phone swipe surface + haptics, Home/TV/media messages
- [x] **v0.3 Customization**: wallpapers (animated aurora, solid color, image, folder slideshow, looping video, blur/dim), tile editing (add website/Windows app, reorder, rename, color, icon, hide), dark/light/auto theme + accent color, optional weather (Open-Meteo), FR/EN
- [x] **v0.4 Mods**: manifest + permissions, themes, widgets, pages, tiles, site adapters, translations, sandboxed execution, in-app GitHub catalog with pinned commits, 5 example mods ([docs/MODS.md](MODS.md))
- [x] **v0.5 Live TV and library**: mpv player (on-demand install, IPC), live TV from M3U + XMLTV with now/next, local library (movies, shows, photos, music), file explorer, Windows settings shortcuts, now-playing on the phone (SMTC), universal search
- [x] **v1.0 Polish**: NSIS installer, update check via GitHub Releases, interface size and reduced motion, Bitwarden integration, YouTube in Brave or Vivaldi, UGREEN Theater tile, 42 app tiles, documentation
- [x] **v1.1 Remote and mod store**: Google TV style phone remote with PC volume sync and haptics, navigation sounds, real key presses and keyboard-layout aware typing, spatial navigation fixes for web apps, UGREEN Link verified, generated [APPS.md](APPS.md), mod store redesign (categories, search, local ratings, install and uninstall, publisher, dependencies, icon and banner) with catalog format v2 and `scripts/mods-tool.py`, 42 mods, `icons` contribution, mods for the phone remote (themes and button panels), see the [changelog](../CHANGELOG.md)
- [x] **v1.2 A complete TV**: boot to TV and TV guard, complete live TV (guide, DVR, tuners, timeshift), complete player (tracks, resume, continue watching, queue), DLNA/DIAL cast receiver, Hyperion ambilight, audio output / resolution / refresh / HDR settings, game controller, Home Assistant over MQTT, ultrawide-ready interface, see the [changelog](../CHANGELOG.md)

Ideas for later:

- Universal search across streaming services (with availability data), private libraries and mapped folders, extensible by mods (planned for v1.3).
- HDMI-CEC, per-profile gamepad layouts, voice dictation from the phone.
- Wake-on-LAN from the phone.
- More mod categories (for example subtitles, screensavers, sound packs).
- A signed installer and a release-signed APK (today the installer is unsigned and the APK is debug-signed).
- UGREEN Theater deep-linking: open Theater directly instead of the UGOS desktop (the exact URL is still unknown).
- Test the phone remote themes and panels on physical phones, and test more streaming apps with real accounts.

## Help wanted

Good first issues and exploration ideas. Open an issue before starting so we can discuss the approach
(see [CONTRIBUTING.md](../CONTRIBUTING.md)).

- **Site adapters**: small JS/CSS adapters (as mods) that make more streaming sites comfortable with a remote.
- **Translations**: more languages for the TV interface and the phone app (both tables are needed).
- **Mods**: themes, widgets, pages and tiles for the community catalog ([MODS.md](MODS.md)).
- **Gamepad support**: use a game controller to navigate the home screen.
- **Wake-on-LAN**: wake the PC from the phone app.
- **Accessibility review**: focus order, contrast, text sizes, screen-reader and reduced-motion behavior.
- **Tests and docs**: more unit tests in `rustv-protocol` and `rustv-shell`, clearer docs, screenshots.
- **Exploration, macOS and Linux ports**: what would a non-Windows shell and input layer need? A written findings
  issue is already a valuable contribution.
