// Le mode démo (`vite build --mode demo`, drapeau `VITE_DEMO`) : la vraie
// interface, sur un album composé d'avance, sans moteur ni serveur.
//
// La seule porte vers le moteur est `bridge.ts`, et hors de Tauri elle parle
// au harnais de dev par `fetch("/__dev/...")`. Ici ce `fetch` est remplacé :
// les lectures sortent d'un manifeste d'enregistrements (`demo-data/`, écrit
// par `scripts/demo.sh` en rejouant le harnais), l'écriture de l'album reste
// en mémoire, et tout ce qui demande le moteur ou le système répond 501 avec
// une phrase qui mène au téléchargement. Un rechargement repart de l'album
// d'origine, par construction : rien n'est jamais écrit nulle part.
//
// Ce module n'est atteint que derrière `import.meta.env.VITE_DEMO === "1"`,
// écrit en clair à chaque site pour que le build normal le laisse tomber ; ses
// phrases vivent ici et non dans `i18n.ts` pour la même raison. Aucun effet de
// bord au chargement : sans cela le build normal le garderait.

import { langue, type Lang } from "./i18n";

/** La page de téléchargement du site, dans la langue de l'interface. */
export function lienTelecharger(l: Lang = langue()): string {
  return l === "fr" ? "https://booklibre.app/fr/#get" : "https://booklibre.app/#get";
}

/** Au démarrage la démo n'ouvre pas la planche 1, page collée dans la
 *  couverture et vide : la première planche qui porte au moins deux photos,
 *  à défaut une seule, à défaut la première. */
export function premierCoupDOeil(planches: { slots: unknown[] }[]): number {
  const i = planches.findIndex((p) => p.slots.length >= 2);
  if (i >= 0) return i;
  return Math.max(0, planches.findIndex((p) => p.slots.length >= 1));
}

/** Les largeurs sous lesquelles l'éditeur n'a pas la place de se montrer. */
export const LARGEUR_MIN = 900;

export const DEMO_FR = {
  "bandeau.texte": "Démo : rien n’est enregistré",
  "bandeau.lien": "Télécharger Booklibre",
  "bandeau.credit": "Photos : {auteur}, {licence}",
  "bandeau.credits": "crédits",
  "refus.titre": "Disponible dans l’app",
  "refus.lien": "Télécharger Booklibre",
  "refus.export":
    "Exporter le PDF prêt à imprimer se fait dans l’application Booklibre, sur votre ordinateur. La démo ne produit aucun fichier. Téléchargement : booklibre.app",
  "refus.commande":
    "Commander l’album imprimé se fait dans l’application Booklibre. Rien ne part de cette page. Téléchargement : booklibre.app",
  "refus.recomposer":
    "Recomposer l’album ou changer de format demande le moteur de composition, qui est dans l’application. Téléchargement : booklibre.app",
  "refus.remplir":
    "Remplir les cases vides demande le moteur de composition, qui est dans l’application. Téléchargement : booklibre.app",
  "refus.album":
    "Créer un album ou en ouvrir un autre se fait dans l’application, à partir de vos propres photos : la démo ne lit aucun fichier de votre ordinateur. Téléchargement : booklibre.app",
  "refus.signaler":
    "Signaler un problème se fait depuis l’application, qui joint ce qu’il faut au rapport. Téléchargement : booklibre.app",
  "refus.stockage": "Le stockage des albums n’existe que dans l’application. Téléchargement : booklibre.app",
  "refus.origine":
    "Rendre une planche à l’automatique relit la composition d’origine, que seule l’application garde. Téléchargement : booklibre.app",
  "mobile.titre": "La démo est faite pour un ordinateur",
  "mobile.texte":
    "L’éditeur d’album a besoin d’un grand écran et d’une souris. Téléchargez Booklibre pour Mac et Windows.",
  "mobile.telecharger": "Télécharger Booklibre",
  "fidele.demo": "Aperçu fidèle : le PDF enregistré de la démo, sans vos modifications",
  "chargement": "Ouverture de l’album…",
  "apropos.demo":
    "Démo en ligne. Photos : voir la page des crédits. Police : Source Sans 3, SIL Open Font License 1.1. Aucune police du système n’est embarquée.",
};

