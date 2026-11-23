# Ambilight (Hyperion)

RusTV can drive an ambient-light LED strip behind the TV through [Hyperion](https://github.com/hyperion-project/hyperion.ng) (hyperion.ng), the open-source ambient lighting server. Hyperion grabs the screen and drives LEDs over USB/serial, WLED, Philips Hue and others.

The feature is a built-in, optional module (`crates/rustv-shell/src/hyperion.rs`), **disabled by default**. It is not a mod: the mod system can neither spawn processes nor open raw sockets, and managing a `hyperiond.exe` child process plus a TCP client needs both.

## How it fits together

```
 screen --(Hyperion DXGI grabber)--> hyperiond --> LED strip (USB / WLED / Hue ...)
                                         ^
 RusTV --(JSON API, TCP 19444)-----------+
   - starts/stops the hyperiond.exe it launched
   - LEDs on/off, brightness, effects
   - ambient colour of the focused home tile
   - dim when a video is paused
```

RusTV never does the screen capture itself: Hyperion does. RusTV only tells Hyperion what to do.

## First-time setup

1. Settings, "Ambilight": turn **Ambilight** on.
2. **Install Hyperion** (or install it yourself, see below). RusTV queries the latest release of `hyperion-project/hyperion.ng` on GitHub, picks the Windows x64 asset, and checks its SHA-256 digest (the digest GitHub publishes for the asset; if none is published the install is refused).
   - If a portable `.zip` is published, it is extracted to `%APPDATA%\RusTV\hyperion\app\`.
   - If only an installer `.exe` is published, it is downloaded and verified into `%APPDATA%\RusTV\hyperion\installer\` but **never run silently**: RusTV asks, then opens it so you follow its own instructions. Afterwards, Hyperion is detected in the usual install folders.
   - Already installed? RusTV looks for `hyperiond.exe` in `%ProgramFiles%\Hyperion`, `%ProgramFiles(x86)%\Hyperion`, `%LOCALAPPDATA%\Programs\Hyperion` and on `PATH`.
3. **Start** the Hyperion process from the page (RusTV also starts it when TV mode starts). Its settings live in `%APPDATA%\RusTV\hyperion\data` (passed with `--userdata`), separate from any Hyperion you may have installed yourself.
4. Choose **Open configuration**: this opens the Hyperion web UI (`http://127.0.0.1:8090`) in your default browser. There you configure, once:
   - **LED hardware**: the controller type. Common options: USB serial (Adalight, Arduino, ESP with Adalight, ...), WLED over the network (DDP/UDP), Philips Hue entertainment, etc.
   - **LED layout**: use the LED layout wizard (number of LEDs on each side, start position, direction).
   - **Capture**: under "Capture", enable the platform (screen) grabber and choose the **DXGI** grabber on Windows, then pick the display that is the TV. Set the frame rate (25 to 30 is enough).
   - Remember that "Capture" works on the display that is actually shown on the TV.
5. Back in RusTV the status should read "Connected".

## Default behaviour (each item can be switched off on the page)

| Moment | What RusTV does |
| --- | --- |
| TV mode starts | Starts Hyperion if "RusTV manages Hyperion" is on, then LEDs on (`componentstate ALL` true, `LEDDEVICE` true). |
| Home screen | Mode "Tile colour" (default): colour derived from the focused tile's gradient, sent after a 400 ms debounce at priority 200 (lower number = higher priority; the grabber is usually 240, so this takes over). Mode "Screen capture": the colour is cleared and the grabber is left in charge. Mode "None": nothing. |
| Video or web app starts | The ambient colour is cleared so the grabber takes over, and the `GRABBER` component is enabled (option "Keep capture on during video"). |
| Video paused longer than the delay (default 30 s) | Brightness drops to 20 percent, restored on resume or stop. Only for the internal player. |
| TV mode exits | RusTV's colours/effects are cleared, brightness restored, LEDs off, and the Hyperion process it started is stopped. |

The home screen has little worth capturing, which is why the default there is the tile colour. Effects are started from the page (favourites first) at priority 150 and removed with "Stop effect and ambient colour".

Only the Hyperion process **started by RusTV** is ever stopped. If something already listens on the API port when RusTV would start it (for example a Hyperion you started yourself), RusTV simply connects to it and never kills it. On Windows the managed process is placed in a Job object that is killed when RusTV ends, even after a crash, so no orphan is left behind.

## Remote Hyperion (for example on a Raspberry Pi)

Hyperion also runs on a Raspberry Pi with a capture card or in a pure "receive" setup. In RusTV set **Hyperion host** to the Pi's address (and the API port, 19444 by default). With a non-local host RusTV never starts or stops any process; it only sends commands. Screen capture of the PC is then up to whatever feeds that Hyperion (for example a Hyperion screen capture client on the PC, or an HDMI capture card on the Pi: see the Hyperion documentation). The tile colour mode works without any capture.

In the Hyperion web UI, "Network Services" controls whether the JSON server is reachable and whether local/remote API authentication is required. If you enable an API token, enter it in RusTV ("API token"). RusTV stores it encrypted with Windows DPAPI (current user) in `%APPDATA%\RusTV\hyperion.json`; it is never returned to the interface and never logged. It is only sent to the host you configured, and the JSON API on TCP is not encrypted: do not expose it beyond your home network.

## Configuration file

`%APPDATA%\RusTV\hyperion.json`, camelCase keys, notably `enabled`, `manage`, `path` (custom `hyperiond.exe`), `extraArgs` (extra command line arguments for `hyperiond.exe`, default none), `host`, `port`, `webPort`, `ambient` (`"tile"`, `"grabber"`, `"off"`), `dimOnPause`, `dimAfterSecs`, `dimLevel`, `ambientPriority`, `effectPriority`, `favourites`.

Command line used: `hyperiond.exe --userdata <RusTV data dir>` followed by `extraArgs`. Some Hyperion builds have an option to hide the tray icon (`--withoutgui`, from memory); it is not in the default arguments because an unknown option would make Hyperion exit. RusTV hides the console window; the tray icon, if any, remains.

## Commands (Tauri)

`hyperion_get`, `hyperion_set`, `hyperion_install`, `hyperion_start`, `hyperion_stop`, `hyperion_power`, `hyperion_brightness`, `hyperion_effect`, `hyperion_clear`, `hyperion_ambient`, `hyperion_open_web_ui`. JSON API commands used: `serverinfo`, `componentstate`, `adjustment` (brightness), `effect`, `color`, `clear`, `authorize` (login). `sourceselect` and `instance` command builders exist and are unit-tested but are not wired to the page yet.

## Troubleshooting

- **Status "Unreachable"**: Hyperion is not running or the host/port is wrong. In the web UI check Network Services, JSON server port. Another firewall prompt may be waiting on the first launch.
- **LEDs stay black while a video plays**: grabber black screen. Common causes: HDR on (the grabber returns a dark or washed-out picture: disable HDR, or enable the tone mapping/HDR options of the Hyperion grabber if your version has them), protected (DRM) content that Windows hides from capture, capturing the wrong display, or a fullscreen exclusive game/app. Also verify in the Hyperion web UI that the grabber component is on and its priority is visible.
- **Flicker**: lower the grabber frame rate, enable Hyperion's smoothing, check the LED power supply and the data cable length/ground, and enable "black border detection" if the picture has bars.
- **Install refused with "no SHA-256 digest"**: the release does not publish a digest through the GitHub API; install Hyperion manually from its releases page, then RusTV will detect it.
- **Hyperion exits immediately when started by RusTV**: check `extraArgs` in `hyperion.json`.

## What is verified and what is not

Verified: the code compiles; unit tests cover JSON command building, response parsing (including malformed lines and `success: false`), `tan` matching, the ambient colour computation, configuration defaults and bounds, the dim-on-pause and TV mode state machines, and release asset selection from a sample GitHub API JSON.

Not verified (no real Hyperion, LED hardware or network was used while developing): the exact JSON field names against a live Hyperion (written from my recollection of the Hyperion JSON API documentation: command names, `componentstate`, `adjustment`, `effect`, `color`, `clear`, `authorize`/`login` and the `serverinfo` layout, including the version field); the `--userdata` option name; the real names of the Windows release assets (the selection is tolerant but was only tested on a sample, and may find only an installer or nothing); the zip layout; DPAPI encryption of the token and the Job object on the target machine; priority values against your Hyperion setup; the browser/web-app detection used to switch to capture. Please treat the first run as an integration test.
