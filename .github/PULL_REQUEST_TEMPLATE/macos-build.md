## macOS build

- Version / ref built:
- Commit (from `build-report.json`):
- macOS version and architecture (Apple silicon, Intel, or universal):
- Built with: [ ] `scripts/build-macos.sh` [ ] by hand (explain)

## Build report

<!-- Paste dist-macos/build-report.json here. It contains no secrets; check it anyway. -->

```json
```

## Files

- Release or download link:
- SHA-256 lines (`dist-macos/SHA256SUMS`):

## Changes in this pull request

<!-- One line per commit: what failed on macOS and what you changed. Write "none" if the build worked as is. -->

## What I verified

- [ ] The app starts and stays alive (smoke test in the report)
- [ ] The menu-bar icon appears
- [ ] The pairing window opens and the QR code shows
- [ ] The phone remote connects
- [ ] Media playback (mpv) works
- [ ] Other (describe):

## What I could not verify

<!-- For example: an AI agent cannot see the screen. Be honest, it helps the maintainer. -->

## Checklist

- [ ] Only macOS-specific code changed (cfg(target_os = "macos") or macOS stubs); no Windows code touched
- [ ] No certificates, tokens, passwords or personal data anywhere in the diff
- [ ] `dist-macos/` is not committed
- [ ] `Cargo.lock` unchanged, or the dependency change is explained above
