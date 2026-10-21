// Le feuilletage : la scène où les feuilles tournent, et le seul endroit du
// projet où une animation a le droit d'être remarquée — parce qu'elle est le
// produit. Un album n'est pas un diaporama : c'est un livre, et un livre se
// feuillette.
//
// **Ce qui bouge est une image du PDF, jamais un redessin de la scène.**
// L'aperçu fidèle existe précisément pour que ce qu'on montre soit ce que
// l'imprimeur produira ; une feuille qui se redessinerait en tournant
// mentirait sur le résultat au moment exact où le lecteur regarde le plus. Les
// deux faces de la feuille et les deux moitiés immobiles sortent toutes du
// même bitmap, par `drawImage` : quatre morceaux, deux planches, zéro
// arithmétique de mise en page.
//
// **Où ce geste vit, et où il ne vit pas.** Ici, et dans les surfaces de
// lecture. Jamais dans l'éditeur, où le glisser sert déjà au recadrage : deux
// sens pour le même geste au même endroit est un défaut, pas une richesse. La
// zone de coin est écrite dans `feuille.ts`, une fois, et le curseur comme le
// test de départ la lisent tous les deux là.
//
// **Le geste n'est jamais le seul chemin.** Les flèches, Page haut et Page bas
// font exactement la même chose, par la même mécanique — et un lecteur qui a
// demandé moins de mouvement reçoit un changement sec, sans que rien d'autre
// ne change pour lui.
//
// **La page se courbe, en rendu maison** (1.2-c, la lamelle levée). La
// feuille est une chaîne de lamelles en CSS 3D, chacune enfant de la
// précédente et tournée d'un petit angle de plus ; chacune est un `drawImage`
// du même bitmap, recto et verso. Les angles sortent de `feuille.ts::courbe`,
// le voile de chaque lamelle de `voileDe`. Aucune bibliothèque n'entre, donc
// aucun audit de licence n'est dû.
//
// **Le livre fermé s'ouvre.** Sur la couverture, la scène montre `Livre3D`,
// son avant tiré du PDF de couverture ; l'ouvrir le met de face, puis son plat
// tourne autour du dos comme une feuille (`feuilleDe(COUVERTURE, 1)`), et le
// refermer rejoue le même tour à l'envers.

