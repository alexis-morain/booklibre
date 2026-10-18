import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it } from "vitest";
import fixture from "./geometrie.fixture.json";
import { setGeometrieFormat, type Dump } from "./geometrie";
import { setLangue } from "./i18n";
import { attribuer, echelleCommune, MAIN_MM, planchePx } from "./visionneuse";
import VisionneuseFormat from "./VisionneuseFormat";

const carre21 = { name: "carre-21", w: 210, h: 210, about: "" };
const carre30 = { name: "carre-30", w: 300, h: 300, about: "" };
const a4paysage = { name: "paysage-a4", w: 297, h: 210, about: "" };

describe("l'échelle de la visionneuse", () => {
  it("est la même pour tous les formats : le 30 × 30 est plus grand que le 21 × 21", () => {
    const s = echelleCommune(440);
    const petit = planchePx(carre21, s);
    const grand = planchePx(carre30, s);
    expect(grand.w / petit.w).toBeCloseTo(300 / 210, 6);
    expect(grand.h / petit.h).toBeCloseTo(300 / 210, 6);
    expect(petit.w).toBeCloseTo(2 * 210 * s, 6);
  });

  it("loge la plus grande planche et la main dans la boîte", () => {
    const s = echelleCommune(440);
    expect(planchePx(carre30, s).w + MAIN_MM.w * s).toBeLessThanOrEqual(440);
    expect(planchePx(a4paysage, s).w).toBeLessThanOrEqual(440);
  });
});

describe("l'attribution des vignettes aux cases", () => {
  const caseP = { x: 0, y: 0, w: 10, h: 20 };
  const caseL = { x: 0, y: 0, w: 20, h: 10 };

  it("met chaque vignette dans la case de forme la plus proche", () => {
    const v = [
      { url: "paysage", ratio: 1.5 },
      { url: "portrait", ratio: 0.66 },
    ];
    expect(attribuer([caseP, caseL], v).map((x) => x?.url)).toEqual(["portrait", "paysage"]);
  });

  it("laisse un aplat là où il n'y a plus de vignette", () => {
    expect(attribuer([caseP, caseL], [{ url: "seule" }])).toEqual([{ url: "seule" }, null]);
  });
});

describe("le composant", () => {
  beforeEach(() => setLangue("fr"));

  it("dit la cote ouverte et garnit d'aplats sans vignette, jamais un vide", () => {
    const html = renderToStaticMarkup(
      <VisionneuseFormat format={carre21} vignettes={[]} titre="Corse" />,
    );
    expect(html).toContain("42 cm ouvert");
    expect(html).toContain("visionneuse-aplat");
    expect(html).not.toContain("<img");
  });

  it("pose les vignettes données", () => {
    const html = renderToStaticMarkup(
      <VisionneuseFormat
        format={carre30}
        vignettes={[{ url: "data:image/jpeg;base64,AAAA", ratio: 1.33 }]}
        titre="Corse"
      />,
    );
    expect(html).toContain("60 cm ouvert");
    expect(html).toContain('src="data:image/jpeg;base64,AAAA"');
  });

  it("prend ses cases du moteur dès que le dump du format est là", () => {
    // Le dump de la fixture est un 21 × 21 ; seul le nombre de cases compte ici.
    setGeometrieFormat({
      ...fixture,
      bleed_mm: 0,
      media: { ...fixture.media, w: 420, h: 210 },
    } as unknown as Dump);
    const html = renderToStaticMarkup(
      <VisionneuseFormat
        format={carre21}
        vignettes={[{ url: "data:a", ratio: 1 }]}
        titre="Corse"
      />,
    );
    expect(html.match(/visionneuse-aplat/g)?.length).toBe(3);
    expect(html.match(/<img/g)?.length).toBe(1);
  });

  it("dessine le 30 × 30 plus grand que le 21 × 21", () => {
    const largeur = (html: string) =>
      Number(/visionneuse-planche" style="width:([\d.]+)px/.exec(html)?.[1]);
    const petit = renderToStaticMarkup(
      <VisionneuseFormat format={carre21} vignettes={[]} titre="" />,
    );
    const grand = renderToStaticMarkup(
      <VisionneuseFormat format={carre30} vignettes={[]} titre="" />,
    );
    expect(largeur(grand) / largeur(petit)).toBeCloseTo(300 / 210, 3);
  });
});
