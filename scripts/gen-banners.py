#!/usr/bin/env python3
"""Génère mods/real-icons/banners/<tuile>.svg : un visuel « écran de l'appli » par service, affiché dans le bandeau du haut.

Ce ne sont PAS des captures officielles : ce sont des illustrations originales (aucun contenu, affiche ou logo de tiers
hors le logo déjà présent dans icons/), dessinées aux couleurs du service. Pour mettre une vraie capture, déposez un
fichier dans mods/real-icons/banners/ et pointez-le dans mod.json (contributes.banners).
Usage : python scripts/gen-banners.py
"""
import colorsys, json, re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MOD = ROOT / "mods" / "real-icons"
CATALOG = json.loads((ROOT / "crates/rustv-shell/src/catalog.json").read_text(encoding="utf-8"))
CATALOG = CATALOG if isinstance(CATALOG, list) else CATALOG.get("tiles", CATALOG)
TILES = {t["id"]: t for t in CATALOG}

# Mise en page par service ; les autres suivent leur catégorie.
LAYOUT = {"netflix": "poster", "molotov": "channels", "youtube": "thumbs", "twitch": "thumbs", "kick": "thumbs", "dailymotion": "thumbs", "vimeo": "thumbs", "odysee": "thumbs"}
BY_CAT = {"video": "poster", "tv": "channels", "music": "albums", "library": "poster", "games": "cards"}
BG = {"netflix": "#0b0b0b", "youtube": "#0f0f0f", "molotov": "#16101f", "spotify": "#121212", "twitch": "#0e0e10", "steam": "#171a21", "appletv": "#000000", "canal": "#000000", "tidal": "#000000"}


