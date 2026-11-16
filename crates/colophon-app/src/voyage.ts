// L'import d'un voyage Polarsteps, vu de l'écran (P-s3).
//
// Comme la photothèque : l'import produit un dossier (photos et
// `voyage.json`), l'écran de création le reprend et compose. Ce fichier tient
// ce qui se teste sans fenêtre : la machine d'états de l'écran, les phrases
// d'erreur (un genre du moteur, une phrase et un remède) et les lignes du
// bilan. L'écran lui-même est `VoyageImport.tsx`.
//
// **Absent de la version Mac App Store** : `voyageOffert()` est faux là, le
// bouton ne se dessine pas, et la commande Rust n'y existe pas non plus.

import { invoke } from "@tauri-apps/api/core";
import type { Album } from "./album";
import { inTauri } from "./bridge";
import { Cle } from "./i18n";
import { saveurMas } from "./saveur";

/** Le point d'entrée existe-t-il dans cette version de l'app ? */
export function voyageOffert(): boolean {
  return inTauri && !saveurMas();
}

// ---- ce que le moteur rend ---------------------------------------------------

/** Les genres d'`Erreur::genre` (lien.rs), plus ceux que seule l'app connaît. */
export type GenreErreur =
  | "url"
  | "prive"
  | "format"
  | "reseau"
  | "indisponible"
  | "disque_plein"
  | "disque"
  | "trop_gros"
  | "photo"
  | "statut"
  | "dossier"
  | "annule"
  | "inconnu";

export type ErreurVoyage = { genre: GenreErreur; detail: string };

export type LienVoyage = { id: number; dossier: string };

/** `voyage::Bilan` : ce que l'import a pris et laissé. */
export type BilanImport = {
  etapes: number;
  etapes_sans_photo: number;
  pays: number;
  photos: number;
  videos_ignorees: number;
  photos_sous_seuil: number;
  trace_presente: boolean;
};

export type RapportVoyage = { dossier: string; titre: string; bilan: BilanImport };

/** `voyage::composition::Bilan`, ce que la composition a fait des étapes. */
export type BilanComposition = {
  titre: string;
  etapes: number;
  etapes_groupees: number;
  recits_debordants: string[];
  emojis_non_imprimes: number;
  refus: string | null;
};

const GENRES: GenreErreur[] = [
  "url", "prive", "format", "reseau", "indisponible", "disque_plein", "disque",
  "trop_gros", "photo", "statut", "dossier", "annule", "inconnu",
];

/** Ce qu'une commande rejetée a rendu, remis en `ErreurVoyage`. Tauri rend
 *  l'objet sérialisé du moteur ; toute autre chose est un « inconnu » dont le
 *  texte voyage quand même. */
export function normaliserErreur(e: unknown): ErreurVoyage {
  if (e && typeof e === "object" && "genre" in e) {
    const { genre, detail } = e as { genre: unknown; detail?: unknown };
    if (typeof genre === "string" && (GENRES as string[]).includes(genre)) {
      return { genre: genre as GenreErreur, detail: typeof detail === "string" ? detail : "" };
    }
  }
  return { genre: "inconnu", detail: e instanceof Error ? e.message : String(e) };
}

/** La phrase et le remède d'un genre. Typé sur les clés du dictionnaire : une
 *  clé absente est une erreur de compilation, pas un libellé brut à l'écran. */
