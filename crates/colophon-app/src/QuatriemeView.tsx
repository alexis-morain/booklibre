// La quatrième de couverture dans l'éditeur (1.2-d) : son texte dessiné à
// sa taille d'impression, coupé aux endroits où le PDF le coupera, et sa
// photo encadrée, recadrable comme celle de la première.
//
// **La règle des blocs libres** : partout ailleurs l'éditeur grossit le texte
// pour la lisibilité ; ici, comme pour un bloc, la largeur de la boîte décide
// des coupures, donc grossir le texte montrerait des coupures que le papier
// n'a pas. Les lignes viennent de `quatrieme.ts`, le port de
// `cover.rs::quatrieme`, mesurées sur les octets de la face qui s'imprimera
// (`font.ts`) ; elles se posent dans un SVG au repère du panneau, en
// millimètres, ligne de base comprise. Ni plancher ni plafond de taille.
//
// Le champ de saisie s'ouvre au clic, droit, dans la bande du texte, à la
// même taille et à la même largeur ; il écrit au blur, Échap referme sans
// écrire.

import { useRef, useState, useSyncExternalStore } from "react";
import type { Cover, Slot } from "./album";
import { PT_MM } from "./album";
import {
  afficher,
  faceObjetManque,
  familleObjet,
  measureMm,
  oublierFaceObjet,
  pile,
  surLaFace,
  tourDeFace,
} from "./font";
import { t, type Cle } from "./i18n";
import {
  BACK_LEADING,
  TAILLES_OFFERTES,
  TITLE_INSET,
  avecCadre,
  avecPoliceQuatrieme,
  avecTailleTexte,
  quatrieme,
  sansPhotoQuatrieme,
  tailleQuatrieme,
} from "./quatrieme";
import { PoliceOfferte, policesDeFamille, poserPoliceObjet } from "./bridge";
import { ListePolices } from "./ListePolices";
import { faceReguliere, nomLisible } from "./police";
import { CoverPhoto } from "./CoverView";

