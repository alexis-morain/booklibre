// Changer de format sans recomposer : le même album, mêmes planches, même
// ordre, mêmes photos, et seules les mises en page que le nouveau format
// trahirait se replient (`core::bascule`).
//
// On n'y arrive que par le lien « Changer de format sans recomposer » du
// panneau Recomposer : recomposer est le chemin ordinaire d'un changement de
// format, celui-ci est pour qui veut garder toutes ses planches. Le panneau
// occupe la place de Recomposer, il ne s'empile pas sur lui.
//
// Rien n'est écrit ici : le moteur rend un album, l'éditeur l'applique par
// son historique. ⌘Z l'annule comme n'importe quelle retouche, ⌘S le grave.

import { BasculeBilan, FormatPreset } from "./bridge";
import { Dialogue } from "./Dialogue";
import { t } from "./i18n";
import { aboutFormat } from "./moteur";

export function BasculeView({
  formats,
  courant,
  choisi,
  apercu,
  enCours,
  onChoisir,
  onAppliquer,
  onClose,
}: {
  formats: FormatPreset[];
  courant: { w: number; h: number };
  choisi: string | null;
  apercu: BasculeBilan | null;
  enCours: boolean;
  onChoisir: (f: FormatPreset) => void;
  onAppliquer: () => void;
  onClose: () => void;
}) {
  const memeFormat = (f: FormatPreset) => f.w === courant.w && f.h === courant.h;

  return (
    <Dialogue titre={t("bascule.titre")} onClose={onClose} className="bascule" panelClassName="bascule-panel">

        <p className="bascule-intro">{t("bascule.intro")}</p>

        <ul className="bascule-formats">

          {formats.map((f) => (
            <li key={f.name}>
              <button
                className={"bascule-format" + (choisi === f.name ? " choisi" : "")}
                onClick={() => onChoisir(f)}
                disabled={memeFormat(f) || enCours}
                aria-pressed={choisi === f.name}
              >
                <span className="bascule-format-nom">{aboutFormat(f)}</span>
                <span className="bascule-format-mm">
                  {f.w} × {f.h} mm
                  {memeFormat(f) ? ` · ${t("bascule.format.courant")}` : ""}
                </span>
              </button>
            </li>
          ))}
        </ul>

        {enCours && <p className="bascule-attente">{t("bascule.calcul")}</p>}

        {apercu && !enCours && (
          <section className="bascule-bilan">
            <h3>{t("bascule.bilan")}</h3>
            <p className="bascule-resume">
              {t("bascule.inchangees", {
                n: apercu.planches_inchangees,
                total: apercu.planches,
              })}
            </p>

            {/* La résolution d'abord : c'est le seul dégât qu'aucune main ne
                rattrape. Un gabarit se rechange d'un clic, une photo à court
                de pixels demande une autre photographie. */}
            {apercu.sous_resolution.length > 0 && (
              <div className="bascule-alerte">
                <p>{t("bascule.sous_resolution", { n: apercu.sous_resolution.length })}</p>
                <ul>
                  {apercu.sous_resolution.slice(0, 6).map((s) => (
                    <li key={`${s.planche}-${s.src}`}>
                      {t("bascule.ppi", {
                        planche: s.planche,
                        src: s.src,
                        avant: Math.round(s.ppi_avant),
                        apres: Math.round(s.ppi_apres),
                      })}
                    </li>
                  ))}
                </ul>
                {apercu.sous_resolution.length > 6 && (
                  <p className="bascule-reste">
                    {t("bascule.et_reste", { n: apercu.sous_resolution.length - 6 })}
                  </p>
                )}
              </div>
            )}

            {apercu.couverture_sous_resolution && (
              <p className="bascule-alerte">
                {t("bascule.couverture", {
                  apres: Math.round(apercu.couverture_sous_resolution.ppi_apres),
                })}
              </p>
            )}

            {apercu.inaptes.length > 0 && (
              <p className="bascule-note">
                {t("bascule.inaptes", {
                  n: apercu.inaptes.length,
                  planches: apercu.inaptes.map((i) => i.planche).join(", "),
                })}
              </p>
            )}

            {apercu.replis.length > 0 && (
              <p className="bascule-note">
                {t("bascule.replis", { n: apercu.replis.length })}
              </p>
            )}

            {apercu.epinglees_touchees.length > 0 && (
              <p className="bascule-note">
                {t("bascule.epinglees", {
                  n: apercu.epinglees_touchees.length,
                  planches: apercu.epinglees_touchees.join(", "),
                })}
              </p>
            )}

            {apercu.tailles_manquantes.length > 0 && (
              <p className="bascule-note">
                {t("bascule.manquantes", { n: apercu.tailles_manquantes.length })}
              </p>
            )}

            <p className="bascule-actions">
              <button className="bascule-appliquer" onClick={onAppliquer}>
                {t("bascule.appliquer")}
              </button>
              <span className="bascule-annulable">{t("bascule.annulable")}</span>
            </p>
          </section>
        )}

    </Dialogue>
  );
}
