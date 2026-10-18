// Envoi : le dernier écran avant que le livre quitte la machine (A2).
//
// Il parle à quelqu'un qui fait un album, pas à un imprimeur. D'abord ce qui
// bloque, et où (chaque défaut mène à ce qui le répare ; une photo trop
// petite pour sa case porte ses propres gestes). Puis deux portes, une seule
// ouverte : « Télécharger le PDF », pour l'imprimeur de son choix, et « Faire
// imprimer par Colophon », le livre relié livré chez soi. Le bouton suit la
// porte. À droite, le livre : sa couverture, trois lignes en clair, et
// « Feuilleter ». La fiche technique, que demande un imprimeur au téléphone,
// est repliée sous « Pour les imprimeurs », sous la porte PDF seulement.
//
// Sans relais compilé (la release publique), la porte Colophon n'existe pas
// et rien ne l'annonce : le PDF seul, sans carte. Sous la porte Colophon,
// « Faire imprimer » prépare sans boîte et la commande s'ouvre derrière
// (`CommandeView`), avec à droite ce qu'on peut vérifier de ce qui se passe
// (`Rassurance`).
//
// Rien ici n'est calculé dans le navigateur. Le contrôle, la fiche et le dos
// viennent du moteur, par profil : deux fournisseurs ne s'accordent sur aucun
// champ, et une seconde copie de leurs specs en TypeScript serait une chose
// de plus à garder vraie. `EnvoiView` tient l'état ; `EnvoiPage` le dessine,
// et se rend sans navigateur (`envoi-page.test.tsx`).

import { useEffect, useState } from "react";
import { phraseAbsents } from "./PoliceView";
import { CommandeView, Rassurance, useRelais } from "./CommandeView";
import { phraseDefaut, texteMoteur } from "./moteur";
import { Album } from "./album";
import {
  Defaut,
  Fiche,
  Preparation,
  Printer,
  PrevolReport,
  montrerDossierPrepare,
  openReportUrl,
  preflight,
} from "./bridge";
import {
  PROFIL_ENVOI,
  PROFIL_LIBRE,
  Porte,
  actionEnvoi,
  ActionDefaut,
  Destination,
  actionsDefaut,
  destinationDe,
  formatEnCm,
  imprimeurAffiche,
  libelleDestination,
  papierEnClair,
  phrasePreparation,
  phraseSimple,
  porteEffective,
  portesOffertes,
  ppiDe,
} from "./envoi";
import { Niveau, Relais, clePrix, livreEnFrance, prixConnu, prixDeCentimes, retenirPrix } from "./commande";
import { t, useLangue } from "./i18n";
import { Livre3D } from "./Livre3D";
import { PaletteFond } from "./PaletteFond";
import { LazyThumb } from "./TriView";
import { VERDICT_URL } from "./signaler";

const mm = (v: number) => v.toFixed(1).replace(".", ",");

