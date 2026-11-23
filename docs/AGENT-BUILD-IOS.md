# iOS remote app, for an AI coding agent

RusTV has an Android phone remote (`remote/`, Svelte + Capacitor). There is no iOS version yet. This page is the playbook to create it on a Mac, with Xcode, and send it back as a pull request. It is task 2 of the Mac session: do it after the macOS app pull request described in [AGENT-BUILD-MACOS.md](AGENT-BUILD-MACOS.md), as a second branch and a second pull request, unless the human tells you otherwise. The rules of that page (scope, secrets, no sudo, ask before anything dangerous) apply here too.

## What exists and what to write

The remote is a web app (Svelte) wrapped by Capacitor 8. Everything is shared with Android except one native plugin, `RusTVTransport`, which gives the web code raw TCP sockets and a UDP listener (web views cannot open raw sockets). On Android it is `remote/android/app/src/main/java/com/rustv/remote/RusTVTransportPlugin.java`. The web side that calls it is `remote/src/lib/native.ts`. You must write the same plugin in Swift, so that the unchanged web code works on iOS.

### Plugin contract (Capacitor plugin name `RusTVTransport`)

All binary data crosses the bridge as base64 strings. Connection ids are positive integers chosen by the plugin.

| Method | Arguments | Result | Behaviour |
|---|---|---|---|
| `tcpConnect` | `{ host: string, port: number, timeoutMs: number }` | `{ id: number }` | Open a TCP connection with the timeout, start reading, register it under a new id. Reject with a short message on failure. |
| `btConnect` | `{ address: string, uuid: string }` | rejects `unsupported_ios` | Classic Bluetooth RFCOMM does not exist on iOS. The web code handles the rejection and uses Wi-Fi. |
| `write` | `{ id: number, data: string }` (base64) | none | Write the decoded bytes to connection `id`, reject if unknown or closed. |
| `close` | `{ id: number }` | none | Close and forget the connection. Closing an unknown id is not an error. |
| `startBeacon` | `{ port: number }` | none | Listen for UDP datagrams on `port` (the PC broadcasts its beacon on 47821 by default). For each datagram emit `beacon`. Stop any previous listener first. |
| `stopBeacon` | none | none | Stop the UDP listener. |

Events to emit with `notifyListeners`:

| Event | Payload | When |
|---|---|---|
| `data` | `{ id: number, data: string }` (base64) | bytes read from a connection (any chunk size up to 8 KB is fine) |
| `closed` | `{ id: number }` | the connection ended, by either side or by an error; emit once |
| `beacon` | `{ ip: string, payload: string }` | one UDP datagram; `ip` is the sender address as text, `payload` is the datagram decoded as UTF-8 |

