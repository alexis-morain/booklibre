// « Photos d'origine » (P-s8) : le bloc qui remplace les vignettes d'un voyage
// par les photos de la personne. Il vit à deux endroits, le bilan qui suit
// l'import et le réglage « Dates et lieux » d'un album de voyage ouvert.
//
// Deux étages, comme les autres écrans testés sans fenêtre : `OriginauxVue`
// dessine un état, `Originaux` tient l'état et les commandes.

import { useEffect, useReducer, useRef } from "react";
import { confirmDialog, pickPhotosFolder } from "./bridge";
import { langue, t } from "./i18n";
import {
  cleRaison,
  Etat,
  ETAT_INITIAL,
  etatOriginaux,
  LIGNES_VISIBLES,
  lignesOriginaux,
  reduire,
  remplacerOriginaux,
  retablirOriginaux,
} from "./voyage-originaux";
import { entier } from "./voyage";

export function OriginauxVue({
  etat,
  onChoisir,
  onAnnuler,
  onRetablir,
}: {
  etat: Etat;
  onChoisir: () => void;
  onAnnuler: () => void;
  onRetablir: () => void;
}) {
  const enCours = etat.phase === "cours";
  const lignes = lignesOriginaux(etat, langue());
  const deja = etat.remplacees > 0;
  const visibles = etat.refus.slice(0, LIGNES_VISIBLES);
  return (
    <section className="originaux" aria-label={t("originaux.titre")}>
      <h2 className="bilan-choix-titre">{t("originaux.titre")}</h2>
      {!deja && !etat.rapport && <p className="setup-hint">{t("originaux.lede")}</p>}

      {lignes.length > 0 && (
        <ul className="bilan-voyage-lignes">
          {lignes.map((l) => (
            <li key={l.cle}>{t(l.cle, l.params)}</li>
          ))}
        </ul>
      )}
      {etat.dossier && !enCours && (
        <p className="setup-hint">{t("originaux.dossier", { dossier: etat.dossier })}</p>
      )}

      {etat.refus.length > 0 && !enCours && (
        <details className="originaux-liste">
          <summary>{t("originaux.liste")}</summary>
          <ul>
            {visibles.map((r) => (
              <li key={r.fichier}>
                <span className="originaux-etape">{r.etape}</span> <code>{r.fichier}</code>
                <span className="originaux-raison"> : {t(cleRaison(r.raison))}</span>
              </li>
            ))}
          </ul>
          {etat.refus.length > visibles.length && (
            <p className="setup-hint">
              {t("originaux.plus", { n: entier(etat.refus.length - visibles.length, langue()) })}
            </p>
          )}
        </details>
      )}

      {enCours && (
        <p className="lede" role="status">
          {etat.total === 0
            ? t("originaux.lecture.debut")
            : t("originaux.lecture", { fait: entier(etat.fait, langue()), total: entier(etat.total, langue()) })}
        </p>
      )}
      {etat.phase === "echec" && (
        <p className="voyage-phrase" role="alert">
          {t("originaux.err", { detail: etat.detail })}
        </p>
      )}
      {etat.phase === "annule" && (
        <p className="voyage-phrase" role="status">
          {t("originaux.annulee")}
        </p>
      )}
      {etat.rendues && (
        <p className="setup-hint" role="status">
          {t("originaux.retablies")}
        </p>
      )}

      <p className="setup-actions">
        {enCours ? (
          <button type="button" className="btn-second" onClick={onAnnuler}>
            {t("originaux.annuler")}
          </button>
        ) : (
          <>
            <button type="button" className="btn-second" onClick={onChoisir}>
              {deja || etat.rapport ? t("originaux.autre") : t("originaux.choisir")}
            </button>
            {deja && (
              <button type="button" className="link" onClick={onRetablir}>
                {t("originaux.retablir")}
              </button>
            )}
          </>
        )}
      </p>
    </section>
  );
}

export function Originaux({
  onChange,
  onAnnuler,
}: {
  /** Les photos du dossier ont changé : l'écran qui les montre se rafraîchit. */
  onChange: () => Promise<void> | void;
  onAnnuler: () => void;
}) {
  const [etat, envoyer] = useReducer(reduire, ETAT_INITIAL);
  const vivant = useRef(true);
  useEffect(() => {
    vivant.current = true;
    void etatOriginaux().then((e) => {
      if (e && vivant.current) envoyer({ type: "etat", etat: e });
    });
    return () => {
      vivant.current = false;
    };
  }, []);

  const choisir = async () => {
    if (etat.phase === "cours") return;
    const dossier = await pickPhotosFolder();
    if (dossier === null) return;
    envoyer({ type: "lancer" });
    try {
      const rapport = await remplacerOriginaux(dossier, (fait, total) => {
        if (vivant.current) envoyer({ type: "progres", fait, total });
      });
      if (vivant.current) envoyer({ type: "fait", rapport, dossier });
      await onChange();
    } catch (e) {
      const detail = e instanceof Error ? e.message : String(e);
      if (vivant.current) envoyer({ type: "echec", detail, annule: detail.includes("annulé") });
      // Une annulation a pu laisser des photos remplacées : l'écran les relit.
      await onChange();
      const e2 = await etatOriginaux();
      if (e2 && vivant.current) envoyer({ type: "etat", etat: e2 });
    }
  };

  const retablir = async () => {
    if (!(await confirmDialog(t("originaux.retablir.confirme")))) return;
    try {
      await retablirOriginaux();
      if (vivant.current) envoyer({ type: "rendues" });
      await onChange();
    } catch (e) {
      if (vivant.current) envoyer({ type: "echec", detail: e instanceof Error ? e.message : String(e), annule: false });
    }
  };

  return <OriginauxVue etat={etat} onChoisir={() => void choisir()} onAnnuler={onAnnuler} onRetablir={() => void retablir()} />;
}
