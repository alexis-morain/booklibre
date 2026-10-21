// Ce que l'écran Envoi décide, hors du DOM : quel imprimeur il nomme, ce que
// fait son bouton, et la phrase qu'il dit une fois le dossier préparé.
//
// L'écran n'offre plus qu'un imprimeur. Cloudprinter est celui qu'on
// commande ; Prodigi et Lulu restent des profils du moteur et de la CLI
// (`--profil`), et ne s'affichent plus. Le PDF sans contrainte reste à un
// lien de distance : l'export gratuit, hors ligne et sans compte ne dépend
// d'aucun imprimeur.

import type { Album } from "./album";
import type { Defaut, PageSousSeuil, Preparation, Printer } from "./bridge";
import type { Relais } from "./commande";
import { Cle, FR, langue, t } from "./i18n";
import codes from "./moteur.codes.json";

/** L'imprimeur que l'écran nomme et prépare. */
export const PROFIL_ENVOI = "cloudprinter";
/** Le PDF sans contrainte, derrière le lien « Un autre imprimeur ? ». */
export const PROFIL_LIBRE = "generique";

/** Le profil que l'écran montre : le PDF sans contrainte quand on l'a
 *  choisi, Cloudprinter dans tous les autres cas. Null tant que le moteur
 *  n'a pas rendu la liste : l'écran attend plutôt que d'inventer. */
export function imprimeurAffiche(
  printers: Printer[] | null,
  profil: string,
): Printer | null {
  if (!printers) return null;
  const id = profil === PROFIL_LIBRE ? PROFIL_LIBRE : PROFIL_ENVOI;
  return printers.find((p) => p.id === id) ?? null;
}

/** Les deux portes d'Envoi : le PDF, pour l'imprimeur de son choix, et le
 *  livre que Colophon fait imprimer (A2). */
export type Porte = "pdf" | "colophon";

/** Les portes que l'écran offre. Sans relais compilé, la porte Colophon
 *  n'existe pas, et rien ne l'annonce : la release publique ne montre que
 *  le PDF. Tant que le relais n'a pas répondu, pareil. Un relais compilé en
 *  panne garde sa porte, qui dit la panne. */
export function portesOffertes(relais: Relais | null): Porte[] {
  return relais === null || relais.etat === "absent" ? ["pdf"] : ["pdf", "colophon"];
}

/** La porte ouverte : celle qu'on a choisie si elle existe encore, sinon
 *  Colophon quand le relais répond, le PDF dans tous les autres cas. */
export function porteEffective(choisie: Porte | null, relais: Relais | null): Porte {
  const offertes = portesOffertes(relais);
  if (choisie !== null && offertes.includes(choisie)) return choisie;
  return relais?.etat === "joignable" ? "colophon" : "pdf";
}

/** Le bouton de l'écran, qui suit la porte. Par la porte PDF il télécharge :
 *  chez qui relie deux fichiers, le dossier préparé dans une boîte qu'on
 *  choisit ; sinon un seul PDF. Par la porte Colophon, chez Cloudprinter, il
 *  devient « Faire imprimer » : la préparation se fait sans boîte, dans le
 *  dossier de données, et mène droit à la commande (`commande`). */
export function actionEnvoi(
  p: Printer,
  porte: Porte = "pdf",
): { prepare: boolean; commande: boolean; texte: string; titre: string } {
  if (porte === "colophon" && p.id === PROFIL_ENVOI && p.fichiers === "deux") {
    return { prepare: true, commande: true, texte: t("envoi.faire_imprimer"), titre: t("envoi.faire_imprimer.titre") };
  }
  return p.fichiers === "deux"
    ? { prepare: true, commande: false, texte: t("envoi.telecharger"), titre: t("envoi.preparer.titre") }
    : { prepare: false, commande: false, texte: t("envoi.telecharger"), titre: t("envoi.exporter.titre") };
}

