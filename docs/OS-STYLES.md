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
| `vidaa` | VIDAA style | available |
| `titan` | Titan style | available |
| `tizen` | Tizen style | available |
| `webos` | webOS style | available |
| `fire`, `tvos` | (to come) | ids reserved in the registry, hidden until their files exist |

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

Left out or not done yet: quick-settings overlay and profile switcher (the avatar opens the RusTV Settings); style-specific Settings layout (full-screen two-column page), search screen look, volume OSD, screensaver / ambient mode, notification look, input panel: the shared screens are only re-skinned through tokens (palette, radius, font).

### Roku style (`roku`)

Font Montserrat (OFL). Reproduced: left menu (Home, Search, Apps, Live TV, Library, Music, Games, Settings) with a white pill on the focused entry, 4-column grid of 16:9 tiles with names below, thick white ring and scale 1.08, item counter, clock and phones at the top right, hint bar "OK Open / * Options" (the star is the ContextMenu key and opens the tile menu), Left from the first column goes to the menu, Right returns, menu entries switch the grid or open Search / Live TV / Settings.

Approximated: no theme art background (a purple gradient), no icons in the menu (as in the reference), tile art is a letter or the tile icon.

Not done: Roku Settings as a left list plus right detail pane, search screen with the on-screen keyboard, tile reordering, screensaver, volume OSD, input panel.

### VIDAA style (`vidaa`)

Font Roboto (OFL). Reproduced: bright blue-teal background, a status strip with the "Home" label and four shortcut pills (Live TV, Library, Search, Settings), clock, phones and mod widgets at the right, a large rounded promotional banner that follows the focused tile (colours, title, description, Open button, dots), an "Apps" row of 16:9 colourful rounded tiles with the name below, then "Continue watching" and the category rows, focus = scale 1.12 with a white 0.25 rem frame and a cyan glow, vertical anchoring (the banner slides away once you leave the first row), horizontal scrolling that keeps the focused tile visible, Up from the first row focuses the shortcut strip, Down returns, Enter on a pill opens the page, Back from the strip returns to content.

Approximated: there are NO VIDAA UI screenshots in the reference folder (only remote photos), so the layout comes from general knowledge of VIDAA U5 to U7 and is the least verified of the two: the banner is tile colours with a letter or icon (no key art), the strip has no real input-source or network icon set (a plain "Home" label and a Wi-Fi glyph for connected phones), proportions and colours are estimates.

Left out: input sources panel, quick settings, VIDAA's own settings layout, screensaver, notification centre, tile reordering.

### Titan style (`titan`)

Font Poppins (OFL). Reproduced (checked against four photos of Philips Titan OS): an icon-only top bar (search, home, apps, live, library) with a white underline under the current page, a blue underline plus label under the focused icon, settings gear and clock at the right, a full-width hero (category chip, large title, description, translucent "Open" pill, elongated page dot), a row of small rounded-square app tiles right under the hero, then 16:9 rails with the row title above and the name over a bottom gradient, "Continue watching" with a progress bar, focus = scale 1.1 with a thin bright ring and a soft glow, the hero fades out and rows scroll up when you go down, Up from the first row reaches the top bar, Left/Right moves between icons, Enter opens or switches page (Home, Apps grid, Library), Back returns to content.

Approximated: hero art is the tile colour with a large letter or icon (no key art), no auto-rotation of the hero (the dots follow the focused column), the indigo tint is an ambient gradient, icon glyphs are generic.

Left out: Titan's account/profile entry, the "Gaming" and "Sport" style category pages, the Ambilight-linked background, Settings layout, screensaver.

Both styles share a small helper `styles/trackfit.ts` (horizontal scroll fitting). No shared framework file was edited.
### Tizen style (`tizen`)

