## What and why

<!-- What does this change, and why? Link the issue if there is one. -->

## Checklist

- [ ] The change is focused (one topic) and the commit messages are clear
- [ ] Tests pass: `cargo test -p rustv-protocol -p rustv-shell`, and `npm run check` in `ui/` and `remote/` when touched
- [ ] Rust code is formatted (rustfmt) and `cargo clippy` is clean for the files I touched
- [ ] New UI strings have **both French and English** entries (`ui/src/lib/i18n.ts`, `remote/src/lib/i18n.ts`)
- [ ] Documentation updated if behavior, commands or file formats changed
- [ ] Screenshots attached for UI changes (TV and/or phone)
- [ ] No telemetry, no secrets in logs, no new Tauri capability for web app windows
- [ ] Mods: permissions impact described below

## Permissions impact (mods only)

<!-- List permissions added or changed (storage, launch, network:, inject:) and justify each one. Write "none" otherwise. -->