/** Le format d'une page, en centimètres, à la virgule de la langue. */
export function formatEnCm([w, h]: [number, number]): string {
  const cm = (mm: number) => {
    const v = Math.round(mm) / 10;
    return (Number.isInteger(v) ? String(v) : v.toFixed(1)).replace(".", langue() === "fr" ? "," : ".");
  };
  return `${cm(w)} × ${cm(h)} cm`;
}

const FINITIONS: Record<string, Cle> = {
  cover_finish_matte: "envoi.papier.finition.matte",
  cover_finish_gloss: "envoi.papier.finition.gloss",
  cover_finish_soft_touch: "envoi.papier.finition.soft_touch",
};

/** Le papier en clair, tiré du profil et jamais de ses codes : le grammage
 *  de la fiche, « couverture rigide » quand le profil cartonne (un rempli
 *  autour des plats), et la finition quand l'app la connaît. Null pour un
 *  profil sans options de commande : c'est l'imprimeur choisi qui décide. */
export function papierEnClair(p: Printer, grammage: number): string | null {
  if (!p.commande) return null;
  const papier = t("envoi.papier.grammage", { g: grammage });
  if (p.rempli_mm <= 0) return papier;
  const f = Object.prototype.hasOwnProperty.call(FINITIONS, p.commande.finition)
    ? FINITIONS[p.commande.finition]
    : null;
  const couverture = f
    ? t("envoi.papier.rigide.finition", { finition: t(f) })
    : t("envoi.papier.rigide");
  return `${papier}, ${couverture}`;
}

/** Une phrase : le dossier est prêt, ou ce que le contrôle relu sur ses
 *  fichiers y a trouvé. */
export function phrasePreparation(p: Preparation): string {
  const n = p.rapport.bloquants;
  if (p.rapport.ok) return t("envoi.prepare.ok", { dossier: p.dossier });
  return n === 1
    ? t("envoi.prepare.ko.un", { dossier: p.dossier })
    : t("envoi.prepare.ko", { dossier: p.dossier, n });
}

/** Un geste offert sur une ligne de défaut d'Envoi. */
export type ActionDefaut = "voir" | "remplacer" | "retirer";

/** Les gestes d'un défaut `resolution`. Retirer marche toujours, même quand
 *  aucune case n'est assez petite pour cet original : c'est l'échappatoire
 *  qui ne manque jamais. Remplacer demande une réserve. Rien pour un défaut
 *  sans planche ni case, rien pour une autre règle. Et rien non plus quand la
 *  case ne porte plus la photo que le rapport nomme : le prévol lit le
 *  disque, et après une édition non enregistrée retirer viserait une autre
 *  photo. */
export function actionsDefaut(d: Defaut, album: Album, reserveVide: boolean): ActionDefaut[] {
  if (d.regle !== "resolution" || d.planche === undefined || d.case === undefined) return [];
  if (album.spreads[d.planche - 1]?.slots[d.case]?.src !== d.src) return [];
  return reserveVide ? ["voir", "retirer"] : ["voir", "remplacer", "retirer"];
}

/** La résolution qu'un défaut `resolution` mesure : la valeur que le moteur
 *  donne avec sa phrase, sinon lue dans la cause, pour un moteur plus ancien
 *  que la fenêtre. Null quand ni l'une ni l'autre ne la dit. */
export function ppiDe(d: Defaut): number | null {
  const valeur = d.phrase?.valeurs?.ppi;
  if (valeur !== undefined && /^\d+$/.test(valeur)) return Number(valeur);
  const m = / (\d+) ppi /.exec(d.cause);
  return m ? Number(m[1]) : null;
}

/** Où mène une ligne de défaut sans planche, ou plutôt ce qui la répare.
 *
 *  - `planche` : le saut existant, quand le défaut en nomme une ;
 *  - `couverture` : l'éditeur de couverture (l'index `COVER` d'App) ;
 *  - `format` : le panneau Recomposer, où se choisit le format ;
 *  - `preparer` : refaire le dossier préparé, seul endroit où un fichier se
 *    juge depuis que la fenêtre ne regarde plus ceux d'à côté de l'album ;
 *  - `ici` : le geste est sur cet écran même (une page à décocher) ;
 *  - `composition` : rien dans la fenêtre ne le change, l'album se compose à
 *    nouveau. Le fond perdu est fixé à la composition, et l'app compose
 *    toujours aux trois millimètres que Cloudprinter demande. */