export type EnvoiViewProps = {
  album: Album;

  /** Loaded once by the window, shared with the cover editor. */
  printers: Printer[] | null;
  profil: string;
  onProfil: (id: string) => void;
  /** Show spread `n` (1-based) in the book view. */
  onJump: (planche: number) => void;
  /** L'éditeur de couverture ; `choisir` ouvre aussi le choix de sa photo. */
  onCouverture: (choisir: boolean) => void;
  /** Le panneau Recomposer, où se choisit le format. */
  onFormat: () => void;
  /** Une photo trop petite pour sa case : la planche ouverte sur la case. */
  onVoir: (planche: number, cell: number) => void;
  /** Le choix de la réserve pour cette case, ouvert sous le bouton. */
  onRemplacer: (planche: number, cell: number, point: { x: number; y: number }) => void;
  /** La photo retourne à la réserve ; une entrée d'historique. */
  onRetirer: (planche: number, cell: number) => void;
  /** Aucune photo à proposer à la place : « Remplacer » ne s'offre pas. */
  reserveVide: boolean;
  /** Le PDF sans contrainte : un fichier, où la boîte le pose. */
  onExport: () => void;
  /** Chez qui relie deux fichiers : le dossier de la commande. `commande` :
   *  « Faire imprimer », sans boîte, et la commande s'ouvre derrière. */
  onPrepare: (commande: boolean) => void;
  /** Ce que la dernière préparation a écrit et ce que le contrôle relu en
   *  dit. Null tant que rien n'a été préparé pour cet album. */
  preparation: Preparation | null;
  exporting: boolean;
  /** A print PDF was written for this album: the verdict form is offered. */
  exporte: boolean;
  /** Unsaved edits: the preflight reads the disk, so it would lie. */
  dirty: boolean;
  /** The album carries the facts the page is made of. False on albums
   *  composed before the page existed: nothing to offer, so nothing shows. */
  colophonPossible: boolean;
  colophonActif: boolean;
  onColophon: (on: boolean) => void;
  /** The half-title travels with the colophon: both need the facts the
   *  composition measured, so `colophonPossible` answers for the two. */
  gardeActif: boolean;
  onGarde: (on: boolean) => void;
  /** La police que l'album nomme n'est plus dans son dossier. L'export ne
   *  échoue pas — il sort dans celle du moteur —, et c'est bien pour ça que
   *  ça se dit ici : c'est le dernier écran avant l'imprimeur, et un livre
   *  composé dans une police que personne n'a choisie se découvre au colis. */
  policeManquante: boolean;
  /** The album's characters its face cannot draw: printed as « ? ». */
  absents: string[];
  /** « Feuilleter » : l'aperçu fidèle, le livre tel que le PDF l'imprimera. */
  onFeuilleter: () => void;
  /** Le fond de la couverture et du dos choisi sous le livre : un pas
   *  d'annulation, enregistré. `null` : le blanc papier. */
  onFond: (fond: string | null) => void;
  /** Où en est la préparation qui tourne, en pages rendues ; null sinon. */
  progresExport: { done: number; total: number } | null;
};

export function EnvoiView(props: EnvoiViewProps) {
  const { album, profil, dirty, preparation } = props;
  const [report, setReport] = useState<PrevolReport | null>(null);
  // The folder prepared before this opening of the screen is not offered to
  // Commander: the screen unmounts when left, so what it found on arrival
  // is the line between the two.
  const [prepareeAvant] = useState(preparation);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [porte, setPorte] = useState<Porte | null>(null);

  // Re-run on every profile change, on every arrival, and the moment the
  // album stops being dirty: the preflight reads the disk, so a save is what
  // makes a correction real to it, and the verdict has to follow within the
  // second rather than waiting for the user to leave and come back.
  useEffect(() => {
    let alive = true;
    setRunning(true);
    setError(null);
    preflight(profil).then(
      (r) => alive && (setReport(r), setRunning(false)),
      (e) => alive && (setError(String(e)), setReport(null), setRunning(false)),
    );
    return () => {
      alive = false;
    };
  }, [profil, album, dirty]);

  const { relais, sonde, sonder } = useRelais();
  const cle = clePrix(report?.fiche.produit, report?.fiche.pages_interieur ?? 0);
  const [prixVu, setPrixVu] = useState<{ cle: string | null; centimes: number | null }>({ cle: null, centimes: null });
  const prixMin = prixVu.cle === cle && prixVu.centimes !== null ? prixVu.centimes : prixConnu(cle);
  const onGrille = (grille: Niveau[]) => {
    retenirPrix(cle, grille);
    setPrixVu({ cle, centimes: prixConnu(cle) });
  };

  return (
    <EnvoiPage
      {...props}
      report={report}
      running={running}
      error={error}
      relais={relais}
      sonde={sonde}
      sonder={sonder}
      porte={porte}
      onPorte={setPorte}
      prixMin={prixMin}
      onGrille={onGrille}
      prepareeAvant={prepareeAvant}
    />
  );
}

