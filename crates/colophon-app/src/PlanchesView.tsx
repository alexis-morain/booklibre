// The light table: every spread as a miniature, on one scrollable grid.
// Reordering is a drag, duplicating a keystroke, a photo-less or text
// spread one click away: the sequencing work that makes or breaks the book
// happens here, nearly for free. Images only load when their cell scrolls
// into view; the blob pool stays bounded.
//
// **Le glisser passe par le pointeur, jamais par le glisser HTML5.** Celui-ci
// dépend de ce que le webview veut bien livrer, et ne se vérifie pas au
// harnais. Un appui devient un glisser au-delà de six pixels (en deçà, c'est
// le clic d'avant) ; la figure suit la main par `transform`, jamais par le
// layout, et la cible est la cellule dont le rectangle contient le pointeur
// (`planches.ts::cibleSous`). Rien ne s'écrit avant le relâchement, et Échap
// annule sans rien écrire.
//
// **Et elle se parcourt au clavier**, parce que c'est un écran de
// navigation : c'est ici qu'on cherche une planche, et une grille qu'on ne
// peut atteindre qu'à la souris n'est pas une grille, c'est une image. Un
// seul arrêt de tabulation — la cellule courante — puis les flèches à
// l'intérieur, les verticales d'une rangée entière. La couverture est la
// page zéro et se parcourt avec le reste ; ce qu'elle porte dedans attend
// la session qui lui donnera une scène.

import { couleurValide, encreSurFond } from "./couleur";
import { useEffect, useRef, useState } from "react";
import { t } from "./i18n";
import { Album, Spread, spreadGeometry, slotsFor } from "./album";
import { largeurCollee } from "./scene";
import { Alerte, badgesDe, useReleve } from "./photos";
import { legendeQualite, motDeLigne } from "./qualite";
import { reglagePose, useReglages } from "./reglages";
import { thumbCropStyle } from "./SpreadView";
import { cachedThumb, loadThumb } from "./thumbs";
import { cibleSous, RectCellule, seuilFranchi } from "./planches";

/** Un appui sur une planche, qui deviendra peut-être un glisser. */
type Appui = {
  from: number;
  pointeur: number;
  x0: number;
  y0: number;
  /** Les cellules mesurées au début du glisser : seule la figure tenue
   *  bouge, par `transform`, donc la grille ne change pas sous la main. */
  rects: RectCellule[] | null;
  cible: number | null;
};

