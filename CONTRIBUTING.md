# Contributing to RusTV

Thank you for helping! RusTV is free software ([GPL-3.0-or-later](LICENSE)) and will stay that way. By contributing
you agree that your work is released under the same license. Please read the [Code of Conduct](CODE_OF_CONDUCT.md).

New here? Start with [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) (setup, repo map, build and debug workflow), then look
at the "Help wanted" section of the [roadmap](docs/ROADMAP.md).

## Proposing a change

1. **Open an issue first** for anything bigger than a small fix (new feature, protocol change, new permission).
   Check whether it could be a [mod](docs/MODS.md) instead: mods need no change to RusTV itself.
2. Fork the repository `TisaTHor08/rustv` and create a branch from `main`
   (for example `fix/pairing-timeout` or `feat/wake-on-lan`).
3. Make your change, keep it focused on one topic, and run the checks below.
4. Open a pull request against `main` and fill in the template. Mention the issue it closes.
5. Review comments are normal and friendly: push follow-up commits to the same branch.

Security problems must **not** go in a public issue: see [SECURITY.md](SECURITY.md).

## Commit messages

Look at `git log --oneline`: one short English line, lowercase after the prefix, no trailing period, saying what
changes and, if useful, why. Examples from the history:

```
v0.4.1: catalog detail no longer says no permission before the manifest is downloaded
registry: community mod catalog (index.json) with the 5 example mods pinned to a commit
```

Use a scope prefix when it helps (`registry:`, `docs:`, `remote:`, `ui:`, `core:`, `protocol:`, `shell:`).
Add a body for non-trivial changes. Avoid double quotes in the subject line.

## Code style

- **Rust**: rustfmt with the repository settings (`rustfmt.toml`, edition 2021, width 120) and `cargo clippy` clean
  for the code you touch. Do not reformat files you are not changing; formatting-only PRs make reviews and merges
  harder. Add unit tests next to the code (`#[cfg(test)]`). No `unwrap()` on data coming from the network, a file or
  a user.
- **Svelte and TypeScript** (`ui/`, `remote/`): Svelte 5 with runes (`$state`, `$derived`, `$effect`, `$props`),
  not the legacy `export let` / `$:` syntax. TypeScript, no `any` without a reason. `npm run check` must pass.
  2 spaces, see `.editorconfig`.
- **Comments and identifiers**: the existing code has French comments in places; English is preferred for new
  code and documentation.
- **Dependencies**: keep them few. A new crate or npm package needs a reason in the PR description.

### Internationalization (required)

RusTV is bilingual. Every user-visible string goes through the i18n tables, never hard-coded:

- TV interface: `ui/src/lib/i18n.ts`, add the key in **both** `fr` and `en`.
- Phone app: `remote/src/lib/i18n.ts`, add the key in **both** `fr` and `en`.
- Mods add their own strings through `contributes.strings` (see [docs/MODS.md](docs/MODS.md)).

## Tests and checks

```
cargo test -p rustv-protocol -p rustv-shell
cargo clippy -p rustv-protocol -p rustv-core -p rustv-shell
cd ui && npm install && npm run check && npm run build
cd remote && npm install && npm run check && npm run build
```

Full details, including debugging a running TV window, are in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## What makes a good pull request

- Small, focused, and explained: what, why, how you tested it.
- Tests for new logic (especially protocol, permissions, parsing).
- French and English strings, updated docs when behavior or file formats change.
- Screenshots or a short recording for any UI change (TV or phone).
- Respects the security rules: no telemetry, no secrets in logs, no new Tauri capability for web app windows, no
  weakening of the mod permission model. See the security section of [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).
- Does not touch unrelated files.

## Recipes

### Add an app tile

Built-in tiles are entries in [`crates/rustv-shell/src/catalog.json`](crates/rustv-shell/src/catalog.json). Copy a
neighbor and adapt it, then run `python scripts/gen-apps-doc.py` to refresh the list in [docs/APPS.md](docs/APPS.md)
(and update its `TESTED` table if you verified the app on a real account):

- Common fields: `id` (unique, lowercase), `name`, `desc`, `category` (`video`, `tv`, `music`, `library`, `games`),
  `kind` (`web` or `native`), `color` and `color2` (gradient), optional `dark_text`.
- `web`: `url`, `spatnav` (inject arrow-key navigation; `false` when the site has its own TV mode), optional `ua`
  (user-agent override, as YouTube TV does), or `needs_url` + `url_hint` when the user must type their own server.
- `native`: `target` (executable name or URI such as `steam://open/bigpicture`).

Only official websites; RusTV is not affiliated with the services it opens. If the tile does not need to ship by
default, make it a mod instead.

### Add a mod

Read [docs/MODS.md](docs/MODS.md) (manifest, permissions, SDK). Examples are in [`mods/`](mods). To list it in the
in-app catalog, open a pull request that adds an entry to [`registry/index.json`](registry/index.json) pointing to a
**full 40-character commit** of your public repository (or fill in the "Mod submission" issue form). Mods are reviewed
for honesty of their permissions; every permission must be justified and minimal.

### Add or fix a translation

French and English are complete. To fix a string, edit the `fr` / `en` tables in `ui/src/lib/i18n.ts` and
`remote/src/lib/i18n.ts`. To propose a new language, open an issue first so the language switcher and both tables
(TV and phone) can be extended together. Mods can also ship translations through `contributes.strings`.

### Add a site adapter

Adapters are JS/CSS injected into a web app (to hide an overlay, add a skip button, fix focus). They are shipped as
mods under `contributes.adapters` with `match` patterns (see [docs/MODS.md](docs/MODS.md) and
[`mods/skip-intro`](mods/skip-intro)). Adapters run inside the third-party page, so users approve `inject:<pattern>`
explicitly: keep the `match` pattern as narrow as possible and never exfiltrate page content.

## Questions

Open a GitHub issue or discussion. Beginners are welcome.
