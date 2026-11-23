# Real icons

Shows each app's logo on its tile instead of a letter. Enable it in *Settings → Mods*. It needs no permission.

Logos come from [Simple Icons](https://simpleicons.org) (CC0, versions 14.15.0, and 9.21.0 for Deezer and Xbox).
Brands missing from Simple Icons get a plain text monogram. All names and logos belong to their owners and are used
only to identify the matching app.

To change or add one, drop an SVG/PNG in `icons/` and add `{ "tile": "<tile id>", "src": "icons/<file>" }` to
`contributes.icons` in `mod.json`. Tile ids are in `crates/rustv-shell/src/catalog.json`.
