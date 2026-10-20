// « Remplacer par… » : une candidate trop petite pour la case porte le signe
// de la case et sa phrase à la place de sa date ; une qui tient garde sa
// raison d'aujourd'hui (1.1-a, décision 4).

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type { Candidat } from "./bridge";
import { ChoixReserve } from "./ChoixReserve";
import { setLangue, t } from "./i18n";
import { alerteDeCandidat } from "./reserve";

afterEach(() => setLangue("en"));

const candidat = (src: string, ppi?: number): Candidat => ({
  src,
  score: 1,
  raison: "meme_jour",
  focal: [0.5, 0.42],
  ...(ppi === undefined ? {} : { ppi }),
});

describe("alerteDeCandidat", () => {
  it("ne dit rien sans ppi, ni au-dessus du plancher", () => {
    expect(alerteDeCandidat(candidat("a.jpg"))).toBeNull();
    expect(alerteDeCandidat(candidat("a.jpg", 250))).toBeNull();
  });
  it("dit la résolution sous le plancher, arrondie comme sur une case", () => {
    expect(alerteDeCandidat(candidat("a.jpg", 249.6))).toEqual({
      code: "sous_resolution",
      ppi: 250,
    });
  });
});

describe("ChoixReserve", () => {
  it("met le signe à la place de la date pour celle qui ne tient pas", () => {
    setLangue("fr");
    const html = renderToStaticMarkup(
      <ChoixReserve
        point={{ x: 0, y: 0 }}
        reserve={{
          candidats: [candidat("grande.jpg", 400), candidat("petite.jpg", 120)],
        }}
        pleine={null}
        onChoisir={() => {}}
        onInsererApres={() => {}}
        onFermer={() => {}}
      />,
    );
    const [grande, petite] = html.split('role="option"').slice(1);
    expect(grande).toContain(t("reserve.raison.meme_jour"));
    expect(grande).not.toContain("alerte-signe");
    expect(petite).toContain("alerte-signe grave");
    // La phrase de la bulle, dans le nom de l'option que la voix lit.
    expect(petite).toContain(`aria-label="${t("alerte.phrase.sous_resolution").slice(0, 20)}`);
    expect(petite).not.toContain(`>${t("reserve.raison.meme_jour")}<`);
    // La ligne du texte de bulle est réservée ; la grande, visée d'abord,
    // n'y écrit rien.
    expect(html).toContain('class="choix-alerte"');
    expect(html.split('class="choix-alerte"')[1]).not.toContain("Trop petite");
  });
});