export type EnvoiPageProps = EnvoiViewProps & {
  report: PrevolReport | null;
  running: boolean;
  error: string | null;
  relais: Relais | null;
  sonde: boolean;
  sonder: () => void;
  /** La porte choisie ; null tant qu'on n'a rien choisi (`porteEffective`). */
  porte: Porte | null;
  onPorte: (p: Porte) => void;
  /** Le « à partir de » de la porte Colophon, en centimes, quand une grille
   *  du relais l'a dit ; null sinon, et alors aucun prix. */
  prixMin: number | null;
  onGrille: (grille: Niveau[]) => void;
  prepareeAvant: Preparation | null;
};

export function EnvoiPage(props: EnvoiPageProps) {
  const {
    album,
    printers,
    profil,
    onProfil,
    onJump,
    onCouverture,
    onFormat,
    onVoir,
    onRemplacer,
    onRetirer,
    reserveVide,
    onExport,
    onPrepare,
    preparation,
    exporting,
    exporte,
    dirty,
    colophonPossible,
    colophonActif,
    onColophon,
    gardeActif,
    onGarde,
    policeManquante,
    absents,
    onFeuilleter,
    onFond,
    progresExport,
    report,
    running,
    error,
    relais,
    sonde,
    sonder,
    porte,
    onPorte,
    prixMin,
    onGrille,
    prepareeAvant,
  } = props;
  const langue = useLangue();
  const chosen = imprimeurAffiche(printers, profil);
  const libre = profil === PROFIL_LIBRE;
  const offertes = portesOffertes(relais);
  // Le PDF sans contrainte est une affaire de la porte PDF : y être arrivé
  // l'ouvre, tant qu'on n'a pas choisi l'autre.
  const ouverte: Porte = libre && porte === null ? "pdf" : porteEffective(porte, relais);
  const action = chosen ? actionEnvoi(chosen, ouverte) : null;
  const choisir = (p: Porte) => {
    onPorte(p);
    // Colophon ne fait imprimer que chez Cloudprinter.
    if (p === "colophon" && libre) onProfil(PROFIL_ENVOI);
  };
  // « Préparer à nouveau » refait la préparation de la même façon.
  const preparerEncore = () => onPrepare(preparation?.commande === true);
  const bloquants = report?.defauts.filter((d) => d.bloquant) ?? [];
  const avertissements = report?.defauts.filter((d) => !d.bloquant) ?? [];
  // Un défaut dont le geste est une case de « Pages de début et de fin »
  // ouvre ce repli : on ne cache pas ce qu'une ligne demande de décocher.
  const pagesOuvertes = (report?.defauts ?? []).some((d) => destinationDe(d) === "ici");
  const enFrance = relais?.etat === "joignable" && livreEnFrance(relais.pays);
  const indisponible = ouverte === "colophon" && relais?.etat === "injoignable";
  // « Faire imprimer » a préparé ce dossier dans cette ouverture : le
  // parcours a pris la main, et « Commander » le rouvre au repos. Le bouton
  // ne se répète pas au-dessus, où il relancerait une préparation.
  const parcoursOuvert =
    ouverte === "colophon" &&
    preparation?.commande === true &&
    preparation !== prepareeAvant &&
    preparation.rapport.ok;
  const papier = ouverte === "colophon" && chosen ? papierEnClair(chosen, report?.fiche.grammage_g_m2 ?? 0) : null;
  // Ce que chaque destination fait d'un clic. « Préparer à nouveau » ne
  // s'offre que sous le dossier préparé : les fichiers ne se jugent plus
  // ailleurs, et avant lui le bouton du bas est déjà la porte.
  const gestes = (preparer?: () => void): Gestes => ({
    planche: onJump,
    couverture: onCouverture,
    format: onFormat,
    preparer,
  });
  const ligne = (d: Defaut, key: string) => {
    const actions = actionsDefaut(d, album, reserveVide);
    return actions.length > 0 ? (
      <ResolutionLigne
        key={key}
        d={d}
        actions={actions}
        onVoir={onVoir}
        onRemplacer={onRemplacer}
        onRetirer={onRetirer}
      />
    ) : (
      <DefautLigne key={key} d={d} gestes={gestes()} />
    );
  };

  return (
    <div className="envoi">
      <div className="envoi-gauche">
        <section className="envoi-verdict">
          {policeManquante && <p className="envoi-dirty">{t("police.manquante")}</p>}
          {absents.length > 0 && <p className="envoi-dirty">{phraseAbsents(absents)}</p>}
          {dirty && <p className="envoi-dirty">{t("envoi.dirty")}</p>}
          {error ? (
            <>
              <h2 className="envoi-ko">{t("envoi.prevol.echec")}</h2>
              <details className="fault-detail">
                <summary>{t("erreur.detail")}</summary>
                <pre>{error}</pre>
              </details>
            </>
          ) : running || !report ? (
            <h2 className="envoi-wait">{t("envoi.controle")}</h2>
          ) : report.ok ? (
            <h2 className="envoi-ok envoi-pret">
              <SigneCoche />
              {t("envoi.pret")}
            </h2>
          ) : (
            <>
              <h2 className="envoi-ko">
                {bloquants.length === 1 ? t("envoi.ko.un") : t("envoi.ko", { n: bloquants.length })}
              </h2>
              <p className="envoi-sub">{t("envoi.ko.sub")}</p>
            </>
          )}
        </section>

        {(bloquants.length > 0 || avertissements.length > 0) && (
          <section className="envoi-defauts">
            {bloquants.some((d) => d.regle === "resolution") && (
              <p className="envoi-resolution-tete">{t("envoi.resolution.tete")}</p>
            )}
            {bloquants.map((d, i) => ligne(d, `b${i}`))}
            {avertissements.map((d, i) => ligne(d, `a${i}`))}
          </section>
        )}

        <section className="envoi-portes">
          {offertes.length > 1 ? (
            <Portes
              ouverte={ouverte}
              onChoisir={choisir}
              enFrance={enFrance}
              prix={prixMin === null ? null : prixDeCentimes(prixMin, langue)}
            />
          ) : (
            <div className="envoi-porte-seule">
              <h3>{t("envoi.porte.pdf.titre")}</h3>
              <p>{t("envoi.porte.pdf.texte")}</p>
            </div>
          )}
          {ouverte === "pdf" && libre && (
            <p className="envoi-libre">
              {t("envoi.libre.tete")}{" "}
              <button className="link" onClick={() => onProfil(PROFIL_ENVOI)}>
                {t("envoi.libre.retour")}
              </button>
            </p>
          )}
        </section>

        <section className="envoi-actions">
          {indisponible ? (
            <div className="commande-ouvrir">
              <p className="commande-erreur" role="status">
                {t("commande.indisponible")}
              </p>
              <button className="link" disabled={sonde} onClick={() => sonder()}>
                {t("commande.reessayer")}
              </button>
            </div>
          ) : parcoursOuvert ? null : (
            <button
              className="envoi-exporter"
              onClick={action?.prepare ? () => onPrepare(action.commande) : onExport}
              disabled={exporting || !report?.ok || !action}
              title={report?.ok ? action?.titre : t("envoi.exporter.bloque")}
            >
              {exporting ? t("envoi.exporter.rendu") : (action?.texte ?? t("envoi.telecharger"))}
            </button>
          )}
          {preparation && action?.prepare && (!preparation.commande || !preparation.rapport.ok) && (
            <div className="envoi-prepare">
              {!preparation.commande && (
                <p className={preparation.rapport.ok ? "envoi-prepare-ok" : "envoi-prepare-ko"}>
                  {phrasePreparation(preparation)}
                </p>
              )}
              {preparation.rapport.defauts.some((d) => d.bloquant) && (
                <div className="envoi-defauts">
                  {preparation.rapport.defauts
                    .filter((d) => d.bloquant)
                    .map((d, i) => (
                      <DefautLigne key={i} d={d} gestes={gestes(preparerEncore)} />
                    ))}
                </div>
              )}
              {!preparation.commande && (
                <button className="link" onClick={() => void montrerDossierPrepare()}>
                  {t("envoi.prepare.montrer")}
                </button>
              )}
            </div>
          )}
          <CommandeView
            preparationNeuve={action?.prepare && preparation !== prepareeAvant ? preparation : null}
            relais={relais}
            sonde={sonde}
            sonder={sonder}
            album={album}
            fiche={report?.fiche ?? null}
            preparant={exporting && action?.commande === true}
            progres={progresExport}
            onGrille={onGrille}
          />
          {ouverte === "pdf" && !libre && (
            <p className="envoi-autre">
              <button className="link" onClick={() => onProfil(PROFIL_LIBRE)}>
                {t("envoi.autre")}
              </button>
            </p>
          )}
          {ouverte === "pdf" && report && <PourLesImprimeurs report={report} chosen={chosen} />}
        </section>

        {colophonPossible && (
          <details className="envoi-colophon" open={pagesOuvertes}>
            <summary>{t("envoi.pages")}</summary>
            <label>
              <input type="checkbox" checked={gardeActif} onChange={(e) => onGarde(e.target.checked)} />
              {t("envoi.garde.label")}
            </label>
            <p>{t("envoi.garde.note")}</p>
            <label>
              <input type="checkbox" checked={colophonActif} onChange={(e) => onColophon(e.target.checked)} />
              {t("envoi.colophon.label")}
            </label>
            <p>{t("envoi.colophon.note")}</p>
          </details>
        )}

        {exporte && (
          <section className="envoi-verdict-appel">
            <h3>{t("envoi.verdict.titre")}</h3>
            <p>{t("envoi.verdict.texte")}</p>
            <button className="link" onClick={() => void openReportUrl(VERDICT_URL)}>
              {t("envoi.verdict.bouton")}
            </button>
          </section>
        )}
      </div>

      <div className="envoi-droite">
        <LivreEnvoi
          album={album}
          fiche={report?.fiche ?? null}
          papier={papier}
          onFeuilleter={onFeuilleter}
          onFond={onFond}
        />
        {ouverte === "colophon" && <Rassurance enFrance={enFrance} />}
      </div>
    </div>
  );
}

