# Universal search

Type a title on the TV (with the phone keyboard) and RusTV lists every place that has it and that it can open:

- **Where to watch**: the streaming services that offer the title in your country (Netflix, Prime Video, Disney+, Max, Canal+, Apple TV+, Arte, France.tv, YouTube and more). Enter on a service opens that app, already searching for the title.
- **Your libraries**: UGREEN Theater, Plex, Jellyfin, the local library, and the files found in the folders you listed (mapped network drives, UNC paths, any folder). Files play in the RusTV player.
- **On TV**: programmes from the automatic TV guide (off by default, see [LIVE-TV.md](LIVE-TV.md#tv-guide-automatic-epg)): what is on now or later today, with the channel. Enter plays the channel if the programme is on and the channel is one of yours; otherwise a small menu offers Remind me, Record and Show in guide. While the guide is off, a row offers to enable it.
- **Apps** and **Channels** (as before), plus a "Search in an app" row with one chip per app that has a search page.

Everything is driven by arrows, Enter and Back: up/down choose a row, left/right choose a service chip, Enter opens. Services that are not in your RusTV apps are shown dimmed ("not in RusTV") and cannot be selected.

## Setup

All settings are in **Settings > Universal search** (phone friendly: arrows, Enter, text typed on the phone).

### Where to watch (TMDB key)

There is no free keyless API that tells which service offers a title, so RusTV uses [TMDB](https://www.themoviedb.org/) (its availability data is supplied by [JustWatch](https://www.justwatch.com/)). You bring your own free key:

1. Create a free account on themoviedb.org, then Settings > API, and copy the **API Read Access Token** (a long token starting with `eyJ`, not the short "API key").
2. In RusTV: Settings > Universal search > *TMDB key*, paste it. It is stored encrypted with Windows (DPAPI), tied to your Windows account, and never shown again (the row says "saved").
3. *Country of the services* is automatic (FR for French, US for English); set a 2-letter code (FR, BE, CH, CA, GB...) to override.
4. *Test: TMDB* checks the key.

Without a key everything else still works and the search shows one discreet line explaining that a TMDB key enables "where to watch".

Attribution required by TMDB and JustWatch is shown under the results: "Data provided by JustWatch" and "This product uses the TMDB API but is not endorsed or certified by TMDB." The interface shows the text names; see "Not verified" for the logo.

### Plex

*Plex: address* (for example `http://192.168.1.10:32400`) and *Plex: token* (your `X-Plex-Token`, see Plex support "Finding an authentication token"). *Test: Plex* checks both. Results open the title page in the Plex tile (app.plex.tv, signed in with the tile's own profile). The token is sent only to the address you entered, in an HTTP header.

### Jellyfin

*Jellyfin: address* (empty = reuse the address you gave the Jellyfin tile) and *Jellyfin: API key* (Dashboard > API Keys). Results open the title page in the Jellyfin tile.

### UGREEN Theater

Theater has no documented search API, so RusTV cannot look inside it. If the UGREEN tile has an address ([UGREEN.md](UGREEN.md)), a row "Open UGREEN Theater" is offered; it opens the tile and you search there. To find the NAS files from RusTV itself, add the NAS media share as a folder (below). *Test: UGREEN* only checks that the NAS answers on its port.

### Mapped folders

*Add a folder (browse)* or *Add a path* (type `Z:\Films` or `\\NAS\Video`). The folders already used by the local library (Settings > Library) are indexed too. RusTV keeps a list of video, audio and photo file names:

- Background walk on a worker thread; each directory read has a 4 second budget, so an unplugged NAS or a dead mapped drive is skipped quickly and never freezes the interface (the stuck read is abandoned; Windows releases it later).
- Bounded: 12 levels deep, 60,000 files per folder, 100,000 in total, 10 minutes per folder. When a limit is hit the folder is marked "partial".
- Symbolic links and junctions are never followed (no loops). Hidden names, `$RECYCLE.BIN`, `System Volume Information`, `@eaDir`, `#recycle` are skipped.
- Refresh: 45 seconds after RusTV starts, when you open the search or the settings page, and with *Update the index now*; automatically never more than once an hour per folder. A folder that cannot be reached keeps its previous index (a NAS switched off does not empty the results).
- The index is a compact text file at `%APPDATA%\RusTV\search\index.tsv`, written atomically. *Clear caches* deletes it.

Titles come from the file names ("The.Movie.2019.1080p.BluRay.x264-GRP.mkv" gives "The Movie", 2019); the parent folder name is also matched. Matching ignores case, accents and punctuation.

## Privacy: what leaves the PC

| Data | Where it goes | When |
| --- | --- | --- |
| The text you typed | `api.themoviedb.org` (HTTPS, with your token) | Only if a TMDB key is set and the source is on, 350 ms after you stop typing, at most 4 requests per query (1 search + 1 per top-3 title) |
| Poster pictures | `image.tmdb.org` (loaded by the interface) | When TMDB results with posters are shown |
| The text you typed | Your own Plex / Jellyfin server | Only if configured and on |
| The text you typed | The streaming app's own site | Only when you press Enter on a service: it opens in that app's window like any tile |
| File names and folders | Nowhere | The index never leaves the PC |
| TV guide programmes | Nowhere on a keystroke: matched in the local copy of the guide | The guide itself is downloaded once a day when enabled (see LIVE-TV.md) |

No telemetry. Tokens are encrypted with Windows DPAPI in `%APPDATA%\RusTV\usearch.json`, are never logged, never returned to the interface and never sent anywhere except the service they belong to. TMDB answers are cached 24 hours in `%APPDATA%\RusTV\search\tmdb-cache.json` (the cache contains the titles you searched; *Clear caches* removes it). Each source can be switched off in the settings.

## How mods add providers

A mod can declare search providers in its manifest (`contributes.search`): a service name, the tile that opens it and a search URL template with `{q}`. See [MODS.md](MODS.md#search-providers) and the example `mods/search-internet-archive`. The built-in services are plain data in `crates/rustv-shell/src/search-providers.json`: one line per service (tile id, names used by TMDB, URL template). Adding a service is a one-line change; a unit test checks that every tile id exists in the catalog and every template is valid.

## Limits

- "Where to watch" is only as good as TMDB/JustWatch: some services are not listed, names differ ("Amazon Prime Video with Ads"); unknown names are shown dimmed.
- Service search pages are web pages that change without notice. When a template stops working the app opens its search page without results (or its home); fix the one line in the table.
- Not every service offers a search URL that works for a logged-out or TV-user-agent session; web apps open in their own window with their own profile.
- A Plex/Jellyfin server with a self-signed HTTPS certificate is not accepted by the search requests (use `http://` on the LAN or a valid certificate).
- No season/episode deep links: a series opens the series page or the search.
- The file index lists names only: no metadata, no posters; results are matched on file and folder names.
- 100,000 files in total; a very large NAS is partly indexed.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| "Where to watch" never appears | Add a TMDB key; run *Test: TMDB*; check the country; look for the message under the results ("TMDB key rejected", "TMDB unreachable") |
| A service is dimmed "not in RusTV" | Add or unhide its tile (Home > tile menu), or install the mod that provides it |
| Plex or Jellyfin missing | Run the matching *Test*; "token rejected" means the token/key is wrong; "unreachable" means address, port or HTTPS certificate |
| A folder shows "unreachable" | The drive or share did not answer within 4 s; connect the drive (Explorer), then *Update the index now* |
| A folder shows "partial" | A limit was hit; add a deeper sub-folder as its own folder |
| Files are not found | Check the folder is listed and indexed (the row shows a count); only video, audio and photo extensions are indexed |

## Not verified

This was developed without network access, a real TMDB key, Plex, Jellyfin, a NAS or a TV; the logic is unit-tested with sample payloads but nothing below was exercised against the real services:

- TMDB live requests (headers, rate limits) and the exact provider names JustWatch returns for each country.
- The search URL templates of every built-in streaming service (written from public knowledge of each site, not tested). Likely to need adjustments, especially Disney+, Max, Canal+, Apple TV+, Prime Video, TF1+, M6+, Pluto TV and Molotov.
- Plex deep link `https://app.plex.tv/desktop/#!/server/<machineId>/details?key=...` and the Plex `/hubs/search` payload shape; Jellyfin `/Items?searchTerm=` with `X-Emby-Token` and the `#!/details?id=...&serverId=...` link.
- Behaviour on a dead mapped drive or a sleeping NAS, and Windows junction handling during the walk.
- Opening a deep link in a tile whose web app ignores unknown paths (it may land on the home page).
- The official TMDB logo is not bundled: the interface shows the text "TMDB" and "JustWatch" with the required sentences. TMDB's brand guidelines ask for their logo; add the approved asset from themoviedb.org/about/logos-attribution before a public release.
