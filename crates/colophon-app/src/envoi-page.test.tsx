// L'écran Envoi rendu, sans navigateur : ce qu'une personne qui fait un
// album lit sans ouvrir un repli (A2). Les mots de l'imprimeur restent sous
// « Pour les imprimeurs » ; sans relais compilé, rien n'annonce la commande.

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type { Album, Spread } from "./album";
import type { Defaut, Printer, PrevolReport } from "./bridge";
import type { Relais } from "./commande";
import { EnvoiPage, EnvoiPageProps } from "./EnvoiView";
import { Porte } from "./envoi";
import { Lang, setLangue } from "./i18n";

afterEach(() => setLangue("en"));

const album = {
  version: 3,
  title: "Corse",
  root: "/p",
  trim_mm: { w: 210, h: 210 },
  bleed_mm: 3,
  spreads: [{ template: "solo", slots: [] } as unknown as Spread],
  cover: { title: "Corse" },
  colophon: { photos: 1 },
} as unknown as Album;

const base = {
  pdf_x: "x4",
  espace: "rgb",
  bleed_mm: { haut: 3, bas: 3, exterieur: 3, dos: 3 },
  debord_mm: 3,
  mors_mm: 5,
  safe_mm: 7,
  pages_simples: true,
  dos: { mode: "fourni" },
  pages_min: 24,
  pages_max: 800,
  pas_pagination: 2,
  min_ppi: 250,
  certitude: "provisoire",
} as const;

const PRINTERS: Printer[] = [
  {
    ...base,
    id: "cloudprinter",
    nom: "Cloudprinter",
    rempli_mm: 18,
    fichiers: "deux",
    reserves: ["leur zone sûre est une recommandation à fourchette, avant commande"],
    commande: { papier: "pageblock_150mcs", couverture: "cover_130mcg", finition: "cover_finish_matte" },
  },
  { ...base, id: "generique", nom: "Imprimeur local", rempli_mm: 0, fichiers: "un", reserves: [], commande: null },
];

// Un verdict vert, et l'avertissement qu'un album légendé porte toujours.
const zoneSure: Defaut = {
  regle: "zone_sure",
  bloquant: false,
  planche: 3,
  cause: "la légende tombe à 7,0 mm de la coupe, sous la zone sûre de 7,0 mm",
  remede: "rentrez-la",
  phrase: { cause: "zone_sure.legende", remede: "zone_sure.rentrer", valeurs: { mm: "7,0" } },
};

const REPORT: PrevolReport = {
  album: "Corse",
  profil: "cloudprinter",
  ok: true,
  bloquants: 0,
  avertissements: 1,
  fiche: {
    imprimeur: "Cloudprinter",
    format_page_mm: [210, 210],
    produit: "photobook_cw_s210_s_fc",
    planches: 48,
    pages_interieur: 96,
    pages_fichier: 96,
    fond_perdu_mm: { haut: 3, bas: 3, exterieur: 3, dos: 3 },
    zone_sure_mm: 7,
    feuille_couverture_mm: [490.48, 258],
    espace: "rgb",
    output_intent: "sRGB IEC61966-2.1",
    conformite: "x4",
    fichiers: "deux",
    dos_mm: 12.48,
    grammage_g_m2: 150,
    resolution_cible_dpi: 300,
  },
  reserves: ["leur zone sûre est une recommandation à fourchette"],
  notes: ["le PDF se déclare PDF/X-4 en RVB, intention de sortie sRGB"],
  defauts: [zoneSure],
};

const FRANCE: Relais = { etat: "joignable", pays: [{ code: "FR", nom: "France", require_state: false }] };

const rendu = (lang: Lang, relais: Relais | null, porte: Porte | null, profil = "cloudprinter", extra: Partial<EnvoiPageProps> = {}) => {
  setLangue(lang);
  const props: EnvoiPageProps = {
    album,
    printers: PRINTERS,
    profil,
    onProfil: () => {},
    onJump: () => {},
    onCouverture: () => {},
    onFormat: () => {},
    onVoir: () => {},
    onRemplacer: () => {},
    onRetirer: () => {},
    reserveVide: false,
    onExport: () => {},
    onPrepare: () => {},
    preparation: null,
    exporting: false,
    exporte: false,
    dirty: false,
    colophonPossible: true,
    colophonActif: true,
    onColophon: () => {},
    gardeActif: true,
    onGarde: () => {},
    policeManquante: false,
    absents: [],
    onFeuilleter: () => {},
    onFond: () => {},
    progresExport: null,
    report: REPORT,
    running: false,
    error: null,
    relais,
    sonde: false,
    sonder: () => {},
    porte,
    onPorte: () => {},
    prixMin: 4600,
    onGrille: () => {},
    prepareeAvant: null,
    ...extra,
  };
  return renderToStaticMarkup(<EnvoiPage {...props} />);
};

/** Le texte qu'on lit sans rien ouvrir : les replis retirés, puis les balises
 *  (les attributs avec elles : un `title` ne se lit qu'au survol). */
