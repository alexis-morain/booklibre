// Le panneau de propriétés du Livre, à droite de la planche, comme la page
// d'une base : ce que l'application sait de la planche (chapitre,
// disposition, moment, état, qualité), puis la photo choisie et ses réglages.
// Rien n'est calculé ici qui ne l'était déjà : le chapitre vient de
// `chapitres.ts`, la qualité de la règle des cases (`badgesDe`), la
// disposition du sélecteur de gabarit, les glissières de `ReglageBloc`. On a
// déplacé et habillé, on n'a pas recodé.

import { useEffect, useState } from "react";
import { Album, Spread, templateCapacity } from "./album";
import { FichePhoto as Fiche, photoFiche } from "./bridge";
import { Chapitre } from "./chapitres";
import { libelleGabarit } from "./gabarit";
import { langue, t } from "./i18n";
import { LockGlyph } from "./PlanchesView";
import { MiniThumb } from "./MiniThumb";
import { OrnementPicker } from "./OrnementPicker";
import { Ornement } from "./ornement";
import { Alerte, phraseDAlerte, detailDAlerte } from "./photos";
import { legendeQualite, motDeLigne } from "./qualite";
import { ReglageBloc } from "./ReglageBloc";
import { Reglage } from "./album";
import { TemplatePicker } from "./TemplatePicker";
import { useAlertesDeLaPlanche } from "./LegendeQualite";

/** La fiche d'une photo, lue une fois par photo ; rien tant qu'elle n'est pas
 *  là ou si le moteur ne la lit pas : le panneau n'invente pas ce qu'il ignore. */
function useFiche(src: string | null): Fiche | null {
  const [fiche, setFiche] = useState<{ src: string; f: Fiche } | null>(null);
  useEffect(() => {
    if (!src) return;
    let vivant = true;
    photoFiche(src).then(
      (f) => vivant && setFiche({ src, f }),
      () => {},
    );
    return () => {
      vivant = false;
    };
  }, [src]);
  return src && fiche?.src === src ? fiche.f : null;
}

const lieu = () => (langue() === "fr" ? "fr-FR" : "en-GB");

/** Le moment d'une planche : la prise de vue de sa première photo, si le
 *  fichier la dit vraiment. Une date venue du disque ne vaut rien ici. */
