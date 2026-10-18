// La couleur d'un objet libre, et l'encre du texte sur le fond de la
// couverture, côté écran.
//
// Le moteur lit `#rrggbb` (`couleur.rs::parse`) et retombe sur un défaut pour
// tout le reste : l'encre de texte du livre pour un bloc, le noir pour un
// ornement. L'écran fait exactement pareil, sinon un bloc à la couleur
// abîmée à la main serait rouge ici et gris sur le papier.
//
// **Les deux encres par défaut sont des miroirs de `pdf.rs`**, et
// `couleur.test.ts` les relit dans le source Rust : une copie tenue à la
// main serait fausse au premier réglage d'encre.

import { Objet } from "./album";

/** `pdf.rs::TEXT_INK`, l'encre du texte d'un livre, composantes de 0 à 1. */
export const TEXT_INK: readonly [number, number, number] = [0.2, 0.19, 0.16];

/** Trois composantes de 0 à 1 en `#rrggbb`, comme `couleur::parse` les relira. */
export function versHex(rgb: readonly [number, number, number]): string {
  return (
    "#" +
    rgb
      .map((c) =>
        Math.round(Math.min(Math.max(c, 0), 1) * 255)
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")
  );
}

/** L'encre du livre, ce que vaut un bloc sans couleur. */
export const ENCRE_LIVRE = versHex(TEXT_INK);

/** `pdf.rs::ORNEMENT_INK`, ce que vaut un ornement sans couleur. */
export const ENCRE_ORNEMENT = "#000000";

/** La grammaire de `couleur::parse` : un dièse et six hexadécimaux. */
export function couleurValide(s: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(s);
}

// ---- le texte posé sur le fond de la couverture (A4) ----------------------

/** `cover.rs::PAPER`, le blanc papier du texte de couverture. */
export const PAPIER: readonly [number, number, number] = [0.99, 0.97, 0.94];

/** `couleur.rs::CONTRASTE_MIN`, le seuil de WCAG pour un texte courant. */
export const CONTRASTE_MIN = 4.5;

/** `#rrggbb` en trois composantes de 0 à 1 ; null hors grammaire. */
export function versRgb(hex: string): [number, number, number] | null {
  if (!couleurValide(hex)) return null;
  return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) as [number, number, number];
}

/** `couleur.rs::luminance`, la luminance relative de WCAG 2. */
export function luminance(rgb: readonly [number, number, number]): number {
  const lin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2]);
}

/** `couleur.rs::contraste`, de 1 à 21. */
export function contraste(
  a: readonly [number, number, number],
  b: readonly [number, number, number],
): number {
  const [la, lb] = [luminance(a), luminance(b)];
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/** L'encre du texte de couverture posé sur `fond`, comme `cover.rs` la
 *  choisit (`couleur::encre_lisible`) : l'encre du livre tant qu'elle tient
 *  4,5:1, sinon la plus contrastée d'elle et du blanc papier. Sans fond, ou
 *  sur un fond hors grammaire, l'encre du livre, comme au PDF. */
export function encreSurFond(fond: string | null | undefined): string {
  const f = fond ? versRgb(fond) : null;
  if (!f) return ENCRE_LIVRE;
  const ce = contraste(f, TEXT_INK);
  return ce >= CONTRASTE_MIN || ce >= contraste(f, PAPIER) ? ENCRE_LIVRE : versHex(PAPIER);
}

/** La couleur dans laquelle un objet s'imprime. */
export function couleurDe(objet: Objet): string {
  if (objet.couleur !== undefined && couleurValide(objet.couleur)) return objet.couleur;
  return objet.type === "texte" ? ENCRE_LIVRE : ENCRE_ORNEMENT;
}

/** Les six pastilles du popover. `hex: null` est l'encre du livre : la
 *  choisir retire la couleur de l'objet, qui retombe sur son défaut. Les
 *  deux gris sont des teintes de noir au sens de l'imprimeur : 40 % d'encre
 *  pour le clair, 70 % pour le soutenu. */
export const PASTILLES: readonly { cle: string; hex: string | null }[] = [
  { cle: "livre", hex: null },
  { cle: "noir", hex: "#000000" },
  { cle: "blanc", hex: "#ffffff" },
  { cle: "gris40", hex: "#999999" },
  { cle: "gris70", hex: "#4d4d4d" },
  { cle: "terracotta", hex: "#b04a1f" },
];