export function PlanchesView({
  album,
  current,
  onSelect,
  onOpen,
  onMove,
  onLock,
  onRemove,
  onMenu,
  reliure = null,
}: {
  album: Album;
  /** Le profil d'impression courant : la vignette de la première planche
   *  grise sa page collée dans la couverture, comme l'éditeur (1.1-d). */
  reliure?: { pages_simples: boolean } | null;
  /** Index of the highlighted cell; -1 is the cover, page zero. */
  current: number;
  onSelect: (at: number) => void;
  /** Double-click: open this spread (or the cover, -1) in the book view. */
  onOpen: (at: number) => void;
  onMove: (from: number, to: number) => void;
  onLock: (at: number) => void;
  /** La corbeille de la vignette : un pas d'annulation, sans confirmation. */
  onRemove: (at: number) => void;
  /** Le menu d'une planche, au point donné en pixels de la fenêtre. */
  onMenu: (at: number, point: { x: number; y: number }) => void;
}) {
  const [dropAt, setDropAt] = useState<number | null>(null);
  const [vue, setVue] = useState<Vue>("toutes");
  // Les cases sombres, que seule une vignette décodée sait : « i:c ».
  const [sombres, setSombres] = useState<ReadonlySet<string>>(() => new Set());
  const surSombre = (i: number, c: number, sombre: boolean) =>
    setSombres((avant) => {
      const cle = `${i}:${c}`;
      if (avant.has(cle) === sombre) return avant;
      const apres = new Set(avant);
      if (sombre) apres.add(cle);
      else apres.delete(cle);
      return apres;
    });
  // Les cases en alerte, par la règle de la planche (`badgesDe`) : ce que le
  // relevé sait tout de suite, et « sombre » quand la vignette d'une case
  // s'est décodée, puisqu'elle lit ses pixels. Un seul calcul, pour les
  // étiquettes des cellules et pour les vues filtrées.
  const releve = useReleve();
  const geom = spreadGeometry(album);
  const parPlanche: Alerte[][][] = album.spreads.map((spread, i) => {
    const rects = slotsFor(spread.template, spread.slots.length, geom);
    return spread.slots.map((slot, c) => {
      const r = rects[c];
      if (!r) return [];
      const b = badgesDe(slot.src, null, r, 1, slot.zoom ?? 1, undefined, releve);
      return sombres.has(`${i}:${c}`) ? [...b.alertes, { code: "sombre" as const }] : b.alertes;
    });
  });
  const aVerifier = parPlanche.map((cases) => cases.some((a) => a.length > 0));
  const comptes: Record<Vue, number> = {
    toutes: album.spreads.length,
    verifier: aVerifier.filter(Boolean).length,
    editees: album.spreads.filter((s) => s.edited).length,
  };
  const garde = (i: number) =>
    vue === "toutes" || (vue === "verifier" ? aVerifier[i] : album.spreads[i].edited === true);
  /** Les cellules à l'écran, dans l'ordre du livre ; -1 est la couverture,
   *  qui n'est que de la vue complète. */
  const visibles = [
    ...(vue === "toutes" ? [-1] : []),
    ...album.spreads.map((_, i) => i).filter(garde),
  ];
  const arret = visibles.includes(current) ? current : (visibles[0] ?? current);
  const [tenue, setTenue] = useState<{ at: number; dx: number; dy: number } | null>(null);
  const appui = useRef<Appui | null>(null);
  // Le clic qui suit le relâchement d'un glisser n'est pas une sélection.
  const avaleClic = useRef(false);
  const grille = useRef<HTMLDivElement>(null);

  const lacher = () => {
    appui.current = null;
    setTenue(null);
    setDropAt(null);
  };

  // Échap pendant un glisser annule sans rien écrire. En capture, et arrêtée
  // là : la table clavier d'App ne doit pas la lire une seconde fois.
  const glisse = tenue !== null;
  useEffect(() => {
    if (!glisse) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      avaleClic.current = true;
      lacher();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [glisse]);

  const surAppui = (e: React.PointerEvent<HTMLElement>, at: number) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
    // Réordonner dans une vue filtrée déplacerait une planche entre des
    // voisines qu'on ne voit pas : le glisser n'a lieu que sur la table
    // entière.
    if (vue !== "toutes") return;
    avaleClic.current = false;
    appui.current = {
      from: at,
      pointeur: e.pointerId,
      x0: e.clientX,
      y0: e.clientY,
      rects: null,
      cible: null,
    };
  };

  const surMouvement = (e: React.PointerEvent<HTMLElement>) => {
    const a = appui.current;
    if (!a || a.pointeur !== e.pointerId) return;
    const dx = e.clientX - a.x0;
    const dy = e.clientY - a.y0;
    if (!a.rects) {
      if (!seuilFranchi(dx, dy)) return;
      e.currentTarget.setPointerCapture(e.pointerId);
      a.rects = [...(grille.current?.querySelectorAll<HTMLElement>("[data-at]") ?? [])]
        .map((el) => {
          const r = el.getBoundingClientRect();
          return {
            at: Number(el.dataset.at),
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
          };
        })
        .filter((r) => r.at !== a.from);
    }
    a.cible = cibleSous(a.rects, e.clientX, e.clientY);
    setTenue({ at: a.from, dx, dy });
    setDropAt(a.cible);
  };

  const surRelache = (e: React.PointerEvent<HTMLElement>) => {
    const a = appui.current;
    if (!a || a.pointeur !== e.pointerId) return;
    if (a.rects) {
      avaleClic.current = true;
      if (a.cible !== null && a.cible !== a.from) onMove(a.from, a.cible);
    }
    lacher();
  };

  /** Combien de cellules par rangée, tel que la grille les pose vraiment :
   *  une flèche verticale doit sauter une rangée, pas un nombre inventé. */
  const colonnes = () => {
    const el = grille.current;
    if (!el) return 1;
    const pistes = getComputedStyle(el)
      .gridTemplateColumns.split(" ")
      .filter(Boolean).length;
    return Math.max(1, pistes);
  };

  // Le clavier suit la cellule courante — mais seulement s'il est déjà dans
  // la grille : sinon la vue volerait le focus à la barre ou au pied.
  useEffect(() => {
    const el = grille.current;
    if (!el || !el.contains(document.activeElement)) return;
    el.querySelector<HTMLElement>(`[data-at="${current}"]`)?.focus();
  }, [current]);

  const surTouche = (e: React.KeyboardEvent, at: number) => {
    const dernier = album.spreads.length - 1;
    const pos = visibles.indexOf(at);
    // Arrêtée ici, la touche n'atteint pas la table de commandes d'App, qui
    // ferait le même pas une seconde fois. On marche sur les cellules
    // visibles : sur la table entière c'est le même pas qu'avant.
    const vers = (p: number) => visibles[Math.min(visibles.length - 1, Math.max(0, p))];
    const aller = (p: number) => {
      e.preventDefault();
      e.stopPropagation();
      if (visibles.length > 0) onSelect(vers(p));
    };
    // ⌥ flèche déplace la planche elle-même : le livre se réordonne au
    // clavier comme à la souris, et le focus suit la planche déplacée.
    if (e.altKey && at >= 0 && vue === "toutes") {
      const vers: Record<string, number> = {
        ArrowRight: at + 1,
        ArrowLeft: at - 1,
        ArrowDown: at + colonnes(),
        ArrowUp: at - colonnes(),
      };
      const to = vers[e.key];
      if (to !== undefined) {
        e.preventDefault();
        e.stopPropagation();
        const borne = Math.min(dernier, Math.max(0, to));
        if (borne !== at) onMove(at, borne);
        return;
      }
    }
    switch (e.key) {
      case "ArrowRight":
        aller(pos + 1);
        break;
      case "ArrowLeft":
        aller(pos - 1);
        break;
      case "ArrowDown":
        aller(pos + colonnes());
        break;
      case "ArrowUp":
        aller(pos - colonnes());
        break;
      case "Home":
        aller(0);
        break;
      case "End":
        aller(visibles.length - 1);
        break;
      // Entrée et Espace ouvrent : la cellule se comporte comme le bouton
      // qu'elle est pour qui ne la voit pas.
      case "Enter":
      case " ":
        e.preventDefault();
        e.stopPropagation();
        onOpen(at);
        break;
      // Maj+F10 et la touche Menu : le menu de la planche, posé au coin de
      // la vignette. La couverture n'en a pas.
      case "F10":
      case "ContextMenu":
        if (at < 0 || (e.key === "F10" && !e.shiftKey)) break;
        e.preventDefault();
        e.stopPropagation();
        {
          const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
          onMenu(at, { x: r.left, y: r.bottom });
        }
        break;
    }
  };

  const onglets: [Vue, string][] = [
    ["toutes", t("planches.vue.toutes")],
    ["verifier", t("planches.vue.verifier")],
    ["editees", t("planches.vue.editees")],
  ];
  return (
    <div className="planches-vue">
    <div className="planches-barre" role="tablist" aria-label={t("planches.vues")}>
      {onglets.map(([v, nom]) => (
        <button
          key={v}
          role="tab"
          aria-selected={vue === v}
          className={"planches-onglet" + (vue === v ? " actif" : "")}
          onClick={() => setVue(v)}
        >
          {nom}
          {v !== "toutes" && <span className="typed"> {comptes[v]}</span>}
        </button>
      ))}
    </div>
    <div className="planches" role="list" aria-label={t("table.liste")} ref={grille}>
      {vue === "toutes" && (
        <CoverCell
          album={album}
          current={current === -1}
          arret={arret === -1}
          onSelect={() => onSelect(-1)}
          onOpen={() => onOpen(-1)}
          onKey={(e) => surTouche(e, -1)}
        />
      )}
      {visibles.length === 0 && <p className="planches-vide">{t("planches.vue.vide")}</p>}
      {album.spreads.map((spread, i) => !garde(i) ? null : (
        <PlancheCell
          reliure={reliure}
          key={i}
          album={album}
          spread={spread}
          index={i}
          parCase={parPlanche[i]}
          onSombre={(c, sombre) => surSombre(i, c, sombre)}
          arret={i === arret}
          current={i === current}
          dropping={dropAt === i}
          onSelect={() => onSelect(i)}
          onOpen={() => onOpen(i)}
          onLock={() => onLock(i)}
          onRemove={() => onRemove(i)}
          onContextMenu={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onSelect(i);
            onMenu(i, { x: e.clientX, y: e.clientY });
          }}
          onKey={(e) => surTouche(e, i)}
          tenue={tenue?.at === i ? tenue : null}
          onAppui={(e) => surAppui(e, i)}
          onMouvement={surMouvement}
          onRelache={surRelache}
          onAnnule={lacher}
          avaleClic={() => {
            const avale = avaleClic.current;
            avaleClic.current = false;
            return avale;
          }}
        />
      ))}
    </div>
    </div>
  );
}

