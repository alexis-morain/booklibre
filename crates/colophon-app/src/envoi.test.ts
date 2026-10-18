// L'écran Envoi, sans DOM : quel imprimeur il nomme, ce que fait son bouton,
// et la phrase qu'il dit après avoir préparé le dossier. Vitest tourne sans
// `navigator`, langue anglaise par défaut : chaque cas pose la sienne.

import { afterEach, describe, expect, it } from "vitest";
import type { Album } from "./album";
import type { Defaut, Preparation, Printer, PrevolReport } from "./bridge";
import { FR, EN, setLangue } from "./i18n";
import codes from "./moteur.codes.json";
import {
  DESTINATIONS,
  PROFIL_ENVOI,
  PROFIL_LIBRE,
  actionEnvoi,
  actionsDefaut,
  destinationDe,
  imprimeurAffiche,
  libelleDestination,
  phrasePreparation,
  phraseSimple,
  ppiDe,
} from "./envoi";

afterEach(() => setLangue("en"));

const profil = (id: string, nom: string, fichiers: "un" | "deux"): Printer => ({
  id,
  nom,
  pdf_x: "x4",
  espace: "rgb",
  bleed_mm: { haut: 3, bas: 3, exterieur: 3, dos: 3 },
  rempli_mm: 0,
  debord_mm: 0,
  mors_mm: 0,
  safe_mm: 7,
  fichiers,
  pages_simples: true,
  dos: { mode: "fourni" },
  pages_min: 24,
  pages_max: 800,
  pas_pagination: 2,
  min_ppi: 250,
  certitude: "confirme",
  reserves: [],
});

/** Les quatre profils du moteur, dans son ordre. */
const PROFILS: Printer[] = [
  profil("cloudprinter", "Cloudprinter", "deux"),
  profil("prodigi", "Prodigi", "un"),
  profil("lulu", "Lulu", "deux"),
  profil("generique", "Imprimeur local (PDF sans contrainte)", "un"),
];

describe("imprimeurAffiche", () => {
  it("ne nomme que Cloudprinter, ou le PDF sans contrainte quand on l'a choisi", () => {
    expect(PROFIL_ENVOI).toBe("cloudprinter");
    expect(PROFIL_LIBRE).toBe("generique");
    expect(imprimeurAffiche(PROFILS, "cloudprinter")?.id).toBe("cloudprinter");
    expect(imprimeurAffiche(PROFILS, "generique")?.id).toBe("generique");
  });

  it("ne montre jamais Prodigi ni Lulu, qui restent au moteur et à la CLI", () => {
    // Un profil hérité d'ailleurs retombe sur l'imprimeur de l'écran.
    expect(imprimeurAffiche(PROFILS, "prodigi")?.id).toBe("cloudprinter");
    expect(imprimeurAffiche(PROFILS, "lulu")?.id).toBe("cloudprinter");
  });

  it("attend la liste plutôt que d'inventer un imprimeur", () => {
    expect(imprimeurAffiche(null, "cloudprinter")).toBeNull();
  });
});

describe("actionEnvoi", () => {
  it("prépare un dossier chez qui relie deux fichiers", () => {
    setLangue("fr");
    const a = actionEnvoi(PROFILS[0]);
    expect(a.prepare).toBe(true);
    expect(a.texte).toBe("Préparer pour Cloudprinter");
  });

  it("enregistre un PDF, comme avant, sous le profil sans contrainte", () => {
    setLangue("fr");
    const a = actionEnvoi(PROFILS[3]);
    expect(a.prepare).toBe(false);
    expect(a.texte).toBe("Enregistrer le PDF d’impression");
  });

  it("devient « Faire imprimer » chez Cloudprinter quand le relais répond", () => {
    setLangue("fr");
    const a = actionEnvoi(PROFILS[0], true);
    expect(a).toMatchObject({ prepare: true, commande: true, texte: "Faire imprimer" });
    setLangue("en");
    expect(actionEnvoi(PROFILS[0], true).texte).toBe("Have it printed");
  });

  it("reste « Préparer » sans relais joignable, et ne commande jamais ailleurs que chez Cloudprinter", () => {
    setLangue("fr");
    expect(actionEnvoi(PROFILS[0], false)).toMatchObject({
      prepare: true,
      commande: false,
      texte: "Préparer pour Cloudprinter",
    });
    expect(actionEnvoi(PROFILS[0])).toMatchObject({ commande: false });
    for (const p of PROFILS.slice(1)) expect(actionEnvoi(p, true).commande, p.id).toBe(false);
  });
});

