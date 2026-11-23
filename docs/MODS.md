# Writing RusTV mods

A mod is a folder with a `mod.json` manifest and some web files (JS, CSS, HTML, images). No build step, no
special tooling. Mods can add themes, tiles, full-screen pages, top-bar widgets and site adapters.

Installed mods live in `%APPDATA%\RusTV\mods\<id>\` (open it from *Settings → Mods → Open the mods folder*).
A mod dropped there appears as **disabled**; enabling it shows the permissions it needs.

Working examples are in [`mods/`](../mods).

## Manifest (`mod.json`)

```json
{
  "id": "my-mod",
  "name": "My mod",
  "version": "1.0.0",
  "author": "You",
  "description": "What it does.",
  "license": "MIT",
  "permissions": ["storage", "launch", "network:https://api.example.org"],
  "contributes": {
    "themes":   [{ "id": "dark-red", "name": "Dark red", "vars": { "--accent": "#e11d48" }, "css": "theme.css" }],
    "tiles":    [{ "id": "site", "name": "My site", "kind": "web", "url": "https://example.org", "category": "video", "color": "#123456", "color2": "#001122", "icon": "icon.png" }],
    "pages":    [{ "id": "home", "name": "Dashboard", "entry": "index.html", "category": "library", "color": "#2d6cdf", "color2": "#0e2a63" }],
    "widgets":  [{ "id": "w", "slot": "topbar", "entry": "widget.html", "width": 12 }],
    "adapters": [{ "match": ["*://*.example.org/*"], "js": "adapter.js", "css": "adapter.css", "runAt": "ready" }],
    "icons":    [{ "tile": "youtube", "src": "icons/youtube.svg" }],
    "remote":   { "themes": [], "panels": [] },
    "strings":  { "fr": { "my.key": "Bonjour" }, "en": { "my.key": "Hello" } },
    "search":   [{ "id": "site", "name": "My site", "tile": "site", "urlTemplate": "https://example.org/search?q={q}", "providerNames": ["My site"], "kinds": ["movie"] }]
  }
}
```

Rules: `id` is `a-z`, `0-9` and `-` (2 to 40 characters) and **must equal the folder name**. `name` and `version` are
required. Categories are `video`, `tv`, `music`, `library`, `games`. Tile `kind` is `web`, `native` (an executable or URI,
needs the `launch` permission) or implied `page` for entries under `pages`.

## Contributions

- **Themes**: CSS variables (`vars`, only names starting with `--`) and/or a stylesheet. Useful variables:
  `--bg`, `--bg-rgb` (r, g, b), `--text`, `--muted`, `--accent`, `--glass`, `--sheet`, `--ring`. Users pick the theme in
  *Settings → Appearance → Mod theme*.
- **Tiles**: extra rows entries, handled like built-in ones (users can rename, move, hide them).
- **Pages**: a full-screen HTML page opened from a tile, shown in a sandboxed iframe. Back closes it.
- **Widgets**: a small iframe in the top bar (`width` in rem, height fixed).
- **Adapters**: JS/CSS injected into web apps (Netflix, YouTube, …) whose URL matches a pattern (`*` wildcard).
  `runAt: "ready"` (default) waits for the DOM, `"start"` runs before the page. Adapters run **inside the third-party
  page**, so they can read what that page shows: this is why they require an explicit `inject:<pattern>` approval.
- **Icons**: replace the letter on an existing tile (catalog, built-in or another mod's, by tile id) with an image from your
  mod. See [`mods/real-icons`](../mods/real-icons). Icons the user picked themselves still win.
- **Strings**: translations added to the interface (`registerStrings`).
- **Search**: providers of the universal search, see [Search providers](#search-providers).

## Search providers

`contributes.search` registers a streaming or media site with the universal search ([SEARCH.md](SEARCH.md)): when the
user types a title, the site appears in the "Search in an app" row (and as a "where to watch" chip when the
availability source names it), and Enter opens the mod's tile directly on the site's search page. Declarative only, no
code and no permission. A working example is [`mods/search-internet-archive`](../mods/search-internet-archive).

```json
"search": [
  { "id": "arte", "name": "Arte", "tile": "arte", "urlTemplate": "https://www.arte.tv/fr/search/?q={q}",
    "providerNames": ["Arte"], "kinds": ["movie", "series"] }
]
```

| Field | Rule |
| --- | --- |
| `id` | `a-z`, `0-9`, `-`, 1 to 40 characters, unique in the mod (the app uses `<mod id>:<id>`) |
| `name` | Shown name, 1 to 40 characters |
| `tile` | Tile that opens the site: a tile of this mod (by its `id` in `contributes.tiles`), or an existing tile (catalog tile id such as `arte`). The tile must be a `web` tile and visible in RusTV for the result to be selectable |
| `urlTemplate` | `https://` only, at most 300 characters, contains `{q}` (replaced by the percent-encoded title), `{q}` never in the host name, no credentials, no spaces |
| `providerNames` | Up to 12 names the availability source (TMDB / JustWatch) uses for this service, 1 to 40 characters each, compared without case or accents. Optional |
| `kinds` | Subset of `movie`, `series`, `anime`, `video`. Optional |