export const DEMO_EN: typeof DEMO_FR = {
  "bandeau.texte": "Demo: nothing is saved",
  "bandeau.lien": "Download Booklibre",
  "bandeau.credit": "Photos: {auteur}, {licence}",
  "bandeau.credits": "credits",
  "refus.titre": "Available in the app",
  "refus.lien": "Download Booklibre",
  "refus.export":
    "Exporting the print-ready PDF is done in the Booklibre app, on your computer. The demo produces no file. Download: booklibre.app",
  "refus.commande":
    "Ordering the printed album is done in the Booklibre app. Nothing leaves this page. Download: booklibre.app",
  "refus.recomposer":
    "Recomposing the album or changing its format needs the composition engine, which is in the app. Download: booklibre.app",
  "refus.remplir":
    "Filling the empty cells needs the composition engine, which is in the app. Download: booklibre.app",
  "refus.album":
    "Creating an album or opening another one is done in the app, from your own photos: the demo reads no file on your computer. Download: booklibre.app",
  "refus.signaler":
    "Reporting a problem is done from the app, which attaches what the report needs. Download: booklibre.app",
  "refus.stockage": "Album storage only exists in the app. Download: booklibre.app",
  "refus.origine":
    "Handing a spread back to automatic reads the original composition, which only the app keeps. Download: booklibre.app",
  "mobile.titre": "The demo is made for a computer",
  "mobile.texte":
    "The album editor needs a large screen and a mouse. Download Booklibre for Mac and Windows.",
  "mobile.telecharger": "Download Booklibre",
  "fidele.demo": "Faithful preview: the demo’s saved PDF, without your edits",
  "chargement": "Opening the album…",
  "apropos.demo":
    "Online demo. Photos: see the credits page. Typeface: Source Sans 3, SIL Open Font License 1.1. No system font is embedded.",
};

export type CleDemo = keyof typeof DEMO_FR;
export type Quoi =
  | "export"
  | "commande"
  | "recomposer"
  | "remplir"
  | "album"
  | "signaler"
  | "stockage"
  | "origine";

export function texteDemo(cle: CleDemo, l: Lang = langue(), params?: Record<string, string>): string {
  const texte = (l === "en" ? DEMO_EN : DEMO_FR)[cle];
  return params ? texte.replace(/\{(\w+)\}/g, (m, k) => params[k] ?? m) : texte;
}

/** Le manifeste que `scripts/demo-enregistrer.py` écrit : chaque requête
 *  rejouable (chemin et paramètres) vers un fichier de `demo-data/`. */
export type Manifeste = {
  version: 1;
  routes: Record<string, { f: string; t: string }>;
};

/** La clé d'une requête : le chemin et ses paramètres triés, valeurs
 *  décodées. Écrite à l'identique par le script d'enregistrement. */
export function cleDeRoute(chemin: string, params: URLSearchParams): string {
  const paires = [...params.entries()].sort(([a, x], [b, y]) => (a === b ? (x < y ? -1 : x > y ? 1 : 0) : a < b ? -1 : 1));
  return [chemin, ...paires.map(([k, v]) => `${k}=${v}`)].join("\n");
}

export type EtatDemo = { album: unknown };

export type Reponse = {
  status: number;
  type: string;
  /** Un fichier de `demo-data/`, ou bien un corps déjà écrit. */
  fichier?: string;
  corps?: string;
  /** Le fichier est l'album ouvert : on y remet l'album tenu en mémoire. */
  fusionAlbum?: boolean;
};

const TEXTE = "text/plain; charset=utf-8";

