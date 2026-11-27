#!/usr/bin/env python3
"""Enregistre les réponses du harnais pour la démo en ligne (`scripts/demo.sh`).

Lance le serveur de dev de l'app sur une COPIE de l'album composé (le harnais
écrit le vrai album.json : jamais l'original), demande une à une les réponses
GET que la démo rejouera, de façon déterministe : on énumère les planches, les
cases, les photos, les formats, on ne crawle pas. Écrit `demo-data/` et son
manifeste. Dépend de Pillow (vignettes) et de node (le serveur).

Ce qui ne sort JAMAIS d'ici :
- un chemin de cette machine (JSON scrutés avant la fin, échec sinon) ;
- l'EXIF ou le GPS d'une image (JPEG réencodés sans métadonnées, le profil
  couleur excepté) ; le GPS des fiches est arrondi à deux décimales (~1 km) ;
- une police du système : un album dont la face n'est pas celle du moteur est
  refusé, le moteur n'a que Source Sans 3 (SIL OFL 1.1) à offrir.
"""

import argparse
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

PORT = 1420
PLAFOND = 25 * 1024 * 1024  # Cloudflare Workers assets : 25 MiB par fichier
RACINE = Path(__file__).resolve().parent.parent
APP = RACINE / "crates" / "colophon-app"


def cle_de_route(chemin: str, params: dict[str, str]) -> str:
    """Jumelle de `demo.ts::cleDeRoute` : le chemin, puis `k=v` triés."""
    paires = sorted(params.items())
    return "\n".join([chemin] + [f"{k}={v}" for k, v in paires])


