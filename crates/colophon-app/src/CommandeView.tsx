// Commander, dans Envoi, sous « Préparer » (S-s3).
//
// Le parcours : un pays, la grille des prix du relais, l'envoi des deux PDF
// avec leurs barres, la confirmation qui dit le prix et le mode en toutes
// lettres, puis la page de paiement de Stripe dans le navigateur du système.
// L'écran attend en relisant l'état toutes les cinq secondes, et la liste
// des commandes prend le relais quand le paiement a abouti.
//
// Après « Faire imprimer » (A1), le parcours s'ouvre seul, sans « Commander »,
// la grille se demande d'elle-même quand le relais n'ouvre qu'un pays, et le
// bouton de la grille enchaîne l'envoi et la confirmation : trois gestes
// jusqu'à Stripe.
//
// A3 : le parcours se montre en six étapes, d'un bandeau en tête (Votre
// livre · Livraison · Prix · Fichiers · Paiement · Suivi), la machine gardant
// ses états (`commande.ts::etapeVisible`). « Votre livre » est l'écran de la
// préparation, barre comprise ; « Suivi » est la carte de la commande, avec la
// frise de ses jalons. Ce qui rassure n'est pas ici mais à côté, dans un
// encadré fixe (`Rassurance`), et chaque phrase y est un fait que le relais
// tient.
//
// Rien ne s'affiche sans relais compilé, sans dossier préparé dans cette
// ouverture d'Envoi ou sans contrôle vert (`commandePossible`) : jamais un
// bouton grisé. Un relais compilé qui ne répond pas se dit, à la place de
// Commander, avec « Réessayer » (`relaisIndisponible`). La liste des commandes, elle, se montre dès qu'il y en a une,
// et se relit à chaque ouverture d'Envoi.

import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { Dialogue } from "./Dialogue";
import type { Album } from "./album";
import { formatEnCm } from "./envoi";
import { MiniCouverture } from "./PlanchesView";
import {
  Fiche,
  Preparation,
  ouvrirPaiement,
  relaisAnnuler,
  relaisCreer,
  relaisEnvoyer,
  relaisNiveau,
  relaisOublier,
  relaisPays,
  relaisRelire,
} from "./bridge";
import {
  ETAPES,
  IntentionVue,
  JALONS,
  Niveau,
  Parcours,
  Relais,
  aRelire,
  abandonnable,
  dejaPayee,
  ignorable,
  paiementRecu,
  payable,
  refusDefinitif,
  annulableIntention,
  avancer,
  commandeDirecte,
  commandePossible,
  etapesDuParcours,
  formatInvendable,
  jalonsAtteints,
  niveauChoisi,
  nomNiveau,
  part,
  phraseConfirmation,
  phraseErreur,
  phraseIntention,
  phraseLivraison,
  prixDeCentimes,
  referenceCourte,
  relaisDe,
  relaisIndisponible,
  visibles,
} from "./commande";
import { t, useLangue } from "./i18n";

/** Cinq secondes entre deux relectures d'un paiement qui attend. */
const RELECTURE_MS = 5000;

const mo = (octets: number) => (octets / 1e6).toFixed(1);

/** Le relais répond-il ? Sans relais compilé, `relaisPays` rend null sans
 *  un appel ; un échec, lui, est un relais compilé qui ne répond pas, et
 *  l'écran le dit au lieu de se taire. Tenu par Envoi, qui en tire le nom de
 *  son bouton (« Faire imprimer »), et passé à `CommandeView`. `relais` est
 *  null tant que la première réponse n'est pas venue. */
export function useRelais(): { relais: Relais | null; sonde: boolean; sonder: () => void } {
  const [relais, setRelais] = useState<Relais | null>(null);
  const [sonde, setSonde] = useState(false);
  const vivant = useRef(true);
  const sonder = useCallback(() => {
    setSonde(true);
    relaisPays().then(
      (p) => {
        if (!vivant.current) return;
        const r = relaisDe(p);
        // La release publique n'en porte aucun : ce n'est pas une panne,
        // mais au harnais comme dans une release de test on doit le voir.
        if (r.etat === "absent") console.info("colophon : aucun relais compilé, Commander ne s'offre pas");
        setRelais(r);
        setSonde(false);
      },
      () => vivant.current && (setRelais({ etat: "injoignable" }), setSonde(false)),
    );
  }, []);
  useEffect(() => {
    vivant.current = true;
    sonder();
    return () => {
      vivant.current = false;
    };
  }, [sonder]);
  return { relais, sonde, sonder };
}

