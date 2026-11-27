#!/usr/bin/env python3
"""Écrit les crédits de la démo depuis un fichier de sources (hors dépôt).

    demo-credits.py <sources.json> <sortie>

<sources.json> : une liste d'objets {titre, auteur, licence, page, date?}.
Écrit <sortie>/credits.html (liste complète, liens, mention des modifications)
et <sortie>/demo-data/credits.json (ce que lit le bandeau : auteur, licence).
Jamais lu par l'app hors démo, jamais versionné avec les sources.
"""
import html
import json
import sys
from pathlib import Path

LICENCES = {
    "CC BY-SA 4.0": "https://creativecommons.org/licenses/by-sa/4.0/",
    "CC BY 4.0": "https://creativecommons.org/licenses/by/4.0/",
    "CC0": "https://creativecommons.org/publicdomain/zero/1.0/",
}


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    sources = json.loads(Path(sys.argv[1]).read_text())
    sortie = Path(sys.argv[2])
    if not sources:
        sys.exit("fichier de sources vide")
    auteurs = sorted({s["auteur"] for s in sources})
    licences = sorted({s["licence"] for s in sources})
    (sortie / "demo-data").mkdir(parents=True, exist_ok=True)
    (sortie / "demo-data" / "credits.json").write_text(
        json.dumps({"auteur": ", ".join(auteurs), "licence": ", ".join(licences)}, ensure_ascii=False))

    e = html.escape
    lignes = []
    for s in sorted(sources, key=lambda s: s.get("date", "")):
        url = LICENCES.get(s["licence"])
        lic = f'<a href="{e(url)}">{e(s["licence"])}</a>' if url else e(s["licence"])
        lignes.append(
            f'<li><a href="{e(s["page"])}">{e(s["titre"])}</a>, {e(s["auteur"])}, {lic}</li>')
    corps = "\n".join(lignes)
    lic_liens = " / ".join(f'<a href="{e(LICENCES[l])}">{e(l)}</a>' for l in licences if l in LICENCES) or e(", ".join(licences))
    (sortie / "credits.html").write_text(f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>Photo credits · Booklibre demo</title>
<style>
body {{ font: 16px/1.55 system-ui, sans-serif; color: #37352f; max-width: 46rem; margin: 3rem auto; padding: 0 1.25rem; }}
h1 {{ font-size: 1.4rem; }} h2 {{ font-size: 1.05rem; margin-top: 2rem; }}
li {{ margin: .25rem 0; }} a {{ color: inherit; }} .note {{ color: #6b6a65; }}
</style>
</head>
<body>
<h1>Photo credits</h1>
<p>The photographs of the Booklibre online demo are by {e(", ".join(auteurs))}, from Wikimedia Commons, under {lic_liens}.
They have been resized and cropped to fit the album’s layouts, and the demo files are shared under the same license.</p>
<ol>
{corps}
</ol>
<h2>Crédits photographiques</h2>
<p>Les photographies de la démo en ligne de Booklibre sont de {e(", ".join(auteurs))}, depuis Wikimedia Commons, sous licence {lic_liens}.
Elles ont été redimensionnées et recadrées pour les mises en page de l’album ; les fichiers de la démo sont partagés sous la même licence.</p>
<p class="note"><a href="./">← Demo</a> · <a href="https://booklibre.app/">booklibre.app</a></p>
</body>
</html>
""")
    print(f"crédits : {len(sources)} photos, {len(auteurs)} auteur(s)")


main()
