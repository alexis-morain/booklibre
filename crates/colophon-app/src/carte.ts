// Les cartes d'un album de voyage, tenues une fois par album ouvert, et le seul
// endroit qui sait transformer leur dessin en tracé.
//
// **Le dessin vient du moteur, jamais d'un second calcul.** `carte::dessiner`
// est celui du PDF : le moteur le rend par plage d'étapes (`--dump-cartes`,
// commande `cartes`), et l'écran ne projette rien, ne simplifie rien, ne
// découpe rien. Une côte que l'écran montre est une côte que le papier
// imprime, parce qu'il n'y a qu'un dessin.
//
// Comme pour `ornement.ts`, **un seul tracé pour les deux rendus** : le DOM pose
// des `<path d>`, le canvas des `Path2D` construits sur la même chaîne
// (`attributD`). La scène, elle, ne sait que la plage : la parité Rust/TypeScript
// porte sur la boîte, l'angle et l'identité.

import { attributD, Chemin } from "./ornement";

/** Comment un calque se pose, en mm **à la taille du dessin** : la mise à
 *  l'échelle de la boîte (`viewBox`, ou `ctx.scale`) les met à la taille de
 *  la page, épaisseurs et tirets compris. Miroir de `carte::Style`. */
export type StyleCalque =
  | { style: "remplir" }
  | { style: "tracer"; epaisseur_mm: number; pointille?: [number, number] | null };

export type Calque = {
  role: string;
  style: StyleCalque;
  /** `#rrggbb`, celle de `carte::Role::encre`. */
  encre: string;
  /** Le contour de même encre d'un calque rempli : recolle les moitiés d'un
   *  polygone coupé à l'antiméridien. */
  contour_mm?: number;
  chemins: Chemin[];
};

/** `viewbox` est `[0, 0, largeur, hauteur]` en mm, `y` vers le bas. */
export type DessinCarte = {
  viewbox: [number, number, number, number];
  niveau: string;
  calques: Calque[];
};

let dessins = new Map<string, DessinCarte>();

export const cleCarte = (debut: number, fin: number): string => `${debut}-${fin}`;

/** Poser les cartes de l'album qui vient de s'ouvrir. */
export function setCartes(parPlage: Record<string, DessinCarte>): void {
  dessins = new Map(Object.entries(parPlage));
}

/** Le dessin d'une plage, ou `undefined` : le voyage a disparu ou n'a plus ces
 *  étapes. **Ce n'est pas une erreur** — le moteur ne dessine rien non plus, et
 *  le prévol le dit (`carte_vide`). */
export function carteDe(debut: number, fin: number): DessinCarte | undefined {
  return dessins.get(cleCarte(debut, fin));
}

/** Le rapport largeur/hauteur que la boîte d'une carte garde. Miroir de
 *  `carte::Dessin::rapport`. */
export function rapportCarte(d: DessinCarte): number {
  return d.viewbox[2] / d.viewbox[3];
}

export function attributViewBox(d: DessinCarte): string {
  return d.viewbox.join(" ");
}

/** Les attributs SVG d'un calque, pour le DOM. Les mêmes valeurs que
 *  `peindre` pose au canvas, et que l'émetteur PDF écrit dans son flux. */
export function attributsCalque(c: Calque): {
  fill: string;
  stroke: string;
  strokeWidth?: number;
  strokeDasharray?: string;
} {
  if (c.style.style === "remplir") {
    return c.contour_mm === undefined
      ? { fill: c.encre, stroke: "none" }
      : { fill: c.encre, stroke: c.encre, strokeWidth: c.contour_mm };
  }
  const p = c.style.pointille;
  return {
    fill: "none",
    stroke: c.encre,
    strokeWidth: c.style.epaisseur_mm,
    strokeDasharray: p ? `${p[0]} ${p[1]}` : undefined,
  };
}

/** Peindre une carte dans un rectangle du canvas, en millimètres et origine en
 *  haut à gauche — le repère de la scène. La rotation est posée par
 *  l'appelant. Tout est **découpé à la boîte** : un trait au bord de la carte
 *  ne déborde pas de l'encre que la boîte promet, ici comme au PDF. */
export function peindreCarte(
  ctx: CanvasRenderingContext2D,
  dessin: DessinCarte,
  boite: { x: number; y: number; w: number; h: number },
): void {
  const [minx, miny, vw, vh] = dessin.viewbox;
  ctx.save();
  ctx.translate(boite.x, boite.y);
  ctx.scale(boite.w / vw, boite.h / vh);
  ctx.translate(-minx, -miny);
  ctx.beginPath();
  ctx.rect(minx, miny, vw, vh);
  ctx.clip();
  ctx.lineJoin = "round";
  ctx.lineCap = "round";
  for (const c of dessin.calques) {
    const chemins = c.chemins.map((ch) => ({ ch, p: new Path2D(attributD(ch)) }));
    if (c.style.style === "remplir") {
      ctx.fillStyle = c.encre;
      for (const { ch, p } of chemins) ctx.fill(p, ch.evenodd ? "evenodd" : "nonzero");
      if (c.contour_mm !== undefined) {
        ctx.strokeStyle = c.encre;
        ctx.lineWidth = c.contour_mm;
        ctx.setLineDash([]);
        for (const { p } of chemins) ctx.stroke(p);
      }
    } else {
      ctx.strokeStyle = c.encre;
      ctx.lineWidth = c.style.epaisseur_mm;
      ctx.setLineDash(c.style.pointille ?? []);
      for (const { p } of chemins) ctx.stroke(p);
    }
  }
  ctx.restore();
}
