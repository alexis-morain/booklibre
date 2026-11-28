#!/usr/bin/env bash
# Le build web de la démo en ligne : un album composé d'avance, ses réponses
# enregistrées, l'app construite en « mode démo » (vite build --mode demo).
#
#   scripts/demo.sh <album-fr> <album-en> <sortie> [sources.json]
#
# <album-fr> et <album-en> : le même dossier de photos composé deux fois,
# `booklibre <photos> -o <album-fr> -t "<titre>" --langue fr`, puis `--langue en`
# avec le titre anglais. La démo montre le livre de la langue de l'interface.
#
# sources.json (facultatif, hors dépôt) : les crédits des photos, voir
# scripts/demo-credits.py ; il produit credits.html et le crédit du bandeau.
#
# <sortie> reçoit le site de la démo, prêt à être servi sous /demo/ :
# index.html, assets/ et demo-data/. Les albums d'entrée ne sont jamais touchés (le
# harnais de dev écrit son album.json : le script travaille sur une copie).
# Prérequis : le moteur construit (cargo build --release), node, Pillow ; pikepdf
# si le PDF d'aperçu dépasse 25 Mio. DEMO_PYTHON choisit l'interpréteur (un venv).
set -euo pipefail
if [ $# -lt 3 ] || [ $# -gt 4 ]; then
  echo "usage : scripts/demo.sh <album-fr> <album-en> <sortie> [sources.json]" >&2
  exit 2
fi
# Chemins absolus avant de changer de dossier.
ALBUM_FR="$(cd "$1" && pwd)"
ALBUM_EN="$(cd "$2" && pwd)"
SOURCES=""
[ $# -eq 4 ] && SOURCES="$(cd "$(dirname "$4")" && pwd)/$(basename "$4")"
mkdir -p "$(dirname "$3")"
SORTIE="$(cd "$(dirname "$3")" && pwd)/$(basename "$3")"
cd "$(dirname "$0")/.."

PY="${DEMO_PYTHON:-$(command -v python3 || command -v python || true)}"
[ -n "$PY" ] || { echo "python introuvable" >&2; exit 1; }
[ -x target/release/booklibre ] || { echo "le moteur manque : cargo build --release" >&2; exit 1; }
"$PY" -c "import PIL" 2>/dev/null || { echo "Pillow manque : pip install Pillow" >&2; exit 1; }

# Un build natif en cours n'est pas à déranger : ce script ne le lance ni ne le coupe.
APP=crates/colophon-app
"$PY" scripts/demo-enregistrer.py "$ALBUM_FR" "$ALBUM_EN" "$APP/demo-data"

(cd "$APP" && npm run build:demo)

rm -rf "$SORTIE"
cp -R "$APP/dist-demo" "$SORTIE"
cp -R "$APP/demo-data" "$SORTIE/demo-data"
[ -z "$SOURCES" ] || "$PY" scripts/demo-credits.py "$SOURCES" "$SORTIE"

echo "démo prête : $SORTIE"
du -sh "$SORTIE"
find "$SORTIE" -type f -size +25M -print | sed 's/^/AU-DESSUS DU PLAFOND : /'