export type Destination = "planche" | "couverture" | "format" | "preparer" | "ici" | "composition";

/** Une destination par règle du prévol, et la liste est celle du moteur
 *  (`prevol::REGLES`, dans `moteur.codes.json`) : `envoi.test.ts` refuse une
 *  règle qui n'aurait nulle part où mener. Les règles qui nomment toujours
 *  leur planche y mènent ; la table dit où va le reste. */
export const DESTINATIONS: Record<string, Destination> = {
  conformite: "preparer",
  couverture_resolution: "couverture",
  couverture_vide: "couverture",
  dos: "couverture",
  dos_nu: "couverture",
  espace: "preparer",
  fichier_couverture: "preparer",
  fichier_interieur: "preparer",
  fichier_perime: "preparer",
  fichier_profil: "preparer",
  fond_perdu: "composition",
  format_sans_produit: "format",
  imposition: "planche",
  objet_coupe: "planche",
  objet_pli: "planche",
  pagination: "format",
  quatrieme_deborde: "couverture",
  quatrieme_resolution: "couverture",
  quatrieme_zone_sure: "couverture",
  resolution: "planche",
  zone_sure: "planche",
};

/** Les remèdes dont le geste est sur cet écran même : décocher la page de
 *  colophon ou la page de garde, plus bas dans Envoi. Les envoyer vers Format
 *  serait les envoyer là où le geste n'est pas. */
const REMEDES_ICI = new Set(["pagination.decocher_colophon", "pagination.decocher_garde"]);

/** Où mène ce défaut. Null pour une règle que la table ne connaît pas (un
 *  moteur plus récent que la fenêtre) : l'écran ne l'invente pas. */
export function destinationDe(d: Defaut): Destination | null {
  if (d.planche !== undefined) return "planche";
  if (d.phrase && REMEDES_ICI.has(d.phrase.remede)) return "ici";
  const dest = DESTINATIONS[d.regle];
  // Une règle de planche sans planche ne mène nulle part de sûr.
  return dest === undefined || dest === "planche" ? null : dest;
}

const LIBELLES: Record<Exclude<Destination, "planche">, Cle> = {
  couverture: "envoi.ou.couverture",
  format: "envoi.ou.format",
  preparer: "envoi.ou.fichiers",
  ici: "envoi.ou.pages",
  composition: "envoi.ou.composition",
};

/** L'étiquette d'une ligne : sa planche, sinon sa destination. « L'album »
 *  ne reste que pour une règle inconnue. */
export function libelleDestination(d: Defaut): string {
  if (d.planche !== undefined) return t("envoi.defaut.planche", { n: d.planche });
  const dest = destinationDe(d);
  return dest === null || dest === "planche" ? t("envoi.defaut.album") : t(LIBELLES[dest]);
}

const estCle = (k: string): k is Cle => k in FR;

/** La phrase que l'écran montre d'abord : ce qui ne va pas et le geste, sans
 *  un chiffre. Les ppi, les millimètres et les noms de fichiers restent dans
 *  la phrase du moteur, sous « Détails ». Écrite par l'app dans les deux
 *  langues, comme `moteur.ts` écrit l'anglais : le français du moteur ne
 *  change pas, la CLI et les tests le lisent. Un remède qui change le geste
 *  (décocher une page plutôt qu'en ajouter) a sa phrase à lui. Null pour une
 *  règle inconnue : l'écran garde alors la phrase du moteur. */
export function phraseSimple(d: Defaut): string | null {
  const parRemede = d.phrase ? `envoi.simple.${d.phrase.remede}` : "";
  if (estCle(parRemede)) return t(parRemede);
  const cle = `envoi.simple.${d.regle}`;
  return estCle(cle) ? t(cle) : null;
}

