// Le fond de la couverture et du dos (A4), côté écran : les huit pastilles,
// les cinq teintes tirées de la photo de couverture, et le nom d'une couleur.
//
// Le moteur ne connaît que `cover.fond`, un `#rrggbb` qu'il pose en aplat sur
// toute la feuille. Ce module ne fait que proposer : la palette est un
// confort de l'écran, et ce qui s'imprime est la chaîne choisie, rien d'autre.
//
// **Les teintes sont déterministes.** Des k-moyennes sur la vignette, sans
// hasard : les pixels tombent d'abord dans un histogramme de 4 bits par
// canal (4 096 cases, le bruit d'une photo s'y fond), les centres partent de
// la case la plus lourde puis, une à une, de la case la plus éloignée de
// ceux déjà posés. La même vignette rend donc toujours la même palette, et
// un pixel isolé ne devient jamais une teinte : une case doit peser un
// demi-pour-cent de l'image pour partir centre.
//
// **Les saturations sont ramenées pour l'impression.** Une couleur vive de
// l'écran sort du gamut d'une presse en quadrichromie et s'imprime terne,
// autrement que ce qu'on a choisi. Plafonner la saturation à l'écran montre
// d'avance une couleur que le papier sait tenir.

import { couleurValide, versHex, versRgb } from "./couleur";
import { Cle, t } from "./i18n";

export const FOND_CREME = "#f3ead8";
export const FOND_NOIR = "#1a1a1a";

/** Le nombre de teintes tirées de la photo. */
export const TEINTES = 5;

/** Le plafond de saturation HSL d'une teinte proposée. */
export const SATURATION_MAX = 0.55;

/** Au-delà, on échantillonne : une vignette de 1600 px n'a pas besoin de
 *  ses deux millions de pixels pour dire ses cinq couleurs. */
const ECHANTILLONS_MAX = 16384;

/** Le poids minimal d'une case de l'histogramme pour partir centre. */
const POIDS_CENTRE = 0.005;

const ITERATIONS = 24;

type Case = { r: number; g: number; b: number; n: number };

/**
 * Les teintes d'une image RGBA, de la plus étendue à la moins étendue, au
 * plus `k`, sans doublon, en `#rrggbb` aux saturations ramenées. Les pixels
 * à moitié transparents ne comptent pas.
 */
