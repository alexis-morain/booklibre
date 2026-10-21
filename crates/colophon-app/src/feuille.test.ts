// The sheet's own arithmetic, with no canvas, no PDF and no pointer.
//
// What is worth pinning here is the model, not the pixels: which half of
// which spread each face carries, where a corner starts and where it does
// not, what a release means, and the fact that the free edge tracks the
// finger. Everything else — bitmaps, frames, events — is `Feuilletage.tsx`,
// and it reads these answers rather than deciding anything itself.

import { describe, expect, it } from "vitest";
import {
  CLIC,
  COIN,
  COURBURE,
  COUVERTURE,
  bandes,
  courbe,
  voileDe,
  DUREE_MIN,
  DUREE_TOUR,
  SEUIL_PROGRES,
  SEUIL_VITESSE,
  adoucir,
  angle,
  coinTouche,
  dureeRestante,
  estUnClic,
  feuilleDe,
  issue,
  planchesAPrecharger,
  progresDuPointeur,
  relief,
  versoVisible,
} from "./feuille";

describe("what one sheet carries", () => {
  it("holds the right page of a spread and the left page of the next", () => {
    const f = feuilleDe(3, 1, 10);
    expect(f).not.toBeNull();
    expect(f!.recto).toEqual({ planche: 3, cote: "droite" });
    expect(f!.verso).toEqual({ planche: 4, cote: "gauche" });
  });

  it("leaves the other two halves in place, one per spread", () => {
    const f = feuilleDe(3, 1, 10)!;
    expect(f.fixeGauche).toEqual({ planche: 3, cote: "gauche" });
    expect(f.fixeDroite).toEqual({ planche: 4, cote: "droite" });
  });

  it("turns back on the same sheet, read the other way", () => {
    const f = feuilleDe(4, -1, 10)!;
    expect(f.recto).toEqual({ planche: 4, cote: "gauche" });
    expect(f.verso).toEqual({ planche: 3, cote: "droite" });
    expect(f.fixeGauche).toEqual({ planche: 3, cote: "gauche" });
    expect(f.fixeDroite).toEqual({ planche: 4, cote: "droite" });
  });

  it("never needs more than the two spreads it joins", () => {
    for (const sens of [1, -1] as const) {
      const f = feuilleDe(4, sens, 10)!;
      const planches = new Set(
        [f.recto, f.verso, f.fixeGauche!, f.fixeDroite].map((x) => x.planche),
      );
      expect([...planches].sort()).toEqual(sens === 1 ? [4, 5] : [3, 4]);
    }
  });

  it("shows one whole page per face, so nothing that moves is cut", () => {
    const f = feuilleDe(0, 1, 10)!;
    for (const face of [f.recto, f.verso, f.fixeGauche!, f.fixeDroite]) {
      expect(face.cote === "gauche" || face.cote === "droite").toBe(true);
    }
  });
});

// 1.2-c : le livre fermé s'ouvre. Le plat tourne autour du dos comme une
// feuille — son recto est l'avant de la couverture, son verso le contre-plat,
// qui est la page de gauche de la première planche.
describe("the cover opens like a sheet", () => {
  it("turns the front board about the spine onto the first spread", () => {
    const f = feuilleDe(COUVERTURE, 1, 10)!;
    expect(f.depuis).toBe(COUVERTURE);
    expect(f.vers).toBe(0);
    expect(f.recto).toEqual({ planche: COUVERTURE, cote: "droite" });
    expect(f.verso).toEqual({ planche: 0, cote: "gauche" });
    expect(f.fixeDroite).toEqual({ planche: 0, cote: "droite" });
  });

  it("leaves the table bare on the left: a closed book has no left page", () => {
    expect(feuilleDe(COUVERTURE, 1, 10)!.fixeGauche).toBeNull();
    expect(feuilleDe(0, -1, 10)!.fixeGauche).toBeNull();
  });

  it("closes by the same sheet, read the other way", () => {
    const f = feuilleDe(0, -1, 10)!;
    expect(f.vers).toBe(COUVERTURE);
    expect(f.recto).toEqual({ planche: 0, cote: "gauche" });
    expect(f.verso).toEqual({ planche: COUVERTURE, cote: "droite" });
    expect(f.fixeDroite).toEqual({ planche: 0, cote: "droite" });
  });

  it("opens even a book of one spread", () => {
    expect(feuilleDe(COUVERTURE, 1, 1)).not.toBeNull();
    expect(feuilleDe(0, -1, 1)).not.toBeNull();
  });
});

describe("the turns that do not exist", () => {
  it("has none before the cover or after the last spread", () => {
    expect(feuilleDe(COUVERTURE, -1, 10)).toBeNull();
    expect(feuilleDe(9, 1, 10)).toBeNull();
  });

  it("has none in a book with no spread", () => {
    expect(feuilleDe(COUVERTURE, 1, 0)).toBeNull();
  });

  it("has none past the end of a book of one spread", () => {
    expect(feuilleDe(0, 1, 1)).toBeNull();
  });
});

