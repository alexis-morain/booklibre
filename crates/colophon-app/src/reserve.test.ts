// Choisir une candidate de la réserve change l'album par `addPhoto` ou
// `placePhoto`, et rien d'autre ; une planche pleine se dit avant le choix
// et s'ouvre sur la planche d'après.

import { describe, expect, it } from "vitest";
import { Album, Slot, Spread, templateCapacity } from "./album";
import { Candidat } from "./bridge";
import fixture from "./geometrie.fixture.json";
import { Dump, setGeometrie } from "./geometrie";
import {
  appliquerRemplissage,
  effetDeDepot,
  messageRemplissage,
  pleineAvant,
  poser,
  poserApres,
  TYPE_PHOTO,
} from "./reserve";
import { setLangue } from "./i18n";

setGeometrie(fixture as unknown as Dump);

function slot(n: number): Slot {
  return { src: `p${n}.jpg`, focal: [0.5, 0.5] };
}

function spread(template: string, count: number): Spread {
  return { template, slots: Array.from({ length: count }, (_, i) => slot(i)) };
}

function album(...spreads: Spread[]): Album {
  return {
    version: 1,
    title: "test",
    root: "/tmp",
    trim_mm: { w: 210, h: 210 },
    bleed_mm: 3,
    spreads,
  };
}

const candidate: Candidat = {
  src: "reserve.jpg",
  score: 6.2,
  taken: "2013-10-27T17:43:10",
  raison: "meme_jour",
  focal: [0.3, 0.2],
};

describe("poser", () => {
  it("ajouter fait grandir la planche, le focal du moteur avec", () => {
    const a = album(spread("duo", 2));
    const r = poser(a, 0, { type: "ajouter" }, candidate);
    if (r.type !== "posee") throw new Error(r.type);
    expect(r.at).toBe(0);
    expect(r.album.spreads[0].template).toBe("trio");
    expect(r.album.spreads[0].slots[2]).toEqual({ src: "reserve.jpg", focal: [0.3, 0.2] });
    expect(r.album.spreads[0].slots.length).toBe(templateCapacity("trio"));
  });

  it("remplacer est placePhoto : la case change, la planche ne grandit pas", () => {
    const a = album(spread("duo", 2));
    const r = poser(a, 0, { type: "remplacer", cell: 1 }, candidate);
    if (r.type !== "remplacee") throw new Error(r.type);
    expect(r.album.spreads[0].template).toBe("duo");
    expect(r.album.spreads[0].slots[1].src).toBe("reserve.jpg");
    expect(r.album.spreads[0].slots[0].src).toBe("p0.jpg");
  });

  it("une planche pleine ne bouge pas et le dit", () => {
    const a = album(spread("quad", 4));
    expect(poser(a, 0, { type: "ajouter" }, candidate)).toEqual({
      type: "pleine",
      raison: "target_full",
    });
    expect(pleineAvant(a, 0)).toBe("target_full");
  });

  it("une page de texte refuse une photo, avant même le choix", () => {
    const a = album({ ...spread("texte", 0), text: "un mot" });
    expect(pleineAvant(a, 0)).toBe("target_text");
    expect(poser(a, 0, { type: "ajouter" }, candidate).type).toBe("pleine");
  });

  it("une planche qui a de la place ne se dit pas pleine", () => {
    expect(pleineAvant(album(spread("duo", 2)), 0)).toBeNull();
  });

  it("remplacer par une photo déjà sur la planche se refuse", () => {
    const a = album(spread("duo", 2));
    const deja = { ...candidate, src: "p0.jpg" };
    expect(poser(a, 0, { type: "remplacer", cell: 1 }, deja)).toEqual({ type: "refus" });
  });
});

describe("poserApres", () => {
  it("insère une planche vide après et y pose la candidate, seule", () => {
    const a = album(spread("quad", 4), spread("duo", 2));
    const r = poserApres(a, 0, candidate);
    expect(r.at).toBe(1);
    expect(r.album.spreads).toHaveLength(3);
    expect(r.album.spreads[1].template).toBe("solo");
    expect(r.album.spreads[1].slots).toEqual([{ src: "reserve.jpg", focal: [0.3, 0.2] }]);
    expect(r.album.spreads[2]).toBe(a.spreads[1]);
  });
});

