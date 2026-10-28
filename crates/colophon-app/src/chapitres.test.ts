import { describe, expect, it } from "vitest";
import { chapitreDe, chapitres } from "./chapitres";

const planche = (caption?: string) => ({ template: "vide", slots: [], caption });
const album = {
  spreads: [
    planche(),
    planche(),
    planche("Calvi, 27 octobre"),
    planche(),
    planche(),
    planche("28 octobre"),
    planche(),
  ],
};

describe("les chapitres d'un album", () => {
  it("compte chaque chapitre de sa légende à la suivante", () => {
    expect(chapitres(album)).toEqual([
      { titre: "Calvi, 27 octobre", debut: 2, planches: 3 },
      { titre: "28 octobre", debut: 5, planches: 2 },
    ]);
  });

  it("n'a aucun chapitre sans légende", () => {
    expect(chapitres({ spreads: [planche(), planche()] })).toEqual([]);
  });

  it("trouve le chapitre d'une planche, et rien avant le premier ni sur la couverture", () => {
    const liste = chapitres(album);
    expect(chapitreDe(liste, 0)).toBeNull();
    expect(chapitreDe(liste, 2)?.titre).toBe("Calvi, 27 octobre");
    expect(chapitreDe(liste, 4)?.titre).toBe("Calvi, 27 octobre");
    expect(chapitreDe(liste, 5)?.titre).toBe("28 octobre");
    expect(chapitreDe(liste, 6)?.titre).toBe("28 octobre");
    expect(chapitreDe(liste, -1)).toBeNull();
  });
});
