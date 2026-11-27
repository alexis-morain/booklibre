#!/usr/bin/env bash
# Le build web de la démo en ligne : un album composé d'avance, ses réponses
# enregistrées, l'app construite en « mode démo » (vite build --mode demo).
#
#   scripts/demo.sh <dossier-album-composé> <sortie>
#
# <sortie> reçoit le site de la démo, prêt à être servi sous /demo/ :
# index.html, assets/ et demo-data/. L'album d'entrée n'est jamais touché (le
# harnais de dev écrit son album.json : le script travaille sur une copie).
# Prérequis : le moteur construit (cargo build --release), node, Pillow.
set -euo pipefail
if [ $# -ne 2 ]; then
  echo "usage : scripts/demo.sh <dossier-album-composé> <sortie>" >&2
  exit 2
fi
# Chemins absolus avant de changer de dossier.
ALBUM="$(cd "$1" && pwd)"
mkdir -p "$(dirname "$2")"
SORTIE="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
cd "$(dirname "$0")/.."

PY="$(command -v python3 || command -v python || true)"
[ -n "$PY" ] || { echo "python introuvable" >&2; exit 1; }
[ -x target/release/booklibre ] || { echo "le moteur manque : cargo build --release" >&2; exit 1; }
"$PY" -c "import PIL" 2>/dev/null || { echo "Pillow manque : pip install Pillow" >&2; exit 1; }

# Un build natif en cours n'est pas à déranger : ce script ne le lance ni ne le coupe.
APP=crates/colophon-app
"$PY" scripts/demo-enregistrer.py "$ALBUM" "$APP/demo-data"

(cd "$APP" && npm run build:demo)

rm -rf "$SORTIE"
cp -R "$APP/dist-demo" "$SORTIE"
cp -R "$APP/demo-data" "$SORTIE/demo-data"

echo "démo prête : $SORTIE"
du -sh "$SORTIE"
find "$SORTIE" -type f -size +25M -print | sed 's/^/AU-DESSUS DU PLAFOND : /'
