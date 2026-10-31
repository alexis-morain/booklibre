// Le texte que l'écran grossit (retours du 30/10). L'éditeur dessine le
// texte du moteur d'un tiers plus gros qu'il ne s'imprime, avec un plancher
// pour qu'il reste lisible : sur une petite planche, ce plancher faisait
// remonter la légende sur sa photo et sortir le colophon de sa page.

import { describe, expect, it } from "vitest";
import { blocQuiTient, legendeSousSaCase } from "./texte-ecran";

describe("la légende d'une photo à l'écran", () => {
  it("garde sa ligne d'impression quand le corps grossi tient sous la case", () => {
    // Grande planche : 4 px/mm, corps 12 px, la case finit 3,4 mm plus haut.
    const l = legendeSousSaCase({ ligneMm: 103.4, basCaseMm: 100, mm: 4, corpsPx: 12 });
    expect(l.lignePx).toBeCloseTo(103.4 * 4);
    expect(l.hautPx).toBeGreaterThan(100 * 4);
  });

  it("descend quand le plancher la ferait remonter sur la photo", () => {
    // Petite planche : 1,4 px/mm, corps planché à 9 px. Posée sur sa ligne
    // d'impression, son haut monterait 2,9 mm dans la photo.
    const l = legendeSousSaCase({ ligneMm: 103.4, basCaseMm: 100, mm: 1.4, corpsPx: 9 });
    expect(l.hautPx).toBeGreaterThanOrEqual(100 * 1.4 + 1);
    expect(l.lignePx).toBeCloseTo(l.hautPx + 0.85 * 9);
  });
});

describe("un bloc du moteur à l'écran", () => {
  // Une mesure factice : un caractère vaut la moitié du corps.
  const mesure = (s: string, sizeMm: number) => s.length * sizeMm * 0.5;
  const colophon = {
    sizeMm: 3,
    leadingMm: 4.6,
    plancherPx: 11,
    // La page de droite d'un carré 30 : la place à droite de l'ancre, et la
    // ligne au-dessus de la coupe du bas.
    roomMm: 275,
    basMm: 296,
    mesure,
  };
  const texte =
    "Colophon\n\n54 photographs kept out of 228, taken from 27 October to 3 November 2013.\nCalvi.\nV-LUX 20 and iPhone 5.\n\nComposed on 28 October 2026 with Booklibre 1.1.0.\n300 × 300 mm, 150 g/m² paper.";

  it("garde le grossissement et le plancher quand ils tiennent", () => {
    const b = blocQuiTient({ ...colophon, text: texte, ancreMm: 200, mm: 6 });
    expect(b.corpsPx).toBeCloseTo(3 * 6 * 1.35);
    expect(b.interlignePx).toBeCloseTo((b.corpsPx * 4.6) / 3);
  });

  it("ne sort jamais de sa page en largeur", () => {
    // 1,4 px/mm : planché à 11 px, la ligne la plus longue ferait 2,6 fois sa
    // largeur d'impression et passerait la coupe.
    const b = blocQuiTient({ ...colophon, roomMm: 100, text: texte, ancreMm: 200, mm: 1.4 });
    const plusLongue = Math.max(...texte.split("\n").map((l) => mesure(l, 3)));
    const largeurEcran = plusLongue * (b.corpsPx / (3 * 1.4)) * 1.4;
    expect(largeurEcran).toBeLessThanOrEqual(100 * 1.4 + 1e-6);
  });

  it("ne sort jamais de sa page par le bas", () => {
    const b = blocQuiTient({ ...colophon, text: texte, ancreMm: 270, mm: 1.4 });
    const lignes = texte.split("\n").length;
    const bas = 270 * 1.4 - b.corpsPx + lignes * b.interlignePx;
    expect(bas).toBeLessThanOrEqual(296 * 1.4 + 1e-6);
  });

  it("un texte vide garde le corps voulu", () => {
    const b = blocQuiTient({ ...colophon, text: "", ancreMm: 200, mm: 1.4 });
    expect(b.corpsPx).toBe(11);
  });
});