/** La coche du verdict vert, dessinée. */
function SigneCoche() {
  return (
    <svg className="envoi-coche" viewBox="0 0 16 16" width="18" height="18" aria-hidden="true">
      <path d="M 3 8.5 L 6.5 12 L 13 4.5" fill="none" stroke="currentColor" strokeWidth="2" />
    </svg>
  );
}

/** Les deux portes, en cartes : un groupe de boutons radio, un seul choisi,
 *  les flèches passant de l'un à l'autre. */
function Portes({
  ouverte,
  onChoisir,
  enFrance,
  prix,
}: {
  ouverte: Porte;
  onChoisir: (p: Porte) => void;
  enFrance: boolean;
  prix: string | null;
}) {
  const ordre: Porte[] = ["pdf", "colophon"];
  const clavier = (e: React.KeyboardEvent) => {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) return;
    e.preventDefault();
    const autre = ordre[(ordre.indexOf(ouverte) + 1) % 2];
    onChoisir(autre);
    (e.currentTarget.querySelector(`[data-porte="${autre}"]`) as HTMLElement | null)?.focus();
  };
  const carte = (p: Porte, contenu: React.ReactNode) => (
    <button
      type="button"
      role="radio"
      data-porte={p}
      aria-checked={ouverte === p}
      tabIndex={ouverte === p ? 0 : -1}
      className={"envoi-porte" + (ouverte === p ? " active" : "")}
      onClick={() => onChoisir(p)}
    >
      {contenu}
    </button>
  );
  return (
    <div className="envoi-portes-cartes" role="radiogroup" aria-label={t("envoi.portes")} onKeyDown={clavier}>
      {carte(
        "pdf",
        <>
          <span className="envoi-porte-titre">{t("envoi.porte.pdf.titre")}</span>
          <span className="envoi-porte-texte">{t("envoi.porte.pdf.texte")}</span>
        </>,
      )}
      {carte(
        "colophon",
        <>
          <span className="envoi-porte-titre">{t("envoi.porte.colophon.titre")}</span>
          <span className="envoi-porte-texte">
            {t(enFrance ? "envoi.porte.colophon.texte.france" : "envoi.porte.colophon.texte")}
          </span>
          {prix && <span className="envoi-porte-prix">{t("envoi.porte.colophon.prix", { prix })}</span>}
          <span className="envoi-porte-partenaire">{t("envoi.porte.colophon.partenaire")}</span>
        </>,
      )}
    </div>
  );
}

