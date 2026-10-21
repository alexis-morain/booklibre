// « Comment l'imprimer ? » et la table des tarifs (1.1-f). Ce qui compte ici
// est surtout ce qui ne part pas : aucun appel avant « Par Booklibre »,
// aucun après « Moi-même », aucun dans la version App Store (C59).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Tarifs } from "./bridge";

const appels = { offerts: 0, lire: 0 };
let reponse: Tarifs | null = null;

vi.mock("./bridge", () => ({
  tarifsOfferts: async () => {
    appels.offerts++;
    return true;
  },
  lireTarifs: async () => {
    appels.lire++;
    return reponse;
  },
}));

type Faux = { getItem: (c: string) => string | null; setItem: (c: string, v: string) => void };

function poser(faux: Faux | null): void {
  if (faux === null) {
    delete (globalThis as { localStorage?: unknown }).localStorage;
    return;
  }
  (globalThis as { localStorage?: unknown }).localStorage = faux;
}

function memoire(depart: Record<string, string> = {}): Faux & { sac: Record<string, string> } {
  const sac = { ...depart };
  return { sac, getItem: (c) => sac[c] ?? null, setItem: (c, v) => void (sac[c] = v) };
}

const TABLE: Tarifs = {
  mode: "sandbox",
  genere_le: "2026-10-20T04:00:00.000Z",
  devise: "EUR",
  pays: ["FR"],
  formats: {
    "carre-21": { FR: { de: 24, pas: 2, ttc_centimes: [4600, 4650, 4700, null] } },
  },
};

beforeEach(() => {
  vi.resetModules();
  appels.offerts = 0;
  appels.lire = 0;
  reponse = TABLE;
});
afterEach(() => {
  poser(null);
  vi.unstubAllEnvs();
});

describe("la question", () => {
  it("n'a pas de réponse au premier lancement, et rien ne part", async () => {
    poser(memoire());
    const m = await import("./impression");
    expect(m.impression()).toBeNull();
    await m.chargerTarifs();
    expect(appels.lire).toBe(0);
    expect(m.etatTarifs()).toEqual({ etat: "aucun" });
  });

  it("se pose si le build a des tarifs, hors App Store, tant qu'elle n'a pas de réponse", async () => {
    const { questionAPoser } = await import("./impression");
    expect(questionAPoser(true, null, false)).toBe(true);
    expect(questionAPoser(true, { choix: "moi-meme", pays: null }, false)).toBe(false);
    expect(questionAPoser(true, null, true)).toBe(false);
    expect(questionAPoser(false, null, false)).toBe(false);
    expect(questionAPoser(null, null, false)).toBe(false);
  });

  it("« Par Booklibre » lit la table, une fois par ouverture", async () => {
    const sac = memoire();
    poser(sac);
    const m = await import("./impression");
    m.setImpression({ choix: "booklibre", pays: "FR" });
    await m.chargerTarifs();
    await m.chargerTarifs();
    expect(appels.lire).toBe(1);
    expect(m.etatTarifs()).toEqual({ etat: "lus", tarifs: TABLE });
    expect(JSON.parse(sac.sac["colophon.impression"]!)).toEqual({ choix: "booklibre", pays: "FR" });
  });

  it("« Moi-même » oublie la table et ne lit plus rien", async () => {
    poser(memoire({ "colophon.impression": JSON.stringify({ choix: "booklibre", pays: "FR" }) }));
    const m = await import("./impression");
    await m.chargerTarifs();
    expect(appels.lire).toBe(1);
    m.setImpression({ choix: "moi-meme", pays: null });
    expect(m.etatTarifs()).toEqual({ etat: "aucun" });
    await m.chargerTarifs(true);
    expect(appels.lire).toBe(1);
  });

  it("la version App Store ne pose pas la question, n'appelle rien, ignore un choix d'avant", async () => {
    vi.stubEnv("VITE_COLOPHON_MAS", "1");
    poser(memoire({ "colophon.impression": JSON.stringify({ choix: "booklibre", pays: "FR" }) }));
    const m = await import("./impression");
    expect(m.impression()).toBeNull();
    m.setImpression({ choix: "booklibre", pays: "FR" });
    await m.chargerTarifs(true);
    expect(appels.offerts + appels.lire).toBe(0);
    expect(await m.chargerOffre()).toBe(false);
  });

  it("une réponse absente ou une panne : aucun prix, aucune valeur codée en dur", async () => {
    poser(memoire({ "colophon.impression": JSON.stringify({ choix: "booklibre", pays: "FR" }) }));
    reponse = null;
    const m = await import("./impression");
    await m.chargerTarifs();
    expect(m.etatTarifs()).toEqual({ etat: "indisponible" });
  });

  it("un texte abîmé ou un pays hors forme se relisent sans casser", async () => {
    poser(memoire({ "colophon.impression": "{pas du json" }));
    expect((await import("./impression")).impression()).toBeNull();
    vi.resetModules();
    poser(memoire({ "colophon.impression": JSON.stringify({ choix: "booklibre", pays: "../x" }) }));
    expect((await import("./impression")).impression()).toEqual({ choix: "booklibre", pays: null });
    vi.resetModules();
    poser(null);
    const m = await import("./impression");
    expect(() => m.setImpression({ choix: "moi-meme", pays: null })).not.toThrow();
  });
});

describe("lire une case", () => {
  it("deux pages par planche", async () => {
    const { pagesDuLivre } = await import("./impression");
    expect(pagesDuLivre({ spreads: new Array(13) as never })).toBe(26);
  });

  it("la case du nombre de pages, relevée au minimum du produit", async () => {
    const { prixPour } = await import("./impression");
    expect(prixPour(TABLE, "carre-21", "FR", 26)).toBe(4650);
    expect(prixPour(TABLE, "carre-21", "FR", 2)).toBe(4600);
    expect(prixPour(TABLE, "carre-21", "FR", 27)).toBe(4700);
  });

  it("rien au-delà de la table, sans prix, sans format ou sans pays livré", async () => {
    const { prixPour } = await import("./impression");
    expect(prixPour(TABLE, "carre-21", "FR", 30)).toBeNull();
    expect(prixPour(TABLE, "carre-21", "FR", 202)).toBeNull();
    expect(prixPour(TABLE, "carre-21", "BE", 26)).toBeNull();
    expect(prixPour(TABLE, "paysage-28x21", "FR", 26)).toBeNull();
    expect(prixPour(TABLE, null, "FR", 26)).toBeNull();
    expect(prixPour(TABLE, "carre-21", null, 26)).toBeNull();
  });

  it("le format se lit par ses cotes", async () => {
    const { formatDe } = await import("./impression");
    const f = [{ name: "carre-21", w: 210, h: 210, about: "" }];
    expect(formatDe(f, { w: 210, h: 210 })).toBe("carre-21");
    expect(formatDe(f, { w: 280, h: 210 })).toBeNull();
  });
});
