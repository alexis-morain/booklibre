// La légende de la qualité, au pied du livre : « Qualité : 2 trop petites ·
// 1 sombre · 3 peu nettes ». Chaque compte mène à la première planche
// concernée, puis à la suivante à chaque clic. Rien quand tout est bon : pas
// de note, pas d'étoiles, Colophon signale ce qui imprimera mal.
//
// Les alertes sont celles des cases (`badgesDe`, seule règle), calculées
// sur tout l'album. « Sombre » lit les pixels de la vignette : son compte
// grandit à mesure que les vignettes se décodent.

import { useEffect, useMemo, useRef, useState } from "react";
import { Album, slotsFor, spreadGeometry } from "./album";
import { t } from "./i18n";
import { Alerte, badgesDe, imageDe, surImage, useReleve } from "./photos";
import { CodeAlerte, legendeQualite, motDeLigne, plancheSuivante } from "./qualite";
import { reglagePose, useReglages } from "./reglages";
import { SignesAlerte } from "./SignesAlerte";

/** Les alertes de chaque case de chaque planche, par la règle des cases. */
export function useAlertesDeLAlbum(album: Album): Alerte[][][] {
  const releve = useReleve();
  useReglages();
  // Une vignette décodée peut faire naître un « sombre » : on recompte.
  const [, setDecodees] = useState(0);
  useEffect(() => surImage(() => setDecodees((n) => n + 1)), []);
  const geom = useMemo(() => spreadGeometry(album), [album]);
  return album.spreads.map((spread) => {
    const rects = slotsFor(spread.template, spread.slots.length, geom);
    return spread.slots.map((slot, i) => {
      const r = rects[i];
      if (!r) return [];
      return badgesDe(
        slot.src,
        imageDe(slot.src),
        r,
        1,
        slot.zoom ?? 1,
        reglagePose(slot.src),
        releve,
      ).alertes;
    });
  });
}

/** Les alertes des cases d'une seule planche, par la même règle : le panneau
 *  de propriétés n'a pas besoin de compter tout l'album à chaque rendu. */
export function useAlertesDeLaPlanche(album: Album, index: number): Alerte[][] {
  const releve = useReleve();
  useReglages();
  const [, setDecodees] = useState(0);
  useEffect(() => surImage(() => setDecodees((n) => n + 1)), []);
  const geom = useMemo(() => spreadGeometry(album), [album]);
  const spread = album.spreads[index];
  if (!spread) return [];
  const rects = slotsFor(spread.template, spread.slots.length, geom);
  return spread.slots.map((slot, i) => {
    const r = rects[i];
    if (!r) return [];
    return badgesDe(slot.src, imageDe(slot.src), r, 1, slot.zoom ?? 1, reglagePose(slot.src), releve).alertes;
  });
}

export function LegendeQualite({
  album,
  index,
  onSeek,
}: {
  album: Album;
  index: number;
  onSeek: (i: number) => void;
}) {
  const lignes = legendeQualite(useAlertesDeLAlbum(album));
  /** Où le dernier clic sur chaque compte a mené. */
  const derniere = useRef(new Map<CodeAlerte, number>());
  if (lignes.length === 0) return null;
  return (
    <div className="foot-qualite" role="group" aria-label={t("qualite.groupe")}>
      <span className="foot-qualite-titre">{t("qualite.titre")}</span>
      {lignes.map((l, k) => (
        <span key={l.code} className="foot-qualite-ligne">
          {k > 0 && (
            <span className="foot-qualite-sep" aria-hidden="true">
              ·
            </span>
          )}
          <button
            className="link foot-qualite-compte"
            title={t("qualite.voir")}
            onClick={() => {
              const cible = plancheSuivante(
                l.planches,
                index,
                derniere.current.get(l.code) ?? null,
              );
              derniere.current.set(l.code, cible);
              onSeek(cible);
            }}
          >
            <SignesAlerte alertes={[signeDe(l.code)]} enLigne muet />
            {motDeLigne(l)}
          </button>
        </span>
      ))}
    </div>
  );
}

/** Le signe d'une cause, sans le chiffre d'une case : la légende compte, elle
 *  ne mesure pas. */
function signeDe(code: CodeAlerte): Alerte {
  return code === "sous_resolution" ? { code, ppi: 0 } : { code };
}
