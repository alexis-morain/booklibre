// Le livre en volume (A4) : six faces en CSS 3D (`preserve-3d`), aucune
// dépendance. Il remplace le plat en perspective posé par A2 dans Envoi, et
// se montre en petit dans l'éditeur de couverture.
//
// Ce qu'il montre vient du moteur : le format de la page, le dos de la fiche
// (`fiche.dos_mm`) à la même échelle, le fond de la couverture et l'encre que
// `cover.rs` posera dessus (`couleur.ts::encreSurFond`). L'avant est la
// vignette de couverture, le dos et l'arrière l'aplat du fond, la tranche, le
// dessus et le dessous le bloc de pages.
//
// Il tourne au glisser (souris et trackpad, glisser ou balayage horizontal à
// deux doigts) et aux flèches quand il a le focus ; relâché, il revient au
// trois-quarts par le chemin court (`volume.ts::repos`), en douceur. Sous
// `prefers-reduced-motion`, ni glisser ni transition : trois vues fixes,
// l'avant, le trois-quarts et le dos, au clic.
//
// La visionneuse (1.2-c) le montre fermé, avant de l'ouvrir : elle lui donne
// son avant tiré du PDF de couverture (`avant`), et le met de face
// (`deFace`) le temps que le plat se pose à la place de la page de droite.

import { useEffect, useRef, useState } from "react";
import { Album, SPINE_TEXT_MIN_MM } from "./album";
import { couleurValide, encreSurFond } from "./couleur";
import { formatEnCm } from "./envoi";
import { nomDuFond } from "./fond";
import { afficher } from "./font";
import { t } from "./i18n";
import {
  ANGLE_REPOS,
  INCLINAISON_REPOS,
  apresFleche,
  cotesLivre,
  repos,
  vueSuivante,
} from "./volume";
import { MiniCouverture } from "./PlanchesView";

/** Degrés par pixel de glisser. */
const SENSIBILITE = 0.6;
/** Le balayage à deux doigts se tait si longtemps avant le retour au repos. */
const REPOS_APRES_BALAYAGE_MS = 350;

