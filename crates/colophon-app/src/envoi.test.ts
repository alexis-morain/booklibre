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
  formatEnCm,
  imprimeurAffiche,
  papierEnClair,
  porteEffective,
  portesOffertes,
  libelleDestination,
  ouvertureDuFeuilletage,
  phrasePreparation,
  phraseSimple,
  ppiDe,
  aVousDeVoir,
  groupesDefauts,
  pagesEnClair,
  libelleAcquittement,
  cleAcquittement,
  verdictPret,
  boutonEnvoi,
} from "./envoi";
import type { PageSousSeuil } from "./bridge";

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
  it("télécharge le PDF par la porte PDF : le dossier préparé chez Cloudprinter, sans boîte de commande", () => {
    setLangue("fr");
    const a = actionEnvoi(PROFILS[0], "pdf");
    expect(a).toMatchObject({ prepare: true, commande: false, texte: "Télécharger le PDF" });
    setLangue("en");
    expect(actionEnvoi(PROFILS[0], "pdf").texte).toBe("Download the PDF");
  });

  it("enregistre un seul PDF, comme avant, sous le profil sans contrainte", () => {
    setLangue("fr");
    const a = actionEnvoi(PROFILS[3], "pdf");
    expect(a).toMatchObject({ prepare: false, commande: false, texte: "Télécharger le PDF" });
  });

  it("devient « Faire imprimer » par la porte Colophon, chez Cloudprinter seulement", () => {
    setLangue("fr");
    expect(actionEnvoi(PROFILS[0], "colophon")).toMatchObject({ prepare: true, commande: true, texte: "Faire imprimer" });
    setLangue("en");
    expect(actionEnvoi(PROFILS[0], "colophon").texte).toBe("Have it printed");
    for (const p of PROFILS.slice(1)) expect(actionEnvoi(p, "colophon").commande, p.id).toBe(false);
  });

  it("garde la porte PDF par défaut", () => {
    expect(actionEnvoi(PROFILS[0])).toMatchObject({ commande: false });
  });
});

describe("les deux portes", () => {
  it("n'offrent que le PDF sans relais compilé, ou tant que le relais n'a pas répondu", () => {
    expect(portesOffertes(null)).toEqual(["pdf"]);
    expect(portesOffertes({ etat: "absent" })).toEqual(["pdf"]);
  });

  it("offrent les deux dès qu'un relais est compilé, même en panne", () => {
    expect(portesOffertes({ etat: "joignable", pays: [] })).toEqual(["pdf", "colophon"]);
    expect(portesOffertes({ etat: "injoignable" })).toEqual(["pdf", "colophon"]);
  });

  it("ouvrent sur Colophon quand le relais répond, sur le PDF sinon, et gardent le choix fait", () => {
    const joignable = { etat: "joignable" as const, pays: [] };
    expect(porteEffective(null, joignable)).toBe("colophon");
    expect(porteEffective(null, { etat: "injoignable" })).toBe("pdf");
    expect(porteEffective("pdf", joignable)).toBe("pdf");
    expect(porteEffective("colophon", { etat: "injoignable" })).toBe("colophon");
    // Une porte qui n'existe plus ne reste pas choisie.
    expect(porteEffective("colophon", { etat: "absent" })).toBe("pdf");
    expect(porteEffective("colophon", null)).toBe("pdf");
  });
});

