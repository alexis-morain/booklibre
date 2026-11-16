// Le bouton « Commander » de la barre, et ce qui le grise (décision d'Alexis,
// 15/11) : un livre de plus de pages que le relais n'en vend ne se commande pas
// dans Booklibre, et l'écran le dit avant le clic au lieu d'un refus après.
// La limite est celle du moteur (`Tarifs.pages_max`), le compte celui des
// pages intérieures. L'export du PDF, lui, n'a aucune limite : son bouton est
// à côté et ne passe pas par ici.

import { pagesInvendables } from "./commande";
import { t } from "./i18n";

export const ID_TROP_DE_PAGES = "bar-trop-de-pages";

export function BoutonCommander({
  pages,
  limite,
  onCommander,
}: {
  /** Les pages intérieures du livre, deux par planche. */
  pages: number;
  /** `Tarifs.pages_max`, `null` tant que la table n'est pas lue. */
  limite: number | null;
  onCommander: () => void;
}) {
  const trop = pagesInvendables(pages, limite);
  const message = trop === null ? null : t("commande.trop.pages", { n: trop, max: limite ?? 0 });
  return (
    <>
      <button
        className="btn-second"
        onClick={onCommander}
        disabled={trop !== null}
        aria-describedby={trop !== null ? ID_TROP_DE_PAGES : undefined}
        title={message ?? t("bar.commander.titre")}
      >
        {t("bar.commander")}
      </button>
      {message && (
        <span id={ID_TROP_DE_PAGES} className="sr-seul">
          {message}
        </span>
      )}
    </>
  );
}
