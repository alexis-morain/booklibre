// Un panneau qui s'ouvre depuis la colonne des propriétés y serait coupé :
// la colonne défile, donc tout ce qui en déborde est rogné. Le sélecteur de
// dispositions l'a été (#166), celui des ornements aussi. Les deux passent
// par ce crochet ; ailleurs (barre contextuelle, menu), le panneau garde sa
// position de CSS.

import { useLayoutEffect, useState, type RefObject } from "react";
import { styleFixeAGauche } from "./popover";

export function usePanneauFixe(
  racine: RefObject<HTMLElement | null>,
  ouvert: boolean,
): React.CSSProperties | undefined {
  const [style, setStyle] = useState<React.CSSProperties | undefined>();
  useLayoutEffect(() => {
    const el = racine.current;
    if (!ouvert || !el?.closest(".props")) return setStyle(undefined);
    setStyle(styleFixeAGauche(el.getBoundingClientRect(), { w: window.innerWidth, h: window.innerHeight }));
  }, [ouvert, racine]);
  return style;
}