/** Ce qui demande le moteur, par chemin : la phrase qui le dit. */
const REFUS: [RegExp, Quoi][] = [
  [/^\/__dev\/recomposer$/, "recomposer"],
  [/^\/__dev\/preparer(-commande)?$/, "export"],
  [/^\/__dev\/prevol$/, "export"],
  [/^\/__dev\/remplir$/, "remplir"],
  [/^\/__dev\/survol$/, "album"],
];

/**
 * La réponse de la démo à une requête du harnais, sans rien lire : le
 * chemin du fichier à servir, ou le corps. Pure, donc testée sans navigateur.
 */
export function repondre(
  manifeste: Manifeste,
  methode: string,
  url: URL,
  corps: string | undefined,
  etat: EtatDemo,
  l: Lang,
): Reponse {
  const chemin = url.pathname;
  const refus = (quoi: Quoi): Reponse => ({
    status: 501,
    type: TEXTE,
    corps: texteDemo(`refus.${quoi}`, l),
  });

  if (methode === "POST" && chemin === "/__dev/album") {
    try {
      etat.album = JSON.parse(corps ?? "");
      return { status: 200, type: TEXTE, corps: "ok" };
    } catch (e) {
      return { status: 500, type: TEXTE, corps: String(e) };
    }
  }
  // L'aperçu de couverture est déjà dans les enregistrements.
  if (methode === "POST" && chemin === "/__dev/couverture") {
    return { status: 200, type: TEXTE, corps: "ok" };
  }
  // Le faux relais : aucun relais compilé, donc Commander ne parle à personne.
  if (chemin.startsWith("/__dev/relais/")) {
    const action = chemin.slice("/__dev/relais/".length);
    if (action === "pays" || action === "tarifs") return { status: 200, type: "application/json", corps: "null" };
    if (action === "tarifs-offerts") return { status: 200, type: "application/json", corps: "false" };
    return refus("commande");
  }
  for (const [motif, quoi] of REFUS) if (motif.test(chemin)) return refus(quoi);

  if (methode !== "GET") return { status: 405, type: TEXTE, corps: `${methode} ${chemin} : absent de la démo` };
  const route = manifeste.routes[cleDeRoute(chemin, url.searchParams)];
  if (!route) return { status: 404, type: TEXTE, corps: `${chemin}${url.search} : absent de la démo` };
  return {
    status: 200,
    type: route.t,
    fichier: route.f,
    fusionAlbum: chemin === "/__dev/album" && etat.album !== null ? true : undefined,
  };
}

// ---- le refus à l'écran -------------------------------------------------

type Ecouteur = (quoi: Quoi | null) => void;
const ecouteurs = new Set<Ecouteur>();

/** Ouvre la boîte « disponible dans l'app » pour cette fonction. */
export function demoRefus(quoi: Quoi): void {
  ecouteurs.forEach((f) => f(quoi));
}

export function surRefus(f: Ecouteur): () => void {
  ecouteurs.add(f);
  return () => ecouteurs.delete(f);
}

// ---- l'ordre des vignettes ----------------------------------------------

/**
 * Une file qui ne laisse passer que `max` tâches à la fois, et sert la
 * dernière arrivée d'abord. À l'arrivée, la légende de qualité demande les
 * vignettes de tout l'album pour juger de leur exposition ; dans l'app elles
 * sortent du disque, en ligne elles passent par le réseau, et la planche à
 * l'écran, demandée ensuite, attendait les quatre-vingts autres. Dernière
 * arrivée, première servie : ce qu'on regarde passe devant.
 */
export type File = {
  <T>(tache: () => Promise<T>, cle?: string): Promise<T>;
  /** Remet sur le dessus de la pile les tâches en attente de ces clés, la
   *  première nommée servie d'abord. `loadThumb` ne redemande jamais une
   *  vignette déjà en route : la planche à l'écran ne peut passer devant
   *  la légende qu'en le disant. */
  devant(cles: string[]): void;
};

