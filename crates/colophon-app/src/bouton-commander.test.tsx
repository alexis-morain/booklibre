// « Commander » grisé au-delà de ce que le relais vend (décision d'Alexis,
// 15/11) : le bouton de la barre et la ligne du pied. La limite est passée par
// l'appelant, elle vient de `Tarifs.pages_max` : aucun test ne la recopie du code.

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import { BoutonCommander, ID_TROP_DE_PAGES } from "./BoutonCommander";
import { LignePrix } from "./ChoixImpression";
import { setLangue } from "./i18n";

afterEach(() => setLangue("en"));

const MAX = 200;
const bouton = (pages: number, limite: number | null = MAX) =>
  renderToStaticMarkup(<BoutonCommander pages={pages} limite={limite} onCommander={() => {}} />).replace(/&#x27;/g, "’");
const grise = (html: string) => /<button[^>]*disabled/.test(html);

describe("le bouton Commander de la barre", () => {
  it("488 pages : désactivé, et son message lui est relié", () => {
    setLangue("fr");
    const h = bouton(488);
    expect(grise(h)).toBe(true);
    expect(h).toContain(`aria-describedby="${ID_TROP_DE_PAGES}"`);
    expect(h).toContain(`id="${ID_TROP_DE_PAGES}"`);
    expect(h).toContain(
      "Ce livre fait 488 pages. La commande dans Booklibre va jusqu’à 200 pages : retirez des planches, ou exportez le PDF pour le faire imprimer ailleurs.",
    );
  });

  it("200 pages passe, 201 ne passe pas", () => {
    setLangue("fr");
    expect(grise(bouton(200))).toBe(false);
    expect(bouton(200)).not.toContain("aria-describedby");
    expect(bouton(200)).not.toContain(ID_TROP_DE_PAGES);
    expect(grise(bouton(201))).toBe(true);
  });

  it("202 puis 198 : le bouton se réactive sans rechargement", () => {
    setLangue("fr");
    expect(grise(bouton(202))).toBe(true);
    expect(grise(bouton(198))).toBe(false);
  });

  it("tant que la limite n'est pas lue, rien ne se grise", () => {
    setLangue("fr");
    expect(grise(bouton(488, null))).toBe(false);
  });

  it("le message existe en anglais", () => {
    setLangue("en");
    expect(bouton(488)).toContain("This book has 488 pages. Ordering in Booklibre goes up to 200 pages: remove some spreads");
  });
});

describe("la ligne du pied", () => {
  const ligne = (pages: number, prix: number | null) =>
    renderToStaticMarkup(<LignePrix pages={pages} prix={prix} limite={MAX} langue="fr" />).replace(/&#x27;/g, "’");

  it("dit la raison au-delà de la limite, le prix en deçà", () => {
    setLangue("fr");
    expect(ligne(488, null)).toContain("Ce livre fait 488 pages");
    expect(ligne(201, 4600)).toContain("Ce livre fait 201 pages");
    expect(ligne(201, 4600)).not.toContain("À partir de");
    expect(ligne(200, 4600)).toContain("À partir de");
    expect(ligne(200, null)).toBe("");
  });
});
