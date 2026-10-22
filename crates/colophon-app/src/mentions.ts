// Le réglage « sous les photos » côté fenêtre (1.2-b).
//
// Les mots viennent du moteur, toujours : `legende::mention` lit la photo, et
// la commande `mentions_photos` rend la table par src. Ce module ne fait que
// la poser. Il ne dérive rien lui-même, donc la fenêtre ne peut pas écrire
// sous une photo ce que le moteur n'y écrirait pas.

import { Album, Legendes, slotsFor, SpreadGeometry, Spread } from "./album";
import { legendeImprimee } from "./scene";

/** La table du moteur, et la clé du réglage qui l'a produite : une table ne
 *  s'applique qu'à un album de ce réglage-là, sinon un ⌘Z en arrière se
 *  verrait réécrire avec les mots d'un autre choix. */
export type TableMentions = { cle: string; textes: Record<string, string> };

export function cleDe(l: Legendes): string {
  return `${l.contenu}/${l.date}/${l.langue}`;
}

/** Les mentions de la table sur chaque case, ou aucune sans table. Rend
 *  l'album d'entrée tel quel quand rien ne change ; ne marque aucune
 *  planche `edited`, ce ne sont pas des mots de la main. */
function poser(album: Album, textes: Record<string, string> | null): Album {
  let change = false;
  const spreads = album.spreads.map((spread): Spread => {
    let touche = false;
    const slots = spread.slots.map((s) => {
      const voulue = textes?.[s.src];
      if ((s.mention ?? undefined) === voulue) return s;
      touche = true;
      const { mention: _, ...sans } = s;
      return voulue ? { ...sans, mention: voulue } : sans;
    });
    if (!touche) return spread;
    change = true;
    return { ...spread, slots };
  });
  return change ? { ...album, spreads } : album;
}

/** Choisir un réglage, ou « Rien » (`null`) : un seul album, donc un seul
 *  pas d'annulation. `table` est celle que le moteur a rendue pour `l`. */
export function poserMentions(
  album: Album,
  l: Legendes | null,
  table: TableMentions | null,
): Album {
  const textes = l && table && table.cle === cleDe(l) ? table.textes : null;
  const posees = poser(album, l ? textes ?? {} : null);
  const avant = album.legendes ? cleDe(album.legendes) : null;
  if ((l ? cleDe(l) : null) === avant) return posees;
  if (l) return { ...posees, legendes: l };
  const { legendes: _, ...sans } = posees;
  return sans;
}

/** Après toute édition : chaque case porte la mention de sa photo, jamais
 *  celle d'une autre. C'est ce qui donne sa date à une photo remplacée,
 *  ajoutée ou sauvée de la réserve. Sans réglage, ou avec la table d'un
 *  autre réglage, l'album passe tel quel. */
export function avecMentions(album: Album, table: TableMentions | null): Album {
  if (!album.legendes || !table || table.cle !== cleDe(album.legendes)) return album;
  return poser(album, table.textes);
}

/** Ce que la ligne d'état dit après un choix : les photos qui ont leurs mots
 *  sous elles, celles qui ne savent ni leur date ni leur lieu, et celles dont
 *  la case n'a pas de place dessous (une pleine page). */
export function bilanMentions(
  album: Album,
  g: SpreadGeometry,
): { posees: number; sansRien: number; sansPlace: number } {
  let posees = 0;
  let sansRien = 0;
  let sansPlace = 0;
  for (const spread of album.spreads) {
    const rects = slotsFor(spread.template, spread.slots.length, g);
    spread.slots.forEach((s, i) => {
      if (!s.mention) {
        sansRien++;
        return;
      }
      const rect = rects[i];
      const sans = { ...s, caption: undefined };
      if (rect && legendeImprimee(sans, rect, g)) posees++;
      else sansPlace++;
    });
  }
  return { posees, sansRien, sansPlace };
}
