// « Comment l'imprimer ? » : moi-même, ou par Booklibre, et le pays où l'on
// se fait livrer. Et la table des tarifs que ce choix autorise à lire.
//
// **Pourquoi c'est un réglage qui se dit.** Lire les tarifs est un appel qui
// sort de la machine : une requête HTTPS vers le Worker des tarifs, sans
// identifiant, sans rien de l'album. Il ne part **qu'après « Par
// Booklibre »** (C59 de la revue 1.1-c-f), une fois par ouverture de l'app,
// et « Moi-même » le coupe. Tant que personne n'a répondu, rien ne part.
//
// **Jamais dans la version Mac App Store** : la question ne s'y pose pas, le
// choix rangé par un DMG d'avant n'y vaut rien, et la commande Tauri n'y
// appelle rien.
//
// Même forme que `maj.ts` : un magasin, des crochets, une clé de
// `localStorage`, aucune bibliothèque. La table lue reste en mémoire.

import { useSyncExternalStore } from "react";
import type { Album } from "./album";
import { lireTarifs, tarifsOfferts, type FormatPreset, type Tarifs } from "./bridge";
import { saveurMas } from "./saveur";

export type Choix = "moi-meme" | "booklibre";

/** Ce qui est rangé : le choix, et le pays quand c'est Booklibre. `pays`
 *  vaut `null` pour « un autre pays », que le relais ne livre pas encore. */
export type Impression = { choix: Choix; pays: string | null };

const CLE = "colophon.impression";

function lu(): Impression | null {
  try {
    const brut = typeof localStorage === "undefined" ? null : localStorage.getItem(CLE);
    if (brut === null) return null;
    const v = JSON.parse(brut) as Partial<Impression>;
    const pays = typeof v.pays === "string" && /^[A-Z]{2}$/.test(v.pays) ? v.pays : null;
    if (v.choix === "moi-meme") return { choix: "moi-meme", pays: null };
    if (v.choix === "booklibre") return { choix: "booklibre", pays };
  } catch {
    /* un stockage bloqué ou un texte abîmé : la question se repose */
  }
  return null;
}

let courante: Impression | null = lu();

/** La table lue, ou ce qui l'empêche. `null` : pas lue. */
export type EtatTarifs =
  | { etat: "aucun" }
  | { etat: "lecture" }
  | { etat: "lus"; tarifs: Tarifs }
  | { etat: "indisponible" };

let tarifs: EtatTarifs = { etat: "aucun" };
/** Ce build a-t-il un Worker de tarifs ? Inconnu tant que l'app n'a pas
 *  demandé (sans réseau). */
let offert: boolean | null = null;

const abonnes = new Set<() => void>();
const prevenir = () => abonnes.forEach((f) => f());

export function abonneImpression(f: () => void): () => void {
  abonnes.add(f);
  return () => {
    abonnes.delete(f);
  };
}

/** Le choix, ou null si la question n'a pas de réponse. Toujours null dans
 *  la version App Store. */
export function impression(): Impression | null {
  return saveurMas() ? null : courante;
}

export function setImpression(i: Impression): void {
  if (saveurMas()) return;
  courante = i;
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(CLE, JSON.stringify(i));
  } catch {
    /* idem : l'oubli à la fermeture, rien de plus */
  }
  // « Moi-même » oublie la table : plus aucun prix, plus aucun appel.
  if (i.choix === "moi-meme") tarifs = { etat: "aucun" };
  prevenir();
  if (i.choix === "booklibre") void chargerTarifs();
}

/** La question se pose-t-elle ? Seulement si ce build a des tarifs, hors
 *  App Store, et que personne n'a encore répondu. */
export function questionAPoser(offre: boolean | null, choix: Impression | null, mas: boolean): boolean {
  return offre === true && !mas && choix === null;
}

/** L'app peut-elle appeler le Worker des tarifs ? Après « Par Booklibre »,
 *  et jamais dans la version App Store (C59). */
export function appelPermis(offre: boolean | null, choix: Impression | null, mas: boolean): boolean {
  return offre === true && !mas && choix?.choix === "booklibre";
}

/** Demande au shell, sans réseau, si ce build a un Worker de tarifs. */
export async function chargerOffre(): Promise<boolean> {
  if (offert === null) {
    offert = saveurMas() ? false : await tarifsOfferts().catch(() => false);
    prevenir();
  }
  return offert;
}

/** Lit la table, une fois par ouverture de l'app, si le choix le permet. */
export async function chargerTarifs(force = false): Promise<void> {
  const offre = await chargerOffre();
  if (!appelPermis(offre, impression(), saveurMas())) return;
  if (!force && (tarifs.etat === "lecture" || tarifs.etat === "lus")) return;
  tarifs = { etat: "lecture" };
  prevenir();
  try {
    const t = await lireTarifs();
    tarifs = t ? { etat: "lus", tarifs: t } : { etat: "indisponible" };
  } catch {
    tarifs = { etat: "indisponible" };
  }
  // Un « Moi-même » arrivé pendant la lecture l'emporte.
  if (impression()?.choix !== "booklibre") tarifs = { etat: "aucun" };
  prevenir();
}

export function etatTarifs(): EtatTarifs {
  return tarifs;
}

export function offre(): boolean | null {
  return offert;
}

export function useImpression(): Impression | null {
  return useSyncExternalStore(abonneImpression, impression, impression);
}

export function useTarifs(): EtatTarifs {
  return useSyncExternalStore(abonneImpression, etatTarifs, etatTarifs);
}

export function useOffre(): boolean | null {
  return useSyncExternalStore(abonneImpression, offre, offre);
}

// ---- lire une case ----------------------------------------------------------

/** Le nombre de pages que Cloudprinter compte : deux par planche
 *  (`prevol.rs::check`, un fournisseur à deux fichiers). */
export function pagesDuLivre(album: Pick<Album, "spreads">): number {
  return album.spreads.length * 2;
}

/** Le nom du format de l'album, par ses cotes, parmi ceux du moteur. */
export function formatDe(formats: FormatPreset[], trim: { w: number; h: number }): string | null {
  return formats.find((f) => f.w === trim.w && f.h === trim.h)?.name ?? null;
}

/** Le prix « à partir de », en centimes, pour `pages` pages : la case de la
 *  table, relevée au minimum du produit. Null au-delà de la table, pour un
 *  format ou un pays qu'elle ne porte pas, ou une case sans prix. L'app lit,
 *  elle ne recalcule rien. */
export function prixPour(t: Tarifs, format: string | null, pays: string | null, pages: number): number | null {
  if (format === null || pays === null) return null;
  const l = t.formats[format]?.[pays];
  if (!l || l.pas <= 0) return null;
  const p = Math.max(pages, l.de);
  const i = Math.ceil((p - l.de) / l.pas);
  return l.ttc_centimes[i] ?? null;
}
