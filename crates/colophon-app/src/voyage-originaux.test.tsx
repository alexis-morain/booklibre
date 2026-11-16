// « Photos d'origine » (P-s8), vu de l'écran : la machine d'états, les lignes
// du bilan, et le bloc rendu état par état, sans fenêtre.

import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import { setLangue, t } from "./i18n";
import { OriginauxVue } from "./VoyageOriginaux";
import {
  cleRaison,
  Etat,
  ETAT_INITIAL,
  LIGNES_VISIBLES,
  lignesOriginaux,
  petitesRestantes,
  RapportOriginaux,
  RaisonRefus,
  reduire,
  RefusOriginal,
} from "./voyage-originaux";

afterEach(() => setLangue("en"));

const refus = (n: number, raison: RaisonRefus = "ambigu"): RefusOriginal[] =>
  Array.from({ length: n }, (_, i) => ({ fichier: `e001-p${i}-aaaa.jpg`, etape: "Hanoï", raison }));

const rapport = (p: Partial<RapportOriginaux> = {}): RapportOriginaux => ({
  remplacees: 1412,
  deja: 0,
  refus: refus(134),
  illisibles: 0,
  heic_illisibles: 0,
  reecrites: 0,
  sous_seuil_avant: 1546,
  sous_seuil_apres: 134,
  revenues_possibles: 0,
  ...p,
});

const termine = (r: RapportOriginaux, deja = 0): Etat =>
  reduire(reduire({ ...ETAT_INITIAL, remplacees: deja }, { type: "lancer" }), { type: "fait", rapport: r, dossier: "/Users/x/Originaux" });

