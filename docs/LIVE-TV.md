# Live TV, guide and recorder

RusTV ships no channel list. You bring your own sources; RusTV plays them with mpv, shows a TV guide and can record.

Recording is meant for personal use (time shifting of programmes you are entitled to watch). Check the law in your country before keeping or sharing recordings. RusTV does not handle DRM or encrypted channels and will not record them.

## Sources

Sources live in the preferences (`prefs.live.sources`): `{ id, name, m3u, xmltv, kind?, host?, port?, user?, password? }`. `kind` is `m3u` (default, existing behaviour), `hdhomerun` or `tvheadend`. Tuner sources are turned into an `{ m3u, xmltv }` pair by `tuner_build_source`, then loaded by the same `live_load` as any list.

- M3U: URL or local file (`.gz` accepted). XMLTV: URL or local file.
- URLs may carry `user:pass@`; the credentials are sent as HTTP Basic authentication, are removed from the URL given to the HTTP client, and are added to stream URLs of the same host:port only (never to another server).

### HDHomeRun
Settings > Recording > "Look for tuners" sends the HDHomeRun discovery broadcast (UDP port 65001, request type 0x0002, wildcard device type and id) and reads `http://<ip>/discover.json` and `http://<ip>/lineup.json`. You can also type the IP address. The lineup is written as an M3U in `%APPDATA%\RusTV\tuners\`, channels flagged DRM are skipped, stream URLs are the device's own (`http://<ip>:5004/auto/v<channel>`, MPEG-TS). HDHomeRun has no XMLTV: give an external XMLTV address when adding the source. The number of tuners limits simultaneous recordings.

### TVHeadend
Enter host, port (9981), user and password. Assumed URLs (check them on your server):
- playlist: `http://user:pass@host:9981/playlist/channels?profile=pass`
- guide: `http://user:pass@host:9981/xmltv/channels`
The user needs streaming and EPG rights. Stream URLs from the playlist get the credentials added.

Credential limitation: they are stored in clear text in the RusTV preferences (like other sources) and, for stream URLs, can appear in the mpv command line and in `dvr.json`. They are never written to logs. Use a dedicated read-only TVHeadend user.

## The guide
Live TV has three views, cycled with the "View" entry of the left column: list, guide, recordings.
- Guide: channels as rows, 3 hours visible, red "now" line, blocks sized by duration. Up/Down: channel. Left/Right: previous/next programme (the window scrolls by itself, and by 30 minutes when a channel has no guide). OK: watch if the programme is on now, otherwise the action menu. The Menu key (ContextMenu) opens the menu anywhere: Watch, Record, Record series (same title), Remind me, Record now, favourite. A red dot marks scheduled or running recordings. The bottom panel shows title, time and description.
- Groups and a Favourites entry (stored in the browser storage of the TV window, by channel name) filter the channels.
- Backend: `live_epg(fromTs, toTs, channelIds?)` returns `{ channels: [{ id, programs: [{ start, stop, title, desc, left, width, clipL, clipR }] }] }`. XMLTV times are converted to UTC honouring offsets (`+0100`, `+01:00`, `Z`, none = UTC). Programmes are sorted, de-duplicated by start, overlaps are clamped and empty ones dropped; 6 h past and 72 h future are kept in memory; a window query is a binary search. Windows are limited to 48 h.
- Reminders are shown while RusTV is running (kept in browser storage).
- A mini guide over the playing channel is not possible: mpv owns the screen in full screen. Pause and rewind during live use `player_timeshift(seconds)` and `player_go_live()` (player agent); `timeshift()` and `goLive()` wrappers are exported from `live.svelte.ts`.

## Recorder (DVR)
State: `%APPDATA%\RusTV\dvr.json`. Default folder: `Videos\RusTV Recordings`.
- Statuses: scheduled, recording, done, failed, cancelled. A scheduler thread checks every 2 seconds.
- Padding before/after (default 1 and 3 minutes), `maxParallel` simultaneous recordings (default 2). Scheduling beyond that shows a warning; at run time extra recordings wait for a free slot and are marked "missed" if their window ends first.
- Two methods, chosen per URL (`choose_method`):
  - HTTP(S) streams without a playlist/container extension (HDHomeRun, TVHeadend, most IPTV): copied as raw MPEG-TS to a `.ts` file by a thread, with reconnection when the stream drops. No external process.
  - Everything else (`.m3u8`, rtmp, rtsp, udp, srt...): `mpv --no-config --really-quiet --no-terminal --o=<file>.mkv --ovc=copy --oac=copy --of=matroska <url>`, started without a window. Dump mode (`--o`) was preferred to `--stream-record` because it does not need a playback window or audio output. At the end time RusTV asks mpv to quit over its IPC pipe so the Matroska is finalised, and kills only that child after 8 seconds.
- Disk: refuses to start below the free-space floor (default 2 GB) and checks during recording (the recording is stopped as failed, file kept).
- Retention (optional): keep at most N GB and/or N days of finished recordings (oldest removed first, files deleted).
- After a crash or forced exit, recordings that were running become `failed` (the file is kept) and, if the programme is still on, the rest is rescheduled as "Title (2)".
- Series: "Record series" stores a rule (title + channel name); every minute upcoming programmes with the same title in the next 48 h are scheduled.
- Files: `<channel> - <title> - <date>.ts|mkv`, characters illegal on Windows and reserved names are replaced, a number is added on collision.
- Recordings tab: status, size, free space; OK opens Play (kind `video`, so the resume logic applies) / Cancel / Delete (with confirmation, optionally with the file).

Commands: `dvr_list(tz?)`, `dvr_schedule(spec)`, `dvr_cancel(id)`, `dvr_delete(id, deleteFile)`, `dvr_get_config()`, `dvr_set_config(cfg)`, `dvr_start_now(channel, minutes)`, `dvr_series_add(rule)`, `dvr_series_remove(title, channelName)`, plus `tuner_discover()` and `tuner_build_source(spec)`. The `dvr-changed` event is emitted when the list changes.

## Limits
- No DRM, no encrypted channels (HDHomeRun DRM channels are skipped).
- The scheduler only runs while RusTV runs (tray icon is enough, TV mode is not needed). Missed recordings are not made up.
- Timezone in file names comes from the TV window; the stream clock is UTC.
- Not verified on real hardware at the time of writing: HDHomeRun discovery on a real network, TVHeadend URLs and auth, mpv dump mode on Windows, graceful quit over the pipe. Unit tests cover the packet format and parsing, scheduling decisions, naming and argument construction.

## Troubleshooting
- "mpv missing": play any channel once and accept the mpv download, or install mpv.
- Empty guide: the channel needs a matching `tvg-id` or the exact display name in the XMLTV file.
- HDHomeRun not found: broadcast UDP 65001 is blocked by a firewall or another subnet; type the IP address.
- Status "Tuner busy": all tuners are in use (HDHomeRun answers 503).
- "Access denied": check TVHeadend user rights and password.
- Recording of 0 bytes is deleted and shown as failed ("No data received").
