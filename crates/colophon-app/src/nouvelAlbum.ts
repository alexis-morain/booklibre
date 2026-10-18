// La logique pure de l'écran « Nouvel album » : quels formats il offre, ce
// qu'il annonce du livre avant toute composition, et la photo qu'il montre en
// couverture. Rien ici ne lit le disque ni le moteur : les chiffres arrivent
// en arguments, ce qui garde l'estimation testable sans dossier.

import type { FormatPreset, Miniature } from "./bridge";

/** Les formats du moteur que Colophon sait faire imprimer, dans l'ordre du
 *  moteur. Tant que la liste des imprimables n'est pas arrivée (`null`),
 *  tous restent : un écran sans carte serait pire qu'une carte de trop
 *  pendant quelques millisecondes. */
export function formatsOfferts(
  formats: FormatPreset[],
  imprimables: string[] | null,
): FormatPreset[] {
  if (imprimables === null) return formats;
  return formats.filter((f) => imprimables.includes(f.name));
}

export type Estimation = { photos: number; planches: number; pages: number };

/**
 * Ce que le livre portera, à la manière du Composer (`build.rs`) : un petit
 * dossier entre en entier, et ses planches suivent ses photos ; sinon le
 * budget vaut les planches demandées fois la densité moyenne du rythme, et
 * un dossier qui n'atteint pas ce budget donne moins de planches. La
 * curation retire encore des doublons et des parasites : l'écran dit
 * « environ », et le nombre de photos sort arrondi (`arrondiDizaine`).
 */
export function estimerLivre(e: {
  photos: number;
  petit: boolean;
  planches: number;
  parPlanche: number;
}): Estimation {
  if (e.photos <= 0) return { photos: 0, planches: 0, pages: 0 };
  const budget = Math.floor(e.planches * e.parPlanche);
  const livre = e.petit ? e.photos : Math.min(e.photos, budget);
  const voulues = Math.max(1, Math.ceil(livre / e.parPlanche));
  const planches = e.petit ? voulues : Math.min(e.planches, voulues);
  return { photos: arrondiDizaine(livre), planches, pages: planches * 2 };
}

/** À la dizaine la plus proche ; sous vingt le chiffre exact, qu'un arrondi
 *  rendrait faux du simple au double. */
export function arrondiDizaine(n: number): number {
  return n < 20 ? n : Math.round(n / 10) * 10;
}

/** La photo de la couverture proposée : la première paysage ou carrée, comme
 *  le Composer en choisit une (`build::photo_de_couverture`), sinon la
 *  première. Ce n'est pas celle que le Composer prendra : l'écran le dit. */
export function couvertureProposee(v: Miniature[]): Miniature | null {
  return v.find((x) => x.w >= x.h) ?? v[0] ?? null;
}