export function CommandeView({
  preparationNeuve,
  relais,
  sonde,
  sonder,
  album,
  fiche,
  preparant,
  progres,
  onGrille,
}: {
  preparationNeuve: Preparation | null;
  relais: Relais | null;
  sonde: boolean;
  sonder: () => void;
  album: Album;
  /** La fiche du contrôle, pour résumer le livre commandé. */
  fiche: Fiche | null;
  /** « Faire imprimer » prépare les fichiers en ce moment : l'étape « Votre
   *  livre » se montre avant tout état du parcours. */
  preparant: boolean;
  /** La préparation, en pages rendues. */
  progres: { done: number; total: number } | null;
  /** Une grille est venue : Envoi en tire son « à partir de ». */
  onGrille: (grille: Niveau[]) => void;
}) {
  const langue = useLangue();
  const [intentions, setIntentions] = useState<IntentionVue[]>([]);
  const [parcours, envoyer] = useReducer(avancer, { etape: "repos" });
  const [occupe, setOccupe] = useState<string | null>(null);
  const [erreur, setErreur] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  // La commande qu'un paiement vient de faire aboutir : le bandeau reste, à
  // l'étape Suivi, jusqu'à « Fermer » ou la commande suivante.
  const [suivi, setSuivi] = useState<string | null>(null);
  const parcoursRef = useRef(parcours);
  parcoursRef.current = parcours;

  const recevoir = useCallback((liste: IntentionVue[]) => {
    setIntentions(liste);
    const p = parcoursRef.current;
    for (const i of liste) {
      if (p.etape === "attente" && i.id === p.offre.id && dejaPayee(i.etat)) setSuivi(i.id);
      envoyer({ type: "etat", intention: i });
    }
  }, []);

  // À l'ouverture d'Envoi : où en sont les commandes.
  useEffect(() => {
    let vivant = true;
    relaisRelire().then(
      (r) => {
        if (!vivant) return;
        recevoir(r.intentions);
        if (r.erreur) setErreur(phraseErreur(r.erreur));
      },
      () => {},
    );
    return () => {
      vivant = false;
    };
  }, [recevoir]);

  // Tant que l'écran est ouvert, ce qui attend se relit toutes les cinq
  // secondes. L'effet tombe avec la dernière intention en attente.
  const enAttente = aRelire(intentions).join(",");
  useEffect(() => {
    if (!enAttente) return;
    const minuterie = setInterval(() => {
      for (const id of enAttente.split(",")) {
        relaisRelire(id).then((r) => recevoir(r.intentions), () => {});
      }
    }, RELECTURE_MS);
    return () => clearInterval(minuterie);
  }, [enAttente, recevoir]);

  const pays = relais?.etat === "joignable" ? relais.pays : null;
  const joignable = pays !== null;
  const indisponible = relais !== null && relaisIndisponible(relais, preparationNeuve);
  const possible = commandePossible(joignable, preparationNeuve);
  const invendable = formatInvendable(joignable, preparationNeuve);
  const prix = (c: number) => prixDeCentimes(c, langue);

  const tenter = async (quoi: string, geste: () => Promise<void>, sinon?: (e: unknown) => void) => {
    setOccupe(quoi);
    setErreur(null);
    try {
      await geste();
    } catch (e) {
      setErreur(phraseErreur(e));
      sinon?.(e);
    } finally {
      setOccupe(null);
    }
  };

  // Un échec rend le choix du pays, d'où « Voir les prix » réessaie.
  const voirLesPrix = () => {
    if (parcours.etape !== "pays") return;
    void tenter(
      t("commande.devis.encours"),
      async () => {
        const offre = await relaisCreer(parcours.choisi, parcours.region);
        onGrille(offre.grille);
        envoyer({ type: "offre", offre });
      },
      () => envoyer({ type: "echec" }),
    );
  };

  // « Faire imprimer » a préparé ce dossier : le parcours s'ouvre seul, une
  // fois par préparation. Revenu au repos, « Commander » le rouvre.
  const direct = commandeDirecte(joignable, preparationNeuve);
  const [ouvertPour, setOuvertPour] = useState<Preparation | null>(null);
  useEffect(() => {
    if (!direct || !pays || parcours.etape !== "repos" || ouvertPour === preparationNeuve) return;
    setOuvertPour(preparationNeuve);
    setSuivi(null);
    envoyer({ type: "commander", pays });
  }, [direct, pays, parcours.etape, ouvertPour, preparationNeuve]);

  // Un seul pays, rien à choisir : la grille se demande tout de suite. Une
  // fois par étape, le double rendu de développement compris : chaque
  // demande crée une intention au relais.
  const demandee = useRef<Parcours | null>(null);
  useEffect(() => {
    if (parcours.etape !== "pays" || !parcours.saute || demandee.current === parcours) return;
    demandee.current = parcours;
    voirLesPrix();
  });

  // Le niveau se garde dès le choix : la liste rouvre le paiement avec lui.
  const choisirNiveau = (id: string, niveau: string) => {
    envoyer({ type: "niveau", niveau });
    relaisNiveau(id, niveau).catch(() => {});
  };

  const envoyerLesFichiers = async () => {
    if (parcours.etape !== "grille") return;
    const { offre, niveau } = parcours;
    envoyer({ type: "envoyer" });
    setErreur(null);
    try {
      await relaisNiveau(offre.id, niveau);
      await relaisEnvoyer(offre.id, (p) => envoyer({ type: "progres", progres: p }));
      envoyer({ type: "envoye" });
    } catch (e) {
      // Un second envoi pendant le premier : le moteur l'a refusé sans rien
      // toucher, le premier continue.
      if (ignorable(e)) return;
      setErreur(phraseErreur(e));
      envoyer({ type: "echec" });
    }
  };

  /** Le paiement refusé parce qu'il a déjà eu lieu : la session payée
   *  (`paiement_recu`), ou le webhook passé avant le clic (`etat (409)` sur
   *  une intention payée, relue pour le savoir). */
  const payeeEntreTemps = async (e: unknown, id: string) => {
    if (paiementRecu(e)) return true;
    if (!String(e).includes("etat (409)")) return false;
    const elle = (await relaisRelire(id)).intentions.find((i) => i.id === id);
    return elle !== undefined && dejaPayee(elle.etat);
  };

  const payer = () => {
    if (parcours.etape !== "confirmation" && parcours.etape !== "attente") return;
    const { offre, niveau } = parcours;
    envoyer({ type: "payer" });
    void tenter(
      t("commande.payer.encours"),
      async () => {
        // Déjà payée : pas d'erreur, l'attente le dit et la relecture suit.
        try {
          await ouvrirPaiement(offre.id, niveau);
          envoyer({ type: "ouvert" });
        } catch (e) {
          if (!(await payeeEntreTemps(e, offre.id))) throw e;
          envoyer({ type: "recu" });
        }
        recevoir((await relaisRelire(offre.id)).intentions);
      },
      // Un refus définitif mène à l'étape d'erreur, d'où l'on recommence ;
      // le reste revient au bouton qui rouvre le paiement.
      (e) => envoyer(refusDefinitif(e) ? { type: "echec", definitif: phraseErreur(e) } : { type: "echec" }),
    );
  };

  /** « Payer » depuis la liste : le niveau gardé, la même porte. */
  const payerDepuisLaListe = (i: IntentionVue) => {
    setMessage(null);
    void tenter(t("commande.payer.encours"), async () => {
      try {
        await ouvrirPaiement(i.id, i.niveau!);
      } catch (e) {
        if (!(await payeeEntreTemps(e, i.id))) throw e;
        setMessage(t("commande.attente.recu"));
      }
      recevoir((await relaisRelire(i.id)).intentions);
    });
  };

  // Le moteur demande l'abandon au relais avant d'oublier : la phrase de
  // confirmation ne vient qu'après lui. Un refus garde l'intention, et la
  // liste se relit.
  const abandonner = (id: string) => {
    setMessage(null);
    void tenter(
      t("commandes.abandonner"),
      async () => {
        setIntentions(await relaisOublier(id));
        setMessage(t("commandes.abandonnee"));
      },
      () => void relaisRelire(id).then((r) => recevoir(r.intentions), () => {}),
    );
  };

  const annuler = (id: string) =>
    void tenter(
      t("commandes.annuler"),
      async () => recevoir((await relaisAnnuler(id)).intentions),
      // Un refus (409 compris) dit pourquoi, puis l'état se relit : l'écran
      // ne garde pas un état périmé.
      () => void relaisRelire(id).then((r) => recevoir(r.intentions), () => {}),
    );

  const liste = visibles(intentions, "offre" in parcours ? parcours.offre.id : null);
  // Entre la fin de la préparation et l'ouverture du parcours, « Votre
  // livre » reste à l'écran : le bandeau ne clignote pas.
  const ouvrira = direct && parcours.etape === "repos" && ouvertPour !== preparationNeuve;
  const pas = etapesDuParcours(parcours, { preparation: preparant || ouvrira, suivi: suivi !== null });
  const ici = pas?.find((x) => x.etat === "courante")?.etape ?? null;
  const offreEnCours = "offre" in parcours ? parcours.offre : null;
  const sandbox = offreEnCours?.mode === "sandbox" || (suivi !== null && intentions.find((i) => i.id === suivi)?.mode === "sandbox");

  return (
    <>
      {invendable && parcours.etape === "repos" && (
        <p className="commande-format" role="status">
          {t("commande.format.invendable")}
        </p>
      )}

      {indisponible && parcours.etape === "repos" && (
        <div className="commande-ouvrir">
          <p className="commande-erreur" role="status">
            {t("commande.indisponible")}
          </p>
          <button className="link" disabled={sonde} onClick={() => sonder()}>
            {t("commande.reessayer")}
          </button>
        </div>
      )}

      {possible && parcours.etape === "repos" && suivi === null && (!direct || ouvertPour === preparationNeuve) && (
        <div className="commande-ouvrir">
          <button
            className="envoi-exporter"
            title={t("commande.commander.titre")}
            onClick={() => {
              if (!pays) return;
              setSuivi(null);
              envoyer({ type: "commander", pays });
            }}
          >
            {t("commande.commander")}
          </button>
        </div>
      )}

      {pas !== null && (
        <section className="commande" aria-label={t("commande.titre")}>
          <h3>{t("commande.titre")}</h3>

          <ol className="etapes" aria-label={t("commande.etapes")}>
            {pas.map((x) => {
              const nom = t(`commande.etape.${x.etape}`);
              const libelle =
                x.etat === "passee"
                  ? t("commande.etape.faite", { etape: nom })
                  : x.etat === "courante"
                    ? t("commande.etape.courante", { etape: nom })
                    : nom;
              const contenu = (
                <>
                  <span className="etape-signe" aria-hidden="true">
                    {x.etat === "passee" ? "✓" : ETAPES.indexOf(x.etape) + 1}
                  </span>
                  <span className="etape-nom">{nom}</span>
                </>
              );
              return (
                <li
                  key={x.etape}
                  className={"etape " + x.etat}
                  aria-current={x.etat === "courante" ? "step" : undefined}
                  aria-label={libelle}
                >
                  {x.retour ? (
                    <button
                      type="button"
                      title={t("commande.etape.revenir")}
                      disabled={occupe !== null}
                      onClick={() => envoyer(x.retour!)}
                    >
                      {contenu}
                    </button>
                  ) : (
                    <span>{contenu}</span>
                  )}
                </li>
              );
            })}
          </ol>

          {sandbox && (
            <p className="commande-sandbox" role="note">
              {t("commande.sandbox.bandeau")}
            </p>
          )}

          {ici === "livre" && (
            <VotreLivre album={album} fiche={fiche} preparant={preparant} progres={progres} />
          )}
          {pas.some((x) => x.etape === "livre" && x.etat === "passee") && ici !== "suivi" && (
            <p className="commande-resume">
              <span className="commande-resume-coche" aria-hidden="true">✓</span>
              {resumeLivre(fiche)}
            </p>
          )}

          {ici === "suivi" && (
            <div className="commande-gestes">
              <button className="link" onClick={() => setSuivi(null)}>
                {t("commande.fermer")}
              </button>
            </div>
          )}

          {parcours.etape === "pays" && parcours.saute && occupe === null && (
            <p className="envoi-sub" role="status">
              {t("commande.devis.encours")}
            </p>
          )}

          {parcours.etape === "pays" && !parcours.saute && (
            <>
              <div className="commande-grille">
                <label htmlFor="commande-pays">{t("commande.pays")}</label>
                <select
                  id="commande-pays"
                  value={parcours.choisi}
                  onChange={(e) => envoyer({ type: "pays", code: e.target.value })}
                >
                  {parcours.pays.map((p) => (
                    <option key={p.code} value={p.code}>
                      {p.nom}
                    </option>
                  ))}
                </select>
                {parcours.region !== null && (
                  <>
                    <label htmlFor="commande-region">{t("commande.region")}</label>
                    <select
                      id="commande-region"
                      value={parcours.region}
                      onChange={(e) => envoyer({ type: "region", code: e.target.value })}
                    >
                      {(parcours.pays.find((p) => p.code === parcours.choisi)?.etats ?? []).map((r) => (
                        <option key={r.code} value={r.code}>
                          {r.nom}
                        </option>
                      ))}
                    </select>
                  </>
                )}
              </div>
              <div className="commande-gestes">
                <button className="envoi-exporter" disabled={occupe !== null} onClick={voirLesPrix}>
                  {t("commande.voir")}
                </button>
                <button className="link" onClick={() => envoyer({ type: "retour" })}>
                  {t("commande.abandonner")}
                </button>
              </div>
            </>
          )}

          {parcours.etape === "grille" && parcours.livraison && (
            <p className="commande-livraison">
              <span>{phraseLivraison(parcours.livraison)}</span>
              <button className="link" disabled={occupe !== null} onClick={() => envoyer({ type: "changer" })}>
                {t("commande.changer")}
              </button>
            </p>
          )}

          {(parcours.etape === "grille" || parcours.etape === "envoi") && (
            <fieldset className="commande-offres" disabled={parcours.etape === "envoi"}>
              <legend>{t("commande.grille")}</legend>
              {parcours.offre.grille.map((n) => (
                <label key={n.niveau}>
                  <input
                    type="radio"
                    name="commande-niveau"
                    checked={n.niveau === parcours.niveau}
                    onChange={() => choisirNiveau(parcours.offre.id, n.niveau)}
                  />
                  <span className="commande-offre-nom">{nomNiveau(n)}</span>
                  <span className="commande-offre-prix">{prix(n.prix_ttc_centimes)}</span>
                  {n.delai_jours !== null && (
                    <span className="commande-offre-delai">{t("commande.delai", { n: n.delai_jours })}</span>
                  )}
                </label>
              ))}
            </fieldset>
          )}

          {parcours.etape === "grille" && (
            <div className="commande-gestes">
              <button
                className="envoi-exporter"
                title={t("commande.continuer.titre")}
                disabled={occupe !== null}
                onClick={() => void envoyerLesFichiers()}
              >
                {t("commande.continuer", { prix: prix(niveauChoisi(parcours).prix_ttc_centimes) })}
              </button>
              <button className="link" onClick={() => envoyer({ type: "retour" })}>
                {t("commande.abandonner")}
              </button>
            </div>
          )}

          {parcours.etape === "envoi" && (
            <div className="commande-envoi">
              <p className="commande-explique" role="status">
                {t("commande.explique.envoi")}
              </p>
              {(["interieur", "couverture"] as const).map((q) => (
                <div key={q} className="commande-barre">
                  <label htmlFor={`commande-barre-${q}`}>{t(`commande.envoi.${q}`)}</label>
                  <progress id={`commande-barre-${q}`} max={1} value={part(parcours.progres[q])} />
                  <span className="commande-barre-mo">
                    {t("commande.envoi.mo", {
                      fait: mo(parcours.progres[q].envoyes),
                      total: mo(parcours.progres[q].total),
                    })}
                  </span>
                </div>
              ))}
            </div>
          )}

          {parcours.etape === "pret" && (
            <div className="commande-gestes">
              <button className="envoi-exporter" onClick={() => envoyer({ type: "confirmer" })}>
                {t("commande.payer", { prix: prix(niveauChoisi(parcours).prix_ttc_centimes) })}
              </button>
              <button className="link" onClick={() => envoyer({ type: "retour" })}>
                {t("commande.abandonner")}
              </button>
            </div>
          )}

          {parcours.etape === "erreur" && (
            <>
              <p className="commande-erreur" role="alert">
                {parcours.phrase}
              </p>
              <div className="commande-gestes">
                <button
                  className="envoi-exporter"
                  disabled={!pays?.length}
                  onClick={() => pays && envoyer({ type: "recommencer", pays })}
                >
                  {t("commande.recommencer")}
                </button>
                <button className="link" onClick={() => envoyer({ type: "retour" })}>
                  {t("commande.abandonner")}
                </button>
              </div>
            </>
          )}

          {parcours.etape === "attente" && (
            <div className="commande-attente" role="status">
              <p>{t(parcours.recu ? "commande.attente.recu" : "commande.attente")}</p>
              <div className="commande-gestes">
                {!parcours.recu && (
                  <button className="link" disabled={occupe !== null} onClick={payer}>
                    {t("commande.attente.rouvrir")}
                  </button>
                )}
                <button className="link" onClick={() => envoyer({ type: "fermer" })}>
                  {t("commande.fermer")}
                </button>
              </div>
            </div>
          )}

          {occupe && parcours.etape !== "envoi" && (
            <p className="envoi-sub" role="status">
              {occupe}
            </p>
          )}
          {erreur && parcours.etape !== "erreur" && (
            <p className="commande-erreur" role="alert">
              {erreur}
            </p>
          )}

          {parcours.etape === "confirmation" && (
            <Dialogue
              titre={t("commande.confirmer.titre")}
              onClose={() => envoyer({ type: "retour" })}
              panelClassName="commande-confirmation"
            >
              <p className="commande-confirmation-phrase">
                {phraseConfirmation(prix(niveauChoisi(parcours).prix_ttc_centimes), parcours.offre.mode)}
              </p>
              <p className="envoi-sub">{nomNiveau(niveauChoisi(parcours))}</p>
              <p className="commande-explique">{t("commande.explique.paiement")}</p>
              <div className="commande-confirmation-gestes">
                <button className="envoi-exporter" onClick={payer}>
                  {t("commande.payer.ouvrir")}
                </button>
                <button className="link" onClick={() => envoyer({ type: "retour" })}>
                  {t("commande.retour")}
                </button>
              </div>
            </Dialogue>
          )}
        </section>
      )}

      {message && (
        <p className="envoi-sub" role="status">
          {message}
        </p>
      )}
      {parcours.etape === "repos" && erreur && (
        <p className="commande-erreur" role="alert">
          {erreur}
        </p>
      )}

      {liste.length > 0 && (
        <section className="commandes">
          <h3>{t("commandes.titre")}</h3>
          <ul>
            {(suivi ? [...liste].sort((a, b) => Number(b.id === suivi) - Number(a.id === suivi)) : liste).map((i) => (
              <CarteCommande key={i.id} i={i} langue={langue}>
                {(annulableIntention(i) || payable(i) || abandonnable(i)) && (
                  <span className="commandes-gestes">
                    {payable(i) && (
                      <button className="link" disabled={occupe !== null} onClick={() => payerDepuisLaListe(i)}>
                        {t("commandes.payer")}
                      </button>
                    )}
                    {annulableIntention(i) && (
                      <button className="link" disabled={occupe !== null} onClick={() => annuler(i.id)}>
                        {t("commandes.annuler")}
                      </button>
                    )}
                    {abandonnable(i) && (
                      <button
                        className="link"
                        disabled={occupe !== null}
                        title={t("commandes.abandonner.note")}
                        onClick={() => abandonner(i.id)}
                      >
                        {t("commandes.abandonner")}
                      </button>
                    )}
                  </span>
                )}
                {abandonnable(i) && <span className="commandes-note">{t("commandes.abandonner.note")}</span>}
              </CarteCommande>
            ))}
          </ul>
        </section>
      )}
    </>
  );
}

