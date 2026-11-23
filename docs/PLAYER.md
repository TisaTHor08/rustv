# Player

RusTV plays video, audio and live TV with mpv (launched full screen on the TV screen, controlled over its JSON IPC named pipe). The phone remote sends real key presses to the mpv window; RusTV writes an `input.conf` that maps them (`%APPDATA%\RusTV\player\<kind>.conf`) and a small Lua script (`rustv-menu.lua`, same folder) that draws the menus. Both files are regenerated at every playback start.

## Remote keys in the player

| Key on the phone | Video / audio | Live TV |
|---|---|---|
| Up | open the player menu (moves up when the menu is open) | next/previous channel (Up = previous) |
| Down | show progress (moves down in the menu) | next channel |
| Left / Right | seek -10 s / +10 s; hold the key: 30 s then 60 s | volume -5 / +5 |
| OK (Enter) | pause / resume (skips the countdown to the next episode) | pause / resume (timeshift) |
| Menu (context key, long press) | open / close the player menu | open / close the menu (audio and subtitle tracks, info) |
| Back | close the menu or sub-menu; otherwise stop playback | same |
| Next / Previous (media keys) | next / previous item of the queue | next / previous channel |
| Search | nothing | nothing |

In the menu: Up/Down choose, OK validates, Left/Right adjust values (subtitle delay and size, zoom), Back goes up one level.

## Menus

The menu is our own Lua script drawn with mpv's `osd-overlay` (no extra window). We did not use mpv's built-in selection menu (`select.lua`): it only exists in recent builds and is keyboard-text oriented. Ours needs `mp.create_osd_overlay` (mpv 0.31+).

Entries (video): audio track, subtitles (Off, each track, delay, size), speed (0.5 to 2), aspect ratio and zoom (Auto, 16:9, 4:3, 2.35:1, zoom, fill, reset), chapters (only if the file has some), sleep timer (15/30/60/90 min), information overlay. Audio kind: speed, sleep timer, information. Live: audio, subtitles, information.

Track labels: title, language, codec, channels (mono, stereo, 5.1, 7.1), forced/external.

## Resume and history

* mpv's watch-later (folder `player\watch_later`) keeps the exact position and the per-file audio/subtitle choice, delays, subtitle size and speed.
* RusTV keeps its own `history.json` (`url, title, kind, position, duration, updated, poster, finished`), updated at most every 15 s and on quit or item change. An item is finished at 92 % or more (or at end of file) and leaves the list. Videos under 10 s are not recorded. Audio is recorded only if longer than 10 minutes. Live TV is never recorded. 200 entries max, 20 shown.
* The Home screen shows a "Continue watching" row at the top when there is something to resume: card with poster, title, progress bar and time left. OK resumes (`--start`), the Menu key removes the card.
* Settings > Player > "Resume where I stopped" off: always start from the beginning (`--no-resume-playback`).
* Commands: `continue_list()`, `continue_remove(url)`, `continue_clear()`.

If the watch-later file and the history disagree, mpv's watch-later position wins (they differ by a few seconds at most).

## Queue and series

Starting an episode from a series queues the following episodes of the show. When a file ends, mpv stays open (`--idle=yes`) and the screen shows "Next: <title> in 5 s (Back to cancel)". Back stops the player, OK plays the next one immediately. At the last item playback ends and the app returns home. Next/Previous media keys move in the queue without a countdown. Option "Auto-play next episode" (default on); when off mpv quits at the end of the file.

## Live timeshift

For live the cache is enabled with `--cache=yes --demuxer-max-bytes=<N>MiB --demuxer-max-back-bytes=<N>MiB --cache-secs=7200` (N = 256 by default, 64 to 1024 in settings). Pausing a live channel keeps buffering; `PlayerState` (`player_state` command, `player-state` event) exposes `timeshift_available_secs` (seconds buffered behind the current position) and `behind_live_secs` (0 = live), polled from `time-pos`, `demuxer-cache-time` and `demuxer-cache-state`. The OSD shows "LIVE" or "-2:34" on pause and seek.

Commands: `player_pause_toggle()`, `player_timeshift(seconds)` (relative, negative = back, clamped to the cache; returns the new delay; errors `not_live`, `nothing_buffered`, `already_live`, `no_ipc`), `player_go_live()`, `player_set_track(kind, id)`, `player_menu()`.

## Configuration (`player.json`)

`player_get_config()` / `player_set_config(cfg)`: `audio_lang`, `sub_lang` (`auto` follows the interface language then English, or fr/en/de/es/it/ja), `subs_default` (off by default), `auto_next`, `resume`, `timeshift_mb`. Translated into `--alang`, `--slang`, `--sid=no`. Hardware decoding stays `--hwdec=auto-safe`. All of it is in Settings > Player.

## Not verified

mpv, the Lua script and the pipe were never run during development (unit tests only). Things to check on a real machine: the key names `MENU`, `NEXT`, `PREV` produced by the phone's keys, menu rendering and Lua compatibility with the installed mpv build, `--idle=yes` queue behaviour at the end of a file, the real depth of the timeshift cache on a given stream (many live streams are not seekable backwards), and `--watch-later-options` support in older mpv builds.
