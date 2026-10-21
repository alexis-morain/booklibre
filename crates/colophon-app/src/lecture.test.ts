// La visionneuse (1.2-c), sa part pure : la loupe et l'avant du livre fermé.

import { describe, expect, it } from "vitest";
import { coverSheet } from "./album";
import { bandeDe, decoupeAvant, largeurDeLoupe, partDe, pasDeLoupe } from "./lecture";

describe("la loupe va page par page", () => {
  it("passe de la gauche à la droite de la même planche", () => {
    expect(pasDeLoupe({ planche: 3, cote: "gauche" }, 1, 10)).toEqual({ planche: 3, cote: "droite" });
    expect(pasDeLoupe({ planche: 3, cote: "droite" }, -1, 10)).toEqual({ planche: 3, cote: "gauche" });
  });

  it("franchit le pli d'une planche à l'autre", () => {
    expect(pasDeLoupe({ planche: 3, cote: "droite" }, 1, 10)).toEqual({ planche: 4, cote: "gauche" });
    expect(pasDeLoupe({ planche: 4, cote: "gauche" }, -1, 10)).toEqual({ planche: 3, cote: "droite" });
  });

  it("s'arrête aux deux bouts du livre, sans revenir à la couverture", () => {
    expect(pasDeLoupe({ planche: 0, cote: "gauche" }, -1, 10)).toBeNull();
    expect(pasDeLoupe({ planche: 9, cote: "droite" }, 1, 10)).toBeNull();
  });
});

describe("la largeur de la planche qu'on tire pour la loupe", () => {
  it("fait tenir une page dans la boîte, fond perdu compris", () => {
    // Une page de 210 × 210 et 3 mm de fond perdu : 213 × 216.
    const l = largeurDeLoupe({ w: 2000, h: 600 }, { w: 210, h: 210 }, 3);
    expect(l).toBe(2 * Math.floor((600 * 213) / 216));
  });

  it("se règle sur la largeur quand la boîte est étroite", () => {
    expect(largeurDeLoupe({ w: 300, h: 2000 }, { w: 210, h: 210 }, 3)).toBe(600);
  });

  it("ne rend rien de négatif ni de fractionnaire", () => {
    expect(largeurDeLoupe({ w: 0, h: 0 }, { w: 210, h: 210 }, 3)).toBe(0);
    expect(Number.isInteger(largeurDeLoupe({ w: 333, h: 777 }, { w: 210, h: 297 }, 3))).toBe(true);
  });
});

const album = { trim_mm: { w: 210, h: 210 }, spreads: new Array(24).fill(null) };
// Le dos fourni par le fournisseur : 0 mm de dos dans la feuille, ce qui
// laisse les cotes se lire à la main.
const souple = {
  dos: { mode: "fourni" } as const,
  bleed_mm: { haut: 3, bas: 3, exterieur: 3 },
};

describe("l'avant du livre fermé, découpé dans la feuille de couverture", () => {
  it("prend le panneau avant et le fond perdu de l'album, à la taille d'une demi-planche", () => {
    const profil = souple;
    const f = coverSheet(album, profil);
    const d = decoupeAvant(f, profil, 3);
    expect(d.x * f.w).toBeCloseTo(f.front[0], 9);
    expect(d.w * f.w).toBeCloseTo(213, 9);
    expect(d.y * f.h).toBeCloseTo(0, 9);
    expect(d.h * f.h).toBeCloseTo(216, 9);
  });

  it("descend sous le rempli d'un cartonné : on ne montre pas ce qui se replie", () => {
    const profil = {
      ...souple,
      rempli_mm: 15,
      debord_mm: 3,
      mors_mm: 7,
    };
    const f = coverSheet(album, profil);
    const d = decoupeAvant(f, profil, 3);
    expect(d.x * f.w).toBeCloseTo(f.front[0], 9);
    expect(d.y * f.h).toBeCloseTo(3 + 15 + 3 - 3, 9);
    expect(d.h * f.h).toBeCloseTo(216, 9);
    expect(d.w * f.w).toBeCloseTo(213, 9);
  });

  it("reste dans la feuille même quand l'album a plus de fond perdu que le profil", () => {
    const profil = { ...souple, bleed_mm: { haut: 0, bas: 0, exterieur: 0 } };
    const f = coverSheet(album, profil);
    const d = decoupeAvant(f, profil, 5);
    expect(d.x).toBeGreaterThanOrEqual(0);
    expect(d.y).toBeGreaterThanOrEqual(0);
    expect(d.x + d.w).toBeLessThanOrEqual(1 + 1e-12);
    expect(d.y + d.h).toBeLessThanOrEqual(1 + 1e-12);
  });
});

describe("les parts d'un bitmap", () => {
  it("coupe la planche au pli en pixels entiers, même d'une largeur impaire", () => {
    const img = { width: 1001, height: 500 };
    const g = partDe(img, { x: 0, y: 0, w: 0.5, h: 1 });
    const d = partDe(img, { x: 0.5, y: 0, w: 0.5, h: 1 });
    expect(g.sx + g.sw).toBe(d.sx);
    expect(g.sw + d.sw).toBe(1001);
    expect(Number.isInteger(g.sw)).toBe(true);
  });

  it("découpe une page en douze bandes sans trou ni doublon", () => {
    const p = { sx: 501, sy: 0, sw: 500, sh: 500 };
    let x = p.sx;
    for (let k = 0; k < 12; k++) {
      const b = bandeDe(p, [k / 12, (k + 1) / 12]);
      expect(b.sx).toBe(x);
      x = b.sx + b.sw;
    }
    expect(x).toBe(p.sx + p.sw);
  });
});
