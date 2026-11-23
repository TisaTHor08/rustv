# remote-demo

Reference example of a mod that contributes to the **phone remote**. It adds a theme ("Neon") and a panel ("Demo")
whose buttons each show one kind of action. Copy this folder, rename it, and edit.

Everything is declarative: the mod ships JSON and one CSS file. No code runs on the phone, and the mod needs no
permission (`"permissions": []`).

## Where things show up

- **Theme**: the phone settings sheet lists the themes of enabled mods. Picking one applies its `vars` and `css`.
- **Panel**: the phone "Mods" tab shows one section per panel, with one big button per entry of `buttons`.

## mod.json, line by line

The top-level fields (`id`, `name`, `version`, `author`, `publisher`, `category`, `description`, `details`, `tags`,
`license`, `art`, `i18n`, `icon`, `banner`) are the usual catalog metadata described in `docs/MODS.md`. Here the
category is `remote`. The new part is `contributes.remote`:

```json
"contributes": { "remote": { "themes": [ ... ], "panels": [ ... ] } }
```

### Theme

```json
{ "id": "neon", "name": { "en": "Neon", "fr": "Neon" }, "vars": { "--accent": "#ff2bd6", ... }, "css": "theme.css" }
```

- `id`: unique within this mod.
- `name`: shown in the phone settings. A string or `{ "en": "...", "fr": "..." }`, 40 characters max (per language).
- `vars`: CSS custom properties applied on the phone. Keys must start with `--`. Values are plain strings: no `;`,
  `{`, `}`, and no `http` or `https`. The phone app reads: `--bg`, `--panel`, `--panel-2`, `--text`, `--muted`,
  `--accent`, `--ok`, `--warn`.
- `css`: optional path of a stylesheet inside the mod, 64 KB max, with no `@import`, no `http:`, no `https:` and no
  `//` anywhere in the file (this also forbids `//` comments and protocol-relative URLs; use `/* ... */`).

At most 8 themes per mod.

### theme.css

The phone app styles its components with scoped CSS (Svelte), so a mod stylesheet must use `!important` to win.
Useful selectors: `button`, `button.primary`, `.ring` (directional ring) with `.dir`, `.cut` and `.ok`, `.vol`
(volume bar) with `.mute`, `.step`, `.track`, `.fill`, `.num`. The Neon file adds a pink outline to buttons, a purple
glow to the ring and a cyan to pink gradient on the volume fill.

### Panel

```json
{ "id": "demo", "name": { "en": "Demo", "fr": "Demo" }, "glyph": "bolt", "buttons": [ ... ] }
```

- `id`: unique within this mod. `name`: 30 characters max. `glyph`: a small icon name (optional).
- At most 6 panels per mod, 24 buttons per panel.

### Button

```json
{ "label": { "en": "Type text", "fr": "Taper du texte" }, "glyph": "type", "actions": [ { "t": "text", "text": "Hello from a mod" } ] }
```

- `label`: 24 characters max (string or `{en, fr}`).
- `glyph`: optional icon name.
- `actions`: run **in order**, 20 max. A button with no valid action is dropped.

## The seven demo buttons

| Button | Action(s) | What it shows |
| --- | --- | --- |
| Key: Menu | `{"t":"key","k":"menu"}` | A remote key. `k` is one of `up down left right enter back backlong menu search`. |
| Media: Play/Pause | `{"t":"media","a":"playPause"}` | A media command. `a` is one of `playPause next prev stop volUp volDown mute`. |
| Volume 25 | `{"t":"setVolume","level":25}` | Sets the PC volume, 0 to 100. |
| Open YouTube | `{"t":"openTile","id":"youtube"}` | Opens a tile of the TV home by id (ids are in `crates/rustv-shell/src/catalog.json`, or a tile added by a mod). |
| Type text | `{"t":"text","text":"Hello from a mod"}` | Types text into the focused field on the PC (200 characters max). Harmless here. |
| Chain + wait | `key down`, `wait 300`, `key down`, `wait 300`, `key up` | Several actions in a row. `wait` pauses `ms` milliseconds (5000 max). |
| Home | `{"t":"home"}` | Goes back to the TV home. |

Other allowed actions not used by a button here: `{"t":"tv","on":true|false}` (start or quit the TV session) and
`{"t":"altTab","a":"open|left|right|confirm"}` (window switcher). Anything else is ignored.

## Security model

1. The PC reads `mod.json` only for enabled mods and **sanitizes** every field: labels are length-limited, theme
   variables and CSS are rejected if unsafe, actions outside the allowed list are dropped, numbers are clamped.
2. The phone **validates the bundle again** before using it, so a modified PC or message cannot do more.
3. No JavaScript from the mod ever reaches the phone, and the CSS cannot load anything from the network.
4. Actions are only commands that the remote itself could already send. They run on your PC through the same
   channel as the buttons of the built-in remote.

## Try it

Copy the folder to `%APPDATA%\RusTV\mods\remote-demo\`, enable it in Settings, Mods, then open the phone remote: pick
the theme in its settings and open the Mods tab.
