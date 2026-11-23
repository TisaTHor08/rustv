# Testing the app tiles

This page explains how the status column of [APPS.md](APPS.md) is produced, and which services still need a real account to be checked. It never contains credentials.

## What is checked

1. **Guest check** (no account): the tile opens, the launch animation ends, the page paints, and the catalog or landing page is visible. Done with the sound muted, on the TV screen, from the real application.
2. **Account check** (free or paid account supplied by the owner): sign in with the remote and the on-screen keyboard, browse, start a title, pause and resume, go back, and check that the session is still signed in after restarting RusTV.

Playback of paid content is only tried with an account the owner provides. Nothing is bought and no payment form is ever filled in.

## Services that need an account to go further

Paid subscription required:

- Netflix, Prime Video, Disney+, Max, Apple TV+, Canal+, Paramount+, OCS
- Apple Music, TIDAL
- Xbox Cloud Gaming (Game Pass Ultimate), Amazon Luna

A free account is enough (or a free tier exists):

- Crunchyroll, ADN, Spotify, Deezer, YouTube Music, Molotov
- TF1+, 6play (M6+), RMC BFM Play, RTL Play, France.tv
- GeForce NOW, Plex (account for your own library)
- Twitch, Kick (chat and follow only)

Needs your own server address: Jellyfin, Emby, UGREEN Theater.

Not available from France: Tubi.

## Handling of credentials

- Credentials are given in the chat for a test session, used only to sign in during that test and never written to files, commits, logs or memory.
- Windows keeps the session cookies in the per-app WebView2 profile under `%APPDATA%\RusTV\profiles\<tile id>`; delete that folder to forget a session.
- After a test the profiles created for it are removed.
- Services that ask for a captcha, a one-time code or a payment confirmation need the owner to complete that step.

## When it is run

Before each release, for the services that were given an account, and whenever a change touches window handling, navigation or the launch animation.
