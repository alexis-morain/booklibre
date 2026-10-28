// La barre latérale dit où l'on est : la vue, le chapitre, la couverture.
// Elle ne lance rien d'elle-même, chaque bouton rend la main à App.

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type { Album } from "./album";
import { chapitreDe, chapitres } from "./chapitres";
import { setLangue } from "./i18n";
import { formatCm, Lateral } from "./Lateral";

afterEach(() => setLangue("en"));

const planche = (caption?: string) => ({ template: "vide", slots: [], caption });
const album = {
  title: "Corse",
  trim_mm: { w: 210, h: 210 },
  spreads: [planche(), planche("27 octobre"), planche(), planche("28 octobre")],
} as unknown as Album;

function rendu(
  over: Partial<Parameters<typeof Lateral>[0]> = {},
  index = 2,
  view: "livre" | "tri" | "planches" | "envoi" = "livre",
): string {
  const liste = chapitres(album);
  return renderToStaticMarkup(
    <Lateral
      album={album}
      view={view}
      triCount={12}
      index={index}
      onView={() => {}}
      onCouverture={() => {}}
      onChapitre={() => {}}
      chapitres={liste}
      courant={chapitreDe(liste, index)}
      onNouveau={() => {}}
      virgule
      {...over}
    />,
  ).replace(/&#x27;|&rsquo;/g, "’");
}

describe("la barre latérale", () => {
  it("nomme l'album, son format et ses planches", () => {
    setLangue("fr");
    const h = rendu();
    expect(h).toContain("Corse");
    expect(h).toContain("21 × 21 cm · 4 planches");
  });

  it("marque la vue courante, et seulement elle", () => {
    setLangue("fr");
    const h = rendu({}, 0, "planches");
    expect(h.match(/aria-current="page"/g)).toHaveLength(1);
    expect(h).toMatch(/aria-current="page"[^>]*>(?:(?!<\/button>).)*Planches/);
  });

  it("donne les quatre raccourcis de vue", () => {
    setLangue("fr");
    const h = rendu();
    for (const n of ["⌘1", "⌘2", "⌘3", "⌘4"]) expect(h).toContain(n);
  });

  it("compte les photos du tri et les planches", () => {
    setLangue("fr");
    const h = rendu();
    expect(h).toContain(">12<");
    expect(h).toContain(">4<");
  });

  it("surligne le chapitre de la planche affichée", () => {
    setLangue("fr");
    const h = rendu({}, 3);
    expect(h).toMatch(/aria-current="location"[^>]*>(?:(?!<\/button>).)*28 octobre/);
    expect(h).not.toMatch(/aria-current="location"[^>]*>(?:(?!<\/button>).)*27 octobre/);
  });

  it("n'a de chapitre courant ni dans Tri ni dans Envoi", () => {
    setLangue("fr");
    expect(rendu({}, 3, "tri")).not.toContain('aria-current="location"');
    expect(rendu({}, 3, "envoi")).not.toContain('aria-current="location"');
  });

  it("marque la couverture quand c'est elle qu'on voit", () => {
    setLangue("fr");
    expect(rendu({}, -1)).toMatch(/aria-current="location"[^>]*>(?:(?!<\/button>).)*Couverture/);
  });

  it("n'offre la police qu'à qui la passe, et Ouvrir qu'au harnais", () => {
    setLangue("fr");
    expect(rendu()).not.toContain("Police");
    expect(rendu({ onPolice: () => {} })).toContain("Police");
    expect(rendu()).not.toContain("Ouvrir un album");
    expect(rendu({ onOuvrir: () => {} })).toContain("Ouvrir un album");
  });
});

describe("le format en centimètres", () => {
  it("met la virgule en français et le point en anglais", () => {
    expect(formatCm({ w: 297, h: 210 }, true)).toBe("29,7 × 21 cm");
    expect(formatCm({ w: 297, h: 210 }, false)).toBe("29.7 × 21 cm");
  });
});
