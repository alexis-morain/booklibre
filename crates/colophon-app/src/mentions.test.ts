// Le réglage « sous les photos » côté fenêtre (1.2-b) : poser la table du
// moteur sur les cases, la garder juste à travers les éditions, et compter.

import { describe, expect, it } from "vitest";
import fixture from "./geometrie.fixture.json";
import { Album, Legendes, spreadGeometry } from "./album";
import { Dump, setGeometrie } from "./geometrie";
import { placePhoto } from "./edits";
import { avecMentions, bilanMentions, cleDe, poserMentions, TableMentions } from "./mentions";

setGeometrie(fixture as unknown as Dump);

const DL: Legendes = { contenu: "date_lieu", date: "jour", langue: "fr" };
const D: Legendes = { contenu: "date", date: "jour", langue: "fr" };

function album(): Album {
  return {
    version: 3,
    title: "t",
    root: "/p",
    trim_mm: fixture.trim_mm,
    bleed_mm: fixture.bleed_mm,
    spreads: [
      {
        template: "duo",
        slots: [
          { src: "a.jpg", focal: [0.5, 0.42] },
          { src: "b.jpg", focal: [0.5, 0.42], caption: "la plage" },
        ],
      },
      { template: "full1", slots: [{ src: "c.jpg", focal: [0.5, 0.42] }] },
    ],
  };
}

const table: TableMentions = {
  cle: cleDe(DL),
  textes: { "a.jpg": "Calvi, 10 mai 2020", "c.jpg": "Calvi", "r.jpg": "Ajaccio" },
};

describe("poserMentions", () => {
  it("pose le réglage et la table, sans marquer une planche", () => {
    const a = poserMentions(album(), DL, table);
    expect(a.legendes).toEqual(DL);
    expect(a.spreads[0].slots.map((s) => s.mention)).toEqual(["Calvi, 10 mai 2020", undefined]);
    expect(a.spreads[0].slots[1].caption).toBe("la plage");
    expect(a.spreads.some((s) => s.edited)).toBe(false);
  });

  it("Rien ôte tout et rend le fichier d'avant", () => {
    const avant = album();
    const pose = poserMentions(avant, DL, table);
    const rien = poserMentions(pose, null, null);
    expect(JSON.stringify(rien)).toBe(JSON.stringify(avant));
    expect("legendes" in rien).toBe(false);
  });

  it("rend l'album tel quel quand rien ne change", () => {
    const pose = poserMentions(album(), DL, table);
    expect(poserMentions(pose, DL, table)).toBe(pose);
    const a = album();
    expect(poserMentions(a, null, null)).toBe(a);
  });
});

describe("avecMentions", () => {
  it("donne sa mention à une photo qui entre, jamais celle d'une autre", () => {
    const pose = poserMentions(album(), DL, table);
    // Remplacer la photo de la case 0 par une de la réserve.
    const remplace = placePhoto(pose, 0, 0, { src: "r.jpg", focal: [0.5, 0.42] });
    expect(avecMentions(remplace, table).spreads[0].slots[0].mention).toBe("Ajaccio");
    // Une photo que la table ne connaît pas n'a rien sous elle.
    const inconnue = placePhoto(pose, 0, 0, { src: "x.jpg", focal: [0.5, 0.42], mention: "faux" });
    expect(avecMentions(inconnue, table).spreads[0].slots[0].mention).toBeUndefined();
  });

  it("ne touche pas un album d'un autre réglage, ni un album sans réglage", () => {
    const autre = poserMentions(album(), D, { cle: cleDe(D), textes: { "a.jpg": "10 mai 2020" } });
    expect(avecMentions(autre, table)).toBe(autre);
    const sans = album();
    expect(avecMentions(sans, table)).toBe(sans);
    expect(avecMentions(autre, null)).toBe(autre);
  });
});

describe("bilanMentions", () => {
  it("compte les photos légendées, celles qui ne savent rien, celles sans place", () => {
    const a = poserMentions(album(), DL, table);
    const g = spreadGeometry(a);
    // a : posée ; b : rien de connu ; c : sous une pleine page, sans place.
    expect(bilanMentions(a, g)).toEqual({ posees: 1, sansRien: 1, sansPlace: 1 });
  });
});
