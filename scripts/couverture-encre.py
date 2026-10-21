#!/usr/bin/env python3
"""L'encre de la feuille de couverture, lue sur le papier (1.2-d).

Le prévol mesure la feuille par ses cotes, et des cotes justes ne disent rien
de ce qui est imprimé : le 21/09 une couverture blanche est partie aux cotes
exactes, et c'est l'imprimeur qui l'a vue. Ce script rend une vraie feuille et
lit ses pixels.

Un album synthétique : une photo bleue en première, une photo rouge dans le
cadre de la quatrième, un texte de deux lignes sous elle. On exporte la
feuille de Cloudprinter, on la rasterise à dix pixels par millimètre, et on
vérifie :

- que la première est encrée (la photo y est) ;
- que l'encre rouge tombe dans le cadre de la quatrième, à un demi-millimètre ;
- que le texte de la quatrième est imprimé, sous le cadre, dans la page ;
- que rien de la quatrième ne passe dans le mors ni le dos.

Le cadre attendu est **calculé ici à la main**, depuis les cotes du dump et la
règle de 1.2-d (boîte intérieure à 9 %, côté moyen = la moitié, en haut) :
réutiliser `cover::cadre_quatrieme` ferait mesurer le code par lui-même.

`--avec-mutant` rejoue la lecture sur le même raster en décalant le cadre
attendu d'un millimètre et demi : la sonde doit alors rougir, sinon elle ne
mesure rien.

Usage : scripts/couverture-encre.py [binaire] [--avec-mutant]
Prérequis : macOS (sips) et Pillow, comme `ornement-encre.py`.
"""

import json
import os
import subprocess
import sys
import tempfile

from PIL import Image

args = [a for a in sys.argv[1:] if not a.startswith("--")]
BIN = args[0] if args else "target/release/booklibre"
AVEC_MUTANT = "--avec-mutant" in sys.argv
FORMAT = "carre-21"
PROFIL = "cloudprinter"
PLANCHES = 48  # une des épaisseurs que le dump porte pour chaque profil
TOLERANCE_MM = 0.5
PX_PAR_MM = 10
INSET = 0.09
DECALAGE_MUTANT_MM = 1.5

BLEU = (60, 100, 170)
ROUGE = (200, 30, 30)


def est_rouge(p):
    return p[0] > 150 and p[1] < 90 and p[2] < 90


def est_bleu(p):
    return p[2] > 120 and p[0] < 110


def est_encre(p):
    # Le texte de la quatrième : l'encre du livre, sombre et neutre.
    return max(p) < 110 and max(p) - min(p) < 40


geo = json.loads(subprocess.check_output([BIN, "--dump-geometry", "--format", FORMAT]))
trim = geo["trim_mm"]
couv = next(c for c in geo["covers"] if c["profil"] == PROFIL and c["spreads"] == PLANCHES)
media_w, media_h = couv["sheet"]
cotes = couv["profil_cotes"]
(bx, bw), (fx, fw) = couv["panels"]
by = cotes["bas"] + cotes["rempli"] + cotes["debord"]
bh = trim["h"]

# Le cadre, à la main : boîte intérieure, côté moitié de son plus petit côté,
# centré en largeur, collé en haut de la boîte.
iw, ih = bw * (1 - 2 * INSET), bh * (1 - 2 * INSET)
cote = 0.5 * min(iw, ih)
cadre = {
    "x0": bx + (bw - cote) / 2,
    "x1": bx + (bw + cote) / 2,
    "y1": by + bh * INSET + ih,  # haut, repère du moteur
}
cadre["y0"] = cadre["y1"] - cote


def album_json(root):
    return {
        "version": 3,
        "title": "encre",
        "root": root,
        "trim_mm": trim,
        "bleed_mm": geo["bleed_mm"],
        "spreads": [{"template": "vide", "slots": []} for _ in range(PLANCHES)],
        "cover": {
            "title": "Encre",
            "photo": {"src": "avant.jpg", "focal": [0.5, 0.5]},
            "back_text": "Pour Marie.\nEt pour la suite.",
            "back_photo": {
                "photo": {"src": "dos.jpg", "focal": [0.5, 0.5]},
                "position": "haut",
                "taille": "moyenne",
            },
        },
    }