/** La fiche technique, repliée : ce qu'un imprimeur demande au téléphone. */
function PourLesImprimeurs({ report, chosen }: { report: PrevolReport; chosen: Printer | null }) {
  return (
    <details className="envoi-imprimeurs">
      <summary>{t("envoi.imprimeurs")}</summary>
      {chosen && (
        <div className="envoi-imprimeur active">
          <span className="envoi-imprimeur-nom">{chosen.nom}</span>
          <span className="envoi-imprimeur-quoi">
            {chosen.pdf_x === "x4" ? "PDF/X-4" : t("envoi.pdf.simple")} ·{" "}
            {chosen.espace === "rgb" ? t("envoi.rvb") : t("envoi.cmjn")} ·{" "}
            {chosen.fichiers === "deux" ? t("envoi.deux.fichiers") : t("envoi.un.fichier")} ·{" "}
            {chosen.dos.mode === "calcule" ? t("envoi.dos.fournir") : t("envoi.dos.non")}
          </span>
          {chosen.certitude === "provisoire" && (
            <span className="envoi-provisoire" title={chosen.reserves.map(texteMoteur).join(" · ")}>
              {t("envoi.provisoire")}
            </span>
          )}
        </div>
      )}
      <div className="envoi-fiche">
        <h3>{t("envoi.fiche.titre")}</h3>
        <dl>
          <Ligne k={t("envoi.fiche.format")}>
            {mm(report.fiche.format_page_mm[0])} × {mm(report.fiche.format_page_mm[1])} mm
          </Ligne>
          <Ligne k={t("envoi.fiche.interieur")}>
            {t("envoi.fiche.interieur.v", {
              planches: report.fiche.planches,
              pages: report.fiche.pages_interieur,
            })}
          </Ligne>
          <Ligne k={t("envoi.fiche.fond")}>
            {t("envoi.fiche.fond.v", {
              haut: mm(report.fiche.fond_perdu_mm.haut),
              bas: mm(report.fiche.fond_perdu_mm.bas),
              ext: mm(report.fiche.fond_perdu_mm.exterieur),
              dos: mm(report.fiche.fond_perdu_mm.dos),
            })}
          </Ligne>
          <Ligne k={t("envoi.fiche.zone")}>{t("envoi.fiche.zone.v", { mm: mm(report.fiche.zone_sure_mm) })}</Ligne>
          <Ligne k={t("envoi.fiche.espace")}>
            {report.fiche.espace === "rgb" ? t("envoi.rvb") : t("envoi.fiche.espace.cmjn")} · {report.fiche.output_intent}
          </Ligne>
          <Ligne k={t("envoi.fiche.conformite")}>
            {report.fiche.conformite === "x4" ? t("envoi.fiche.conformite.x4") : t("envoi.fiche.conformite.aucune")}
          </Ligne>
          <Ligne k={t("envoi.fiche.livraison")}>
            {report.fiche.fichiers === "deux"
              ? t("envoi.fiche.livraison.deux")
              : t("envoi.fiche.livraison.un", { n: report.fiche.pages_fichier ?? 0 })}
          </Ligne>
          {report.fiche.feuille_couverture_mm !== undefined && (
            <Ligne k={t("envoi.fiche.feuille")}>
              {mm(report.fiche.feuille_couverture_mm[0])} × {mm(report.fiche.feuille_couverture_mm[1])} mm
            </Ligne>
          )}
          {report.fiche.dos_mm !== undefined && (
            <Ligne k={t("envoi.fiche.dos")}>
              {t("envoi.fiche.dos.v", {
                mm: mm(report.fiche.dos_mm),
                pages: report.fiche.pages_interieur,
                g: report.fiche.grammage_g_m2 ?? 0,
              })}
            </Ligne>
          )}
          <Ligne k={t("envoi.fiche.resolution")}>
            {t("envoi.fiche.resolution.v", { dpi: report.fiche.resolution_cible_dpi })}
          </Ligne>
        </dl>
        {(report.reserves?.length ?? 0) > 0 && (
          <div className="envoi-reserves">
            <h4>{t("envoi.reserves")}</h4>
            <ul>
              {report.reserves!.map((r, i) => (
                <li key={i}>{texteMoteur(r)}</li>
              ))}
            </ul>
          </div>
        )}
        {(report.notes?.length ?? 0) > 0 && (
          <div className="envoi-notes">
            {report.notes!.map((n, i) => (
              <p key={i}>{texteMoteur(n)}</p>
            ))}
          </div>
        )}
      </div>
    </details>
  );
}

