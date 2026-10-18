// L'arithmétique de la visionneuse de format (`VisionneuseFormat.tsx`) : une
// seule échelle pour tous les formats, pour que le 30 × 30 se voie plus grand
// que le 21 × 21 dans la même boîte, et une main de taille réelle à côté.

import type { Rect } from "./album";
import { FR, langue, t, type Cle } from "./i18n";

/** Des millimètres dits en centimètres, à la décimale près, dans la langue
 *  de l'écran. */
export const cm = (mm: number) =>
  (mm / 10).toLocaleString(langue() === "fr" ? "fr-FR" : "en-GB", {
    maximumFractionDigits: 1,
  });

/** Le nom d'un format pour l'écran (`format.*`), l'identifiant du moteur
 *  quand le dictionnaire n'en a pas : jamais la phrase `about` du moteur. */
export function formatLabel(name: string): string {
  const cle = `format.${name}`;
  return cle in FR ? t(cle as Cle) : name.replace(/-/g, " ");
}

/** La main de repère, en millimètres : une main adulte, du poignet au bout
 *  du majeur, et sa largeur doigts écartés. */
export const MAIN_MM = { w: 100, h: 190 };

/** Entre la planche la plus large et la main. */
const ECART_MM = 20;

/** Ce que la boîte doit loger, en millimètres : la plus grande planche
 *  ouverte du moteur (30 × 30, soit 600 × 300) et la main à côté. Une
 *  constante, pas un calcul sur les formats offerts : si l'échelle suivait la
 *  liste, retirer un format grossirait tous les autres. */
export const BOITE_MM = { w: 600 + ECART_MM + MAIN_MM.w, h: 300 };

/** Pixels par millimètre pour une boîte de `largeur` pixels. */
export function echelleCommune(largeur: number): number {
  return largeur / BOITE_MM.w;
}

/** La planche ouverte d'un format, en pixels. */
export function planchePx(f: { w: number; h: number }, s: number): { w: number; h: number } {
  return { w: 2 * f.w * s, h: f.h * s };
}

/** Le gabarit montré, un par forme de page : un carré prend quatre photos,
 *  un portrait trois, un paysage six. Des noms du catalogue historique, que
 *  le moteur sait dessiner dans tous les formats. */
export function gabaritApercu(f: { w: number; h: number }): [string, number] {
  if (f.w === f.h) return ["quad", 4];
  return f.h > f.w ? ["trio", 3] : ["six", 6];
}

export type VignetteVue = { url: string; ratio?: number };

/**
 * Une vignette par case, au plus : chaque case prend, parmi celles qui
 * restent, la vignette dont la forme est la plus proche de la sienne (un
 * ratio inconnu compte pour un carré). Une case sans vignette rend `null`,
 * que le composant dessine en aplat.
 */
export function attribuer(cases: Rect[], vignettes: VignetteVue[]): (VignetteVue | null)[] {
  const libres = [...vignettes];
  return cases.map((c) => {
    if (libres.length === 0) return null;
    const cible = Math.log(c.w / c.h);
    let meilleur = 0;
    for (let i = 1; i < libres.length; i++) {
      const ecart = (v: VignetteVue) => Math.abs(Math.log(v.ratio ?? 1) - cible);
      if (ecart(libres[i]) < ecart(libres[meilleur])) meilleur = i;
    }
    return libres.splice(meilleur, 1)[0];
  });
}
