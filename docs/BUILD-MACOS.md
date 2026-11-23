# Building RusTV for macOS

RusTV is developed on Windows. The macOS version is built and verified by contributors who own a Mac. This page is the complete procedure, for a person. If you prefer to let an AI coding agent do it, give it [AGENT-BUILD-MACOS.md](AGENT-BUILD-MACOS.md) instead: it contains a ready-to-paste block.

Status: the macOS port is a work in progress. The shared code (interfaces, phone protocol, parsing, search, recording) is portable. Parts that talk to Windows APIs (key injection, system volume, window guard, game controller, display modes) are stubs on macOS until their macOS backends exist, see [PORTING.md](PORTING.md). A first build may therefore need small macOS-only fixes: that is expected, and fixing them is part of the job.

## What you need

| Item | Why | How |
|---|---|---|
| A Mac, macOS 12 or newer, Apple silicon or Intel | native build | |
| Xcode Command Line Tools | compiler, linker, SDK | `xcode-select --install`, accept the window, wait |
| Rust (stable) | the app | official installer from <https://rustup.rs>, then open a new terminal |
| Node.js 20 or newer | the TV interface | `brew install node` (or <https://nodejs.org>) |
| Git | clone and push | included with the Command Line Tools |
| A GitHub account | to send the result back | free |
| About 10 GB of disk and 20 to 40 minutes | first build | later builds are much faster |

No Apple developer account is needed. The app is not signed (see "Opening an unsigned app" below).

## Step by step

1. **Fork** <https://github.com/TisaTHor08/rustv> on GitHub (button "Fork"), then clone your fork:

   ```sh
   git clone https://github.com/<your-user>/rustv.git
   cd rustv
   git remote add upstream https://github.com/TisaTHor08/rustv.git
   git checkout -b macos/<your-user>-$(date +%Y%m%d)
   ```

   To build a specific release instead of the latest code: `git fetch upstream --tags && git checkout v1.3.0` (use the tag you were asked to build), then create your branch from it.

2. **Build**:

   ```sh
   scripts/build-macos.sh
   ```

   The script checks the prerequisites (and tells you what is missing), builds the interface, runs the tests, builds the `.app` and the `.dmg`, starts the app for 8 seconds as a smoke test, and writes everything to `dist-macos/`:

   - `RusTV_<version>_<arch>.dmg` and `RusTV_<version>_<arch>.app.zip`
   - `SHA256SUMS`
   - `build-report.json` (machine readable summary) and `build.log` (full log)

   Add `--universal` to produce one app that runs on both Apple silicon and Intel Macs.

3. **If it fails**, read the end of `dist-macos/build.log`. Typical causes:
   - missing Command Line Tools or an old Rust: re-run after `rustup update stable`
   - a compile error in code that has no macOS version yet: fix it only inside `#[cfg(target_os = "macos")]` code or by adding a macOS stub (see [PORTING.md](PORTING.md) for the stub convention), then re-run
   - a failing test: if it is a path or platform assumption, make the test portable; do not delete it

4. **Try it** (optional but valuable): open the app from `dist-macos/`, see that the RusTV icon appears in the menu bar, open "Phones", scan the QR code with the Android remote, and check what works. Write down what works and what does not in your pull request.

5. **Send it back** (pull request):

   ```sh
   git add -A
   git commit -m "macOS: build fixes and report"
   git push -u origin HEAD
   ```

   Then on GitHub open a pull request from your branch to `TisaTHor08/rustv`, choose the template "macOS build" (the link is `.../compare/main...<your-user>:macos/...?template=macos-build.md`) and paste the content of `dist-macos/build-report.json`. Upload the `.dmg` and the `.app.zip` to a draft release on your fork (or any file host) and put the link and the SHA-256 in the pull request. Do not commit the `dist-macos/` folder: it is ignored by git.

   The maintainer reviews the changes, may rebuild from the same commit to check the checksum, and merges.

## Opening an unsigned app

The app is not signed or notarized, so macOS blocks it the first time. After copying `RusTV.app` to `/Applications`:

```sh
xattr -dr com.apple.quarantine /Applications/RusTV.app
```

or right-click the app, choose "Open", then "Open" again. Signing and notarization need a paid Apple developer account and will be set up separately.

## Rules for macOS changes

- Do not change Windows code paths (`cfg(windows)`) and do not reformat unrelated files.
- Keep every macOS change behind `cfg(target_os = "macos")` or in a macOS-specific file.
- Never commit certificates, passwords, tokens or personal data. The build report contains no secrets; check it before pasting.
- Keep pull requests small and describe exactly what you ran.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `xcode-select: error: command line tools are already installed` but the build cannot find a compiler | `sudo xcode-select --reset`, then retry |
| `error: linker 'cc' not found` | install the Command Line Tools |
| `failed to run custom build command for ...` | read the lines just above it in `build.log`; usually a missing tool |
| `npm ci` fails | check `node -v` is 20 or newer, delete `ui/node_modules`, retry |
| The app starts and quits immediately | run `dist-macos/../target/release/bundle/macos/RusTV.app/Contents/MacOS/rustv` from a terminal to see the error and include it in the pull request |
| Out of disk space | `cargo clean` removes the `target` folder (rebuild takes a while) |
