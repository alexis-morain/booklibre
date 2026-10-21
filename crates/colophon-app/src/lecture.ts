// La visionneuse (1.2-c), sa part pure : où va la loupe, à quelle taille on
// tire la page qu'elle montre, et quelle part de la feuille de couverture est
// l'avant du livre fermé. Rien qui touche au DOM : `pdfview.tsx` et
// `Feuilletage.tsx` lisent ces réponses.

import type { CoverSheet } from "./album";
import type { Cote } from "./feuille";

/** La page que montre la loupe : une moitié d'une planche. */
export type PageLue = { planche: number; cote: Cote };

/**
 * La page suivante ou précédente sous la loupe, ou `null` au bout du livre.
 * La loupe lit l'intérieur seulement : la couverture a sa propre vue à plat,
 * et y revenir d'une flèche ferait d'une page une feuille de trois panneaux.
 */
export function pasDeLoupe(page: PageLue, sens: 1 | -1, total: number): PageLue | null {
  if (sens === 1) {
    if (page.cote === "gauche") return { planche: page.planche, cote: "droite" };
    return page.planche + 1 < total ? { planche: page.planche + 1, cote: "gauche" } : null;
  }
  if (page.cote === "droite") return { planche: page.planche, cote: "gauche" };
  return page.planche - 1 >= 0 ? { planche: page.planche - 1, cote: "droite" } : null;
}

/**
 * La largeur, en pixels CSS, de la planche entière qu'il faut tirer pour que
 * l'une de ses pages tienne dans `boite`. La loupe montre une moitié de ce
 * bitmap à sa taille : jamais un agrandissement CSS d'une image plus petite,
 * qui serait la page la plus floue de l'écran au moment où l'on regarde de
 * près.
 *
 * Le PDF d'aperçu porte le fond perdu (`pdf.rs::geometry`) : une page fait
 * donc `w + fond perdu` sur `h + 2 × fond perdu`.
 */
export function largeurDeLoupe(
  boite: { w: number; h: number },
  trim: { w: number; h: number },
  bleed: number,
): number {
  const rapport = (trim.w + bleed) / (trim.h + 2 * bleed);
  const page = Math.min(boite.w, boite.h * rapport);
  return Math.max(0, 2 * Math.floor(page));
}

/** Un rectangle en fractions d'une image, origine en haut à gauche. */
export type Decoupe = { x: number; y: number; w: number; h: number };

/**
 * L'avant du livre fermé dans la feuille de couverture, en fractions de la
 * feuille : le panneau avant, plus le fond perdu de l'album en haut, en bas et
 * au bord extérieur — si bien qu'il a exactement la forme d'une demi-planche
 * du PDF d'intérieur, et que le plat qui tourne se pose sur la page de droite
 * sans changer de taille. Ce qui se replie sur le carton (rempli, débord)
 * reste hors de la découpe ; la découpe reste dans la feuille.
 */
export function decoupeAvant(
  feuille: CoverSheet,
  profil: {
    bleed_mm: { haut: number; bas: number };
    rempli_mm?: number;
    debord_mm?: number;
  },
  bleed: number,
): Decoupe {
  const carton = (profil.rempli_mm ?? 0) + (profil.debord_mm ?? 0);
  const haut = profil.bleed_mm.haut + carton;
  const bas = profil.bleed_mm.bas + carton;
  const [x0, largeur] = feuille.front;
  const x1 = Math.min(feuille.w, x0 + largeur + bleed);
  const y0 = Math.max(0, haut - bleed);
  const y1 = Math.min(feuille.h, feuille.h - bas + bleed);
  return {
    x: x0 / feuille.w,
    y: y0 / feuille.h,
    w: (x1 - x0) / feuille.w,
    h: (y1 - y0) / feuille.h,
  };
}

/** Une part d'un bitmap, en pixels entiers de la source. */
export type Part = { sx: number; sy: number; sw: number; sh: number };

/**
 * La part d'une image que désigne une découpe en fractions, en pixels
 * entiers. Les bords sont arrondis chacun de leur côté, jamais la largeur :
 * deux découpes voisines se partagent ainsi le même pixel de frontière, et
 * une page découpée en lamelles ne montre ni trou ni doublon.
 */
export function partDe(img: { width: number; height: number }, d: Decoupe): Part {
  const x0 = Math.round(d.x * img.width);
  const x1 = Math.round((d.x + d.w) * img.width);
  const y0 = Math.round(d.y * img.height);
  const y1 = Math.round((d.y + d.h) * img.height);
  return { sx: x0, sy: y0, sw: x1 - x0, sh: y1 - y0 };
}

/** Une bande verticale d'une part, `[f0, f1]` en fractions de sa largeur. */
export function bandeDe(p: Part, [f0, f1]: [number, number]): Part {
  const x0 = p.sx + Math.round(f0 * p.sw);
  const x1 = p.sx + Math.round(f1 * p.sw);
  return { sx: x0, sy: p.sy, sw: x1 - x0, sh: p.sh };
}
