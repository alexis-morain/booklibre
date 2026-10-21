// Planche → « Dates et lieux sous les photos… » (1.2-b) : le réglage
// d'album, quatre contenus et trois formes de date. Chaque choix s'applique
// aussitôt et fait un pas d'annulation ; « Rien » retire tout d'un clic.
// Les mots viennent du moteur, ce panneau ne fait que choisir.

import { Legendes } from "./album";
import { Dialogue } from "./Dialogue";
import { Cle, t } from "./i18n";

type Contenu = Legendes["contenu"] | null;

const CONTENUS: [Contenu, Cle][] = [
  [null, "legendes.rien"],
  ["date", "legendes.date"],
  ["lieu", "legendes.lieu"],
  ["date_lieu", "legendes.date.lieu"],
];

const FORMATS: [Legendes["date"], Cle][] = [
  ["jour", "legendes.format.jour"],
  ["court", "legendes.format.court"],
  ["mois", "legendes.format.mois"],
];

export function LegendesView({
  reglage,
  enCours,
  onChoisir,
  onClose,
}: {
  /** Le réglage de l'album, `null` pour rien. */
  reglage: Legendes | null;
  /** Le moteur lit les photos : les choix attendent. */
  enCours: boolean;
  /** Le contenu et la forme voulus ; `null` = rien. */
  onChoisir: (contenu: Contenu, date: Legendes["date"]) => void;
  onClose: () => void;
}) {
  const contenu = reglage?.contenu ?? null;
  const date = reglage?.date ?? "jour";
  const avecDate = contenu === "date" || contenu === "date_lieu";
  return (
    <Dialogue titre={t("legendes.titre")} onClose={onClose} panelClassName="prefs-panel">
      <h3>{t("legendes.contenu")}</h3>
      <div className="prefs-langues" role="radiogroup" aria-label={t("legendes.contenu")}>
        {CONTENUS.map(([id, cle]) => (
          <button
            key={id ?? "rien"}
            role="radio"
            aria-checked={contenu === id}
            className={"prefs-langue" + (contenu === id ? " active" : "")}
            disabled={enCours}
            onClick={() => onChoisir(id, date)}
          >
            {t(cle)}
          </button>
        ))}
      </div>
      {avecDate && (
        <>
          <h3>{t("legendes.format")}</h3>
          <div className="prefs-langues" role="radiogroup" aria-label={t("legendes.format")}>
            {FORMATS.map(([id, cle]) => (
              <button
                key={id}
                role="radio"
                aria-checked={date === id}
                className={"prefs-langue" + (date === id ? " active" : "")}
                disabled={enCours}
                onClick={() => onChoisir(contenu, id)}
              >
                {t(cle)}
              </button>
            ))}
          </div>
        </>
      )}
      <p className="signaler-note" role="status">
        {enCours ? t("legendes.lecture") : t("legendes.note")}
      </p>
    </Dialogue>
  );
}
