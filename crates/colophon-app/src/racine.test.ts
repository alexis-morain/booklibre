// L'état du dossier de photos d'un album ouvert, tel que l'écran le dit.
// Trois états et pas deux : sous le bac à sable du Mac App Store, un dossier
// présent mais fermé à l'app (« à revoir ») n'appelle pas la même phrase
// qu'un dossier disparu, et c'est lui qui a un bouton.

import { afterEach, describe, expect, it } from "vitest";
import { etatRacine } from "./racine";
import { setLangue, t } from "./i18n";

const ouvert = (root_present: boolean, dossier_a_revoir: boolean) => ({
  root_present,
  dossier_a_revoir,
});

afterEach(() => setLangue("en"));

describe("l'état du dossier de photos", () => {
  it("est présent quand tout se lit", () => {
    expect(etatRacine(ouvert(true, false))).toBe("present");
  });

  it("est introuvable quand le dossier a disparu", () => {
    expect(etatRacine(ouvert(false, false))).toBe("introuvable");
  });

  it("est à revoir quand le bac à sable le ferme, qu'il existe ou non", () => {
    expect(etatRacine(ouvert(true, true))).toBe("a-revoir");
    expect(etatRacine(ouvert(false, true))).toBe("a-revoir");
  });

  it("le dit dans les deux langues, avec un bouton qui nomme le geste", () => {
    setLangue("fr");
    expect(t("racine.a.revoir")).toContain("Colophon a besoin de revoir le dossier de photos");
    expect(t("racine.a.revoir.bouton")).not.toBe("");
    setLangue("en");
    expect(t("racine.a.revoir")).toContain("photo folder");
    expect(t("racine.a.revoir.bouton")).not.toBe("");
  });
});
