#!/bin/sh
# Bundle Booklibre.app puis remplace la copie de /Applications.
# Une seule version installée à la fois : l'ancienne est supprimée, jamais
# renommée en « Booklibre 2.app » par un glisser-déposer de plus. Ni gardée
# sous son ancien nom : jusqu'à la 1.0.1 l'app s'appelait Colophon.app, avec
# le même identifiant, et deux bundles d'un même identifiant laissent macOS
# choisir lequel il ouvre.
set -e

cd "$(dirname "$0")/../crates/colophon-app"
npm run tauri build

APP="../../target/release/bundle/macos/Booklibre.app"
DEST="/Applications/Booklibre.app"
ANCIEN="/Applications/Colophon.app"

# Le processus s'appelle « colophon-app », pas « Booklibre » : c'est le
# CFBundleExecutable, et c'est ce que pgrep voit. Écrite avec le nom
# d'affichage, la garde ne matchait jamais — le bundle se faisait remplacer
# sous une app qui tournait, et l'app gardait l'ancien code. Trouvé le 29/08
# en vérifiant un correctif à l'écran : c'est l'ancien qui répondait.
if pgrep -x colophon-app >/dev/null 2>&1; then
  echo "Booklibre tourne : quitter l'app avant de la remplacer." >&2
  exit 1
fi


rm -rf "$DEST" "$ANCIEN"
ditto "$APP" "$DEST"
echo "Installé : $DEST ($(plutil -extract CFBundleShortVersionString raw "$DEST/Contents/Info.plist"))"