export function phrasesDe(genre: GenreErreur): { phrase: Cle; remede: Cle } | null {
  switch (genre) {
    case "annule":
      return null;
    case "url":
      return { phrase: "voyage.err.url", remede: "voyage.err.url.remede" };
    case "prive":
      return { phrase: "voyage.err.prive", remede: "voyage.err.prive.remede" };
    case "format":
      return { phrase: "voyage.err.format", remede: "voyage.err.format.remede" };
    case "reseau":
      return { phrase: "voyage.err.reseau", remede: "voyage.err.reseau.remede" };
    case "indisponible":
      return { phrase: "voyage.err.indisponible", remede: "voyage.err.indisponible.remede" };
    case "disque_plein":
      return { phrase: "voyage.err.disque_plein", remede: "voyage.err.disque_plein.remede" };
    case "disque":
    case "dossier":
      return { phrase: "voyage.err.disque", remede: "voyage.err.disque.remede" };
    case "trop_gros":
      return { phrase: "voyage.err.trop_gros", remede: "voyage.err.trop_gros.remede" };
    case "photo":
      return { phrase: "voyage.err.photo", remede: "voyage.err.photo.remede" };
    case "statut":
      return { phrase: "voyage.err.statut", remede: "voyage.err.statut.remede" };
    case "inconnu":
      return { phrase: "voyage.err.inconnu", remede: "voyage.err.inconnu.remede" };
  }
}

// ---- la machine d'états de l'écran ------------------------------------------

export type Etat = {
  /** `saisie` : le champ ; `cours` : l'import tourne ; `echec` : une phrase et
   *  un remède ; `annule` : arrêté par la personne, reprise possible. */
  phase: "saisie" | "cours" | "echec" | "annule";
  url: string;
  /** Ce que le moteur a compris de l'adresse, sans réseau. */
  lien: LienVoyage | null;
  /** Un lien collé mais illisible : la phrase s'affiche sous le champ. */
  invalide: boolean;
  fait: number;
  total: number;
  erreur: ErreurVoyage | null;
};

export const ETAT_INITIAL: Etat = {
  phase: "saisie",
  url: "",
  lien: null,
  invalide: false,
  fait: 0,
  total: 0,
  erreur: null,
};

export type Evenement =
  | { type: "url"; url: string; lien: LienVoyage | null }
  | { type: "lancer" }
  | { type: "progres"; fait: number; total: number }
  | { type: "echec"; erreur: ErreurVoyage };

export function reduire(e: Etat, ev: Evenement): Etat {
  switch (ev.type) {
    case "url": {
      const url = ev.url;
      return {
        ...ETAT_INITIAL,
        url,
        lien: ev.lien,
        invalide: url.trim() !== "" && ev.lien === null,
      };
    }
    case "lancer":
      // Sans lien compris, rien ne part : la validation est locale et passe
      // avant tout réseau.
      if (e.lien === null || e.phase === "cours") return e;
      return { ...e, phase: "cours", fait: 0, total: 0, erreur: null };
    case "progres":
      return e.phase === "cours" ? { ...e, fait: ev.fait, total: ev.total } : e;
    case "echec":
      return {
        ...e,
        phase: ev.erreur.genre === "annule" ? "annule" : "echec",
        erreur: ev.erreur,
      };
  }
}

/** « Photos : 412 / 1 546 », ou la lecture du voyage tant que le total n'est
 *  pas connu. */
export function cleProgres(e: Etat): { cle: Cle; params?: Record<string, number | string> } {
  if (e.total === 0) return { cle: "voyage.lecture" };
  return { cle: "voyage.encours", params: { fait: e.fait, total: e.total } };
}

/** Un entier à la française ou à l'anglaise : « 1 546 » / « 1,546 ». */
export function entier(n: number, langue: "fr" | "en"): string {
  // L'espace fine insécable du français ne se dessine pas dans toutes les
  // polices (« 1546 » à l'écran) : l'espace insécable ordinaire, toujours.
  return new Intl.NumberFormat(langue === "fr" ? "fr-FR" : "en-US").format(n).replace(/\u202f/g, "\u00a0");
}

// ---- le bilan ----------------------------------------------------------------

export type LigneBilan = { cle: Cle; params?: Record<string, number | string> };