export function Livre3D({
  album,
  formatMm,
  dosMm,
  pages,
  hauteurPx,
  avant,
  deFace,
}: {
  album: Album;
  /** Le format de la page fini, en millimètres. */
  formatMm: [number, number];
  /** Le dos de la fiche ; null tant qu'il n'est pas connu. */
  dosMm: number | null;
  /** Les pages de l'intérieur, null tant que la fiche n'est pas rendue. */
  pages: number | null;
  hauteurPx: number;
  /** Ce que porte l'avant ; la vignette de couverture quand rien n'est
   *  donné. */
  avant?: React.ReactNode;
  /** Vrai : le livre se met de face, angle et inclinaison à zéro. Faux : il
   *  revient au trois-quarts. Absent : il est seul maître de sa pose. */
  deFace?: boolean;
}) {
  const { largeur: w, hauteur: h, epaisseur: e } = cotesLivre(
    { wMm: formatMm[0], hMm: formatMm[1], dosMm },
    hauteurPx,
  );
  const cover = album.cover ?? { title: album.title };
  const fond = cover.fond && couleurValide(cover.fond) ? cover.fond.toLowerCase() : null;
  const encre = encreSurFond(fond);
  const aplat = fond ?? "var(--livre-papier)";
  const titre = afficher(cover.title || album.title);

  const [angle, setAngle] = useState(deFace ? 0 : ANGLE_REPOS);
  const [incl, setIncl] = useState(deFace ? 0 : INCLINAISON_REPOS);
  const [enMain, setEnMain] = useState(false);
  const [reduit, setReduit] = useState(false);
  const geste = useRef<{ x: number; y: number; a: number; i: number } | null>(null);
  const racine = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const suivre = () => setReduit(mq.matches);
    suivre();
    mq.addEventListener("change", suivre);
    return () => mq.removeEventListener("change", suivre);
  }, []);

  useEffect(() => {
    if (deFace === undefined) return;
    if (deFace) {
      setAngle(0);
      setIncl(0);
    } else {
      setAngle((a) => repos(a));
      setIncl(INCLINAISON_REPOS);
    }
  }, [deFace]);

  // Le balayage horizontal à deux doigts d'un trackpad, sans voler le
  // défilement vertical : l'écouteur est posé à la main, non passif.
  useEffect(() => {
    const el = racine.current;
    if (!el || reduit) return;
    let minuterie: ReturnType<typeof setTimeout> | undefined;
    const surRoue = (ev: WheelEvent) => {
      if (Math.abs(ev.deltaX) <= Math.abs(ev.deltaY)) return;
      ev.preventDefault();
      setEnMain(true);
      setAngle((a) => a - ev.deltaX * SENSIBILITE * 0.5);
      clearTimeout(minuterie);
      minuterie = setTimeout(() => {
        setEnMain(false);
        setAngle((a) => repos(a));
      }, REPOS_APRES_BALAYAGE_MS);
    };
    el.addEventListener("wheel", surRoue, { passive: false });
    return () => {
      el.removeEventListener("wheel", surRoue);
      clearTimeout(minuterie);
    };
  }, [reduit]);

  const relacher = () => {
    if (!geste.current) return;
    geste.current = null;
    setEnMain(false);
    setAngle((a) => repos(a));
    setIncl(INCLINAISON_REPOS);
  };

  const pagesTexte = pages !== null ? t("envoi.livre.pages", { n: pages }) : null;
  const etiquette = t(pagesTexte ? "livre3d.aria" : "livre3d.aria.sans_pages", {
    format: formatEnCm(formatMm),
    pages: pagesTexte ?? "",
    fond: nomDuFond(fond),
  });

  const face = (classe: string, style: React.CSSProperties, enfants?: React.ReactNode) => (
    <div className={`livre3d-face livre3d-${classe}`} style={style}>
      {enfants}
    </div>
  );

  return (
    <div
      ref={racine}
      className={
        "livre3d" +
        (enMain ? " en-main" : "") +
        (reduit ? " reduit" : "") +
        (deFace ? " de-face" : "")
      }
      role="img"
      aria-label={etiquette}
      title={t(reduit ? "livre3d.titre.reduit" : "livre3d.titre")}
      tabIndex={0}
      style={{ width: w + e * 2, height: h + e * 2 }}
      onPointerDown={(ev) => {
        if (reduit || ev.button !== 0) return;
        ev.currentTarget.setPointerCapture?.(ev.pointerId);
        geste.current = { x: ev.clientX, y: ev.clientY, a: angle, i: incl };
        setEnMain(true);
      }}
      onPointerMove={(ev) => {
        const g = geste.current;
        if (!g) return;
        setAngle(g.a + (ev.clientX - g.x) * SENSIBILITE);
        setIncl(Math.max(-40, Math.min(25, g.i - (ev.clientY - g.y) * SENSIBILITE * 0.5)));
      }}
      onPointerUp={relacher}
      onPointerCancel={relacher}
      onClick={() => {
        if (reduit) setAngle((a) => vueSuivante(a));
      }}
      onKeyDown={(ev) => {
        if (ev.key === "ArrowLeft" || ev.key === "ArrowRight") {
          ev.preventDefault();
          // Le livre a pris la flèche : elle ne tourne pas aussi la page
          // derrière lui (la visionneuse ouvre le livre aux flèches).
          ev.stopPropagation();
          setAngle((a) => apresFleche(a, ev.key === "ArrowLeft" ? "gauche" : "droite"));
        } else if (reduit && (ev.key === "Enter" || ev.key === " ")) {
          ev.preventDefault();
          setAngle((a) => vueSuivante(a));
        }
      }}
    >
      <div
        className="livre3d-objet"
        style={{
          width: w,
          height: h,
          transform: `rotateX(${incl}deg) rotateY(${angle}deg)`,
        }}
      >
        {face(
          "avant",
          { width: w, height: h, transform: `translateZ(${e / 2}px)` },
          avant ?? <MiniCouverture album={album} />,
        )}
        {face(
          "arriere",
          { width: w, height: h, transform: `rotateY(180deg) translateZ(${e / 2}px)`, background: aplat, color: encre },
          cover.back_text ? <p className="livre3d-quatrieme">{afficher(cover.back_text)}</p> : null,
        )}
        {face(
          "dos",
          {
            width: e,
            height: h,
            left: (w - e) / 2,
            transform: `rotateY(-90deg) translateZ(${w / 2}px)`,
            background: aplat,
            color: encre,
          },
          dosMm !== null && dosMm >= SPINE_TEXT_MIN_MM ? (
            <span className="livre3d-titre-dos" style={{ fontSize: Math.min(12, e * 0.55) }}>
              {titre}
            </span>
          ) : null,
        )}
        {face("tranche", {
          width: e,
          height: h,
          left: (w - e) / 2,
          transform: `rotateY(90deg) translateZ(${w / 2}px)`,
        })}
        {face("dessus", {
          width: w,
          height: e,
          top: (h - e) / 2,
          transform: `rotateX(90deg) translateZ(${h / 2}px)`,
        })}
        {face("dessous", {
          width: w,
          height: e,
          top: (h - e) / 2,
          transform: `rotateX(-90deg) translateZ(${h / 2}px)`,
        })}
      </div>
    </div>
  );
}
