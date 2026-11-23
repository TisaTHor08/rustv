# macOS build, for an AI coding agent

This page is written to be handed to an AI coding agent (Claude Code, Codex, or similar) running on a Mac, so that it clones RusTV, builds the macOS version, fixes macOS-only problems, and sends the result back as a pull request, with little help from the human. The human version of the procedure is [BUILD-MACOS.md](BUILD-MACOS.md).

## Two tasks in this Mac session

1. **Task 1, the macOS app** (this page): build RusTV for macOS, fix macOS-only problems, open a pull request.
2. **Task 2, the iOS remote app** ([AGENT-BUILD-IOS.md](AGENT-BUILD-IOS.md)): create the iPhone version of the phone remote (a Swift Capacitor plugin and an Xcode project), build it for the iOS simulator with `scripts/build-ios.sh`, open a second pull request. Do task 2 right after task 1, on its own branch, unless the human says to skip it. Task 2 needs full Xcode (from the App Store), not just the Command Line Tools.

## For the human: what to do

1. Make sure the Mac has the **Xcode Command Line Tools** (and, for the iOS task, **Xcode** from the App Store, opened once to accept its licence) (`xcode-select --install`, a window opens and needs your click), **Node.js 20+** (`brew install node`) and **Rust** (official installer from <https://rustup.rs>). The agent cannot click system dialogs for you.
2. Make sure you are signed in to GitHub in the terminal: `gh auth login` (GitHub CLI: `brew install gh`) or have git credentials configured. Fork <https://github.com/TisaTHor08/rustv> on GitHub first, or let the agent do it with `gh repo fork`.
3. Open your agent in an empty folder and paste the block below. Replace the two placeholders. Stay nearby: the first build takes 20 to 40 minutes, and the agent may ask you to click a system dialog.

## Block to paste into the agent

```text
You are helping build the macOS version of RusTV, an open-source (GPL-3.0) TV system written in Rust (Tauri 2) with a Svelte interface. The project lives at https://github.com/TisaTHor08/rustv and is developed on Windows. Your job: build the macOS app from the commit I name, fix macOS-only build problems, verify it starts, and send everything back as a pull request. Work autonomously, but follow the rules.

Inputs
- GitHub user (the fork owner): <MY_GITHUB_USER>
- Ref to build (a tag like v1.3.0, a branch, or "main"): <REF>

Rules
- Read docs/AGENT-BUILD-MACOS.md, docs/BUILD-MACOS.md and docs/PORTING.md in the repository first and follow them; they have the details and the stub convention.
- Only change code that is macOS specific: code behind #[cfg(target_os = "macos")], macOS stubs for functions that have no macOS version yet, the macOS bundle settings, scripts and docs. Never edit code behind cfg(windows) and never reformat unrelated files. Keep the diff small.
- Never create, ask for, print, or commit secrets, certificates, tokens, passwords or personal data. Do not sign or notarize. Do not upload anything anywhere except to my fork and the pull request.
- Install only what the docs list (Xcode Command Line Tools, Rust via rustup.rs, Node 20+, Tauri CLI via cargo). If a system dialog needs a human click (for example xcode-select --install), stop and tell me exactly what to click.
- Do not run commands with sudo unless the docs say so and tell me first.

Steps
1. If not already done: fork and clone. Using the GitHub CLI: gh repo fork TisaTHor08/rustv --clone --remote, then cd rustv. Otherwise clone https://github.com/<MY_GITHUB_USER>/rustv.git and add the remote upstream https://github.com/TisaTHor08/rustv.git. Fetch upstream with tags, check out <REF>, and create the branch macos/<MY_GITHUB_USER>-<YYYYMMDD>.
2. Run: scripts/build-macos.sh   (add --universal if I asked for a universal app). Wait for it; it takes a long time the first time.
3. If it fails (exit code 2), read the end of dist-macos/build.log, find the first real error, fix it following the rules, and run the script again. Make one commit per fix with a clear message ("macOS: <what and why>"). Stop after 15 attempts and report what blocks you. Exit code 3 means a prerequisite is missing: install it as the message says, or ask me.
4. When the build succeeds, check dist-macos/build-report.json: steps.tests, steps.bundle and steps.smoke_test should be ok / started_and_stayed_alive. If tests failed because of a platform assumption (paths, separators, Windows-only API), make the test portable; never delete or weaken a test to make it pass.
5. Commit your changes (do NOT commit dist-macos/, it is git-ignored), push the branch to my fork: git push -u origin HEAD.
6. Upload dist-macos/*.dmg, dist-macos/*.app.zip and dist-macos/SHA256SUMS to a draft release on my fork named macos-<short commit sha> (gh release create ... --draft --repo <MY_GITHUB_USER>/rustv), or tell me where else you put them.
7. Open a pull request to TisaTHor08/rustv with the template .github/PULL_REQUEST_TEMPLATE/macos-build.md filled in: gh pr create --repo TisaTHor08/rustv --base main --head <MY_GITHUB_USER>:<branch> --title "macOS build <version> (<arch>)" --body-file <filled template>. Paste the content of build-report.json, the release link and the SHA-256 lines in the body.
8. If you cannot push or open the pull request (no credentials), create a git bundle (git bundle create dist-macos/macos.bundle upstream/main..HEAD) and tell me exactly which files in dist-macos/ to send to the maintainer.
9. Finish with a short summary for me: commit built, macOS version and architecture, result of each step, what you changed and why, the links, and anything you could not verify (for example: you cannot see the screen, so you did not check the interface visually).

10. When the macOS pull request is open, continue with task 2: read docs/AGENT-BUILD-IOS.md in the repository and do it exactly as written (separate branch ios/<MY_GITHUB_USER>-<YYYYMMDD>, separate pull request, same rules). If full Xcode is not installed, tell me how to install it and wait.

Definition of done: the macOS branch is pushed and its pull request is open (or the bundle is ready), the report and the checksums are attached, task 2 is done the same way (or I told you to skip it), and I have your summary of both.
```

## Reference for the agent

### What a good result contains

- `dist-macos/build-report.json` with `steps.ui = ok`, `steps.tests = ok`, `steps.bundle = ok`, `steps.smoke_test = started_and_stayed_alive`.
- The `.dmg` and the `.app.zip`, with their SHA-256.
- Commits that only touch macOS code, with messages explaining each fix.
- An honest list of what was not verified. The agent cannot see the screen unless it takes screenshots with macOS tools; if it can, a screenshot of the menu-bar icon and the pairing window is welcome (no personal data in it).

### Known expectations

- Several features are stubs on macOS (see [PORTING.md](PORTING.md)): keyboard and mouse injection from the phone, system volume, window guard, display modes, game controller, secure storage. The app should still start, show the TV interface, accept phone connections and play media; features that are stubs must fail visibly with `unsupported_platform` rather than pretend to work.
- Web apps (Netflix and so on) use the system web view on macOS. DRM streaming services are not expected to play there; Chromium-based browsers in app mode are the planned route. Do not try to work around that in this task.
- The app is not signed. Do not try to sign it.

### If something looks dangerous

Stop and ask the human before: running anything with `sudo`, downloading and executing scripts other than the official `rustup.rs` installer and Homebrew, changing system settings, touching files outside the repository folder and the Rust and Node tool folders, or contacting services other than GitHub, crates.io, npm and Apple's developer tools.

### Maintainer notes

- A pull request from a macOS contributor is reviewed like any other. The binary from a third party must not be published as is: the maintainer rebuilds from the same commit on a Mac (or asks a second contributor) and compares the SHA-256 and `cargo_lock_sha256` from the report before attaching a macOS file to a release.
- `Cargo.lock` must be unchanged by a macOS build, except for intentional dependency changes that are explained in the pull request.