/** Les vues de la table : tout, ce que le contrôle voudra relire, ce qu'une
 *  main a touché. */
type Vue = "toutes" | "verifier" | "editees";

function PlancheCell({
  album,
  spread,
  index,
  parCase,
  onSombre,
  arret,
  current,
  dropping,
  onSelect,
  onOpen,
  onLock,
  onRemove,
  onContextMenu,
  onKey,
  tenue,
  onAppui,
  onMouvement,
  onRelache,
  onAnnule,
  avaleClic,
  reliure,
}: {
  reliure: { pages_simples: boolean } | null;
  album: Album;
  spread: Spread;
  index: number;
  /** Les alertes de chaque case, calculées une fois par la vue. */
  parCase: Alerte[][];
  /** Une vignette décodée dit si sa case est sombre. */
  onSombre: (cell: number, sombre: boolean) => void;
  /** L'unique arrêt de tabulation de la grille. */
  arret: boolean;
  current: boolean;
  dropping: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onLock: () => void;
  onRemove: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onKey: (e: React.KeyboardEvent) => void;
  /** Le décalage de la figure quand c'est elle qu'on glisse. */
  tenue: { dx: number; dy: number } | null;
  onAppui: (e: React.PointerEvent<HTMLElement>) => void;
  onMouvement: (e: React.PointerEvent<HTMLElement>) => void;
  onRelache: (e: React.PointerEvent<HTMLElement>) => void;
  onAnnule: () => void;
  /** Vrai une fois si le clic qui arrive suit un glisser. */
  avaleClic: () => boolean;
}) {
  const enAlerte = parCase.filter((a) => a.length > 0).length;
  const lignes = legendeQualite([parCase]);
  const ditAlerte =
    enAlerte === 0
      ? ""
      : enAlerte === 1
        ? t("table.alerte.une")
        : t("table.alerte", { n: enAlerte });
  return (
    <figure
      role="listitem"
      aria-roledescription={t("table.cellule.role")}
      className={
        "planche-cell" +
        (current ? " current" : "") +
        (dropping ? " dropping" : "") +
        (tenue ? " tenue" : "")
      }
      style={tenue ? { transform: `translate(${tenue.dx}px, ${tenue.dy}px)` } : undefined}
      onPointerDown={onAppui}
      onPointerMove={onMouvement}
      onPointerUp={onRelache}
      onPointerCancel={onAnnule}
      // Une vignette se glisserait d'elle-même, en HTML5, et le navigateur
      // annulerait alors le pointeur au milieu du geste.
      onDragStart={(e) => e.preventDefault()}
      onClick={(e) => {
        e.stopPropagation();
        if (avaleClic()) return;
        onSelect();
      }}
      onDoubleClick={(e) => {
        e.stopPropagation();
        onOpen();
      }}
      onContextMenu={onContextMenu}
      title={
        (spread.caption ? `${spread.caption} · ` : "") +
        t("table.cellule.titre", { n: index + 1 })
      }
      // Un seul arrêt de tabulation pour toute la grille, celui de la
      // cellule courante : les flèches font le reste, et Tab ne devient pas
      // soixante pressions pour traverser un album.
      data-at={index}
      tabIndex={arret ? 0 : -1}
      aria-current={current ? true : undefined}
      aria-label={
        (spread.caption ? `${spread.caption} · ` : "") +
        t("table.cellule.nom", { n: index + 1 }) +
        (ditAlerte ? ` · ${ditAlerte}` : "") +
        (largeurCollee(index, reliure, spreadGeometry(album)) > 0
          ? ` · ${t("planche.collee")}`
          : "")
      }
      onKeyDown={onKey}
    >
      <MiniSpread
        album={album}
        spread={spread}
        onSombre={onSombre}
        colleeMm={largeurCollee(index, reliure, spreadGeometry(album))}
      />
      <figcaption className="planche-meta">
        <span className="planche-num">{index + 1}</span>
        {spread.caption && <span className="tag chap planche-chapitre">{spread.caption}</span>}
        <span className="planche-flags">
          {lignes.map((l) => (
            <span key={l.code} className="tag qual">
              {motDeLigne(l)}
            </span>
          ))}
          {spread.edited && (
            <span className="tag main" title={t("table.editee")}>
              {t("table.tag.editee")}
            </span>
          )}
          <button
            className={"lock" + (spread.locked ? " locked" : "")}
            onClick={(e) => {
              e.stopPropagation();
              onLock();
            }}
            title={spread.locked ? t("table.figee") : t("table.figer")}
            aria-pressed={spread.locked ?? false}
          >
            <LockGlyph open={!spread.locked} />
          </button>
          {/* Visible au survol et au focus, toujours lue par VoiceOver. */}
          <button
            className="corbeille"
            onClick={(e) => {
              e.stopPropagation();
              onRemove();
            }}
            title={t("planche.supprimer", { n: index + 1 })}
            aria-label={t("planche.supprimer", { n: index + 1 })}
          >
            <TrashGlyph />
          </button>
        </span>
      </figcaption>
    </figure>
  );
}

