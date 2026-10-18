// Le panneau Recomposer : le format, le rythme, puis le bouton.
//
// Il remplace la confirmation qui précédait la recomposition, et il est le
// chemin ordinaire d'un changement de format : recomposer refait la mise en
// page des planches que personne n'a touchées, au nouveau format, et les
// planches éditées ou verrouillées passent par la bascule (`core::recomposition`).
// Le lien « Changer de format sans recomposer » mène à la bascule seule
// (`BasculeView.tsx`), qui garde toutes les planches.
//
// Rien ne part avant le bouton : Annuler ne touche rien. Ce que le panneau
// envoie au moteur est calculé par `recomposer.ts`, et n'est que ce qui a
// changé.

import { useEffect, useState } from "react";
import type { Album } from "./album";
import { loadThumb } from "./thumbs";
import { Dialogue } from "./Dialogue";
import { FormatCard } from "./FormatCard";
import { t } from "./i18n";
import {
  choixDeRecomposition,
  type FormatOffert,
  phraseGardees,
  rythmeDe,
  srcsDeLAlbum,
} from "./recomposer";
import VisionneuseFormat from "./VisionneuseFormat";
import type { VignetteVue } from "./visionneuse";

/** Combien de photos de l'album garnissent la visionneuse. */
const VIGNETTES = 8;

export function RecomposerView({
  album,
  formats,
  courant,
  gardees,
  onRecomposer,
  onSansRecomposer,
  onClose,
}: {
  album: Album;
  formats: FormatOffert[];
  /** Le nom du format de l'album, présélectionné. */
  courant: string;
  /** Les planches éditées ou verrouillées, que la recomposition garde. */
  gardees: number;
  onRecomposer: (choix: ReturnType<typeof choixDeRecomposition>) => void;
  onSansRecomposer: () => void;
  onClose: () => void;
}) {
  const [choisi, setChoisi] = useState(courant);
  const [aeree, setAeree] = useState(rythmeDe(album) === "aeree");
  const vignettes = useVignettes(album);

  const format = formats.find((f) => f.name === choisi) ?? formats[0];
  const formatChange =
    format !== undefined && (format.w !== album.trim_mm.w || format.h !== album.trim_mm.h);
  const phrase = phraseGardees(gardees, formatChange);

  return (
    <Dialogue
      titre={t("recomposer.titre")}
      onClose={onClose}
      className="bascule"
      panelClassName="bascule-panel recomposer-panel"
    >
      <p className="bascule-intro">{t("recomposer.intro")}</p>

      <div className="recomposer-corps">
        <div className="recomposer-choix">
          <h3 className="bascule-section">{t("recomposer.section.format")}</h3>
          <div className="format-cards">
            {formats.map((f) => (
              <FormatCard
                key={f.name}
                f={f}
                active={f.name === choisi}
                onPick={() => setChoisi(f.name)}
                note={f.imprimable ? undefined : t("recomposer.non_imprimable")}
              />
            ))}
          </div>

          <label className="recomposer-aeree">
            <input type="checkbox" checked={aeree} onChange={(e) => setAeree(e.target.checked)} />
            <span>
              <span className="recomposer-aeree-nom">{t("recomposer.aeree")}</span>
              <span className="recomposer-aeree-note">{t("recomposer.aeree.note")}</span>
            </span>
          </label>

          {phrase && <p className="recomposer-gardees">{phrase}</p>}
          <p className="recomposer-historique">{t("recomposer.historique")}</p>

          <p className="recomposer-actions">
            <button
              className="cta"
              disabled={format === undefined}
              onClick={() => format && onRecomposer(choixDeRecomposition(album, format, aeree))}
            >
              {t("recomposer.bouton")}
            </button>
            <button className="link" onClick={onClose}>
              {t("recomposer.annuler")}
            </button>
          </p>

          <p className="recomposer-sans">
            <button className="link" onClick={onSansRecomposer} title={t("recomposer.sans.note")}>
              {t("recomposer.sans")}
            </button>
            <span className="recomposer-sans-note">{t("recomposer.sans.note")}</span>
          </p>
        </div>

        {format && (
          <VisionneuseFormat format={format} vignettes={vignettes} titre={album.title} largeur={400} />
        )}
      </div>
    </Dialogue>
  );
}

/** Les vignettes de l'album pour la visionneuse, avec leur forme : elles
 *  arrivent une à une du cache, et la visionneuse se garnit à mesure. */
function useVignettes(album: Album): VignetteVue[] {
  const [vues, setVues] = useState<VignetteVue[]>([]);
  const cle = srcsDeLAlbum(album, VIGNETTES).join("\n");
  useEffect(() => {
    let vivant = true;
    const srcs = cle ? cle.split("\n") : [];
    Promise.all(
      srcs.map((src) =>
        loadThumb(src)
          .then(
            (url) =>
              new Promise<VignetteVue>((ok) => {
                const img = new Image();
                img.onload = () =>
                  ok({ url, ratio: img.naturalHeight ? img.naturalWidth / img.naturalHeight : undefined });
                img.onerror = () => ok({ url });
                img.src = url;
              }),
          )
          .catch(() => null),
      ),
    ).then((v) => {
      if (vivant) setVues(v.filter((x): x is VignetteVue => x !== null));
    });
    return () => {
      vivant = false;
    };
  }, [cle]);
  return vues;
}
