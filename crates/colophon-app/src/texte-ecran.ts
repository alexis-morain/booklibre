// Le texte du moteur tel que l'écran le grossit, sans qu'il quitte sa place.
//
// L'éditeur dessine les légendes et les pages de texte d'un tiers plus gros
// qu'elles ne s'impriment, avec un plancher en pixels pour qu'elles restent
// lisibles. Sur une petite planche, le plancher l'emporte de loin : une
// légende de 7 pt posée sur sa ligne d'impression remontait dans sa photo,
// et le colophon sortait de sa page à droite et en bas (retours du 30/10).
// Le papier n'a jamais eu ce défaut ; ces deux fonctions retirent à l'écran
// ce que le grossissement lui ajoutait. Rien ici ne touche au PDF.

/** Le grossissement de l'écran, celui de tout le texte du moteur. */
export const GROSSISSEMENT = 1.35;

/** Part du corps au-dessus de la ligne de base : c'est ce que la feuille de
 *  style remonte (`.photo-caption`, `translateY(-0.85em)`). */
const MONTEE = 0.85;

/**
 * Où poser une légende de photo à l'écran : sur sa ligne d'impression, sauf
 * si le corps grossi la ferait mordre sur la case, auquel cas elle descend
 * juste ce qu'il faut pour commencer un pixel sous la photo.
 */
export function legendeSousSaCase(o: {
  ligneMm: number;
  basCaseMm: number;
  mm: number;
  corpsPx: number;
}): { lignePx: number; hautPx: number } {
  const montee = MONTEE * o.corpsPx;
  const lignePx = Math.max(o.ligneMm * o.mm, o.basCaseMm * o.mm + 1 + montee);
  return { lignePx, hautPx: lignePx - montee };
}

/**
 * Le corps et l'interligne d'un bloc du moteur (colophon, page de texte) à
 * l'écran : grossi et planché comme le reste, puis réduit s'il le faut pour
 * que la ligne la plus longue tienne dans `roomMm` et la dernière au-dessus
 * de `basMm`. L'interligne garde la proportion de l'impression, si bien que
 * le bloc à l'écran est le bloc imprimé à une autre échelle.
 */
export function blocQuiTient(o: {
  text: string;
  /** La première ligne de base, en millimètres depuis le haut. */
  ancreMm: number;
  sizeMm: number;
  leadingMm: number;
  mm: number;
  plancherPx: number;
  roomMm: number;
  basMm: number;
  mesure: (s: string, sizeMm: number) => number;
}): { corpsPx: number; interlignePx: number } {
  const voulu = Math.max(o.sizeMm * o.mm * GROSSISSEMENT, o.plancherPx);
  const lignes = o.text.split("\n");
  const interligneMm = Math.max(o.leadingMm, o.sizeMm * 1.3);
  const largeur = Math.max(0, ...lignes.map((l) => (l ? o.mesure(l, o.sizeMm) : 0)));
  // L'échelle `k` multiplie le bloc d'impression : corps k·size, interligne
  // k·interligne. Le haut est à `ancre − k·size`, le bas après n lignes.
  const enLargeur = largeur > 0 ? o.roomMm / largeur : Infinity;
  const hauteur = lignes.length * interligneMm - o.sizeMm;
  const enHauteur =
    o.text !== "" && hauteur > 0 ? Math.max(0, o.basMm - o.ancreMm) / hauteur : Infinity;
  const corpsPx = Math.min(voulu, o.sizeMm * o.mm * Math.min(enLargeur, enHauteur));
  return { corpsPx, interlignePx: (corpsPx * interligneMm) / o.sizeMm };
}
