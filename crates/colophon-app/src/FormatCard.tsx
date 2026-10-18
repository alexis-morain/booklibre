// La carte d'un format : la double page ouverte, à une échelle commune à
// toutes les cartes (le 30 × 30 est la grande), son nom et ses cotes.
// L'écran de création et le panneau Recomposer la partagent.

import type { FormatPreset } from "./bridge";
import { cm, echelleCommune, formatLabel, planchePx } from "./visionneuse";

/** Every card draws at this scale, so the shapes compare in size as well as
 *  in proportion: the 30 × 30 is the large one. */
const ECHELLE_CARTE = echelleCommune(140);

export function FormatCard({
  f,
  active,
  onPick,
  note,
}: {
  f: FormatPreset;
  active: boolean;
  onPick: () => void;
  /** Une mention sous les cotes : « non imprimable par Colophon ». */
  note?: string;
}) {
  const p = planchePx(f, ECHELLE_CARTE);
  return (
    <button
      type="button"
      className={"format-card" + (active ? " active" : "")}
      onClick={onPick}
      aria-pressed={active}
    >
      <span className="format-preview">
        <span className="format-page" style={{ width: p.w / 2, height: p.h }} />
        <span className="format-page" style={{ width: p.w / 2, height: p.h }} />
      </span>
      <span className="format-name">{formatLabel(f.name)}</span>
      <span className="format-dims">
        {cm(f.w)} × {cm(f.h)} cm
      </span>
      {note && <span className="format-note">{note}</span>}
    </button>
  );
}
