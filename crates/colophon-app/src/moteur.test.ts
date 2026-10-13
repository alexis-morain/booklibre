// L'anglais de ce que le moteur dit en français. La liste des codes vient du
// moteur (`moteur.codes.json`, tenu frais par `codes.rs`) : chaque code a sa
// phrase, chaque trou a sa valeur, et l'écran français ne change pas d'un mot.

import { afterEach, describe, expect, it } from "vitest";
import type { Defaut, VarianteResume } from "./bridge";
import { setLangue } from "./i18n";
import codes from "./moteur.codes.json";
import {
  DENSITES_EN,
  FORMATS_EN,
  PHRASES_EN,
  PROPOSITIONS_EN,
  TEXTES_EN,
  aboutFormat,
  aboutProposition,
  nomDensite,
  nomProposition,
  phraseDefaut,
  texteMoteur,
  trous,
} from "./moteur";

afterEach(() => setLangue("en"));

const phrases = codes.phrases as Record<string, string[]>;

describe("la liste du moteur", () => {
  it("chaque phrase du prévol a son anglais, et rien de plus", () => {
    expect(Object.keys(PHRASES_EN).sort()).toEqual(Object.keys(phrases).sort());
  });

  it("chaque trou d'une phrase anglaise est une valeur que le moteur donne", () => {
    for (const [code, modele] of Object.entries(PHRASES_EN)) {
      for (const trou of trous(modele)) {
        expect(phrases[code], `${code} : {${trou}}`).toContain(trou);
      }
    }
  });

  it("chaque note et chaque réserve a son anglais, à son français exact", () => {
    const francais = [...codes.notes, ...Object.values(codes.reserves).flat()];
    expect(Object.keys(TEXTES_EN).sort()).toEqual([...francais].sort());
    setLangue("fr");
    expect(texteMoteur(codes.notes[1])).toBe(codes.notes[1]);
    setLangue("en");
    expect(texteMoteur(codes.notes[1])).toBe(
      "photo folder not found: the effective resolution could not be checked",
    );
    expect(texteMoteur("une réserve qu'aucune table ne connaît")).toBe(
      "une réserve qu'aucune table ne connaît",
    );
  });

  it("chaque format, rythme et proposition a son anglais", () => {
    expect(Object.keys(FORMATS_EN).sort()).toEqual([...codes.formats].sort());
    expect(Object.keys(DENSITES_EN).sort()).toEqual([...codes.densites].sort());
    for (const id of codes.propositions) {
      if (id !== "autre-rythme") expect(PROPOSITIONS_EN[id], id).toBeDefined();
    }
  });
});

const resolution: Defaut = {
  regle: "resolution",
  bloquant: true,
  planche: 4,
  case: 1,
  src: "IMG_1.jpg",
  cause: "IMG_1.jpg imprimerait à 212 ppi dans cette case, Cloudprinter exige 250",
  remede: "mettez-la dans une case plus petite, remplacez-la, ou retirez-la",
  phrase: {
    cause: "resolution",
    remede: "resolution.case",
    valeurs: { src: "IMG_1.jpg", ppi: "212", imprimeur: "Cloudprinter", min_ppi: "250" },
  },
};

describe("un défaut du prévol", () => {
  it("l'écran français montre la phrase du moteur, telle quelle", () => {
    setLangue("fr");
    expect(phraseDefaut(resolution, "cause")).toBe(resolution.cause);
    expect(phraseDefaut(resolution, "remede")).toBe(resolution.remede);
  });

  it("l'écran anglais l'écrit depuis les codes", () => {
    setLangue("en");
    expect(phraseDefaut(resolution, "cause")).toBe(
      "IMG_1.jpg would print at 212 ppi in this cell, Cloudprinter requires 250",
    );
    expect(phraseDefaut(resolution, "remede")).toBe(
      "put it in a smaller cell, replace it, or remove it",
    );
  });

  it("un code inconnu ou une valeur manquante rend le français, jamais un trou", () => {
    setLangue("en");
    const inconnu = { ...resolution, phrase: { ...resolution.phrase!, cause: "regle_future" } };
    expect(phraseDefaut(inconnu, "cause")).toBe(resolution.cause);
    const sansValeur = { ...resolution, phrase: { ...resolution.phrase!, valeurs: {} } };
    expect(phraseDefaut(sansValeur, "cause")).toBe(resolution.cause);
    expect(phraseDefaut(sansValeur, "remede")).toBe("put it in a smaller cell, replace it, or remove it");
    const ancien: Defaut = { ...resolution, phrase: undefined };
    expect(phraseDefaut(ancien, "cause")).toBe(resolution.cause);
  });
});

describe("formats, rythmes et propositions", () => {
  const autre: VarianteResume = {
    id: "autre-rythme",
    nom: "Aérée",
    about: "Une ou deux photos par double page…",
    densite: "aeree",
    planches: 20,
    photos: 60,
    apercu: [],
  };

  it("le français reste celui du moteur", () => {
    setLangue("fr");
    expect(aboutFormat({ name: "carre-21", about: "carré 21 × 21, le format d'album courant" })).toBe(
      "carré 21 × 21, le format d'album courant",
    );
    expect(nomProposition(autre)).toBe("Aérée");
    expect(nomDensite({ id: "aeree", nom: "Aérée", about: "", photos_par_planche: 2 })).toBe("Aérée");
  });

  it("l'anglais suit l'identifiant, et l'autre rythme suit son rythme", () => {
    setLangue("en");
    expect(aboutFormat({ name: "portrait-20x25", about: "x" })).toBe("portrait 20 × 25, close to 8 × 10 inches");
    expect(nomProposition(autre)).toBe("Airy");
    expect(aboutProposition(autre)).toBe(DENSITES_EN.aeree.about);
    expect(nomProposition({ ...autre, densite: "equilibree" })).toBe("Balanced");
    expect(nomProposition({ ...autre, id: "resserree" })).toBe("Shorter");
    // Un moteur qui ne dit pas le rythme garde son français.
    expect(nomProposition({ ...autre, densite: undefined })).toBe("Aérée");
  });
});
