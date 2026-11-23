#!/usr/bin/env python3
"""Tools for the mods in mods/ and the catalog in registry/index.json.

  python scripts/mods-tool.py check              validate every mods/*/mod.json
  python scripts/mods-tool.py art                (re)generate icon.svg and banner.svg from each mod's "art" block
  python scripts/mods-tool.py registry --commit <sha>
                                                 rebuild registry/index.json for the mods of this repo, pinned to <sha>

Catalog metadata lives in each mod.json (see docs/MODS.md, "Catalog metadata").
"""
import json
import re
import sys
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
MODS = ROOT / "mods"
REGISTRY = ROOT / "registry/index.json"
REPO = "TisaTHor08/rustv"

CATEGORIES = ["themes", "widgets", "pages", "games", "apps", "enhancements", "icons", "remote"]
PERMISSIONS = re.compile(r"^(storage|launch|network:https://[a-z0-9.\-]+(:\d+)?|inject:.+)$")

# 24x24 line glyphs (stroke is drawn by the generator). Several sub-paths are separated by "|".
GLYPHS = {
    "palette": "M12 3a9 9 0 100 18c1.2 0 2-.8 2-1.8 0-.5-.2-.9-.5-1.3-.3-.4-.5-.8-.5-1.3 0-1 .8-1.8 1.8-1.8H17a4 4 0 004-4c0-4.4-4-8-9-8z|M7.5 11h.01M10 7.5h.01M14.5 7.5h.01",
    "clock": "M12 3a9 9 0 100 18 9 9 0 000-18z|M12 7v5l3 2",
    "cloud": "M7 18a4 4 0 010-8 5.5 5.5 0 0110.6 1.2A3.4 3.4 0 0117 18z",
    "sun": "M12 8a4 4 0 100 8 4 4 0 000-8z|M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.4 1.4M17.6 17.6L19 19M5 19l1.4-1.4M17.6 6.4L19 5",
    "moon": "M20 14.5A8 8 0 019.5 4 8 8 0 1020 14.5z",
    "gamepad": "M6 8h12a4 4 0 014 4v2a3 3 0 01-5.2 2L15 14H9l-1.8 2A3 3 0 012 14v-2a4 4 0 014-4z|M7 11v3M5.5 12.5h3|M15.5 11.5h.01M17.5 13.5h.01",
    "newspaper": "M5 4h12v16H7a2 2 0 01-2-2z|M17 8h3v10a2 2 0 01-2 2|M8 8h6M8 12h6M8 16h4",
    "play": "M8 5l11 7-11 7z",
    "star": "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z",
    "grid": "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z",
    "heart": "M12 20s-7-4.4-7-10a4 4 0 017-2.6A4 4 0 0119 10c0 5.6-7 10-7 10z",
    "bolt": "M13 3L5 14h6l-1 7 8-11h-6z",
    "calendar": "M4 6h16v14H4z|M4 10h16|M8 3v4M16 3v4",
    "note": "M5 4h14v16H5z|M8 9h8M8 13h8M8 17h5",
    "radio": "M4 9h16v11H4z|M7 9l10-5|M14 14h4M14 17h4|M9 12.5a2.5 2.5 0 100 5 2.5 2.5 0 000-5z",
    "globe": "M12 3a9 9 0 100 18 9 9 0 000-18z|M3 12h18|M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18",
    "book": "M4 5a2 2 0 012-2h13v16H6a2 2 0 00-2 2z|M4 19V5",
    "trophy": "M8 4h8v5a4 4 0 01-8 0z|M8 6H4v2a3 3 0 003 3M16 6h4v2a3 3 0 01-3 3|M12 13v4M8 20h8M10 17h4",
    "smile": "M12 3a9 9 0 100 18 9 9 0 000-18z|M8 14a5 5 0 008 0|M9 9.5h.01M15 9.5h.01",
    "cursor-off": "M5 3l14 8-6 1.5L10 19z|M3 3l18 18",
    "type": "M5 6V4h14v2M12 4v16M9 20h6",
    "sparkles": "M12 3l1.8 4.7L18.5 9.5 13.8 11.3 12 16l-1.8-4.7L5.5 9.5l4.7-1.8z|M19 15l.8 2 2 .8-2 .8-.8 2-.8-2-2-.8 2-.8z",
    "music": "M9 18V6l11-2v12|M6 15a3 3 0 100 6 3 3 0 000-6z|M17 13a3 3 0 100 6 3 3 0 000-6z",
    "tv": "M3 6h18v12H3z|M8 21h8|M12 18v3",
    "film": "M4 4h16v16H4z|M8 4v16M16 4v16|M4 9h4M4 15h4M16 9h4M16 15h4",
    "shield": "M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z",
    "eye": "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z|M12 9a3 3 0 100 6 3 3 0 000-6z",
    "contrast": "M12 3a9 9 0 100 18 9 9 0 000-18z|M12 3a9 9 0 010 18z",
    "timer": "M12 6a8 8 0 100 16 8 8 0 000-16z|M12 10v4l2 1|M9 2h6M12 2v4",
    "map": "M9 4L3 6v14l6-2 6 2 6-2V4l-6 2z|M9 4v14M15 6v14",
    "cart": "M3 4h3l2 11h10l2-8H7|M9 20h.01M17 20h.01",
    "wave": "M2 12c2-4 4-4 6 0s4 4 6 0 4-4 6 0 2 2 2 2",
    "photo": "M4 5h16v14H4z|M4 16l5-5 4 4 3-3 4 4|M15 9h.01",
}

