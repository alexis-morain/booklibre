// L'import d'un voyage, côté écran : la machine d'états, les phrases d'erreur,
// les lignes du bilan, et l'absence du point d'entrée dans la version Mac App
// Store. Aucun réseau, aucune fenêtre.

import { afterEach, describe, expect, it, vi } from "vitest";
import { EN, FR } from "./i18n";
import {
  BilanComposition,
  cleProgres,
  cleReste,
  ETAT_INITIAL,
  Etat,
  resteAffiche,
  STAGNE_MS,
  suivre,
  Suivi,

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
  it("existe dans l'application, jamais dans la version Mac App Store", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    vi.resetModules();
    expect((await import("./voyage")).voyageOffert()).toBe(true);
    vi.stubEnv("VITE_COLOPHON_MAS", "1");
    expect((await import("./voyage")).voyageOffert()).toBe(false);
  });

  it("n'existe pas dans le navigateur", async () => {
    vi.resetModules();
    expect((await import("./voyage")).voyageOffert()).toBe(false);
  });
});

// ---- le temps restant ---------------------------------------------------------

/** Rejoue des photos qui finissent à `cadence` photos/s depuis `debut` (ms). */
function rejouer(total: number, fin_ms: number, cadence: (s: number) => number, debut = 0, depart: Suivi | null = null) {
  let suivi = depart;
  let fait = depart ? depart.points[depart.points.length - 1].fait : 0;
  const dits: (number | null)[] = [];
  for (let a = debut + 1000; a <= debut + fin_ms; a += 1000) {
    fait = Math.min(total, fait + cadence((a - debut) / 1000));
    suivi = suivre(suivi, a, fait, total);
    dits.push(resteAffiche(suivi, a));
  }
  return { suivi: suivi as Suivi, dits };
}

