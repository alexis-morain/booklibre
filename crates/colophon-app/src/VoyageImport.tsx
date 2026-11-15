// L'écran « Importer un voyage Polarsteps » (P-s3) : un champ pour le lien,
// le dossier proposé, la progression, l'annulation, et des phrases d'erreur
// qui portent chacune leur remède. En texte : ni logo ni couleurs de la
// marque, et la mention « non affilié ».
//
// Deux étages, comme les autres écrans testés sans fenêtre : `VoyageImportVue`
// dessine un état (c'est lui que les tests rendent), `VoyageImport` tient
// l'état, la commande et l'annulation.

import { useEffect, useReducer, useRef } from "react";
import { cancelBuild } from "./bridge";
import { langue, t } from "./i18n";
import {
  analyserLien,
  cleProgres,
  entier,
  Etat,
  ETAT_INITIAL,
  importerVoyage,
  normaliserErreur,
  phrasesDe,
  RapportVoyage,
  reduire,
} from "./voyage";

export function VoyageImportVue({
  etat,
  onUrl,
  onLancer,
  onAnnuler,
  onRetour,
}: {
  etat: Etat;
  onUrl: (url: string) => void;
  onLancer: () => void;
  onAnnuler: () => void;
  onRetour: () => void;
}) {
  const enCours = etat.phase === "cours";
  const phrases = etat.erreur ? phrasesDe(etat.erreur.genre) : null;
  const progres = cleProgres(etat);
  const parts =
    progres.params && typeof progres.params.fait === "number"
      ? {
          fait: entier(progres.params.fait, langue()),
          total: entier(Number(progres.params.total), langue()),
        }
      : undefined;
  return (
    <div className="photos-import voyage-import">
      <h1 className="setup-heading">{t("voyage.titre")}</h1>
      <p className="lede">{t("voyage.lede")}</p>

      <label className="setup-field">
        <span className="setup-label">{t("voyage.lien")}</span>
        <input
          className="setup-input"
          type="url"
          inputMode="url"
          autoFocus
          spellCheck={false}
          autoComplete="off"
          value={etat.url}
          disabled={enCours}
          placeholder={t("voyage.lien.exemple")}
          aria-invalid={etat.invalide}
          onChange={(e) => onUrl(e.target.value)}
        />
      </label>
      {etat.invalide && (
        <p className="voyage-phrase" role="alert">
          {t("voyage.lien.invalide")}
        </p>
      )}

      {etat.lien && (
        <p className="setup-folder">
          {t("voyage.dossier")} <code>{etat.lien.dossier}</code>
        </p>
      )}

      {enCours && (
        <>
          <p className="lede" role="status">
            {t(progres.cle, parts ?? progres.params)}
          </p>
          <p className="setup-hint">{t("voyage.reprise")}</p>
          <p className="setup-actions">
            <button type="button" className="btn-second" onClick={onAnnuler}>
              {t("voyage.annuler")}
            </button>
          </p>
        </>
      )}

      {etat.phase === "annule" && (
        <p className="voyage-phrase" role="status">
          {t("voyage.annulee")}
        </p>
      )}

      {etat.phase === "echec" && etat.erreur && phrases && (
        <div className="voyage-erreur" role="alert">
          <p className="voyage-phrase">{t(phrases.phrase)}</p>
          <p className="setup-hint">
            {t(phrases.remede, { detail: etat.erreur.detail })}
          </p>
        </div>
      )}

      {!enCours && (
        <p className="setup-actions">
          <button className="cta" disabled={etat.lien === null} onClick={onLancer}>
            {etat.phase === "annule" || etat.phase === "echec"
              ? t("voyage.reessayer")
              : t("voyage.importer")}
          </button>
        </p>
      )}

      <p className="setup-hint voyage-zip">{t("voyage.zip")}</p>
      <p className="setup-hint">{t("voyage.non.affilie")}</p>

      {!enCours && (
        <p className="hint">
          <button className="link" onClick={onRetour}>
            {t("voyage.retour")}
          </button>
        </p>
      )}
    </div>
  );
}

export function VoyageImport({
  onImporte,
  onRetour,
}: {
  /** L'import est fini : le dossier est prêt à composer. */
  onImporte: (rapport: RapportVoyage) => void;
  onRetour: () => void;
}) {
  const [etat, envoyer] = useReducer(reduire, ETAT_INITIAL);
  // Une réponse d'`analyserLien` qui arrive après une frappe plus récente ne
  // doit pas écraser celle-ci.
  const derniere = useRef("");
  const vivant = useRef(true);
  useEffect(() => {
    vivant.current = true;
    return () => {
      vivant.current = false;
    };
  }, []);

  const changerUrl = (url: string) => {
    derniere.current = url;
    envoyer({ type: "url", url, lien: null });
    void analyserLien(url).then((lien) => {
      if (derniere.current === url && vivant.current) envoyer({ type: "url", url, lien });
    });
  };

  const lancer = async () => {
    if (!etat.lien || etat.phase === "cours") return;
    const { lien, url } = etat;
    envoyer({ type: "lancer" });
    try {
      const rapport = await importerVoyage(url, lien.dossier, (fait, total) => {
        if (vivant.current) envoyer({ type: "progres", fait, total });
      });
      if (vivant.current) onImporte(rapport);
    } catch (e) {
      if (vivant.current) envoyer({ type: "echec", erreur: normaliserErreur(e) });
    }
  };

  return (
    <VoyageImportVue
      etat={etat}
      onUrl={changerUrl}
      onLancer={() => void lancer()}
      onAnnuler={() => void cancelBuild()}
      onRetour={onRetour}
    />
  );
}