class Harnais:
    def __init__(self, album: Path):
        self.album = album
        self.proc: subprocess.Popen | None = None

    def __enter__(self):
        env = dict(os.environ, COLOPHON_ALBUM=str(self.album))
        self.proc = subprocess.Popen(
            ["npx", "vite", "--port", str(PORT), "--strictPort"],
            cwd=APP, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        for _ in range(120):
            if self.proc.poll() is not None:
                sys.exit("le serveur de dev s'est arrêté au démarrage (port 1420 pris ?)")
            try:
                self.demander("/__dev/album")
                return self
            except (urllib.error.URLError, ConnectionError):
                time.sleep(0.5)
        sys.exit("le serveur de dev ne répond pas")

    def __exit__(self, *_):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(10)
            except subprocess.TimeoutExpired:
                self.proc.kill()

    def demander(self, chemin: str, params: dict[str, str] | None = None, methode="GET") -> bytes:
        q = "?" + urllib.parse.urlencode(params, quote_via=urllib.parse.quote) if params else ""
        req = urllib.request.Request(f"http://localhost:{PORT}{chemin}{q}", method=methode,
                                     data=b"" if methode == "POST" else None)
        with urllib.request.urlopen(req, timeout=300) as r:
            return r.read()


def vignette_web(octets: bytes, px: int, qualite: int) -> bytes:
    from PIL import Image

    im = Image.open(io.BytesIO(octets))
    icc = im.info.get("icc_profile")
    im = im.convert("RGB")
    im.thumbnail((px, px))
    sortie = io.BytesIO()
    # Aucun `exif=` : Pillow n'en écrit pas. Le profil couleur, lui, est gardé :
    # le retirer déplacerait les couleurs.
    im.save(sortie, "JPEG", quality=qualite, optimize=True, progressive=True,
            **({"icc_profile": icc} if icc else {}))
    return sortie.getvalue()


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("album", type=Path, help="dossier de l'album composé")
    ap.add_argument("sortie", type=Path, help="dossier d'écriture de demo-data/")
    ap.add_argument("--px", type=int, default=1600)
    ap.add_argument("--qualite", type=int, default=72)
    a = ap.parse_args()

    album_dir = a.album.resolve()
    if not (album_dir / "album.json").is_file():
        sys.exit(f"{album_dir} n'est pas un album composé (album.json absent)")
    sortie = a.sortie.resolve()
    if sortie.exists():
        shutil.rmtree(sortie)
    sortie.mkdir(parents=True)

    original = json.loads((album_dir / "album.json").read_text())
    if original.get("police"):
        sys.exit("cet album porte une police choisie : la démo n'embarque que la face du moteur (Source Sans 3)")

    # Le harnais écrit album.json : on lui donne une copie, `.cache/` comprise.
    tmp = Path(tempfile.mkdtemp(prefix="demo-album-"))
    copie = tmp / "album"
    shutil.copytree(album_dir, copie)
    interdits = [str(Path.home()), str(album_dir), str(copie), str(tmp)]

    routes: dict[str, dict] = {}
    octets_total = 0

    def poser(chemin: str, params: dict[str, str], nom: str, donnees: bytes, type_: str) -> None:
        nonlocal octets_total
        if len(donnees) > PLAFOND:
            sys.exit(f"{nom} pèse {len(donnees) / 2**20:.1f} Mio : au-dessus du plafond de 25 Mio par fichier")
        cible = sortie / nom
        cible.parent.mkdir(parents=True, exist_ok=True)
        cible.write_bytes(donnees)
        octets_total += len(donnees)
        routes[cle_de_route(chemin, params)] = {"f": nom, "t": type_}

    def poser_json(chemin, params, nom, valeur) -> None:
        poser(chemin, params, nom, json.dumps(valeur, ensure_ascii=False, separators=(",", ":")).encode(),
              "application/json")

    try:
        with Harnais(copie) as h:
            def json_de(chemin, params=None):
                return json.loads(h.demander(chemin, params))

            def enregistrer(chemin, params, nom):
                poser_json(chemin, params or {}, nom, json_de(chemin, params))

            # L'album d'ouverture : ni le dossier de la copie ni celui des photos.
            ouvert = json_de("/__dev/album")
            ouvert["dir"] = "demo"
            ouvert["album"]["root"] = "demo"
            poser_json("/__dev/album", {}, "album.json", ouvert)
            alb = ouvert["album"]
            srcs = ouvert["thumb_srcs"]
            planches = alb["spreads"]
            print(f"album : {len(planches)} planches, {len(srcs)} photos")

            for chemin, nom in [("/__dev/curation", "curation.json"), ("/__dev/geometrie", "geometrie.json"),
                                ("/__dev/cartes", "cartes.json"), ("/__dev/ornements", "ornements.json"),
                                ("/__dev/printers", "printers.json"), ("/__dev/releve", "releve.json")]:
                enregistrer(chemin, None, nom)
            # La géométrie du format de l'album, par la requête du panneau Format.
            fmt = f"{alb['trim_mm']['w']:g}x{alb['trim_mm']['h']:g}"
            enregistrer("/__dev/geometrie", {"format": fmt, "bleed": f"{alb['bleed_mm']:g}"}, "geometrie-format.json")
            for langue in ("fr", "en"):
                enregistrer("/__dev/dates", {"langue": langue}, f"dates-{langue}.json")

            # La face du moteur, celle que le moteur embarque : jamais une face du système.
            poser("/__dev/police", {}, "SourceSans3-Regular.ttf", h.demander("/__dev/police"), "font/ttf")

            # Les planches : légende proposée, réserve (de la planche, puis de chaque case),
            # gabarits compatibles avec les photos qu'elle porte.
            for i, p in enumerate(planches, 1):
                if not p["slots"]:
                    continue
                for langue in ("fr", "en"):
                    enregistrer("/__dev/proposition", {"planche": str(i), "langue": langue}, f"proposition/{i}-{langue}.json")
                enregistrer("/__dev/reserve", {"planche": str(i)}, f"reserve/{i}.json")
                for k in range(1, len(p["slots"]) + 1):
                    enregistrer("/__dev/reserve", {"planche": str(i), "case": str(k)}, f"reserve/{i}-{k}.json")
                liste = json.dumps([s["src"] for s in p["slots"]], ensure_ascii=False, separators=(",", ":"))
                enregistrer("/__dev/gabarits", {"srcs": liste}, f"gabarits/{i}.json")
            print("planches : enregistrées")

            for n, src in enumerate(srcs, 1):
                fiche = json_de("/__dev/fiche", {"src": src})
                if fiche.get("gps"):
                    fiche["gps"] = [round(c, 2) for c in fiche["gps"]]
                poser_json("/__dev/fiche", {"src": src}, f"fiche/{n:04d}.json", fiche)
                brut = h.demander("/__dev/thumb", {"src": src})
                poser("/__dev/thumb", {"src": src}, f"thumb/{n:04d}.jpg", vignette_web(brut, a.px, a.qualite), "image/jpeg")
            print(f"photos : {len(srcs)} vignettes et fiches")

            # L'aperçu fidèle : le PDF de l'album, puis celui de la couverture (rendu à la demande).
            poser("/__dev/pdf", {"quoi": "album"}, "album.pdf", h.demander("/__dev/pdf", {"quoi": "album"}), "application/pdf")
            h.demander("/__dev/couverture", {"profil": "cloudprinter"}, "POST")
            poser("/__dev/pdf", {"quoi": "couverture"}, "couverture.pdf",
                  h.demander("/__dev/pdf", {"quoi": "couverture"}), "application/pdf")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    (sortie / "manifest.json").write_text(json.dumps({"version": 1, "routes": routes}, ensure_ascii=False))

    # Aucun chemin de cette machine, ni dans un JSON ni dans un PDF.
    for f in sortie.rglob("*"):
        if f.suffix in (".json", ".pdf"):
            contenu = f.read_bytes()
            for chaine in interdits:
                if chaine.encode() in contenu:
                    sys.exit(f"{f.relative_to(sortie)} contient un chemin de cette machine : {chaine}")
    print(f"demo-data : {len(routes)} réponses, {octets_total / 2**20:.1f} Mio")


if __name__ == "__main__":
    main()