HEX = re.compile(r"^#[0-9a-fA-F]{6}$")
ID = re.compile(r"^[a-z0-9-]{2,40}$")


def load_mods():
    out = []
    for d in sorted(MODS.iterdir()):
        mj = d / "mod.json"
        if d.is_dir() and mj.exists():
            out.append((d, json.loads(mj.read_text(encoding="utf-8"))))
    return out


def referenced_files(m):
    c = m.get("contributes", {})
    files = []
    for t in c.get("themes", []):
        files += [t.get("css")]
    for t in c.get("tiles", []):
        files += [t.get("icon")]
    for p in c.get("pages", []):
        files += [p.get("entry"), p.get("icon")]
    for w in c.get("widgets", []):
        files += [w.get("entry")]
    for a in c.get("adapters", []):
        files += [a.get("js"), a.get("css")]
    for i in c.get("icons", []):
        files += [i.get("src")]
    for t in (c.get("remote") or {}).get("themes", []):
        files += [t.get("css")]
    files += [m.get("icon"), m.get("banner")]
    return [f for f in files if f]


def check():
    errors = []
    seen = set()
    mods = load_mods()
    ids = {m["id"] for _, m in mods if "id" in m}
    for d, m in mods:
        e = lambda msg: errors.append(f"{d.name}: {msg}")
        if m.get("id") != d.name:
            e("id must equal the folder name")
        if not ID.match(str(m.get("id", ""))):
            e("bad id")
        if m.get("id") in seen:
            e("duplicate id")
        seen.add(m.get("id"))
        if not m.get("name") or not m.get("version"):
            e("name and version are required")
        if m.get("category") not in CATEGORIES:
            e(f"category must be one of {CATEGORIES}")
        if not str(m.get("publisher", "")).strip():
            e("publisher is required")
        desc = str(m.get("description", ""))
        if not 10 <= len(desc) <= 140:
            e("description must be 10 to 140 characters")
        det = str(m.get("details", ""))
        if det and len(det) > 700:
            e("details is limited to 700 characters")
        for lang, tr in (m.get("i18n") or {}).items():
            for k in tr:
                if k not in ("name", "description", "details"):
                    e(f"i18n.{lang}.{k} is not allowed")
            if len(str(tr.get("description", ""))) > 140:
                e(f"i18n.{lang}.description is longer than 140 characters")
        art = m.get("art") or {}
        if art.get("glyph") not in GLYPHS or not HEX.match(str(art.get("from", ""))) or not HEX.match(str(art.get("to", ""))):
            e('art needs {"from": "#rrggbb", "to": "#rrggbb", "glyph": <one of the glyph names>}')
        for p in m.get("permissions", []):
            if not PERMISSIONS.match(p):
                e(f"unknown permission syntax: {p}")
        for dep in m.get("dependencies", []):
            t = dep.get("type")
            if t == "mod":
                if dep.get("id") not in ids and not dep.get("external_registry"):
                    e(f"dependency on unknown mod {dep.get('id')}")
            elif t == "external":
                if not dep.get("name"):
                    e("external dependency needs a name")
            else:
                e('dependency type must be "mod" or "external"')
        for f in referenced_files(m):
            if not (d / f).exists():
                e(f"missing file {f}")
        c = m.get("contributes", {})
        if not any(c.get(k) for k in ("themes", "tiles", "pages", "widgets", "adapters", "icons", "strings", "remote")):
            e("contributes nothing")
    for msg in errors:
        print("ERROR", msg)
    print(f"{len(mods)} mods checked, {len(errors)} problem(s)")
    return not errors