def lire(im, attendu, nom):
    """Les écarts de la feuille, en lignes lisibles ; vide si tout tient."""
    ppmm = im.width / media_w
    px = im.load()
    echecs = []

    def zone(x0_mm, x1_mm, y0_mm, y1_mm, pas=2):
        """Les pixels d'une bande, repère du moteur, un sur `pas`."""
        for y in range(int((media_h - y1_mm) * ppmm), int((media_h - y0_mm) * ppmm), pas):
            for x in range(int(x0_mm * ppmm), int(x1_mm * ppmm), pas):
                if 0 <= x < im.width and 0 <= y < im.height:
                    yield x, y, px[x, y]

    # 1. La première est encrée.
    bleus = total = 0
    for _, _, p in zone(fx + 5, fx + fw - 5, by + 5, by + bh - 5, pas=8):
        total += 1
        bleus += est_bleu(p)
    if bleus < 0.8 * total:
        echecs.append(f"la première n'est pas encrée : {bleus}/{total} pixels de photo")

    # 2. Le rouge tombe dans le cadre, et nulle part ailleurs.
    rouges = [(x, y) for x, y, p in zone(0, media_w, 0, media_h, pas=1) if est_rouge(p)]
    if not rouges:
        echecs.append("la photo de quatrième n'a rien imprimé")
    else:
        ex0 = min(r[0] for r in rouges) / ppmm
        ex1 = (max(r[0] for r in rouges) + 1) / ppmm
        ey1 = media_h - min(r[1] for r in rouges) / ppmm
        ey0 = media_h - (max(r[1] for r in rouges) + 1) / ppmm
        for bord, mesure, voulu in [
            ("gauche", ex0, attendu["x0"]),
            ("droite", ex1, attendu["x1"]),
            ("bas", ey0, attendu["y0"]),
            ("haut", ey1, attendu["y1"]),
        ]:
            if abs(mesure - voulu) > TOLERANCE_MM:
                echecs.append(
                    f"le cadre de quatrième, bord {bord} : {mesure:.2f} mm contre {voulu:.2f}"
                )
        if ex1 > bx + bw:
            echecs.append(f"la photo de quatrième passe dans le mors : {ex1:.2f} > {bx + bw:.2f}")

    # 3. Le texte est imprimé sous le cadre, dans la page.
    encre = [
        (x, y)
        for x, y, p in zone(bx, bx + bw, by, attendu["y0"] - 0.5, pas=1)
        if est_encre(p)
    ]
    if not encre:
        echecs.append("le texte de la quatrième n'est pas imprimé sous son cadre")
    hors = [
        (x, y)
        for x, y, p in zone(bx, bx + bw, attendu["y0"], by + bh, pas=1)
        if est_encre(p)
        and not (attendu["x0"] - 1 <= x / ppmm <= attendu["x1"] + 1)
    ]
    if hors:
        echecs.append(f"{len(hors)} pixels d'encre de quatrième au-dessus du texte, hors du cadre")

    # 4. Rien de la quatrième dans le mors : entre le panneau et le dos, la
    # feuille reste nue (la photo de première, elle, y entre par l'autre côté).
    dos_x = bx + bw + cotes["mors"]
    mors = [p for _, _, p in zone(bx + bw + 0.3, dos_x - 0.3, by, by + bh) if est_rouge(p) or est_encre(p)]
    if mors:
        echecs.append(f"{len(mors)} pixels d'encre de quatrième dans le mors")

    for e in echecs:
        print(f"couverture-encre ({nom}) : {e}", file=sys.stderr)
    return echecs


with tempfile.TemporaryDirectory() as td:
    photos = os.path.join(td, "photos")
    dossier = os.path.join(td, "album")
    os.makedirs(photos)
    os.makedirs(dossier)
    Image.new("RGB", (3000, 3000), BLEU).save(os.path.join(photos, "avant.jpg"), quality=92)
    Image.new("RGB", (2000, 2000), ROUGE).save(os.path.join(photos, "dos.jpg"), quality=92)
    with open(os.path.join(dossier, "album.json"), "w", encoding="utf-8") as f:
        json.dump(album_json(photos), f)
    rendu = subprocess.run(
        [BIN, "--cover", "--profil", PROFIL, "-o", dossier], capture_output=True, text=True
    )
    if rendu.returncode != 0:
        print(f"couverture-encre : la couverture a refusé\n{rendu.stderr}", file=sys.stderr)
        sys.exit(1)
    pdf = os.path.join(dossier, "album-cover.pdf")
    png = os.path.join(td, "feuille.png")
    subprocess.check_call(
        ["sips", "-s", "format", "png", "-Z", str(int(media_w * PX_PAR_MM)), pdf, "--out", png],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    brut = Image.open(png)
    fond = Image.new("RGB", brut.size, (255, 255, 255))
    fond.paste(brut, mask=brut.split()[-1] if brut.mode == "RGBA" else None)

    if lire(fond, cadre, "feuille"):
        sys.exit(1)
    print(
        f"couverture-encre : première encrée, cadre de quatrième à {TOLERANCE_MM} mm, "
        "texte sous lui, mors nu"
    )
    if AVEC_MUTANT:
        decale = {k: v + DECALAGE_MUTANT_MM for k, v in cadre.items()}
        if not lire(fond, decale, "mutant, attendu"):
            print(
                f"couverture-encre : le mutant ({DECALAGE_MUTANT_MM} mm) passe, la sonde ne mord pas",
                file=sys.stderr,
            )
            sys.exit(1)
        print(f"couverture-encre : le mutant ({DECALAGE_MUTANT_MM} mm) rougit, la sonde mord")
