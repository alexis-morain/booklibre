// La quatrième de couverture mise en page (1.2-d) : le cadre de sa photo,
// la bande où son texte se centre, les lignes qui s'impriment et ce qui ne
// tient pas. Port de `cover.rs::quatrieme`, sous parité par les
// `quatrieme_samples` du dump : l'éditeur dessine ce que le PDF imprimera,
// coupé aux mêmes endroits, et c'est le moteur qui fait foi.
//
// Repère du moteur : millimètres, origine en bas à gauche du panneau. La
// mesure prend une taille en **points**, comme celle du moteur, pour que la
// parité compare l'algorithme et non une conversion : l'éditeur lui passe
// `measureMm` converti par `PT_MM`.

import { onGeometrie } from "./geometrie";
import type { Cover, PhotoQuatrieme, Rect } from "./album";
import { PT_MM } from "./album";
import { replier } from "./scene";

/** La part du panneau que la boîte intérieure laisse de chaque côté :
 *  `cover.rs::TITLE_INSET`. */
export const TITLE_INSET = 0.09;
/** L'interligne de la quatrième, en multiples de sa taille. */
export const BACK_LEADING = 1.45;
/** Où l'encre d'une ligne se tient autour de sa ligne de base. */
export const ENCRE_HAUT = 0.75;
export const ENCRE_BAS = 0.25;
export const BACK_TAILLE_MIN_PT = 6;
export const BACK_TAILLE_MAX_PT = 48;
/** La taille d'un livre de 210 mm, mise à l'échelle du format (hydratée). */
export let BACK_TEXT_PT_AT_210 = 10.5;
onGeometrie((d) => {
  const v = d.constantes.back_text_pt_at_210;
  if (typeof v === "number") BACK_TEXT_PT_AT_210 = v;
});

/** Les tailles que l'éditeur offre, en points d'impression. */
export const TAILLES_OFFERTES = [8, 9, 10, 10.5, 12, 14, 18, 24];

const PARTS = { petite: 1 / 3, moyenne: 0.5, grande: 2 / 3 } as const;

/** La taille d'impression de la quatrième : `cover.rs::taille_quatrieme`. */
export function tailleQuatrieme(cover: Cover, trimW: number): number {
  const t = cover.back_taille_pt;
  if (typeof t === "number" && Number.isFinite(t)) {
    return Math.min(BACK_TAILLE_MAX_PT, Math.max(BACK_TAILLE_MIN_PT, t));
  }
  return BACK_TEXT_PT_AT_210 * (trimW / 210);
}

/** Le cadre carré de la photo de quatrième : `cover.rs::cadre_quatrieme`. */
export function cadreQuatrieme(panneau: Rect, p: PhotoQuatrieme): Rect {
  const iw = panneau.w * (1 - 2 * TITLE_INSET);
  const ih = panneau.h * (1 - 2 * TITLE_INSET);
  const cote = PARTS[p.taille ?? "moyenne"] * Math.min(iw, ih);
  const bas = panneau.y + panneau.h * TITLE_INSET;
  const position = p.position ?? "haut";
  const y =
    position === "haut"
      ? bas + ih - cote
      : position === "centre"
        ? panneau.y + (panneau.h - cote) / 2
        : bas;
  return { x: panneau.x + (panneau.w - cote) / 2, y, w: cote, h: cote };
}

export type LigneQuatrieme = { texte: string; x: number; y: number; largeur: number };

export type Quatrieme = {
  cadre: Rect | null;
  bande: Rect;
  lignes: LigneQuatrieme[];
  taillePt: number;
  coupees: number;
  tropLarge: boolean;
};