describe("le livre en clair", () => {
  it("dit le format en centimètres, à la virgule de la langue", () => {
    setLangue("fr");
    expect(formatEnCm([210, 210])).toBe("21 × 21 cm");
    expect(formatEnCm([203.2, 254])).toBe("20,3 × 25,4 cm");
    setLangue("en");
    expect(formatEnCm([203.2, 254])).toBe("20.3 × 25.4 cm");
    expect(formatEnCm([297, 210])).toBe("29.7 × 21 cm");
  });

  it("tire le papier du profil : grammage, couverture rigide quand elle est cartonnée, finition connue", () => {
    const cp: Printer = {
      ...PROFILS[0],
      rempli_mm: 18,
      commande: { papier: "pageblock_150mcs", couverture: "cover_130mcg", finition: "cover_finish_matte" },
    };
    setLangue("fr");
    expect(papierEnClair(cp, 150)).toBe("Papier 150 g, couverture rigide mate");
    setLangue("en");
    expect(papierEnClair(cp, 150)).toBe("150 gsm paper, matte hardcover");
    // Une finition que l'app ne connaît pas ne s'invente pas.
    setLangue("fr");
    expect(papierEnClair({ ...cp, commande: { ...cp.commande!, finition: "cover_finish_x" } }, 150)).toBe(
      "Papier 150 g, couverture rigide",
    );
  });

  it("ne dit rien du papier sans options de commande : l'imprimeur choisi le décidera", () => {
    expect(papierEnClair(PROFILS[3], 150)).toBeNull();
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
      "Dossier « Corse – Cloudprinter » prêt : les deux PDF relus, rien ne s’oppose à l’impression.",
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
    expect(phraseSimple(d)).toBe(
      "Cette photo est trop petite pour la couverture : elle risque d’imprimer floue. Choisissez-en une autre, ou gardez-la.",
    );
    for (const r of codes.regles) {
      const p = phraseSimple(defaut(r))!;
      expect(p, r).not.toMatch(/\d|ppi|mm\b|\.pdf|--/);
    }
    setLangue("en");
    expect(phraseSimple(d)).toBe("This photo is too small for the cover: it may print blurry. Choose another one, or keep it.");
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

describe("Feuilleter", () => {
  it("ouvre sur la couverture quand son aperçu existe, sur la première planche sinon", () => {
    expect(ouvertureDuFeuilletage(true)).toBe("couverture");
    expect(ouvertureDuFeuilletage(false)).toBe("premiere");
  });
});

// 1.1-c : la résolution ne refuse plus, elle est à vous de voir.
describe("à vous de voir", () => {
  const d = (regle: string, bloquant: boolean): Defaut =>
    ({ regle, bloquant, cause: "", remede: "" }) as Defaut;

  it("ne couvre que la résolution, celle des pages et celle de la couverture", () => {
    expect([...codes.a_vous_de_voir].sort()).toEqual(["couverture_resolution", "resolution"]);
    expect(aVousDeVoir(d("resolution", false))).toBe(true);
    expect(aVousDeVoir(d("couverture_resolution", false))).toBe(true);
    expect(aVousDeVoir(d("zone_sure", false))).toBe(false);
    expect(aVousDeVoir(d("pagination", true))).toBe(false);
  });

  it("range les défauts en trois groupes : refusés, à vous de voir, avertissements", () => {
    const g = groupesDefauts([
      d("zone_sure", false),
      d("resolution", false),
      d("pagination", true),
      d("couverture_resolution", false),
    ]);
    expect(g.refuses.map((x) => x.regle)).toEqual(["pagination"]);
    expect(g.aVoir.map((x) => x.regle)).toEqual(["resolution", "couverture_resolution"]);
    expect(g.avertissements.map((x) => x.regle)).toEqual(["zone_sure"]);
  });

  const p = (fichier: "interieur" | "couverture", page: number): PageSousSeuil => ({ fichier, page, ppi: 180 });

  it("nomme les pages en clair, en français et en anglais", () => {
    setLangue("fr");
    expect(pagesEnClair([p("interieur", 5)])).toBe("la page 5");
    expect(pagesEnClair([p("interieur", 5), p("interieur", 13)])).toBe("les pages 5 et 13");
    expect(pagesEnClair([p("interieur", 5), p("interieur", 13), p("interieur", 21)])).toBe("les pages 5, 13 et 21");
    expect(pagesEnClair([p("couverture", 1)])).toBe("la couverture");
    expect(pagesEnClair([p("interieur", 5), p("couverture", 1)])).toBe("la couverture et la page 5");
    setLangue("en");
    expect(pagesEnClair([p("interieur", 5)])).toBe("page 5");
    expect(pagesEnClair([p("interieur", 5), p("interieur", 13), p("interieur", 21)])).toBe("pages 5, 13 and 21");
    expect(pagesEnClair([p("interieur", 5), p("couverture", 1)])).toBe("the cover and page 5");
  });

  it("écrit la case au singulier et au pluriel", () => {
    setLangue("fr");
    expect(libelleAcquittement([p("interieur", 5)])).toBe(
      "Je sais que la page 5 risque d’imprimer floue (sous 250 ppi), et j’accepte que sa netteté s’écarte de celle d’un livre photo.",
    );
    expect(libelleAcquittement([p("interieur", 5), p("couverture", 1)])).toBe(
      "Je sais que la couverture et la page 5 risquent d’imprimer floues (sous 250 ppi), et j’accepte que leur netteté s’écarte de celle d’un livre photo.",
    );
    setLangue("en");
    expect(libelleAcquittement([p("interieur", 5), p("interieur", 7)])).toBe(
      "I know pages 5 and 7 may print blurry (under 250 ppi), and I accept that their sharpness falls short of a photo book’s.",
    );
  });

  it("change de clé quand la liste change, pour que la case se décoche", () => {
    expect(cleAcquittement([])).toBe("");
    expect(cleAcquittement([p("interieur", 5)])).not.toBe(cleAcquittement([p("interieur", 7)]));
    expect(cleAcquittement([p("interieur", 5)])).toBe(cleAcquittement([p("interieur", 5)]));
  });

  it("dit prêt, avec le compte des photos à vous de voir", () => {
    setLangue("fr");
    expect(verdictPret(0)).toBe("Votre livre est prêt.");
    expect(verdictPret(1)).toBe("Votre livre est prêt. Une photo est à vous de voir.");
    expect(verdictPret(3)).toBe("Votre livre est prêt. 3 photos sont à vous de voir.");
  });

  it("ne grise le bouton que sur un refus, ou sur la case non cochée par la porte Booklibre", () => {
    const pages = [p("interieur", 5)];
    expect(boutonEnvoi({ ok: true, pages: [], commande: false, acquitte: false })).toEqual({ actif: true, confirmer: false });
    expect(boutonEnvoi({ ok: true, pages, commande: false, acquitte: false })).toEqual({ actif: true, confirmer: true });
    expect(boutonEnvoi({ ok: true, pages, commande: true, acquitte: false })).toEqual({ actif: false, confirmer: false });
    expect(boutonEnvoi({ ok: true, pages, commande: true, acquitte: true })).toEqual({ actif: true, confirmer: false });
    expect(boutonEnvoi({ ok: false, pages, commande: false, acquitte: true }).actif).toBe(false);
  });
});