def glyph_svg(name, scale, tx, ty, stroke="#fff", sw=1.7, opacity=1.0):
    paths = "".join(f'<path d="{p}"/>' for p in GLYPHS[name].split("|"))
    return (
        f'<g transform="translate({tx} {ty}) scale({scale})" fill="none" stroke="{stroke}" stroke-width="{sw}" '
        f'stroke-linecap="round" stroke-linejoin="round" opacity="{opacity}">{paths}</g>'
    )


def icon_svg(m):
    a = m["art"]
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">'
        f'<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{a["from"]}"/><stop offset="1" stop-color="{a["to"]}"/></linearGradient></defs>'
        '<rect width="256" height="256" rx="58" fill="url(#g)"/>'
        '<rect width="256" height="128" rx="58" fill="#fff" opacity=".07"/>'
        + glyph_svg(a["glyph"], 7, 44, 44, sw=1.6)
        + "</svg>\n"
    )


def banner_svg(m):
    a = m["art"]
    name = escape(m["name"])
    cat = escape(m["category"].upper())
    # Long names shrink so they never run under the glyph (about 320px available).
    size = max(24, min(46, int(560 / max(1, len(m["name"])))))
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 320" width="640" height="320">'
        f'<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{a["from"]}"/><stop offset="1" stop-color="{a["to"]}"/></linearGradient>'
        '<radialGradient id="r" cx=".8" cy=".35" r=".6"><stop offset="0" stop-color="#fff" stop-opacity=".28"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient></defs>'
        '<rect width="640" height="320" fill="url(#g)"/><rect width="640" height="320" fill="url(#r)"/>'
        + glyph_svg(a["glyph"], 10.5, 350, 34, sw=1.1, opacity=0.9)
        + f'<text x="40" y="228" font-family="Segoe UI, Arial, sans-serif" font-size="{size}" font-weight="800" fill="#fff">{name}</text>'
        + f'<text x="42" y="262" font-family="Segoe UI, Arial, sans-serif" font-size="18" font-weight="700" letter-spacing="3" fill="#fff" opacity=".75">{cat}</text>'
        + "</svg>\n"
    )


def art():
    n = 0
    for d, m in load_mods():
        if not m.get("art"):
            continue
        (d / "icon.svg").write_text(icon_svg(m), encoding="utf-8", newline="\n")
        (d / "banner.svg").write_text(banner_svg(m), encoding="utf-8", newline="\n")
        if m.get("icon") != "icon.svg" or m.get("banner") != "banner.svg":
            m["icon"], m["banner"] = "icon.svg", "banner.svg"
            (d / "mod.json").write_text(json.dumps(m, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
        n += 1
    print(f"art written for {n} mods")


def registry(commit):
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        sys.exit("--commit must be a full 40 character sha")
    old = json.loads(REGISTRY.read_text(encoding="utf-8")) if REGISTRY.exists() else {"version": 1, "mods": []}
    local = {m["id"] for _, m in load_mods()}
    keep = [e for e in old.get("mods", []) if e["id"] not in local and e.get("source", {}).get("repo") != REPO]
    entries = []
    for d, m in load_mods():
        e = {
            "id": m["id"],
            "name": m["name"],
            "version": m["version"],
            "author": m.get("author", m["publisher"]),
            "publisher": m["publisher"],
            "category": m["category"],
            "description": m["description"],
        }
        for k in ("details", "tags", "dependencies", "permissions", "i18n", "icon", "banner", "art", "license"):
            if m.get(k):
                e[k] = m[k]
        e["source"] = {"repo": REPO, "commit": commit, "path": f"mods/{d.name}"}
        entries.append(e)
    out = {"version": 2, "mods": entries + keep}
    REGISTRY.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    print(f"registry: {len(entries)} mods from this repo, {len(keep)} external, pinned to {commit[:8]}")


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "check"
    if cmd == "check":
        sys.exit(0 if check() else 1)
    elif cmd == "art":
        art()
    elif cmd == "registry":
        registry(sys.argv[sys.argv.index("--commit") + 1])
    else:
        sys.exit(__doc__)