function Ligne({ k, children }: { k: string; children: React.ReactNode }) {
  return (
    <>
      <dt>{k}</dt>
      <dd>{children}</dd>
    </>
  );
}

/** Ce qu'une ligne de défaut sait faire d'un clic, par destination. */
type Gestes = {
  planche: (n: number) => void;
  couverture: (choisir: boolean) => void;
  format: () => void;
  /** Absent hors du dossier préparé. */
  preparer?: () => void;
};

/** Le bouton d'une destination, ou rien quand le geste est ailleurs sur cet
 *  écran (une page à décocher), ou nulle part dans la fenêtre. */
function GesteDefaut({ d, dest, gestes }: { d: Defaut; dest: Destination | null; gestes: Gestes }) {
  const bouton = (texte: string, faire: () => void) => (
    <button type="button" className="link" onClick={faire}>
      {texte}
    </button>
  );
  switch (dest) {
    case "couverture":
      return d.regle === "couverture_resolution" || d.regle === "couverture_vide"
        ? bouton(t("envoi.geste.couverture.photo"), () => gestes.couverture(true))
        : bouton(t("envoi.geste.couverture"), () => gestes.couverture(false));
    case "format":
      return bouton(t("envoi.geste.format"), gestes.format);
    case "preparer":
      return gestes.preparer ? bouton(t("envoi.geste.preparer"), gestes.preparer) : null;
    default:
      return null;
  }
}