/**
 * The cover as the light table's page zero: the whole book starts here, so
 * the whole book shows here. Front panel only, at the page's own aspect;
 * it neither drags nor receives drops, a cover has one possible place.
 */
function CoverCell({
  album,
  current,
  arret,
  onSelect,
  onOpen,
  onKey,
}: {
  album: Album;
  current: boolean;
  arret: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onKey: (e: React.KeyboardEvent) => void;
}) {
  return (
    <figure
      role="listitem"
      className={"planche-cell planche-cover" + (current ? " current" : "")}
      onClick={(e) => {
        e.stopPropagation();
        onSelect();
      }}
      onDoubleClick={(e) => {
        e.stopPropagation();
        onOpen();
      }}
      title={t("table.couverture.titre")}
      data-at={-1}
      tabIndex={arret ? 0 : -1}
      aria-current={current ? true : undefined}
      aria-label={t("table.couverture")}
      onKeyDown={onKey}
    >
      <MiniCouverture album={album} />
      <figcaption className="planche-meta">
        <span className="planche-num">C</span>
        <span className="planche-chapter">{t("table.couverture")}</span>
      </figcaption>
    </figure>
  );
}

/** Le plat de la couverture en vignette, à l'aspect de la page : la case
 *  zéro de la table lumineuse, et le livre d'Envoi. */