// 1.2-c : la page courbe. Douze lamelles, chacune un angle ; le bord libre
// mène le pli, tout est à plat aux deux bouts du tour.
describe("the curl", () => {
  const n = 12;

  it("is flat at rest on both sides of the fold", () => {
    for (const sens of [1, -1] as const) {
      for (const a of courbe(0, sens, n)) expect(Math.abs(a)).toBeCloseTo(0, 9);
      for (const a of courbe(1, sens, n)) expect(Math.abs(a)).toBeCloseTo(180, 9);
    }
  });

  it("has one angle per strip", () => {
    expect(courbe(0.3, 1, n)).toHaveLength(n);
    expect(courbe(0.3, 1, 1)).toHaveLength(1);
  });

  it("lets the free edge lead the fold, in the direction of the turn", () => {
    for (const sens of [1, -1] as const) {
      const a = courbe(0.5, sens, n);
      const tourne = a.map((x) => -sens * x);
      expect(tourne[n - 1]).toBeGreaterThan(tourne[0] + COURBURE * 0.9);
      // Monotone : le papier se courbe, il ne se plie pas en accordéon.
      for (let k = 1; k < n; k++) expect(tourne[k]).toBeGreaterThanOrEqual(tourne[k - 1]);
    }
  });

  it("never goes under the page it lands on, nor behind the one it leaves", () => {
    for (const sens of [1, -1] as const) {
      for (let p = 0; p <= 1.0001; p += 0.05) {
        for (const x of courbe(p, sens, n)) {
          expect(-sens * x).toBeGreaterThanOrEqual(0);
          expect(-sens * x).toBeLessThanOrEqual(180);
        }
      }
    }
  });

  it("averages to the flat sheet's angle while nothing is clamped", () => {
    const a = courbe(0.5, 1, n);
    const moyenne = a.reduce((s, x) => s + x, 0) / n;
    expect(moyenne).toBeCloseTo(angle(0.5, 1), 9);
  });

  it("shades a strip by how far it stands from the paper", () => {
    expect(voileDe(0)).toBeCloseTo(0, 9);
    expect(voileDe(-180)).toBeCloseTo(0, 9);
    expect(voileDe(-90)).toBeGreaterThan(voileDe(-30));
    expect(voileDe(90)).toBeCloseTo(voileDe(-90), 9);
  });
});

describe("the strips a sheet is cut into", () => {
  it("covers the page with no gap and no overlap, from the fold out", () => {
    for (const sens of [1, -1] as const) {
      const b = bandes(4, sens);
      const rectos = b.map((x) => x.recto).sort((u, v) => u[0] - v[0]);
      expect(rectos[0][0]).toBe(0);
      expect(rectos[3][1]).toBe(1);
      for (let k = 1; k < 4; k++) expect(rectos[k][0]).toBeCloseTo(rectos[k - 1][1], 12);
    }
  });

  it("starts at the fold: the right page's left edge, the left page's right edge", () => {
    expect(bandes(4, 1)[0].recto).toEqual([0, 0.25]);
    expect(bandes(4, -1)[0].recto).toEqual([0.75, 1]);
  });

  it("carries on its back the mirror strip of the page it uncovers", () => {
    for (const sens of [1, -1] as const) {
      for (const { recto, verso } of bandes(4, sens)) {
        expect(verso[0]).toBeCloseTo(1 - recto[1], 12);
        expect(verso[1]).toBeCloseTo(1 - recto[0], 12);
      }
    }
  });
});

describe("what has to be drawn before a finger lands", () => {
  it("asks for the spread on screen and both its neighbours", () => {
    expect(planchesAPrecharger(4, 10)).toEqual([3, 4, 5]);
  });

  it("clips at the ends rather than asking for a spread that is not there", () => {
    expect(planchesAPrecharger(0, 10)).toEqual([0, 1]);
    expect(planchesAPrecharger(9, 10)).toEqual([8, 9]);
    expect(planchesAPrecharger(0, 1)).toEqual([0]);
  });

  it("asks for nothing from the cover", () => {
    expect(planchesAPrecharger(-1, 10)).toEqual([0]);
  });
});

describe("where a corner is", () => {
  it("starts a turn forward from the bottom right", () => {
    expect(coinTouche(0.97, 0.95)).toBe(1);
  });

  it("starts a turn back from the bottom left", () => {
    expect(coinTouche(0.03, 0.95)).toBe(-1);
  });

  it("leaves the middle of the spread alone", () => {
    expect(coinTouche(0.5, 0.95)).toBeNull();
    expect(coinTouche(0.5, 0.5)).toBeNull();
  });

  it("leaves the top of the page alone, corners included", () => {
    expect(coinTouche(0.98, 0.1)).toBeNull();
    expect(coinTouche(0.02, 0.1)).toBeNull();
  });

  it("keeps most of the page free of the gesture", () => {
    expect(COIN.largeur * 2).toBeLessThan(0.4);
    for (let x = COIN.largeur + 0.01; x < 1 - COIN.largeur; x += 0.05) {
      expect(coinTouche(x, 0.99)).toBeNull();
    }
  });
});

