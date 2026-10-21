// The faithful preview: the spread as the PDF holds it, drawn by pdf.js.
//
// The editor draws in the DOM and the press reads a PDF. Those two can never
// be identical by construction, and the gap between them is the first
// objection anybody technical raises about an album maker. So the preview
// stops being a second renderer and becomes a reader of the one that counts:
// same file, same page, same glyphs, same crops.
//
// It is a mode, not a replacement. Editing needs a DOM, so ⌘1 keeps drawing
// the spread; « Feuilleter », in the context line and in Envoi, shows what
// will print. Leaving the mode costs nothing, entering it costs one PDF render
// when the album has moved since the last one.
//
// Since 1.2-c it is the one viewer (« une seule visionneuse ») : the closed
// book in volume on the cover, which opens; pages that curl as they turn; and
// a loupe on one page at a time.
//
// The bitmaps themselves live in `raster.ts`: two views need them, and the
// sheet that turns needs the same page in two places at once.

import { useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { Album, coverSheet } from "./album";
import type { Printer } from "./bridge";
import { COUVERTURE, Cote } from "./feuille";
import { Couverture, Feuilletage, Toile, Tourneur } from "./Feuilletage";
import { t } from "./i18n";
import { decoupeAvant, largeurDeLoupe, partDe, pasDeLoupe } from "./lecture";
import { Quoi, Raster, ouvrir, rasterPret, rasteriser } from "./raster";

/**
 * One page of one of the album's PDFs, drawn to a canvas at the device's own
 * resolution. `page` is 1-based, the way the file numbers its pages.
 */
export function PdfPage({
  quoi,
  page,
  cle,
  largeur,
  onErreur,
}: {
  quoi: Quoi;
  page: number;
  /** Bumped whenever the file on disk is re-rendered. */
  cle: number;
  /** Width to draw at, in CSS pixels. The height follows the page. */
  largeur: number;
  onErreur?: (message: string) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  // Held in a ref rather than read from the deps: an inline callback changes
  // identity at every render of the window, and the status line alone
  // re-renders it every few seconds. In the deps it would cancel the render
  // in flight each time, and the page would never finish drawing.
  const erreur = useRef(onErreur);
  erreur.current = onErreur;

  useEffect(() => {
    let vivant = true;
    let job: { promise: Promise<void>; cancel(): void } | null = null;
    (async () => {
      const doc = await ouvrir(quoi, cle);
      if (!vivant) return;
      const n = Math.min(Math.max(page, 1), doc.numPages);
      const p = await doc.getPage(n);
      if (!vivant || !canvas.current) return;
      // Draw at the screen's real pixels: a spread at 800 CSS pixels on a
      // retina display is 1600 of them, and half of that is a blurred page
      // that would be blamed on the PDF.
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const unite = p.getViewport({ scale: 1 });
      const viewport = p.getViewport({ scale: (largeur * dpr) / unite.width });
      const ctx = canvas.current.getContext("2d");
      if (!ctx) return;
      canvas.current.width = Math.round(viewport.width);
      canvas.current.height = Math.round(viewport.height);
      canvas.current.style.width = `${largeur}px`;
      canvas.current.style.height = `${Math.round(viewport.height / dpr)}px`;
      job = p.render({ canvasContext: ctx, viewport });
      try {
        await job.promise;
      } catch (e) {
        // A cancelled render is the normal way to leave a page, not a fault.
        if (vivant && !String(e).includes("ancel")) erreur.current?.(String(e));
      }
    })().catch((e) => vivant && erreur.current?.(String(e)));
    return () => {
      vivant = false;
      // pdf.js keeps painting a page nobody is looking at otherwise, and a
      // hundred-spread album flipped through fast would queue a hundred.
      job?.cancel();
    };
  }, [quoi, page, cle, largeur]);

  return (
    <canvas
      ref={canvas}
      className="pdf-page"
      aria-label={t("fidele.pdf.aria")}
    />
  );
}

/** « Planche n, page de gauche », pour la ligne de statut et le nom de la
 *  page agrandie. */
function phraseDeLoupe(cote: Cote, planche: number): string {
  return t(cote === "gauche" ? "visionneuse.loupe.gauche" : "visionneuse.loupe.droite", {
    n: planche + 1,
  });
}

/** Ce que montre la loupe : une page de l'intérieur, ou la feuille de
 *  couverture à plat. */
type Loupe = { cote: Cote } | "couverture" | null;

/** La hauteur de la barre de la visionneuse, retirée de la scène avant d'y
 *  faire tenir le livre. Écrite aussi dans `styles.css`. */
const BARRE_PX = 44;

/**
 * The viewer inside the book view's stage: the book fitted to the room
 * available, on the same paper-coloured ground as the DOM spread so switching
 * between the two compares like with like.
 *
 * It measures the room and hands the width on. The interior and the closed
 * book go to `Feuilletage`, where sheets turn and the cover opens; the loupe
 * shows one page alone, drawn at its own size; the flat cover sheet is the
 * loupe of the cover.
 */
export function ApercuFidele({
  index,
  total,
  cle,
  album,
  printer,
  couvertureRendue,
  onPlanche,
  onStatut,
  onErreur,
  ref,
}: {
  /** L'indice d'App : base zéro, `COUVERTURE` pour le livre fermé. */
  index: number;
  /** How many spreads the album has. */
  total: number;
  cle: number;
  album: Album;
  /** Le profil dont la couverture a été rendue ; null sans profil. */
  printer: Printer | null;
  /** Le rendu de la couverture a abouti : le livre fermé peut porter son
   *  avant, et le plat tourner. */
  couvertureRendue: boolean;
  /** Commits a turn the sheet finished. Returns false when there is none. */
  onPlanche: (sens: number) => boolean;
  onStatut: (message: string) => void;
  onErreur: (message: string) => void;
  ref?: React.Ref<Tourneur | null>;
}) {
  const box = useRef<HTMLDivElement>(null);
  const [boite, setBoite] = useState({ w: 0, h: 0 });
  const [loupe, setLoupe] = useState<Loupe>(null);
  const feuilletage = useRef<Tourneur | null>(null);
  const trim = album.trim_mm;
  const bleed = album.bleed_mm;

  // The stage resizes with the window; the canvas has to be redrawn at the
  // new size rather than stretched, or the viewer would be the blurriest
  // thing on screen.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    // Measured on the stage, never on this element: the canvas lives inside
    // it, so measuring itself would feed its own size back and the page
    // would grow at every observation until it ran off the window. The stage
    // carries `container-type: size`, so its box does not depend on what it
    // holds.
    const scene = el.closest(".stage") as HTMLElement | null;
    const cible = scene ?? el;
    const mesure = () => {
      const style = getComputedStyle(cible);
      setBoite({
        w: Math.max(
          0,
          cible.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight),
        ),
        h: Math.max(
          0,
          cible.clientHeight -
            parseFloat(style.paddingTop) -
            parseFloat(style.paddingBottom) -
            BARRE_PX,
        ),
      });
    };
    mesure();
    const ro = new ResizeObserver(mesure);
    ro.observe(cible);
    return () => ro.disconnect();
  }, []);

  // Fit the spread, bleed included: that is the shape the PDF's page has.
  const ratio = (trim.w * 2 + bleed * 2) / (trim.h + bleed * 2);
  const largeur = Math.max(0, Math.floor(Math.min(boite.w, boite.h * ratio)));

  // L'avant du livre fermé, découpé dans la feuille de couverture du profil
  // dont elle a été rendue. Sans l'un ou l'autre, le livre fermé montre sa
  // vignette et s'ouvre sèchement.
  const feuille = useMemo(
    () => (printer ? coverSheet(album, printer) : null),
    // La feuille ne dépend que du format, du nombre de planches et du profil.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [printer, trim.w, trim.h, album.spreads.length],
  );
  const couverture = useMemo((): Couverture | null => {
    if (!feuille || !printer || !couvertureRendue || largeur <= 0) return null;
    const avecFond = decoupeAvant(feuille, printer, bleed);
    return {
      largeur: Math.round(largeur / 2 / avecFond.w),
      avant: decoupeAvant(feuille, printer, 0),
      avecFond,
    };
  }, [feuille, printer, couvertureRendue, largeur, bleed]);

  // La loupe lit la planche à l'écran : quand celle-ci devient le livre
  // fermé, ou la couverture l'intérieur, la loupe n'a plus de sujet.
  useEffect(() => {
    if (loupe === "couverture" && index !== COUVERTURE) setLoupe(null);
    else if (loupe && loupe !== "couverture" && index === COUVERTURE) setLoupe(null);
  }, [index, loupe]);

  const ouvrirLoupe = (l: Loupe) => {
    setLoupe(l);
    if (l === "couverture") onStatut(t("visionneuse.loupe.couverture"));
    else if (l) onStatut(phraseDeLoupe(l.cote, index));
  };

  useImperativeHandle(
    ref,
    (): Tourneur => ({
      tourner: (sens) => {
        if (sens !== 1 && sens !== -1) return false;
        if (loupe === "couverture") return true;
        if (loupe) {
          const suivante = pasDeLoupe({ planche: index, cote: loupe.cote }, sens, total);
          if (!suivante) return true;
          if (suivante.planche !== index) onPlanche(sens);
          setLoupe({ cote: suivante.cote });
          onStatut(phraseDeLoupe(suivante.cote, suivante.planche));
          return true;
        }
        return feuilletage.current?.tourner(sens) ?? false;
      },
      fermerLoupe: () => {
        if (!loupe) return false;
        setLoupe(null);
        return true;
      },
    }),
    [loupe, index, total, onPlanche, onStatut],
  );

  /** Ouvrir le livre : la feuille quand elle peut tourner, sèchement sinon. */
  const tournerOuSauter = (sens: 1 | -1) => {
    if (!feuilletage.current?.tourner(sens)) onPlanche(sens);
  };

  const pages = album.spreads.length * 2;
  const surCouverture = index === COUVERTURE;

  return (
    <div className="pdf-stage" ref={box}>
      <div className="visionneuse-livre">
        {largeur > 0 &&
          (loupe === "couverture" ? (
            <div className="loupe-fermer" onClick={() => setLoupe(null)} title={t("visionneuse.retour")}>
              <PdfPage
                quoi="couverture"
                page={1}
                cle={cle}
                largeur={Math.floor(Math.min(boite.w, boite.h * ((feuille?.w ?? trim.w * 2.1) / (feuille?.h ?? trim.h))))}
                onErreur={onErreur}
              />
            </div>
          ) : loupe ? (
            <PageALaLoupe
              planche={index}
              cote={loupe.cote}
              cle={cle}
              largeur={largeurDeLoupe(boite, trim, bleed)}
              onFermer={() => setLoupe(null)}
              onErreur={onErreur}
            />
          ) : (
            <Feuilletage
              planche={index}
              total={total}
              cle={cle}
              largeur={largeur}
              livre={{
                album,
                dosMm: feuille?.spine?.[1] ?? null,
                pages,
                bleedMm: bleed,
              }}
              couverture={couverture}
              onPlanche={onPlanche}
              onLoupe={(cote) => ouvrirLoupe({ cote })}
              onErreur={onErreur}
              ref={feuilletage}
            />
          ))}
      </div>
      {/* La barre : les chemins au clavier de ce que la souris fait sur le
          livre — l'ouvrir, et regarder une page de près. */}
      <div
        className="visionneuse-barre"
        role="toolbar"
        aria-label={t("visionneuse.barre")}
        // Le clavier d'App se tait devant un bouton qui a le focus : après un
        // clic ou un Entrée, la barre le rend, et les flèches retournent les
        // pages.
        onClickCapture={() => {
          queueMicrotask(() => (document.activeElement as HTMLElement | null)?.blur());
        }}
      >
        {loupe ? (
          <button type="button" className="link" onClick={() => setLoupe(null)}>
            {t("visionneuse.retour")}
          </button>
        ) : surCouverture ? (
          <>
            <button
              type="button"
              className="link"
              disabled={total === 0}
              onClick={() => tournerOuSauter(1)}
            >
              {t("visionneuse.ouvrir")}
            </button>
            <button
              type="button"
              className="link"
              disabled={!couvertureRendue}
              onClick={() => ouvrirLoupe("couverture")}
            >
              {t("visionneuse.couverture")}
            </button>
          </>
        ) : (
          <>
            <button type="button" className="link" onClick={() => ouvrirLoupe({ cote: "gauche" })}>
              {t("visionneuse.page.gauche")}
            </button>
            <button type="button" className="link" onClick={() => ouvrirLoupe({ cote: "droite" })}>
              {t("visionneuse.page.droite")}
            </button>
          </>
        )}
      </div>
    </div>
  );
}