At most 8 providers per mod. Invalid entries are ignored by the app (and rejected by `python scripts/mods-tool.py
check`). Only providers of **enabled** mods are used. The listing returned by `mods_list` carries the cleaned entries
in a `search` field.

## Phone remote contributions

A mod can add **themes** and **button panels** to the phone remote through `contributes.remote`. It is purely
declarative: JSON plus an optional CSS file, no code. The full working example is [`mods/remote-demo`](../mods/remote-demo)
(see its README for a line-by-line walkthrough). Other examples: `remote-theme-amoled`, `remote-panel-launch`,
`remote-panel-scenes`.

```json
"contributes": { "remote": {
  "themes": [ { "id": "amoled", "name": "Pure black", "vars": { "--bg": "#000000", "--panel": "#0b0b0b" }, "css": "theme.css" } ],
  "panels": [ { "id": "apps", "name": { "en": "Quick launch", "fr": "Lancement rapide" }, "glyph": "play",
                "buttons": [ { "label": "YouTube", "glyph": "play", "actions": [ { "t": "openTile", "id": "youtube" } ] } ] } ]
} }
```

### Fields and limits

| Field | Rule |
| --- | --- |
| `name`, `label` | A string or `{ "en": "...", "fr": "..." }`. Max 40 characters (theme), 30 (panel), 24 (button) |
| theme `vars` | Keys start with `--`. Values are plain strings: no `;`, `{`, `}`, no `http`/`https` |
| theme `css` | Path inside the mod, 64 KB max, no `@import`, no `http:`, `https:` or `//` |
| `glyph` | Optional icon name for a panel or button |
| Counts | 8 themes per mod, 6 panels per mod, 24 buttons per panel, 20 actions per button |

Theme variables are those of the phone app: `--bg`, `--panel`, `--panel-2`, `--text`, `--muted`, `--accent`, `--ok`,
`--warn`. The phone styles its components with scoped CSS, so `css` rules need `!important`; useful selectors are
`button`, `button.primary`, `.ring`, `.ok`, `.vol`, `.track`, `.fill`.

### Allowed actions

Actions in a button run in order. Only these exist; anything else is dropped.

| Action | Meaning |
| --- | --- |
| `{"t":"key","k":"up"}` | `k`: `up down left right enter back backlong menu search` |
| `{"t":"media","a":"playPause"}` | `a`: `playPause next prev stop volUp volDown mute` |
| `{"t":"setVolume","level":35}` | PC volume, 0 to 100 |
| `{"t":"openTile","id":"youtube"}` | Opens a tile of the TV home by id (see `crates/rustv-shell/src/catalog.json`) |
| `{"t":"home"}` | Back to the TV home |
| `{"t":"tv","on":false}` | Start (`true`) or quit (`false`) the TV session |
| `{"t":"altTab","a":"open"}` | Window switcher: `open left right confirm` |
| `{"t":"text","text":"..."}` | Types text into the focused field on the PC (200 characters max) |
| `{"t":"wait","ms":400}` | Pause, 5000 ms max |

### Security model

- Declarative only: no JavaScript is sent to the phone and the CSS cannot reach the network.
- The PC sanitizes everything (lengths, unsafe CSS or variables, unknown actions, clamped numbers) and only sends the
  contributions of **enabled** mods.
- The phone validates the received bundle again before showing or running anything.
- Actions are commands the built-in remote could already send. A remote contribution needs no permission.

### How it appears on the phone

