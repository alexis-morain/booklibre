// Le dossier de photos de l'album ouvert, tel que l'écran le dit.
//
// `introuvable` : le dossier a disparu ou changé de place ; l'aperçu tourne
// sur les vignettes, l'export pleine résolution ne marchera pas.
// `a-revoir` : la version Mac App Store ne peut plus le lire (pas de
// marque-page, ou un marque-page que le système ne résout plus). Celui-là a
// une issue, un bouton qui rouvre le choix du dossier, et il passe devant :
// sous le bac à sable, « introuvable » peut n'être qu'un accès fermé.

import type { OpenedAlbum } from "./album";

export type EtatRacine = "present" | "introuvable" | "a-revoir";

export function etatRacine(
  o: Pick<OpenedAlbum, "root_present" | "dossier_a_revoir">,
): EtatRacine {
  if (o.dossier_a_revoir) return "a-revoir";
  return o.root_present ? "present" : "introuvable";
}
