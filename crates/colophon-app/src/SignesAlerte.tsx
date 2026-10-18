// Les signes de l'alerte de qualité sur une case, une photo de la réserve
// ou une planche de la table lumineuse : un signe par cause, la plus grave
// à gauche, et une bulle qui dit en clair ce qui va mal et quoi faire.
//
// Trois signes et pas un : le triangle unique se lisait comme une note de
// la photo. La résolution, seule qui bloque l'impression, porte une
// pastille rouge ; le sombre et le peu net sont des conseils et restent
// neutres. Chaque signe se nomme pour la voix (`role="img"`).
//
// La bulle s'ouvre au survol des signes, et au focus de qui les porte
// (`ouverte`, ou la règle CSS de la cellule) : elle est en CSS, sans état,
// donc la même partout.

import type { CSSProperties } from "react";
import { SigneFlou, SigneLune, SignePixels } from "./icons";
import { Alerte, LieuDAlerte, detailDAlerte, phraseDAlerte } from "./photos";

const SIGNE = {
  sous_resolution: SignePixels,
  sombre: SigneLune,
  floue: SigneFlou,
} as const;

export function SignesAlerte({
  alertes,
  lieu = "case",
  ouverte = false,
  enLigne = false,
  muet = false,
  style,
}: {
  /** Dans l'ordre de `badgesDe`, qui est celui de la gravité. */
  alertes: readonly Alerte[];
  lieu?: LieuDAlerte;
  /** La bulle tenue ouverte, quand la case porteuse a le focus. */
  ouverte?: boolean;
  /** Dans une ligne de texte (la table lumineuse), et non posée sur une
   *  photo. */
  enLigne?: boolean;
  /** Du décor, sans nom ni bulle : quand le texte d'à côté dit déjà tout
   *  (la légende du pied). */
  muet?: boolean;
  style?: CSSProperties;
}) {
  if (alertes.length === 0) return null;
  return (
    <span
      className={"slot-alerte" + (enLigne ? " en-ligne" : "") + (ouverte ? " ouverte" : "")}
      style={style}
    >
      <span className="alerte-signes">
        {alertes.map((a) => {
          const Signe = SIGNE[a.code];
          return (
            <span
              key={a.code}
              className={"alerte-signe" + (a.code === "sous_resolution" ? " grave" : "")}
              role={muet ? undefined : "img"}
              aria-label={muet ? undefined : phraseDAlerte(a, lieu)}
              aria-hidden={muet ? true : undefined}
            >
              <Signe />
            </span>
          );
        })}
      </span>
      {!muet && (
        <span className="alerte-bulle" role="tooltip" aria-hidden="true">
          {alertes.map((a) => {
            const detail = detailDAlerte(a);
            return (
              <span key={a.code} className="alerte-bulle-ligne">
                <span>{phraseDAlerte(a, lieu)}</span>
                {detail && <small>{detail}</small>}
              </span>
            );
          })}
        </span>
      )}
    </span>
  );
}