/** Où « Feuilleter » ouvre le livre (A4, décision 11) : sur la couverture
 *  quand son aperçu vient d'être rendu, sur la première planche sinon — un
 *  rendu de couverture peut échouer (un dossier de photos déplacé) sans
 *  coûter l'aperçu de l'intérieur. */
export function ouvertureDuFeuilletage(couvertureRendue: boolean): "couverture" | "premiere" {
  return couvertureRendue ? "couverture" : "premiere";
}

// ---- à vous de voir (1.1-c) -------------------------------------------------

const A_VOUS_DE_VOIR = new Set<string>(codes.a_vous_de_voir);

/** Un défaut que l'imprimeur ne refuse pas et que l'auteur tranche : une
 *  photo sous le plancher (`prevol::A_VOUS_DE_VOIR`). */
export function aVousDeVoir(d: Defaut): boolean {
  return A_VOUS_DE_VOIR.has(d.regle);
}

/** Les défauts d'un rapport, dans l'ordre de l'écran : ce que l'imprimeur
 *  refuse, ce qui est à vous de voir, puis les avertissements. */
export function groupesDefauts(defauts: Defaut[]): { refuses: Defaut[]; aVoir: Defaut[]; avertissements: Defaut[] } {
  return {
    refuses: defauts.filter((d) => d.bloquant),
    aVoir: defauts.filter((d) => !d.bloquant && aVousDeVoir(d)),
    avertissements: defauts.filter((d) => !d.bloquant && !aVousDeVoir(d)),
  };
}

/** « a, b et c » dans la langue de l'écran. */
function enumerer(mots: string[]): string {
  if (mots.length <= 1) return mots.join("");
  return `${mots.slice(0, -1).join(", ")} ${t("envoi.pages.et")} ${mots[mots.length - 1]}`;
}

/** Les pages sous le plancher, en clair : « la couverture et les pages 5 et
 *  13 ». La couverture d'abord, comme dans le livre. */
export function pagesEnClair(pages: PageSousSeuil[]): string {
  const interieur = pages.filter((p) => p.fichier === "interieur").map((p) => String(p.page));
  const parts: string[] = [];
  if (pages.some((p) => p.fichier === "couverture")) parts.push(t("envoi.pages.couverture"));
  if (interieur.length === 1) parts.push(t("envoi.pages.une", { n: interieur[0] }));
  else if (interieur.length > 1) parts.push(t("envoi.pages.plusieurs", { liste: enumerer(interieur) }));
  return enumerer(parts);
}

/** Le libellé de la case, version 1 (`docs/acquittement-v1.md`) : il
 *  nomme les pages. Changer un mot, c'est changer de version. */
export function libelleAcquittement(pages: PageSousSeuil[]): string {
  return t(pages.length === 1 ? "envoi.acquittement.un" : "envoi.acquittement", { pages: pagesEnClair(pages) });
}

/** Une clé par liste de pages : la case se décoche quand elle change. */
export function cleAcquittement(pages: PageSousSeuil[]): string {
  return pages.map((p) => `${p.fichier}:${p.page}:${p.ppi}`).join(",");
}

/** Le verdict vert, avec le compte des photos à vous de voir. */
export function verdictPret(aVoir: number): string {
  if (aVoir === 0) return t("envoi.pret");
  return aVoir === 1 ? t("envoi.pret.a_voir.un") : t("envoi.pret.a_voir", { n: aVoir });
}

/** Le bouton d'Envoi. Un refus de l'imprimeur le grise. Sinon, par la porte
 *  PDF, des pages sous le plancher demandent une confirmation qui les nomme ;
 *  par la porte Booklibre, la case qui les nomme doit être cochée. */
export function boutonEnvoi(e: {
  ok: boolean;
  pages: PageSousSeuil[];
  commande: boolean;
  acquitte: boolean;
}): { actif: boolean; confirmer: boolean } {
  if (!e.ok) return { actif: false, confirmer: false };
  if (e.pages.length === 0) return { actif: true, confirmer: false };
  if (e.commande) return { actif: e.acquitte, confirmer: false };
  return { actif: true, confirmer: true };
}
