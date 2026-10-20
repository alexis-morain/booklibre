// Le glisser des planches au pointeur : la cible est un calcul pur, testé
// ici sans DOM. La couverture est la cellule zéro de la grille, et elle ne
// reçoit jamais une planche.

import { describe, expect, it } from "vitest";
import { setLangue } from "./i18n";
import { cibleSous, ditSuppression, RectCellule, seuilFranchi, SEUIL_GLISSER_PX } from "./planches";

const cellule = (at: number, left: number, top: number): RectCellule => ({
  at,
  left,
  top,
  right: left + 100,
  bottom: top + 80,
});

// Une rangée : la couverture, puis trois planches, 12 px de gouttière.
const RANGEE = [
  cellule(-1, 0, 0),
  cellule(0, 112, 0),
  cellule(1, 224, 0),
  cellule(2, 336, 0),
];

describe("cibleSous", () => {
  it("rend la planche dont le rectangle contient le pointeur", () => {
    expect(cibleSous(RANGEE, 150, 40)).toBe(0);
    expect(cibleSous(RANGEE, 400, 10)).toBe(2);
  });

  it("compte les bords comme dedans", () => {
    expect(cibleSous(RANGEE, 224, 0)).toBe(1);
    expect(cibleSous(RANGEE, 324, 80)).toBe(1);
  });

  it("ne rend rien hors de toute cellule", () => {
    expect(cibleSous(RANGEE, 600, 40)).toBeNull();
    expect(cibleSous(RANGEE, 150, 200)).toBeNull();
  });

  it("ne rend rien entre deux cellules, dans la gouttière", () => {
    expect(cibleSous(RANGEE, 218, 40)).toBeNull();
  });

  it("n'offre jamais la couverture comme cible", () => {
    expect(cibleSous(RANGEE, 50, 40)).toBeNull();
  });

  it("ne rend rien sur une grille vide", () => {
    expect(cibleSous([], 10, 10)).toBeNull();
  });
});

describe("seuilFranchi", () => {
  it("laisse un clic rester un clic sous le seuil", () => {
    expect(seuilFranchi(0, 0)).toBe(false);
    expect(seuilFranchi(3, 4)).toBe(false);
    expect(seuilFranchi(SEUIL_GLISSER_PX - 0.1, 0)).toBe(false);
  });

  it("commence un glisser au seuil, dans tous les sens", () => {
    expect(seuilFranchi(SEUIL_GLISSER_PX, 0)).toBe(true);
    expect(seuilFranchi(0, -SEUIL_GLISSER_PX)).toBe(true);
    expect(seuilFranchi(-5, 5)).toBe(true);
  });
});

describe("ditSuppression", () => {
  const cloudprinter = { pages_min: 24, fichiers: "deux" as const };
  const lulu = { pages_min: 24, fichiers: "un" as const };

  it("au-dessus du minimum, la phrase d'avant", () => {
    setLangue("fr");
    expect(ditSuppression(3, 12, cloudprinter)).toBe("Planche 3 supprimée (⌘Z la ramène)");
  });

  it("sous le minimum, le dit sans bloquer, avec le compte du prévol", () => {
    setLangue("fr");
    expect(ditSuppression(3, 11, cloudprinter)).toBe(
      "Planche 3 supprimée (⌘Z la ramène). L’album a maintenant 22 pages, sous les 24 que demande ce format : l’impression le refusera tant qu’il en manque",
    );
  });

  it("compte les deux pages de couverture chez qui relie un seul fichier, comme le prévol", () => {
    setLangue("fr");
    expect(ditSuppression(1, 11, lulu)).toBe("Planche 1 supprimée (⌘Z la ramène)");
    expect(ditSuppression(1, 10, lulu)).toContain("22 pages, sous les 24");
  });

  it("sans profil en mémoire, la phrase d'avant", () => {
    setLangue("fr");
    expect(ditSuppression(1, 1, null)).toBe("Planche 1 supprimée (⌘Z la ramène)");
  });
});
