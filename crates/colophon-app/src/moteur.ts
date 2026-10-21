// Ce que le moteur dit en français, écrit en anglais pour l'écran anglais.
//
// Le moteur rédige en français : les défauts du prévol, ses notes, les
// réserves d'un profil, la phrase d'un format, le nom et la phrase d'un rythme
// ou d'une proposition. L'écran français les
// affiche **tels quels**, sans en recopier un mot ici : une seconde copie
// française finirait par dire autre chose que la ligne de commande. L'écran
// anglais, lui, les écrit depuis des codes que le moteur donne à côté de sa
// phrase (`Defaut.phrase`, `VarianteResume.densite`, les identifiants), jamais
// en relisant le français. Seuls les textes fixes, sans valeur à y poser — les
// notes et les réserves —, se reconnaissent à leur français exact.
//
// La liste des codes est `moteur.codes.json`, tenue à jour par un test du
// moteur (`codes.rs`). `moteur.test.ts` exige ici une phrase par code, et
// aucun trou qu'une valeur du moteur ne remplirait pas. Un code inconnu — un
// moteur plus récent que la fenêtre — rend le français du moteur plutôt
// qu'une phrase vide.

import type { Defaut, DensitePreset, VarianteResume } from "./bridge";
import { langue } from "./i18n";

/** La phrase d'un format, après son nom (« Square 21 × 21 · … »). */
export const FORMATS_EN: Record<string, string> = {
  "carre-21": "square 21 × 21, the usual album format",
  "carre-30": "square 30 × 30, a large coffee-table format",
  "portrait-a4": "A4 portrait",
  "paysage-a4": "A4 landscape",
  "paysage-28x21": "landscape 28 × 21",
  "portrait-20x25": "portrait 20 × 25, close to 8 × 10 inches",
};

export const DENSITES_EN: Record<string, { nom: string; about: string }> = {
  aeree: {
    nom: "Airy",
    about:
      "One or two photos per spread, often a single large one. Fewer photos kept, each one bigger.",
  },
  equilibree: {
    nom: "Balanced",
    about: "Two to four photos, with a mosaic now and then. The default pace.",
  },
  dense: {
    nom: "Dense",
    about: "Mosaics of four to eight photos. Many more photos kept, each one smaller.",
  },
};

/** Les propositions qui ne sont pas un rythme. « autre-rythme » prend le nom
 *  et la phrase de son rythme, comme dans le moteur. */
export const PROPOSITIONS_EN: Record<string, { nom: string; about: string }> = {
  demandee: { nom: "As asked", about: "" },
  resserree: {
    nom: "Shorter",
    about:
      "A third fewer spreads, so fewer photos kept. A book you leaf through in one go, and cheaper to print.",
  },
};

/** Les phrases du prévol, par code. Les trous sont les valeurs que le moteur
 *  pose dans `phrase.valeurs`, déjà formatées. En minuscule initiale comme
 *  le français du moteur : l'écran les pose après « Spread 4 ». */