const rapport = (bloquants: number): PrevolReport =>
  ({
    album: "Corse",
    profil: "cloudprinter",
    ok: bloquants === 0,
    bloquants,
    avertissements: 0,
    defauts: [],
  }) as unknown as PrevolReport;

const preparation = (bloquants: number): Preparation => ({
  dossier: "Corse – Cloudprinter",
  fichiers: ["album-print.pdf", "album-cover.pdf", "export.json", "fiche.txt"],
  rapport: rapport(bloquants),
});

describe("phrasePreparation", () => {
  it("dit en une phrase que le dossier est prêt, et lequel", () => {
    setLangue("fr");
    expect(phrasePreparation(preparation(0))).toBe(
      "Dossier « Corse – Cloudprinter » prêt : les deux PDF relus, rien ne s’oppose à la commande.",
    );
  });

  it("dit ce que le contrôle relu a trouvé, au singulier et au pluriel", () => {
    setLangue("fr");
    expect(phrasePreparation(preparation(1))).toContain("un défaut");
    expect(phrasePreparation(preparation(3))).toContain("3 défauts");
    setLangue("en");
    expect(phrasePreparation(preparation(3))).toContain("3 defects");
  });
});

describe("actionsDefaut", () => {
  // Planche 10, case 1 : IMG_2217, la photo que le prévol nomme.
  const album = {
    spreads: Array.from({ length: 12 }, (_, i) => ({
      template: "g",
      slots:
        i === 9
          ? [
              { src: "IMG_2216.jpg", focal: [0.5, 0.5] },
              { src: "IMG_2217.jpg", focal: [0.5, 0.5] },
            ]
          : [],
    })),
  } as unknown as Album;
  const resolution: Defaut = {
    regle: "resolution",
    bloquant: true,
    planche: 10,
    case: 1,
    src: "IMG_2217.jpg",
    cause: "IMG_2217.jpg imprimerait à 130 ppi dans cette case, Cloudprinter exige 250",
    remede: "mettez-la dans une case plus petite, remplacez-la, ou retirez-la",
  };

  it("voir, remplacer et retirer quand la réserve a des photos", () => {
    expect(actionsDefaut(resolution, album, false)).toEqual(["voir", "remplacer", "retirer"]);
  });

  it("retirer reste quand la réserve est vide", () => {
    expect(actionsDefaut(resolution, album, true)).toEqual(["voir", "retirer"]);
  });

  it("n'invente rien pour un défaut sans case", () => {
    const { case: _, ...sansCase } = resolution;
    expect(actionsDefaut(sansCase, album, false)).toEqual([]);
  });

  it("ne donne aucune action neuve à une autre règle", () => {
    const pli: Defaut = { ...resolution, regle: "objet_pli" };
    expect(actionsDefaut(pli, album, false)).toEqual([]);
  });

  it("se tait quand la case ne porte plus la photo du rapport", () => {
    // Le rapport lit le disque : après une édition non enregistrée, la case
    // 1 peut porter une autre photo, et retirer viserait la mauvaise.
    expect(actionsDefaut({ ...resolution, case: 0 }, album, false)).toEqual([]);
  });
});

describe("ppiDe", () => {
  it("lit la résolution dans la cause du moteur", () => {
    const d = {
      regle: "resolution",
      bloquant: true,
      cause: "IMG_2217.jpg imprimerait à 130 ppi dans cette case, Cloudprinter exige 250",
      remede: "",
    };
    expect(ppiDe(d)).toBe(130);
    expect(ppiDe({ ...d, cause: "autre chose" })).toBeNull();
  });
});

const defaut = (regle: string, extra: Partial<Defaut> = {}): Defaut => ({
  regle,
  bloquant: true,
  cause: "la cause du moteur",
  remede: "le remède du moteur",
  ...extra,
});

