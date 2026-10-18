// Le livre en volume (A4), sa part pure : les cotes à l'échelle et les
// angles. `Livre3D.tsx` les dessine en CSS 3D, sans aucune dépendance.
//
// Les cotes viennent du moteur et de rien d'autre : le format de la page et
// le dos de la fiche (`fiche.dos_mm`), à une seule échelle, si bien qu'un
// livre de 24 pages et un de 200 ne se ressemblent pas à l'écran plus qu'à la
// main.

/** L'angle de repos, en degrés autour de l'axe vertical : le trois-quarts,
 *  qui montre la couverture et le dos à la fois. Positif, le bord gauche du
 *  livre (le dos, pour un livre qui s'ouvre à droite) vient vers nous. */
export const ANGLE_REPOS = 30;

/** L'inclinaison de repos, autour de l'axe horizontal : on regarde le livre
 *  d'un peu au-dessus, et le dessus du bloc de pages se devine. */
export const INCLINAISON_REPOS = -10;

/** Les trois vues fixes du mouvement réduit : l'avant, le trois-quarts, le
 *  dos. */
export const VUES_FIXES = [0, ANGLE_REPOS, 90] as const;

/** Un appui sur une flèche. */
export const PAS_FLECHE = 15;

/** L'épaisseur montrée quand le dos n'est pas connu, ou qu'il est plus mince
 *  qu'un trait : un livre sans tranche n'est plus un livre. */
export const EPAISSEUR_MIN_PX = 3;

export type Cotes = { largeur: number; hauteur: number; epaisseur: number };

/** Les trois cotes du livre en pixels, la hauteur donnée, le reste à la même
 *  échelle. `dosMm` null : le dos n'est pas encore connu. */
export function cotesLivre(
  { wMm, hMm, dosMm }: { wMm: number; hMm: number; dosMm: number | null },
  hauteurPx: number,
): Cotes {
  const echelle = hauteurPx / hMm;
  return {
    largeur: wMm * echelle,
    hauteur: hauteurPx,
    epaisseur: Math.max(EPAISSEUR_MIN_PX, (dosMm ?? 0) * echelle),
  };
}

/** Le trois-quarts le plus proche d'un angle : au relâchement, le livre y
 *  revient par le chemin court, sans refaire les tours qu'on lui a donnés. */
export function repos(angle: number): number {
  return ANGLE_REPOS + 360 * Math.round((angle - ANGLE_REPOS) / 360);
}

/** La vue fixe qui suit, au clic, en mouvement réduit. Un angle qui n'en est
 *  pas une repart de la plus proche. */
export function vueSuivante(angle: number): number {
  const a = ((angle % 360) + 360) % 360;
  let i = 0;
  VUES_FIXES.forEach((v, j) => {
    if (Math.abs(v - a) < Math.abs(VUES_FIXES[i] - a)) i = j;
  });
  return VUES_FIXES[(i + 1) % VUES_FIXES.length];
}

/** L'angle après une flèche : la gauche montre le dos, la droite la tranche. */
export function apresFleche(angle: number, sens: "gauche" | "droite"): number {
  return angle + (sens === "gauche" ? PAS_FLECHE : -PAS_FLECHE);
}
