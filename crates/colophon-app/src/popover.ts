// Où s'ouvre un popover ancré à un objet de la planche : la légende sous sa
// photo, les réglages d'un bloc sous le bloc. Position fixe, en pixels de la
// fenêtre, pour qu'il puisse pendre sous la feuille sans être coupé par elle.

/** Un rectangle droit à l'écran. */
export type Boite = { left: number; top: number; right: number; bottom: number };

/** L'écart entre l'objet et son popover, et entre le popover et la fenêtre. */
const ECART = 8;

/** La légende prend la largeur de sa case, entre 240 et 420 px : plus
 *  étroite, le champ ne se lit plus ; plus large, l'œil perd la case. */
export function largeurLegende(largeurCase: number): number {
  return Math.max(240, Math.min(largeurCase, 420));
}

/** Sous l'objet, aligné sur son bord gauche ; au-dessus quand le bas est
 *  trop proche ; toujours dans la fenêtre.
 *
 *  `bas` est la limite qu'il ne doit pas franchir : le haut de la barre
 *  contextuelle (`basUtile`), pas le bas de la fenêtre. Mesuré avec #55 : un
 *  bloc posé bas ouvrait ses réglages par-dessus « Bloc de texte » et
 *  « Ornement ». */
export function placerSous(
  ancre: Boite,
  taille: { w: number; h: number },
  fenetre: { w: number; h: number },
  bas: number = fenetre.h,
): { left: number; top: number } {
  const left = Math.max(ECART, Math.min(ancre.left, fenetre.w - taille.w - ECART));
  const dessous = ancre.bottom + ECART;
  const top =
    dessous + taille.h + ECART > bas
      ? Math.max(ECART, ancre.top - taille.h - ECART)
      : dessous;
  return { left, top };
}

/** Un panneau qui s'ouvre vers la gauche depuis un bouton de la colonne de
 *  droite : sous le bouton, bord droit sur le sien, et pas plus haut que ce
 *  qui reste de la fenêtre. Position fixe : la colonne défile, et un panneau
 *  absolu y serait coupé à son bord gauche. */
export function placerAGauche(
  ancre: Boite,
  fenetre: { w: number; h: number },
): { top: number; right: number; maxHeight: number } {
  const top = ancre.bottom + 6;
  return { top, right: Math.max(ECART, fenetre.w - ancre.right), maxHeight: Math.max(120, fenetre.h - top - ECART) };
}

/** Le rectangle droit qui contient des points : l'emprise d'un objet tourné. */
export function enveloppe(points: { x: number; y: number }[]): Boite {
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  return {
    left: Math.min(...xs),
    top: Math.min(...ys),
    right: Math.max(...xs),
    bottom: Math.max(...ys),
  };
}

/** Le haut de la barre contextuelle, à défaut le bas de la fenêtre : ce
 *  qu'un popover ne recouvre jamais. */
export function basUtile(): number {
  const barre = document.querySelector(".context-line");
  return barre ? barre.getBoundingClientRect().top : window.innerHeight;
}

/** Le style d'un panneau de la colonne des propriétés (dispositions,
 *  ornements) : fixe, posé par `placerAGauche`. En `absolute`, la colonne
 *  qui défile le coupait à son bord gauche. */
export function styleFixeAGauche(
  ancre: Boite,
  fenetre: { w: number; h: number },
): { position: "fixed"; bottom: "auto"; left: "auto"; top: number; right: number; maxHeight: number } {
  return { position: "fixed", bottom: "auto", left: "auto", ...placerAGauche(ancre, fenetre) };
}