export function teintesDe(px: Uint8ClampedArray, w: number, h: number, k = TEINTES): string[] {
  if (w <= 0 || h <= 0 || px.length < w * h * 4) return [];
  const pas = Math.max(1, Math.floor(Math.sqrt((w * h) / ECHANTILLONS_MAX)));
  const cases = new Map<number, Case>();
  let total = 0;
  for (let y = 0; y < h; y += pas) {
    for (let x = 0; x < w; x += pas) {
      const o = (y * w + x) * 4;
      if (px[o + 3] < 128) continue;
      const [r, g, b] = [px[o], px[o + 1], px[o + 2]];
      const cle = ((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4);
      const c = cases.get(cle);
      if (c) {
        c.r += r;
        c.g += g;
        c.b += b;
        c.n += 1;
      } else cases.set(cle, { r, g, b, n: 1 });
      total += 1;
    }
  }
  if (total === 0) return [];
  // Chaque case à sa couleur moyenne, dans l'ordre de leur clé : un ordre
  // fixe, que les égalités de poids ne rendent jamais ambigu.
  const pts = [...cases.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([, c]) => ({ r: c.r / c.n, g: c.g / c.n, b: c.b / c.n, n: c.n }));
  const d2 = (a: Case, b: Case) => (a.r - b.r) ** 2 + (a.g - b.g) ** 2 + (a.b - b.b) ** 2;

  // Les centres : la case la plus lourde, puis la plus éloignée des centres
  // déjà posés parmi celles qui pèsent assez.
  const candidates = pts.filter((p) => p.n >= Math.max(1, total * POIDS_CENTRE));
  const premiere = candidates.reduce((m, p) => (p.n > m.n ? p : m), candidates[0] ?? pts[0]);
  const centres: Case[] = [{ ...premiere }];
  while (centres.length < k) {
    let loin: Case | null = null;
    let dLoin = 0;
    for (const p of candidates) {
      const d = Math.min(...centres.map((c) => d2(p, c)));
      if (d > dLoin) {
        dLoin = d;
        loin = p;
      }
    }
    if (!loin) break;
    centres.push({ ...loin });
  }

  // Lloyd, pondéré par le poids des cases.
  const poids = new Array<number>(centres.length).fill(0);
  for (let it = 0; it < ITERATIONS; it++) {
    const somme = centres.map(() => ({ r: 0, g: 0, b: 0, n: 0 }));
    for (const p of pts) {
      let j = 0;
      let dj = Infinity;
      centres.forEach((c, i) => {
        const d = d2(p, c);
        if (d < dj) {
          dj = d;
          j = i;
        }
      });
      somme[j].r += p.r * p.n;
      somme[j].g += p.g * p.n;
      somme[j].b += p.b * p.n;
      somme[j].n += p.n;
    }
    let bouge = false;
    somme.forEach((s, i) => {
      poids[i] = s.n;
      if (s.n === 0) return;
      const nouveau = { r: s.r / s.n, g: s.g / s.n, b: s.b / s.n, n: s.n };
      if (d2(nouveau, centres[i]) > 1e-6) bouge = true;
      centres[i] = nouveau;
    });
    if (!bouge) break;
  }

  const ordre = centres
    .map((c, i) => ({ c, n: poids[i], i }))
    .filter((x) => x.n > 0)
    .sort((a, b) => b.n - a.n || a.i - b.i);
  const vus = new Set<string>();
  const out: string[] = [];
  for (const { c } of ordre) {
    const hex = pourLImpression([c.r / 255, c.g / 255, c.b / 255]);
    if (vus.has(hex)) continue;
    vus.add(hex);
    out.push(hex);
  }
  return out;
}

/** Une couleur à la saturation plafonnée, en `#rrggbb`. */
function pourLImpression(rgb: [number, number, number]): string {
  const [hue, s, l] = hsl(rgb);
  if (s <= SATURATION_MAX) return versHex(rgb);
  return versHex(depuisHsl(hue, SATURATION_MAX, l));
}

function hsl([r, g, b]: readonly [number, number, number]): [number, number, number] {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = d / (1 - Math.abs(2 * l - 1));
  let h: number;
  if (max === r) h = ((g - b) / d) % 6;
  else if (max === g) h = (b - r) / d + 2;
  else h = (r - g) / d + 4;
  return [(h * 60 + 360) % 360, s, l];
}

function depuisHsl(h: number, s: number, l: number): [number, number, number] {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  const [r, g, b] =
    h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return [r + m, g + m, b + m];
}

/** Une pastille : `hex: null` est le blanc papier, l'absence de fond. */
export type Pastille = { cle: "papier" | "creme" | "noir" | "teinte"; hex: string | null };

/** Les pastilles offertes : blanc papier, crème, noir, puis les teintes de
 *  la photo qui ne redisent pas une des trois. */
export function pastillesFond(teintes: string[]): Pastille[] {
  const fixes: Pastille[] = [
    { cle: "papier", hex: null },
    { cle: "creme", hex: FOND_CREME },
    { cle: "noir", hex: FOND_NOIR },
  ];
  const deja = new Set([FOND_CREME, FOND_NOIR]);
  return [
    ...fixes,
    ...teintes
      .map((h) => h.toLowerCase())
      .filter((h) => !deja.has(h))
      .map((hex) => ({ cle: "teinte" as const, hex })),
  ];
}

/** Le nom d'un fond, pour VoiceOver et les infobulles : les trois fixes par
 *  leur nom, toute autre couleur par sa famille. Un fond hors grammaire vaut
 *  le blanc papier, comme au moteur. */
export function nomDuFond(fond: string | null | undefined): string {
  return t(cleDuFond(fond));
}

function cleDuFond(fond: string | null | undefined): Cle {
  if (!fond || !couleurValide(fond)) return "fond.nom.papier";
  const f = fond.toLowerCase();
  if (f === FOND_CREME) return "fond.nom.creme";
  if (f === FOND_NOIR) return "fond.nom.noir";
  const [h, s, l] = hsl(versRgb(f)!);
  if (s < 0.12 || l < 0.08 || l > 0.95) {
    return l < 0.15 ? "fond.nom.noir" : l > 0.9 ? "fond.nom.blanc" : "fond.nom.gris";
  }
  if (h < 45 && l < 0.3) return "fond.nom.brun";
  if (h < 20 || h >= 345) return "fond.nom.rouge";
  if (h < 45) return "fond.nom.orange";
  if (h < 70) return "fond.nom.jaune";
  if (h < 170) return "fond.nom.vert";
  if (h < 260) return "fond.nom.bleu";
  if (h < 300) return "fond.nom.violet";
  return "fond.nom.rose";
}