export function fileDernierArrive(max: number): File {
  const attente: { cle?: string; go: () => void }[] = [];
  let actives = 0;
  const suivante = () => {
    if (actives >= max) return;
    const t = attente.pop();
    if (!t) return;
    actives++;
    t.go();
  };
  const file = (<T,>(tache: () => Promise<T>, cle?: string) =>
    new Promise<T>((resoudre, rejeter) => {
      attente.push({
        cle,
        go: () => {
          tache()
            .then(resoudre, rejeter)
            .finally(() => {
              actives--;
              suivante();
            });
        },
      });
      suivante();
    })) as File;
  file.devant = (cles) => {
    for (const cle of [...cles].reverse()) {
      const i = attente.findIndex((t) => t.cle === cle);
      if (i >= 0) attente.push(...attente.splice(i, 1));
    }
  };
  return file;
}

/** Les vignettes de la planche à l'écran et de ses voisines, à servir avant
 *  le reste. Sans effet hors de la démo installée. */
export function prioriserVignettes(srcs: string[]): void {
  fileCourante?.devant(srcs);
}

let fileCourante: File | null = null;

// ---- l'installation -----------------------------------------------------

let manifesteCourant: Manifeste | null = null;
const etat: EtatDemo = { album: null };

/** L'album tel qu'il est dans les enregistrements, jamais celui de la
 *  séance : « Revenir à l'ouverture » le relit. */
export async function albumDOrigine(): Promise<unknown> {
  const m = manifesteCourant;
  const route = m?.routes[cleDeRoute("/__dev/album", new URLSearchParams())];
  if (!route) throw new Error("démo sans album");
  const r = await fetch(`${import.meta.env.BASE_URL}demo-data/${route.f}`);
  return (await r.json()).album;
}

/**
 * Remplace `window.fetch`. Les `/__dev/*` sortent du manifeste ; tout le reste
 * du même site passe tel quel (le worker de pdf.js, les fichiers de la démo) ;
 * ce qui irait ailleurs sur le réseau est refusé : la démo ne parle à
 * personne.
 */
export async function installDemo(): Promise<void> {
  const natif = window.fetch.bind(window);
  const base = import.meta.env.BASE_URL;
  manifesteCourant = (await (await natif(`${base}demo-data/manifest.json`)).json()) as Manifeste;
  const manifeste = manifesteCourant;
  const vignettes = fileDernierArrive(6);
  fileCourante = vignettes;

  window.fetch = async (entree, init) => {
    const requete = entree instanceof Request ? entree : null;
    const url = new URL(requete?.url ?? String(entree), window.location.href);
    if (url.origin !== window.location.origin && /^https?:$/.test(url.protocol)) {
      throw new TypeError("démo : aucune requête hors de la page");
    }
    if (!url.pathname.startsWith("/__dev/")) return natif(entree, init);

    const methode = (init?.method ?? requete?.method ?? "GET").toUpperCase();
    const corps = typeof init?.body === "string" ? init.body : requete ? await requete.text() : undefined;
    const r = repondre(manifeste, methode, url, corps, etat, langue());
    if (r.fichier === undefined) {
      return new Response(r.corps ?? "", { status: r.status, headers: { "Content-Type": r.type } });
    }
    const lire = () => natif(`${base}demo-data/${r.fichier}`);
    // Lue en entier dans la file : une place ne se rend qu'octets reçus.
    const f =
      url.pathname === "/__dev/thumb"
        ? await vignettes(async () => {
            const brut = await lire();
            return new Response(await brut.arrayBuffer(), { status: brut.status });
          }, url.searchParams.get("src") ?? undefined)
        : await lire();
    if (!f.ok) return new Response(`${r.fichier} : absent de la démo`, { status: 404 });
    if (r.fusionAlbum) {
      const ouvert = (await f.json()) as Record<string, unknown>;
      return new Response(JSON.stringify({ ...ouvert, album: etat.album }), {
        status: 200,
        headers: { "Content-Type": r.type },
      });
    }
    return new Response(f.body, { status: 200, headers: { "Content-Type": r.type } });
  };
}