export const PHRASES_EN: Record<string, string> = {
  format_sans_produit:
    "{imprimeur} does not print this format: none of its books measures {w_mm} × {h_mm} mm",
  "format_sans_produit.choisir": "choose one of the formats it prints in Format",
  "pagination.multiple":
    "the file has {pages} pages, and {imprimeur} only binds page counts in multiples of {pas}",
  "pagination.bornes":
    "the file has {pages} pages, outside {imprimeur}’s range ({min} to {max})",
  "pagination.decocher_colophon":
    "untick the colophon page in Send: it counts as two pages, and without it the count comes out right",
  "pagination.decocher_garde":
    "untick the half-title in Send: it counts as two pages, and without it the count comes out right",
  "pagination.ajouter":
    "add spreads or choose an airier pace: a spread counts as two pages, and at least {planches_min} are needed",
  "pagination.ajuster":
    "add or remove spreads: a spread counts as two pages, and between {planches_min} and {planches_max} are needed",
  "imposition.objet":
    "{imprimeur} binds one PDF page per book page, and the left page of the first spread is not printed: it holds one object that would be lost",
  "imposition.objets":
    "{imprimeur} binds one PDF page per book page, and the left page of the first spread is not printed: it holds {objets} objects that would be lost",
  "imposition.garde":
    "move what is on the left page, or add the half-title in Send: it goes first, and its left page stays blank",
  fond_perdu:
    "the album is composed with {album_mm} mm of bleed, {imprimeur} asks for {requis_mm}",
  "fond_perdu.recomposer":
    "recompose the album at the profile’s bleed: the bleed is set when the album is composed",
  espace: "{imprimeur} prints in CMYK {intent} and Booklibre only exports RGB",
  "espace.choisir": "choose a profile that accepts RGB, or ask the printer to convert",
  conformite:
    "{imprimeur} asks for PDF/X-4 and the file does not declare itself as such: the fonts are embedded, but the OutputIntent and the XMP metadata are still missing",
  "conformite.choisir":
    "choose a profile without PDF/X conformance, or ask the printer whether they accept a plain RGB PDF",
  resolution: "{src} would print at {ppi} ppi in this cell, {imprimeur} requires {min_ppi}",
  "resolution.case": "put it in a smaller cell, replace it, or remove it",
  zone_sure:
    "the caption sits {marge_mm} mm from the trim, {imprimeur} keeps {sure_mm} mm clear",
  "zone_sure.retirer":
    "remove this spread’s caption if you would rather not take the risk: its baseline comes from the format’s margin, and neither shortening it nor changing the layout moves it",
  objet_coupe: "free object no. {n} runs {depasse_mm} mm past the trim: the cut goes through it",
  "objet_coupe.deplacer": "move the object inside the page, or make it smaller",
  objet_pli: "free object no. {n} straddles the fold: the binding will cut it in two",
  "objet_pli.glisser":
    "slide the object to one side of the fold: the editor stops it there on its own",
  couverture_resolution:
    "{src} would print at {ppi} ppi on the {w_mm} × {h_mm} mm cover, {imprimeur} requires {min_ppi}",
  "couverture_resolution.choisir":
    "choose a higher-resolution photo for the cover: the sheet is larger than a page, so it needs more pixels",
  "couverture_vide.titre":
    "the cover has no photo: it would print blank, with only its title",
  "couverture_vide.dossier":
    "the cover has no photo: it would print blank, with only the folder’s name",
  "couverture_vide.choisir": "choose a photo in Cover",
  quatrieme_deborde:
    "{n} line(s) of the back cover do not fit on the page: they leave the panel, run onto the photo or do not print",
  "quatrieme_deborde.raccourcir":
    "shorten the back-cover text, or make it or its photo smaller",
  quatrieme_zone_sure:
    "the back-cover text or photo enters the {sure_mm} mm safety margin {imprimeur} asks for",
  "quatrieme_zone_sure.reduire": "make the back-cover text smaller, or break the overlong word",
  quatrieme_resolution:
    "{src} would print at {ppi} ppi on the back cover, {imprimeur} requires {min_ppi}",
  "quatrieme_resolution.choisir":
    "choose a higher-resolution photo for the back cover, or a smaller frame",
  dos_nu:
    "the spine is {dos_mm} mm wide: under {min_mm} mm it carries no title, so it will come out bare",
  "dos_nu.ajouter":
    "add spreads: the spine thickens with the book, and the title goes on it by itself",
  dos: "spine width calculated at {dos_mm} mm with a provisional coefficient, not confirmed by {imprimeur}",
  "dos.confirmer":
    "confirm the formula with the printer before a print run, or measure it on the first album received",
  "fichier.illisible": "{fichier} sits next to the album and cannot be read: {pourquoi}",
  "fichier.page": "{fichier} has {n} page, {imprimeur} expects {attendues}",
  "fichier.pages": "{fichier} has {n} pages, {imprimeur} expects {attendues}",
  "fichier.taille":
    "{fichier}: page {page} measures {w_mm} × {h_mm} mm, {imprimeur} expects {attendu_w_mm} × {attendu_h_mm}",
  fichier_profil:
    "{fichier} was rendered for the {rendu} profile, but the file is expected by {profil} ({imprimeur}): it is another printer’s book",
  "fichier_perime.album":
    "{fichier} was rendered from another version of the album: fingerprint {avant} at render time, {maintenant} today",
  "fichier_perime.octets":
    "{fichier} is {octets} bytes, the render wrote {rendu}: the file has been replaced since",
  "fichier.refaire":
    "run {commande} --profil {profil} again to remake it, or remove it from the folder",
};

/** Les textes fixes du moteur — notes du rapport, réserves des profils —,
 *  reconnus mot pour mot. La clé est le français exact que donne
 *  `moteur.codes.json` : un mot changé dans le moteur fait rougir
 *  `moteur.test.ts` jusqu'à ce que l'anglais ait suivi. */
