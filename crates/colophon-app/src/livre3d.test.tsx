// Le livre en volume (A4) : ses cotes à l'échelle, ses angles, et ce qu'il
// dit à VoiceOver. Rendu sans navigateur, comme l'écran Envoi.

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type { Album, Spread } from "./album";
import { Livre3D } from "./Livre3D";
import {
  ANGLE_REPOS,
  EPAISSEUR_MIN_PX,
  PAS_FLECHE,
  VUES_FIXES,
  apresFleche,
  cotesLivre,
  repos,
  vueSuivante,
} from "./volume";
import { setLangue } from "./i18n";

afterEach(() => setLangue("en"));

const album = (fond?: string, back_text?: string) =>
  ({
    version: 3,
    title: "Corse",
    root: "/p",
    trim_mm: { w: 210, h: 210 },
    bleed_mm: 3,
    spreads: [{ template: "solo", slots: [] } as unknown as Spread],
    cover: { title: "Corse", ...(fond ? { fond } : {}), ...(back_text ? { back_text } : {}) },
  }) as unknown as Album;

describe("les cotes du livre", () => {
  it("garde les proportions du format et de l'épaisseur", () => {
    const c = cotesLivre({ wMm: 280, hMm: 210, dosMm: 12.48 }, 210);
    expect(c.hauteur).toBe(210);
    expect(c.largeur).toBeCloseTo(280, 9);
    expect(c.epaisseur).toBeCloseTo(12.48, 9);
  });

  it("un 24 pages et un 200 pages ne se ressemblent pas", () => {
    // Les dos de Cloudprinter à 150 g : 7,62 mm et 19,5 mm environ.
    const mince = cotesLivre({ wMm: 210, hMm: 210, dosMm: 7.62 }, 260);
    const epais = cotesLivre({ wMm: 210, hMm: 210, dosMm: 19.5 }, 260);
    expect(epais.epaisseur / mince.epaisseur).toBeCloseTo(19.5 / 7.62, 6);
    expect(mince.largeur).toBe(epais.largeur);
  });

  it("donne une épaisseur visible quand le dos n'est pas connu", () => {
    expect(cotesLivre({ wMm: 210, hMm: 210, dosMm: null }, 260).epaisseur).toBe(EPAISSEUR_MIN_PX);
    expect(cotesLivre({ wMm: 210, hMm: 210, dosMm: 0.1 }, 260).epaisseur).toBe(EPAISSEUR_MIN_PX);
  });
});

describe("les angles", () => {
  it("revient au trois-quarts le plus proche, sans refaire de tours", () => {
    expect(repos(0)).toBe(ANGLE_REPOS);
    expect(repos(170)).toBe(ANGLE_REPOS);
    expect(repos(400)).toBe(ANGLE_REPOS + 360);
    expect(repos(-300)).toBe(ANGLE_REPOS - 360);
  });

  it("trois vues fixes, l'avant, le trois-quarts et le dos, en cycle", () => {
    expect(VUES_FIXES).toEqual([0, ANGLE_REPOS, 90]);
    expect(vueSuivante(0)).toBe(ANGLE_REPOS);
    expect(vueSuivante(ANGLE_REPOS)).toBe(90);
    expect(vueSuivante(90)).toBe(0);
    // Un angle qui n'est pas une vue repart de la plus proche.
    expect(vueSuivante(33)).toBe(90);
  });

  it("les flèches tournent d'un pas, la gauche montre le dos", () => {
    expect(apresFleche(ANGLE_REPOS, "gauche")).toBe(ANGLE_REPOS + PAS_FLECHE);
    expect(apresFleche(ANGLE_REPOS, "droite")).toBe(ANGLE_REPOS - PAS_FLECHE);
  });
});

describe("Livre3D", () => {
  it("est une image nommée par son format, ses pages et sa couleur", () => {
    setLangue("fr");
    const html = renderToStaticMarkup(
      <Livre3D album={album("#1a1a1a")} formatMm={[210, 210]} dosMm={12.48} pages={96} hauteurPx={260} />,
    );
    expect(html).toContain('role="img"');
    expect(html).toContain('tabindex="0"');
    const label = html.match(/aria-label="([^"]+)"/)?.[1] ?? "";
    expect(label).toContain("21 × 21 cm");
    expect(label).toContain("96 pages");
    expect(label).toContain("fond noir");
  });

  it("pose six faces, le dos et l'arrière dans la couleur du fond", () => {
    const html = renderToStaticMarkup(
      <Livre3D album={album("#1a1a1a", "Pour Marie.")} formatMm={[210, 210]} dosMm={12.48} pages={96} hauteurPx={210} />,
    );
    for (const face of ["avant", "arriere", "dos", "tranche", "dessus", "dessous"]) {
      expect(html, face).toContain(`livre3d-${face}`);
    }
    expect(html).toMatch(/livre3d-dos"[^>]*background:#1a1a1a/);
    expect(html).toMatch(/livre3d-arriere"[^>]*background:#1a1a1a/);
    // Le dos à sa largeur réelle, à l'échelle : 12,48 mm sur 210 px de 210 mm.
    expect(html).toMatch(/livre3d-dos"[^>]*width:12\.48px/);
    // La quatrième, quand elle existe, sur l'arrière.
    expect(html).toContain("Pour Marie.");
  });

  it("sans fond, le blanc papier, et le nom le dit", () => {
    setLangue("fr");
    const html = renderToStaticMarkup(
      <Livre3D album={album()} formatMm={[210, 210]} dosMm={12.48} pages={96} hauteurPx={260} />,
    );
    expect(html.match(/aria-label="([^"]+)"/)?.[1]).toContain("fond blanc papier");
  });

  // 1.2-c : la visionneuse le montre fermé, avec l'avant du PDF, et le met de
  // face avant d'ouvrir son plat.
  it("porte l'avant qu'on lui donne à la place de la vignette", () => {
    const html = renderToStaticMarkup(
      <Livre3D
        album={album()}
        formatMm={[210, 210]}
        dosMm={12.48}
        pages={96}
        hauteurPx={260}
        avant={<canvas className="avant-du-pdf" />}
      />,
    );
    expect(html).toContain("avant-du-pdf");
    expect(html).not.toContain("mini-cover");
  });

  it("de face, sans angle ni inclinaison ; sinon au trois-quarts", () => {
    const deFace = renderToStaticMarkup(
      <Livre3D album={album()} formatMm={[210, 210]} dosMm={12.48} pages={96} hauteurPx={260} deFace />,
    );
    expect(deFace).toContain("rotateX(0deg) rotateY(0deg)");
    expect(deFace).toContain("de-face");
    const repos = renderToStaticMarkup(
      <Livre3D album={album()} formatMm={[210, 210]} dosMm={12.48} pages={96} hauteurPx={260} />,
    );
    expect(repos).toContain(`rotateY(${ANGLE_REPOS}deg)`);
    expect(repos).not.toContain("de-face");
  });
});