/** Une ligne pour le livre une fois l'étape passée. */
function resumeLivre(fiche: Fiche | null): string {
  const morceaux = fiche
    ? [formatEnCm(fiche.format_page_mm), t("envoi.livre.pages", { n: fiche.pages_interieur })]
    : [];
  return [...morceaux, t("commande.livre.verifies")].join(" · ");
}

/** La première étape : ce qu'on commande, et la préparation de ses
 *  fichiers, avec sa barre, avant qu'aucun appel au relais ne parte. */
function VotreLivre({
  album,
  fiche,
  preparant,
  progres,
}: {
  album: Album;
  fiche: Fiche | null;
  preparant: boolean;
  progres: { done: number; total: number } | null;
}) {
  return (
    <div className="commande-livre">
      <div className="commande-livre-plat" aria-hidden="true">
        <MiniCouverture album={album} />
      </div>
      <div className="commande-livre-quoi">
        {fiche && (
          <p>
            {formatEnCm(fiche.format_page_mm)} · {t("envoi.livre.pages", { n: fiche.pages_interieur })}
          </p>
        )}
        {preparant ? (
          <div className="commande-barre" role="status">
            <label htmlFor="commande-barre-preparation">{t("commande.livre.preparation")}</label>
            <progress
              id="commande-barre-preparation"
              max={1}
              value={progres && progres.total > 0 ? Math.min(1, progres.done / progres.total) : undefined}
            />
          </div>
        ) : (
          <p className="commande-livre-verifies">
            <span aria-hidden="true">✓ </span>
            {t("commande.livre.verifies")}
          </p>
        )}
        <p className="commande-explique">{t("commande.explique.preparation")}</p>
      </div>
    </div>
  );
}

