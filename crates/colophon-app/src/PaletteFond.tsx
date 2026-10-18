// Le choix du fond de la couverture et du dos (A4) : huit pastilles — blanc
// papier, crème, noir, cinq teintes tirées de la photo de couverture —, puis
// « Autre… », le sélecteur natif. Sous le livre dans Envoi, et dans l'éditeur
// de couverture.
//
// Un choix est un pas d'annulation (`edits.ts::setFond`), comme une
// retouche. Le sélecteur natif valide à `change`, jamais à `input` : glisser
// dans la roue empilerait cent pas — la règle d'`ObjetBloc`.

import { useEffect, useRef, useState } from "react";
import type { Album } from "./album";
import { couleurValide } from "./couleur";
import { nomDuFond, pastillesFond, teintesDe } from "./fond";
import { t } from "./i18n";
import { imageDe, surImage } from "./photos";

/** Le côté de l'échantillon lu dans la vignette : assez pour cinq teintes,
 *  assez peu pour ne rien coûter. */
const COTE = 96;

/** Les teintes déjà tirées, par source : la palette se rouvre sans relire. */
const memoire = new Map<string, string[]>();

function teintesDeLaPhoto(src: string): string[] | null {
  const deja = memoire.get(src);
  if (deja) return deja;
  const img = imageDe(src);
  if (!img || !img.naturalWidth || typeof document === "undefined") return null;
  const k = Math.min(1, COTE / Math.max(img.naturalWidth, img.naturalHeight));
  const w = Math.max(1, Math.round(img.naturalWidth * k));
  const h = Math.max(1, Math.round(img.naturalHeight * k));
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, w, h);
  const teintes = teintesDe(ctx.getImageData(0, 0, w, h).data, w, h);
  memoire.set(src, teintes);
  return teintes;
}

export function PaletteFond({
  album,
  onFond,
}: {
  album: Album;
  /** Un pas d'annulation. `null` : le blanc papier. */
  onFond: (fond: string | null) => void;
}) {
  const src = album.cover?.photo?.src ?? null;
  const [teintes, setTeintes] = useState<string[]>(() => (src ? memoire.get(src) ?? [] : []));
  useEffect(() => {
    if (!src) {
      setTeintes([]);
      return;
    }
    const lire = () => {
      const tt = teintesDeLaPhoto(src);
      if (tt) setTeintes(tt);
    };
    lire();
    return surImage((s) => {
      if (s === src) lire();
    });
  }, [src]);

  const brut = album.cover?.fond;
  const pose = brut && couleurValide(brut) ? brut.toLowerCase() : null;
  const pastilles = pastillesFond(teintes);
  const autre = pose !== null && !pastilles.some((p) => p.hex === pose);

  const natif = useRef<HTMLInputElement>(null);
  const onFondRef = useRef(onFond);
  onFondRef.current = onFond;
  useEffect(() => {
    const el = natif.current;
    if (!el) return;
    const surChange = () => onFondRef.current(el.value.toLowerCase());
    el.addEventListener("change", surChange);
    return () => el.removeEventListener("change", surChange);
  }, [pose]);

  return (
    <div className="fond">
      <div className="fond-pastilles" role="group" aria-label={t("fond.titre")}>
        {pastilles.map((p, i) => {
          const choisie = p.hex === pose;
          const nom =
            p.cle === "teinte"
              ? t("fond.teinte", { n: i - 2, nom: nomDuFond(p.hex) })
              : nomDuFond(p.hex);
          return (
            <button
              key={p.hex ?? "papier"}
              type="button"
              aria-pressed={choisie}
              className={"fond-pastille" + (choisie ? " actif" : "") + (p.hex === null ? " papier" : "")}
              style={p.hex ? { background: p.hex } : undefined}
              aria-label={nom}
              title={nom}
              onClick={() => onFond(p.hex)}
            />
          );
        })}
        <button
          type="button"
          aria-pressed={autre}
          className={"fond-autre" + (autre ? " actif" : "")}
          title={t("fond.autre.titre")}
          onClick={() => {
            const el = natif.current;
            if (!el) return;
            try {
              el.showPicker();
            } catch {
              el.click();
            }
          }}
        >
          {autre && <span className="fond-autre-temoin" style={{ background: pose ?? undefined }} />}
          {t("fond.autre")}
        </button>
        <input
          ref={natif}
          key={pose ?? "papier"}
          type="color"
          className="fond-natif"
          tabIndex={-1}
          aria-hidden="true"
          defaultValue={pose ?? "#ffffff"}
        />
      </div>
      <p className="fond-phrase">{t("fond.phrase")}</p>
    </div>
  );
}
