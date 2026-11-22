# RusTV wire protocol (v1)

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
`home`, `tv{on}`, `media{a: playPause|next|prev|stop|volUp|volDown|mute}`, `ping`.

`menu` is injected as the Windows context-menu key (VK_APPS); the TV home screen uses it to open a tile's options.

PC → phone: `apps{apps[{id,name,icon}]}`, `icons{icons[{id,icon}]}` (PNG base64, sent in batches),
`state{pinned, textFocus, tvMode}`, `nowPlaying{active, title, artist, playing, position, duration, source}`, `pong`.

`search` is injected as F3 (it opens the TV home screen's universal search).

## Discovery
UDP broadcast every 2 s on port 47821: `{"app":"rustv","id","name","port"}`. Unauthenticated hint only;
the handshake is what establishes trust.

## Safety behaviours
- Alt held for Alt+Tab is released after 5 s without activity, on OK, or when the last phone disconnects.
- The PC drops a connection that is silent for 15 s (the phone pings every 4 s) and, for a revoked
  pairing, at the next received frame.
