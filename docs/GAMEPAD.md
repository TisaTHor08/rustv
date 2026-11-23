# Game controller as a remote

RusTV can use an Xbox-compatible controller (any XInput pad: Xbox 360/One/Series, most wireless dongles and
third-party pads in XInput mode) to drive the whole TV experience. The controller produces exactly the same
messages as the phone remote (`PhoneMsg`, handled by `Core::handle`), so it behaves identically.

## Default mapping

| Controller | Action (same as the phone) |
| --- | --- |
| D-pad, left stick | Arrow keys, with auto-repeat (initial delay 380 ms, then every 90 ms) |
| A | Enter (OK). Also validates the Alt+Tab switcher |
| B | Back (Escape), sent on release. Held 600 ms: long Back (Alt+Left, browser back) |
| Y | Search (F3) |
| X | Menu / context menu key |
| Start | Menu / context menu key (on release) |
| View (Back/Select) | Home (`Event::Home`, like the phone Home button), on release |
| LB / RB | Window switcher: previous / next (Alt+Tab, then Left/Right; A confirms) |
| LT / RT | Volume down / up (media keys), with repeat |
| L3 | Play / pause |
| Right stick (vertical) | Mouse wheel scroll |

Swap A/B exchanges OK and Back.

Pointer mode (off by default): the right stick moves the mouse cursor, R3 is a left click and L3 a right click.
Scrolling with the right stick is not available in this mode.

Start + View pressed together never trigger Menu or Home.

## Safety

- The mapping acts only when the foreground window belongs to RusTV, to a process started by RusTV (mpv, the
  browser it launched; detected through the process parent chain, 7 levels), or to an executable in the
  `allow` list. An unknown or inaccessible process (for example an elevated one) is never driven. Games and
  Steam keep full use of the pad.
- The `deny` list always wins, even over processes started by RusTV (not over RusTV's own windows).
- `onlyWhenTv` (default on): nothing is injected outside TV mode.
- Escape combo: hold Start + View for 3 seconds to suspend or resume the mapping (long rumble as feedback).
  It only works while the foreground window is allowed, so it cannot fire inside a game. The pause is not
  saved: it is lifted when TV mode starts and at the next launch.
- After a window regains focus or the mapping resumes, buttons held at that moment are ignored until
  everything is released.

## Configuration

Stored in `%APPDATA%\RusTV\gamepad.json`, editable from Settings > Controller:

```json
{ "enabled": true, "deadzone": 0.35, "repeatDelayMs": 380, "repeatRateMs": 90, "vibration": 1,
  "swapAB": false, "pointerMode": false, "onlyWhenTv": true, "allow": [], "deny": [] }
```

`vibration`: 0 off, 1 low (default), 2 medium, 3 strong. Haptics: a very short pulse on OK and Back, a double
pulse when a controller connects (only while the mapping is active) and on long Back. Values are clamped
(dead zone 0.1 to 0.9, delay 150 to 1500 ms, rate 30 to 400 ms). `allow` / `deny` hold executable names such as
`vlc.exe` (case insensitive, `.exe` optional, at most 32 each).

## How it works

A thread loads `xinput1_4.dll`, then `xinput1_3.dll`, then `xinput9_1_0.dll` at runtime. Battery information
is optional (missing in 9.1.0). It polls connected slots at about 120 Hz and probes the four slots once per
second to detect hot-plugging (every 250 ms loop when nothing is connected). The page shows each connected
pad with its battery state.

## Limits

- The Guide / Xbox button is not exposed by XInput and cannot be used.
- Only XInput pads: PlayStation, Switch or generic DirectInput pads need Steam Input or an XInput wrapper.
- Not verified on real hardware yet: everything logic-related is unit tested, the XInput, rumble and
  foreground detection calls were only compiled.
- Rumble and battery readings depend on the pad and driver.

## Troubleshooting

- Nothing happens: check that the pad is listed in Settings > Controller, that TV mode is on, that the mapping
  is not suspended (Start + View for 3 s), and that the focused window is RusTV or one it started.
- A tool started outside RusTV (for example a player you launched by hand): add its `.exe` name to `allow`.
- Drift: raise the dead zone.
