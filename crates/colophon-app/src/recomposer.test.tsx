import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it } from "vitest";
import type { Album, Spread } from "./album";
import type { FormatPreset } from "./bridge";
import { setLangue } from "./i18n";
import { formatsDeRecomposition } from "./recomposer";
import { RecomposerView } from "./RecomposerView";

const TOUS: FormatPreset[] = [
  { name: "carre-21", w: 210, h: 210, about: "" },
  { name: "carre-30", w: 300, h: 300, about: "" },
  { name: "portrait-a4", w: 210, h: 297, about: "" },
  { name: "paysage-a4", w: 297, h: 210, about: "" },
  { name: "paysage-28x21", w: 280, h: 210, about: "" },
  { name: "portrait-20x25", w: 203.2, h: 254, about: "" },
];
const IMPRIMABLES = ["carre-21", "carre-30", "portrait-a4", "paysage-a4", "portrait-20x25"];

const album = (trim: { w: number; h: number }, densite?: Album["densite"]): Album =>
  ({
    version: 3,
    title: "Corse",
    root: "/p",
    trim_mm: trim,
    bleed_mm: 3,
    spreads: [{ template: "solo", slots: [], locked: true } as unknown as Spread],
    densite,
  }) as Album;

const rendu = (a: Album, gardees = 3) => {
  const { formats, courant } = formatsDeRecomposition(TOUS, IMPRIMABLES, a.trim_mm);
  return renderToStaticMarkup(
    <RecomposerView
      album={a}
      formats={formats}
      courant={courant}
      gardees={gardees}
      onRecomposer={() => {}}
      onSansRecomposer={() => {}}
      onClose={() => {}}
    />,
  );
};

/** Les cartes pressées, par leur nom affiché. */
const pressees = (html: string) =>
  [...html.matchAll(/aria-pressed="true"[^>]*>.*?<span class="format-name">([^<]+)</g)].map(
    (m) => m[1],
  );

describe("le panneau Recomposer", () => {
  beforeEach(() => setLangue("fr"));

  it("offre les cinq formats, le courant coché, et dit ce qu'il garde en chiffre", () => {
    const html = rendu(album({ w: 210, h: 210 }));
    expect(html.match(/class="format-card( active)?"/g)).toHaveLength(5);
    expect(pressees(html)).toEqual(["Carré 21 × 21"]);
    expect(html).toContain("3 planches que vous avez modifiées restent telles quelles.");
    expect(html).toContain("L’historique d’annulation repart de zéro.");
    expect(html).toContain("Recomposer l’album");
    expect(html).toContain("Changer de format sans recomposer");
    expect(html).not.toContain("non imprimable");
  });

  it("garde un 28 × 21 coché, avec sa mention", () => {
    const html = rendu(album({ w: 280, h: 210 }));
    expect(html.match(/class="format-card( active)?"/g)).toHaveLength(6);
    expect(pressees(html)).toEqual(["Paysage 28 × 21"]);
    expect(html).toContain("non imprimable par Booklibre");
  });

  it("coche « moins de photos » pour un album aéré, et pas pour un autre", () => {
    const coche = (html: string) => /type="checkbox" checked/.test(html);
    expect(coche(rendu(album({ w: 210, h: 210 }, "aeree")))).toBe(true);
    expect(coche(rendu(album({ w: 210, h: 210 })))).toBe(false);
    expect(rendu(album({ w: 210, h: 210 }))).toContain(
      "Moins de photos par page, chacune plus grande.",
    );
  });

  it("se tait sur les planches gardées quand il n'y en a pas", () => {
    expect(rendu(album({ w: 210, h: 210 }), 0)).not.toContain("modifiée");
  });
});