export function MiniCouverture({ album }: { album: Album }) {
  const cover = album.cover ?? { title: album.title };
  // Le fond de la couverture (A4) se voit là où la photo ne couvre pas, et
  // le titre posé dessus prend l'encre que `cover.rs` lui donnera.
  const fond = cover.fond && couleurValide(cover.fond) ? cover.fond : undefined;
  return (
    <div
      className="mini-cover"
      style={{ aspectRatio: `${album.trim_mm.w} / ${album.trim_mm.h}`, background: fond }}
    >
      {cover.photo && <MiniImg slot={cover.photo} />}
      <span
        className="mini-cover-title"
        style={fond && !cover.photo ? { color: encreSurFond(fond) } : undefined}
      >
        {cover.title || album.title}
      </span>
    </div>
  );
}

/** A minimal padlock, drawn rather than emoji'd. */
export function LockGlyph({ open }: { open: boolean }) {
  return (
    <svg viewBox="0 0 10 12" width="10" height="12" aria-hidden="true">
      <rect x="1" y="5.5" width="8" height="6" fill="currentColor" />
      <path
        d={open ? "M 2.8 5.5 V 3.2 A 2.2 2.2 0 0 1 7.2 3.2 V 4.2" : "M 2.8 5.5 V 3.2 A 2.2 2.2 0 0 1 7.2 3.2 V 5.5"}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.3"
        transform={open ? "translate(1.6 -1.4) rotate(24 5 3.5)" : undefined}
      />
    </svg>
  );
}