/**
 * Une page seule, à la hauteur de la scène. La planche entière est tirée à la
 * taille qu'il faut pour que sa moitié remplisse la boîte, et la moitié est
 * peinte telle quelle : jamais un agrandissement CSS d'un bitmap plus petit.
 * Un clic la referme.
 */
function PageALaLoupe({
  planche,
  cote,
  cle,
  largeur,
  onFermer,
  onErreur,
}: {
  planche: number;
  cote: Cote;
  cle: number;
  /** La largeur de la planche entière tirée pour la loupe. */
  largeur: number;
  onFermer: () => void;
  onErreur: (message: string) => void;
}) {
  const pret = rasterPret("album", planche + 1, cle, largeur);
  const [raster, setRaster] = useState<Raster | null>(pret);
  const erreur = useRef(onErreur);
  erreur.current = onErreur;

  useEffect(() => {
    if (largeur <= 0) return;
    if (pret) {
      setRaster(pret);
      return;
    }
    let vivant = true;
    rasteriser("album", planche + 1, cle, largeur).then(
      (r) => vivant && setRaster(r),
      (e) => vivant && erreur.current(String(e)),
    );
    return () => {
      vivant = false;
    };
  }, [planche, cle, largeur, pret]);

  // La planche d'avant reste à l'écran le temps du tirage, à sa propre
  // taille : un trou blanc ferait croire à une page vide.
  const affiche = raster;
  return (
    // Pas un bouton : le clavier d'App se tait devant un bouton qui a le
    // focus, et les flèches doivent continuer d'aller de page en page. La
    // barre porte « Revenir au livre » pour le clavier, Échap aussi.
    <div
      className="loupe-page"
      onClick={onFermer}
      role="img"
      aria-label={phraseDeLoupe(cote, planche)}
      title={t("visionneuse.retour")}
    >
      {affiche && (
        <Toile
          peinture={{
            source: affiche.source,
            part: partDe(affiche.source, { x: cote === "droite" ? 0.5 : 0, y: 0, w: 0.5, h: 1 }),
          }}
          w={affiche.largeur / 2}
          h={affiche.hauteur}
          classe="loupe-toile"
        />
      )}
    </div>
  );
}
