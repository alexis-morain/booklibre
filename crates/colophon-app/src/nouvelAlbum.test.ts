import { describe, expect, it } from "vitest";
import {
  arrondiDizaine,
  couvertureProposee,
  estimerLivre,
  formatsOfferts,
} from "./nouvelAlbum";

describe("les formats offerts", () => {
  const tous = [
    { name: "carre-21", w: 210, h: 210, about: "" },
    { name: "paysage-28x21", w: 280, h: 210, about: "" },
    { name: "carre-30", w: 300, h: 300, about: "" },
  ];

  it("ne gardent que les imprimables, dans l'ordre du moteur", () => {
    expect(formatsOfferts(tous, ["carre-30", "carre-21"]).map((f) => f.name)).toEqual([
      "carre-21",
      "carre-30",
    ]);
  });

  it("restent tous quand le moteur n'a pas encore répondu", () => {
    expect(formatsOfferts(tous, null)).toEqual(tous);
  });
});

describe("l'estimation du livre", () => {
  it("vise la densité du Composer sur les planches demandées", () => {
    // 48 × 3,2 = 153 photos, que 572 photos remplissent.
    expect(estimerLivre({ photos: 572, petit: false, planches: 48, parPlanche: 3.2 })).toEqual({
      photos: 150,
      planches: 48,
      pages: 96,
    });
  });

  it("ne promet jamais plus de photos que le dossier n'en a", () => {
    const e = estimerLivre({ photos: 100, petit: false, planches: 48, parPlanche: 3.2 });
    expect(e.photos).toBe(100);
    expect(e.planches).toBe(32);
    expect(e.pages).toBe(64);
  });

  it("prend tout un petit dossier, quelles que soient les planches demandées", () => {
    expect(estimerLivre({ photos: 13, petit: true, planches: 48, parPlanche: 3.2 })).toEqual({
      photos: 13,
      planches: 5,
      pages: 10,
    });
  });

  it("ne compte rien pour un dossier vide", () => {
    expect(estimerLivre({ photos: 0, petit: true, planches: 48, parPlanche: 3.2 })).toEqual({
      photos: 0,
      planches: 0,
      pages: 0,
    });
  });
});

describe("l'arrondi à la dizaine", () => {
  it("arrondit au plus proche, sauf sous vingt où le chiffre exact parle mieux", () => {
    expect(arrondiDizaine(153)).toBe(150);
    expect(arrondiDizaine(155)).toBe(160);
    expect(arrondiDizaine(24)).toBe(20);
    expect(arrondiDizaine(13)).toBe(13);
    expect(arrondiDizaine(0)).toBe(0);
  });
});

describe("la couverture proposée", () => {
  const v = (src: string, w: number, h: number) => ({ src, w, h, url: `data:${src}` });

  it("prend la première miniature paysage ou carrée", () => {
    expect(couvertureProposee([v("a", 120, 160), v("b", 160, 120), v("c", 160, 160)])?.src).toBe(
      "b",
    );
  });

  it("se rabat sur la première quand toutes sont en portrait", () => {
    expect(couvertureProposee([v("a", 120, 160), v("b", 90, 160)])?.src).toBe("a");
  });

  it("ne propose rien sans miniature", () => {
    expect(couvertureProposee([])).toBeNull();
  });
});
