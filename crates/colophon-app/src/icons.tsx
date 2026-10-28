// One icon system: small SVGs drawn in the padlock's line (PlanchesView's
// LockGlyph set the tone), replacing the unicode glyphs ▾ ▴ ‹ › and the
// lettered cover tick. Stroke follows currentColor, so every icon inherits
// the state colours of its button.

export function Chevron({ dir }: { dir: "up" | "down" | "left" | "right" }) {
  const d = {
    up: "M 2.5 7.5 L 6 4 L 9.5 7.5",
    down: "M 2.5 4.5 L 6 8 L 9.5 4.5",
    left: "M 7.5 2.5 L 4 6 L 7.5 9.5",
    right: "M 4.5 2.5 L 8 6 L 4.5 9.5",
  }[dir];
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      <path d={d} fill="none" stroke="currentColor" strokeWidth="1.3" />
    </svg>
  );
}

/** The cover as a glyph: a closed board, spine to the left. */
export function CoverGlyph() {
  return (
    <svg viewBox="0 0 12 14" width="11" height="13" aria-hidden="true">
      <rect
        x="1.5"
        y="1.5"
        width="9"
        height="11"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.3"
      />
      <line
        x1="4"
        y1="1.5"
        x2="4"
        y2="12.5"
        stroke="currentColor"
        strokeWidth="1"
      />
    </svg>
  );
}

/** L'alignement d'un bloc : quatre lignes de longueurs inégales, calées à
 *  gauche, au centre ou à droite. */
export function AlignGlyph({ cote }: { cote: "gauche" | "centre" | "droite" }) {
  const longueurs = [9, 6, 9, 5];
  const x = (l: number) =>
    cote === "gauche" ? 1.5 : cote === "droite" ? 10.5 - l : 6 - l / 2;
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      {longueurs.map((l, i) => (
        <line
          key={i}
          x1={x(l)}
          y1={2 + i * 2.7}
          x2={x(l) + l}
          y2={2 + i * 2.7}
          stroke="currentColor"
          strokeWidth="1.3"
        />
      ))}
    </svg>
  );
}

/** Les trois signes de l'alerte de qualité, 12 px, dessinés en
 *  `currentColor` : c'est la pastille qui les porte (`SignesAlerte`) qui
 *  pose le fond et l'encre, lisibles sur toute photo. Sans texte : le nom
 *  est sur la pastille. */

/** Trop petite : quatre carrés, l'image qui se pixelise. */
export function SignePixels() {
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      {[
        [1.5, 1.5],
        [6.5, 1.5],
        [1.5, 6.5],
        [6.5, 6.5],
      ].map(([x, y]) => (
        <rect key={`${x}-${y}`} x={x} y={y} width="4" height="4" fill="currentColor" />
      ))}
    </svg>
  );
}

/** Sombre : un demi-disque, la lune. */
export function SigneLune() {
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      <circle cx="6" cy="6" r="4.6" fill="none" stroke="currentColor" strokeWidth="1.2" />
      <path d="M 6 1.4 A 4.6 4.6 0 0 0 6 10.6 Z" fill="currentColor" />
    </svg>
  );
}

/** Peu nette : deux cercles, l'extérieur estompé, le contour qui bave. */
export function SigneFlou() {
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      <circle
        cx="6"
        cy="6"
        r="4.6"
        fill="none"
        stroke="currentColor"
        strokeOpacity="0.45"
        strokeWidth="1.6"
      />
      <circle cx="6" cy="6" r="2.4" fill="currentColor" />
    </svg>
  );
}

// ---- the chrome's line icons -------------------------------------------------
// 24-unit grid, 1.5 stroke in currentColor, decorative: the label or the
// title beside each one says what it is.

function Trait({ children, taille = 17 }: { children: React.ReactNode; taille?: number }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={taille}
      height={taille}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export const IcoLivre = () => (
  <Trait>
    <path d="M3 5.5C6 4.5 9 4.5 12 6c3-1.5 6-1.5 9-.5V19c-3-1-6-1-9 .5-3-1.5-6-1.5-9-.5z" />
    <path d="M12 6v13.5" />
  </Trait>
);
export const IcoTri = () => (
  <Trait>
    <rect x="3.5" y="3.5" width="7" height="7" rx="1" />
    <rect x="13.5" y="3.5" width="7" height="7" rx="1" />
    <rect x="3.5" y="13.5" width="7" height="7" rx="1" />
    <path d="M14 14l6 6M20 14l-6 6" />
  </Trait>
);
export const IcoPlanches = () => (
  <Trait>
    <rect x="3" y="4" width="8" height="6" rx="1" />
    <rect x="13" y="4" width="8" height="6" rx="1" />
    <rect x="3" y="14" width="8" height="6" rx="1" />
    <rect x="13" y="14" width="8" height="6" rx="1" />
  </Trait>
);
export const IcoEnvoi = () => (
  <Trait>
    <path d="M6 3h9l4 4v14H6z" />
    <path d="M14 3v5h5M9 13l2.5 2.5L16 11" />
  </Trait>
);
export const IcoPolice = () => (
  <Trait>
    <path d="M4 7h16M4 12h16M4 17h10" />
  </Trait>
);
export const IcoPlus = () => (
  <Trait>
    <path d="M12 5v14M5 12h14" />
  </Trait>
);
export const IcoOuvrir = () => (
  <Trait>
    <path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z" />
  </Trait>
);
export const IcoAnnuler = () => (
  <Trait>
    <path d="M9 14L4 9l5-5" />
    <path d="M4 9h10a6 6 0 010 12h-3" />
  </Trait>
);
export const IcoRetablir = () => (
  <Trait>
    <path d="M15 14l5-5-5-5" />
    <path d="M20 9H10a6 6 0 000 12h3" />
  </Trait>
);
/** The sidebar itself: a window with its left column marked. */
export const IcoPanneau = () => (
  <Trait>
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <path d="M9 4v16" />
  </Trait>
);

/** The brand: two pages and a fold, the book's ink beside its terracotta.
 *  The only place a square of terracotta sits outside a chapter. */
export function MarqueBooklibre() {
  return <span className="marque-booklibre" aria-hidden="true" />;
}