/** Une corbeille au trait, à la taille du cadenas. */
function TrashGlyph() {
  return (
    <svg viewBox="0 0 10 12" width="10" height="12" aria-hidden="true">
      <path d="M 0.5 2.5 H 9.5 M 3.5 2.5 V 1 H 6.5 V 2.5" fill="none" stroke="currentColor" strokeWidth="1.1" />
      <path d="M 1.6 2.5 L 2.3 11.3 H 7.7 L 8.4 2.5" fill="none" stroke="currentColor" strokeWidth="1.1" />
      <path d="M 4 4.5 V 9.5 M 6 4.5 V 9.5" stroke="currentColor" strokeWidth="0.9" />
    </svg>
  );
}

/**
 * One spread at postage size: real geometry, real crops, images gated by
 * an IntersectionObserver so sixty planches cost only what is on screen.
 */
function MiniSpread({
  album,
  spread,
  onSombre,
  colleeMm = 0,
}: {
  album: Album;
  spread: Spread;
  /** La page collée dans la couverture, en millimètres depuis la gauche :
   *  zéro partout sauf sur la première planche chez qui relie page par page. */
  colleeMm?: number;
  /** Une vignette décodée dit si sa case est sombre. */
  onSombre: (cell: number, sombre: boolean) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  const geom = spreadGeometry(album);
  const rects = slotsFor(spread.template, spread.slots.length, geom);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          setVisible(true);
          io.disconnect();
        }
      },
      { rootMargin: "300px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      className="mini-spread"
      style={{ aspectRatio: `${geom.w} / ${geom.h}` }}
    >
      {spread.slots.map((slot, i) => {
        const r = rects[i];
        if (!r) return null;
        return (
          <span
            key={i}
            className="mini-slot"
            style={{
              left: `${(r.x / geom.w) * 100}%`,
              top: `${(r.y / geom.h) * 100}%`,
              width: `${(r.w / geom.w) * 100}%`,
              height: `${(r.h / geom.h) * 100}%`,
            }}
          >
            {visible && (
              <MiniImg slot={slot} rect={r} onSombre={(v) => onSombre(i, v)} />
            )}
          </span>
        );
      })}
      {/* Les planches de texte se lisent dans la grille : sans ça, la garde
          et le colophon y sont deux rectangles vides. */}
      {spread.text !== undefined && (
        <span className="mini-text" aria-hidden="true">
          {spread.text.trim() ? spread.text : "texte"}
        </span>
      )}
      {colleeMm > 0 && (
        <span
          className="mini-collee"
          style={{ width: `${(colleeMm / geom.w) * 100}%` }}
          title={t("planche.collee")}
          aria-hidden="true"
        />
      )}
      <span className="mini-fold" aria-hidden="true" />
    </div>
  );
}

function MiniImg({
  slot,
  rect,
  onSombre,
}: {
  slot: { src: string; focal: [number, number]; zoom?: number };
  /** La case, pour une planche ; la couverture n'en passe pas, son
   *  alerte n'est pas de ce chantier. */
  rect?: { x: number; y: number; w: number; h: number };
  onSombre?: (sombre: boolean) => void;
}) {
  const [url, setUrl] = useState<string | undefined>(() => cachedThumb(slot.src));
  // thumbCropStyle carries the photo's adjustment: follow it.
  useReglages();
  const img = useRef<HTMLImageElement>(null);
  const releve = useReleve();
  // « Sombre » lit les pixels au travers du réglage posé, comme sur la
  // planche ; la clé relance la lecture quand le réglage change.
  const r = reglagePose(slot.src);
  const cle = `${r?.expo ?? 0}|${r?.contraste ?? 0}|${r?.nb ?? false}`;
  useEffect(() => {
    const el = img.current;
    if (!el || !url || !rect || !onSombre) return;
    const lire = () => {
      if (!el.naturalWidth) return;
      const b = badgesDe(slot.src, el, rect, 1, slot.zoom ?? 1, reglagePose(slot.src), releve);
      onSombre(b.alertes.some((a) => a.code === "sombre"));
    };
    if (el.complete) {
      lire();
      return;
    }
    el.addEventListener("load", lire, { once: true });
    return () => el.removeEventListener("load", lire);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [url, releve, cle, slot.src]);
  useEffect(() => {
    let alive = true;
    if (!cachedThumb(slot.src)) setUrl(undefined);
    loadThumb(slot.src).then(
      (u) => alive && setUrl(u),
      () => {},
    );
    return () => {
      alive = false;
    };
  }, [slot.src]);
  return url ? <img ref={img} src={url} alt="" style={thumbCropStyle(slot)} /> : null;
}

