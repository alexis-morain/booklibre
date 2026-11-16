// Remplacer les vignettes d'un voyage par ses photos d'origine (P-s8, plan
// D17), vu de l'écran. Ce fichier tient ce qui se teste sans fenêtre : les
// types que rend le moteur, la machine d'états du bloc, les lignes du bilan.
// Le bloc lui-même est `VoyageOriginaux.tsx`.
//
// Le moteur ne remplace que ce dont il est sûr, sans effacer la vignette ; le
// bilan dit le compte et nomme chaque photo laissée, avec sa raison.

import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./bridge";
import { Cle } from "./i18n";
import { entier } from "./voyage";

export type RaisonRefus =
  | "sans_original"
  | "ambigu"
  | "conflit"
  | "pas_plus_grand"
  | "format"
  | "illisible";

/** `originaux::Refus` : une vignette laissée telle quelle, et pourquoi. */
export type RefusOriginal = { fichier: string; etape: string; raison: RaisonRefus };

/** `originaux::Rapport`, ce qu'un passage vient de faire. */
export type RapportOriginaux = {
  remplacees: number;
  deja: number;
  refus: RefusOriginal[];
  illisibles: number;
  sous_seuil_avant: number;
  sous_seuil_apres: number;
  revenues_possibles: number;
};

/** `originaux_etat` : ce qu'un album de voyage sait déjà de ses originaux. */
export type EtatOriginaux = {
  voyage: boolean;
  remplacees: number;
  refus: RefusOriginal[];
  dossier: string;
};

const RAISONS: Record<RaisonRefus, Cle> = {
  sans_original: "originaux.raison.sans_original",
  ambigu: "originaux.raison.ambigu",
  conflit: "originaux.raison.conflit",
  pas_plus_grand: "originaux.raison.pas_plus_grand",
  format: "originaux.raison.format",
  illisible: "originaux.raison.illisible",
};

export function cleRaison(r: RaisonRefus): Cle {
  return RAISONS[r] ?? RAISONS.sans_original;
}

/** Combien de lignes de la liste se dessinent : une liste de 700 lignes ne
 *  se lit pas, elle se défile ; le reste est dit en chiffre. */
export const LIGNES_VISIBLES = 200;

// ---- la machine d'états -------------------------------------------------------

export type Etat = {
  /** `repos` : rien en cours ; `cours` : le moteur lit et remplace ; `echec`. */
  phase: "repos" | "cours" | "echec" | "annule";
  fait: number;
  total: number;
  /** Les photos remplacées, tous passages confondus. */
  remplacees: number;
  refus: RefusOriginal[];
  dossier: string;
  /** Un passage vient de finir : ce qu'il a changé. */
  rapport: RapportOriginaux | null;
  /** Les vignettes viennent d'être rendues. */
  rendues: boolean;
  detail: string;
};

export const ETAT_INITIAL: Etat = {
  phase: "repos",
  fait: 0,
  total: 0,
  remplacees: 0,
  refus: [],
  dossier: "",
  rapport: null,
  rendues: false,
  detail: "",
};

export type Evenement =
  | { type: "etat"; etat: EtatOriginaux }
  | { type: "lancer" }
  | { type: "progres"; fait: number; total: number }
  | { type: "fait"; rapport: RapportOriginaux; dossier: string }
  | { type: "echec"; detail: string; annule: boolean }
  | { type: "rendues" };

export function reduire(e: Etat, ev: Evenement): Etat {
  switch (ev.type) {
    case "etat":
      // Ce que l'album sait déjà ; jamais par-dessus un passage en cours.
      if (e.phase === "cours") return e;
      return { ...e, remplacees: ev.etat.remplacees, refus: ev.etat.refus, dossier: ev.etat.dossier };
    case "lancer":
      if (e.phase === "cours") return e;
      return { ...e, phase: "cours", fait: 0, total: 0, rapport: null, rendues: false, detail: "" };
    case "progres":
      return e.phase === "cours" ? { ...e, fait: ev.fait, total: ev.total } : e;
    case "fait":
      return {
        ...e,
        phase: "repos",
        rapport: ev.rapport,
        dossier: ev.dossier,
        remplacees: e.remplacees + ev.rapport.remplacees,
        refus: ev.rapport.refus,
      };
    case "echec":
      return { ...e, phase: ev.annule ? "annule" : "echec", detail: ev.detail };
    case "rendues":
      return { ...e, phase: "repos", remplacees: 0, refus: [], dossier: "", rapport: null, rendues: true };
  }
}

// ---- les lignes du bilan --------------------------------------------------------

export type Ligne = { cle: Cle; params?: Record<string, number | string> };

/** Les lignes que l'écran dit : le compte, les sans-original, ce qui peut
 *  revenir de la réserve, ce qui n'a pas pu se lire. Rien à zéro ne s'écrit,
 *  hors le cas où rien du tout n'a été remplacé : il se dit. */
export function lignesOriginaux(e: Etat, langue: "fr" | "en"): Ligne[] {
  const out: Ligne[] = [];
  const n = e.remplacees;
  const m = e.refus.length;
  if (n === 0 && e.rapport) out.push({ cle: "originaux.aucune" });
  if (n === 1) out.push({ cle: "originaux.remplacees.un" });
  else if (n > 1) out.push({ cle: "originaux.remplacees", params: { n: entier(n, langue) } });
  if (n > 0 || e.rapport) {
    if (m === 1) out.push({ cle: "originaux.sans.un" });
    else if (m > 1) out.push({ cle: "originaux.sans", params: { n: entier(m, langue) } });
  }
  const r = e.rapport;
  if (r) {
    if (r.revenues_possibles === 1) out.push({ cle: "originaux.reserve.un" });
    else if (r.revenues_possibles > 1) {
      out.push({ cle: "originaux.reserve", params: { n: entier(r.revenues_possibles, langue) } });
    }
    if (r.illisibles > 0) out.push({ cle: "originaux.illisibles", params: { n: entier(r.illisibles, langue) } });
  }
  return out;
}

// ---- les commandes -------------------------------------------------------------

/** Ce que l'album ouvert sait de ses originaux ; `null` hors de l'app. */
export async function etatOriginaux(): Promise<EtatOriginaux | null> {
  if (!inTauri) return null;
  try {
    return await invoke<EtatOriginaux>("originaux_etat");
  } catch {
    return null;
  }
}

/** Remplace les vignettes par les photos de `dossier`. La progression est
 *  `(lues, total)`. Rejette en texte. */
export async function remplacerOriginaux(
  dossier: string,
  onProgres: (fait: number, total: number) => void,
): Promise<RapportOriginaux> {
  const { listen } = await import("@tauri-apps/api/event");
  const off = await listen<[number, number]>("originaux:progres", (e) => onProgres(e.payload[0], e.payload[1]));
  try {
    return await invoke<RapportOriginaux>("originaux_remplacer", { originaux: dossier });
  } finally {
    off();
  }
}

/** Rend les vignettes ; le nombre rendu. */
export async function retablirOriginaux(): Promise<number> {
  return invoke<number>("originaux_retablir");
}
