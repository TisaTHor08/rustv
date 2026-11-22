# Bitwarden integration

Typing a password on a TV with a phone is painful. RusTV can type it for you from your Bitwarden vault: focus a
login field on the PC, open the key button on the phone, pick the login, tap **Fill**.

It works with the official Bitwarden server and with self-hosted servers such as Vaultwarden.

## How it works

RusTV drives the **official Bitwarden CLI (`bw`)** installed on the PC. It never bundles, downloads or reimplements it.

1. The phone sends the master password (typed in a masked field) over the already encrypted RusTV session.
2. The PC runs `bw unlock --passwordenv ... --raw`. The password is passed through an environment variable of the child
   process, never on a command line, and is dropped (zeroed, best effort) right after. It is never stored.
3. The CLI returns a session key. It is kept **only in memory** and handed to later `bw` calls through `BW_SESSION`.
4. `bw list items --search ...` results are reduced to non-secret metadata (id, name, username, host, has-TOTP) before
   anything goes to the phone. The password field is not even parsed for the list.
5. **Fill** runs `bw get item <id>` (or `bw get totp <id>`) on the PC and types the result into the focused field with
   Unicode keystrokes (`SendInput`): username, Tab, password, optionally Enter. Nothing is put on the clipboard, shown on
   the TV, written to a log or sent to the phone.

The button is only enabled while a text field has the focus on the PC (UI Automation). RusTV also remembers which window
was in front when you tapped, and **cancels the typing if the window or field changed** while `bw` was working, so a
password is not typed into the wrong place.

With an empty search, RusTV suggests logins whose domain or name appears in the title of the foreground window.

### Locking
- Auto-lock after inactivity: default 5 minutes, configurable from 1 to 30 (TV: Settings, Bitwarden).
- The last phone disconnecting locks the vault.
- **Lock** (phone or TV) clears the session key and runs `bw lock`, which also removes the CLI's local key.
- A session key refused by `bw` locks the vault.

### `bw` detection and calls
`bw` is looked up on `PATH` and in the usual winget, Scoop, Chocolatey and npm locations. Calls run with no console
window, no stdin, `--nointeraction` and a timeout. If it is missing, the phone and the TV settings show
`winget install Bitwarden.CLI`.

## Setup
1. On the PC: `winget install Bitwarden.CLI`.
2. Self-hosted only: TV, Settings, Bitwarden, Server, enter the URL (the CLI only allows changing it while logged out;
   use "Log out of the vault" first if needed).
3. Phone: key button, **Log in** once (email, master password, two-step code if asked: method app, email or YubiKey).
   Later, **Unlock** with the master password only.
4. Focus the login field on the PC, open the key button, search or pick a suggestion, then Fill / Fill + Enter /
   Username / Password / TOTP.

API-key login (`BW_CLIENTID`/`BW_CLIENTSECRET`) is not driven by RusTV; if you already logged in that way in a terminal,
Unlock works as usual. Vault data is read from the CLI's local cache; run `bw sync` (or any Bitwarden app) to refresh it.

## Threat model
Protected:
- The master password is not stored, not logged, not on a command line, not shown on the TV, not sent back.
- The session key is memory-only, wiped on lock, timeout and last disconnect; `bw lock` also removes the CLI's copy.
- The phone only ever receives non-secret metadata. A stolen or malicious paired phone cannot read passwords through the
  protocol (but see below).
- No clipboard use, so clipboard managers and history do not see secrets.
- Secrets are typed only while the same window and a text field are focused.

Not protected:
- A paired phone that is unlocked in someone's hands can make the PC type secrets into whatever field is focused, and can
  unlock if the master password is known. Revoke lost phones in "Phones".
- Anyone with local code execution as your Windows user can read process memory, environment blocks or the CLI data
  file; it can also read the screen. Typed keystrokes can be captured by keyloggers.
- Typing into a field shows the password only if the site shows it; fields that are not password fields reveal it on
  screen (and on the TV). Prefer Fill on masked fields.
- Zeroing is best effort: Rust/OS copies (process environment, decoded JSON, the encrypted frame buffer) cannot all be wiped.
- The two-step code is passed as `--code` on the `bw` command line for a few seconds (the CLI has no environment option);
  it is single-use. The account email is also on that command line.
- RusTV does not verify a site's URL: matching by window title is only a convenience for ordering results.

## Files
`crates/rustv-core/src/vault.rs` (CLI driver, state machine, fill), protocol messages in
`crates/rustv-protocol/src/messages.rs`, phone UI in `remote/src/lib/Vault.svelte`, TV settings in `Settings.svelte`.
The only persisted setting is the auto-lock delay (`%APPDATA%\RusTV\vault.json`).