/** One defect. What the user reads first is the app's plain sentence, the
 *  gesture beside it; the engine's sentence, with its ppi, millimetres and
 *  file names, waits under « Détails », closed. A rule the window does not
 *  know keeps the engine's sentence in full. A defect on a spread names it
 *  with a button that jumps there, as the whole line used to: a fold inside
 *  a button would be a control inside a control. */
function DefautLigne({ d, gestes }: { d: Defaut; gestes: Gestes }) {
  const dest = destinationDe(d);
  const simple = phraseSimple(d);
  const moteur = (
    <>
      <span className="envoi-defaut-cause">{phraseDefaut(d, "cause")}</span>
      <span className="envoi-defaut-remede">{phraseDefaut(d, "remede")}</span>
    </>
  );
  const cls = "envoi-defaut" + (d.bloquant ? " bloquant" : "");
  return (
    <div className={cls}>
      {dest === "planche" ? (
        <button type="button" className="envoi-defaut-ou" onClick={() => gestes.planche(d.planche!)}>
          {libelleDestination(d)}
        </button>
      ) : (
        <span className="envoi-defaut-ou">{libelleDestination(d)}</span>
      )}
      {simple === null ? (
        moteur
      ) : (
        <>
          <span className="envoi-defaut-simple">{simple}</span>
          <span className="envoi-defaut-geste">
            <GesteDefaut d={d} dest={dest} gestes={gestes} />
          </span>
          <details className="envoi-defaut-details">
            <summary>{t("envoi.details")}</summary>
            {moteur}
          </details>
        </>
      )}
    </div>
  );
}

