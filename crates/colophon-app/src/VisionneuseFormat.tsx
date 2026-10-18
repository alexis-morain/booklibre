// La visionneuse de format : la double page ouverte d'un format, dessinée à
// une échelle commune à tous les formats, une main de taille réelle à côté
// pour repère, et la cote ouverte dessous. Garnie des vignettes qu'on lui
// donne, en aplats quand il n'y en a pas : jamais un vide.
//
// Elle ne sait rien de l'écran qui l'appelle : la création la garnit des
// vignettes du dossier, Recomposer de celles de l'album. Changer de format
// anime la taille de la planche (styles.css, `.visionneuse-planche`), et
// `prefers-reduced-motion` coupe l'animation au lieu de la raccourcir.

import { useEffect, useState } from "react";
import { slotsFor, type Rect } from "./album";
import { chargeGeometrieFormat, type FormatPreset } from "./bridge";
import { geometrieFormatSync } from "./geometrie";
import { t } from "./i18n";
import {
  attribuer,
  BOITE_MM,
  cm,
  echelleCommune,
  formatLabel,
  gabaritApercu,
  MAIN_MM,
  planchePx,
  type VignetteVue,
} from "./visionneuse";

export default function VisionneuseFormat({
  format,
  vignettes,
  titre,
  largeur = 440,
}: {
  format: FormatPreset;
  vignettes: VignetteVue[];
  /** Le titre de l'album, pour le nom que VoiceOver lit. */
  titre: string;
  /** La largeur de la boîte, en pixels ; la hauteur en découle. */
  largeur?: number;
}) {
  const trim = { w: format.w, h: format.h };
  const [, setCharge] = useState(0);
  useEffect(() => {
    if (geometrieFormatSync(trim, 0)) return;
    let vivant = true;
    chargeGeometrieFormat(trim.w, trim.h, 0).then(
      () => vivant && setCharge((n) => n + 1),
      () => {},
    );
    return () => {
      vivant = false;
    };
  }, [trim.w, trim.h]);

  const s = echelleCommune(largeur);
  const planche = planchePx(trim, s);
  const cases = casesEnFraction(format);
  const garnies = attribuer(
    cases.map((c) => ({ ...c, w: c.w * format.w * 2, h: c.h * format.h })),
    vignettes,
  );

  return (
    <figure
      className="visionneuse"
      role="img"
      aria-label={t("visionneuse.nom", {
        titre: titre || t("compo.album.defaut"),
        format: formatLabel(format.name),
        cm: cm(format.w * 2),
      })}
      style={{ width: largeur }}
    >
      <div className="visionneuse-scene" style={{ height: BOITE_MM.h * s }}>
        <svg
          className="visionneuse-main"
          viewBox={`0 0 ${MAIN_MM.w} ${MAIN_MM.h}`}
          style={{ width: MAIN_MM.w * s, height: MAIN_MM.h * s }}
          aria-hidden="true"
        >
          <Main />
        </svg>
        <div className="visionneuse-planche" style={{ width: planche.w, height: planche.h }}>
          {cases.map((c, i) => {
            const v = garnies[i];
            const style = {
              left: `${c.x * 100}%`,
              top: `${c.y * 100}%`,
              width: `${c.w * 100}%`,
              height: `${c.h * 100}%`,
            };
            return v ? (
              <img key={i} className="visionneuse-photo" src={v.url} alt="" style={style} />
            ) : (
              <span key={i} className="visionneuse-aplat" style={style} />
            );
          })}
          <span className="visionneuse-pli" />
        </div>
      </div>
      <figcaption className="visionneuse-cote" style={{ width: planche.w }}>
        {t("setup.cm.ouvert", { cm: cm(format.w * 2) })}
      </figcaption>
    </figure>
  );
}

/**
 * Les cases du gabarit montré, en fractions de la planche : des pourcentages
 * suivent la planche quand sa taille s'anime. Elles viennent du moteur
 * (`slotsFor`, sur le dump du format) dès qu'il est là ; avant, une case par
 * page dans une marge de convention, pour qu'il n'y ait jamais de vide.
 */
function casesEnFraction(f: { w: number; h: number }): Rect[] {
  const dump = geometrieFormatSync({ w: f.w, h: f.h }, 0);
  if (!dump) {
    const m = 0.08;
    return [
      { x: m / 2, y: m, w: 0.5 - m, h: 1 - 2 * m },
      { x: 0.5 + m / 2, y: m, w: 0.5 - m, h: 1 - 2 * m },
    ];
  }
  const g = { ...dump.media, bleed: dump.bleed_mm };
  const [nom, n] = gabaritApercu(f);
  return slotsFor(nom, n, g).map((r) => ({
    x: r.x / g.w,
    y: r.y / g.h,
    w: r.w / g.w,
    h: r.h / g.h,
  }));
}

/** Une main ouverte, paume face à soi, dans une boîte de 100 × 190 mm :
 *  quatre doigts et un pouce en traits ronds sur une paume. Un repère, pas
 *  un dessin. */
function Main() {
  const doigts: [number, number, number, number][] = [
    [20, 104, 20, 64],
    [39, 96, 39, 32],
    [58, 96, 58, 12],
    [77, 100, 77, 28],
    [86, 132, 97, 100],
  ];
  return (
    <g fill="currentColor" stroke="currentColor" strokeLinecap="round">
      {doigts.map(([x1, y1, x2, y2], i) => (
        <line key={i} x1={x1} y1={y1} x2={x2} y2={y2} strokeWidth={15} />
      ))}
      <rect x={12} y={88} width={76} height={74} rx={22} stroke="none" />
      <rect x={26} y={150} width={46} height={40} stroke="none" />
    </g>
  );
}
