// La logique pure du panneau Recomposer (`RecomposerView.tsx`) : quels
// formats il offre, ce qu'il envoie au moteur, comment il dit ce qui est
// gardé. Le panneau ne fait que dessiner ce que ces fonctions rendent.

import type { Album } from "./album";
import type { FormatPreset } from "./bridge";
import { t } from "./i18n";

export type FormatOffert = FormatPreset & {
  /** Faux pour un format que Colophon ne sait pas faire imprimer (28 × 21,
   *  taille libre) : il n'est offert que parce que l'album l'a déjà. */
  imprimable: boolean;
};

const memeTaille = (a: { w: number; h: number }, b: { w: number; h: number }) =>
  a.w === b.w && a.h === b.h;

/**
 * Les cinq formats imprimables, ceux de l'écran de création
 * (`commande::formats_imprimables`), plus le format de l'album quand il n'en
 * est pas : un album en 28 × 21 ou en taille libre le garde, coché, et
 * recomposer sans y toucher le garde. Le courant est rendu par son nom.
 */
export function formatsDeRecomposition(
  tous: FormatPreset[],
  imprimables: string[] | null,
  courant: { w: number; h: number },
): { formats: FormatOffert[]; courant: string } {
  const formats: FormatOffert[] = tous
    .filter((f) => imprimables === null || imprimables.includes(f.name))
    .map((f) => ({ ...f, imprimable: true }));
  const present = formats.find((f) => memeTaille(f, courant));
  if (present) return { formats, courant: present.name };
  const connu = tous.find((f) => memeTaille(f, courant));
  const ajoute: FormatOffert = connu
    ? { ...connu, imprimable: false }
    : {
        name: `${courant.w}x${courant.h}`,
        w: courant.w,
        h: courant.h,
        about: `${courant.w} × ${courant.h} mm`,
        imprimable: false,
      };
  return { formats: [...formats, ajoute], courant: ajoute.name };
}

/** Le rythme d'un album, `equilibree` quand le fichier ne le dit pas (le
 *  moteur ne l'écrit pas quand c'est le défaut). */
export function rythmeDe(album: Album): string {
  return album.densite ?? "equilibree";
}

/**
 * Ce que le panneau demande au moteur : seulement ce qui change. Rien de
 * changé, rien d'envoyé, et le moteur recompose alors exactement comme avant
 * que le panneau existe. La case « moins de photos » ne parle que si on l'a
 * changée d'état : un album composé dense à la CLI reste dense tant que
 * personne n'y touche.
 */
export function choixDeRecomposition(
  album: Album,
  format: { w: number; h: number },
  aeree: boolean,
): { format?: { w: number; h: number }; densite?: string } {
  const choix: { format?: { w: number; h: number }; densite?: string } = {};
  if (!memeTaille(format, album.trim_mm)) choix.format = { w: format.w, h: format.h };
  if (aeree !== (rythmeDe(album) === "aeree")) choix.densite = aeree ? "aeree" : "equilibree";
  return choix;
}

/** Les planches qu'une recomposition garde : éditées ou verrouillées, la
 *  règle de `Spread::pinned`. */
export function planchesGardees(album: Album): number {
  return album.spreads.filter((s) => s.edited || s.locked).length;
}

/**
 * La phrase de ce qui est gardé, chiffrée. Au même format, les planches
 * restent telles quelles ; à un autre, elles gardent leurs photos mais leur
 * mise en page peut se replier, et le dire « telles quelles » serait faux.
 */
export function phraseGardees(n: number, formatChange: boolean): string {
  if (n === 0) return "";
  if (formatChange) {
    return n === 1
      ? t("recomposer.gardees.format.un")
      : t("recomposer.gardees.format", { n });
  }
  return n === 1 ? t("recomposer.gardees.un") : t("recomposer.gardees", { n });
}

/**
 * Les photos qui garnissent la visionneuse : la première de planches prises
 * à intervalles réguliers dans tout le livre, pour qu'elle montre le voyage
 * et pas son seul début ; puis, sur un petit album, les autres photos.
 */
export function srcsDeLAlbum(album: Album, n: number): string[] {
  const avec = album.spreads.filter((s) => s.slots.length > 0);
  const vues = new Set<string>();
  const out: string[] = [];
  const prendre = (src: string | undefined) => {
    if (src === undefined || vues.has(src) || out.length >= n) return;
    vues.add(src);
    out.push(src);
  };
  const pas = Math.max(1, avec.length / n);
  for (let i = 0; i < n && Math.floor(i * pas) < avec.length; i++) {
    prendre(avec[Math.floor(i * pas)].slots[0]?.src);
  }
  for (const s of avec) for (const sl of s.slots) prendre(sl.src);
  return out;
}
