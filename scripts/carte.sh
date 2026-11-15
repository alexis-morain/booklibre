#!/usr/bin/env bash
# L'actif de carte (P-s4) : Natural Earth compilé en un fichier, à l'octet près.
#
#   scripts/carte.sh compiler  [dossier]   réécrit crates/colophon-core/assets/carte/carte.bin
#   scripts/carte.sh verifier  [dossier]   recompile, compare à l'actif versionné, ne touche à rien
#
# `dossier` contient un dossier par couche, comme les zips de naturalearthdata.com
# dézippés (ne_10m_coastline/ne_10m_coastline.shp…) ; par défaut
# $COLOPHON_NATURAL_EARTH. Les six couches, leurs versions et leurs empreintes
# sont dans crates/colophon-core/assets/carte/SOURCE.md. Les fichiers bruts ne
# sont pas dans le dépôt : sans eux `verifier` s'arrête avec un code non nul,
# et le gate le saute en le disant (check.sh).
set -euo pipefail
cd "$(dirname "$0")/.."

mode="${1:-verifier}"
NE="${2:-${COLOPHON_NATURAL_EARTH:-$HOME/Documents/08_IA/02_Outputs/colophon/travail/natural-earth}}"
ACTIF="crates/colophon-core/assets/carte/carte.bin"

if [ ! -d "$NE/ne_10m_coastline" ]; then
  echo "carte : Natural Earth introuvable dans $NE" >&2
  exit 2
fi

case "$mode" in
  compiler)
    cargo run --release -q -p colophon-core --example carte_compiler -- "$NE" "$ACTIF"
    ;;
  verifier)
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    cargo run --release -q -p colophon-core --example carte_compiler -- "$NE" "$tmp/carte.bin" >/dev/null
    if cmp -s "$tmp/carte.bin" "$ACTIF"; then
      echo "carte : l'actif se recompile à l'identique ($(wc -c < "$ACTIF" | tr -d ' ') octets)"
    else
      echo "carte : l'actif versionné n'est pas celui que ce code compile à partir de $NE" >&2
      exit 1
    fi
    ;;
  *)
    echo "usage : scripts/carte.sh compiler|verifier [dossier-natural-earth]" >&2
    exit 64
    ;;
esac
