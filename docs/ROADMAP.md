# Roadmap

- [x] **0. Remote foundation**: protocol, service, phone app, pairing, Bluetooth, text-field detection, icons
- [x] **1. Repository**: license, docs, logo
- [x] **v0.2 TV mode**: Tauri shell, screen picker + black-out, home (hero + rows), web apps with per-app profiles and spatial navigation, native apps (VLC, Steam), autostart, phone swipe surface + haptics, Home/TV/media messages
- [x] **v0.3 Customization**: wallpapers (animated aurora, solid color, image, folder slideshow, looping video, blur/dim), tile editing (add website/Windows app, reorder, rename, color, icon, hide), dark/light/auto theme + accent color, optional weather (Open-Meteo), FR/EN
- [x] **v0.4 Mods**: manifest + permissions, themes, widgets, pages, tiles, site adapters, translations, sandboxed execution, in-app GitHub catalog with pinned commits, 5 example mods ([docs/MODS.md](MODS.md))
- [x] **v0.5 Live TV and library**: mpv player (on-demand install, IPC), live TV from M3U + XMLTV with now/next, local library (movies, shows, photos, music), file explorer, Windows settings shortcuts, now-playing on the phone (SMTC), universal search
- [ ] **v1.0 Polish**: installer, update check via GitHub Releases, accessibility, documentation

Ideas for later: gamepad support, per-profile layouts, voice dictation from the phone, Wake-on-LAN.

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
