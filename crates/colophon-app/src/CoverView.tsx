// The cover editor: back cover, spine, front cover, laid flat like the
// printed sheet. Title and subtitle edit in place, the front photo comes
// from the album and recrops like any case (drag to frame, wheel to zoom),
// the back text is optional.
//
// The sheet is drawn for the supplier chosen on the Envoi screen, from that
// profile's own coefficients: one who builds its own spine gets no spine band
// here either, and a provisional formula says so where it shows. Same
// arithmetic as `cover.rs`, and the parity check compares the two.

import { useEffect, useRef, useState } from "react";
import { afficher } from "./font";
import {
  Album,
  Cover,
  Slot,
  SPINE_TEXT_MIN_MM,
  ZOOM_MAX,
  ZOOM_MIN,
  coverSheet,
  surfacePhotoCouverture,
} from "./album";
import { t } from "./i18n";
import { Printer } from "./bridge";
import { LazyThumb } from "./TriView";
import { alerteDeCouverture, useReleve } from "./photos";
import { SignesAlerte } from "./SignesAlerte";
import { ReglageBloc } from "./ReglageBloc";
import type { Reglage } from "./album";
import { filtreDe, useReglages } from "./reglages";
import { cachedThumb, loadThumb } from "./thumbs";
import { couleurValide, encreSurFond } from "./couleur";
import { Livre3D } from "./Livre3D";
import { PaletteFond } from "./PaletteFond";
import { QuatriemePanneau, QuatriemeReglages } from "./QuatriemeView";
import { avecCadre, avecPhotoQuatrieme } from "./quatrieme";
import type { PoliceOfferte } from "./bridge";

/** Le livre en petit, sous la feuille. */
const HAUTEUR_LIVRE_PX = 84;