import {
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { Album } from "./album";
import {
  Cote,
  COUVERTURE,
  Face,
  Feuille,
  LAMELLES,
  Sens,
  adoucir,
  bandes,
  coinTouche,
  COIN,
  courbe,
  dureeRestante,
  estUnClic,
  feuilleDe,
  issue,
  planchesAPrecharger,
  progresDuPointeur,
  relief,
  voileDe,
} from "./feuille";
import { t } from "./i18n";
import { Decoupe, Part, bandeDe, partDe } from "./lecture";
import { Livre3D } from "./Livre3D";
import { echantillon } from "./mesure";
import { Raster, precharger, rasterPret, rasteriser } from "./raster";

/** Ce qu'App peut demander au feuilletage : tourner, et savoir s'il a pris. */
export type Tourneur = {
  /** Lance un tour animé. Rend `false` quand il n'a pas pu — au lecteur de
   *  changer de planche sèchement, ce que fait déjà App. */
  tourner(sens: number): boolean;
  /** Referme la loupe si elle est ouverte ; `false` sinon. */
  fermerLoupe(): boolean;
};

/** Ce qu'il faut pour montrer le livre fermé et faire tourner son plat : le
 *  PDF de couverture tiré à `largeur`, et deux découpes de sa feuille —
 *  l'avant rogné, que montre le livre fermé, et l'avant avec le fond perdu de
 *  l'album, qui a la forme d'une demi-planche et tourne à sa place. */
export type Couverture = { largeur: number; avant: Decoupe; avecFond: Decoupe };

/** Le livre tel que `Livre3D` le dessine. */
export type LivreFerme = {
  album: Album;
  dosMm: number | null;
  pages: number;
  /** Le fond perdu de l'album, en millimètres : la planche du PDF le porte. */
  bleedMm: number;
};

/** Un tour en cours : la feuille, les images qui l'alimentent, et la
 *  manière dont il a commencé. */
type Tour = {
  feuille: Feuille;
  /** Par planche ; la couverture sous `COUVERTURE`. */
  images: Map<number, Raster>;
  /** Au clavier ou au clic, la feuille se conduit seule jusqu'au bout. */
  auto: boolean;
};

/** Le temps que met le livre fermé à se mettre de face avant que son plat
 *  tourne. Écrit aussi dans `styles.css` (`.livre3d.de-face`). */
const DUREE_POSE = 300;

/** Le lecteur qui a demandé moins de mouvement reçoit un changement sec. Lu à
 *  chaque geste plutôt qu'abonné : le réglage change entre deux gestes, jamais
 *  pendant l'un d'eux. */
function mouvementReduit(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export function Feuilletage({
  planche,
  total,
  cle,
  largeur,
  livre,
  couverture,
  onPlanche,
  onLoupe,
  onErreur,
  ref,
}: {
  /** L'indice de la planche à l'écran, base zéro ; `COUVERTURE` pour le
   *  livre fermé. */
  planche: number;
  total: number;
  cle: number;
  /** La largeur de la planche double, en pixels CSS. */
  largeur: number;
  livre: LivreFerme;
  /** Null sans profil ou sans rendu de couverture : le livre fermé montre
   *  alors sa vignette, et s'ouvre sèchement. */
  couverture: Couverture | null;
  onPlanche: (sens: number) => boolean;
  /** Un clic sur une page : la loupe. */
  onLoupe: (cote: Cote) => void;
  onErreur: (message: string) => void;
  ref?: React.Ref<Tourneur | null>;
}) {
  const [repos, setRepos] = useState<Raster | null>(null);
  const [tour, setTour] = useState<Tour | null>(null);
  const [coinSurvole, setCoinSurvole] = useState<Sens | null>(null);
  // Le livre fermé de face : pendant la pose qui précède l'ouverture, et à
  // l'arrivée d'une fermeture, avant de revenir au trois-quarts.
  const [deFace, setDeFace] = useState(false);

  const scene = useRef<HTMLDivElement>(null);
  const lamelles = useRef<(HTMLDivElement | null)[]>([]);
  const voiles = useRef<(HTMLDivElement | null)[]>([]);
  const ombre = useRef<HTMLDivElement>(null);
  const trame = useRef<number | null>(null);
  const pose = useRef<ReturnType<typeof setTimeout> | null>(null);
  const progres = useRef(0);
  const geste = useRef<Geste | null>(null);
  const clic = useRef<{ id: number; x: number; y: number; t: number; cote: Cote } | null>(null);

  const trim = livre.album.trim_mm;
  const b = livre.bleedMm;
  // La planche du PDF, fond perdu compris : c'est sa forme qui fixe la
  // hauteur de la scène quand aucune image n'est encore là.
  const hauteurCalculee = Math.round((largeur * (trim.h + 2 * b)) / (2 * trim.w + 2 * b));

  // La planche à plat. Prise dans le cache sans attendre quand elle y est,
  // ce qui est le cas courant grâce au préchargement ; sinon on garde la
  // dernière dessinée à l'écran le temps du tirage, plutôt que d'ouvrir un
  // trou blanc au milieu d'un livre.
  const pret = planche >= 0 ? rasterPret("album", planche + 1, cle, largeur) : null;
  const affiche = pret ?? repos;

  useEffect(() => {
    if (largeur <= 0 || planche < 0) return;
    if (pret) {
      if (pret !== repos) setRepos(pret);
      return;
    }
    let vivant = true;
    rasteriser("album", planche + 1, cle, largeur).then(
      (r) => vivant && setRepos(r),
      (e) => vivant && onErreur(String(e)),
    );
    return () => {
      vivant = false;
    };
    // `onErreur` change d'identité à chaque rendu de la fenêtre et relancerait
    // le tirage à chaque battement de la ligne de statut.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [planche, cle, largeur, pret, repos]);

  // L'avant du livre fermé, tiré du PDF de couverture.
  const [avant, setAvant] = useState<Raster | null>(null);
  const couvPrete = couverture ? rasterPret("couverture", 1, cle, couverture.largeur) : null;
  useEffect(() => {
    if (!couverture || couverture.largeur <= 0) return;
    if (couvPrete) {
      if (couvPrete !== avant) setAvant(couvPrete);
      return;
    }
    if (planche > 0) return;
    let vivant = true;
    rasteriser("couverture", 1, cle, couverture.largeur).then(
      (r) => vivant && setAvant(r),
      () => {
        /* sans avant, le livre fermé montre sa vignette */
      },
    );
    return () => {
      vivant = false;
    };
  }, [planche, cle, couverture, couvPrete, avant]);

  // Les voisines, dessinées avant qu'un doigt se pose. Une fenêtre en
  // arrière-plan n'en dessine aucune (`precharger` le refuse), donc on
  // redemande quand elle revient : c'est aussi le seul moment où précharger
  // vaut quelque chose.
  useEffect(() => {
    if (largeur <= 0) return;
    const lancer = () =>
      precharger(
        "album",
        planchesAPrecharger(planche, total).map((n) => n + 1),
        cle,
        largeur,
      );
    lancer();
    const doc = window.document;
    doc.addEventListener("visibilitychange", lancer);
    return () => doc.removeEventListener("visibilitychange", lancer);
  }, [planche, total, cle, largeur]);

  // ---- le mouvement ------------------------------------------------------

  /** Écrit la position de chaque lamelle dans le DOM, sans repasser par
   *  React : soixante fois par seconde, un rendu React coûterait le
   *  mouvement. Chaque lamelle est l'enfant de la précédente, donc elle reçoit
   *  l'écart à sa voisine, pas son angle. */
  const appliquer = (p: number, sens: Sens) => {
    progres.current = p;
    const a = courbe(p, sens, LAMELLES);
    a.forEach((angle, k) => {
      const el = lamelles.current[k];
      if (el) el.style.transform = `rotateY(${k === 0 ? angle : angle - a[k - 1]}deg)`;
      const v = voiles.current[k];
      if (v) v.style.opacity = String(voileDe(angle));
    });
    if (ombre.current) ombre.current.style.opacity = String(relief(p) * 0.3);
  };

  const arreter = () => {
    if (trame.current !== null) cancelAnimationFrame(trame.current);
    trame.current = null;
  };

  const lacherPose = () => {
    if (pose.current !== null) clearTimeout(pose.current);
    pose.current = null;
  };

  useEffect(() => lacherPose, []);

  /** Finit le mouvement tout seul, de `de` à `vers`, puis fait ce qu'il faut. */
  const animer = (de: number, vers: number, sens: Sens, apres: () => void) => {
    arreter();
    const duree = dureeRestante(de, vers);
    const t0 = performance.now();
    let precedente = t0;
    const pas = (maintenant: number) => {
      echantillon("feuille.trame", maintenant - precedente);
      precedente = maintenant;
      const avance = Math.min(1, (maintenant - t0) / duree);
      appliquer(de + (vers - de) * adoucir(avance), sens);
      if (avance < 1) {
        trame.current = requestAnimationFrame(pas);
      } else {
        trame.current = null;
        apres();
      }
    };
    trame.current = requestAnimationFrame(pas);
  };

  /** L'image d'une planche, ou de la couverture, si elle est déjà dessinée. */
  const imagePrete = (n: number): Raster | null =>
    n === COUVERTURE
      ? couverture
        ? rasterPret("couverture", 1, cle, couverture.largeur)
        : null
      : rasterPret("album", n + 1, cle, largeur);

  /**
   * Monte la feuille, si un tour est possible et si ses images sont déjà
   * dessinées. Sans elles le premier pas du mouvement sauterait, et un
   * mouvement qui saute est pire que pas de mouvement : on rend `false`, et
   * le changement se fait sec.
   *
   * Depuis le livre fermé, le livre se met d'abord de face, puis le plat
   * tourne : la demande est prise (`true`) dès la pose.
   */
  const demarrer = (sens: Sens, auto: boolean): boolean => {
    if (tour || pose.current !== null || mouvementReduit() || largeur <= 0) return false;
    const feuille = feuilleDe(planche, sens, total);
    if (!feuille) return false;
    const images = new Map<number, Raster>();
    for (const n of [feuille.depuis, feuille.vers]) {
      const r = imagePrete(n);
      if (!r) return false;
      images.set(n, r);
    }
    if (feuille.depuis === COUVERTURE) {
      setDeFace(true);
      pose.current = setTimeout(() => {
        pose.current = null;
        setTour({ feuille, images, auto });
      }, DUREE_POSE);
      return true;
    }
    setTour({ feuille, images, auto });
    return true;
  };

  /** Le tour est allé au bout : la planche change, et la feuille s'efface sur
   *  la même trame — la planche d'arrivée est déjà dans le cache, donc elle
   *  est peinte avant que quoi que ce soit disparaisse. Un livre qu'on vient
   *  de refermer arrive de face, puis revient au trois-quarts. */
  const conclure = (sens: Sens) => {
    onPlanche(sens);
    setTour(null);
    if (planche + sens === COUVERTURE) {
      setDeFace(true);
      requestAnimationFrame(() => requestAnimationFrame(() => setDeFace(false)));
    } else {
      setDeFace(false);
    }
  };

  const annuler = () => {
    setTour(null);
    setDeFace(false);
  };

  // Le mouvement ne peut commencer qu'une fois la feuille dans le DOM : un
  // effet de mise en page part après le commit de React et avant la peinture,
  // donc la première trame est écrite au bon endroit et rien ne clignote.
  useLayoutEffect(() => {
    if (!tour) return;
    appliquer(0, tour.feuille.sens);
    if (tour.auto) {
      animer(0, 1, tour.feuille.sens, () => conclure(tour.feuille.sens));
    }
    return arreter;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tour]);

  // Ce qu'App tient de la feuille : une seule commande, et la réponse
  // honnête quand elle n'a pas pu la prendre.
  //
  // Une demande pendant qu'une feuille tourne est **avalée**, et c'est
  // délibéré : une flèche maintenue tourne alors une page toutes les quatre
  // cents millisecondes, ce qui est exactement le rythme d'un livre qu'on
  // feuillette. Laisser passer la demande ferait sauter la planche sous une
  // feuille dont les images sont celles d'avant. Mettre les demandes en file
  // ferait tourner une page de trop après qu'on a lâché la touche. Qui
  // parcourt vite a la table lumineuse (⌘3) et la règle du pied, qui sont
  // faites pour ça.
  useImperativeHandle(
    ref,
    (): Tourneur => ({
      tourner: (sens) => {
        if (sens !== 1 && sens !== -1) return false;
        if (tour || pose.current !== null) return true;
        return demarrer(sens, true);
      },
      fermerLoupe: () => false,
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [planche, total, cle, largeur, tour, couverture],
  );

  // La planche a changé ailleurs qu'au bout de ce tour — Début, Fin, la table
  // lumineuse, une recomposition : la feuille en vol n'a plus de sujet, et la
  // laisser finir poserait un indice calculé depuis un livre qui a bougé.
  useEffect(() => {
    if (pose.current !== null && planche !== COUVERTURE) {
      lacherPose();
      setDeFace(false);
    }
    if (!tour) return;
    if (planche === tour.feuille.depuis || planche === tour.feuille.vers) return;
    arreter();
    setTour(null);
    setDeFace(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [planche, tour]);

  // ---- le geste ----------------------------------------------------------

  const fractions = (e: React.PointerEvent) => {
    const r = scene.current?.getBoundingClientRect();
    if (!r || r.width <= 0 || r.height <= 0) return null;
    return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height, r };
  };

  // Sur le livre fermé, le glisser appartient à `Livre3D`, qui tourne le
  // volume : aucun coin n'y répond, on l'ouvre au clavier ou par le bouton.
  const fermeAuRepos = planche === COUVERTURE;

  const surPointeurEntre = (e: React.PointerEvent) => {
    if (geste.current || fermeAuRepos) return;
    const f = fractions(e);
    const c = f ? coinTouche(f.x, f.y) : null;
    const possible = c !== null && feuilleDe(planche, c, total) !== null;
    setCoinSurvole(possible ? c : null);
  };

  const surPointeurBas = (e: React.PointerEvent) => {
    // Une feuille en vol garde la main jusqu'au bout, comme au clavier.
    if (geste.current || tour || pose.current !== null || e.button !== 0 || fermeAuRepos) return;
    const f = fractions(e);
    if (!f) return;
    const sens = coinTouche(f.x, f.y);
    if (sens === null || !feuilleDe(planche, sens, total)) {
      // Hors des coins, un clic sur une page ouvre la loupe.
      clic.current = { id: e.pointerId, x: e.clientX, y: e.clientY, t: e.timeStamp, cote: f.x < 0.5 ? "gauche" : "droite" };
      return;
    }
    e.preventDefault();
    scene.current?.setPointerCapture(e.pointerId);
    geste.current = {
      sens,
      id: e.pointerId,
      t0: e.timeStamp,
      x0: e.clientX,
      y0: e.clientY,
      // Sans les images, le coin répond quand même : il tourne la page d'un
      // coup au relâchement. Un coin mort serait pris pour une panne.
      anime: demarrer(sens, false),
      vu: { p: 0, t: e.timeStamp },
      vitesse: 0,
    };
  };

  const surPointeurBouge = (e: React.PointerEvent) => {
    const g = geste.current;
    if (!g || g.id !== e.pointerId) return surPointeurEntre(e);
    if (!g.anime) return;
    const r = scene.current?.getBoundingClientRect();
    if (!r) return;
    const p = progresDuPointeur(e.clientX - r.left, r.width, g.sens);
    const dt = e.timeStamp - g.vu.t;
    // Une vitesse prise sur deux trames voisines est du bruit ; huit
    // millisecondes suffisent à la rendre lisible sans la rendre molle.
    if (dt >= 8) {
      g.vitesse = ((p - g.vu.p) / dt) * 1000;
      g.vu = { p, t: e.timeStamp };
    }
    appliquer(p, g.sens);
  };

  const surPointeurHaut = (e: React.PointerEvent) => {
    const c = clic.current;
    if (c && c.id === e.pointerId) {
      clic.current = null;
      if (estUnClic(Math.hypot(e.clientX - c.x, e.clientY - c.y), e.timeStamp - c.t)) {
        onLoupe(c.cote);
      }
      return;
    }
    const g = geste.current;
    if (!g || g.id !== e.pointerId) return;
    geste.current = null;
    setCoinSurvole(null);
    const course = Math.hypot(e.clientX - g.x0, e.clientY - g.y0);
    const unClic = estUnClic(course, e.timeStamp - g.t0);
    // Sans feuille montée, le coin n'a rien montré : seul un clic tourne la
    // page. Un glisser qui n'a rien fait bouger ne doit rien décider non plus.
    if (!g.anime) {
      if (unClic) onPlanche(g.sens);
      return;
    }
    const verdict = unClic ? "termine" : issue(progres.current, g.vitesse);
    if (verdict === "termine") {
      animer(progres.current, 1, g.sens, () => conclure(g.sens));
    } else {
      animer(progres.current, 0, g.sens, annuler);
    }
  };

  const surPointeurPerdu = (e: React.PointerEvent) => {
    clic.current = null;
    const g = geste.current;
    if (!g || g.id !== e.pointerId) return;
    geste.current = null;
    setCoinSurvole(null);
    if (g.anime) animer(progres.current, 0, g.sens, annuler);
  };

  // ---- ce qui est à l'écran ----------------------------------------------

  const hauteur = planche >= 0 && affiche ? affiche.hauteur : hauteurCalculee;
  const style = { width: `${largeur}px`, height: `${hauteur}px` };
  const f = tour?.feuille;
  const demi = largeur / 2;
  const ferme = planche === COUVERTURE || (f && (f.depuis === COUVERTURE || f.vers === COUVERTURE));

  /** Ce que montre une face : une moitié de planche, ou l'avant de la
   *  couverture avec le fond perdu de l'album. */
  const peinture = (face: Face): Peinture | null => {
    const r = tour?.images.get(face.planche) ?? null;
    if (!r) return null;
    if (face.planche === COUVERTURE) {
      return couverture ? { source: r.source, part: partDe(r.source, couverture.avecFond) } : null;
    }
    return {
      source: r.source,
      part: partDe(r.source, { x: face.cote === "droite" ? 0.5 : 0, y: 0, w: 0.5, h: 1 }),
    };
  };

  // L'échelle du livre fermé : la planche du PDF porte le fond perdu, le
  // livre fermé n'en a pas — il se pose donc sur la page de droite, dos au
  // pli, à la hauteur de la coupe.
  const echelle = hauteur / (trim.h + 2 * b);
  const avantRogne: Peinture | null =
    couverture && avant ? { source: avant.source, part: partDe(avant.source, couverture.avant) } : null;

  return (
    <div
      ref={scene}
      className={
        "feuilletage" + (coinSurvole !== null ? " au-coin" : "") + (ferme ? " ferme" : "")
      }
      style={style}
      // Le nom vit sur la scène et pas sur un morceau : au repos il y a un
      // rectangle, au milieu d'un tour il y en a vingt-six, et l'aperçu fidèle
      // reste une seule chose à annoncer dans les deux cas. Ce qui est peint
      // dedans est du décor, comme partout ailleurs dans ce projet.
      // Sur le livre fermé, c'est `Livre3D` qui se nomme, et il se prend au
      // clavier : une image ne peut pas contenir ce qui se prend.
      role={planche === COUVERTURE ? undefined : "img"}
      aria-label={planche === COUVERTURE ? undefined : t("fidele.pdf.aria")}
      onPointerDown={surPointeurBas}
      onPointerMove={surPointeurBouge}
      onPointerUp={surPointeurHaut}
      onPointerCancel={surPointeurPerdu}
      onPointerLeave={() => !geste.current && setCoinSurvole(null)}
    >
      {f ? (
        <>
          {/* Pendant un tour, les deux moitiés qui ne bougent pas, prises sur
              les planches que la feuille joint — à gauche, la table quand le
              livre est fermé de ce côté. */}
          {f.fixeGauche && (
            <Toile peinture={peinture(f.fixeGauche)} w={demi} h={hauteur} classe="demi demi-gauche" />
          )}
          <Toile peinture={peinture(f.fixeDroite)} w={demi} h={hauteur} classe="demi demi-droite" />
          <div
            ref={ombre}
            className={
              "feuille-ombre " +
              (f.sens === 1 ? "ombre-droite" : "ombre-gauche")
            }
            aria-hidden="true"
          />
          <Lamelles
            sens={f.sens}
            recto={peinture(f.recto)}
            verso={peinture(f.verso)}
            largeur={demi}
            hauteur={hauteur}
            lamelles={lamelles}
            voiles={voiles}
          />
        </>
      ) : planche === COUVERTURE ? (
        <div
          className="livre-ferme"
          style={{
            top: `${b * echelle}px`,
            width: `${trim.w * echelle}px`,
            height: `${trim.h * echelle}px`,
          }}
        >
          <Livre3D
            album={livre.album}
            formatMm={[trim.w, trim.h]}
            dosMm={livre.dosMm}
            pages={livre.pages}
            hauteurPx={trim.h * echelle}
            deFace={deFace}
            avant={
              avantRogne ? (
                <Toile peinture={avantRogne} w={trim.w * echelle} h={trim.h * echelle} classe="livre-avant" />
              ) : undefined
            }
          />
        </div>
      ) : (
        <Toile
          peinture={affiche ? { source: affiche.source, part: partDe(affiche.source, { x: 0, y: 0, w: 1, h: 1 }) } : null}
          w={largeur}
          h={hauteur}
          classe="demi demi-pleine"
        />
      )}
      {/* Les deux coins, décor seul : ils disent où le geste vit, ils ne le
          prennent pas. Le test de départ lit `coinTouche`, une seule fois,
          pour que la zone visible et la zone active soient la même chose. */}
      {!fermeAuRepos && (
        <>
          <div
            className={"coin coin-avant" + (coinSurvole === 1 ? " vif" : "")}
            style={{ width: pourcent(COIN.largeur), height: pourcent(COIN.hauteur) }}
            aria-hidden="true"
          />
          <div
            className={"coin coin-arriere" + (coinSurvole === -1 ? " vif" : "")}
            style={{ width: pourcent(COIN.largeur), height: pourcent(COIN.hauteur) }}
            aria-hidden="true"
          />
        </>
      )}
    </div>
  );
}

type Geste = {
  sens: Sens;
  id: number;
  t0: number;
  x0: number;
  y0: number;
  /** La feuille est montée et suit le doigt. Faux quand les images
   *  manquaient : le coin répond alors d'un seul coup. */
  anime: boolean;
  vu: { p: number; t: number };
  /** Tours par seconde, signée : c'est elle qui décide d'une chiquenaude. */
  vitesse: number;
};

function pourcent(f: number): string {
  return `${(f * 100).toFixed(3)}%`;
}

/** Une part d'un bitmap partagé, prête à peindre. */
type Peinture = { source: HTMLCanvasElement; part: Part };

/**
 * La feuille qui tourne : `LAMELLES` bandes, du pli vers le bord libre,
 * chacune enfant de la précédente. La première porte la classe `feuillet` et
 * la charnière au pli ; chacune porte son recto, son verso et son voile, et
 * les angles sont écrits par `appliquer`, hors de React.
 */
function Lamelles({
  sens,
  recto,
  verso,
  largeur,
  hauteur,
  lamelles,
  voiles,
}: {
  sens: Sens;
  recto: Peinture | null;
  verso: Peinture | null;
  /** La largeur de la page, en pixels CSS. */
  largeur: number;
  hauteur: number;
  lamelles: React.MutableRefObject<(HTMLDivElement | null)[]>;
  voiles: React.MutableRefObject<(HTMLDivElement | null)[]>;
}) {
  const parts = bandes(LAMELLES, sens);
  const w = largeur / LAMELLES;
  const bande = (p: Peinture | null, f: [number, number]): Peinture | null =>
    p ? { source: p.source, part: bandeDe(p.part, f) } : null;
  let enfant: React.ReactNode = null;
  for (let k = LAMELLES - 1; k >= 0; k--) {
    const premiere = k === 0;
    enfant = (
      <div
        key={k}
        ref={(el) => {
          lamelles.current[k] = el;
        }}
        className={
          (premiere ? "feuillet " : "lamelle ") + (sens === 1 ? "vers-avant" : "vers-arriere")
        }
        style={premiere ? { width: `${w}px` } : undefined}
        aria-hidden="true"
      >
        <div className="face recto">
          <Toile peinture={bande(recto, parts[k].recto)} w={w + 1} h={hauteur} classe="bande" />
        </div>
        <div className="face verso">
          <Toile peinture={bande(verso, parts[k].verso)} w={w + 1} h={hauteur} classe="bande" />
        </div>
        <div
          ref={(el) => {
            voiles.current[k] = el;
          }}
          className={premiere ? "lamelle-voile feuille-courbure" : "lamelle-voile"}
        />
        {enfant}
      </div>
    );
  }
  return <>{enfant}</>;
}

/**
 * Une part du bitmap partagé, peinte dans son propre canvas. Un `drawImage`
 * et rien d'autre : pas de second dessin du PDF, et la taille CSS est écrite
 * depuis la scène, pas déduite d'une mise à l'échelle.
 *
 * La peinture se fait avant l'affichage (`useLayoutEffect`) : au bout d'un
 * tour, la planche d'arrivée est à l'écran sur la trame même où la feuille
 * disparaît.
 */
function Toile({
  peinture,
  w,
  h,
  classe,
}: {
  peinture: Peinture | null;
  w: number;
  h: number;
  classe: string;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useLayoutEffect(() => {
    const c = canvas.current;
    if (!c || !peinture || peinture.part.sw <= 0 || peinture.part.sh <= 0) return;
    const { sx, sy, sw, sh } = peinture.part;
    c.width = sw;
    c.height = sh;
    c.getContext("2d")?.drawImage(peinture.source, sx, sy, sw, sh, 0, 0, sw, sh);
  }, [peinture?.source, peinture?.part.sx, peinture?.part.sy, peinture?.part.sw, peinture?.part.sh]);

  return (
    <canvas
      ref={canvas}
      className={classe}
      style={{ width: `${w}px`, height: `${h}px` }}
      aria-hidden="true"
    />
  );
}

export { Toile };
export type { Peinture };
