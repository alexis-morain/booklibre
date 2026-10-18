// Ce que l'écran Envoi décide, hors du DOM : quel imprimeur il nomme, ce que
// fait son bouton, et la phrase qu'il dit une fois le dossier préparé.
//
// L'écran n'offre plus qu'un imprimeur. Cloudprinter est celui qu'on
// commande ; Prodigi et Lulu restent des profils du moteur et de la CLI
// (`--profil`), et ne s'affichent plus. Le PDF sans contrainte reste à un
// lien de distance : l'export gratuit, hors ligne et sans compte ne dépend
// d'aucun imprimeur.

import type { Album } from "./album";
import type { Defaut, Preparation, Printer } from "./bridge";
import { Cle, FR, t } from "./i18n";

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

/** Le bouton de l'écran. Chez qui relie deux fichiers, il prépare un
 *  dossier ; sinon il enregistre un PDF, comme il l'a toujours fait. Chez
 *  Cloudprinter, quand le relais répond, il devient « Faire imprimer » : la
 *  préparation se fait sans boîte, dans le dossier de données, et mène droit
 *  à la commande (`commande`). Sans relais joignable, rien ne change. */
export function actionEnvoi(
  p: Printer,
  relaisJoignable = false,
): { prepare: boolean; commande: boolean; texte: string; titre: string } {
  if (p.fichiers !== "deux") {
    return { prepare: false, commande: false, texte: t("envoi.exporter"), titre: t("envoi.exporter.titre") };
  }
  return relaisJoignable && p.id === PROFIL_ENVOI
    ? { prepare: true, commande: true, texte: t("envoi.faire_imprimer"), titre: t("envoi.faire_imprimer.titre") }
    : {
        prepare: true,
        commande: false,
        texte: t("envoi.preparer", { imprimeur: p.nom }),
        titre: t("envoi.preparer.titre"),
      };
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
 *  - `format` : le panneau Format ;
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