const rendre = (etat: Etat) =>
  renderToStaticMarkup(<OriginauxVue etat={etat} onChoisir={() => {}} onAnnuler={() => {}} onRetablir={() => {}} />).replace(/&#x27;/g, "’").replace(/\u00a0/g, " ");

describe("la machine d'états", () => {
  it("un passage va de repos à cours puis au bilan, et les chiffres s'additionnent", () => {
    const e = termine(rapport());
    expect(e.phase).toBe("repos");
    expect(e.remplacees).toBe(1412);
    expect(e.refus).toHaveLength(134);
    // Un second passage s'ajoute au premier : le total est celui de l'album.
    const e2 = termine(rapport({ remplacees: 20, refus: refus(114) }), 1412);
    expect(e2.remplacees).toBe(1432);
    expect(e2.refus).toHaveLength(114);
  });

  it("la progression ne vaut qu'en cours, l'état de l'album jamais par-dessus un passage", () => {
    expect(reduire(ETAT_INITIAL, { type: "progres", fait: 3, total: 9 })).toBe(ETAT_INITIAL);
    const cours = reduire(ETAT_INITIAL, { type: "lancer" });
    expect(reduire(cours, { type: "progres", fait: 3, total: 9 })).toMatchObject({ fait: 3, total: 9 });
    expect(reduire(cours, { type: "etat", etat: { voyage: true, remplacees: 5, refus: [], dossier: "" } })).toBe(cours);
    expect(reduire(cours, { type: "lancer" })).toBe(cours);
  });

  it("un échec et une annulation se distinguent, et rendre les vignettes remet tout à zéro", () => {
    const cours = reduire(ETAT_INITIAL, { type: "lancer" });
    expect(reduire(cours, { type: "echec", detail: "disque", annule: false }).phase).toBe("echec");
    expect(reduire(cours, { type: "echec", detail: "annulé", annule: true }).phase).toBe("annule");
    const rendu = reduire(termine(rapport()), { type: "rendues" });
    expect(rendu).toMatchObject({ remplacees: 0, refus: [], rendues: true, rapport: null });
  });
});

describe("les lignes du bilan", () => {
  it("disent le compte et les sans-original, au singulier comme au pluriel", () => {
    setLangue("fr");
    const l = lignesOriginaux(termine(rapport()), "fr").map((x) => t(x.cle, x.params).replace(/\u00a0/g, " "));
    expect(l).toContain("1 412 photos remplacées par leur original.");
    expect(l).toContain("134 sans original sûr.");
    setLangue("fr");
    const un = lignesOriginaux(termine(rapport({ remplacees: 1, refus: refus(1) })), "fr").map((x) => t(x.cle, x.params));
    expect(un).toEqual(["1 photo remplacée par son original.", "1 sans original sûr."]);
  });

  it("dit qu'aucune photo n'a été remplacée, au lieu de se taire", () => {
    setLangue("fr");
    const l = lignesOriginaux(termine(rapport({ remplacees: 0, refus: refus(3) })), "fr").map((x) => t(x.cle, x.params));
    expect(l[0]).toContain("Aucune photo remplacée");
  });

  it("annonce les photos de la réserve qui peuvent revenir, et les fichiers illisibles", () => {
    setLangue("fr");
    const l = lignesOriginaux(termine(rapport({ revenues_possibles: 659, illisibles: 2 })), "fr").map((x) => t(x.cle, x.params));
    // Le nom du bouton entre guillemets, jamais une majuscule au milieu d'une phrase.
    expect(l.some((x) => x.startsWith("659 photos de la réserve") && x.includes(" : « Recomposer l’album » peut"))).toBe(true);
    expect(l).toContain("2 fichiers du dossier n’ont pas pu être lus.");
    setLangue("en");
    const en = lignesOriginaux(termine(rapport({ revenues_possibles: 1 })), "en").map((x) => t(x.cle, x.params));
    expect(en.join(" ")).toContain("original: “Recompose the album” can bring it back");
  });

  it("dit les HEIC qu'aucun décodeur du système ne lit", () => {
    setLangue("fr");
    const fr = (n: number) => lignesOriginaux(termine(rapport({ heic_illisibles: n })), "fr").map((x) => t(x.cle, x.params));
    expect(fr(3)).toContain("HEIC non lisible sur ce système : 3 fichiers du dossier sont ignorés.");
    expect(fr(1)).toContain("HEIC non lisible sur ce système : 1 fichier du dossier est ignoré.");
    expect(fr(0).join(" ")).not.toContain("HEIC");
  });

  it("le compte de la réserve s'additionne d'un passage à l'autre et se vide quand on rend les vignettes", () => {
    const e = termine(rapport({ revenues_possibles: 659 }));
    expect(e.revenues).toBe(659);
    expect(termine(rapport({ revenues_possibles: 10 }), 1412).revenues).toBe(10);
    const deux = reduire(reduire(e, { type: "lancer" }), { type: "fait", rapport: rapport({ revenues_possibles: 10 }), dossier: "/x" });
    expect(deux.revenues).toBe(669);
    expect(reduire(deux, { type: "rendues" }).revenues).toBe(0);
  });

  it("le bilan de l'import ne garde pas, dans la réserve, les photos qui ont leur original", () => {
    // 659 trop petites au bilan, 659 qui ont maintenant leur original : plus aucune phrase « trop petites ».
    expect(petitesRestantes(659, 659)).toBe(0);
    expect(petitesRestantes(659, 600)).toBe(59);
    expect(petitesRestantes(659, 0)).toBe(659);
    // Un compte de réserve plus large que le bilan ne passe pas sous zéro.
    expect(petitesRestantes(10, 12)).toBe(0);
  });

  it("n'écrit rien d'un album sans remplacement ni passage", () => {
    expect(lignesOriginaux(ETAT_INITIAL, "fr")).toEqual([]);
  });

  it("chaque raison a sa phrase, dans les deux langues", () => {
    const raisons: RaisonRefus[] = ["sans_original", "ambigu", "conflit", "pas_plus_grand", "format", "illisible"];
    for (const lang of ["fr", "en"] as const) {
      setLangue(lang);
      const phrases = raisons.map((r) => t(cleRaison(r)));
      expect(new Set(phrases).size).toBe(raisons.length);
    }
  });
});

describe("le bloc", () => {
  it("au départ : le principe, rien n'est effacé, un seul bouton", () => {
    setLangue("fr");
    const h = rendre(ETAT_INITIAL);
    expect(h).toContain("Photos d’origine");
    expect(h).toContain("Aucune vignette n’est effacée");
    expect(h).toContain("Choisir le dossier de mes originaux…");
    expect(h).not.toContain("Rendre les vignettes");
  });

  it("après un passage : le compte, la liste consultable, rendre les vignettes", () => {
    setLangue("fr");
    const h = rendre(termine(rapport()));
    expect(h).toContain("1 412 photos remplacées par leur original.");
    expect(h).toContain("134 sans original sûr.");
    expect(h).toContain("<details");
    expect(h).toContain("plusieurs photos se ressemblent");
    expect(h).toContain("Rendre les vignettes");
    expect(h).toContain("Choisir un autre dossier…");
    expect(h).toContain("/Users/x/Originaux");
  });

  it("la liste se borne et dit le reste en chiffre", () => {
    setLangue("fr");
    const h = rendre(termine(rapport({ refus: refus(LIGNES_VISIBLES + 37), remplacees: 3 })));
    expect((h.match(/<li><span class="originaux-etape"/g) ?? []).length).toBe(LIGNES_VISIBLES);
    expect(h).toContain("… et 37 autres.");
  });

  it("en cours : la progression et Annuler, ni choisir ni rendre", () => {
    setLangue("fr");
    const h = rendre({ ...reduire(ETAT_INITIAL, { type: "lancer" }), fait: 412, total: 1546 });
    expect(h).toContain("Lecture de vos photos : 412 / 1 546");
    expect(h).toContain(">Annuler<");
    expect(h).not.toContain("Choisir");
  });

  it("une erreur et une annulation se disent avec leur remède", () => {
    setLangue("fr");
    const cours = reduire(ETAT_INITIAL, { type: "lancer" });
    expect(rendre(reduire(cours, { type: "echec", detail: "disque plein", annule: false }))).toContain("Le remplacement n’a pas abouti : disque plein");
    expect(rendre(reduire(cours, { type: "echec", detail: "annulé", annule: true }))).toContain("reprend là où il s’est arrêté");
  });

  it("existe en anglais", () => {
    setLangue("en");
    const h = rendre(termine(rapport()));
    expect(h).toContain("Original photos");
    expect(h).toContain("1,412 photos replaced by their original.");
    expect(h).toContain("Put the thumbnails back");
  });
});
