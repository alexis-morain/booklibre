// L'écran d'import d'un voyage, rendu état par état, et les deux cases du
// voyage dans le réglage des mentions. Rendu statique : aucune fenêtre.

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import { setLangue } from "./i18n";
import { LegendesView } from "./LegendesView";
import { BilanView } from "./BilanView";
import { ETAT_INITIAL, Etat, reduire } from "./voyage";
import { VoyageImportVue } from "./VoyageImport";
import type { Album } from "./album";

afterEach(() => setLangue("en"));

const LIEN = { id: 4242, dossier: "/Users/x/Pictures/Colophon/Voyage 4242" };
const rendre = (etat: Etat) =>
  renderToStaticMarkup(
    <VoyageImportVue etat={etat} onUrl={() => {}} onLancer={() => {}} onAnnuler={() => {}} onRetour={() => {}} />,
  ).replace(/&#x27;/g, "’");

const colle = reduire(ETAT_INITIAL, { type: "url", url: "https://www.polarsteps.com/q/4242-v", lien: LIEN });

describe("l'écran d'import d'un voyage", () => {
  it("à vide : le champ, la mention non affilié, le guide du zip, pas de bouton actif", () => {
    setLangue("fr");
    const h = rendre(ETAT_INITIAL);
    expect(h).toContain("Importer un voyage Polarsteps");
    expect(h).toContain("n’est pas affilié à Polarsteps");
    expect(h).toContain("Télécharger mes données");
    expect(h).toMatch(/<button[^>]*disabled[^>]*>Importer et composer/);
    expect(h).not.toContain("svg");
  });

  it("lien collé : le dossier proposé et le bouton actif", () => {
    setLangue("fr");
    const h = rendre(colle);
    expect(h).toContain("Voyage 4242");
    expect(h).not.toMatch(/<button[^>]*disabled[^>]*>Importer et composer/);
  });

  it("lien illisible : la phrase, sans dossier", () => {
    setLangue("fr");
    const h = rendre(reduire(ETAT_INITIAL, { type: "url", url: "coucou", lien: null }));
    expect(h).toContain("n’est pas celui d’un voyage Polarsteps");
    expect(h).not.toContain("<code>");
  });

  it("en cours : la progression chiffrée, l'annulation et la reprise dite", () => {
    setLangue("fr");
    const p = reduire(reduire(colle, { type: "lancer" }), { type: "progres", fait: 412, total: 1546 });
    const h = rendre(p);
    expect(h).toMatch(/Photos : 412 \/ 1(&nbsp;|\s| | )546/);
    expect(h).toContain("Annuler l’import");
    expect(h).toContain("reprendra plus tard là où il s’est arrêté");
    expect(h).toMatch(/<input[^>]*disabled/);
    // Pas de bouton d'import pendant l'import.
    expect(h).not.toContain("Importer et composer");
  });

  it("annulé : dit que ça reprend, propose de relancer", () => {
    setLangue("fr");
    const a = reduire(reduire(colle, { type: "lancer" }), { type: "echec", erreur: { genre: "annule", detail: "" } });
    const h = rendre(a);
    expect(h).toContain("reprend là où il s’est arrêté");
    expect(h).toContain("Relancer l’import");
  });

  it("format changé : la phrase et le remède", () => {
    setLangue("fr");
    const e = reduire(reduire(colle, { type: "lancer" }), { type: "echec", erreur: { genre: "format", detail: "d" } });
    const h = rendre(e);
    expect(h).toContain("Polarsteps a changé son format");
    expect(h).toContain("Passez par l’export de vos données Polarsteps");
  });

  it("existe en anglais", () => {
    setLangue("en");
    const h = rendre(colle);
    expect(h).toContain("Import a Polarsteps trip");
    expect(h).toContain("not affiliated with Polarsteps");
  });
});

describe("les cases du voyage dans les mentions", () => {
  const vue = (voyage: boolean) =>
    renderToStaticMarkup(
      <LegendesView
        reglage={{ contenu: "date_lieu", date: "jour", langue: "fr" }}
        enCours={false}
        onChoisir={() => {}}
        voyage={voyage}
        chiffres
        onClose={() => {}}
      />,
    );

  it("Météo et Le voyage en chiffres, pour un voyage seulement", () => {
    setLangue("fr");
    const avec = vue(true);
    expect(avec).toContain("Météo de l’étape");
    expect(avec).toContain("Le voyage en chiffres");
    expect(avec).toMatch(/<input[^>]*checked[^>]*type="checkbox"|<input[^>]*type="checkbox"[^>]*checked/);
    const sans = vue(false);
    expect(sans).not.toContain("Météo");
    expect(sans).not.toContain("en chiffres");
  });
});

describe("le bilan montre le voyage", () => {
  const album = { title: "Vietnam", trim_mm: { w: 210, h: 210 }, spreads: [] } as unknown as Album;
  const rendreBilan = (voyage: boolean) =>
    renderToStaticMarkup(
      <BilanView
        bilan={{
          photos_scanned: 1546,
          photos_kept: 1500,
          chapters: 93,
          voyage: voyage
            ? { titre: "t", etapes: 93, etapes_groupees: 2, recits_debordants: ["Sapa"], emojis_non_imprimes: 12, refus: null }
            : null,
        }}
        album={album}
        curation={[]}
        variantes={[]}
        choisie={null}
        voyageImporte={{
          dossier: "/d",
          titre: "t",
          bilan: { etapes: 93, etapes_sans_photo: 0, pays: 12, photos: 1546, videos_ignorees: 5, photos_sous_seuil: 1546, trace_presente: true },
        }}
        onChoisir={() => {}}
        onOpen={() => {}}
        onTri={() => {}}
      />,
    ).replace(/&#x27;/g, "’");

  it("nomme vidéos, émojis, récits et photos sous 250 ppi", () => {
    setLangue("fr");
    const h = rendreBilan(true);
    expect(h).toContain("5 vidéos ignorées");
    expect(h).toContain("12 émojis non imprimés");
    expect(h).toContain("Sapa");
    expect(h).toContain("1546 photos sont trop petites");
    expect(h).toContain("export de vos données Polarsteps (zip)");
  });

  it("un dossier ordinaire n'a pas de section voyage", () => {
    setLangue("fr");
    expect(rendreBilan(false)).not.toContain("bilan-voyage");
  });
});
