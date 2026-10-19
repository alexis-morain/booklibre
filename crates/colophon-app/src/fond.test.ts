// Le fond de la couverture, côté écran (A4) : les huit pastilles, les cinq
// teintes tirées de la photo, et le nom qu'une couleur porte à VoiceOver.

import { afterEach, describe, expect, it } from "vitest";
import { couleurValide } from "./couleur";
import {
  FOND_CREME,
  FOND_NOIR,
  SATURATION_MAX,
  TEINTES,
  nomDuFond,
  pastillesFond,
  teintesDe,
} from "./fond";
import { setLangue, t } from "./i18n";

afterEach(() => setLangue("en"));

/** Une image RGBA faite de bandes verticales, une couleur par bande, aux
 *  largeurs données : de quoi savoir d'avance ce que les k-moyennes doivent
 *  retrouver, et dans quel ordre. */
function bandes(couleurs: [number, number, number][], largeurs: number[], h = 8) {
  const w = largeurs.reduce((a, b) => a + b, 0);
  const px = new Uint8ClampedArray(w * h * 4);
  for (let y = 0; y < h; y++) {
    let x = 0;
    couleurs.forEach((c, i) => {
      for (let k = 0; k < largeurs[i]; k++, x++) {
        const o = (y * w + x) * 4;
        px.set([c[0], c[1], c[2], 255], o);
      }
    });
  }
  return { px, w, h };
}

/** La saturation HSL d'un `#rrggbb`, de 0 à 1. */
function saturation(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return 0;
  return (max - min) / (1 - Math.abs(2 * l - 1));
}

const CINQ: [number, number, number][] = [
  [200, 40, 30], // rouge vif
  [30, 90, 200], // bleu vif
  [240, 220, 60], // jaune
  [40, 140, 60], // vert
  [90, 60, 40], // brun
];

describe("les teintes de la photo", () => {
  it("retrouve cinq couleurs nettes, la plus étendue d'abord", () => {
    const { px, w, h } = bandes(CINQ, [50, 40, 30, 20, 10]);
    const teintes = teintesDe(px, w, h);
    expect(teintes).toHaveLength(TEINTES);
    for (const hex of teintes) expect(couleurValide(hex), hex).toBe(true);
    // Le rouge occupe la moitié de l'image : il passe en tête, et reste
    // rouge une fois sa saturation ramenée.
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(teintes[0].slice(i, i + 2), 16));
    expect(r).toBeGreaterThan(g + 40);
    expect(r).toBeGreaterThan(b + 40);
  });

  it("est déterministe : la même image rend les mêmes teintes", () => {
    const { px, w, h } = bandes(CINQ, [12, 30, 7, 22, 9]);
    expect(teintesDe(px, w, h)).toEqual(teintesDe(px.slice(), w, h));
  });

  it("ramène les saturations pour l'impression", () => {
    const { px, w, h } = bandes(CINQ, [10, 10, 10, 10, 10]);
    for (const hex of teintesDe(px, w, h)) {
      expect(saturation(hex), hex).toBeLessThanOrEqual(SATURATION_MAX + 0.02);
    }
  });

  it("ne rend jamais deux fois la même teinte, et rien d'une image vide", () => {
    const { px, w, h } = bandes([[120, 120, 120]], [20]);
    expect(teintesDe(px, w, h)).toEqual(["#787878"]);
    expect(teintesDe(new Uint8ClampedArray(0), 0, 0)).toEqual([]);
  });

  it("trouve plusieurs teintes quand aucune case ne pèse un demi-pour-cent", () => {
    // 512 cases de 4 bits, une colonne chacune : chaque case pèse 0,2 %.
    const couleurs: [number, number, number][] = [];
    for (let i = 0; i < 512; i++) couleurs.push([(i & 7) * 32 + 8, ((i >> 3) & 7) * 32 + 8, (i >> 6) * 32 + 8]);
    const { px, w, h } = bandes(couleurs, couleurs.map(() => 1));
    const teintes = teintesDe(px, w, h);
    expect(teintes.length).toBeGreaterThan(1);
    expect(teintesDe(px.slice(), w, h)).toEqual(teintes);
  });

  it("ignore les pixels transparents", () => {
    const { px, w, h } = bandes([[200, 40, 30], [30, 90, 200]], [10, 10]);
    for (let i = 0; i < px.length; i += 4) if ((i / 4) % w >= 10) px[i + 3] = 0;
    expect(teintesDe(px, w, h)).toHaveLength(1);
  });
});

describe("les huit pastilles", () => {
  it("blanc papier, crème, noir, puis les teintes de la photo", () => {
    const p = pastillesFond(["#aa3322", "#2255aa", "#ccbb44", "#337744", "#553322"]);
    expect(p.map((x) => x.hex)).toEqual([
      null,
      FOND_CREME,
      FOND_NOIR,
      "#aa3322",
      "#2255aa",
      "#ccbb44",
      "#337744",
      "#553322",
    ]);
    expect(FOND_CREME).toBe("#f3ead8");
    expect(FOND_NOIR).toBe("#1a1a1a");
  });

  it("garde ses trois premières sans photo", () => {
    expect(pastillesFond([]).map((x) => x.hex)).toEqual([null, FOND_CREME, FOND_NOIR]);
  });

  it("ne répète pas une teinte égale à une couleur fixe", () => {
    expect(pastillesFond([FOND_NOIR, "#aa3322"]).map((x) => x.hex)).toEqual([
      null,
      FOND_CREME,
      FOND_NOIR,
      "#aa3322",
    ]);
  });
});

describe("le nom d'un fond", () => {
  it("nomme les trois fixes, et une teinte par sa famille", () => {
    setLangue("fr");
    expect(nomDuFond(null)).toBe(t("fond.nom.papier"));
    expect(nomDuFond(FOND_CREME)).toBe(t("fond.nom.creme"));
    expect(nomDuFond(FOND_NOIR)).toBe(t("fond.nom.noir"));
    expect(nomDuFond("#b04a1f")).toBe(t("fond.nom.rouge"));
    expect(nomDuFond("#2255aa")).toBe(t("fond.nom.bleu"));
    expect(nomDuFond("#337744")).toBe(t("fond.nom.vert"));
    expect(nomDuFond("#777777")).toBe(t("fond.nom.gris"));
    // Une valeur gâtée vaut « pas de fond », comme au moteur.
    expect(nomDuFond("noir")).toBe(t("fond.nom.papier"));
  });
});
