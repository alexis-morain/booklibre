// Le bilan de composition dit les photos que le Composer a laissées dans la
// réserve faute de case qui les imprime à 250 ppi, et le chapitre qu'elles
// ont vidé (1.1-a2). Rien quand il n'y en a pas, ni quand le moteur est plus
// ancien que la fenêtre et ne porte pas le champ.

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type { Album } from "./album";
import { BilanView } from "./BilanView";
import type { BuildBilan } from "./bridge";
import { setLangue } from "./i18n";

afterEach(() => setLangue("en"));

const album = { title: "Corse", spreads: [] } as unknown as Album;

function rendu(bilan: BuildBilan): string {
  return renderToStaticMarkup(
    <BilanView
      bilan={bilan}
      album={album}
      curation={[]}
      variantes={[]}
      choisie={null}
      onChoisir={() => {}}
      onOpen={() => {}}
      onTri={() => {}}
    />,
  ).replace(/&#x27;|&rsquo;/g, "’");
}

const base: BuildBilan = { photos_scanned: 40, photos_kept: 30, chapters: 3 };

describe("les photos trop petites au bilan", () => {
  it("le dit au pluriel", () => {
    setLangue("fr");
    const h = rendu({ ...base, trop_petites: 2 });
    expect(h).toContain(
      "2 photos trop petites pour être bien imprimées sont restées dans la réserve. Vous pouvez les poser quand même.",
    );
  });

  it("le dit au singulier", () => {
    setLangue("fr");
    const h = rendu({ ...base, trop_petites: 1 });
    expect(h).toContain(
      "1 photo trop petite pour être bien imprimée est restée dans la réserve. Vous pouvez la poser quand même.",
    );
  });

  it("le dit en anglais", () => {
    const h = rendu({ ...base, trop_petites: 2 });
    expect(h).toContain("2 photos too small to print well");
  });

  it("se tait à zéro et quand le champ manque", () => {
    setLangue("fr");
    expect(rendu({ ...base, trop_petites: 0 })).not.toContain("trop petite");
    expect(rendu(base)).not.toContain("trop petite");
  });

  it("nomme le chapitre vidé", () => {
    setLangue("fr");
    const h = rendu({ ...base, trop_petites: 6, chapitres_vides: ["Bonifacio, 24 octobre"] });
    expect(h).toContain("Un chapitre n’a gardé aucune photo : Bonifacio, 24 octobre.");
  });
});