def hsl_shift(hexc, dh=0.0, ds=0.0, dl=0.0):
    h = hexc.lstrip("#")
    r, g, b = (int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    hh, ll, ss = colorsys.rgb_to_hls(r, g, b)
    hh = (hh + dh) % 1.0
    ss = min(1, max(0, ss + ds))
    ll = min(1, max(0.06, ll + dl))
    r, g, b = colorsys.hls_to_rgb(hh, ll, ss)
    return "#%02x%02x%02x" % (round(r * 255), round(g * 255), round(b * 255))


def logo(tile_id):
    """Contenu interne du SVG d'icône (viewBox 24) et sa couleur de remplissage."""
    s = (MOD / "icons" / f"{tile_id}.svg").read_text(encoding="utf-8")
    fill = re.search(r'<svg[^>]*fill="([^"]+)"', s)
    inner = re.sub(r"</?svg[^>]*>|<title>.*?</title>", "", s)
    return inner, (fill.group(1) if fill else "#ffffff")


def posters(c1, c2, x0, y0, cols, rows, w, h, gap):
    out = []
    for r in range(rows):
        for c in range(cols):
            i = r * cols + c
            a = hsl_shift(c1, dh=((i % 5) - 2) * 0.018, dl=-0.16 + (i % 4) * 0.05)
            b = hsl_shift(c2, dh=((i % 3) - 1) * 0.02, dl=-0.02)
            gid = f"p{r}{c}"
            x, y = x0 + c * (w + gap), y0 + r * (h + gap)
            out.append(f'<linearGradient id="{gid}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{a}"/><stop offset="1" stop-color="{b}"/></linearGradient>')
            out.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="14" fill="url(#{gid})"/>')
            out.append(f'<rect x="{x + 14}" y="{y + h - 34}" width="{w * 0.55:.0f}" height="10" rx="5" fill="#fff" opacity=".55"/>')
    return "".join(out)


def build(tid):
    t = TILES[tid]
    c1, c2 = t["color"], t["color2"]
    lay = LAYOUT.get(tid) or BY_CAT.get(t["category"], "poster")
    bg = BG.get(tid) or hsl_shift(c2, dl=-0.12)
    glow = c1 if tid not in ("appletv", "canal", "tidal", "tf1") else hsl_shift(c1, dl=0.1)
    inner, fill = logo(tid)
    d = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMid slice">',
         f'<defs><radialGradient id="g" cx=".75" cy=".3" r=".8"><stop offset="0" stop-color="{glow}" stop-opacity=".55"/><stop offset="1" stop-color="{bg}" stop-opacity="0"/></radialGradient></defs>',
         f'<rect width="1600" height="900" fill="{bg}"/><rect width="1600" height="900" fill="url(#g)"/>']
    # fenêtre de l'appli, à droite (le texte du héros occupe la gauche)
    d.append('<g transform="translate(588 70)">')
    d.append(f'<rect width="1000" height="780" rx="26" fill="#000" opacity=".35"/>')
    d.append(f'<g transform="translate(46 36) scale(3)" fill="{fill}">{inner}</g>')
    for i in range(4):
        d.append(f'<rect x="{200 + i * 130}" y="52" width="{90 + (i % 2) * 20}" height="22" rx="11" fill="#fff" opacity="{.7 if i == 0 else .25}"/>')
    if lay == "poster":
        d.append(f'<rect x="46" y="130" width="908" height="250" rx="20" fill="{hsl_shift(c1, dl=-0.1)}"/><rect x="46" y="130" width="908" height="250" rx="20" fill="#000" opacity=".25"/>')
        d.append(f'<polygon points="470,205 470,305 555,255" fill="#fff" opacity=".9"/>')
        d.append(posters(c1, c2, 46, 410, 6, 1, 140, 210, 13.6))
        d.append(posters(hsl_shift(c1, dh=.04), c2, 46, 640, 6, 1, 140, 110, 13.6))
    elif lay == "thumbs":
        d.append(posters(c1, c2, 46, 130, 3, 2, 290, 163, 19))
        for r in range(2):
            for c in range(3):
                x, y = 46 + c * 309, 130 + r * 182
                d.append(f'<circle cx="{x + 150}" cy="{y + 82}" r="30" fill="#000" opacity=".55"/><polygon points="{x + 141},{y + 66} {x + 141},{y + 98} {x + 170},{y + 82}" fill="#fff"/>')
        d.append(posters(hsl_shift(c1, dh=.04), c2, 46, 500, 3, 1, 290, 163, 19))
        for c in range(3):
            d.append(f'<circle cx="{66 + c * 309}" cy="{700}" r="18" fill="#fff" opacity=".6"/><rect x="{96 + c * 309}" y="690" width="170" height="12" rx="6" fill="#fff" opacity=".5"/><rect x="{96 + c * 309}" y="712" width="110" height="10" rx="5" fill="#fff" opacity=".28"/>')
    elif lay == "channels":
        d.append(f'<rect x="46" y="130" width="560" height="315" rx="20" fill="{hsl_shift(c1, dl=-0.1)}"/><rect x="46" y="130" width="560" height="315" rx="20" fill="#000" opacity=".22"/>')
        d.append(f'<rect x="70" y="154" width="86" height="34" rx="8" fill="#e50914"/><text x="113" y="178" font-family="Segoe UI,Arial" font-size="20" font-weight="800" fill="#fff" text-anchor="middle">LIVE</text>')
        d.append(f'<rect x="70" y="395" width="360" height="12" rx="6" fill="#fff" opacity=".6"/><rect x="70" y="420" width="470" height="6" rx="3" fill="#fff" opacity=".25"/><rect x="70" y="420" width="230" height="6" rx="3" fill="#fff"/>')
        for r in range(4):
            for c in range(2):
                a = hsl_shift(c1, dh=(((r * 2 + c) % 4) - 1.5) * 0.02, dl=-0.14 + ((r + c) % 3) * 0.05)
                x, y = 636 + c * 160, 130 + r * 82
                d.append(f'<rect x="{x}" y="{y}" width="150" height="72" rx="14" fill="{a}" opacity=".9"/><circle cx="{x + 34}" cy="{y + 36}" r="18" fill="#fff" opacity=".85"/><rect x="{x + 62}" y="{y + 30}" width="70" height="10" rx="5" fill="#fff" opacity=".6"/>')
        d.append(posters(c1, c2, 46, 480, 6, 1, 140, 130, 13.6))
        d.append(posters(hsl_shift(c1, dh=.04), c2, 46, 640, 6, 1, 140, 110, 13.6))
    elif lay == "albums":
        d.append(posters(c1, c2, 46, 130, 4, 2, 210, 210, 19))
        d.append(posters(hsl_shift(c1, dh=.04), c2, 46, 590, 4, 1, 210, 100, 19))
        d.append(f'<rect x="46" y="705" width="908" height="50" rx="25" fill="#000" opacity=".5"/><circle cx="90" cy="730" r="14" fill="#fff"/><rect x="130" y="725" width="640" height="10" rx="5" fill="#fff" opacity=".25"/><rect x="130" y="725" width="300" height="10" rx="5" fill="{fill if fill != "#ffffff" else c1}"/>')
    else:  # cards (jeux)
        d.append(f'<rect x="46" y="130" width="908" height="330" rx="20" fill="{hsl_shift(c1, dl=-0.12)}"/><rect x="46" y="130" width="908" height="330" rx="20" fill="#000" opacity=".25"/>')
        d.append(f'<rect x="80" y="380" width="170" height="48" rx="24" fill="#fff"/>')
        d.append(posters(c1, c2, 46, 490, 4, 1, 210, 130, 19))
        d.append(posters(hsl_shift(c1, dh=.04), c2, 46, 640, 4, 1, 210, 100, 19))
    d.append("</g></svg>")
    return "".join(d)


def main():
    out = MOD / "banners"
    out.mkdir(exist_ok=True)
    mod = json.loads((MOD / "mod.json").read_text(encoding="utf-8"))
    banners = []
    for i in mod["contributes"]["icons"]:
        tid = i["tile"]
        if tid not in TILES:
            continue
        (out / f"{tid}.svg").write_text(build(tid), encoding="utf-8")
        banners.append({"tile": tid, "src": f"banners/{tid}.svg"})
    mod["contributes"]["banners"] = banners
    (MOD / "mod.json").write_text(json.dumps(mod, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(len(banners), "bannières")


if __name__ == "__main__":
    main()
