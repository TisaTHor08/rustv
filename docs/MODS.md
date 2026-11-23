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
    "strings":  { "fr": { "my.key": "Bonjour" }, "en": { "my.key": "Hello" } }
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
