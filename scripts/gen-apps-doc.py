#!/usr/bin/env python3
"""Regenerates docs/APPS.md from crates/rustv-shell/src/catalog.json.

Run it after adding or changing a catalog entry:  python scripts/gen-apps-doc.py
Add `--check` to fail (exit 1) when docs/APPS.md is out of date (useful in CI).

The "Tested" column is not derived: edit TESTED below when you verify an app on a real account or device.
"""
import json
import sys
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / "crates/rustv-shell/src/catalog.json"
OUT = ROOT / "docs/APPS.md"

# id -> (status, note). Anything absent is "not tested yet". Statuses: ok, partial.
TESTED = {
    "youtube": ("ok", "Guest playback checked on a second display (Vivaldi app mode); search typing and D-pad work. Some letters can be dropped by YouTube TV's on-screen search."),
    "ugreen": ("ok", "Checked over UGREEN Link: login with two-factor, UGOS desktop, Theater library and film playback (phone remote: arrows on the desktop, touchpad click on Play). The tile accepts several addresses and uses the first one that answers."),
    "francetv": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the page loads and the catalog shows behind the cookie banner; playback not tried. First paint can take 15 to 30 s."),
    "arte": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): home page and featured films load; cookie banner appears first. Free to watch; playback not tried."),
    "twitch": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the live directory loads without an account; log in to chat or follow. Playback not tried."),
    "pluto": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the landing page loads (consent banner first), with a Watch free TV button; playback not tried."),
    "tf1": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): home page loads behind a consent wall; free catalog visible. Many programmes need a free account. Playback not tried."),
    "m6": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): loads behind a consent wall; part of the catalog is paid (M6+ Max). Playback not tried."),
    "rmcbfm": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): home page loads behind a consent banner; catalog visible. Playback not tried."),
    "tubi": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): Tubi answers \"not currently available in your area\" from France: the service is US only."),
    "vimeo": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the home page loads (slow first paint, around 30 s); cookie banner shown. Playback not tried."),
    "dailymotion": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): loads, but only the privacy dialog was visible after 25 s; accept it with the remote to continue. Playback not tried."),
    "odysee": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): loads after about 45 s (cold profile); the Discover page and a consent banner appear. Playback not tried."),
    "kick": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the live directory and a featured stream load without an account. Playback not tried."),
    "adn": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the anime catalog loads behind a consent dialog; some titles need an account. Playback not tried."),
    "rtlplay": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): loads behind a consent dialog; catalog visible. Playback not tried."),
    "ytmusic": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): loads (Google consent page first); signing in is needed for your library. Playback not tried."),
    "soundcloud": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the home page loads without an account; consent banner shown. Playback not tried."),
    "radiofrance": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the home page and live radios load after a consent dialog. Playback not tried."),
    "rakuten": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the catalog loads without an account (free and paid titles). Playback not tried."),
    "crunchyroll": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the landing page loads (offers shown first); watching needs an account, free or paid. Not tested further."),
    "spotify": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the web player loads and shows browsing pages; playing needs an account. Not tested further."),
    "plex": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the free ad-supported catalog and live TV load without an account (consent dialog first); your own library needs a Plex account. Not tested further."),
    "emby": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the page stays dark: Emby needs your own server address, nothing to show without one. Not tested further."),
    "geforcenow": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the landing page loads behind a consent dialog; playing needs an NVIDIA account and a compatible plan. Not tested further."),
    "deezer": ("partial", "Guest load check (2026-11-23, 1080p, no account, sound muted): the landing page loads behind a cookie dialog; free listening needs an account. Not tested further."),
}

CATEGORIES = [
    ("video", "Video and streaming"),
    ("tv", "Live and replay TV"),
    ("music", "Music and radio"),
    ("library", "Your own library"),
    ("games", "Games"),
]

STATUS = {"ok": "Tested", "partial": "Partly tested", None: "Not tested yet"}


def row(t):
    status, note = TESTED.get(t["id"], (None, ""))
    kind = t.get("kind", "web")
    if kind == "native":
        target = "Windows app"
    else:
        host = urlparse(t.get("url") or t.get("url_hint") or "").hostname or "your own address"
        target = host
    extras = []
    if t.get("needs_url"):
        extras.append("asks for your server address")
    if t.get("adblock_browser"):
        extras.append("opens in Brave/Vivaldi if installed (ad blocker)")
    if t.get("allow_insecure_lan"):
        extras.append("accepts self-signed HTTPS on private addresses")
    if t.get("spatnav", False) and kind != "native":
        extras.append("D-pad navigation")
    desc = t.get("desc", "")
    return f"| {t['name']} | {desc} | {target} | {'; '.join(extras)} | {STATUS[status]} | {note} |"


def build():
    tiles = json.loads(CATALOG.read_text(encoding="utf-8"))
    lines = [
        "# Natively supported apps",
        "",
        "<!-- Generated by scripts/gen-apps-doc.py from crates/rustv-shell/src/catalog.json. Do not edit by hand. -->",
        "",
        f"RusTV ships **{len(tiles)} app tiles**. Each one opens the real service in its own window with a persistent profile, "
        "so you sign in once. Nothing is scraped or re-hosted: the app is the service's own website or the Windows program.",
        "",
        "Apps you add yourself (custom tiles) and apps provided by mods are not listed here. "
        "To add one to the built-in catalog, see [CONTRIBUTING.md](../CONTRIBUTING.md).",
        "",
        "**Tested** means someone checked it on a real account or device and the note says what exactly. "
        "**Partly tested** means the page was loaded as a guest (no account) and the note says what was visible. "
        "Paid services were deliberately not tested and most others need an account, so \"Not tested yet\" does not mean broken: please report what you find. "
        "A cold start of an app window can take 15 to 45 s before the first paint; the window is dark until then.",
        "",
    ]
    seen = set()
    for cat, title in CATEGORIES:
        group = [t for t in tiles if t["category"] == cat]
        if not group:
            continue
        seen.update(t["id"] for t in group)
        lines += [f"## {title}", "", "| App | What it is | Opens | Behaviour | Status | Notes |", "|---|---|---|---|---|---|"]
        lines += [row(t) for t in group]
        lines.append("")
    rest = [t for t in tiles if t["id"] not in seen]
    if rest:
        lines += ["## Other", "", "| App | What it is | Opens | Behaviour | Status | Notes |", "|---|---|---|---|---|---|"]
        lines += [row(t) for t in rest]
        lines.append("")
    lines += [
        "## Built into RusTV (not apps)",
        "",
        "Live TV (M3U/XMLTV, played with mpv), the local media library, a file browser, Windows settings shortcuts "
        "(Wi-Fi, Bluetooth, sound, displays, power) and universal search.",
        "",
    ]
    return "\n".join(lines)


if __name__ == "__main__":
    text = build()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8").replace("\r\n", "\n") != text:
            print("docs/APPS.md is out of date: run python scripts/gen-apps-doc.py")
            sys.exit(1)
        print("docs/APPS.md is up to date")
    else:
        OUT.write_text(text, encoding="utf-8", newline="\n")
        print(f"wrote {OUT.relative_to(ROOT)} ({len(text.splitlines())} lines)")