Implementation guidance (Swift): use Network.framework (`NWConnection` for TCP, `NWListener` or a BSD UDP socket for the beacon), keep a dictionary of connections guarded by a serial queue, never block the main thread, make `write` back-pressure safe (completion handlers), and close everything in `deinit`/`handleOnDestroy`. Register the plugin the way Capacitor 8 documents for in-app Swift plugins (`CAPBridgedPlugin` with `identifier`, `jsName = "RusTVTransport"`, `pluginMethods`; add it to the bridge in the app's view controller if the project template needs a manual registration). Check the real current Capacitor 8 documentation and the generated project instead of trusting this paragraph if they differ.

Protocol facts that matter: [PROTOCOL.md](PROTOCOL.md) (framing and encryption are done in TypeScript, the plugin only moves bytes). The PC listens on TCP 47820.

## Steps

1. Branch from the latest `main` (fetch upstream first): `git checkout -b ios/<your-user>-<YYYYMMDD>`.
2. In `remote/`: `npm ci`, then add the iOS platform: `npm install @capacitor/ios@^8` (same major version as the other Capacitor packages in `package.json`, check `npx cap --version`) and `npx cap add ios`. Commit what Capacitor generates for source control (`remote/ios/App/...`, the Xcode project, `Package.swift` or `Podfile` and `Podfile.lock` depending on the template) but not build products: no `Pods/`, `DerivedData/`, `build/`, `xcuserdata/`. Extend `.gitignore` accordingly.
3. Write the Swift plugin (`RusTVTransportPlugin.swift`) per the contract above and add it to the app target. Keep the plugin file self-contained and commented in English.
4. Configure the app:
   - `Info.plist`: `NSLocalNetworkUsageDescription` (for example "RusTV connects to your PC on the local network."), `NSCameraUsageDescription` (QR code pairing), a portrait-only orientation set for iPhone, display name "RusTV". The bundle identifier stays `com.rustv.remote`.
   - Local network permission: iOS asks the user the first time the app talks to a local address. The first `tcpConnect` may fail while the prompt is shown: make sure the web code retries (it already reconnects in a loop; verify on the simulator).
   - UDP broadcast and multicast reception on a real device need the Apple entitlement `com.apple.developer.networking.multicast`, which must be requested from Apple and is not available to free accounts. Do not add the entitlement. The beacon is only a convenience: pairing by QR code stores the PC's addresses and works without it. `startBeacon` must therefore resolve without error even if no datagram ever arrives, and the web code must not depend on the beacon. Document this in `docs/IOS.md`.
   - Safe areas: check the layout with `env(safe-area-inset-*)` (notch and home indicator) in `remote/index.html` (`viewport-fit=cover`) and the CSS; fix only what is broken on iPhone.
5. Guard Android-only calls in the web code (minimal, behind `Capacitor.getPlatform()`):
   - `remote/src/lib/Pair.svelte` calls `BarcodeScanner.isGoogleBarcodeScannerModuleAvailable()` and `installGoogleBarcodeScannerModule()`, which exist only on Android: call them only when the platform is `android`. The ML Kit scanner plugin supports iOS 15 and newer for the scan itself.
   - `window.RusTVNative.setVolumeCapture` (hardware volume buttons control the PC volume) is Android-only and already optional-chained; on iOS the on-screen volume bar is used. Do not try to intercept the iPhone volume buttons.
   - Haptics (`@capacitor/haptics`) works on iOS as is.
6. App icon and launch screen: generate the iOS icon set from the RusTV logo (`crates/rustv-shell/icons/icon.png`, 1024 px source) into `remote/ios/App/App/Assets.xcassets/AppIcon.appiconset`, and keep the default launch storyboard with a dark background (`#06080c`).
7. Build and test on the simulator with the script: `scripts/build-ios.sh`. It builds the web app, runs `npx cap sync ios`, builds the app for the iOS simulator without code signing, boots a simulator, installs and launches the app, takes a screenshot, and writes `dist-ios/` (`RusTV-Remote-simulator.app.zip`, `launch.png`, `build-report.json`, `build.log`). Fix what fails (Swift compile errors first).
8. Real behaviour check (the simulator shares the Mac's network): start RusTV on a PC on the same network if the human has one, or at least verify that the app launches, shows the pairing screen, opens the camera prompt on the QR button (the simulator has no camera: the permission prompt and the error handling are what you can check), and that a connection attempt to an unreachable address fails cleanly and retries instead of crashing. Say exactly what you could and could not check.
9. Write `docs/IOS.md` (English, no emoji): what the iOS app is, how to build and run it (simulator, then a real iPhone with a free "Personal Team" signing identity: profiles last 7 days, then TestFlight or the App Store with a paid Apple developer account), the plugin contract, the beacon limitation and the entitlement, what was verified. Link it from `docs/README.md` and the README. Add a CHANGELOG line under `[Unreleased]`.
10. Commit in small steps (messages like `ios: Swift transport plugin`), push the branch to the human's fork, and open a pull request to `TisaTHor08/rustv` with the `macos-build.md` template adapted (or a plain description): report content, what was verified, what was not, screenshots (no personal data), the simulator zip link and its SHA-256.

## Do not

- Do not commit signing identities, provisioning profiles, team ids, API keys, `.p12` files or `xcuserdata`. Use `CODE_SIGNING_ALLOWED=NO` for simulator builds; leave `DEVELOPMENT_TEAM` empty in the committed project and say in `docs/IOS.md` how a developer sets their own team in Xcode.
- Do not change the TV application, the Android project or the protocol.
- Do not publish anything to the App Store, TestFlight or any store.

## Definition of done

The branch is pushed, the pull request is open, `scripts/build-ios.sh` produced `dist-ios/build-report.json` with the simulator build and launch steps `ok`, the plugin implements every method and event of the contract, and the summary lists what is verified and what is not (for example: no real iPhone, no real PC pairing).
