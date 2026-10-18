import { describe, expect, it } from "vitest";
import type { Album, Spread } from "./album";
import type { FormatPreset } from "./bridge";
import { setLangue } from "./i18n";
import {
  choixDeRecomposition,
  formatsDeRecomposition,
  phraseGardees,
  planchesGardees,
  srcsDeLAlbum,
} from "./recomposer";

const TOUS: FormatPreset[] = [
  { name: "carre-21", w: 210, h: 210, about: "" },
  { name: "carre-30", w: 300, h: 300, about: "" },
  { name: "portrait-a4", w: 210, h: 297, about: "" },
  { name: "paysage-a4", w: 297, h: 210, about: "" },
  { name: "paysage-28x21", w: 280, h: 210, about: "" },
  { name: "portrait-20x25", w: 203.2, h: 254, about: "" },
];
const IMPRIMABLES = ["carre-21", "carre-30", "portrait-a4", "paysage-a4", "portrait-20x25"];

const planche = (srcs: string[], drapeaux: Partial<Spread> = {}): Spread =>
  ({ template: "x", slots: srcs.map((src) => ({ src, focal: [0.5, 0.5] })), ...drapeaux }) as Spread;

const album = (spreads: Spread[], trim = { w: 210, h: 210 }, densite?: string): Album =>
  ({ version: 3, title: "t", root: "/p", trim_mm: trim, bleed_mm: 3, spreads, densite }) as Album;

describe("les formats du panneau Recomposer", () => {
  it("sont les cinq imprimables, le courant présélectionné", () => {
    const r = formatsDeRecomposition(TOUS, IMPRIMABLES, { w: 210, h: 210 });
    expect(r.formats.map((f) => f.name)).toEqual(IMPRIMABLES);
    expect(r.formats.every((f) => f.imprimable)).toBe(true);
    expect(r.courant).toBe("carre-21");
  });

  it("gardent un format non imprimable déjà dans l'album, marqué comme tel", () => {
    const r = formatsDeRecomposition(TOUS, IMPRIMABLES, { w: 280, h: 210 });
    expect(r.courant).toBe("paysage-28x21");
    const c = r.formats.find((f) => f.name === r.courant)!;
    expect(c.imprimable).toBe(false);
    expect(r.formats.filter((f) => f.imprimable)).toHaveLength(5);
  });

  it("gardent une taille libre, nommée par ses cotes", () => {
    const r = formatsDeRecomposition(TOUS, IMPRIMABLES, { w: 240, h: 180 });
    const c = r.formats.find((f) => f.name === r.courant)!;
    expect(c).toMatchObject({ w: 240, h: 180, imprimable: false });
    expect(r.formats).toHaveLength(6);
  });

  it("sans la liste du moteur, n'inventent rien : le courant seul", () => {
    const r = formatsDeRecomposition([], null, { w: 210, h: 210 });
    expect(r.formats).toHaveLength(1);
    expect(r.formats[0]).toMatchObject({ w: 210, h: 210 });
  });
});

describe("ce que Recomposer envoie au moteur", () => {
  const a = album([planche(["a.jpg"])]);

  it("rien quand rien ne change : la recomposition d'avant, au bit près", () => {
    expect(choixDeRecomposition(a, { w: 210, h: 210 }, false)).toEqual({});
  });

  it("le format quand il change", () => {
    expect(choixDeRecomposition(a, { w: 297, h: 210 }, false)).toEqual({
      format: { w: 297, h: 210 },
    });
  });

  it("le rythme quand la case change d'état", () => {
    expect(choixDeRecomposition(a, { w: 210, h: 210 }, true)).toEqual({ densite: "aeree" });
    const aeree = album([], { w: 210, h: 210 }, "aeree");
    expect(choixDeRecomposition(aeree, { w: 210, h: 210 }, true)).toEqual({});
    expect(choixDeRecomposition(aeree, { w: 210, h: 210 }, false)).toEqual({
      densite: "equilibree",
    });
  });

  it("un album dense que personne ne touche reste dense", () => {
    const dense = album([], { w: 210, h: 210 }, "dense");
    expect(choixDeRecomposition(dense, { w: 210, h: 210 }, false)).toEqual({});
  });
});

describe("les planches gardées", () => {
  it("sont les éditées et les verrouillées, comptées une fois", () => {
    const a = album([
      planche(["a.jpg"], { edited: true }),
      planche(["b.jpg"], { locked: true }),
      planche(["c.jpg"], { edited: true, locked: true }),
      planche(["d.jpg"]),
    ]);
    expect(planchesGardees(a)).toBe(3);
  });

  it("se disent en chiffre, et autrement quand le format change", () => {
    setLangue("fr");
    expect(phraseGardees(3, false)).toBe("3 planches que vous avez modifiées restent telles quelles.");
    expect(phraseGardees(1, false)).toBe("1 planche que vous avez modifiée reste telle quelle.");
    expect(phraseGardees(0, false)).toBe("");
    expect(phraseGardees(3, true)).toContain("3 planches que vous avez modifiées");
    expect(phraseGardees(3, true)).not.toContain("telles quelles");
    setLangue("en");
    expect(phraseGardees(3, false)).toBe("3 spreads you edited stay exactly as they are.");
    setLangue("fr");
  });
});

describe("les vignettes de la visionneuse", () => {
  it("viennent de tout le livre, une par planche, sans doublon", () => {
    const a = album([
      planche([]),
      ...Array.from({ length: 20 }, (_, i) => planche([`p${i}.jpg`, `q${i}.jpg`])),
      planche(["p0.jpg"]),
    ]);
    const srcs = srcsDeLAlbum(a, 8);
    expect(srcs).toHaveLength(8);
    expect(new Set(srcs).size).toBe(8);
    expect(srcs[0]).toBe("p0.jpg");
    expect(srcs.every((s) => s.startsWith("p"))).toBe(true);
  });

  it("un petit album donne ce qu'il a", () => {
    expect(srcsDeLAlbum(album([planche(["a.jpg", "b.jpg"])]), 8)).toEqual(["a.jpg", "b.jpg"]);
  });
});
