// Le livre, dans la colonne de droite d'Envoi (A2) : le plat de la
// couverture posé en légère perspective, le dos à sa largeur réelle et à la
// même échelle que la page, puis trois lignes en clair et « Feuilleter ».
//
// C'est un emplacement : le volume complet (six faces, la tranche des pages)
// et la couleur de la couverture sont A4, qui remplacera ce composant sans
// toucher à ce qui l'entoure. Rien n'y est calculé qui ne vienne du moteur :
// le dos est `fiche.dos_mm`, le format `fiche.format_page_mm`, le papier le
// profil (`envoi.ts::papierEnClair`).

import type { Album } from "./album";
import type { Fiche } from "./bridge";
import { formatEnCm } from "./envoi";
import { t } from "./i18n";
import { MiniCouverture } from "./PlanchesView";

/** La hauteur du plat à l'écran, en pixels : la largeur et le dos suivent à
 *  la même échelle. */
const HAUTEUR_PX = 260;

export function LivreApercu({
  album,
  fiche,
  papier,
  onFeuilleter,
}: {
  album: Album;
  /** Null tant que le contrôle n'a pas rendu sa fiche : le format vient
   *  alors de l'album, et ni le dos ni les pages ne se disent. */
  fiche: Fiche | null;
  /** Le papier en clair, quand l'imprimeur est connu ; sinon rien. */
  papier: string | null;
  onFeuilleter: () => void;
}) {
  const [wMm, hMm] = fiche?.format_page_mm ?? [album.trim_mm.w, album.trim_mm.h];
  const echelle = HAUTEUR_PX / hMm;
  const largeur = wMm * echelle;
  const dos = fiche?.dos_mm !== undefined ? Math.max(2, fiche.dos_mm * echelle) : 0;
  return (
    <section className="envoi-livre" aria-label={t("envoi.livre")}>
      <div className="livre" aria-hidden="true">
        <div
          className="livre-objet"
          style={{ width: `${largeur}px`, height: `${HAUTEUR_PX}px` }}
        >
          <div className="livre-plat">
            <MiniCouverture album={album} />
          </div>
          {dos > 0 && (
            <div className="livre-dos" style={{ width: `${dos}px` }}>
              {dos >= 12 && <span>{album.cover?.title || album.title}</span>}
            </div>
          )}
        </div>
      </div>
      <ul className="livre-lignes">
        <li>{formatEnCm([wMm, hMm])}</li>
        {fiche && <li>{t("envoi.livre.pages", { n: fiche.pages_interieur })}</li>}
        {papier && <li>{papier}</li>}
      </ul>
      <button
        type="button"
        className="livre-feuilleter"
        title={t("envoi.livre.feuilleter.titre")}
        onClick={onFeuilleter}
      >
        {t("envoi.livre.feuilleter")}
      </button>
    </section>
  );
}
