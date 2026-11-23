# UGREEN Theater in RusTV

Research notes (2026-11) and what the `ugreen` tile does. No UGREEN NAS was available during development: anything marked **unverified** must be tested on a real device.

## What Theater is

- Theater is the video app of UGOS Pro, UGREEN's NAS operating system. It is installed from the UGOS Pro App Center, links media folders into libraries, identifies titles from file names, fetches posters and metadata, and syncs watch state across UGREEN clients (desktop, mobile, TV). Sources: [joween.dev comparison](https://joween.dev/blog/ugreen-theater-or-jellyfin-on-a-ugreen-nas) (citing the official Theater guide), [UGREEN.FORUM Theater board](https://ugreen-forum.de/forum/board/40-theater/).
- A **web client exists**: forum threads mention playing through Theater "im Webbrowser" (a firmware update broke it for one user), and a TechPowerUp review notes that NAS video can be played in a browser with fewer options than in the apps and that the Windows app looks like a wrapper around the web UI ([review](https://techpowerup.com/review/ugreen-nasync-dxp4800-plus/8.html)). This is the basis for the tile.

## Addresses and ports

| Item | Finding | Status |
|---|---|---|
| UGOS web UI, HTTP | `http://<nas-ip>:9999` | Reported by igorslab reviews and user posts; the port is configurable (Control Panel > Device Connection > Portal) ([guide](https://mariushosting.com/how-to-enable-https-on-your-ugreen-nas/)) |
| UGOS web UI, HTTPS | `https://<nas-ip>:9443`, certificate not trusted by browsers by default | Third-party sources ([TechPowerUp](https://techpowerup.com/review/ugreen-nasync-dxp4800-plus/7.html)); self-signed |
| Direct URL of the Theater page inside the UI | unknown | **Unverified.** Theater is opened from the UGOS desktop (app launcher), so the tile opens the portal root and the user opens Theater once |
| `find.ugnas.com` / UGREEN Link relay | UGREEN-run discovery and remote access service; fails on direct PC-to-NAS cables | [UGREEN](https://ai.ugreen.com/blogs/how-to/fix-nas-no-device-found-error); requires UGREEN cloud, not used |
| Jellyfin on UGOS | Docker app; UGREEN docs and users use 8096 / 8899 | Different product from Theater; unrelated to the tile |
| DLNA | App Center package, LAN only, no poster wall or subtitle management | [source](https://my1396.github.io/Econ-Study/2025/08/14/Ugreen-Media-Streaming.html) |
| WebDAV / SMB | Media folders can be exposed by WebDAV (ports 5005/5006 mentioned) or SMB | [UGREEN WebDAV guide](https://ai.ugreen.com/blogs/how-to/access-ugreen-nas-files-via-webdav); RusTV can already play SMB paths through its local library |

## Not verifiable from public sources

- Login model (cookies vs tokens): not documented. RusTV keeps a persistent WebView2 profile per tile (`%APPDATA%/RusTV/profiles/ugreen`), so whatever cookie or localStorage session UGOS issues survives restarts, subject to its own expiry. Test: log in, close, relaunch.
- Supported browsers, codecs/DRM, subtitle behaviour in the web client: not documented. WebView2 is Chromium (H.264/AAC, VP9, AV1 via system; HEVC only with the Windows HEVC extension), so HEVC/DTS content may need server-side transcoding or the Theater desktop client. No DRM is involved.
- TV-friendliness of the web UI: it is a mouse/touch desktop UI. The generic arrow-key spatial navigation (`spatnav`) is enabled; its quality there is untested. Phone-remote pointer/click mode remains the fallback.
- LAN discovery: UGREEN only documents `find.ugnas.com` and says UPnP/Bonjour/mDNS "can help"; the mDNS service type and hostname are not published. **Discovery was therefore not implemented** (a guess could silently find nothing). To make it possible, run `avahi-browse -a` or Wireshark against a NAS and share the service names.

## What RusTV does

- Tile `ugreen`: asks once for the NAS address (hint `http://192.168.1.20:9999`), stores it, opens it in a dedicated window with its own persistent profile.
- `allow_insecure_lan` (catalog field, enabled for this tile): for an `https://` address whose host is a private IP (RFC 1918, link-local, loopback, IPv6 ULA), a `.local` name or a dotless name, the window is started with `--ignore-certificate-errors`. Public hostnames (for example `*.ugnas.com`) never get it. Plain `http://` needs no option.
  - Risk: certificate errors are ignored for **everything** loaded in that window, including links that leave the NAS. Do not log in to other sites in this window. Preferred alternative: install a proper certificate on the NAS or use `http://`.
- Logic for host classification lives in `crates/rustv-shell/src/lan.rs` (unit tested).

## Test checklist on a real NAS

1. Tile asks for the address; `http://<ip>:9999` loads the UGOS login.
2. Login persists after closing RusTV (relaunch the tile without re-login).
3. `https://<ip>:9443` loads without a certificate error page.
4. Open Theater, check arrow-key navigation and Enter; check playback, fullscreen, subtitles, and HEVC/Dolby content.
5. Report the exact URL of the Theater page so it can be deep-linked.

## Remote access through UGREEN Link (verified 2026-11-23)

A real NAS was reachable from outside the LAN through the UGREEN Link relay:

- Address form: `https://<ugreenlink-id>.<region>.ug.link/` (for example region servers named `eur3`, `eur6`, `eur81`). The **region matters**: on a wrong region the page loads but says "Failed to connect ... UGREENlink ID not found"; on the right one it redirects to `/desktop/` and shows the UGOS login ("Sign in to <NAS name>"). The region is the one shown in the UGREEN desktop/mobile app for that device.
- The tile accepts this address (hint `https://mon-nas.eur81.ug.link`); it is a public HTTPS host with a valid certificate, so `allow_insecure_lan` does not apply.
- The login page is a Vue/Arco UI with real `<input>` and `<button>` elements. The **Sign in button is disabled until both fields are filled**.
- Tested with the phone remote protocol: arrow keys move between the username field, the password field and the Sign in button; text typing and OK (Enter) work; the NAS answered the login attempt.
- **Not verified**: a successful login, the UGOS desktop, opening Theater and playback (the credentials supplied for the test were rejected by the NAS: "Incorrect account or password").

### Remote-control compatibility (`spatnav.js`)

Fixed while testing this page, useful for every web app:

- Up/Down now leave a single-line text field; Left/Right leave it when the caret is at the edge (multi-line fields keep native behaviour).
- Disabled elements are skipped, and the next candidate is tried if an element refuses focus.
- Clickable elements that are not links or buttons (a `<div>` with a pointer cursor, common in NAS dashboards) are reachable and activated with Enter.