/** La mise en page : `cover.rs::quatrieme`, expression pour expression. */
export function quatrieme(
  panneau: Rect,
  safe: number,
  cover: Cover,
  taillePt: number,
  mesure: (s: string, pt: number) => number,
): Quatrieme {
  const leading = taillePt * PT_MM * BACK_LEADING;
  const photo = cover.back_photo ?? null;
  const cadre = photo ? cadreQuatrieme(panneau, photo) : null;
  let bande: Rect;
  if (cadre && (photo?.position ?? "haut") === "bas") {
    const y = cadre.y + cadre.h + leading;
    bande = { x: panneau.x, y, w: panneau.w, h: panneau.y + panneau.h - y };
  } else if (cadre) {
    bande = { x: panneau.x, y: panneau.y, w: panneau.w, h: cadre.y - leading - panneau.y };
  } else {
    bande = { ...panneau };
  }
  const q: Quatrieme = { cadre, bande, lignes: [], taillePt, coupees: 0, tropLarge: false };
  const texte = cover.back_text ?? "";
  if (texte === "") return q;
  const boxW = panneau.w - 2 * (panneau.w * TITLE_INSET);
  // `replier` transmet sa taille à la mesure sans la lire : on lui donne des
  // points, comme le moteur.
  const { lignes, tropLarge } = replier(texte, boxW, taillePt, mesure);
  q.tropLarge = tropLarge;
  const plancher = Math.max(panneau.y + safe, bande.y);
  const bloc = lignes.length * leading;
  let y = bande.y + (bande.h + bloc) / 2 - leading;
  for (let i = 0; i < lignes.length; i++) {
    if (i > 0 && y < plancher) {
      q.coupees = lignes.slice(i).filter((l) => l !== "").length;
      break;
    }
    const largeur = mesure(lignes[i], taillePt);
    q.lignes.push({ texte: lignes[i], x: panneau.x + (panneau.w - largeur) / 2, y, largeur });
    y -= leading;
  }
  return q;
}

/** Ce qui sort du panneau ou de sa bande, et ce qui entre dans la bande
 *  sûre : `cover.rs::Quatrieme::ecarts`. */
export function ecartsQuatrieme(q: Quatrieme, panneau: Rect, safe: number): [number, number] {
  const mm = q.taillePt * PT_MM;
  const hautBande = q.bande.y + q.bande.h;
  let dehors = 0;
  let zone = 0;
  for (const l of q.lignes) {
    if (l.texte === "") continue;
    const [x0, x1] = [l.x, l.x + l.largeur];
    const [y0, y1] = [l.y - mm * ENCRE_BAS, l.y + mm * ENCRE_HAUT];
    if (x0 < panneau.x || x1 > panneau.x + panneau.w || y1 > hautBande) dehors += 1;
    else if (
      x0 < panneau.x + safe ||
      x1 > panneau.x + panneau.w - safe ||
      y0 < panneau.y + safe ||
      y1 > panneau.y + panneau.h - safe
    )
      zone += 1;
  }
  const c = q.cadre;
  if (
    c &&
    (c.x < panneau.x + safe ||
      c.x + c.w > panneau.x + panneau.w - safe ||
      c.y < panneau.y + safe ||
      c.y + c.h > panneau.y + panneau.h - safe)
  ) {
    zone += 1;
  }
  return [dehors + q.coupees, zone];
}

// ---- les gestes, sur une couverture -------------------------------------
// Chacun rend une couverture neuve que l'éditeur pose par `onCover`, un pas
// d'annulation. Retirer un champ le retire du fichier, pour qu'une quatrième
// revenue à son état d'avant écrive l'`album.json` d'avant.

/** Poser une photo, ou en changer : le cadre garde sa position et sa
 *  taille ; une photo neuve naît en haut, moyenne. */
export function avecPhotoQuatrieme(cover: Cover, src: string): Cover {
  const avant = cover.back_photo;
  return {
    ...cover,
    back_photo: {
      photo: { src, focal: [0.5, 0.5] },
      position: avant?.position ?? "haut",
      taille: avant?.taille ?? "moyenne",
    },
  };
}

export function sansPhotoQuatrieme(cover: Cover): Cover {
  const { back_photo: _p, ...reste } = cover;
  return reste;
}

export function avecCadre(
  cover: Cover,
  patch: Partial<Pick<PhotoQuatrieme, "photo" | "position" | "taille">>,
): Cover {
  if (!cover.back_photo) return cover;
  return { ...cover, back_photo: { ...cover.back_photo, ...patch } };
}

/** `null` rend la quatrième à la face du livre. */
export function avecPoliceQuatrieme(cover: Cover, police: Cover["back_police"] | null): Cover {
  const { back_police: _p, ...reste } = cover;
  return police ? { ...reste, back_police: police } : reste;
}

/** `null` rend la quatrième à la taille de son format. */
export function avecTailleTexte(cover: Cover, pt: number | null): Cover {
  const { back_taille_pt: _t, ...reste } = cover;
  return pt === null ? reste : { ...reste, back_taille_pt: pt };
}