- **Themes**: listed in the phone settings sheet, grouped by mod. Choosing one sets its `vars` on the app root and
  injects its `css`; choosing the default theme removes them.
- **Mods tab**: one section per panel, titled by its `name` in the phone language (`fr` or `en`, falling back to the
  other). Each button sends its actions in order when tapped.

## Permissions

| Permission | Grants |
| --- | --- |
| `storage` | Small private key/value store (256 KB) |
| `launch` | Open websites (`rustv.launch`) and native tiles |
| `network:https://host` | `rustv.fetch` GET requests to that exact origin |
| `inject:<pattern>` | *Implied by `adapters`.* Run code in matching web pages |

Declare them in `permissions`; `inject:*` and `launch` for native tiles are added automatically from your contributions.
The user sees the full list before enabling. If an update adds permissions, the mod stays disabled until re-approved.

## Page and widget API

Pages and widgets run in a sandbox: no direct network (a strict Content-Security-Policy blocks it), no access to RusTV
internals. Include the SDK and talk to the host:

```html
<script src="__rustv.js"></script>
<script>
  rustv.storage.set('count', 1);                       // permission "storage"
  rustv.fetchJson('https://api.example.org/data')      // permission "network:https://api.example.org"
    .then(console.log);
  rustv.launch({ url: 'https://example.org', name: 'Example' });   // permission "launch"
  rustv.toast('Hello');
  rustv.onKey(key => { /* "ArrowUp", "ArrowDown", "Enter", … (pages) */ });
  rustv.close();                                       // pages only
</script>
```

The remote's **Back** always closes a page; every other key is forwarded to `rustv.onKey`.

## Catalog metadata

Every mod that is listed in the online catalog describes itself with these top-level fields of `mod.json` (the catalog
entry in `registry/index.json` is generated from them by `python scripts/mods-tool.py registry`):

| Field | Required | Meaning |
| --- | --- | --- |
| `category` | yes | `themes`, `widgets`, `pages`, `games`, `apps` (tile packs), `enhancements` (site adapters), `icons` or `remote` (phone remote themes and panels) |
| `publisher` | yes | Name shown as the mod's publisher (individual, team or organization) |
| `description` | yes | One sentence, 10 to 140 characters |
| `details` | no | A longer text, up to 700 characters |
| `tags` | no | Search keywords, lowercase |
| `dependencies` | no | What the mod needs: `{ "type": "mod", "id": "other-mod" }` or `{ "type": "external", "name": "mpv", "url": "https://...", "note": "why" }` |
| `art` | yes | `{ "from": "#rrggbb", "to": "#rrggbb", "glyph": "<name>" }`: colors and glyph used to draw the icon and banner |
| `icon`, `banner` | generated | `icon.svg` (256x256) and `banner.svg` (640x320), written by `python scripts/mods-tool.py art`; you may ship your own PNG/SVG instead |
| `i18n` | no | `{ "fr": { "name": "", "description": "", "details": "" } }` translations shown in the interface language |

Glyph names are listed in `GLYPHS` in [`scripts/mods-tool.py`](../scripts/mods-tool.py). Run `python scripts/mods-tool.py check`
before opening a pull request: it validates the manifest, the referenced files and the metadata.

In the app, each catalog entry shows its icon, banner, description, publisher, category, dependencies and the permissions
it will ask for. You can search by name, publisher, tag or description, install and uninstall it, and give it a personal
star rating. Ratings are stored only on your PC (RusTV has no server and no telemetry).

## Install from the catalog

*Settings → Mods → Online catalog* lists mods from [`registry/index.json`](../registry/index.json). Installing downloads a
**full commit** of the mod's GitHub repository (content cannot change after review), shows the permissions found in the
downloaded manifest, and only then installs and enables it.

### Publish your mod

1. Put the mod folder in a public GitHub repository (it can be a subfolder).
2. Open a pull request adding an entry to `registry/index.json`:

```json
{
  "id": "my-mod",
  "name": "My mod",
  "version": "1.0.0",
  "author": "You",
  "description": "What it does.",
  "source": { "repo": "you/your-repo", "commit": "<40-character commit sha>", "path": "path/to/my-mod" }
}
```

To release an update, open another pull request with the new commit and version. Mods are reviewed for honesty of their
permissions before being merged; a mod that does something its manifest does not declare is removed.
