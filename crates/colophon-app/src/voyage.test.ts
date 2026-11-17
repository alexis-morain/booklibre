// L'import d'un voyage, côté écran : la machine d'états, les phrases d'erreur,
// les lignes du bilan, et l'absence du point d'entrée dans la version Mac App
// Store. Aucun réseau, aucune fenêtre.

import { afterEach, describe, expect, it, vi } from "vitest";
import { EN, FR } from "./i18n";
import {
  BilanComposition,
  cleProgres,
  ETAT_INITIAL,

  GenreErreur,
  lignesBilanVoyage,
  normaliserErreur,
  phrasesDe,
  RapportVoyage,
  reduire,
} from "./voyage";

afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  vi.resetModules();
});

const LIEN = { id: 11306874, dossier: "/Users/x/Pictures/Colophon/Voyage 11306874" };
const COLLE = reduire(ETAT_INITIAL, { type: "url", url: "https://www.polarsteps.com/q/11306874-v", lien: LIEN });

describe("la machine d'états", () => {
  it("ne lance rien sans lien compris, et dit le lien illisible", () => {
    const a = reduire(ETAT_INITIAL, { type: "url", url: "pas un lien", lien: null });
    expect(a.invalide).toBe(true);
    expect(reduire(a, { type: "lancer" }).phase).toBe("saisie");
    // Un champ vide n'est pas une faute.
    expect(reduire(ETAT_INITIAL, { type: "url", url: "  ", lien: null }).invalide).toBe(false);
  });

  it("suit la progression : lecture, puis 412 / 1 546", () => {
    const en = reduire(COLLE, { type: "lancer" });
    expect(en.phase).toBe("cours");
    expect(cleProgres(en).cle).toBe("voyage.lecture");
    const p = reduire(en, { type: "progres", fait: 412, total: 1546 });
    expect(cleProgres(p)).toEqual({ cle: "voyage.encours", params: { fait: 412, total: 1546 } });
    // Une progression qui arrive après l'arrêt ne relance pas l'écran.
    const arrete = reduire(p, { type: "echec", erreur: { genre: "annule", detail: "" } });
    expect(reduire(arrete, { type: "progres", fait: 413, total: 1546 })).toEqual(arrete);
  });

  it("l'annulation n'est pas une erreur : elle se reprend", () => {
    const en = reduire(COLLE, { type: "lancer" });
    const a = reduire(en, { type: "echec", erreur: { genre: "annule", detail: "x" } });
    expect(a.phase).toBe("annule");
    expect(phrasesDe("annule")).toBeNull();
    // Le lien reste : « relancer » repart du même dossier.
    expect(a.lien).toEqual(LIEN);
    expect(reduire(a, { type: "lancer" }).phase).toBe("cours");
  });

  it("une erreur garde le lien et se relance", () => {
    const en = reduire(COLLE, { type: "lancer" });
    const e = reduire(en, { type: "echec", erreur: { genre: "format", detail: "d" } });
    expect(e.phase).toBe("echec");
    expect(e.lien).toEqual(LIEN);
    expect(reduire(e, { type: "lancer" }).erreur).toBeNull();
  });

  it("ne lance pas deux fois", () => {
    const en = reduire(COLLE, { type: "lancer" });
    expect(reduire(en, { type: "lancer" })).toBe(en);
  });
});

describe("les phrases d'erreur", () => {
  const tous: GenreErreur[] = [
    "url", "prive", "format", "reseau", "indisponible", "disque_plein", "disque",
    "trop_gros", "photo", "statut", "dossier", "inconnu",
  ];

  it("chaque genre a sa phrase et son remède, dans les deux langues", () => {
    for (const g of tous) {
      const p = phrasesDe(g);
      expect(p, g).not.toBeNull();
      for (const cle of [p!.phrase, p!.remede]) {
        expect(FR[cle], cle).toBeTruthy();
        expect(EN[cle], cle).toBeTruthy();
      }
    }
  });

  it("nomme ce que la personne peut faire", () => {
    expect(FR[phrasesDe("format")!.remede]).toContain("export");
    expect(FR[phrasesDe("prive")!.remede]).toContain("?s=");
    expect(FR[phrasesDe("reseau")!.remede]).toContain("reprend");
    expect(FR[phrasesDe("disque_plein")!.remede]).toContain("Libérez");
    expect(FR[phrasesDe("format")!.phrase]).toContain("Polarsteps a changé son format");
  });

  it("remet en genre ce que Tauri rejette", () => {
    expect(normaliserErreur({ genre: "prive", detail: "d" })).toEqual({ genre: "prive", detail: "d" });
    expect(normaliserErreur({ genre: "inventé", detail: "d" }).genre).toBe("inconnu");
    expect(normaliserErreur("boum")).toEqual({ genre: "inconnu", detail: "boum" });
    expect(normaliserErreur(new Error("x")).detail).toBe("x");
  });
});

