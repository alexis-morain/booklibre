// Le menu contextuel décide sans fenêtre : son contenu par cible, sa place
// dans la fenêtre, le parcours au clavier.

import { describe, expect, it } from "vitest";
import { entreesPour, placerMenu, rangSuivant } from "./contextuel";

describe("entreesPour", () => {
  it("une photo : remplacer, retirer, copier, puis la fiche inactive tant que F n'est pas là", () => {
    const e = entreesPour({ type: "photo", cell: 1, src: "p.jpg" }, { fiche: false });
    expect(e.map((x) => x.id)).toEqual([
      "remplacer",
      "retirer",
      "copier",
      "voir-original",
      "informations",
    ]);
    expect(e[3].desactive).toBe("ctx.fiche.plus.tard");
    expect(e[4].desactive).toBe("ctx.fiche.plus.tard");
    expect(e.slice(0, 3).every((x) => x.desactive === undefined)).toBe(true);
  });

  it("la fiche arrivée, ses deux entrées s'activent sans changer de place", () => {
    const e = entreesPour({ type: "photo", cell: 1, src: "p.jpg" }, { fiche: true });
    expect(e.map((x) => x.id)).toHaveLength(5);
    expect(e.every((x) => x.desactive === undefined)).toBe(true);
  });

  it("le papier nu : ajouter une photo, bloc, ornement", () => {
    const e = entreesPour(
      { type: "page", droite: false, point: { x: 10, y: 10 } },
      { fiche: false },
    );
    expect(e.map((x) => x.id)).toEqual(["ajouter", "bloc", "ornement"]);
  });

  it("un objet libre : copier, couper, dupliquer, réglages, puis supprimer seul au bout", () => {
    const e = entreesPour({ type: "objet", index: 0 }, { fiche: false });
    expect(e.map((x) => x.id)).toEqual([
      "copier",
      "couper",
      "dupliquer",
      "reglages",
      "supprimer",
    ]);
  });

  it("une planche : dupliquer, insérer avant et après, recomposer, mise en page, supprimer", () => {
    const e = entreesPour(
      { type: "planche", index: 2, origine: true, photos: true },
      { fiche: true },
    );
    expect(e.map((x) => x.id)).toEqual([
      "dupliquer",
      "inserer-avant",
      "inserer-apres",
      "recomposer-planche",
      "mise-en-page",
      "supprimer",
    ]);
    expect(e.every((x) => x.desactive === undefined)).toBe(true);
  });

  it("Supprimer est seul de son groupe et le seul en rouge, sur l'objet comme sur la planche", () => {
    for (const cible of [
      { type: "objet", index: 0 } as const,
      { type: "planche", index: 1, origine: true, photos: true } as const,
    ]) {
      const e = entreesPour(cible, { fiche: true });
      const rouges = e.filter((x) => x.danger);
      expect(rouges.map((x) => x.id)).toEqual(["supprimer"]);
      expect(rouges[0].filet).toBe(true);
      expect(e[e.length - 1].id).toBe("supprimer");
    }
  });

  it("une planche insérée à la main ne se recompose pas, et le dit", () => {
    const e = entreesPour(
      { type: "planche", index: 0, origine: false, photos: true },
      { fiche: true },
    );
    expect(e.find((x) => x.id === "recomposer-planche")?.desactive).toBe("ctx.planche.sans.origine");
  });

  it("tant que l'origine n'est pas connue, Recomposer reste active", () => {
    const e = entreesPour({ type: "planche", index: 0, photos: true }, { fiche: true });
    expect(e.find((x) => x.id === "recomposer-planche")?.desactive).toBeUndefined();
  });

  it("une planche sans photo n'a pas de mise en page à changer", () => {
    const e = entreesPour(
      { type: "planche", index: 0, origine: true, photos: false },
      { fiche: true },
    );
    expect(e.find((x) => x.id === "mise-en-page")?.desactive).toBe("ctx.planche.sans.photo");
    expect(e.filter((x) => x.desactive).map((x) => x.id)).toEqual(["mise-en-page"]);
  });
});

describe("placerMenu", () => {
  const fenetre = { w: 900, h: 600 };
  const taille = { w: 200, h: 120 };

  it("s'ouvre au pointeur", () => {
    expect(placerMenu({ x: 100, y: 100 }, taille, fenetre)).toEqual({ left: 100, top: 100 });
  });

  it("se rabat sur le bord droit et sur le bas", () => {
    expect(placerMenu({ x: 850, y: 550 }, taille, fenetre)).toEqual({ left: 692, top: 472 });
  });

  it("ne sort jamais par le haut ni par la gauche", () => {
    expect(placerMenu({ x: -20, y: -20 }, taille, fenetre)).toEqual({ left: 8, top: 8 });
  });
});

describe("rangSuivant", () => {
  const libelles = ["Remplacer par…", "Retirer", "Copier", "Voir l'original", "Informations…"];
  const actives = [true, true, true, false, false];

  it("descend et remonte en boucle, en sautant les inactives", () => {
    expect(rangSuivant(null, "ArrowDown", libelles, actives)).toBe(0);
    expect(rangSuivant(2, "ArrowDown", libelles, actives)).toBe(0);
    expect(rangSuivant(0, "ArrowUp", libelles, actives)).toBe(2);
    expect(rangSuivant(null, "ArrowUp", libelles, actives)).toBe(2);
  });

  it("Début et Fin vont aux deux bouts actifs", () => {
    expect(rangSuivant(1, "Home", libelles, actives)).toBe(0);
    expect(rangSuivant(1, "End", libelles, actives)).toBe(2);
  });

  it("une lettre va au prochain libellé qui commence par elle, puis reboucle", () => {
    expect(rangSuivant(null, "r", libelles, actives)).toBe(0);
    expect(rangSuivant(0, "r", libelles, actives)).toBe(1);
    expect(rangSuivant(1, "R", libelles, actives)).toBe(0);
    expect(rangSuivant(null, "v", libelles, actives)).toBeNull();
  });

  it("ignore ce qui ne navigue pas", () => {
    expect(rangSuivant(1, "Enter", libelles, actives)).toBeNull();
    expect(rangSuivant(1, " ", libelles, actives)).toBeNull();
    expect(rangSuivant(null, "ArrowDown", [], [])).toBeNull();
  });

  it("une liste toute inactive ne rend rien", () => {
    expect(rangSuivant(null, "ArrowDown", ["a", "b"], [false, false])).toBeNull();
  });
});
