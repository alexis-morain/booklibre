// La feuille qui tourne : le modèle, et rien qui touche au DOM.
//
// L'unité de Colophon est la planche double. La feuille de papier, elle, n'en
// est pas une : elle porte au recto la page de droite de la planche N, et au
// verso la page de gauche de la planche N + 1. Un livre relié n'a pas d'autre
// géométrie, et celle-ci tombe juste avec l'invariant du projet — aucune image
// ne traverse le pli, donc rien de ce qui bouge n'est coupé par le mouvement.
// Deux planches rasterisées suffisent à un tour : la feuille et les deux
// moitiés qui restent en place sortent toutes les quatre de ces deux images.
//
// Tout ce qui décide vit ici, sans DOM ni React : ce qu'un indice et un sens
// désignent, où commence un coin, ce qu'un relâchement veut dire, et la courbe
// du mouvement. Ce fichier se teste sous Vitest sans navigateur ; les pixels,
// les événements et les trames vivent dans `Feuilletage.tsx`.

/** Le sens du tour : +1 vers la planche suivante, −1 vers la précédente. */
export type Sens = 1 | -1;

/** Une des deux pages d'une planche. */
export type Cote = "gauche" | "droite";

/** Une demi-planche à afficher : de quelle planche, et de quel côté du pli.
 *  La planche `COUVERTURE` n'a qu'une face, son avant, rangée à droite : c'est
 *  là qu'un livre fermé pose son plat. */
export type Face = { planche: number; cote: Cote };

/** L'indice de la couverture, le même que celui d'App. */
export const COUVERTURE = -1;

/**
 * Ce qu'un tour met à l'écran. Quatre faces, deux planches : la feuille en
 * porte deux, et les deux qui ne bougent pas encadrent le mouvement.
 */
export type Feuille = {
  sens: Sens;
  depuis: number;
  vers: number;
  /** Ce que la feuille montre à plat, avant le tour. */
  recto: Face;
  /** Ce qu'elle découvre en passant la verticale. */
  verso: Face;
  /** La moitié gauche qui ne bouge pas, sous la feuille ou à côté. `null`
   *  quand le livre est fermé d'un côté : il n'y a que la table. */
  fixeGauche: Face | null;
  /** La moitié droite qui ne bouge pas. */
  fixeDroite: Face;
};

/**
 * La feuille qu'un tour ferait tourner, ou `null` si ce tour n'existe pas.
 *
 * La couverture en est une depuis 1.2-c : le livre fermé s'ouvre, et son plat
 * tourne autour du dos comme une feuille. Son recto est l'avant de la
 * couverture, son verso le contre-plat — la page de gauche de la première
 * planche, que rien n'imprime parce qu'elle est collée dans le carton. À
 * gauche il n'y a alors que la table.
 */
export function feuilleDe(
  depuis: number,
  sens: Sens,
  total: number,
): Feuille | null {
  const vers = depuis + sens;
  if (!dansLeLivre(depuis, total) || !dansLeLivre(vers, total)) return null;
  const ferme = depuis === COUVERTURE || vers === COUVERTURE;
  const avant: Face = { planche: COUVERTURE, cote: "droite" };
  return sens === 1
    ? {
        sens,
        depuis,
        vers,
        recto: ferme ? avant : { planche: depuis, cote: "droite" },
        verso: { planche: vers, cote: "gauche" },
        fixeGauche: ferme ? null : { planche: depuis, cote: "gauche" },
        fixeDroite: { planche: vers, cote: "droite" },
      }
    : {
        sens,
        depuis,
        vers,
        recto: { planche: depuis, cote: "gauche" },
        verso: ferme ? avant : { planche: vers, cote: "droite" },
        fixeGauche: ferme ? null : { planche: vers, cote: "gauche" },
        fixeDroite: { planche: depuis, cote: "droite" },
      };
}

/** Une planche de l'intérieur, ou la couverture d'un livre qui en a une. */
function dansLeLivre(planche: number, total: number): boolean {
  return Number.isInteger(planche) && planche >= COUVERTURE && planche < total && total > 0;
}