export function QuatriemePanneau({
  cover,
  form,
  trim,
  safe,
  onForm,
  onCommit,
  onCadre,
}: {
  /** La couverture enregistrée : sa photo, sa face, sa taille. */
  cover: Cover;
  /** Le formulaire local, où vit le texte en cours de saisie. */
  form: Cover;
  trim: { w: number; h: number };
  /** La zone sûre du profil choisi : sous elle, le PDF n'imprime aucune
   *  ligne, et l'éditeur n'en montre aucune. */
  safe: number;
  onForm: (texte: string) => void;
  /** Écrire le texte tel qu'il est dans le formulaire. */
  onCommit: () => void;
  /** Le recadrage de la photo, un pas d'annulation. */
  onCadre: (photo: Slot) => void;
}) {
  useSyncExternalStore(surLaFace, tourDeFace);
  const [edition, setEdition] = useState(false);
  const annule = useRef(false);
  const police = cover.back_police;
  const famille = police && !faceObjetManque(police.fichier) ? familleObjet(police) : undefined;
  const taille = tailleQuatrieme(cover, trim.w);
  const tailleMm = taille * PT_MM;
  const panneau = { x: 0, y: 0, w: trim.w, h: trim.h };
  const mesure = (s: string, pt: number) => measureMm(s, pt * PT_MM, famille);
  const vue = { ...cover, back_text: form.back_text ?? "" };
  const q = quatrieme(panneau, safe, vue, taille, mesure);
  const pct = (mm: number, total: number) => `${(mm / total) * 100}%`;
  const boiteW = trim.w * (1 - 2 * TITLE_INSET);
  const bande = q.bande;
  const style = {
    "--q-taille-mm": tailleMm,
    "--q-panneau-mm": trim.w,
    fontFamily: pile(famille),
  } as React.CSSProperties;
  const texte = form.back_text ?? "";

  return (
    <div className="cover-back" style={{ flex: trim.w, ...style }}>
      {q.cadre && cover.back_photo && (
        <div
          className="cover-back-cadre"
          style={{
            left: pct(q.cadre.x, trim.w),
            top: pct(trim.h - q.cadre.y - q.cadre.h, trim.h),
            width: pct(q.cadre.w, trim.w),
            height: pct(q.cadre.h, trim.h),
          }}
        >
          <CoverPhoto
            photo={cover.back_photo.photo}
            onChange={onCadre}
            titre={t("couverture.q.recadrer")}
          />
        </div>
      )}
      {edition ? (
        <textarea
          className="cover-back-champ"
          autoFocus
          value={texte}
          aria-label={t("couverture.quatrieme")}
          style={{
            left: pct((trim.w - boiteW) / 2, trim.w),
            width: pct(boiteW, trim.w),
            top: pct(trim.h - bande.y - bande.h, trim.h),
            height: pct(bande.h, trim.h),
            lineHeight: BACK_LEADING,
          }}
          onChange={(e) => onForm(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              e.stopPropagation();
              annule.current = true;
              e.currentTarget.blur();
            }
          }}
          onBlur={() => {
            setEdition(false);
            if (annule.current) {
              annule.current = false;
              onForm(cover.back_text ?? "");
              return;
            }
            onCommit();
          }}
        />
      ) : (
        <>
          {/* Ce que le PDF imprimera, ligne à ligne : décor, la boîte
              ci-dessous porte le nom. */}
          <svg
            className="cover-back-lignes"
            viewBox={`0 0 ${trim.w} ${trim.h}`}
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            {q.lignes.map((l, i) => (
              <text key={i} x={l.x} y={trim.h - l.y} fontSize={tailleMm}>
                {afficher(l.texte)}
              </text>
            ))}
          </svg>
          <button
            type="button"
            className={"cover-back-zone" + (texte === "" ? " vide" : "")}
            style={{
              left: pct((trim.w - boiteW) / 2, trim.w),
              width: pct(boiteW, trim.w),
              top: pct(trim.h - bande.y - bande.h, trim.h),
              height: pct(bande.h, trim.h),
            }}
            aria-label={
              texte === "" ? t("couverture.quatrieme") : t("couverture.q.aria", { texte })
            }
            onClick={() => setEdition(true)}
          >
            {texte === "" ? t("couverture.quatrieme") : null}
          </button>
        </>
      )}
    </div>
  );
}

/** La rangée sous la feuille : la photo, son cadre, la face et la taille du
 *  texte. Chaque geste est une couverture neuve, un pas d'annulation. */
