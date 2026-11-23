# Phone remote models

The phone remote can look like the physical remote of the TV platform whose home screen the PC is showing. Layout, proportions and colors follow
the real remote at a glance, but every model sends exactly the same protocol messages as the default remote; only the skin and the arrangement change.

## Models

| Id | Inspired by | Main controls |
| --- | --- | --- |
| `rustv` | The original RusTV remote (default) | Ring or touch surface, round buttons, volume slider, media row |
| `googletv` | Google TV voice remote | White body, large ring with OK, Back / Home, voice (search), input, mute, 2 shortcuts, side volume rocker |
| `tizen` | Samsung Smart Remote (2021+) | Charcoal body with solar strip, ring, Back / Hub / Play, volume and channel rockers, 3 shortcuts |
| `webos` | LG Magic Remote | Scroll wheel with OK in the ring, Back / Home / Settings, volume and channel rockers, 3 shortcuts, pointer mode |
| `vidaa` | Hisense VIDAA remote | Large rounded cross with OK, Back / Home / Menu / Settings, rockers, 4 color keys, 4 shortcuts |
| `titan` | Philips Titan OS remote | Brushed charcoal body, cross with OK, Back / Home / Options, Ambilight button, rockers, color keys, 3 shortcuts |
| `fire` | Amazon Fire TV / Vega OS voice remote | Ring, Back / Home, Menu / voice, rewind / play / forward, volume bar and mute, 4 shortcuts |
| `tvos` | Apple Siri Remote | Aluminium frame, round clickpad, Back / TV, voice, play / pause, mute, side volume rocker |
| `roku` | Roku voice remote | Black and purple body, ring with OK, Back / Home / Replay / Options, rewind / play / forward, side volume, 4 wide shortcuts |

The ids are the same as the TV home style ids.

## Choosing a model

- `Auto` (default): the PC announces its TV home style in the `state` message (`osStyle`, see [PROTOCOL.md](PROTOCOL.md)) and the phone follows it live.
  A PC without the field, or an unknown id, gives `rustv`.
- Manual: phone Settings, "Remote model", pick one model. The choice is stored on that phone only (`localStorage`, key `rustv.remoteModel`).
- The PC side: the TV interface calls the Tauri command `set_os_style(id)` when the style changes; the core validates the id, stores it and broadcasts
  a new `state` to connected phones only when it actually changed.

Remote mods keep working: themes recolor the header, tabs, now-playing bar and the pressed state of the keys (`--accent`). The body colors of a model are
the physical colors of the real remote and are not recolored by a theme. The Apps, Mouse and Mods tabs are the same for every model.
Keyboard, Bitwarden vault and Alt+Tab (tabs) are not on real remotes, so non-default models show them in a small strip under the remote.

## Button mapping

Common to all models: direction keys send `key` up/down/left/right with auto-repeat while held; OK sends `key enter` (or confirms Alt+Tab when it is open)
and a long press sends `key menu`; Back sends `key back`, long press `key backlong`; Home sends `home`; the voice / assistant button sends `key search`
(universal search); mute and volume send `media` `mute` / `volUp` / `volDown` (the hardware volume keys of the phone still control the PC volume);
play / pause sends `media playPause`.
Channel buttons have no equivalent on a PC: channel up / down send `media next` / `media prev`.
Rewind and fast forward send `media prev` / `media next` (there is no seek message in the protocol).
Streaming shortcuts show a neutral "Stream n" label and send `openTile` with the catalog id below; nothing happens if the tile is missing on the PC.

| Model | Specific buttons and mapping |
| --- | --- |
| `rustv` | Unchanged: Back, Home, Search, Tabs row; volume slider; prev / play / next, keyboard, vault |
| `googletv` | Input = `home`; shortcuts: `youtube`, `netflix` |
| `tizen` | Hub = `home`; shortcuts: `netflix`, `prime`, `disney` |
| `webos` | Gear = `key menu`; wheel swipe up/down = `key up`/`key down` (one step per 26 px), wheel tap = OK, wheel long press = `key menu`; pointer toggle swaps the ring for the existing touchpad (`mouse` messages); shortcuts: `netflix`, `prime`, `disney` |
| `vidaa` | Menu and Settings = `key menu`; color keys (convenience mapping, no teletext on the PC): red = `media stop`, green = `media playPause`, yellow = `media prev`, blue = `media next`; shortcuts: `netflix`, `youtube`, `prime`, `disney` |
| `titan` | Options = `key menu`; Ambilight = no message, only a toast "Ambilight: available when Hyperion is enabled on the PC" (see [AMBILIGHT.md](AMBILIGHT.md)); color keys as VIDAA; shortcuts: `netflix`, `youtube`, `prime` |
| `fire` | Menu = `key menu`; rewind / forward = `media prev` / `media next`; shortcuts: `prime`, `netflix`, `disney`, `spotify` |
| `tvos` | Clickpad: swipe = arrow steps with haptic ticks and inertia, tap center = `key enter`, tap on the edge = the arrow of that side, long press = `key menu`; TV button = `home` |
| `roku` | Replay = `media prev` (no seek message); Options (*) = `key menu`; shortcuts: `netflix`, `disney`, `prime`, `max` |

## Adding a model

1. Create `remote/src/lib/remotes/<id>.svelte`. It receives `{ ctx }` (see `remotes/types.ts`: OK handling, Alt+Tab state, toast, keyboard and vault hooks)
   and builds its layout from `remotes/parts/` (`Dpad`, `Rocker`, `Btn`, `Shortcut`, `Wheel`, `Icon`) and `remotes/actions.ts` (button to message mapping).
   Skin it with the `--dp-*` (D-pad) and `--rk-*` (rocker) CSS variables and your own styles. Keep touch targets at 44 px or more.
2. Add one line to `MODELS` in `remotes/registry.ts` with the id, the French and English names and the component.

If the TV style is new, also add its id to `OS_STYLES` in `crates/rustv-core/src/core.rs`. Aria labels live in `remotes/labels.ts` (French and English).

## Trademarks

These are generic recreations drawn for convenience. RusTV is not affiliated with, endorsed by or sponsored by any of the companies whose products
inspired the layouts. The platform names above are used only to say which remote a layout resembles. No logos, wordmarks or brand artwork are used;
shortcut buttons are labeled "Stream 1", "Stream 2", and so on.

## Not verified

- No test on a physical phone: appearance was judged from headless Chrome screenshots of a 390 x 844 frame only, and touch behavior (long press, repeat, wheel swipe, clickpad edge taps, haptics) was not exercised on a touchscreen.
- The proportions follow the real remotes from memory and are simplified (buttons are enlarged to stay touch friendly); they are not measured copies.
- The color keys, channel keys, Replay and rewind / forward are convenience mappings, not equivalents of what the real keys do.
- The Android APK was not rebuilt; only the web build (`npm run build` in `remote/`) was run.
