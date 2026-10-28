// La barre latérale : l'album (sa couverture, son nom, son format), les
// quatre vues, puis les chapitres en arbre, et en pied ce qui change le livre
// ou en ouvre un autre. Elle remplace les quatre segments du haut : où l'on
// est se lit à gauche, ce qu'on y fait se lit en haut. Aucune commande ne
// naît ici, chaque bouton appelle ce que le menu appelle déjà.

import { Album } from "./album";
import { Chapitre } from "./chapitres";
import { t } from "./i18n";
import {
  IcoEnvoi,
  IcoLivre,
  IcoOuvrir,
  IcoPlanches,
  IcoPlus,
  IcoPolice,
  IcoTri,
} from "./icons";
import { MiniCouverture } from "./PlanchesView";

type Vue = "livre" | "tri" | "planches" | "envoi";

/** « 21 × 21 cm », le format tel qu'Envoi le dit. */
export function formatCm(trim: { w: number; h: number }, virgule: boolean): string {
  const cm = (mm: number) => {
    const n = Math.round(mm) / 10;
    return virgule ? String(n).replace(".", ",") : String(n);
  };
  return `${cm(trim.w)} × ${cm(trim.h)} cm`;
}

export function Lateral({
  album,
  view,
  triCount,
  index,
  onView,
  onCouverture,
  onChapitre,
  chapitres,
  courant,
  onPolice,
  onNouveau,
  onOuvrir,
  virgule,
}: {
  album: Album;
  view: Vue;
  triCount: number;
  /** La planche affichée ; -1 est la couverture. */
  index: number;
  onView: (v: Vue) => void;
  onCouverture: () => void;
  onChapitre: (debut: number) => void;
  chapitres: Chapitre[];
  /** Le chapitre de la planche affichée. */
  courant: Chapitre | null;
  onPolice?: () => void;
  onNouveau: () => void;
  /** Au harnais navigateur seulement : le shell passe par le menu. */
  onOuvrir?: () => void;
  virgule: boolean;
}) {
  const n = album.spreads.length;
  const sousTitre = t(n === 1 ? "lateral.format.une" : "lateral.format", {
    format: formatCm(album.trim_mm, virgule),
    n,
  });
  const vues: { v: Vue; nom: string; icone: React.ReactNode; compte?: number; touche: string; titre?: string }[] = [
    { v: "livre", nom: t("bar.livre"), icone: <IcoLivre />, touche: "⌘1" },
    { v: "tri", nom: t("bar.tri"), icone: <IcoTri />, compte: triCount, touche: "⌘2" },
    { v: "planches", nom: t("bar.planches"), icone: <IcoPlanches />, compte: n, touche: "⌘3" },
    { v: "envoi", nom: t("bar.envoi"), icone: <IcoEnvoi />, touche: "⌘4", titre: t("bar.envoi.titre") },
  ];
  const surCouverture = view !== "tri" && view !== "envoi" && index === -1;
  return (
    <aside className="lateral" aria-label={t("lateral.aria")}>
      <div className="lateral-haut" />
      <button className="lateral-album" onClick={onCouverture} title={t("lateral.album")}>
        <span className="lateral-couv">
          <MiniCouverture album={album} />
        </span>
        <span className="lateral-album-nom">
          {album.title}
          <small>{sousTitre}</small>
        </span>
      </button>

      <nav className="lateral-nav" aria-label={t("lateral.aria")}>
        {vues.map(({ v, nom, icone, compte, touche, titre }) => (
          <button
            key={v}
            className={"lateral-item" + (view === v ? " actif" : "")}
            aria-current={view === v ? "page" : undefined}
            aria-keyshortcuts={`Meta+${touche.slice(1)}`}
            title={titre ?? touche}
            onClick={() => onView(v)}
          >
            {icone}
            <span className="lateral-nom">{nom}</span>
            {compte !== undefined && <span className="lateral-n typed">{compte}</span>}
            <span className="lateral-touche" aria-hidden="true">
              {touche}
            </span>
          </button>
        ))}
      </nav>

      <p className="lateral-titre">{t("lateral.chapitres")}</p>
      <nav className="lateral-nav lateral-chapitres" aria-label={t("lateral.chapitres")}>
        <button
          className={"lateral-item" + (surCouverture ? " actif" : "")}
          aria-current={surCouverture ? "location" : undefined}
          onClick={onCouverture}
        >
          <span className="lateral-nom">{t("table.couverture")}</span>
        </button>
        {chapitres.map((c) => {
          const ici = courant?.debut === c.debut && index >= 0 && view !== "tri" && view !== "envoi";
          return (
            <button
              key={c.debut}
              className={"lateral-item lateral-chapitre" + (ici ? " actif" : "")}
              aria-current={ici ? "location" : undefined}
              title={t("lateral.chapitre.titre", { titre: c.titre, n: c.planches, debut: c.debut + 1 })}
              onClick={() => onChapitre(c.debut)}
            >
              <span className="lateral-repere" aria-hidden="true" />
              <span className="lateral-nom">{c.titre}</span>
              <span className="lateral-n typed">{c.planches}</span>
            </button>
          );
        })}
      </nav>

      <nav className="lateral-pied">
        {onPolice && (
          <button className="lateral-item" onClick={onPolice} title={t("bar.format.titre")}>
            <IcoPolice />
            <span className="lateral-nom">{t("bar.format")}</span>
          </button>
        )}
        <button className="lateral-item" onClick={onNouveau} title={t("bar.nouveau.titre")}>
          <IcoPlus />
          <span className="lateral-nom">{t("lateral.nouvel.album")}</span>
        </button>
        {onOuvrir && (
          <button className="lateral-item" onClick={onOuvrir} title="⌘O">
            <IcoOuvrir />
            <span className="lateral-nom">{t("lateral.ouvrir")}</span>
          </button>
        )}
      </nav>
    </aside>
  );
}