function dansLInterieur(planche: number, total: number): boolean {
  return Number.isInteger(planche) && planche >= 0 && planche < total;
}

/**
 * Les planches dont l'image doit exister avant qu'un geste commence. Sans
 * elles la première image du mouvement saute, et une animation qui saute est
 * pire que pas d'animation du tout : on préfère alors le changement sec.
 */
export function planchesAPrecharger(index: number, total: number): number[] {
  return [index - 1, index, index + 1].filter((n) => dansLInterieur(n, total));
}

/**
 * La zone de coin, en fractions de la planche entière. Elle est délimitée
 * explicitement parce que le glisser sert déjà au recadrage dans l'éditeur :
 * deux sens pour le même geste au même endroit est un défaut, et la seule
 * défense contre cela est une frontière écrite quelque part.
 *
 * 0,15 de la planche double fait 0,30 d'une page : le pouce le trouve, et il
 * reste soixante-dix pour cent de la page où glisser ne tourne rien.
 */
export const COIN = Object.freeze({ largeur: 0.15, hauteur: 0.3 });

/**
 * Le sens qu'un point démarre, ou `null` s'il ne tombe dans aucun coin.
 * `x` et `y` sont des fractions de la planche, origine en haut à gauche.
 */
export function coinTouche(x: number, y: number): Sens | null {
  if (y < 1 - COIN.hauteur || y > 1 || x < 0 || x > 1) return null;
  if (x >= 1 - COIN.largeur) return 1;
  if (x <= COIN.largeur) return -1;
  return null;
}

/**
 * Où en est le tour quand le coin libre est sous le pointeur.
 *
 * La feuille est charnière au pli : son bord libre part d'un bord de la
 * planche, passe par le pli à mi-course et finit sur l'autre bord. Le bord
 * libre est donc en `largeur / 2 · (1 ± cos πp)`, et le progrès s'en déduit
 * par un arc cosinus. C'est la seule mise en correspondance qui fasse suivre
 * le papier au doigt plutôt que de le tirer avec un facteur inventé.
 */
export function progresDuPointeur(
  x: number,
  largeur: number,
  sens: Sens,
): number {
  if (largeur <= 0) return 0;
  const relatif = (2 * x) / largeur - 1;
  const cosinus = sens === 1 ? relatif : -relatif;
  return Math.acos(borne(cosinus, -1, 1)) / Math.PI;
}

/**
 * L'angle de la feuille, en degrés, pour un `rotateY`. Le signe suit le sens :
 * la feuille se lève vers le lecteur et bascule de l'autre côté du pli.
 */
export function angle(progres: number, sens: Sens): number {
  return -sens * 180 * borne(progres, 0, 1);
}

/** Passé la verticale, c'est le verso qu'on regarde. */
export function versoVisible(progres: number): boolean {
  return progres > 0.5;
}

/**
 * Le relief du tour : nul à plat des deux côtés, maximum à la verticale. Le
 * voile de courbure sur la feuille et l'ombre portée sur la page dessous
 * lisent ce seul nombre, donc ils ne peuvent pas se désaccorder.
 */
export function relief(progres: number): number {
  return Math.sin(Math.PI * borne(progres, 0, 1));
}

/** Le nombre de lamelles d'une feuille. Assez pour que l'œil lise une
 *  courbe et non une ligne brisée sur une demi-planche, assez peu pour que
 *  vingt-quatre `drawImage` à la pose d'un tour ne se sentent pas. */
export const LAMELLES = 12;

/** De combien de degrés le bord libre mène le pli à mi-tour, quand le
 *  papier est le plus courbé. */
export const COURBURE = 50;

/**
 * La page courbe (1.2-c, la lamelle levée) : l'angle de chacune des `n`
 * lamelles, du pli vers le bord libre, dans la convention d'`angle`.
 *
 * La feuille moyenne est celle d'`angle` ; le pli traîne de la moitié de la
 * courbure et le bord libre la mène d'autant, la courbure suivant `relief` —
 * nulle à plat des deux côtés, entière à la verticale. Chaque angle est borné
 * à la course d'un tour : une lamelle ne passe jamais sous la page d'arrivée
 * ni derrière celle qu'elle quitte, et la suite reste monotone, donc le
 * papier se courbe sans jamais se plier.
 */