const compo = (o: Partial<BilanComposition> = {}): BilanComposition => ({
  titre: "t", etapes: 93, etapes_groupees: 0, recits_debordants: [], emojis_non_imprimes: 0, refus: null, ...o,
});
const importe = (o: Partial<RapportVoyage["bilan"]> = {}): RapportVoyage => ({
  dossier: "/d",
  titre: "t",
  bilan: { etapes: 93, etapes_sans_photo: 2, pays: 12, photos: 1546, videos_ignorees: 0, photos_sous_seuil: 0, trace_presente: true, ...o },
});

describe("le bilan d'un voyage", () => {
  it("n'écrit que ce qui est non nul", () => {
    const l = lignesBilanVoyage(compo(), importe());
    expect(l.map((x) => x.cle)).toEqual(["bilan.voyage.resume"]);
  });

  it("nomme vidéos, émojis, récits débordants, étapes regroupées", () => {
    const l = lignesBilanVoyage(
      compo({ emojis_non_imprimes: 12, recits_debordants: ["Hanoï", "Sapa"], etapes_groupees: 4 }),
      importe({ videos_ignorees: 3, photos_sous_seuil: 1546, trace_presente: false }),
    );
    const cles = l.map((x) => x.cle);
    expect(cles).toEqual([
      "bilan.voyage.resume",
      "bilan.voyage.videos",
      "bilan.voyage.emojis",
      "bilan.voyage.debordants",
      "bilan.voyage.groupees",
      "bilan.voyage.trace",
    ]);
    expect(l.find((x) => x.cle === "bilan.voyage.debordants")!.params).toEqual({ noms: "Hanoï, Sapa" });
    // Le repère de pixels n'est jamais dit en ppi : seule la réserve parle de définition.
    expect(cles.join()).not.toContain("ppi");
    expect(FR["bilan.trop_petites.lien"]).toMatch(/vignettes.*export de vos données Polarsteps.*photos d’origine/);
    expect(EN["bilan.trop_petites.lien"]).toMatch(/thumbnails.*export of your Polarsteps data.*original photos/);
  });

  it("le singulier a sa phrase, et un voyage refusé le dit", () => {
    const l = lignesBilanVoyage(
      compo({ emojis_non_imprimes: 1, refus: "voyage.json illisible" }),
      importe({ videos_ignorees: 1, photos_sous_seuil: 1 }),
    );
    const cles = l.map((x) => x.cle);
    expect(cles).toContain("bilan.voyage.videos.un");
    expect(cles).toContain("bilan.voyage.emojis.un");
    expect(cles[0]).toBe("bilan.voyage.refus");
  });

  it("une étape sans photo posée n'est pas un défaut, sauf si ses photos étaient trop petites : une phrase", () => {
    expect(lignesBilanVoyage(compo(), importe(), []).map((x) => x.cle)).not.toContain("bilan.voyage.etapes_vides");
    const l = lignesBilanVoyage(compo(), importe(), ["Chicago Airport", "", "San Francisco Airport"]);
    expect(l.find((x) => x.cle === "bilan.voyage.etapes_vides")!.params).toEqual({
      n: 2,
      noms: "Chicago Airport, San Francisco Airport",
    });
    const un = lignesBilanVoyage(compo(), importe(), ["Chicago Airport"]);
    expect(un.map((x) => x.cle)).toContain("bilan.voyage.etapes_vides.un");
    expect(FR["bilan.voyage.etapes_vides"]).toBe("{n} étapes n’ont gardé aucune photo, trop petites : {noms}.");
  });

  it("sans import dans l'écran, ni vidéos ni ppi inventés", () => {
    const l = lignesBilanVoyage(compo({ emojis_non_imprimes: 2 }), null);
    expect(l.map((x) => x.cle)).toEqual(["bilan.voyage.emojis"]);
  });
});

describe("le point d'entrée", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
  });

  it("en pause : absent de l'application tant que VITE_COLOPHON_VOYAGE ne vaut pas 1", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    vi.resetModules();
    expect((await import("./voyage")).voyageOffert()).toBe(false);
    vi.stubEnv("VITE_COLOPHON_VOYAGE", "0");
    expect((await import("./voyage")).voyageOffert()).toBe(false);
  });

  it("rallumé, existe dans l'application, jamais dans la version Mac App Store", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    vi.stubEnv("VITE_COLOPHON_VOYAGE", "1");
    vi.resetModules();
    expect((await import("./voyage")).voyageOffert()).toBe(true);
    vi.stubEnv("VITE_COLOPHON_MAS", "1");
    expect((await import("./voyage")).voyageOffert()).toBe(false);
  });

  it("n'existe pas dans le navigateur, même rallumé", async () => {
    vi.stubEnv("VITE_COLOPHON_VOYAGE", "1");
    vi.resetModules();
    expect((await import("./voyage")).voyageOffert()).toBe(false);
  });
});
