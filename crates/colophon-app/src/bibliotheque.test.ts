// Le seul vrai danger de la vague 5.2 tient en une phrase : PhotoKit ne lève
// pas quand l'accès manque, il rend une liste vide. Trois causes rendent la
// même liste vide, et deux d'entre elles se corrigent en dix secondes si on
// les nomme. Ce fichier tient cette distinction, et rien d'autre.

import { describe, expect, it } from "vitest";
import { Etat, RapportImport, cleDeLetat, poids, tenterImport } from "./bibliotheque";
import { setLangue, t } from "./i18n";

describe("les trois états d'une liste vide", () => {
  /** Le test qui mord. Si quelqu'un remplace `Etat` par un booléen
   *  « autorisé », ou laisse tomber la lecture du statut avant la requête,
   *  ces trois clés se confondent, et l'écran dit « aucun album » à quelqu'un
   *  qui n'a jamais été interrogé. */
  it("ne se disent pas la même chose", () => {
    const pasEncoreDemande = cleDeLetat({ etat: "a-demander" });
    const injoignable = cleDeLetat({ etat: "injoignable", chemin: "/x.photoslibrary" });
    const vraimentVide = cleDeLetat({ etat: "lisible", albums: [] });

    expect(new Set([pasEncoreDemande, injoignable, vraimentVide]).size).toBe(3);
    expect(pasEncoreDemande).not.toBeNull();
    expect(injoignable).not.toBeNull();
    expect(vraimentVide).not.toBeNull();
  });

  /** Une liste pleine n'a aucune phrase à afficher : c'est la liste qui
   *  parle. Une clé rendue ici mettrait un message au-dessus des albums. */
  it("se taisent dès qu'il y a un album", () => {
    const plein: Etat = {
      etat: "lisible",
      albums: [{ id: "1", nom: "Corse 2013", photos: 64, intelligent: false }],
    };
    expect(cleDeLetat(plein)).toBeNull();
  });
});

describe("la phrase de l'état injoignable", () => {
  /** Elle existe pour épargner une heure de diagnostic, donc elle doit porter
   *  le chemin mort et la manœuvre. Une phrase vague ne vaudrait pas le
   *  troisième état. */
  it("nomme le chemin que macOS cherche, et la case à cocher", () => {
    setLangue("fr");
    const phrase = t("photos.injoignable", {
      chemin: "/Users/x/Pictures/Photos Library.photoslibrary",
    });
    expect(phrase).toContain("/Users/x/Pictures/Photos Library.photoslibrary");
    expect(phrase).toContain("photothèque système");
  });

  /** Quand le délai a mordu, on n'a pas de chemin à montrer : la phrase de
   *  repli doit rester utile, pas devenir un trou. */
  it("garde une manœuvre même sans chemin", () => {
    setLangue("fr");
    const phrase = t("photos.injoignable.sans.chemin");
    expect(phrase).not.toContain("{");
    expect(phrase).toContain("Photos");
  });
});

describe("poids", () => {
  it("monte de kilooctet en gigaoctet", () => {
    expect(poids(2048)).toBe("2 Ko");
    expect(poids(219_100_811)).toBe("209 Mo");
    expect(poids(3_221_225_472)).toBe("3.0 Go");
  });

  /** Zéro octet n'existe pas dans un import réussi, mais un arrondi qui rend
   *  « 0 Ko » ferait croire à une photo vide. */
  it("ne rend jamais zéro pour un fichier qui existe", () => {
    expect(poids(1)).toBe("1 Ko");
  });
});

describe("un import qui échoue", () => {
  /** Mesuré le 17/10 sous bac à sable : « Importer 64 photos… » levait, et
   *  l'écran restait tel quel, sans un mot. Une erreur de l'import se rend
   *  comme une issue, jamais comme une promesse rejetée que personne n'attrape. */
  it("rend sa raison au lieu de se taire", async () => {
    const issue = await tenterImport(() => Promise.reject("création de /x : Operation not permitted"));
    expect(issue).toEqual({ erreur: "création de /x : Operation not permitted" });
  });

  it("garde le rapport d'un import qui aboutit", async () => {
    const rapport: RapportImport = {
      album: "Colophon test",
      dossier: "/x",
      demandees: 1,
      importees: 1,
      octets: 1,
      absentes_du_mac: [],
      echecs: [],
    };
    expect(await tenterImport(() => Promise.resolve(rapport))).toEqual({ rapport });
  });

  it("a une phrase dans les deux langues", () => {
    setLangue("fr");
    expect(t("photos.import.echec", { raison: "R" })).toContain("R");
    setLangue("en");
    expect(t("photos.import.echec", { raison: "R" })).toContain("R");
    setLangue("fr");
  });
});