describe("the free edge follows the finger", () => {
  it("is flat when the finger is on the corner it started from", () => {
    expect(progresDuPointeur(1000, 1000, 1)).toBeCloseTo(0, 6);
    expect(progresDuPointeur(0, 1000, -1)).toBeCloseTo(0, 6);
  });

  it("is half way through at the fold, both ways", () => {
    expect(progresDuPointeur(500, 1000, 1)).toBeCloseTo(0.5, 6);
    expect(progresDuPointeur(500, 1000, -1)).toBeCloseTo(0.5, 6);
  });

  it("is done when the finger reached the far edge", () => {
    expect(progresDuPointeur(0, 1000, 1)).toBeCloseTo(1, 6);
    expect(progresDuPointeur(1000, 1000, -1)).toBeCloseTo(1, 6);
  });

  it("never leaves the interval, however far the finger goes", () => {
    for (const x of [-4000, -1, 1001, 9000]) {
      for (const sens of [1, -1] as const) {
        const p = progresDuPointeur(x, 1000, sens);
        expect(p).toBeGreaterThanOrEqual(0);
        expect(p).toBeLessThanOrEqual(1);
      }
    }
  });

  it("only ever advances as the finger crosses the spread", () => {
    let precedent = -1;
    for (let x = 1000; x >= 0; x -= 25) {
      const p = progresDuPointeur(x, 1000, 1);
      expect(p).toBeGreaterThan(precedent);
      precedent = p;
    }
  });

  it("answers something rather than dividing by a width of zero", () => {
    expect(progresDuPointeur(0, 0, 1)).toBe(0);
  });
});

describe("the angle and what it uncovers", () => {
  it("lies flat at rest and flat again at the end", () => {
    expect(angle(0, 1)).toBe(-0);
    expect(Math.abs(angle(1, 1))).toBe(180);
    expect(Math.abs(angle(1, -1))).toBe(180);
  });

  it("turns the two ways round the same fold", () => {
    expect(angle(0.25, 1)).toBe(-45);
    expect(angle(0.25, -1)).toBe(45);
  });

  it("shows the back only past the vertical", () => {
    expect(versoVisible(0.49)).toBe(false);
    expect(versoVisible(0.51)).toBe(true);
  });

  it("carries no relief flat, and all of it upright", () => {
    expect(relief(0)).toBeCloseTo(0, 6);
    expect(relief(1)).toBeCloseTo(0, 6);
    expect(relief(0.5)).toBeCloseTo(1, 6);
  });
});

describe("what a release means", () => {
  it("finishes past the half, comes back before it", () => {
    expect(issue(SEUIL_PROGRES, 0)).toBe("termine");
    expect(issue(SEUIL_PROGRES - 0.01, 0)).toBe("revient");
  });

  it("lets a flick decide on its own, short as it is", () => {
    expect(issue(0.05, SEUIL_VITESSE)).toBe("termine");
  });

  it("lets a firm pull back cancel a turn almost done", () => {
    expect(issue(0.8, -SEUIL_VITESSE)).toBe("revient");
  });

  it("ignores a speed too small to be meant", () => {
    expect(issue(0.7, 0.2)).toBe("termine");
    expect(issue(0.2, -0.2)).toBe("revient");
  });
});

describe("a click on the corner", () => {
  it("is a short move over a short time", () => {
    expect(estUnClic(2, 90)).toBe(true);
  });

  it("is not a drag that went somewhere", () => {
    expect(estUnClic(CLIC.course + 1, 90)).toBe(false);
  });

  it("is not a finger that stayed and hesitated", () => {
    expect(estUnClic(2, CLIC.duree + 1)).toBe(false);
  });
});

describe("how long the end of the movement takes", () => {
  it("costs a whole turn from flat", () => {
    expect(dureeRestante(0, 1)).toBe(DUREE_TOUR);
  });

  it("costs less the closer it already is", () => {
    expect(dureeRestante(0.9, 1)).toBeLessThan(dureeRestante(0.4, 1));
  });

  it("never falls under the floor, where a movement stops being read", () => {
    expect(dureeRestante(0.999, 1)).toBe(DUREE_MIN);
    expect(dureeRestante(0.5, 0.5)).toBe(DUREE_MIN);
  });
});

describe("the easing", () => {
  it("starts where it starts and lands where it lands", () => {
    expect(adoucir(0)).toBe(0);
    expect(adoucir(1)).toBe(1);
  });

  it("never goes past its target and never comes back", () => {
    let precedent = -1;
    for (let t = 0; t <= 1.0001; t += 0.05) {
      const v = adoucir(t);
      expect(v).toBeGreaterThanOrEqual(precedent);
      expect(v).toBeLessThanOrEqual(1);
      precedent = v;
    }
  });

  it("is already most of the way at half time, and slows into the paper", () => {
    expect(adoucir(0.5)).toBeGreaterThan(0.8);
  });
});
