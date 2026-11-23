# Live TV, guide and recorder

RusTV ships no channel list. You bring your own sources; RusTV plays them with mpv, shows a TV guide and can record.

Recording is meant for personal use (time shifting of programmes you are entitled to watch). Check the law in your country before keeping or sharing recordings. RusTV does not handle DRM or encrypted channels and will not record them.

## Watching national TNT channels without an account

With a TNT tuner adapter and an antenna, the national French channels (TF1, France 2, France 3, France 5, M6, Arte and the others) are received live, for free, with no account and no internet connection. The adapter can be:

- a network tuner such as an HDHomeRun (it exposes the channels on your LAN), or
- a USB DVB-T/T2 tuner plugged into a PC or NAS that runs TVHeadend (TVHeadend serves the channels on the LAN).

RusTV already supports both: discovery and source setup are in Settings > Recording (see "Sources" below), recording uses the DVR, and pause and rewind use timeshift. Combined with the TV guide ([below](#tv-guide-automatic-epg)), a programme found in the universal search ("On TV") can be played, reminded or recorded directly, because the guide channels are matched to your tuner channels.

Honest limits: RusTV's tuner support is verified by unit tests only (packet formats, playlist parsing, scheduling), not with real hardware. Which channels you receive, and how well, depends on your antenna and your location (signal strength, multiplex coverage). Without a tuner or another source the TV guide still works, read-only: you can browse and search it, set reminders, but not watch or record.

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

## TV guide (automatic EPG)

RusTV can fetch a free TV programme guide by itself, so the guide, the "On TV" search group and reminders work with no M3U source and no account. It is **off by default**.

### Turning it on
- Settings > TV guide > "Automatic TV guide", or press Enter on the "Enable the TV guide" call-to-action shown in the Live TV guide view and in the search overlay while the guide is off. A confirmation names the host that will be contacted before anything is downloaded.
- Once on, the Live TV "guide" view shows the guide channels even if you have no source. A source that brings its own XMLTV keeps priority in that view; the automatic guide fills the "now / next" lines of the list view for sources that have no guide.

### Source and licence status
- Default: `https://xmltvfr.fr/xmltv/xmltv_tnt.xml.gz` (project XML-TV-Fr): about 0.9 MB compressed, about 30 French TNT channels, about 10 days, updated daily.
- Presets in the settings page: "TNT, 30 channels" (default), "France, about 700 channels" (`xmltv_fr.xml.gz`, about 9 MB, asked for confirmation), and a custom address (https; plain http only for LAN hosts such as `192.168.x.x`, `*.local` or a single-label host name; no credentials in the address; 2000 characters at most; gzip or plain XML).
- The data is third-party scraped data with NO explicit licence. RusTV ships none of it and only downloads it when you turn the feature on. If you need clear rights, use your own XMLTV source instead (a source with an XMLTV address, or a TVHeadend guide).

### Privacy
Only a plain HTTP GET to the chosen address is made: no cookie, no identifier, User-Agent `RusTV`. The site is asked for the compressed file (`Accept-Encoding: gzip`) and conditional headers (`If-None-Match`, `If-Modified-Since`) are sent when a cache exists. Nothing about your channels, searches or viewing leaves the PC. Programme images (logos, posters) are loaded by the interface directly from the hosts named in the guide, https only, failures are silent; the "Guide images" setting turns them off.

### Caching and refresh
- The downloaded file is kept as received in `%APPDATA%\RusTV\epg\guide.cache` and the configuration in `%APPDATA%\RusTV\epg.json`.
- At start (after 30 s), then hourly while RusTV runs, the guide is refreshed only if the cache is older than 24 hours, and never more than once per 6 hours, including after a failure (backoff). If nothing has ever succeeded, a retry waits 2 minutes. "Refresh now" in the settings page is limited to once per 2 minutes. A `304 Not Modified` answer renews the cache without a new download.
- Changing the source erases the cache.

### Limits
Download at most 40 MB compressed, 200 MB decompressed (the file is parsed as a stream, never loaded as text). Kept: programmes from 3 hours ago to 10 days ahead, at most 200 channels (the ones that match your live channels first, otherwise the order of the file) and 100,000 programmes; titles are cut at 200 characters and descriptions at 600. The channels kept are chosen when the file is parsed: with the 700-channel preset, add your live source first, then "Refresh now", so that your channels are preferred.

### Matching with your channels
Guide channels are linked to your live channels (M3U, HDHomeRun, TVHeadend) by `tvg-id` first, then by a normalised name (lower case, no accents, no punctuation or spaces, without ".fr", "HD", "FHD", "UHD", "SD", "4K"; a few aliases such as "Canal+" and "Canal Plus"). So "France 2", "France2.fr" and "France 2 HD" are the same channel, and so are "BFM TV" and "BFMTV.fr". A guide channel with a match can be watched, recorded and reminded; one without is read-only.

### Searching and using it
- Universal search shows an "On TV" group (up to 8 programmes): channel logo and name, title, time ("Now, until 21:30", "Today 20:55", "Tomorrow 14:10") and category. Search is local (no network per keystroke), ignores case and accents, ranks title starts and whole words before substrings, puts programmes on air first and then the soonest, merges back-to-back repeats and skips placeholders such as "Programmes de la nuit" (category "Programme indetermine"), which stay visible in the guide grid. Enter on a programme that is on air and playable plays the channel; otherwise an action menu offers Remind me, Record (if playable) and Show in guide.
- Live TV > View > guide: left column entries "Tonight" (20:50 today, tomorrow after 22:30), "Previous day" and "Next day" (also PageUp / PageDown), limited to the cached range. The details panel shows title, time, duration, category, rating, description and the picture. The action menu (OK on a programme that is not on air, or the Menu key) offers Watch, Record, Record series, Remind me, Record now and favourite when the channel is playable.
- Commands: `epg_get`, `epg_set`, `epg_refresh(force?)`, `epg_channels`, `epg_guide(fromTs, toTs, channelIds?)`, `epg_search(q, limit)`, `epg_tonight(tz)`, `epg_live_nownext`; `usearch_query` also returns `programmes` and `epg` ("off" / "empty" / "ok"). Errors are short codes (`disabled`, `too_soon`, `busy`, `network`, `http_<status>`, `too_large`, `bad_xml`, `empty`, `write_failed`, `bad_url`, `url_too_long`, `http_not_lan`).

### Troubleshooting
- "Nothing loaded" with the guide on: look at the "Last error" row in Settings > TV guide; "network" means the host is unreachable, `http_404` that the address changed, `bad_xml` that the file is not XMLTV.
- A programme cannot be watched: no live channel matches. Give the channel a `tvg-id` equal to the guide id (for example `TF1.fr`) or rename it like the guide.
- Times look off: XMLTV times carry their own offset (`+0100`, `+0200`); they are converted to UTC and shown in the local time of the TV window.

### Not verified
The real files (`xmltv_tnt.xml.gz`, `xmltv_fr.xml.gz`) were not downloaded while developing this feature: no network request was made. Parsing, caching rules, matching and search are unit-tested on a small embedded sample with TNT-like channel ids and `+0100` / `+0200` offsets; the real channel names, ids, icon hosts, ETag support and file sizes were not checked here. Not tested either: the interface on a real TV, and the behaviour of the site's servers under redirects or rate limiting.

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