function momentDe(f: Fiche | null): string | null {
  if (!f || !f.taken_reliable) return null;
  const d = new Date(f.taken);
  if (Number.isNaN(d.getTime())) return null;
  return new Intl.DateTimeFormat(lieu(), {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(d);
}

function dateCourte(f: Fiche): string | null {
  if (!f.taken_reliable) return null;
  const d = new Date(f.taken);
  if (Number.isNaN(d.getTime())) return null;
  return new Intl.DateTimeFormat(lieu(), { day: "numeric", month: "short", year: "numeric" }).format(d);
}

export function Proprietes({
  album,
  spread,
  index,
  onCover,
  chapitre,
  selected,
  onTemplate,
  onLock,
  onReglage,
  onInformations,
  onRemplacer,
  onRetirer,
  onAjouterObjet,
  onAjouterOrnement,
}: {
  album: Album;
  spread: Spread | null;
  index: number;
  onCover: boolean;
  chapitre: Chapitre | null;
  /** La case choisie sur la planche, ou rien. */
  selected: number | null;
  onTemplate: (template: string) => void;
  onLock?: () => void;
  onReglage: (src: string, reglage: Reglage) => void;
  onInformations: () => void;
  onRemplacer: (cell: number, point: { x: number; y: number }) => void;
  onRetirer: (cell: number) => void;
  onAjouterObjet?: () => void;
  onAjouterOrnement?: (o: Ornement) => void;
}) {
  const alertes = useAlertesDeLaPlanche(album, onCover ? -1 : index);
  const slot = spread && selected !== null ? (spread.slots[selected] ?? null) : null;
  const premiere = spread?.slots[0]?.src ?? null;
  const fichePremiere = useFiche(premiere);
  const fichePhoto = useFiche(slot?.src ?? null);
  const moment = momentDe(fichePremiere);

  if (onCover || !spread) {
    return (
      <aside className="props" aria-label={t("props.aria")}>
        <h2>{t("props.couverture")}</h2>
        <p className="props-aide">{t("contexte.couverture")}</p>
      </aside>
    );
  }

  const lignes = legendeQualite([alertes]);
  const photoSpread = templateCapacity(spread.template) > 0;
  const alertesDeLaCase: Alerte[] = selected !== null ? (alertes[selected] ?? []) : [];

  return (
    <aside className="props" aria-label={t("props.aria")}>
      <h2>{t("props.planche", { n: index + 1 })}</h2>

      <dl className="props-liste">
        <div className="props-ligne">
          <dt>{t("props.chapitre")}</dt>
          <dd>
            {chapitre ? (
              <span className="tag chap" title={chapitre.titre}>{chapitre.titre}</span>
            ) : (
              <span className="props-aucun">{t("props.aucun")}</span>
            )}
          </dd>
        </div>
        <div className="props-ligne">
          <dt>{t("props.disposition")}</dt>
          <dd>
            {photoSpread ? (
              <TemplatePicker album={album} spread={spread} index={index} onPick={onTemplate} />
            ) : (
              libelleGabarit(spread.template)
            )}
          </dd>
        </div>
        {moment && (
          <div className="props-ligne">
            <dt>{t("props.moment")}</dt>
            <dd className="typed">{moment}</dd>
          </div>
        )}
        <div className="props-ligne">
          <dt>{t("props.etat")}</dt>
          <dd className="props-etat">
            {!spread.edited && !spread.locked && (
              <span className="tag">{t("props.etat.auto")}</span>
            )}
            {spread.edited && <span className="tag main">{t("props.etat.editee")}</span>}
            {spread.locked && <span className="tag main">{t("props.etat.figee")}</span>}
            {onLock && (
              <button
                className="link props-figer"
                onClick={onLock}
                aria-pressed={spread.locked ?? false}
                title={spread.locked ? t("table.figee") : t("table.figer")}
              >
                <LockGlyph open={!spread.locked} />
                {spread.locked ? t("props.etat.liberer") : t("props.etat.figer")}
              </button>
            )}
          </dd>
        </div>
        <div className="props-ligne">
          <dt>{t("props.qualite")}</dt>
          <dd className="props-etat">
            {lignes.length === 0 ? (
              <span className="props-aucun">{t("props.qualite.rien")}</span>
            ) : (
              lignes.map((l) => (
                <span key={l.code} className="tag qual">
                  {motDeLigne(l)}
                </span>
              ))
            )}
          </dd>
        </div>
      </dl>

      <hr className="props-filet" />

      {slot && selected !== null ? (
        <section className="props-photo" aria-label={t("props.photo")}>
          <div className="props-photo-tete">
            <MiniThumb src={slot.src} />
            <div className="props-photo-nom">
              <b title={fichePhoto?.nom ?? slot.src}>{fichePhoto?.nom ?? slot.src}</b>
              {fichePhoto && (
                <small className="typed">
                  {[dateCourte(fichePhoto), t("fiche.dimensions.v", { l: fichePhoto.largeur, h: fichePhoto.hauteur })]
                    .filter(Boolean)
                    .join(" · ")}
                </small>
              )}
            </div>
          </div>

          {alertesDeLaCase.length > 0 && (
            <div className="encart qual" role="group" aria-label={t("props.qualite")}>
              <div>
                {alertesDeLaCase.map((a) => (
                  <p key={a.code}>
                    {phraseDAlerte(a)}
                    {detailDAlerte(a) && <small className="typed"> {detailDAlerte(a)}</small>}
                  </p>
                ))}
                <div className="encart-gestes">
                  <button
                    onClick={(e) => {
                      const r = e.currentTarget.getBoundingClientRect();
                      onRemplacer(selected, { x: r.left, y: r.bottom });
                    }}
                  >
                    {t("props.photo.remplacer")}
                  </button>
                  <button onClick={() => onRetirer(selected)}>{t("props.photo.retirer")}</button>
                </div>
              </div>
            </div>
          )}

          <div className="props-photo-gestes">
            <button className="link" onClick={onInformations} title="⌘I">
              {t("props.photo.infos")}
            </button>
            {alertesDeLaCase.length === 0 && (
              <>
                <button
                  className="link"
                  onClick={(e) => {
                    const r = e.currentTarget.getBoundingClientRect();
                    onRemplacer(selected, { x: r.left, y: r.bottom });
                  }}
                >
                  {t("props.photo.remplacer")}
                </button>
                <button className="link" onClick={() => onRetirer(selected)}>
                  {t("props.photo.retirer")}
                </button>
              </>
            )}
          </div>

          <h3 className="props-titre">{t("props.reglages")}</h3>
          <ReglageBloc src={slot.src} onCommit={onReglage} />
        </section>
      ) : (
        <p className="props-aide">{t("props.photo.aucune")}</p>
      )}

      {(onAjouterObjet || onAjouterOrnement) && (
        <>
          <hr className="props-filet" />
          <h3 className="props-titre">{t("props.ajouter")}</h3>
          <div className="props-ajouter">
            {onAjouterObjet && (
              <button className="btn-second" onClick={onAjouterObjet} title={t("objet.ajouter.titre")}>
                {t("objet.ajouter")}
              </button>
            )}
            {onAjouterOrnement && <OrnementPicker onPick={onAjouterOrnement} />}
          </div>
        </>
      )}
    </aside>
  );
}
