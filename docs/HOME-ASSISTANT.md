# Home Assistant integration

RusTV can show up in Home Assistant as a device ("RusTV <PC name>") and be controlled from it. It uses MQTT
with Home Assistant's MQTT discovery, so no custom component is needed. The integration is **off by default**.

## What you need

- An MQTT broker (for example the Mosquitto add-on of Home Assistant) and the Home Assistant "MQTT" integration
  connected to it.
- The broker address, and a user name and password if the broker requires them.

## Setup

1. On the TV: Settings > Home Assistant (also reachable from the phone remote while in TV mode).
2. Enter the broker address and port (1883, or 8883 with TLS), user name and password.
3. Choose "Test connection": the result is shown in plain words (wrong password, nothing listening, host not found...).
4. Turn "Home Assistant integration" on. The status row shows "connected" and the number of entities.
5. In Home Assistant, the device appears under Settings > Devices & services > MQTT.

Settings are stored in `%APPDATA%\RusTV\ha.json`. The password is encrypted with Windows DPAPI (tied to your
Windows account, with an application-specific entropy) and is never shown again, never logged and never returned to
the interface. Copying ha.json to another PC or account will not carry the password: enter it again.

Fields: base topic (default `rustv/<pc id>`), discovery prefix (default `homeassistant`), client id (default
`rustv-<pc id>`, editable in ha.json only), TLS.

## Entities

All entities belong to one device, have a stable unique id (`rustv_<pc id>_<key>`) and an availability topic
(`<base>/availability`: `online`, or `offline` through the MQTT Last Will if RusTV disappears).

| Entity | Type | Notes |
| --- | --- | --- |
| State | sensor | `off`, `home`, `playing`, `paused`, `app`; attributes: tv_mode, current_app, title, volume, muted, phones |
| Now playing | sensor | title or `idle`; attributes: artist, source, playing, position, duration (seconds, position rounded to 5 s) |
| Current app | sensor | last tile opened through RusTV (see limits) |
| Volume | sensor, number | 0 to 100 |
| TV mode | binary_sensor, switch | on = start TV mode, off = leave it |
| Mute | switch | |
| Phone connected | binary_sensor | attribute `count` |
| App launcher | select | options are the names of all openable tiles (built-in, catalog, custom, mods); selecting one opens it |
| Open tile (id) | text | for scripts: send a tile id (no state) |
| Buttons | button | Home, Play/Pause, Next, Previous, Stop, Volume up/down, Key up/down/left/right/OK/Back/Menu/Search |
| Uptime, Version | sensor (diagnostic) | |

The tile list is refreshed every minute and when prefs.json changes; the discovery config of the select is re-published
when the list changes. Hidden tiles are not offered; renamed tiles use their custom name; duplicate names get " (2)".

## MQTT topics

State (retained, QoS 1, published only on change, about once per second polling, instantly for TV mode):
`<base>/state/<key>`. Commands (QoS 1, never retained: retained commands are ignored):

| Topic | Payload |
| --- | --- |
| `<base>/cmd/tv` | `ON` / `OFF` |
| `<base>/cmd/mute` | `ON` / `OFF` |
| `<base>/cmd/volume` | `0` to `100` |
| `<base>/cmd/app` | a tile name from the select options |
| `<base>/cmd/open_tile` | a tile id (letters, digits, `-`, `_`, `:`; 80 characters max) |
| `<base>/cmd/button` | `home`, `play_pause`, `next`, `previous`, `stop`, `volume_up`, `volume_down`, `key_up`, `key_down`, `key_left`, `key_right`, `key_ok`, `key_back`, `key_menu`, `key_search` |

Payloads are validated strictly; anything else is ignored, as is any unknown topic. At most 20 commands per second are
executed. Commands go through the same code as the phone remote. Opening a tile while TV mode is off starts TV mode
first (best effort: with several screens the screen picker appears and the tile is not opened).

When Home Assistant restarts it publishes `homeassistant/status` = `online`; RusTV then publishes everything again.

## Buttons in the settings page

- "Re-publish entities": publish all discovery configs and the state again.
- "Remove from Home Assistant": publishes empty retained configs so the device disappears. Turn the integration off
  first, otherwise it comes back on the next start (until you restart RusTV, nothing is re-published after a removal).

## Home Assistant has no MQTT media_player

Home Assistant offers no MQTT `media_player` platform, so RusTV cannot create one by discovery. The file
`integrations/home-assistant/rustv_package.yaml` contains a `universal` media player that wraps the entities above
(power, volume, mute, play/pause, next/previous, source = app launcher). Replace `rustv_salon` by your entity prefix.

## Example automations (in integrations/home-assistant)

- `rustv_package.yaml`: dim the lights when playback starts, restore them when it stops, pause when the doorbell rings
  (press the Play/Pause button), switch the AVR input when TV mode starts, a "movie night" script (starts TV mode, opens
  Plex, sets the volume, dims the lights), an announce script using Home Assistant TTS.
- `lovelace_card.yaml`: a ready dashboard card (status, app launcher, volume, media buttons, directional pad).
- Voice assistants: expose the script "RusTV: open YouTube" (or any script that sets the App launcher select) to Alexa,
  Google or Assist, then say "open YouTube on RusTV" by naming the script.

## Calling Home Assistant from RusTV (not implemented)

There is no action in RusTV that calls Home Assistant yet. The plan for scenes: a phone-remote mod action
`{ "t": "ha", ... }` that publishes to a user-configured MQTT topic (for example `homeassistant/scene/movie/set`) or
calls the Home Assistant REST API with a long-lived token stored with DPAPI like the MQTT password. Until then, use
the other direction: let an automation react to the `State` sensor, or publish yourself with `mqtt.publish`.

## Limits and honesty

- TLS uses the Windows certificate store; brokers with self-signed certificates are not supported (install the CA in
  Windows, or use plain TCP on a trusted LAN). WebSocket transport is not supported.
- "Current app" is the last tile opened through RusTV (phone, Home Assistant). A tile opened with the TV remote
  directly on the screen, or an app closed with Alt+F4, is not detected. It is cleared on "Home" and when TV mode stops.
- State `app` therefore means "a tile was opened and not left through RusTV".
- Plain TCP is not encrypted: user name and password cross the network in clear text unless TLS is on.
- Not verified against a real broker or Home Assistant by the developer (unit tests only); report problems with the
  broker text shown by "Test connection".

## Troubleshooting

- "Nothing answers on this host and port": broker down, wrong port, or firewall.
- "The broker refused the user name or password": check the MQTT user in Home Assistant (a dedicated user is best).
- "Secure connection failed": wrong port for TLS, or untrusted certificate.
- Entities missing: MQTT integration discovery prefix must match (default `homeassistant`); use "Re-publish entities".
- Two PCs: give each a different base topic; ids differ by default (`rustv/<pc id>`).
- Changed the PC name: the device name updates at the next discovery publication.