/** Une photo trop petite pour sa case : où elle est, à combien elle
 *  imprimerait, et les gestes qui la sortent de là. Retirer est toujours
 *  offert, parce qu'il peut n'exister aucune case assez petite. */
function ResolutionLigne({
  d,
  actions,
  onVoir,
  onRemplacer,
  onRetirer,
}: {
  d: Defaut;
  actions: ActionDefaut[];
  onVoir: (planche: number, cell: number) => void;
  onRemplacer: (planche: number, cell: number, point: { x: number; y: number }) => void;
  onRetirer: (planche: number, cell: number) => void;
}) {
  const planche = d.planche!;
  const cell = d.case!;
  const ppi = ppiDe(d);
  return (
    <div className="envoi-defaut bloquant envoi-resolution" title={phraseDefaut(d, "cause")}>
      <span className="envoi-resolution-vignette">
        <LazyThumb src={d.src!} />
      </span>
      <span className="envoi-defaut-cause">
        {ppi === null
          ? t("envoi.defaut.planche", { n: planche })
          : t("envoi.resolution.ou", { n: planche, ppi })}
      </span>
      <span className="envoi-resolution-gestes">
        {actions.includes("voir") && (
          <button type="button" className="link" onClick={() => onVoir(planche, cell)}>
            {t("envoi.resolution.voir")}
          </button>
        )}
        {actions.includes("remplacer") && (
          <button
            type="button"
            className="link"
            onClick={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              onRemplacer(planche, cell, { x: r.left, y: r.bottom + 4 });
            }}
          >
            {t("envoi.resolution.remplacer")}
          </button>
        )}
        {actions.includes("retirer") && (
          <button type="button" className="link" onClick={() => onRetirer(planche, cell)}>
            {t("envoi.resolution.retirer")}
          </button>
        )}
      </span>
    </div>
  );
}

/** La hauteur du livre dans Envoi, en pixels : la largeur et le dos suivent
 *  à la même échelle. */
const HAUTEUR_LIVRE_PX = 260;

/** Le livre, dans la colonne de droite d'Envoi : en volume (A4), le choix de
 *  son fond dessous, puis trois lignes en clair et « Feuilleter ». Rien n'y
 *  est calculé qui ne vienne du moteur : le dos est `fiche.dos_mm`, le format
 *  `fiche.format_page_mm`, le papier le profil (`envoi.ts::papierEnClair`). */
function LivreEnvoi({
  album,
  fiche,
  papier,
  onFeuilleter,
  onFond,
}: {
  album: Album;
  /** Null tant que le contrôle n'a pas rendu sa fiche : le format vient
   *  alors de l'album, et ni le dos ni les pages ne se disent. */
  fiche: Fiche | null;
  /** Le papier en clair, quand l'imprimeur est connu ; sinon rien. */
  papier: string | null;
  onFeuilleter: () => void;
  onFond: (fond: string | null) => void;
}) {
  const format: [number, number] = fiche?.format_page_mm ?? [album.trim_mm.w, album.trim_mm.h];
  return (
    <section className="envoi-livre" aria-label={t("envoi.livre")}>
      <Livre3D
        album={album}
        formatMm={format}
        dosMm={fiche?.dos_mm ?? null}
        pages={fiche?.pages_interieur ?? null}
        hauteurPx={HAUTEUR_LIVRE_PX}
      />
      <PaletteFond album={album} onFond={onFond} />
      <ul className="livre-lignes">
        <li>{formatEnCm(format)}</li>
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
