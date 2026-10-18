// La police de l'album : une face pour tout le livre, copiée à côté
// d'`album.json` pour qu'il voyage.
//
// Elle partageait son panneau avec le format jusqu'à la session C3 ; le
// format vit désormais dans Recomposer (`RecomposerView.tsx`), et le
// changement de format sans recomposer derrière son lien (`BasculeView.tsx`).
// Changer de police ne bouge pas une planche, pas une photo, pas un
// recadrage : seules les chasses changent, donc les coupures de ligne, et au
// rendu seulement.
//
// Rien n'est écrit ici sauf le fichier de la police, qui est le seul octet
// que le moteur doit poser à côté de l'album. Le reste passe par
// l'historique d'édition : ⌘Z l'annule, ⌘S la grave.
//
// La liste des polices vit dans `ListePolices.tsx`, partagée avec le popover
// d'un bloc.

import { Police } from "./album";
import { PoliceEtat, PoliceOfferte } from "./bridge";
import { Dialogue } from "./Dialogue";
import { t } from "./i18n";
import { ListePolices } from "./ListePolices";
import { nomLisible } from "./police";

export function PoliceView({
  polices,
  policeAlbum,
  policeInfo,
  absents,
  filtre,
  onFiltre,
  onPolice,
  onRendrePolice,
  onClose,
}: {
  polices: PoliceOfferte[];
  policeAlbum: Police | null;
  policeInfo: PoliceEtat | null;
  /** The album's characters this face cannot draw, from the engine. */
  absents: string[];
  filtre: string;
  onFiltre: (v: string) => void;
  onPolice: (p: PoliceOfferte) => void;
  onRendrePolice: () => void;
  onClose: () => void;
}) {
  return (
    <Dialogue titre={t("bar.format")} onClose={onClose} className="bascule" panelClassName="bascule-panel">
        <p className="bascule-intro">{t("police.intro")}</p>

        {/* Ce dans quoi le livre sort, et ce qu'il pèse. Le poids est dit
            plutôt que subi : une face du système sortie de sa collection peut
            faire des mégaoctets, et l'album les porte. */}
        <p className="police-courante">
          {policeAlbum ? (
            <>
              <strong>{nomLisible(policeAlbum)}</strong>

              {policeInfo && !policeInfo.manquante && (
                <span className="police-poids">
                  {" · "}
                  {t("police.poids", { ko: Math.round(policeInfo.octets / 1024) })}
                </span>
              )}
              <button className="link police-rendre" onClick={onRendrePolice}>
                {t("police.rendre")}
              </button>
            </>
          ) : (
            <strong>{t("police.projet")}</strong>
          )}
        </p>
        {policeInfo?.manquante && (
          <p className="bascule-alerte">{t("police.manquante")}</p>
        )}
        {absents.length > 0 && (
          <p className="bascule-alerte">{phraseAbsents(absents)}</p>
        )}

        {polices.length > 0 && (
          <ListePolices
            polices={polices}
            projet={{
              libelle: t("police.projet"),
              note: t("police.projet.note"),
              choisi: !policeAlbum,
              onChoisir: onRendrePolice,
            }}
            estChoisie={(p) => policeAlbum?.postscript === p.postscript}
            onVoix={onPolice}
            onFace={onPolice}
            filtre={filtre}
            onFiltre={onFiltre}
          />
        )}
    </Dialogue>
  );
}


/** « Cette police ne dessine pas N caractères… », la liste entre guillemets,
 *  coupée à douze : au-delà, le nombre dit tout. */
export function phraseAbsents(absents: readonly string[]): string {
  const liste =
    absents.slice(0, 12).map((c) => `« ${c} »`).join(", ") +
    (absents.length > 12 ? "…" : "");
  return absents.length === 1
    ? t("police.absents.un", { liste })
    : t("police.absents", { n: absents.length, liste });
}