export function CoverView({
  album,
  printer,
  onCover,
  onFond,
  onReglage,
  choixALOuverture = false,
  onChoixOuvert,
  polices = [],
  onPolices = () => {},
}: {
  album: Album;
  /** The supplier the sheet is drawn for. Null while the profiles load: the
   *  sheet then falls back to two panels and no spine, which is what a
   *  supplier building its own cover receives anyway. */
  printer: Printer | null;
  onCover: (cover: Cover) => void;
  /** Le fond de la couverture et du dos (A4), un pas d'annulation. */
  onFond: (fond: string | null) => void;
  /** One history step through `edits.ts::setReglage`: the cover photo is
   *  adjustable here even when it sits on no spread — the alternative is a
   *  dead end. */
  onReglage: (src: string, reglage: Reglage) => void;
  /** Ouvrir le choix de la photo dès l'arrivée : Envoi y mène une photo de
   *  couverture trop petite, et le geste qui la répare est ce choix-là. */
  choixALOuverture?: boolean;
  /** Le choix s'est ouvert : l'appelant oublie la demande, pour qu'un retour
   *  à la couverture plus tard ne le rouvre pas. */
  onChoixOuvert?: () => void;
  /** Les faces de la machine, pour la police de la quatrième. */
  polices?: PoliceOfferte[];
  /** Les lire, à la première ouverture de la liste. */
  onPolices?: () => void;
}) {
  const cover: Cover = album.cover ?? { title: album.title };
  // Text fields edit a local form and land on the undo stack at blur:
  // one ⌘Z per field touched, not one per keystroke.
  const [form, setForm] = useState<Cover>(cover);
  // Le choix de photo sert les deux faces : la première, ou la quatrième.
  const [picking, setPicking] = useState<false | "premiere" | "quatrieme">(
    choixALOuverture ? "premiere" : false,
  );
  useEffect(() => {
    if (choixALOuverture) onChoixOuvert?.();
    // Une fois, à l'arrivée.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // The sheet the printer receives, from the profile's own coefficients.
  // `parity.test.ts` compares this arithmetic with the engine's.
  const sheet = coverSheet(album, {
    dos: printer?.dos ?? { mode: "fourni" },
    bleed_mm: printer?.bleed_mm ?? { haut: 0, bas: 0, exterieur: 0 },
    rempli_mm: printer?.rempli_mm,
    debord_mm: printer?.debord_mm,
    mors_mm: printer?.mors_mm,
  });
  const spine = sheet.spine?.[1] ?? 0;
  // The note beside the spine is about the spine, so it reads the spine's own
  // certainty and not the profile's. A supplier can have written their bulk
  // down — which makes the width theirs — while the profile stays provisional
  // on everything else it is still waiting for.
  const provisoire =
    printer?.dos.mode === "calcule" && printer.dos.certitude === "provisoire";

  useEffect(() => {
    setForm(album.cover ?? { title: album.title });
  }, [album]);

  // Escape closes the photo picker before anything else reacts.
  useEffect(() => {
    if (!picking) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      setPicking(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [picking]);

  // Ce que chaque photo imprimerait sur la première, chez cet imprimeur :
  // le choix avertit d'une photo trop petite au lieu de la cacher. Sans
  // profil, rien à mesurer.
  const releve = useReleve();
  const surfacePhoto = printer && surfacePhotoCouverture(album, printer);

  // Every distinct photo the album shows, for the picker.
  const srcs = Array.from(
    new Set(album.spreads.flatMap((s) => s.slots.map((sl) => sl.src))),
  );

  /** Commit right away (photo picks, crops). */
  const set = (patch: Partial<Cover>) => onCover({ ...form, ...patch });
  /** Commit the text fields as they stand. */
  const commitForm = () => {
    const c = album.cover ?? { title: album.title };
    if (
      form.title !== c.title ||
      (form.subtitle ?? "") !== (c.subtitle ?? "") ||
      (form.back_text ?? "") !== (c.back_text ?? "")
    ) {
      onCover(form);
    }
  };

  const sheetAspect = (album.trim_mm.w * 2 + spine) / album.trim_mm.h;
  // Le fond choisi peint la feuille entière, et le texte posé dessus prend
  // l'encre que `cover.rs` lui donnera : ce qu'on voit est ce qui s'imprime.
  const fond = cover.fond && couleurValide(cover.fond) ? cover.fond : null;
  const peint = fond
    ? ({ "--couv-fond": fond, "--couv-encre": encreSurFond(fond) } as React.CSSProperties)
    : {};
  return (
    <div
      className="cover"
      style={{ "--cover-aspect": sheetAspect } as React.CSSProperties}
    >
      <div
        className={"cover-sheet" + (fond ? " avec-fond" : "")}
        style={{ aspectRatio: `${album.trim_mm.w * 2 + spine} / ${album.trim_mm.h}`, ...peint }}
      >
        {/* Back cover: the quatrième, at its print size (1.2-d). */}
        <QuatriemePanneau
          cover={cover}
          form={form}
          trim={album.trim_mm}
          safe={printer?.safe_mm ?? 0}
          onForm={(back_text) => setForm((f) => ({ ...f, back_text }))}
          onCommit={commitForm}
          onCadre={(photo) => onCover(avecCadre({ ...form, back_photo: cover.back_photo }, { photo }))}
        />

        {/* Spine: the profile's width, title along it above the floor where
            type stops being readable on a fold. */}
        {sheet.spine && (
          <div
            className="cover-spine"
            style={{ flex: spine }}
            title={
              t("couverture.dos.titre", { mm: spine.toFixed(1).replace(".", ",") }) +
              (provisoire ? t("couverture.dos.provisoire.titre") : "")
            }
          >
            {spine >= SPINE_TEXT_MIN_MM && (
              <span className="cover-spine-title">{afficher(form.title || album.title)}</span>
            )}
          </div>
        )}

        {/* Front cover: photo + title block. */}
        <div className="cover-front" style={{ flex: album.trim_mm.w }}>
          {cover.photo ? (
            <CoverPhoto
              photo={cover.photo}
              onChange={(photo) => set({ photo })}
            />
          ) : (
            <button className="cover-photo-empty" onClick={() => setPicking("premiere")}>
              {t("couverture.choisir.photo")}
            </button>
          )}
          <div className="cover-titles" onClick={(e) => e.stopPropagation()}>
            <input
              className="cover-title-input"
              value={form.title}
              placeholder={album.title}
              onChange={(e) => setForm({ ...form, title: e.target.value })}
              onBlur={commitForm}
              onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
              aria-label={t("couverture.titre.aria")}
            />
            <input
              className="cover-subtitle-input"
              value={form.subtitle ?? ""}
              placeholder={t("couverture.soustitre.placeholder")}
              onChange={(e) => setForm({ ...form, subtitle: e.target.value })}
              onBlur={commitForm}
              onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
              aria-label={t("couverture.soustitre.aria")}
            />
          </div>
          {cover.photo && (
            <button
              className="cover-photo-change"
              onClick={() => setPicking("premiere")}
              title={t("couverture.changer.titre")}
            >
              {t("couverture.changer")}
            </button>
          )}
        </div>
      </div>

      {/* The same three adjustments as the spread bar, on the same store:
          the photo is keyed by source, so a cover photo also on a spread is
          adjusted once and both surfaces follow. */}
      {cover.photo && (
        <div className="cover-reglages">
          <ReglageBloc src={cover.photo.src} onCommit={onReglage} />
        </div>
      )}

      <QuatriemeReglages
        cover={cover}
        trimW={album.trim_mm.w}
        polices={polices}
        onPolices={onPolices}
        onCover={onCover}
        onChoisirPhoto={() => setPicking("quatrieme")}
      />

      {/* Le fond, et le livre en petit qui le montre en volume. */}
      <div className="cover-fond">
        <Livre3D
          album={album}
          formatMm={[album.trim_mm.w, album.trim_mm.h]}
          dosMm={sheet.spine ? spine : null}
          pages={album.spreads.length * 2}
          hauteurPx={HAUTEUR_LIVRE_PX}
        />
        <PaletteFond album={album} onFond={onFond} />
      </div>

      <p className="cover-note">
        {sheet.spine ? (
          <>
            {t("couverture.note.dos", {
              mm: spine.toFixed(1).replace(".", ","),
              pages: album.spreads.length * 2,
            })}
            {provisoire && t("couverture.note.provisoire")}
            {spine < SPINE_TEXT_MIN_MM && t("couverture.note.mince")}
          </>
        ) : (
          t("couverture.note.sans.dos", {
            imprimeur: printer?.nom ?? t("couverture.imprimeur"),
          })
        )}
        {t("couverture.note.feuille", {
          w: sheet.w.toFixed(0),
          h: sheet.h.toFixed(0),
        })}
      </p>

      {picking && (
        <div className="cover-picker" role="listbox">
          <header className="cover-picker-bar">
            <span>
              {t(picking === "quatrieme" ? "couverture.picker.quatrieme" : "couverture.picker.titre")}
            </span>
            <button className="link" onClick={() => setPicking(false)}>
              {t("commun.fermer")}
            </button>
          </header>
          <div className="cover-picker-grid">
            {srcs.map((src) => {
              const choisie =
                (picking === "quatrieme" ? cover.back_photo?.photo.src : cover.photo?.src) === src;
              const alerte =
                picking === "premiere" && surfacePhoto
                  ? alerteDeCouverture(src, surfacePhoto, printer.min_ppi, releve)
                  : null;
              return (
                <button
                  key={src}
                  role="option"
                  aria-selected={choisie}
                  className={"cover-picker-cell" + (choisie ? " active" : "")}
                  onClick={() => {
                    if (picking === "quatrieme") {
                      onCover(avecPhotoQuatrieme({ ...form, back_photo: cover.back_photo }, src));
                    } else {
                      set({ photo: { src, focal: [0.5, 0.42] } });
                    }
                    setPicking(false);
                  }}
                >
                  <LazyThumb src={src} />
                  {alerte && <SignesAlerte alertes={[alerte]} lieu="couverture" />}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

/** The front-cover photo: full-bleed, recroppable with the same gestures
 *  as a case (drag to frame, ⌥ refines, wheel to zoom). The back cover's
 *  framed photo is the same component in a smaller box. */
export function CoverPhoto({
  photo,
  onChange,
  titre,
}: {
  photo: Slot;
  onChange: (photo: Slot) => void;
  titre?: string;
}) {
  const [url, setUrl] = useState<string | undefined>(() => cachedThumb(photo.src));
  const [draft, setDraft] = useState<Slot | null>(null);
  useReglages();
  const box = useRef<HTMLDivElement>(null);
  const img = useRef<HTMLImageElement>(null);
  const gesture = useRef<{ id: number; x: number; y: number; focal: [number, number] } | null>(null);
  const wheelDraft = useRef<Slot | null>(null);
  const wheelTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    let alive = true;
    if (!cachedThumb(photo.src)) setUrl(undefined);
    loadThumb(photo.src).then(
      (u) => alive && setUrl(u),
      () => {},
    );
    return () => {
      alive = false;
    };
  }, [photo.src]);

  useEffect(() => setDraft(null), [photo]);

  const shown = draft ?? photo;
  const zoom = shown.zoom ?? 1;

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const cur = wheelDraft.current ?? shown;
      const z = Math.min(
        ZOOM_MAX,
        Math.max(ZOOM_MIN, (cur.zoom ?? 1) * Math.exp(-e.deltaY * 0.0022)),
      );
      const next = { ...cur, zoom: z };
      wheelDraft.current = next;
      setDraft(next);
      window.clearTimeout(wheelTimer.current);
      wheelTimer.current = window.setTimeout(() => {
        const w = wheelDraft.current;
        wheelDraft.current = null;
        if (w) onChange(w);
      }, 350);
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  return (
    <div
      ref={box}
      className="cover-photo"
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        gesture.current = {
          id: e.pointerId,
          x: e.clientX,
          y: e.clientY,
          focal: [...shown.focal] as [number, number],
        };
        (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        const g = gesture.current;
        const el = img.current;
        const boxEl = box.current;
        if (!g || g.id !== e.pointerId || !el?.naturalWidth || !boxEl) return;
        const w = boxEl.clientWidth;
        const h = boxEl.clientHeight;
        const s = Math.max(w / el.naturalWidth, h / el.naturalHeight) * zoom;
        const spanX = el.naturalWidth * s - w;
        const spanY = el.naturalHeight * s - h;
        const fine = e.altKey ? 0.2 : 1;
        const fx = spanX > 0.5 ? g.focal[0] - ((e.clientX - g.x) * fine) / spanX : g.focal[0];
        const fy = spanY > 0.5 ? g.focal[1] - ((e.clientY - g.y) * fine) / spanY : g.focal[1];
        setDraft({
          ...shown,
          focal: [Math.min(1, Math.max(0, fx)), Math.min(1, Math.max(0, fy))],
        });
      }}
      onPointerUp={(e) => {
        const g = gesture.current;
        if (!g || g.id !== e.pointerId) return;
        gesture.current = null;
        if (draft) onChange(draft);
      }}
      title={titre ?? t("planche.couverture.recadrer")}
    >
      {url && (
        <img
          ref={img}
          src={url}
          alt=""
          draggable={false}
          style={{
            objectPosition: `${shown.focal[0] * 100}% ${shown.focal[1] * 100}%`,
            transform: zoom > 1.001 ? `scale(${zoom})` : undefined,
            transformOrigin: `${shown.focal[0] * 100}% ${shown.focal[1] * 100}%`,
            filter: filtreDe(photo.src),
          }}
        />
      )}
    </div>
  );
}
