# Google account sharing

Settings, Integrations and hardware, **Google account**. Sign in to Google once, then the web apps that offer
"Sign in with Google" can find that session. **Off by default**: nothing happens until you turn it on.

## How it works

1. **Connect.** RusTV opens a full-screen Google sign-in window using its own web profile named `google`
   (`%APPDATA%\RusTV\profiles\google`). It presents a standard Chromium user agent, because Google refuses sign-in from
   browsers it considers embedded ("This browser or app may not be secure"). You type your e-mail, password and any 2-step
   verification **on the Google page itself**, with the phone keyboard like in every other app. RusTV never sees, receives or
   stores the password: the window has no access to RusTV's internal API and RusTV only handles the session cookies afterwards (step 2).
   Close the window with Home when you are done; RusTV then checks whether a Google session exists and shows **Connected**.
2. **Share.** Each time you open an enabled web app, RusTV reads the Google cookies from the `google` profile, keeps only those
   of `google.com` and `youtube.com` (and their sub-domains, not expired), copies them into that app's own profile, and then
   loads the app. So the "Sign in with Google" popup (`accounts.google.com`) and Google One Tap see your session.
   The cookies are kept in memory for ten minutes at most between two launches; RusTV writes none of them itself.
3. **Popups.** `window.open` from an enabled app that targets `accounts.google.com` opens as a RusTV window on the same profile
   (the link to the opening page is kept, which OAuth needs). Other popups keep the default behaviour; non-web schemes
   (`file:`, `javascript:`...) are refused.
4. **Auto-select.** A small script runs on `accounts.google.com` only. If exactly one account is listed it picks it; on the
   consent screen it presses *Continue/Allow* only when the request asks for identity scopes alone (`openid`, `email`,
   `profile`). It never acts on a password page, never on other scopes (Drive, Gmail, YouTube, calendar...), and with several
   accounts it does nothing: you choose.
5. **Disconnect** erases the whole `google` profile data and the Google cookies of every app profile that received them
   (RusTV keeps the list of those profiles in `google.json`), then forgets the connection. Turning one app off in the list
   also erases the Google cookies of that app.

## Per-app setting

Every app has an on/off row. The default comes from the tile: the catalog marks apps that offer Google sign-in with
`"googleSignIn": true` (YouTube, YouTube Music, Spotify, Deezer, SoundCloud, Vimeo, Dailymotion, Tidal, Tubi, Kick, Plex,
Rakuten TV; this list is our best knowledge and has **not** been verified service by service). A mod tile can declare it too:

```json
{ "id": "site", "name": "My site", "kind": "web", "url": "https://example.org", "googleSignIn": true }
```

A setting you change always wins over the tile's default. Apps that do not declare it never receive Google cookies unless you
enable them by hand (a custom tile with `googleSignIn: true` is honoured but not listed in the page).

## Limits, honestly

- **Google may invalidate or refuse a copied session.** Sessions are tied to the device and sometimes to the cookie
  attributes; Google can ask again for the password, a 2-step code, a phone prompt or "confirm it's you" when it sees the same
  session in another profile, an unusual user agent or a new device. Then reconnect.
- **The user agent is a Chromium one, not the real WebView2 one**, with a fixed Chrome version number. Google's detection of
  embedded browsers changes over time; sign-in could be blocked again without warning.
- **Some sites need a real browser** (anti-bot checks, DRM, or sign-in through a native Google app/FedCM prompt) and will not
  accept the session in a WebView.
- **Host-only vs domain cookies.** The cookie API used by Tauri does not say whether a cookie was a domain cookie. RusTV treats
  `google.com` and `youtube.com` cookies as domain cookies, `__Host-` cookies and sub-domain cookies as host-only.
  Rare cookies may be copied with the wrong scope.
- **Session cookies (no expiry) may not survive** closing the `google` profile; Google's main login cookies are persistent.
- **The account address is not shown.** Google's cookies do not contain it and the window has no channel to RusTV, so the
  page only shows "Connected".
- **Terms of use.** Using a Google account in an embedded browser, and copying its session between profiles, may go against
  Google's or a service's terms. You use it at your own risk; we cannot promise Google will not restrict the account.
- **Security.** Cookies of a signed-in session are as sensitive as a password: they live in the WebView2 profiles under
  `%APPDATA%\RusTV\profiles` (protected by your Windows account, not encrypted by RusTV). Anyone who can read your user folder
  can use them. Do not enable this on a shared PC; use Disconnect before giving the PC away. Enabled apps get the whole Google
  session, so only enable apps you trust.

## What was tested

Tested (automatically): which domains are copied, filtering of expired, empty, duplicate and foreign cookies, session
detection, the per-app rule (disabled by default, tile default, user override), the leading-dot handling for WebView2, the
Chromium user agent, and that catalog tiles declaring `googleSignIn` are web tiles (`cargo test`). The project and the web
interface build.

Not tested: a real sign-in (no real or test Google account was used or created), copying the session into an app and
the "Sign in with Google" button working in a real service, the account chooser and consent script on live Google pages,
disconnecting with cookies actually present, behaviour with 2-step verification, and Linux/macOS. Please report what you find.
