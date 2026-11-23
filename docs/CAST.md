# Cast and DLNA

RusTV can receive media pushed from a phone or an app. It is **off by default** because it opens a network port.
Switch it on in Settings > "Cast and DLNA" (or the phone-friendly settings page). It can be switched off the same way at any time.

## What works

| Source | How | Status |
| --- | --- | --- |
| YouTube phone app (cast button) | DIAL: RusTV opens its `youtube` tile on the YouTube TV page | implemented, not tested with a phone |
| DLNA "play to" apps: VLC, BubbleUPnP, Kodi, Plex/Jellyfin DLNA push, other UPnP controllers | UPnP MediaRenderer: AVTransport, RenderingControl, ConnectionManager | implemented, not tested with a real controller |

Supported DLNA actions: SetAVTransportURI (with DIDL-Lite title, MIME type and class), Play, Pause, Stop, Seek (REL_TIME / ABS_TIME),
Next/Previous (no-ops), GetTransportInfo, GetPositionInfo, GetMediaInfo, GetCurrentTransportActions, GetDeviceCapabilities,
GetTransportSettings, SetVolume/GetVolume/SetMute/GetMute (mapped to the PC master volume), GENA eventing (LastChange with
TransportState, CurrentTrackURI, Volume). Playback uses RusTV's mpv player, so mpv must be installed (RusTV offers to download it).

Limits:
- Video and audio only. Pictures (`imageItem`, `image/*`, `.jpg`...) are refused with UPnP error 714.
- Only `http://` and `https://` media URLs are accepted. `file://`, local paths and URLs with embedded credentials are refused.
- Mute is simulated (volume set to 0, then restored) because the audio layer has no mute call.
- One track at a time: Next/Previous do nothing, no playlist queue.
- Windows "Cast to device" (Win+K, Miracast/Play To) is **not** what this feature serves.

## What does not work

- The Chromecast protocol (CASTV2) and the Google Cast SDK: apps that cast only that way (Netflix, Disney+, Spotify Connect over Cast...) need Google
  registration and keys. Not supported.
- AirPlay: not supported.
- Netflix DIAL: not declared. RusTV answers 404 for `/apps/Netflix`, because it could not be implemented truthfully.

## How it works

- **SSDP**: a socket on 239.255.255.250:1900 (SO_REUSEADDR, so it coexists with the Windows SSDP service) answers M-SEARCH for `ssdp:all`,
  `upnp:rootdevice`, the device UUID, `urn:schemas-upnp-org:device:MediaRenderer:1`, the AVTransport, RenderingControl and ConnectionManager services
  and `urn:dial-multiscreen-org:service:dial:1`. It sends NOTIFY `alive` every 15 minutes (CACHE-CONTROL max-age=1800), twice at start and when a network
  appears, and `byebye` when stopped. `LOCATION` uses the PC address on the subnet of the requester.
- **HTTP** on port **47840**, bound only to the private IPv4 addresses of this PC (10/8, 172.16/12, 192.168/16), never to `0.0.0.0` or a public address.
  Requests from non-LAN sources are refused. Bodies are capped at 256 KB, at most 16 simultaneous connections, no chunked uploads.
  Endpoints: `/dd.xml` (device description and `Application-URL: http://<ip>:47840/apps/` header), `/apps/YouTube`, `/dlna/<avt|rc|cm>/{scpd.xml,control,event}`.
- **DIAL** `/apps/YouTube`: GET returns `running`/`stopped`, POST launches, DELETE (on `/apps/YouTube` or `/apps/YouTube/run`) stops.
  Browsers cannot trigger it: POST/DELETE with an `http(s)` `Origin` header are refused (CSRF protection).
- **Launch parameters**: the POST body (for example `pairingCode=...&theme=cl&v=...&t=...`, limited to URL-safe characters, 4 KB) is appended to
  `https://www.youtube.com/tv?` and opened in the `youtube` tile (same browser choice as a normal launch).
- **GENA**: SUBSCRIBE/UNSUBSCRIBE with TIMEOUT clamped to 60..1800 s. The callback must be an `http://` URL whose host is the subscriber's own LAN address
  (no requests to third parties). Max 16 subscriptions, 20 new ones per minute; unreachable subscribers are dropped after two failures.
  Notifications are sent with 1-2 s timeouts, no redirects.
- XML is parsed with quick-xml; any DOCTYPE (hence any entity definition) is rejected. No shell command is ever run. Nothing sensitive is logged.
- The device UUID is generated once and stored in `%APPDATA%\RusTV\cast.json` with the settings (`enabled`, `name`).

### Verified and inferred

Verified by unit tests (no phone and no network were used): SSDP request parsing and response/NOTIFY text, DIAL XML and headers, SOAP parsing and
responses, DIDL-Lite parsing, REL_TIME conversion, GENA header parsing, LAN-only checks, URL validation, routing with a fake host.

Written from memory of the specifications and **not verified against real devices**: the exact YouTube DIAL behaviour. In particular that the phone app
accepts a device whose `/apps/YouTube` returns the DIAL 1.7 document; that the Leanback app (`youtube.com/tv`) picks up the pairing/cast request from the launch
parameters in the query; and that `<additionalData>` (screenId) is not needed (RusTV does not return it, the TV page registers it with YouTube itself).
Also unverified: interoperability with particular controllers, and the behaviour when the ad-blocking browser is used for YouTube (RusTV cannot
track that window, so `state` is "running" only for 15 s after a launch; each cast request relaunches the page).

## Firewall and privacy

The first time it is switched on, Windows may show a firewall prompt for `rustv.exe`: this is expected. Allow **Private networks** only.
Anyone on your local network can see the device and push media to it while the feature is on. There is no pairing for DLNA, as in the standard.
RusTV sends nothing to the Internet for this feature; it only opens the media URLs your phone gives it (like any player), and the YouTube page.

## Troubleshooting

- The phone does not see RusTV: check the status row (address and "listening"), that the phone is on the same subnet (guest Wi-Fi and
  "client isolation" block multicast), and the firewall rule for private networks.
- No address shown: no private IPv4 interface found (VPN-only or public-address network).
- Another program already uses port 47840: the HTTP listener cannot start on that interface and the status stays "stopped".
- Play fails with error 501: mpv is not installed; open the player page once to install it.
- The YouTube icon does not appear: some phone versions need a few seconds after the network changes; toggle the feature off and on.
