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
| `fire` | Fire TV style | available |
| `tvos` | Apple TV style | available |

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

### Fire TV style (`fire`)

Font Roboto (OFL). Rebuilt from real screenshots of the Fire TV home (aftvnews, TechCrunch, Amazon). Reproduced: neutral dark grey background (#171717), full-bleed hero on the top 48 % with a hard cut (kicker, large letter-spaced title, description, grey "Open" pill standing for "Learn more", page dots with an elongated active dot), then the bar under the hero: round profile avatar (opens quick settings), icon-only entries (search, home, live, library) where the focused or selected one sits in a dark circle with a soft halo and its label below, six app shortcuts (small coloured 16:9 tiles) on the right, an "all apps" square (Apps grid) and a round settings gear, then rows with uppercase letter-spaced titles and tight 16:9 cards (5 visible at 1080p, 0.4 rem gap), "Continue watching" with progress bar, focus = white ring + scale 1.06. Going down scrolls hero and bar away, Up from the first row reaches the bar (zone `nav`, Left/Right move, Down returns, Enter selects or opens an app).

Common screens provided in `fire.screens/` (all nine): Settings (tile grid at the root, list plus detail pane in a page), Search (light pill field, on-screen keyboard left, 16:9 results right), tile menu (bottom-right card), pairing, quick settings (top band of cards), volume (bottom pill), screensaver (drifting "photo" gradient, clock bottom-left), toast (top right card), launch (card plus spinner).

Approximated: hero art is the tile colour with a large letter or icon, no carousel auto-advance; the profile avatar is generic; the nav shows only the Home, Library, Apps tabs (Live and Search open pages); the quick settings, volume OSD, screensaver, notifications and settings screens are NOT based on real captures (none found), only on general knowledge of the look; the newer Vega OS (icon-only bar with watchlist/games/search/check/home/live) is not reproduced separately.

Left out: profile switcher, the "Find" tab content, Alexa hint bar, sponsored rows, input panel.

Verification: home checked against the screenshots in headless Chrome (1920x1080: hero, bar, nav focus, scrolling rows, apps tab not inspected) ; screens checked once visually in the mock (settings, search, quick, pairing, tile menu, volume); ambient, toast and launch were built but not inspected.

### Apple TV style (`tvos`)

Font: system stack first (`-apple-system`, SF Pro on Apple devices), then Inter (OFL, bundled variable subsets Latin and Latin-extended, 122 KB, in `ui/public/fonts/inter`). Reproduced: pure black background with the focused tile's colour glowing, centered translucent tab capsule (Home, Apps, Library, Live, search) with a light pill on the focused tab, clock pill and round profile button at the right, full-bleed top shelf (large title, subtitle line, frosted Open button) that follows the focused tile, the first row (all apps) as 16:10 rounded icons docked at about 60 % of the height, other rows below ("Up Next" = continue watching as 16:9 cards with progress, recent, categories), focus = icon lifts, scales 1.16, tilts toward the last arrow key (`model.tilt`) then settles, moving specular highlight, long soft shadow, the glyph layer moves slightly (parallax), the name fades in under it, other icons in the row dim, other rows dimmed, the shelf fades out when the focus goes to a lower row and rows scroll up, Up from the first row reaches the capsule (zone `nav`), Apps tab = grid.

Approximated: tilt follows the arrow keys instead of a touch surface; the top shelf has no key art (tile colour plus a large letter or icon); the real Home screen has no tab capsule (it belongs to the TV app), it is kept because the model needs the tabs; no blur of the wallpaper behind the dock; the icon layers are two (background and glyph).

Common screens provided in `tvos.screens/` (all nine): Settings (big gear tile left, rounded list right, focused row = white pill), Search (centred field, round-key keyboard, result row below), tile menu (centred frosted sheet), pairing (centred), quick settings in the Control Center layout (big "Exit" tile, glass tiles, volume slider), volume pill top right (tvOS has no volume HUD on device, so this borrows the banner look), Aerial-like screensaver, glass toast, launch (icon grows to full screen). A translucent dock now sits behind the first row on the Home page.

Left out: profile switcher, app folders and the jiggle edit mode.

Verification: screenshots of tvOS were available as reference (home with the dock, TV app shelves, Control Center); the tvOS 26 translucent look is only approximated. Home, Settings, Search, tile menu and quick settings were checked in headless Chrome; pairing, volume, screensaver, toast and launch were built but not inspected. The Settings, Search and screensaver designs come from general knowledge, not from captures.

### VIDAA style (`vidaa`)

Font Roboto (OFL). Reworked against two reference images of the modern VIDAA U home (dark navy, left icon rail, framed banner, app row, "You Might Like" rows). Reproduced: near-black blue background (#10121a), a left rail of five outline icons (profile, search, notifications, inputs, settings; focused icon = white disc), a large banner with a dark oblique panel on the left (light uppercase title, outlined Open button) and the focused tile's colours on the right, dots under it, a white frame when the banner has focus, a row of small square app tiles (name under the focused one), then 16:9 poster-style rows with the title above ("Continue watching" with a progress bar, categories), focus = scale 1.06 with a white frame, banner slides away when you go down, horizontal fitting of the focused tile. D-pad: Left from the first column goes to the rail, Right returns, Up from the first row focuses the banner (zone `banner`), Down returns; rail: search and settings open them, notifications shows a toast, inputs opens the library, profile opens Settings.

Common screens provided (`vidaa.screens/`): Settings (category column plus row list with white frames), Search with a 10-column on-screen keyboard and a vertical result list, tile menu (centred dialog), pairing (centred card), quick settings as a left panel, volume bar bottom left, ambient clock with slow drift, toast bottom centre, launch (square tile and ring).

Approximated: hero art is the tile colour with a large letter or icon (no key art, no rotation); the old 2017 VIDAA launcher (coloured tiles on a wallpaper) is not reproduced; the settings layout, search, quick panel, volume OSD, screensaver and toast have NO VIDAA reference image (none found online), they use the style's palette and general knowledge only; proportions come from a 1280-wide image scaled to rem.

Left out: input sources panel contents, VIDAA's own settings tree and icons, notification centre, tile reordering, the "VIDAA Art" mode.

### Titan style (`titan`)

Font Poppins (OFL). Reproduced (checked against photos of Philips Titan OS): an icon-only top bar (search, home, apps, live, library) with a white underline under the current page, a blue underline plus label under the focused icon, settings gear and clock at the right, a full-width hero (category chip, large title, description, translucent "Open" pill, elongated page dot), the "Favourite Apps" title above a row of small rounded-square app tiles, then 16:9 rails with the row title above and the name over a bottom gradient, "Continue watching" with a progress bar, focus = scale 1.1 with a thin bright ring and a soft glow, the hero fades out and rows scroll up when you go down, Up from the first row reaches the top bar, Enter opens or switches page (Home, Apps grid, Library), Back returns to content.

Common screens provided (`titan.screens/`): Settings modelled on a photo of the Philips settings (black category column with a blue highlight bar, translucent navy middle panel with title and rows, free area on the right), Search (pill field, 12-column keyboard strip, result cards below), tile menu (centred navy dialog), pairing, quick settings as a right panel, rounded volume pill at the bottom, ambient clock inside a slowly changing coloured halo, toast under the top bar, launch (tile and pulsing dots).

Approximated: hero art is the tile colour with a large letter or icon (no key art), no auto-rotation of the hero, icon glyphs are generic (settings categories use plain bars instead of icons), the live TV preview on the right of Settings is replaced by a halo. Search, quick menu, volume OSD, screensaver, toast and launch have no reference image (none found), they follow the style's palette.

Left out: Titan's account/profile entry, the "Gaming" and "Sport" category pages, real Ambilight link, sponsored hero.

Both styles share a small helper `styles/trackfit.ts` (horizontal scroll fitting). No shared framework file was edited.
### Tizen style (`tizen`)

Font Inter (OFL). Modelled on the Samsung One UI Tizen 2024-2025 home (reference photos: home, Gaming Hub, quick settings, picture settings panel, Ambient tab). Reproduced: a vertical icon rail on the left (profile, search, ambient, library, live, home highlighted in blue, then settings and a shield at the bottom, thin blue edge line), an ambient hero of the focused app (title, description, translucent "Open" pill at the right), a pill with three tabs For You / Live / Apps at the hero's bottom, a first row of 16:9 rounded cards with the row title above (continue watching with progress, else the first category), then the row of rounded-square app icons (name only under the focused icon), then the other category rows, focused card scale 1.06 with a white ring, spring easing, vertical anchoring (the hero scrolls away from the second row), horizontal anchoring per row. D-pad: Left from the first column goes to the rail, Right returns, Up from the first row reaches the tab pill and the Open button, Down returns, Enter on Live opens Live TV, Apps shows the grid of all apps, ContextMenu opens the tile menu. Rail: profile opens quick settings, ambient shows the Ambient screen inline (any key leaves), library and live open those pages, bell/shield shows "no notifications".

Common screens (all nine in `styles/tizen.screens/`): Settings as two rounded dark cards with a light border (categories, then rows with the value as second line and a drop-down arrow, white focused row), Search with a pill field, 5-column cards and a floating on-screen keyboard (`styles/kbd.ts`, shared with webOS), tile menu as a floating card, pairing page, quick settings card at the bottom left with round icon buttons and blue pills, small volume pill at the bottom left, Ambient with layered mountains and a large clock, toast pill at the bottom centre, launch with a growing rounded icon and a ring.

Approximated: no key art (tile colour with a large letter or icon); Samsung One is not shippable so Inter stands in; the hero is not an auto-playing carousel; the Live tab only opens Live TV; the Apps row has no "edit" button; the quick settings content is RusTV's (volume, output, display, phones, settings, exit), not Samsung's picture/sound modes; the on-screen keyboard, volume pill, toast, launch and pairing layouts are NOT based on a reference (none was found) and follow the palette and shapes of the home.

Left out: Game, Media and Daily+ hubs, voice assistant, source list, Art mode, privacy panel, profile switcher.

### webOS style (`webos`)

Font Noto Sans (OFL; LG Smart UI is not shippable). Modelled on the LG webOS 24-25 home (reference photos from a preview event: home with the banner, coloured quick cards and app row, left icon column, account panel, Home Hub). Reproduced: near-black background, a column of round icons at the top left (profile, notifications, quick settings gear, search), a full-width banner with a right-aligned title and an outlined rounded button, rotating every 8 s with dots and a progress bar for resumable media, a row of flat coloured cards (green Live TV, blue Movies, purple Music, coral Photos), the row of rounded-square app icons (name under the focused one), then 16:9 content rows ("Continue watching" and the categories) that peek from the bottom edge, white thick outline and scale 1.08 on focus. D-pad: Left from the first column or the first quick card reaches the icon column, Right returns, Up from the apps row goes to the quick cards, then the banner (Left/Right change the item, Enter opens), Down goes back, ContextMenu opens the tile menu. The gear opens the quick settings panel.

Common screens (`styles/webos.screens/`): Settings full screen on near-black (category list, then grey rounded rows with the value on the right, thick white outline on focus), Search with an underlined field, cards and a flat-key keyboard, tile menu, pairing page, quick settings as a full-height panel sliding in from the left with two columns of dark cards and a volume slider, volume block at the bottom centre with a large number, screensaver of drifting colour blobs with a centred clock, toast at the top right, launch with a centred square icon and three dots.

Approximated: no promotional art (tile colour with a large letter or icon); banner items are resume entries and a few tiles, not editorial content; the quick cards are shortcuts to Live TV and the media library, not LG's "Sports" or "Home Hub"; there is no pointer-reveal behaviour of the Magic Remote (the phone remote has a pointer mode that sends mouse moves); the older webOS 6-22 bottom launcher of wide cards, which this style used before, is no longer reproduced. No reference was found for the settings, search, volume, toast, launch and screensaver screens: they use the home's palette and shapes.

Left out: Home Hub, input list, app edit mode, LG account panel, notification centre.

### Phone remotes for these two styles

`remote/src/lib/remotes/tizen.svelte` (Samsung Smart Remote: anthracite body, mic, ring with OK, back / home / play row, two rockers, solar strip) and `webos.svelte` (Magic Remote: wheel with OK, back / home / settings, volume and channel rockers, mute, streaming shortcuts, pointer mode). Both compile (`vite build` of `remote/` succeeds). Compared with the photos they are close in layout; not reproduced: the power key, the number pad and 123 key, the colour keys of the Magic Remote, the real logos. Not tried on a phone.

## Screens (extension points)

`OsStyle.screens` (`os/screens.ts`) lets a style redraw the common screens: `settings`, `search`, `tileMenu`, `pairing`, `quick`, `osd`, `ambient`, `toast`, `launch`. Each entry is a lazy loader of a Svelte component (`styles/<id>.screens/<name>.svelte`) that receives typed props (`ScreenProps[name]`). Logic (keys, data, actions) stays in the shared components (`Settings`, `Search`, `TileMenu`, `Pairing`, `os/QuickHost`, `os/SystemLayer`); a screen component only draws the props and calls the given callbacks. `ScreenSlot.svelte` loads the style's component and falls back to the default one (`os/defaults/`, the RusTV look re-skinned by tokens) when the style does not provide it or while it loads. Styles without `screens` therefore keep working unchanged (all styles except googletv, roku, tizen and webos at the time of writing). The Search screen binds the real input with `bindInput` (phone keyboard keeps working) and may take over arrow keys with `setKeyboardHandler` for an on-screen keyboard.

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

Roboto, Montserrat (variable) and Poppins (static 400 to 700, about 54 KB in `ui/public/fonts/poppins`) from the Fontsource packages, SIL Open Font License 1.1, Latin and Latin-extended subsets in `ui/public/fonts` (about 250 KB), declared with `font-display: swap`; licences next to the files. No network access at runtime.
Roboto, Montserrat, Inter and Noto Sans (variable) from the Fontsource packages, SIL Open Font License 1.1, Latin and Latin-extended subsets in `ui/public/fonts` (about 250 KB for the first two, about 135 KB for Inter and 200 KB for Noto Sans), declared with `font-display: swap`; licences next to the files. No network access at runtime.
Roboto, Montserrat and Inter (variable) from the Fontsource packages, SIL Open Font License 1.1, Latin and Latin-extended subsets in `ui/public/fonts` (about 250 KB, plus 136 KB for Inter in `fonts/inter`), declared with `font-display: swap`; licences next to the files. No network access at runtime.

## Developing a style: mock harness

`ui/mock.html`, `ui/src/mock.ts` and `ui/vite.mock.config.ts` run the home in a normal browser with a fake Tauri (catalog from `catalog.json`, sample "Continue watching" entries). They are not part of `npm run build`. Run `npx vite --config vite.mock.config.ts --port 5199` in `ui/`, open `http://localhost:5199/mock.html?os=roku&keys=ArrowDown,ArrowRight,Enter`. Other parameters: `theme=light`, `lang=en`, `scale=110`, `cont=0`, `wp=bl`, `still=1` (no CSS transitions, for screenshots), `dbg=<css selectors>` (writes element rectangles into the page title).

## Not verified

- `cargo test` for `volume.rs` could not run in the last session (disk full); the code compiles logically against `Core::volume()` but was not built.
- Screens were checked with `npm run check` and `npm run build`. Tizen and webOS home pages and screens were captured in headless Chrome through the mock harness at 1920x1080 and compared by eye with the reference photos; other aspect ratios, the light theme and animation timings of these two styles were not inspected, and D-pad navigation of the new layouts was written but not replayed key by key.
- No run on a real TV session or with the real Tauri shell; no check with the phone remote.
- Mod widgets in a style were not exercised in the mock (the mock has no mods); weather refresh was not exercised (network).
- Visual comparison was done against a handful of reference screenshots, not frame by frame; motion timings are estimates.
- Light theme with a style, very small windows, portrait and 32:9 were not all inspected.

- Phone remotes `fire` and `tvos` (remote/src/lib/remotes) were reshaped after photos of the Fire TV Stick remote and the Siri Remote; they compile (`npm run check`, `npm run build` in `remote/`) but were not looked at in a browser.
- Roku screens use i18n keys (`roku.kb.*`, `roku.search.start`, `roku.settings.open`) that are not registered anywhere, so they show the raw key.