describe("effetDeDepot", () => {
  // Le tiroir n'autorise que la copie : une case qui annonce « move » à une
  // photo du tiroir fait annuler le dépôt par le navigateur, `drop` ne part
  // jamais (WebKit comme Chromium).
  it("rend copy pour une photo du tiroir", () => {
    expect(effetDeDepot([TYPE_PHOTO])).toBe("copy");
    expect(effetDeDepot(["text/plain", TYPE_PHOTO])).toBe("copy");
  });

  it("garde move pour l'échange de deux cases", () => {
    expect(effetDeDepot(["text/colophon-slot"])).toBe("move");
    expect(effetDeDepot([])).toBe("move");
  });
});

describe("appliquerRemplissage", () => {
  const pose = (planche: number, cellule: number, src: string, gabarit?: string) => ({
    planche,
    cellule,
    src,
    focal: [0.4, 0.3] as [number, number],
    ppi: 300,
    ...(gabarit ? { gabarit } : {}),
  });

  it("une planche vide prend le gabarit du moteur et sa photo", () => {
    const a = album(spread("duo", 2), spread("vide", 0));
    const r = appliquerRemplissage(a, [pose(1, 0, "r.jpg", "solo_paysage")]);
    expect(r.posees).toBe(1);
    expect(r.album.spreads[1]).toEqual({
      template: "solo_paysage",
      slots: [{ src: "r.jpg", focal: [0.4, 0.3] }],
      edited: true,
    });
    expect(r.album.spreads[0]).toBe(a.spreads[0]);
  });

  it("une case sans photo se remplit au bout, le gabarit ne bouge pas", () => {
    const a = album(spread("quad", 2));
    const r = appliquerRemplissage(a, [pose(0, 2, "x.jpg"), pose(0, 3, "y.jpg")]);
    expect(r.posees).toBe(2);
    expect(r.album.spreads[0].template).toBe("quad");
    expect(r.album.spreads[0].slots.map((s) => s.src)).toEqual(["p0.jpg", "p1.jpg", "x.jpg", "y.jpg"]);
  });

  it("une pose périmée est sautée : case pleine, photo déjà là, planche figée ou changée", () => {
    const figee = { ...spread("vide", 0), locked: true };
    const texte = { ...spread("texte", 0), text: "Bonjour" };
    const a = album(spread("duo", 2), spread("vide", 0), figee, texte, spread("trio", 2));
    const r = appliquerRemplissage(a, [
      pose(0, 1, "a.jpg"), // la case 1 du duo porte déjà une photo
      pose(1, 0, "p0.jpg", "solo"), // déjà dans l'album
      pose(2, 0, "b.jpg", "solo"), // figée entre-temps
      pose(3, 0, "c.jpg", "solo"), // devenue une page de texte
      pose(4, 2, "d.jpg", "solo"), // un gabarit annoncé sur une planche qui n'est pas vide
      pose(9, 0, "e.jpg"), // hors de l'album
    ]);
    expect(r.posees).toBe(0);
    expect(r.album).toBe(a);
  });

  it("deux poses de la même photo n'en posent qu'une", () => {
    const a = album(spread("vide", 0), spread("vide", 0));
    const r = appliquerRemplissage(a, [pose(0, 0, "r.jpg", "solo"), pose(1, 0, "r.jpg", "solo")]);
    expect(r.posees).toBe(1);
    expect(r.album.spreads[1].slots).toEqual([]);
  });
});

describe("messageRemplissage", () => {
  it("dit ce qui a été rempli, ce qui reste, ou qu'il n'y avait rien", () => {
    setLangue("fr");
    expect(messageRemplissage(0, 0)).toBe("Aucune case vide dans l’album");
    expect(messageRemplissage(3, 0)).toBe("Cases vides remplies : 3");
    expect(messageRemplissage(2, 1)).toBe(
      "Cases vides remplies : 2. Restées vides : 1 (aucune photo de la réserve ne tient la case)",
    );
    expect(messageRemplissage(0, 2)).toBe(
      "Aucune case remplie. Restées vides : 2 (aucune photo de la réserve ne tient la case)",
    );
    setLangue("en");
    expect(messageRemplissage(3, 0)).toBe("Empty cells filled: 3");
  });
});