/** Les lignes du bilan d'un voyage : ce qui a été ignoré, non imprimé, ou trop
 *  petit, nommé et compté. Une ligne à zéro ne s'écrit pas. `importe` est le
 *  rapport de l'import, absent quand le dossier n'a pas passé par l'écran
 *  (il n'a alors ni vidéos ni photos sous le repère à dire). */
export function lignesBilanVoyage(
  compo: BilanComposition,
  importe: RapportVoyage | null,
  /** Les étapes dont les photos, trop petites, sont restées à la réserve. */
  etapesVides: string[] = [],
): LigneBilan[] {
  const out: LigneBilan[] = [];
  if (compo.refus) {
    out.push({ cle: "bilan.voyage.refus", params: { raison: compo.refus } });
  }
  const b = importe?.bilan;
  if (b) {
    out.push({ cle: "bilan.voyage.resume", params: { etapes: b.etapes, photos: b.photos } });
    if (b.videos_ignorees === 1) out.push({ cle: "bilan.voyage.videos.un" });
    else if (b.videos_ignorees > 1) out.push({ cle: "bilan.voyage.videos", params: { n: b.videos_ignorees } });
  }
  if (compo.emojis_non_imprimes === 1) out.push({ cle: "bilan.voyage.emojis.un" });
  else if (compo.emojis_non_imprimes > 1) {
    out.push({ cle: "bilan.voyage.emojis", params: { n: compo.emojis_non_imprimes } });
  }
  if (compo.recits_debordants.length > 0) {
    out.push({ cle: "bilan.voyage.debordants", params: { noms: compo.recits_debordants.join(", ") } });
  }
  if (compo.etapes_groupees > 0) {
    out.push({ cle: "bilan.voyage.groupees", params: { n: compo.etapes_groupees } });
  }
  // Une étape sans photo posée est voulue dans un voyage (page de texte ou
  // regroupée) : elle ne se dit que si elle a perdu ses photos faute de
  // définition, en une phrase.
  const noms = etapesVides.filter((n) => n !== "");
  if (noms.length === 1) out.push({ cle: "bilan.voyage.etapes_vides.un", params: { noms: noms[0] } });
  else if (noms.length > 1) {
    out.push({ cle: "bilan.voyage.etapes_vides", params: { n: noms.length, noms: noms.join(", ") } });
  }
  // Le repère de 2 126 px de côté court (`BilanImport.photos_sous_seuil`) n'est
  // pas un ppi : le bilan de l'écran ne le dit pas. La seule ligne vraie est
  // celle de la réserve (`BuildBilan.trop_petites`), à laquelle `BilanView`
  // ajoute la recommandation du lien.
  if (b && !b.trace_presente) out.push({ cle: "bilan.voyage.trace" });
  return out;
}

// ---- les commandes -----------------------------------------------------------

/** Valide l'adresse sans réseau et propose le dossier. `null` si elle n'est
 *  pas celle d'un voyage. */
export async function analyserLien(url: string): Promise<LienVoyage | null> {
  if (!inTauri || url.trim() === "") return null;
  try {
    return await invoke<LienVoyage>("voyage_analyser", { url });
  } catch {
    return null;
  }
}

/** Importe le voyage, la progression en (faites, total). Rejette en
 *  `ErreurVoyage`. */
export async function importerVoyage(
  url: string,
  dossier: string,
  onProgres: (fait: number, total: number) => void,
): Promise<RapportVoyage> {
  const { listen } = await import("@tauri-apps/api/event");
  const off = await listen<[number, number]>("voyage:progres", (e) => onProgres(e.payload[0], e.payload[1]));
  try {
    return await invoke<RapportVoyage>("voyage_importer", { url, dossier });
  } catch (e) {
    throw normaliserErreur(e);
  } finally {
    off();
  }
}

/** Coche ou décoche la page « Le voyage en chiffres » d'un album composé. */
export async function reglerChiffres(album: Album, actif: boolean, langue: "fr" | "en"): Promise<Album> {
  return invoke<Album>("voyage_chiffres", { album, actif, langue });
}
