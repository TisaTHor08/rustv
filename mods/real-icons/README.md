# Real icons

Shows each app's logo on its tile instead of a letter, and an app screen visual in the top banner of the home screen
(Netflix, YouTube, Molotov…). Quickest way to turn it on: *Settings → Appearance → Real app logos and visuals*
(it installs the mod if needed). It can also be enabled in *Settings → Mods*.

The banner visuals in `banners/` are original illustrations drawn in each service's colours by `scripts/gen-banners.py`,
not official screenshots. To use a real screenshot, drop an image in `banners/` and point the tile to it in
`contributes.banners` (`{ "tile": "netflix", "src": "banners/netflix.jpg" }`). It needs no permission.

Logos come from [Simple Icons](https://simpleicons.org) (CC0, versions 14.15.0, and 9.21.0 for Deezer and Xbox).
Brands missing from Simple Icons get a plain text monogram. All names and logos belong to their owners and are used
only to identify the matching app.

To change or add one, drop an SVG/PNG in `icons/` and add `{ "tile": "<tile id>", "src": "icons/<file>" }` to
`contributes.icons` in `mod.json`. Tile ids are in `crates/rustv-shell/src/catalog.json`.