export function QuatriemeReglages({
  cover,
  trimW,
  polices,
  onPolices,
  onCover,
  onChoisirPhoto,
}: {
  cover: Cover;
  trimW: number;
  polices: PoliceOfferte[];
  /** Les faces de la machine, lues à la première ouverture de la liste. */
  onPolices: () => void;
  onCover: (c: Cover) => void;
  onChoisirPhoto: () => void;
}) {
  const [liste, setListe] = useState(false);
  const [filtre, setFiltre] = useState("");
  const [erreur, setErreur] = useState<string | null>(null);
  const photo = cover.back_photo;
  const police = cover.back_police;
  const offerte = police ? polices.find((p) => p.postscript === police.postscript) : undefined;
  const defaut = tailleQuatrieme({ title: "" }, trimW);
  const taille = cover.back_taille_pt;
  const fmt = (pt: number) => String(Math.round(pt * 10) / 10).replace(".", ",");

  const poser = async (p: PoliceOfferte | null) => {
    if (!p) return;
    try {
      const posee = await poserPoliceObjet(p.rang);
      if (faceObjetManque(posee.fichier)) oublierFaceObjet(posee.fichier);
      setListe(false);
      if (police?.fichier === posee.fichier) return;
      onCover(avecPoliceQuatrieme(cover, posee));
    } catch (e) {
      setErreur(String(e));
    }
  };

  const bascule = <V extends string>(
    libelle: string,
    valeurs: readonly V[],
    courante: V,
    cle: (v: V) => Cle,
    choisir: (v: V) => void,
  ) => (
    <div className="cover-q-groupe" role="group" aria-label={libelle}>
      <span className="cover-q-libelle">{libelle}</span>
      {valeurs.map((v) => (
        <button
          key={v}
          type="button"
          className="link"
          aria-pressed={v === courante}
          onClick={() => v !== courante && choisir(v)}
        >
          {t(cle(v))}
        </button>
      ))}
    </div>
  );

  return (
    <div className="cover-q" role="group" aria-label={t("couverture.q.titre")}>
      <span className="cover-q-titre">{t("couverture.q.titre")}</span>
      <button type="button" className="link" onClick={onChoisirPhoto}>
        {photo ? t("couverture.q.photo.changer") : t("couverture.q.photo.ajouter")}
      </button>
      {photo && (
        <>
          <button type="button" className="link" onClick={() => onCover(sansPhotoQuatrieme(cover))}>
            {t("couverture.q.photo.retirer")}
          </button>
          {bascule(
            t("couverture.q.position"),
            ["haut", "centre", "bas"] as const,
            photo.position ?? "haut",
            (v) => `couverture.q.position.${v}` as Cle,
            (position) => onCover(avecCadre(cover, { position })),
          )}
          {bascule(
            t("couverture.q.taille"),
            ["petite", "moyenne", "grande"] as const,
            photo.taille ?? "moyenne",
            (v) => `couverture.q.taille.${v}` as Cle,
            (taille) => onCover(avecCadre(cover, { taille })),
          )}
        </>
      )}
      <button
        type="button"
        className="link"
        aria-expanded={liste}
        onClick={() => {
          if (!liste) onPolices();
          setListe(!liste);
        }}
      >
        {t("couverture.q.police")} : {police ? (offerte?.famille ?? nomLisible(police)) : t("couverture.q.police.livre")}
      </button>
      <label className="cover-q-groupe">
        <span className="cover-q-libelle">{t("couverture.q.texte.taille")}</span>
        <select
          value={taille === undefined ? "" : String(taille)}
          onChange={(e) =>
            onCover(avecTailleTexte(cover, e.target.value === "" ? null : Number(e.target.value)))
          }
        >
          <option value="">{t("couverture.q.texte.taille.format", { pt: fmt(defaut) })}</option>
          {TAILLES_OFFERTES.map((pt) => (
            <option key={pt} value={String(pt)}>
              {t("couverture.q.texte.taille.pt", { pt: fmt(pt) })}
            </option>
          ))}
          {taille !== undefined && !TAILLES_OFFERTES.includes(taille) && (
            <option value={String(taille)}>{t("couverture.q.texte.taille.pt", { pt: fmt(taille) })}</option>
          )}
        </select>
      </label>
      {police && faceObjetManque(police.fichier) && (
        <p className="cover-q-alerte">{t("couverture.q.police.manque")}</p>
      )}
      {erreur && <p className="cover-q-alerte">{erreur}</p>}
      {liste && (
        <div className="cover-q-polices">
          <ListePolices
            polices={polices}
            projet={{
              libelle: t("couverture.q.police.livre"),
              note: t("couverture.q.police.livre.note"),
              choisi: !police,
              onChoisir: () => {
                setListe(false);
                if (police) onCover(avecPoliceQuatrieme(cover, null));
              },
            }}
            estChoisie={(p, genre) =>
              !!police &&
              (genre === "voix" ? p.famille === offerte?.famille : p.postscript === police.postscript)
            }
            onVoix={(p) =>
              void policesDeFamille(p.famille).then(
                (faces) => poser(faceReguliere(faces)),
                (e) => setErreur(String(e)),
              )
            }
            onFace={(p) => void poser(p)}
            filtre={filtre}
            onFiltre={setFiltre}
          />
        </div>
      )}
    </div>
  );
}
