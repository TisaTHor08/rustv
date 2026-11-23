# Interface styles ("OS styles")

RusTV can make its home screen look and behave like the home screens of the big TV platforms. A style changes the layout, the motion, the colours, the corner radius and the font of the home screen, and (through CSS variables) the look of every other screen: Settings, Search, Live TV, media pages, mod manager, overlays and pairing.

## Disclaimer

All trademarks and product names belong to their owners. RusTV is not affiliated with, endorsed by or sponsored by any of them. The styles are independent recreations of the general layout and behaviour of a home screen. They use no logo, wordmark, proprietary font or artwork of those companies; names shown in the picker are descriptive ("Google TV style", "Roku style").

## Choosing a style

Settings > Appearance > Interface style (left/right or OK cycles through the available styles; the row label shows the one-line description). The change is live, with a short cross-fade, no restart. The preference is `prefs.osStyle` (default `rustv`). A style whose files are not present is not offered; an unknown saved value falls back to `rustv`.

Settings > Appearance > Widget position moves the clock / phones / weather / mod widgets group: Auto (the style's own place), Top right, Top left, Bottom right, Bottom left, Hidden (`prefs.widgetPos`).

## Available styles

| id | Name | Status |
| --- | --- | --- |
| `rustv` | RusTV style | the original home, unchanged |
| `googletv` | Google TV style | available |
| `roku` | Roku style | available |
| `tizen`, `webos`, `vidaa`, `titan`, `fire`, `tvos` | (to come) | ids reserved in the registry, hidden until their files exist |

## Framework contract

Files (all under `ui/src/lib/os/`):

- `registry.ts`: `OsStyle`, `STYLES`, `availableStyles()`, `resolveStyle(id)`, `resolveWidgetSlot()`. Styles are found with `import.meta.glob`, so adding a style does not touch any existing file.
- `model.svelte.ts`: the `HomeModel` interface (what a layout may read and call).
- `apply.svelte.ts`: applies `prefs.osStyle` (tokens, `data-os`, font preload, lazy component load, live switch).
- `strings.ts`: shared strings (picker label, widget positions). `util.ts`: `scrollFocus`, `reducedMotion`, `tileFg`.
- `WidgetDock.svelte`: the widget group shown when the user chose a corner.
- `styles/<id>.svelte`: the layout component, receives `{ model }`.
- `styles/<id>.meta.ts`: default export `OsStyleMeta`. It also imports `./<id>.strings` (French and English through `registerStrings`) and an optional `./<id>.css` (for `@font-face`).

To add a style: create `styles/<id>.svelte` and `styles/<id>.meta.ts` (plus `.strings.ts` and `.css`). The id must be one of the reserved ids in `STYLE_IDS` (add a new id there only for a brand-new platform).

### OsStyle

```ts
interface OsStyle {
  id: OsStyleId;                       // 'rustv'|'googletv'|'tizen'|'webos'|'vidaa'|'titan'|'fire'|'tvos'|'roku'
  nameKey: string; descKey: string;    // i18n keys
  load: () => Promise<{ default: Component<{ model: HomeModel }> }>;   // filled by the registry
  tokens: Record<string, string>;      // CSS variables set on html[data-os="<id>"]
  font?: string;                       // family preloaded with document.fonts.load
  reducedMotionSafe?: boolean;
  slots?: WidgetSlot[];                // widget slots the layout provides (default ['status'])
  nav?: (dir: 'up'|'down'|'left'|'right', model: HomeModel) => boolean;  // true = handled
  activate?: (model: HomeModel) => boolean;                              // OK key, true = handled
  rows?: (base: HomeRow[], model: HomeModel) => HomeRow[];               // build the rows the style shows
}
```

`activate` and `rows` are additions to the original contract: they let a style give OK a different meaning in a zone (tab bar, menu) and supply its own content (tabs, grids) without touching Home.

### HomeModel (read)

`rows` (`{key,title,tiles}[]`, after the style's `rows` hook), `focus` (`{row,col}`, row -1 = settings icon of the RusTV style), `focused` (DTile), `zone` (string, default `'content'`), `tilt`, `time`, `date`, `phones`, `phonesLabel`, `weatherEnabled`, `widgetPos`, `widgetsNative`, `recent` (DTile[]), `continueEntries`, `allTiles`, `tinted`, `hero` (`{title,desc,category,color,color2,tile}`), `ui` (a `$state` bag for the style: current tab, menu entry...).

### HomeModel (call)

`move(dRow,dCol,dir)` default move with sound; `focusAt(row,col,silent?)` (plays the move sound unless `silent`, use `silent` for mouse hover); `setZone(zone)`; `activate()`; `open(tile)`; `resume(url)`; `openSearch()`; `openSettings()`; `openLive()`; `openMedia(tab)`; `contextMenu(tile)`; `tileById(id)`; `note(msg)`.

Home keeps everything else: key handling, launch overlay with input blocking, modals, updates, idle cursor, Back. Back from a zone other than `'content'` returns to `'content'` first; Back in `'content'` does nothing (as before).

### Navigation

Arrow keys call `style.nav(dir, model)`; if it returns false (or is absent) `model.move` runs. OK calls `style.activate(model)` then `model.activate()`. A layout changes the focus with `focusAt` and zones with `setZone`, so sounds, the hero and the ambient colour keep working. Scroll the focused tile into view with CSS transforms or `scrollFocus()` (smooth unless prefers-reduced-motion).

### Tokens

Set on `html[data-os="<id>"]` by a `<style id="rustv-os-tokens">` element, so the order of layers is: style tokens, then mod theme (`!important`, inline), then the accent chosen by the user (inline). The palette tokens `--bg --bg-rgb --text --muted --glass --glass-strong --sheet --field --ring` apply only in the dark theme (the light theme keeps the RusTV light palette). Other tokens are always applied: `--accent` (used only while the user's accent is still the default orange), `--r` (corner radius; `.sheet` panels use 1.5 x `--r` in non-RusTV styles), `--font`, `--focus-scale`, `--focus-ring`, and optional `--dock-top/-left/-right/-bottom` (margins of the widget dock). `app.css` holds today's values as defaults, so `rustv` is unchanged.

## Widgets

The existing widget areas are the clock and date, the phones pill, the weather and the mod widgets (`ModWidgets.svelte`). Slot vocabulary (`WidgetSlot`): `status` (compact strip, top right), `hero`, `bar`, `corner-bl`, `corner-br`. A layout declares the slots it provides in `meta.slots` and renders its own widgets only when `model.widgetsNative` is true (position Auto).

- Mod widgets: the manifest `slot` (default `topbar`, which means `status`) is resolved by `resolveWidgetSlot(requested, provided)`: the requested slot if the style provides it, else `status`, else the first provided slot. A layout renders `<ModWidgets at="status" provided={...} />`; without `at` all widgets render. A widget therefore never silently disappears (except when the user chooses Hidden).
- Per-style defaults: RusTV top right (pill, clock, icons); Google TV top right (phone icon with count, weather, clock, profile avatar stays as a button); Roku top right (phones, clock).
- User setting: Auto / Top right / Top left / Bottom right / Bottom left / Hidden. For a corner, Home shows `WidgetDock` (same content in every style: mod widgets, weather, phones, clock) and the layout hides its own group. Dock margins can be tuned by the style with `--dock-*` tokens (Google TV puts the top dock under the tab bar, Roku moves it beside the menu).
- Limits: a bottom dock can cover the lowest row for a moment; it has a translucent background and is not focusable. Mod widgets keep their messaging through `modHost` (unchanged).

## Per-style notes and completeness checklists

### Google TV style (`googletv`)

Font Roboto (OFL), local files. Reproduced: avatar then a pill holding search and the tabs For you / Movies / Shows / Apps / Library (focused tab = light filled pill, selected tab underlined), clock and phone status at the top right, full-bleed hero with the focused tile's colours and a gradient scrim, title, description, Open and "+" buttons, page dots at the right, "Continue watching" cards (16:9, progress bar, remaining time), "Your apps" as round icons, category rows, focused card scale 1.08 with a white 0.2 rem ring, hero cross-fade after 150 ms, vertical scroll anchoring of the focused row, up from the first row focuses the tab bar, Down returns, Enter on a tab selects it, tabs filter content (Apps = grid, Movies/Shows = video and TV tiles, Library = library tiles), Back from the tab bar returns to content.

Approximated: the hero art is the tile colour with a large letter or icon (no key art); the "+" button is decorative; the dots follow the column; the Live tab of recent Google TV versions is not present; wallpaper is replaced by the ambient gradient.

Common screens (see "Screens" below) are all provided: two-column full-screen Settings, Search with on-screen keyboard, quick settings panel (top right, tiles), tile menu, pairing page, vertical volume OSD on the right, ambient mode (landscape with clock and weather), toast bottom left, launch transition (tile colour grows to full screen). Left out: profile switcher (the avatar opens the RusTV Settings), input panel.

### Roku style (`roku`)

Font Montserrat (OFL). Reproduced: left menu (Home, Search, Apps, Live TV, Library, Music, Games, Settings) with a white pill on the focused entry, 4-column grid of 16:9 tiles with names below, thick white ring and scale 1.08, item counter, clock and phones at the top right, hint bar "OK Open / * Options" (the star is the ContextMenu key and opens the tile menu), Left from the first column goes to the menu, Right returns, menu entries switch the grid or open Search / Live TV / Settings.

Approximated: no theme art background (a purple gradient), no icons in the menu (as in the reference), tile art is a letter or the tile icon.

Common screens provided: Settings as a left list plus right detail pane, Search with on-screen keyboard, Options panel on the right (tile menu and quick settings), pairing page, horizontal volume OSD at the bottom, bouncing-card screensaver, toast at the top, launch screen with loading bar. Not done: tile reordering, input panel.

## Screens (extension points)

`OsStyle.screens` (`os/screens.ts`) lets a style redraw the common screens: `settings`, `search`, `tileMenu`, `pairing`, `quick`, `osd`, `ambient`, `toast`, `launch`. Each entry is a lazy loader of a Svelte component (`styles/<id>.screens/<name>.svelte`) that receives typed props (`ScreenProps[name]`). Logic (keys, data, actions) stays in the shared components (`Settings`, `Search`, `TileMenu`, `Pairing`, `os/QuickHost`, `os/SystemLayer`); a screen component only draws the props and calls the given callbacks. `ScreenSlot.svelte` loads the style's component and falls back to the default one (`os/defaults/`, the RusTV look re-skinned by tokens) when the style does not provide it or while it loads. Styles without `screens` (tizen, webos... to come) therefore keep working unchanged. The Search screen binds the real input with `bindInput` (phone keyboard keeps working) and may take over arrow keys with `setKeyboardHandler` for an on-screen keyboard.

New common features:

- Quick settings (`QuickHost`, opened by `model.openQuick()`): volume slider, audio output, display settings, phones, all settings, exit TV mode. Network and PC sleep are not offered (no safe native command).
- Volume OSD: the shell (`rustv-shell/src/volume.rs`) compares the core's polled volume every 250 ms and emits the Tauri event `volume-changed { level, muted }` on real changes only (never at start). `SystemLayer` shows the OSD for about 1.5 s; preference `prefs.volumeOsd` turns it off.
- Ambient / screensaver: `SystemLayer` shows the style's `ambient` after `prefs.ambientMin` minutes without input; any key or pointer move closes it, and any opened window cancels it.
- Toast (`model.note`), launch transition (`launch` screen, shown by Home while an app opens), pairing and tile menu use the same slot mechanism.
- Widget position: style default (`slots`, `--dock-*`) overridden by `prefs.widgetPos`, as described under Widgets.

To give a new style these screens, add the loaders to `meta.screens`; any missing one uses the default.

### Not done

Profile switcher, input panel, network and PC-sleep tiles in quick settings; screens for the styles still to come (they use the defaults).

## Fonts and licences

Roboto and Montserrat (variable) from the Fontsource packages, SIL Open Font License 1.1, Latin and Latin-extended subsets in `ui/public/fonts` (about 250 KB), declared with `font-display: swap`; licences next to the files. No network access at runtime.

## Developing a style: mock harness

`ui/mock.html`, `ui/src/mock.ts` and `ui/vite.mock.config.ts` run the home in a normal browser with a fake Tauri (catalog from `catalog.json`, sample "Continue watching" entries). They are not part of `npm run build`. Run `npx vite --config vite.mock.config.ts --port 5199` in `ui/`, open `http://localhost:5199/mock.html?os=roku&keys=ArrowDown,ArrowRight,Enter`. Other parameters: `theme=light`, `lang=en`, `scale=110`, `cont=0`, `wp=bl`, `still=1` (no CSS transitions, for screenshots), `dbg=<css selectors>` (writes element rectangles into the page title).

## Not verified

- `cargo test` for `volume.rs` could not run in the last session (disk full); the code compiles logically against `Core::volume()` but was not built.
- Screens were checked with `npm run check` and `npm run build`, not visually against the reference screenshots frame by frame.
- No run on a real TV session or with the real Tauri shell; no check with the phone remote.
- Mod widgets in a style were not exercised in the mock (the mock has no mods); weather refresh was not exercised (network).
- Visual comparison was done against a handful of reference screenshots, not frame by frame; motion timings are estimates.
- Light theme with a style, very small windows, portrait and 32:9 were not all inspected.
