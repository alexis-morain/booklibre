// La légende de la qualité, dans la barre de statut : un compte par cause,
// la plus grave d'abord, rien quand tout est bon, et chaque compte mène à
// la première planche concernée puis à la suivante.

import { afterEach, describe, expect, it } from "vitest";
import { setLangue } from "./i18n";
import type { Alerte } from "./photos";
import { legendeQualite, motDeLigne, plancheSuivante, signesDePlanche } from "./qualite";

const petite: Alerte = { code: "sous_resolution", ppi: 210 };
const sombre: Alerte = { code: "sombre" };
const floue: Alerte = { code: "floue" };

describe("legendeQualite", () => {
  it("compte les cases par cause, la plus grave d'abord", () => {
    const lignes = legendeQualite([
      [[floue], []],
      [[petite, floue], [sombre]],
      [[]],
      [[petite], [floue]],
    ]);
    expect(lignes).toEqual([
      { code: "sous_resolution", n: 2, planches: [1, 3] },
      { code: "sombre", n: 1, planches: [1] },
      { code: "floue", n: 3, planches: [0, 1, 3] },
    ]);
  });

  it("ne dit rien d'une cause absente", () => {
    expect(legendeQualite([[[floue]], [[floue]]])).toEqual([
      { code: "floue", n: 2, planches: [0, 1] },
    ]);
  });

  it("ne dit rien quand tout est bon", () => {
    expect(legendeQualite([])).toEqual([]);
    expect(legendeQualite([[[], []], [[]]])).toEqual([]);
  });
});

describe("plancheSuivante", () => {
  it("le premier clic mène à la première planche concernée", () => {
    expect(plancheSuivante([3, 7, 12], 9, null)).toBe(3);
  });

  it("chaque clic suivant mène à la suivante, puis revient à la première", () => {
    expect(plancheSuivante([3, 7, 12], 3, 3)).toBe(7);
    expect(plancheSuivante([3, 7, 12], 7, 7)).toBe(12);
    expect(plancheSuivante([3, 7, 12], 12, 12)).toBe(3);
  });

  it("une planche quittée entre deux clics fait repartir de la première", () => {
    expect(plancheSuivante([3, 7, 12], 5, 3)).toBe(3);
  });
});

describe("motDeLigne", () => {
  afterEach(() => setLangue("en"));

  it("accorde en français", () => {
    setLangue("fr");
    expect(motDeLigne({ code: "sous_resolution", n: 2, planches: [] })).toBe("2 trop petites");
    expect(motDeLigne({ code: "sous_resolution", n: 1, planches: [] })).toBe("1 trop petite");
    expect(motDeLigne({ code: "sombre", n: 1, planches: [] })).toBe("1 sombre");
    expect(motDeLigne({ code: "floue", n: 3, planches: [] })).toBe("3 peu nettes");
  });

  it("et en anglais", () => {
    setLangue("en");
    expect(motDeLigne({ code: "sous_resolution", n: 2, planches: [] })).toBe("2 too small");
    expect(motDeLigne({ code: "sombre", n: 4, planches: [] })).toBe("4 dark");
    expect(motDeLigne({ code: "floue", n: 1, planches: [] })).toBe("1 not sharp");
  });
});

describe("signesDePlanche", () => {
  it("un signe par cause, le plus grave d'abord, le ppi de la case la plus pauvre", () => {
    expect(
      signesDePlanche([[floue], [{ code: "sous_resolution", ppi: 230 }, sombre], [petite, floue]]),
    ).toEqual([{ code: "sous_resolution", ppi: 210 }, sombre, floue]);
  });

  it("rien pour une planche sans alerte", () => {
    expect(signesDePlanche([[], []])).toEqual([]);
  });
});