export const TEXTES_EN: Record<string, string> = {
  "conformité PDF/X-4 déclarée : polices incorporées, OutputIntent sRGB avec profil ICC, XMP et TrimBox vérifiés à chaque export. Le contrôle indépendant disponible est PDF/A-2b (veraPDF), qui couvre le même socle ; le verdict PDF/X-4 lui-même revient au prévol de l'imprimeur.":
    "PDF/X-4 conformance declared: fonts embedded, sRGB OutputIntent with its ICC profile, XMP and TrimBox checked at every export. The independent check available is PDF/A-2b (veraPDF), which covers the same ground; the PDF/X-4 verdict itself belongs to the printer’s own preflight.",
  "dossier de photos introuvable : la résolution effective n'a pas pu être vérifiée":
    "photo folder not found: the effective resolution could not be checked",
  "leur documentation prévient que la main du papier et le format du carton varient d'un imprimeur à l'autre : le dos calculé ici est une moyenne":
    "their documentation warns that paper bulk and board size vary from one printer to another: the spine calculated here is an average",
  "et ils ont répondu qu'on ne peut ni épingler un site de production, ni interroger le dos d'un produit avant commande : l'écart se mesurera au pied à coulisse sur l'album reçu":
    "and they answered that a production site cannot be pinned, nor a product’s spine looked up before ordering: the difference will be measured with calipers on the album received",
  "leur zone sûre est une recommandation à fourchette, « at least 7-10 mm », donnée à propos de la couverture : les 7 mm retenus en sont le bas, et aucun de leurs deux gabarits n'en dessine une":
    "their safe zone is a recommendation with a range, “at least 7-10 mm”, given about the cover: the 7 mm kept here is its low end, and neither of their two templates draws one",
  "profil non testé : aucune commande passée chez eux":
    "untested profile: no order placed with them",
  "cotes du cartonné inconnues : rempli, débord et mors sont à zéro faute de gabarit, donc la feuille sortie ici est une couverture souple à plat et non un boîtier":
    "hardcover dimensions unknown: turn-in, overhang and hinge are zero for lack of a template, so the sheet produced here is a flat softcover, not a case",
  "bornes du SKU carré 210 × 210 : le 294 × 294 s'arrête à 298 pages, un autre produit aura d'autres bornes":
    "bounds of the square 210 × 210 SKU: the 294 × 294 stops at 298 pages, another product will have other bounds",
  "profil de repli, jamais commandé : l'intérieur sort désormais page par page, mais aucun album n'est encore passé par leur presse":
    "fallback profile, never ordered: the interior now comes out page by page, but no album has gone through their press yet",
  "ils refusent tout fond perdu et l'album en porte trois millimètres sur les bords extérieurs : leur massicot les prendra, la page finie est la même":
    "they refuse any bleed and the album carries three millimetres on the outer edges: their guillotine will trim them off, and the finished page is the same",
  "ils recommandent un contrôle X-4 en FOGRA39 tout en demandant des images RVB : notre intention de sortie reste sRGB":
    "they recommend an X-4 check in FOGRA39 while asking for RGB images: our output intent stays sRGB",
};

/** Les noms des trous d'une phrase, dans l'ordre où ils viennent. */
export function trous(modele: string): string[] {
  return [...modele.matchAll(/\{(\w+)\}/g)].map((m) => m[1]);
}

/** Une phrase anglaise remplie, ou null si le code est inconnu ou qu'un trou
 *  n'a pas de valeur : une phrase à moitié remplie dirait moins que le
 *  français du moteur. */
function remplir(code: string | undefined, valeurs: Record<string, string>): string | null {
  const modele = code === undefined ? undefined : PHRASES_EN[code];
  if (modele === undefined) return null;
  if (trous(modele).some((n) => !(n in valeurs))) return null;
  return modele.replace(/\{(\w+)\}/g, (_, n: string) => valeurs[n]);
}

/** La cause ou le remède d'un défaut, dans la langue de l'écran. */
export function phraseDefaut(d: Defaut, quoi: "cause" | "remede"): string {
  if (langue() === "fr" || d.phrase === undefined) return d[quoi];
  return remplir(d.phrase[quoi], d.phrase.valeurs ?? {}) ?? d[quoi];
}

/** Une note du rapport ou une réserve d'un profil, dans la langue de
 *  l'écran. Un texte que la table ne connaît pas reste en français. */
export function texteMoteur(fr: string): string {
  if (langue() === "fr") return fr;
  return TEXTES_EN[fr] ?? fr;
}

/** La phrase d'un format, après son nom. */
export function aboutFormat(f: { name: string; about: string }): string {
  if (langue() === "fr") return f.about;
  return FORMATS_EN[f.name] ?? f.about;
}

export function nomDensite(d: DensitePreset): string {
  if (langue() === "fr") return d.nom;
  return DENSITES_EN[d.id]?.nom ?? d.nom;
}

export function aboutDensite(d: DensitePreset): string {
  if (langue() === "fr") return d.about;
  return DENSITES_EN[d.id]?.about ?? d.about;
}

/** Le nom et la phrase d'une proposition : celle de son rythme quand c'est
 *  l'autre rythme, la sienne sinon. Un moteur plus ancien, qui ne donne pas
 *  le rythme, garde son français. */
function enProposition(v: VarianteResume): { nom: string; about: string } | undefined {
  if (v.id === "autre-rythme") return v.densite === undefined ? undefined : DENSITES_EN[v.densite];
  return PROPOSITIONS_EN[v.id];
}

export function nomProposition(v: VarianteResume): string {
  if (langue() === "fr") return v.nom;
  return enProposition(v)?.nom ?? v.nom;
}

export function aboutProposition(v: VarianteResume): string {
  if (langue() === "fr") return v.about;
  return enProposition(v)?.about ?? v.about;
}
