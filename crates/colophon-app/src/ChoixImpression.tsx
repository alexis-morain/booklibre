// « Comment l'imprimer ? » (1.1-f), et le prix qui en découle dans le pied.
//
// `ChoixImpression` est la question elle-même, posée sur l'accueil tant
// qu'elle n'a pas de réponse et rangée dans les Préférences ensuite : les
// deux disent la même chose avec les mêmes boutons. Le pays vient de la
// table des tarifs, donc il ne se demande qu'après « Par Booklibre » : rien
// ne part avant (C59). Les noms des pays sont ceux du système
// (`Intl.DisplayNames`), rien de géolocalisé.
//
// `PrixLivre` lit une case de la table : « À partir de 46 € livré », pour
// deux pages par planche. Il se tait sans table, sans pays livré, pour un
// format que Booklibre ne vend pas, au-delà de 200 pages : jamais un chiffre
// qui ne viendrait pas de la table.

import { useEffect, useState } from "react";
import type { Album } from "./album";
import { listFormats, type FormatPreset } from "./bridge";
import { prixDeCentimes } from "./commande";
import { t, useLangue } from "./i18n";
import {
  chargerOffre,
  formatDe,
  pagesDuLivre,
  prixPour,
  questionAPoser,
  setImpression,
  useImpression,
  useOffre,
  useTarifs,
} from "./impression";
import { saveurMas } from "./saveur";

/** Le nom d'un pays dans la langue de l'écran, ou son code. */
function nomDuPays(code: string, langue: string): string {
  try {
    return new Intl.DisplayNames([langue], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}

const AUTRE = "autre";

export function ChoixImpression() {
  const langue = useLangue();
  const choix = useImpression();
  const tarifs = useTarifs();
  const CHOIX: ["moi-meme" | "booklibre", string, string][] = [
    ["moi-meme", t("impression.moi"), t("impression.moi.note")],
    ["booklibre", t("impression.booklibre"), t("impression.booklibre.note")],
  ];
  const pays = tarifs.etat === "lus" ? tarifs.tarifs.pays : [];
  return (
    <div className="impression">
      <div className="prefs-langues" role="radiogroup" aria-label={t("impression.question")}>
        {CHOIX.map(([id, nom, note]) => (
          <button
            key={id}
            role="radio"
            aria-checked={choix?.choix === id}
            className={"prefs-langue" + (choix?.choix === id ? " active" : "")}
            title={note}
            onClick={() => setImpression({ choix: id, pays: id === "booklibre" ? (choix?.pays ?? null) : null })}
          >
            {nom}
          </button>
        ))}
      </div>
      {choix?.choix === "booklibre" && (
        <div className="impression-pays">
          {tarifs.etat === "lecture" && <p className="signaler-note">{t("impression.lecture")}</p>}
          {tarifs.etat === "indisponible" && <p className="signaler-note">{t("impression.indisponible")}</p>}
          {tarifs.etat === "lus" && (
            <label>
              {t("impression.pays")}{" "}
              <select
                value={choix.pays === null ? (pays.length ? "" : AUTRE) : pays.includes(choix.pays) ? choix.pays : AUTRE}
                onChange={(e) => setImpression({ choix: "booklibre", pays: e.target.value === AUTRE ? null : e.target.value })}
              >
                <option value="" disabled>
                  —
                </option>
                {pays.map((p) => (
                  <option key={p} value={p}>
                    {nomDuPays(p, langue)}
                  </option>
                ))}
                <option value={AUTRE}>{t("impression.pays.autre")}</option>
              </select>
            </label>
          )}
          {tarifs.etat === "lus" && choix.pays !== null && !pays.includes(choix.pays) && (
            <p className="signaler-note">{t("impression.pays.pas.livre")}</p>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * La question sur l'accueil : elle paraît quand ce build a des tarifs, hors
 * App Store, et que personne n'a répondu ; une fois ouverte elle reste le
 * temps de choisir le pays. « Moi-même » la referme.
 */
export function QuestionAccueil() {
  const offre = useOffre();
  const choix = useImpression();
  const [ouverte, setOuverte] = useState(false);
  useEffect(() => {
    void chargerOffre();
  }, []);
  useEffect(() => {
    if (questionAPoser(offre, choix, saveurMas())) setOuverte(true);
  }, [offre, choix]);
  if (!ouverte || choix?.choix === "moi-meme") return null;
  return (
    <section className="accueil-impression" aria-labelledby="accueil-impression-titre">
      <h2 className="recents-title" id="accueil-impression-titre">
        {t("impression.question")}
      </h2>
      <ChoixImpression />
      <p className="signaler-note">{t("impression.plus.tard")}</p>
    </section>
  );
}

let formats: Promise<FormatPreset[]> | null = null;

export function PrixLivre({ album }: { album: Album }) {
  const langue = useLangue();
  const choix = useImpression();
  const tarifs = useTarifs();
  const [presets, setPresets] = useState<FormatPreset[] | null>(null);
  useEffect(() => {
    if (tarifs.etat !== "lus") return;
    let vivant = true;
    formats ??= listFormats();
    formats.then((f) => vivant && setPresets(f), () => {});
    return () => {
      vivant = false;
    };
  }, [tarifs.etat]);
  if (tarifs.etat !== "lus" || choix?.choix !== "booklibre" || presets === null) return null;
  const pages = pagesDuLivre(album);
  const prix = prixPour(tarifs.tarifs, formatDe(presets, album.trim_mm), choix.pays, pages);
  if (prix === null) return null;
  return (
    <span className="foot-prix" title={t("prix.a.partir.titre", { pages })}>
      {t("prix.a.partir", { prix: prixDeCentimes(prix, langue) })}
    </span>
  );
}