/** Une date de création, `AAAA-MM-JJ`, dite dans la langue de l'écran. */
function dateDite(iso: string | undefined, langue: string): string | null {
  if (!iso || !/^\d{4}-\d{2}-\d{2}/.test(iso)) return null;
  const d = new Date(`${iso.slice(0, 10)}T12:00:00Z`);
  return new Intl.DateTimeFormat(langue === "fr" ? "fr-FR" : "en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(d);
}

/** Une commande, en carte : sa référence courte, sa date, « test » en
 *  sandbox, la frise de ses quatre jalons, sa phrase et ses gestes. Un état
 *  inconnu garde la frise, sans jalon, et sa phrase neutre. */
function CarteCommande({ i, langue, children }: { i: IntentionVue; langue: string; children?: React.ReactNode }) {
  const n = jalonsAtteints(i);
  const date = dateDite(i.date, langue);
  return (
    <li className="commande-carte">
      <div className="commande-carte-tete">
        <span className="commandes-ref" title={i.id}>
          {referenceCourte(i.id)}
        </span>
        {date && <span className="commande-carte-date">{date}</span>}
        {i.mode === "sandbox" && <span className="commande-badge">{t("commande.badge.test")}</span>}
      </div>
      <ol className="jalons" aria-label={t("commande.jalons")}>
        {JALONS.map((j, k) => (
          <li
            key={j}
            className={"jalon" + (k < n ? " atteint" : "") + (k === n - 1 ? " dernier" : "")}
            aria-current={k === n - 1 ? "step" : undefined}
          >
            <span className="jalon-point" aria-hidden="true">
              {k < n ? "✓" : ""}
            </span>
            <span className="jalon-nom">{t(`commande.jalon.${j}`)}</span>
          </li>
        ))}
      </ol>
      <p className="commandes-etat" role="status">
        {phraseIntention({ ...i, id: referenceCourte(i.id) })}
      </p>
      {children}
    </li>
  );
}

/** Ce qui se passe, en faits que le relais tient (A3, décision 10). Chaque
 *  phrase a sa preuve dans `colophon-relais` : Stripe Checkout encaisse sur
 *  sa page (le relais ne voit qu'une session) ; rien n'est débité sans
 *  session payée ; les fichiers montent en `https` (l'app refuse toute autre
 *  URL de dépôt) et le balayage quotidien retire les objets dès l'état 100
 *  ou à 30 jours ; l'annulation rembourse sous l'état 30 ; la France n'est
 *  dite que quand le relais n'ouvre qu'elle. Rien sur le suivi du colis ni
 *  sur un délai : la grille ne promet ni l'un ni l'autre. Et aucun lien vers
 *  Signaler (A4, décision 10) : il crée une issue GitHub publique, et
 *  quelqu'un y citerait sa commande. */
export function Rassurance({ enFrance }: { enFrance: boolean }) {
  return (
    <aside className="rassurance" aria-label={t("commande.rassure.titre")}>
      <h3>{t("commande.rassure.titre")}</h3>
      <ul>
        <li>{t("commande.rassure.stripe")}</li>
        <li>{t("commande.rassure.debit")}</li>
        <li>{t("commande.rassure.fichiers")}</li>
        <li>{t("commande.rassure.annuler")}</li>
        {enFrance && <li>{t("commande.rassure.france")}</li>}
      </ul>
    </aside>
  );
}