const visible = (html: string) =>
  html
    .replace(/<details[\s\S]*?<\/details>/g, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/&#x27;|&#39;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/\s+/g, " ");

const LISTE_NOIRE: Record<Lang, RegExp[]> = {
  fr: [/fond perdu/i, /zone s[ûu]re/i, /PDF\/X/i, /sRGB/i, /\bRVB\b/, /CMJN/, /\bppi\b/i, /\bdpi\b/i, /output intent/i, /imposition/i, /pr[ée]vol/i, /profil/i],
  en: [/bleed/i, /safe (zone|area)/i, /PDF\/X/i, /sRGB/i, /\bRGB\b/, /CMYK/, /\bppi\b/i, /\bdpi\b/i, /output intent/i, /imposition/i, /preflight/i, /profile/i],
};

describe("Envoi, pour quelqu'un qui fait un album", () => {
  for (const lang of ["fr", "en"] as const) {
    for (const porte of ["colophon", "pdf"] as const) {
      it(`ne montre aucun mot d'imprimeur hors d'un repli (${lang}, porte ${porte})`, () => {
        const html = rendu(lang, FRANCE, porte);
        const texte = visible(html);
        for (const mot of LISTE_NOIRE[lang]) expect(texte, String(mot)).not.toMatch(mot);
      });
    }

    it(`garde la fiche sous « Pour les imprimeurs », fermé, et sous la porte PDF seulement (${lang})`, () => {
      const pdf = rendu(lang, FRANCE, "pdf");
      expect(pdf).toMatch(/<details class="envoi-imprimeurs"><summary>/);
      expect(pdf).toContain("490,5");
      expect(rendu(lang, FRANCE, "colophon")).not.toContain("envoi-imprimeurs");
    });
  }

  it("dit le verdict vert en une ligne, et le livre en trois", () => {
    const texte = visible(rendu("fr", FRANCE, "colophon"));
    expect(texte).toContain("Votre livre est prêt.");
    expect(texte).toContain("21 × 21 cm");
    expect(texte).toContain("96 pages");
    expect(texte).toContain("Papier 150 g, couverture rigide mate");
    expect(texte).toContain("Feuilleter");
  });

  it("offre deux portes, une seule choisie, et le prix « à partir de » quand il est connu", () => {
    const html = rendu("fr", FRANCE, "colophon");
    expect(html).toContain('role="radiogroup"');
    expect(html.match(/role="radio"/g)).toHaveLength(2);
    expect(html.match(/aria-checked="true"/g)).toHaveLength(1);
    const texte = visible(html);
    expect(texte).toContain("Télécharger le PDF");
    expect(texte).toContain("Faire imprimer par Colophon");
    expect(texte).toContain("livré chez vous en France");
    expect(texte).toContain("Imprimé et relié par notre partenaire Cloudprinter.");
    expect(texte).toContain("à partir de 46,00 €");
    expect(visible(rendu("fr", FRANCE, "colophon", "cloudprinter", { prixMin: null }))).not.toContain("à partir de");
  });

  it("range les pages de début et de fin dans un repli", () => {
    const html = rendu("fr", FRANCE, "pdf");
    expect(html).toMatch(/<details class="envoi-colophon"><summary>Pages de début et de fin<\/summary>/);
    expect(visible(html)).not.toContain("Imprimer la page de garde");
  });

  it("met les défauts au-dessus des portes, avec leurs gestes", () => {
    const resolution: Defaut = {
      regle: "couverture_vide",
      bloquant: true,
      cause: "la couverture n'a pas de photo",
      remede: "choisissez-en une",
    };
    const html = rendu("fr", FRANCE, "colophon", "cloudprinter", {
      report: { ...REPORT, ok: false, bloquants: 1, defauts: [resolution] },
    });
    expect(html.indexOf("envoi-defauts")).toBeGreaterThan(-1);
    expect(html.indexOf("envoi-defauts")).toBeLessThan(html.indexOf('role="radiogroup"'));
    expect(visible(html)).toContain("Choisir une autre photo");
  });
});

describe("Comment ça se passe", () => {
  // Signaler ouvre une issue GitHub publique : quelqu'un y citerait sa
  // commande. L'encadré n'y mène donc nulle part (A4, décision 10).
  for (const lang of ["fr", "en"] as const) {
    it(`ne mène jamais à Signaler (${lang})`, () => {
      const html = rendu(lang, FRANCE, "colophon");
      const encadre = html.match(/<aside class="rassurance"[\s\S]*?<\/aside>/)?.[0];
      expect(encadre).toBeDefined();
      expect(encadre).not.toMatch(/<button|<a /);
      expect(encadre).not.toMatch(/Signaler|Report a problem|Une question|A question/);
    });
  }
});

describe("le livre et son fond", () => {
  it("montre le livre en volume, et les huit pastilles sous lui", () => {
    setLangue("fr");
    const html = rendu("fr", FRANCE, "colophon");
    expect(html).toContain("livre3d");
    expect(html).toContain("La couleur du fond de la couverture et du dos.");
    expect(html.match(/class="fond-pastille[^"]*"/g)?.length).toBeGreaterThanOrEqual(3);
    expect(visible(html)).toContain("Autre…");
  });
});

describe("Envoi sans relais compilé", () => {
  for (const lang of ["fr", "en"] as const) {
    for (const relais of [null, { etat: "absent" } as Relais]) {
      it(`n'annonce aucune commande (${lang}, ${relais === null ? "relais pas encore lu" : "aucun relais"})`, () => {
        const html = rendu(lang, relais, null);
        expect(html).not.toContain('role="radiogroup"');
        expect(html).not.toContain("Colophon s");
        expect(html).not.toContain("Cloudprinter.");
        expect(html).not.toMatch(/Faire imprimer|Have it printed|Have Colophon|Stripe|à partir de|from €/);
        expect(visible(html)).not.toMatch(lang === "fr" ? /command/i : /\border/i);
        expect(visible(html)).toContain(lang === "fr" ? "Télécharger le PDF" : "Download the PDF");
      });
    }
  }
});