Font Inter (OFL). Reproduced: full-bleed ambient hero of the focused app in the upper part with title and description, a dark translucent launcher bar over the lower fifth of the screen, a fixed group of five round buttons at its left (Settings, Sources, Search, Ambient, Options) then 16:9 app tiles (recent first, then all, no duplicates), focused tile scaled 1.12 with a white outline and its name below, spring-like easing, a preview strip above the bar for the focused app (continue-watching entries with progress for video, TV and library apps, then other tiles of the same category), small bell and profile buttons at the top right with the clock, anchored horizontal scrolling of the bar. D-pad: Left from the first tile goes to the round buttons, Up goes to the previews then to the bell/profile, Down returns, Enter opens, ContextMenu opens the tile menu (of the focused bar tile), Back returns to the bar. Ambient button: dark full-screen clock (any key or Back leaves it). Sources opens the file library, Options opens the tile menu, Search and Settings open RusTV's.

Approximated: no key art (tile colour with a large letter or icon); previews are left-aligned at a fixed place, not under the focused tile; the bell only shows a "no notifications" toast; the ambient mode is a clock, not art/weather scenes; a bottom widget dock sits above the bar and may cover part of the previews.

Left out: Samsung's quick settings panel, Game/Media/Daily+ hubs, voice assistant, real Source list (inputs do not exist on a PC), Art mode.

### webOS style (`webos`)

Font Noto Sans (OFL). Reproduced: blurred soft-colour wallpaper, rotating banner (8 s, cross-fade, dots, "Watch" or "Open" pill button, progress bar for resumable media), status icons at the top right (search, settings, profile, clock, phones), a "Home" label and the launcher in the lower third: system cards (Live TV, Library, Search, Settings) then 2:1 rounded app cards, focused card scaled 1.1 with a white outline and bold label below, centred label, anchored horizontal scrolling. D-pad: Left from the first app reaches the system cards, Up goes to the banner (Left/Right change the banner item, Enter opens it) then to the status icons, Down returns, ContextMenu opens the tile menu, Back returns to the launcher.

Approximated: no promotional artwork (tile colour with a large letter or icon); banner items are resume entries and a few video/TV/library/music tiles, not editorial content; the profile icon only shows the phones label; no pointer-reveal behaviour of the Magic Remote.

Left out: Home Hub quick cards, input list, app edit mode (drag to reorder), screen saver.

### Not done for all styles (needs further work)

Style-specific variants of Settings, Search, tile menu and pairing; volume and mute OSD; screensaver or ambient mode; quick-settings panel; style-specific launch transition and toast. These require new features (some native) and are listed per style in the checklists above. No bundled mod was added.

## Fonts and licences

Roboto, Montserrat (variable) and Poppins (static 400 to 700, about 54 KB in `ui/public/fonts/poppins`) from the Fontsource packages, SIL Open Font License 1.1, Latin and Latin-extended subsets in `ui/public/fonts` (about 250 KB), declared with `font-display: swap`; licences next to the files. No network access at runtime.
Roboto, Montserrat, Inter and Noto Sans (variable) from the Fontsource packages, SIL Open Font License 1.1, Latin and Latin-extended subsets in `ui/public/fonts` (about 250 KB for the first two, about 135 KB for Inter and 200 KB for Noto Sans), declared with `font-display: swap`; licences next to the files. No network access at runtime.

## Developing a style: mock harness

`ui/mock.html`, `ui/src/mock.ts` and `ui/vite.mock.config.ts` run the home in a normal browser with a fake Tauri (catalog from `catalog.json`, sample "Continue watching" entries). They are not part of `npm run build`. Run `npx vite --config vite.mock.config.ts --port 5199` in `ui/`, open `http://localhost:5199/mock.html?os=roku&keys=ArrowDown,ArrowRight,Enter`. Other parameters: `theme=light`, `lang=en`, `scale=110`, `cont=0`, `wp=bl`, `still=1` (no CSS transitions, for screenshots), `dbg=<css selectors>` (writes element rectangles into the page title).

## Not verified

- No run on a real TV session or with the real Tauri shell; no check with the phone remote.
- Mod widgets in a style were not exercised in the mock (the mock has no mods); weather refresh was not exercised (network).
- Visual comparison was done against a handful of reference screenshots, not frame by frame; motion timings are estimates.
- Light theme with a style, very small windows, portrait and 32:9 were not all inspected.