export function courbe(progres: number, sens: Sens, n: number): number[] {
  const theta = angle(progres, sens);
  const avance = COURBURE * relief(progres);
  return Array.from({ length: n }, (_, k) => {
    const t = n > 1 ? k / (n - 1) : 0.5;
    const tourne = -sens * theta + avance * (t - 0.5);
    return -sens * borne(tourne, 0, 180);
  });
}

/** Le voile d'une lamelle : nul à plat, le plus sombre debout. Le même
 *  plafond que l'ancien voile de la feuille entière. */
export function voileDe(angleDeg: number): number {
  return 0.34 * Math.abs(Math.sin((angleDeg * Math.PI) / 180));
}

/**
 * Ce que porte chaque lamelle, du pli vers le bord libre, en fractions de la
 * page (0 au bord gauche de la page). Vers l'avant la feuille est la page de
 * droite, dont le pli est le bord gauche ; vers l'arrière c'est la page de
 * gauche, pliée à droite. Le verso est la bande miroir de la page découverte :
 * vue de dos, la feuille se lit à l'envers.
 */
export function bandes(
  n: number,
  sens: Sens,
): { recto: [number, number]; verso: [number, number] }[] {
  return Array.from({ length: n }, (_, k) => {
    const pres: [number, number] = [k / n, (k + 1) / n];
    const loin: [number, number] = [1 - (k + 1) / n, 1 - k / n];
    return sens === 1 ? { recto: pres, verso: loin } : { recto: loin, verso: pres };
  });
}

/** Au-delà de cette part du tour, un relâchement finit le mouvement. */
export const SEUIL_PROGRES = 0.5;

/** Un lancer plus rapide que ceci (en tours par seconde) décide seul. */
export const SEUIL_VITESSE = 0.9;

export type Issue = "termine" | "revient";

/**
 * Ce qu'un relâchement veut dire. La vitesse passe avant la distance : une
 * chiquenaude sur le coin tourne la page même si le doigt n'a fait qu'un
 * centimètre, et un retour franc annule même à quatre-vingts pour cent — c'est
 * ce qui rend le geste réversible pour de bon plutôt qu'en théorie.
 */
export function issue(progres: number, vitesse: number): Issue {
  if (vitesse >= SEUIL_VITESSE) return "termine";
  if (vitesse <= -SEUIL_VITESSE) return "revient";
  return progres >= SEUIL_PROGRES ? "termine" : "revient";
}

/** Un geste plus court et plus bref que ceci est un clic, pas un glisser. */
export const CLIC = Object.freeze({ course: 6, duree: 320 });

export function estUnClic(course: number, duree: number): boolean {
  return course <= CLIC.course && duree <= CLIC.duree;
}

/** Un tour entier, en millisecondes. */
export const DUREE_TOUR = 420;

/** Plancher : sous cela le mouvement n'est plus lu, il clignote. */
export const DUREE_MIN = 150;

/**
 * Ce que coûte la fin du mouvement, proportionnel à ce qu'il reste à parcourir.
 * Un relâchement à quatre-vingt-dix pour cent ne doit pas rejouer un tour
 * entier, ni retomber d'un coup.
 */
export function dureeRestante(de: number, vers: number): number {
  return Math.max(DUREE_MIN, DUREE_TOUR * Math.abs(vers - de));
}

/**
 * L'adoucissement, et il est sobre : le mouvement part à sa vitesse et
 * s'arrête sur le papier. Pas de rebond, pas d'élastique — c'est la seule
 * animation du projet qui a le droit d'être remarquée, et elle l'est parce
 * qu'elle montre un livre, pas parce qu'elle fait un effet.
 */
export function adoucir(t: number): number {
  const u = 1 - borne(t, 0, 1);
  return 1 - u * u * u;
}

function borne(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}
