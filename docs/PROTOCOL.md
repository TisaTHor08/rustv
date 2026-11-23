# RusTV wire protocol (v1)

The web remote ([WEB-REMOTE.md](WEB-REMOTE.md)) carries these same bytes in binary WebSocket messages (`GET /ws`, port 47822 by default); the
handshake, framing and encryption below are unchanged, and message boundaries carry no meaning (frames are re-assembled from the byte stream).

All integers are big-endian. The same bytes flow over TCP (port 47820) and Bluetooth RFCOMM
(service UUID `7c1d3a52-5b0e-4f8a-9d63-2e41c0a8b9f7`).

## Framing
`[u32 length][payload]`, maximum 1 MiB per frame.

## Pairing (QR code content, JSON)
`{ v, id, name, device, key (base64, 32 bytes), ips[], port, bt? }`
One pairing = one `device` id + one fresh 32-byte key, revocable on its own.

## Handshake (clear-text JSON frames)
1. client → server `Hello { v, device_id, nc }` (`nc`: 32 random bytes, base64)
2. server → client `Challenge { ns, proof_s }`, `proof_s = HMAC-SHA256(K, "S" | device_id | nc | ns)`
3. client verifies `proof_s` (the PC proves it knows `K`), then sends `Response { proof_c }`,
   `proof_c = HMAC-SHA256(K, "C" | device_id | nc | ns)`
4. server → client `Verdict { ok }`

Unknown devices get `Verdict { ok: false }` instead of a challenge.

## Session
Keys: `HKDF-SHA256(secret = K, salt = nc | ns, info = "c2s" | "s2c")`, 32 bytes each.
Each frame payload is the AES-256-GCM ciphertext (tag appended) of one JSON message.
Nonce = 4 zero bytes + 64-bit message counter (per direction, starts at 0). The counter is implicit:
replay, loss or reordering makes decryption fail and the connection is dropped.

## Messages (JSON, tag field `t`)
Phone → PC: `key{k: up|down|left|right|enter|back|backlong|menu|search}`, `altTab{a: open|left|right|confirm}`, `pin`,
`text{backspaces, text}`, `mouse{dx, dy, click?: left|right, scroll}`, `launch{appId}`, `getApps`,
`home`, `tv{on}`, `openTile{id}` (open a tile of the TV home by id), `getRemoteMods`, `media{a: playPause|next|prev|stop|volUp|volDown|mute}`, `setVolume{level 0-100}`, `ping`.

`menu` is injected as the Windows context-menu key (VK_APPS); the TV home screen uses it to open a tile's options.

PC → phone: `apps{apps[{id,name,icon}]}`, `icons{icons[{id,icon}]}` (PNG base64, sent in batches),
`state{pinned, textFocus, tvMode, osStyle?}`, `nowPlaying{active, title, artist, playing, position, duration, source}`, `volume{level 0-100, muted}` (PC volume, sent on connect and on every change), `remoteMods{mods}`, `pong`. Each `ping` is answered by `pong` followed by a fresh `state`, so a lost or stale state (for example `textFocus`) is reconciled within one ping period (4 s) and on app resume; the phone also clears `textFocus` when the link drops.

`osStyle` (optional, string) is the TV home style the PC currently shows: one of `googletv`, `tizen`, `webos`, `vidaa`, `titan`, `fire`, `tvos`, `roku`.
It is omitted for the default `rustv` style, and omitted by older PCs; older phones ignore it. The phone uses it to pick its remote model
(see [REMOTE-MODELS.md](REMOTE-MODELS.md)); an unknown value falls back to `rustv`. A fresh `state` is broadcast when the style changes.

`search` is injected as F3 (it opens the TV home screen's universal search).

### Remote mods
`getRemoteMods` asks for the phone remote contributions of the enabled mods; the PC answers with `remoteMods{mods}` and also
broadcasts it to connected phones when mods are enabled or disabled. The bundle holds themes (CSS variables and optional CSS)
and button panels (sequences of whitelisted actions). The PC sanitizes it (lengths, unsafe CSS or variables, unknown actions,
clamped numbers) and the phone validates it again. A button runs its actions by sending ordinary messages (`key`, `media`,
`setVolume`, `openTile`, `home`, `tv`, `altTab`, `text`), so no new capability exists. See [MODS.md](MODS.md#phone-remote-contributions).

### Bitwarden vault messages (see [BITWARDEN.md](BITWARDEN.md))
Phone → PC: `vaultStatus`, `vaultUnlock{password}`, `vaultLogin{email, password, code?, method?: 0|1|3}`, `vaultLock`,
`vaultSearch{q}` (empty `q` = suggestions from the foreground window title),
`vaultFill{id, what: both|username|password|totp, enter?}`.
`password` is the master password: it is only ever sent inside the encrypted session and is never echoed back.

PC → phone: `vaultState{status: unknown|notInstalled|loggedOut|locked|unlocked, email, autolockMin}` (broadcast on every change),
`vaultItems{items[{id, name, username, host, hasTotp}]}` (non-secret metadata only),
`vaultResult{op, ok, code}` where `code` is a stable error code (`badPassword`, `need2fa`, `locked`, `loggedOut`, `notInstalled`,
`timeout`, `noFocus`, `focusChanged`, `invalid`, `empty`, `failed`), never a raw message. Secrets are never sent to the phone:
`vaultFill` makes the PC type them into the focused field.

## Discovery
UDP broadcast every 2 s on port 47821: `{"app":"rustv","id","name","port"}`. Unauthenticated hint only;
the handshake is what establishes trust.

## Safety behaviours
- Alt held for Alt+Tab is released after 5 s without activity, on OK, or when the last phone disconnects.
- The PC drops a connection that is silent for 15 s (the phone pings every 4 s) and, for a revoked
  pairing, at the next received frame.
