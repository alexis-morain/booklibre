// La légende de la qualité, dans la barre de statut : combien de cases
// portent chaque alerte, et où les trouver. Colophon ne note pas les
// photos ; il dit ce qui imprimera mal, cause par cause, et rien quand tout
// est bon. Les alertes elles-mêmes viennent de `photos.ts::badgesDe`, seule
// règle : ce module ne fait que les compter.

import { t } from "./i18n";
import type { Alerte } from "./photos";

export type CodeAlerte = Alerte["code"];

/** De la plus grave à la plus légère : la résolution est la seule qui
 *  bloque l'impression. C'est aussi l'ordre où une case montre ses signes. */
export const GRAVITE: readonly CodeAlerte[] = ["sous_resolution", "sombre", "floue"];

/** Une cause de la légende : le nombre de cases, et les planches qui les
 *  portent, dans l'ordre du livre. */
export type LigneQualite = { code: CodeAlerte; n: number; planches: number[] };

/**
 * Les alertes de l'album, planche par planche puis case par case, rendues
 * en une ligne par cause présente. Une case qui porte deux alertes compte
 * dans les deux.
 */
export function legendeQualite(
  parPlanche: ReadonlyArray<ReadonlyArray<readonly Alerte[]>>,
): LigneQualite[] {
  return GRAVITE.flatMap((code) => {
    let n = 0;
    const planches: number[] = [];
    parPlanche.forEach((cases, i) => {
      const ici = cases.filter((alertes) => alertes.some((a) => a.code === code)).length;
      if (ici === 0) return;
      n += ici;
      planches.push(i);
    });
    return n > 0 ? [{ code, n, planches }] : [];
  });
}

/**
 * Où mène un clic sur un compte : la première planche concernée, puis la
 * suivante à chaque clic, et de nouveau la première au bout. `derniere` est
 * la planche où le clic précédent sur ce compte a mené ; si l'on n'y est
 * plus, on a quitté le parcours, et il repart du début.
 */
export function plancheSuivante(
  planches: readonly number[],
  courante: number,
  derniere: number | null,
): number {
  if (derniere === null || derniere !== courante) return planches[0];
  return planches.find((p) => p > courante) ?? planches[0];
}

/** « 2 trop petites », « 1 sombre », « 3 peu nettes ». */
export function motDeLigne(l: LigneQualite): string {
  const un = l.n === 1;
  switch (l.code) {
    case "sous_resolution":
      return t(un ? "qualite.sous_resolution.un" : "qualite.sous_resolution", { n: l.n });
    case "sombre":
      return t(un ? "qualite.sombre.un" : "qualite.sombre", { n: l.n });
    case "floue":
      return t(un ? "qualite.floue.un" : "qualite.floue", { n: l.n });
  }
}

/** Les signes d'une planche entière, sur la table lumineuse : un par cause,
 *  la plus grave d'abord, et pour la résolution la case la plus pauvre,
 *  dont le ppi passe dans la bulle. */
export function signesDePlanche(parCase: ReadonlyArray<readonly Alerte[]>): Alerte[] {
  const toutes = parCase.flat();
  return GRAVITE.flatMap((code): Alerte[] => {
    const ici = toutes.filter((a) => a.code === code);
    if (ici.length === 0) return [];
    const ppis = ici.flatMap((a) => (a.code === "sous_resolution" ? [a.ppi] : []));
    return [ppis.length > 0 ? { code: "sous_resolution", ppi: Math.min(...ppis) } : ici[0]];
  });
}
