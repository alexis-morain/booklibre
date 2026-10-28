// Les chapitres d'un album, pour la barre latérale et le fil d'Ariane. Un
// chapitre n'est pas une donnée de plus : c'est une planche dont la légende
// de chapitre est posée (`Spread.caption`), qui tient jusqu'à la suivante.
// Les planches d'avant la première légende (faux-titre, mots de l'album) ne
// sont dans aucun chapitre.

import type { Album } from "./album";

export type Chapitre = {
  titre: string;
  /** L'index de la première planche du chapitre. */
  debut: number;
  /** Combien de planches il tient, jusqu'à la légende suivante. */
  planches: number;
};

export function chapitres(album: Pick<Album, "spreads">): Chapitre[] {
  const out: Chapitre[] = [];
  album.spreads.forEach((s, i) => {
    if (s.caption) out.push({ titre: s.caption, debut: i, planches: 1 });
    else if (out.length > 0) out[out.length - 1].planches += 1;
  });
  return out;
}

/** Le chapitre qui contient la planche `index`, ou rien avant le premier. */
export function chapitreDe(liste: readonly Chapitre[], index: number): Chapitre | null {
  let trouve: Chapitre | null = null;
  for (const c of liste) {
    if (c.debut > index) break;
    trouve = c;
  }
  return index < 0 ? null : trouve;
}
