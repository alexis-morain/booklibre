// Les deux dictionnaires doivent rester en face l'un de l'autre. Une clé
// ajoutée d'un côté et oubliée de l'autre ne se voit pas à l'écran tant que
// personne n'ouvre l'application dans l'autre langue, c'est-à-dire jamais
// pendant le développement.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { EN, FR, setLangue, t } from "./i18n";

describe("dictionnaires", () => {
  it("ont exactement les mêmes clés", () => {
    const fr = Object.keys(FR).sort();
    const en = Object.keys(EN).sort();
    expect(en).toEqual(fr);
  });

  it("n'ont aucune entrée vide", () => {
    for (const [cle, texte] of Object.entries({ ...FR, ...EN })) {
      expect(texte.trim(), cle).not.toBe("");
    }
  });

  /** Les trous d'un texte sont un contrat : `{poids}` traduit en `{weight}`
   *  laisse un trou béant à l'écran, et personne ne le voit avant la sortie. */
  it("ont les mêmes trous dans les mêmes clés", () => {
    const trous = (s: string) => (s.match(/\{(\w+)\}/g) ?? []).sort();
    for (const cle of Object.keys(FR) as (keyof typeof FR)[]) {
      expect(trous(EN[cle]), cle).toEqual(trous(FR[cle]));
    }
  });

  /** Zéro tiret cadratin, dans les deux langues : la règle vaut pour tout
   *  texte qui sort, et l'anglais est le premier endroit où il revient. */
  it("ne contiennent aucun tiret cadratin", () => {
    for (const [cle, texte] of Object.entries({ ...FR, ...EN })) {
      expect(texte.includes("—"), cle).toBe(false);
    }
  });

  /** Le logiciel s'appelle Booklibre (R-s1). Le mot « colophon » reste le nom
   *  typographique de la page de fin du livre, et seulement lui : une majuscule
   *  ne survit qu'en tête d'une phrase anglaise qui parle de cette page. */
  it("ne nomment plus le logiciel Colophon", () => {
    const survivants: Partial<Record<keyof typeof FR, string>> = {
      "etat.colophon.ajoute": "en : « Colophon page added », la page de fin",
      "etat.colophon.retire": "en : « Colophon page removed », la page de fin",
      "gabarit.colophon": "en : « Colophon page », le nom de la planche de fin",
    };
    for (const [langue, dico] of [["fr", FR], ["en", EN]] as const) {
      for (const [cle, texte] of Object.entries(dico)) {
        if (!/Colophon/.test(texte)) continue;
        const garde = langue === "en" && cle in survivants;
        expect(garde, `${langue} ${cle} : ${texte}`).toBe(true);
      }
    }
    for (const cle of Object.keys(survivants) as (keyof typeof FR)[]) {
      expect(EN[cle], cle).toMatch(/^Colophon page/);
    }
  });
});

describe("t", () => {
  it("remplit les trous et laisse les autres tels quels", () => {
    setLangue("fr");
    expect(t("stockage.supprime", { titre: "Corse", poids: "183 Mo" })).toBe(
      "« Corse » supprimé, 183 Mo libérés.",
    );
    expect(t("stockage.planches", {})).toBe("{n} planches");
  });

  it("suit la langue choisie", () => {
    setLangue("en");
    expect(t("bar.enregistrer")).toBe("Save");
    setLangue("fr");
    expect(t("bar.enregistrer")).toBe("Enregistrer");
  });
});

/** Le libellé de la case de l'acquittement (1.1-c) : le relais garde
 *  `version: 1`, et ce numéro désigne le texte archivé mot pour mot dans
 *  `docs/acquittement-v1.md`. Une retouche de l'un sans l'autre rougit ici :
 *  changer un mot, c'est une version 2. */
describe("l'acquittement, version 1", () => {
  it("dit mot pour mot le texte archivé, en français et en anglais", () => {
    const archive = readFileSync(new URL("../../../docs/acquittement-v1.md", import.meta.url), "utf8");
    const cites = [...archive.matchAll(/^> (.+)$/gm)].map((m) => m[1]);
    expect(cites).toEqual([
      FR["envoi.acquittement.un"],
      FR["envoi.acquittement"],
      EN["envoi.acquittement.un"],
      EN["envoi.acquittement"],
    ]);
  });
});
