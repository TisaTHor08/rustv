# Web remote

Use the RusTV remote from a browser (phone, tablet or another PC on the same network) without installing the Android app.
The page can also be installed as a PWA when the browser allows it. It is **off by default** because it opens a network port.
Switch it on in Settings > Integrations > "Web remote".

## Use it

1. On the TV (or the phone-friendly settings page), open Settings > Integrations > **Web remote** and set **Web remote enabled** to Yes.
   Windows may show a firewall alert: allow RusTV on **private networks** only.
2. The **Address to open** row shows `http://<PC address>:47822` (the port is configurable). Press OK on it to get a QR code of the address.
3. Choose **Pair a browser**. A QR code appears. Scan it with the phone's camera app: the browser opens the remote and pairs itself.
   You can also open the address by hand and paste the pairing code or link, or (on a secure page, see below) scan the QR code of the
   regular Phones window with the page's camera button.
4. The remote connects. The **Connected browsers** rows list the browsers currently connected, with their address.

Each browser has its own key, exactly like a phone. Revoke it in **Phones & pairing**; it is disconnected within seconds and refused afterwards.
A pairing QR code that is not used is deleted when you close it.

## Security model

The web remote does not add a weaker path. It carries the **same protocol** as the Android app ([PROTOCOL.md](PROTOCOL.md)):

- The WebSocket (`GET /ws`) transports the same bytes as the TCP link: length-prefixed frames, mutual HMAC-SHA256 challenge-response with the
  per-device key, then AES-256-GCM with HKDF session keys and an implicit counter nonce (replay, reordering and tampering fail).
  The key never crosses the network.
- The device list is the same one: a device that is not paired, or that was revoked, is refused. Authentication happens before any command.
- The server listens only on the loopback address and on the private IPv4 addresses of this PC (never `0.0.0.0`), and drops any peer outside
  RFC 1918, link-local, loopback or IPv6 unique-local ranges.
- The `Host` header must be a private IP, `localhost`, a `.local` name or a single-label name; the `Origin` of a WebSocket must be the server itself.
  A web page from another site cannot drive the remote (DNS rebinding and cross-site WebSocket are refused).
- Rate limits per address: 6 failed authentications per minute block the address for a minute, at most 8 simultaneous connections and 300 requests
  per minute per address, 32 connections in total. Messages are bounded (2 MiB per WebSocket message, 1 MiB per protocol frame).
- Static files are served from a fixed folder with path traversal refused, `nosniff`, no referrer, and a strict Content-Security-Policy.
- Pairing never sends the key to the server: the QR code encodes `http://<address>/#p=<code>`; the part after `#` stays in the browser,
  which stores it (local storage) and removes it from the address bar. Anyone who can see the QR code can pair, so show it only to your own device.

What it does not do: the page and the WebSocket are **plain HTTP/WS**. The protocol encrypts and authenticates everything that matters, but
someone on your network can see that the page is being used, and a hostile network could serve a modified page to a browser that has not
installed anything yet. Use it on a network you trust. Do not forward the port to the Internet.

## Secure context limits (HTTP on a LAN)

Browsers only expose some features on **secure contexts**: HTTPS, or `localhost`. A page opened as `http://192.168.x.x:47822` is not one. RusTV handles it honestly:

| Feature | HTTPS or localhost | HTTP on the LAN |
| --- | --- | --- |
| Remote, pairing by link or pasted code | works | works |
| Encryption (`crypto.subtle`) | WebCrypto | automatic fallback to a pure JavaScript implementation (`@noble/ciphers`, `@noble/hashes`), byte-identical (checked by `remote/scripts/crypto-check.ts`) |
| Camera QR scan (`getUserMedia`) | works (decoded with jsQR) | not offered: scan the QR code with the phone's camera app, or paste the code |
| Service worker (offline shell) | registered | not available, the page works without |
| Install as an app (PWA) | offered by the browser | most browsers only offer "Add to home screen" as a shortcut, or nothing |

Options to get a secure context:

- **On the PC itself**: open `http://localhost:47822` (secure by definition). Installation and camera work there.
- **Chrome / Edge on Android**: `chrome://flags/#unsafely-treat-insecure-origin-as-secure`, add `http://<PC address>:47822`, relaunch. This is a
  browser setting on your own device; use it only for this address.
- **HTTPS with a certificate you trust**: put a reverse proxy in front (for example Caddy or nginx) that terminates TLS with a certificate for a
  name you control (a local CA such as `mkcert`, installed on the phone, or a real domain), and forwards to `127.0.0.1:47822`, including the
  WebSocket upgrade. The page then loads over `https://` and the remote connects with `wss://` automatically. RusTV does not generate
  self-signed certificates itself: a self-signed warning that users learn to click through would weaken the model more than it helps.

## Build and packaging

The web build is the same project as the phone app (`remote/`): `npm run build` produces `remote/dist`. The TV app serves it from a
`remote-web` folder (Tauri resource) next to the executable. `scripts/sync-web-remote.ps1` (or `.sh`) builds `remote/` and copies it
to `crates/rustv-shell/remote-web/` before `cargo tauri build`. In development, `rustv` also looks in `remote/dist` of the repository.
If no build is found, the Settings page says so and the server answers 503.

## Test it

- Rust: `cargo test -p rustv-core webremote` (handshake over WebSocket, refusal of unpaired devices and wrong keys, rate limit and blocking,
  foreign `Host` and `Origin`, static files and path traversal, framing, clean stop).
- Client against the real server: `cargo run -p rustv-core --example webremote_dev -- remote/dist 47899`, then
  `node remote/scripts/e2e-ws.ts 127.0.0.1:47899 <device> <key> [software]` (`software` forces the JavaScript crypto).
- Crypto equivalence: `node remote/scripts/crypto-check.ts` (from `remote/`).

Verified so far: the points above, plus a real Chrome (headless) driven over the LAN address (insecure context: no `crypto.subtle`, no service
worker) that paired from a `#p=` link, connected through the WebSocket with the JavaScript crypto and showed the remote; and the same server
over `localhost` (service worker registered, shell cached). **Not verified**: a real phone, an installed PWA, the camera scan, iOS Safari,
the TV settings page rendering (type-checked and built only), the Windows installer carrying the `remote-web` resource.

## Troubleshooting

- "port in use": another program uses the port; pick another in Settings (1024-65535, not 47820, 47821 or 47840).
- The page does not load from the phone: check the firewall rule (private network) and that the phone is on the same network, not on a guest network.
- "Disconnected (refused by the PC)": the browser was revoked or never paired; pair it again.
- "PC unreachable" after several failures: the address is blocked for a minute after 6 failed authentications.