describe("le temps restant", () => {
  it("ne dit rien avant quelques photos ni avant quelques secondes", () => {
    let s = suivre(null, 0, 1, 635);
    s = suivre(s, 1000, 2, 635);
    s = suivre(s, 2000, 3, 635);
    s = suivre(s, 3000, 4, 635);
    expect(resteAffiche(s, 3000)).toBeNull(); // 3 photos en 3 s : trop peu
    s = suivre(s, 4000, 5, 635);
    s = suivre(s, 5000, 6, 635);
    expect(resteAffiche(s, 5000)).not.toBeNull();
    // Beaucoup de photos, mais en moins de 3 s : pas encore.
    const vite = suivre(suivre(null, 0, 1, 635), 2000, 30, 635);
    expect(resteAffiche(vite, 2000)).toBeNull();
  });

  it("estime d'après le débit récent : 1 photo/s, 635 photos, environ 10 min", () => {
    const { dits } = rejouer(635, 12_000, () => 1);
    expect(dits[dits.length - 1]).toBe(10);
  });

  it("le temps de lecture du voyage ne compte pas : l'estimation vient des photos", () => {
    // La première photo ne finit qu'à 60 s ; ensuite 2 photos/s sur 600.
    const { dits } = rejouer(600, 20_000, () => 2, 60_000, suivre(null, 60_000, 0, 600));
    expect(dits[dits.length - 1]).toBe(5); // ~560 restantes à 2/s = 280 s, et non 10+ min
  });

  it("une reprise qui saute des photos d'un coup ne fausse pas longtemps", () => {
    // 300 photos déjà là défilent en 5 s, puis 1 photo/s : au bout de 45 s, la fenêtre ne voit plus la rafale.
    const rafale = rejouer(635, 5000, () => 60);
    const { dits } = rejouer(635, 45_000, () => 1, 5000, rafale.suivi);
    expect(dits[dits.length - 1]).toBe(5); // 335 restantes à 1/s ≈ 5,6 min
  });

  it("le chiffre ne saute pas à chaque photo : un débit irrégulier garde un compte à rebours calme", () => {
    // 1 photo/s en moyenne, mais 0 ou 2 en alternance, puis des à-coups.
    const { dits } = rejouer(635, 200_000, (s) => (Math.floor(s) % 7 === 0 ? 3 : Math.floor(s) % 2 === 0 ? 0 : 1));
    const dites = dits.filter((d): d is number => d !== null);
    let changements = 0;
    for (let i = 1; i < dites.length; i++) {
      if (dites[i] !== dites[i - 1]) changements++;
      expect(dites[i] - dites[i - 1]).toBeLessThanOrEqual(1); // jamais un grand bond
    }
    // Environ 195 s de décompte : une dizaine de minutes au plus, jamais un changement par photo.
    expect(changements).toBeLessThan(dites.length / 6);
  });

  it("la minute dite ne change que si l'estimation sort franchement de sa minute", () => {
    const points = Array.from({ length: 10 }, (_, k) => ({ a: 100_000 + k * 1000, fait: 100 + k }));
    const prec: Suivi = { points, lisse: 270, a_lisse: 109_000, affiche: 5, derniere: 109_000 };
    // 268 s restantes (4,47 min) : on dit encore 5 ; à 200 s (3,3 min), on passe à 3.
    const proche = suivre(prec, 110_000, 110, 110 + 268);
    expect(proche.affiche).toBe(5);
    const loin = suivre({ ...prec, lisse: 200 }, 110_000, 110, 110 + 199);
    expect(loin.affiche).toBe(3);
  });

  it("se tait quand plus rien n'arrive (attente de nouvelle tentative), reparle ensuite", () => {
    const { suivi } = rejouer(635, 12_000, () => 1);
    const t = 12_000;
    expect(resteAffiche(suivi, t + STAGNE_MS - 100)).not.toBeNull();
    expect(resteAffiche(suivi, t + STAGNE_MS + 100)).toBeNull();
    const repart = suivre(suivi, t + 20_000, 13, 635);
    expect(resteAffiche(repart, t + 20_000)).not.toBeNull();
  });

  it("se dit en minutes arrondies, FR et EN", () => {
    expect(cleReste(0)).toEqual({ cle: "voyage.reste.moins" });
    expect(cleReste(1)).toEqual({ cle: "voyage.reste.un" });
    expect(cleReste(3)).toEqual({ cle: "voyage.reste.min", params: { n: 3 } });
    expect(cleReste(89)).toEqual({ cle: "voyage.reste.min", params: { n: 89 } });
    expect(cleReste(90)).toEqual({ cle: "voyage.reste.h", params: { n: 2 } });
    expect(FR["voyage.reste.moins"]).toBe("moins d’une minute restante");
    expect(FR["voyage.reste.min"].replace("{n}", "3")).toBe("environ 3 min restantes");
    expect(EN["voyage.reste.moins"]).toBe("less than a minute left");
    expect(EN["voyage.reste.min"].replace("{n}", "3")).toBe("about 3 min left");
  });

  it("l'état : l'estimation vient avec l'instant, s'éteint au tic, repart avec la photo, et se remet à zéro", () => {
    const base: Etat = reduire(COLLE, { type: "lancer" });
    let e = base;
    for (let k = 1; k <= 12; k++) e = reduire(e, { type: "progres", fait: k, total: 635, a: k * 1000 });
    expect(e.reste).toBe(10);
    expect(reduire(e, { type: "tic", a: 12_000 + STAGNE_MS + 500 }).reste).toBeNull();
    expect(reduire(e, { type: "tic", a: 13_000 }).reste).toBe(10);
    // Sans instant, jamais d'estimation.
    expect(reduire(base, { type: "progres", fait: 9, total: 635 }).reste).toBeNull();
    // Un nouvel essai repart de zéro, une annulation éteint.
    expect(reduire(e, { type: "echec", erreur: { genre: "annule", detail: "" } }).reste).toBeNull();
    expect(reduire(reduire(e, { type: "echec", erreur: { genre: "annule", detail: "" } }), { type: "lancer" }).suivi).toBeNull();
  });
});
