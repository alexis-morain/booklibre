// Le glisser des planches, en calcul pur : quand un geste devient un
// glisser, et sur quelle planche il tombe. `PlanchesView` mesure les
// rectangles et suit le pointeur ; ici on ne fait que répondre.

import type { Printer } from "./bridge";
import { t } from "./i18n";

/** Sous ce déplacement, un appui reste un clic : la sélection d'avant. */
export const SEUIL_GLISSER_PX = 6;

/** Le rectangle d'une cellule à l'écran. `at` vaut -1 pour la couverture. */
export type RectCellule = {
  at: number;
  left: number;
  top: number;
  right: number;
  bottom: number;
};

/** Le pointeur est allé assez loin pour que l'appui soit un glisser. */
export function seuilFranchi(dx: number, dy: number): boolean {
  return Math.hypot(dx, dy) >= SEUIL_GLISSER_PX;
}

/** La planche sous le pointeur, ou rien. La couverture n'est jamais une
 *  cible : elle a une seule place possible, la première. */
export function cibleSous(rects: RectCellule[], x: number, y: number): number | null {
  for (const r of rects) {
    if (r.at < 0) continue;
    if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return r.at;
  }
  return null;
}

/**
 * Le message d'une suppression. Sous le minimum du produit, il le dit au lieu
 * de bloquer (1.1-e) : le compte est celui du prévol (`prevol.rs`, deux pages
 * par planche, plus les deux de la couverture chez qui relie un seul fichier),
 * sur le profil déjà en mémoire, sans appel au moteur.
 */
export function ditSuppression(
  n: number,
  planches: number,
  profil: Pick<Printer, "pages_min" | "fichiers"> | null,
): string {
  const dit = t("planche.supprimee", { n });
  if (!profil) return dit;
  const pages = planches * 2 + (profil.fichiers === "un" ? 2 : 0);
  if (pages >= profil.pages_min) return dit;
  return `${dit}. ${t("planche.sous.minimum", { pages, min: profil.pages_min })}`;
}