describe("une destination par règle", () => {
  it("chaque règle du moteur mène quelque part, et la table n'en invente aucune", () => {
    expect(Object.keys(DESTINATIONS).sort()).toEqual([...codes.regles].sort());
  });

  it("une planche mène à la planche, la couverture à la couverture", () => {
    expect(destinationDe(defaut("resolution", { planche: 4, case: 1 }))).toBe("planche");
    expect(destinationDe(defaut("objet_pli", { planche: 2 }))).toBe("planche");
    for (const r of ["couverture_resolution", "couverture_vide", "dos", "dos_nu"]) {
      expect(destinationDe(defaut(r)), r).toBe("couverture");
    }
  });

  it("le format et la pagination mènent au panneau Format", () => {
    expect(destinationDe(defaut("format_sans_produit"))).toBe("format");
    expect(destinationDe(defaut("pagination"))).toBe("format");
  });

  it("une page à décocher se décoche ici, plus bas dans Envoi", () => {
    const d = defaut("pagination", {
      phrase: { cause: "pagination.bornes", remede: "pagination.decocher_colophon" },
    });
    expect(destinationDe(d)).toBe("ici");
  });

  it("les fichiers se refont en préparant à nouveau", () => {
    for (const r of ["fichier_couverture", "fichier_interieur", "fichier_profil", "fichier_perime", "conformite", "espace"]) {
      expect(destinationDe(defaut(r)), r).toBe("preparer");
    }
  });

  it("une règle d'un moteur plus récent n'a pas de destination inventée", () => {
    expect(destinationDe(defaut("regle_future"))).toBeNull();
  });

  it("une ligne sans planche se nomme par sa destination, plus jamais « L'album »", () => {
    setLangue("fr");
    expect(libelleDestination(defaut("couverture_resolution"))).toBe("Couverture");
    expect(libelleDestination(defaut("format_sans_produit"))).toBe("Recomposer");
    expect(libelleDestination(defaut("fichier_couverture"))).toBe("Fichiers");
    expect(libelleDestination(defaut("resolution", { planche: 4 }))).toBe("Planche 4");
    // Une règle de planche nomme toujours la sienne.
    for (const r of codes.regles) {
      const d = defaut(r, DESTINATIONS[r] === "planche" ? { planche: 1 } : {});
      expect(libelleDestination(d), r).not.toBe("L’album");
    }
    setLangue("en");
    expect(libelleDestination(defaut("couverture_resolution"))).toBe("Cover");
  });
});

describe("la phrase simple", () => {
  it("chaque règle a la sienne, en français et en anglais, sans trou à remplir", () => {
    for (const r of codes.regles) {
      const cle = `envoi.simple.${r}`;
      expect(FR, cle).toHaveProperty([cle]);
      expect(EN, cle).toHaveProperty([cle]);
      expect((FR as Record<string, string>)[cle]).not.toMatch(/\{/);
    }
  });

  it("dit le geste, et laisse les ppi, les millimètres et les fichiers aux détails", () => {
    setLangue("fr");
    const d = defaut("couverture_resolution", {
      src: "IMG_1148.heic",
      cause: "IMG_1148.heic imprimerait à 221 ppi sur la couverture de 329 × 348 mm, Cloudprinter exige 250",
    });
    expect(phraseSimple(d)).toBe("Cette photo est trop petite pour la couverture : choisissez-en une autre.");
    for (const r of codes.regles) {
      const p = phraseSimple(defaut(r))!;
      expect(p, r).not.toMatch(/\d|ppi|mm\b|\.pdf|--/);
    }
    setLangue("en");
    expect(phraseSimple(d)).toBe("This photo is too small for the cover: choose another one.");
  });

  it("une page décochable se dit pour ce qu'elle est", () => {
    setLangue("fr");
    const d = defaut("pagination", {
      phrase: { cause: "pagination.bornes", remede: "pagination.decocher_colophon" },
    });
    expect(phraseSimple(d)).toContain("colophon");
  });

  it("une règle inconnue n'a pas de phrase simple : l'écran garde celle du moteur", () => {
    expect(phraseSimple(defaut("regle_future"))).toBeNull();
  });
});
