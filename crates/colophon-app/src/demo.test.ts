// Le shim de la démo : ce que `fetch` rend quand le harnais de dev n'existe
// pas, un manifeste d'enregistrements à sa place.

import { describe, expect, it } from "vitest";
import { cleDeRoute, DEMO_EN, DEMO_FR, fileDernierArrive, lienTelecharger, premierCoupDOeil, repondre, texteDemo, type Manifeste } from "./demo";

const url = (s: string) => new URL(s, "http://demo.test");

const manifeste: Manifeste = {
  version: 1,
  routes: {
    [cleDeRoute("/__dev/album", new URLSearchParams())]: {
      f: "album-en.json",
      t: "application/json",
      langues: { fr: "album-fr.json", en: "album-en.json" },
    },
    [cleDeRoute("/__dev/thumb", new URLSearchParams({ src: "a b.jpg" }))]: { f: "t/0001.jpg", t: "image/jpeg" },
    [cleDeRoute("/__dev/reserve", new URLSearchParams({ planche: "3", case: "2" }))]: {
      f: "r/3-2.json",
      t: "application/json",
    },
  },
};

const etat = () => ({ album: null as unknown });
const get = (u: string, e = etat()) => repondre(manifeste, "GET", url(u), undefined, e, "fr");
const post = (u: string, corps: string, e = etat()) => repondre(manifeste, "POST", url(u), corps, e, "fr");

describe("cleDeRoute", () => {
  it("ne dépend pas de l'ordre des paramètres", () => {
    expect(cleDeRoute("/__dev/reserve", new URLSearchParams("planche=3&case=2"))).toBe(
      cleDeRoute("/__dev/reserve", new URLSearchParams("case=2&planche=3")),
    );
  });

  it("distingue deux requêtes qui ne diffèrent que par une valeur", () => {
    expect(cleDeRoute("/__dev/thumb", new URLSearchParams("src=a"))).not.toBe(
      cleDeRoute("/__dev/thumb", new URLSearchParams("src=b")),
    );
  });
});

describe("repondre : les lectures enregistrées", () => {
  it("sert le livre écrit dans la langue de l'interface", () => {
    const e = etat();
    expect(repondre(manifeste, "GET", url("/__dev/album"), undefined, e, "fr").fichier).toBe("album-fr.json");
    expect(repondre(manifeste, "GET", url("/__dev/album"), undefined, e, "en").fichier).toBe("album-en.json");
  });

  it("sert un fichier du manifeste avec son type", () => {
    expect(get("/__dev/thumb?src=a%20b.jpg")).toMatchObject({ status: 200, type: "image/jpeg", fichier: "t/0001.jpg" });
  });

  it("retrouve une requête quel que soit l'ordre de ses paramètres", () => {
    expect(get("/__dev/reserve?case=2&planche=3")).toMatchObject({ status: 200, fichier: "r/3-2.json" });
  });

  it("rend 404 pour ce qui n'a pas été enregistré, sans rien inventer", () => {
    const r = get("/__dev/thumb?src=inconnue.jpg");
    expect(r.status).toBe(404);
    expect(r.fichier).toBeUndefined();
  });
});

describe("repondre : l'écriture reste en mémoire", () => {
  it("garde l'album posté et le rend au GET suivant", () => {
    const e = etat();
    expect(post("/__dev/album", '{"title":"modifié"}', e)).toMatchObject({ status: 200 });
    expect(e.album).toEqual({ title: "modifié" });
    expect(get("/__dev/album", e)).toMatchObject({ fichier: "album-fr.json", fusionAlbum: true });
  });

  it("refuse un corps qui n'est pas du JSON et ne touche pas à la mémoire", () => {
    const e = etat();
    expect(post("/__dev/album", "{pas du json", e).status).toBe(500);
    expect(e.album).toBeNull();
  });
});

describe("repondre : ce qui demande le moteur", () => {
  const demandes: [string, string, string][] = [
    ["POST", "/__dev/recomposer", "recomposer"],
    ["POST", "/__dev/preparer?profil=cloudprinter", "export"],
    ["POST", "/__dev/preparer-commande", "export"],
    ["GET", "/__dev/prevol?profil=cloudprinter", "export"],
    ["GET", "/__dev/remplir", "remplir"],
    ["GET", "/__dev/survol", "album"],
    ["POST", "/__dev/relais/creer", "commande"],
  ];

  it.each(demandes)("%s %s répond 501 avec le message « %s » et le lien de téléchargement", (m, u, quoi) => {
    const r = repondre(manifeste, m, url(u), "{}", etat(), "fr");
    expect(r.status).toBe(501);
    expect(r.corps).toBe(DEMO_FR[`refus.${quoi}` as keyof typeof DEMO_FR]);
    expect(r.corps).toContain("booklibre.app");
  });

  it("parle anglais quand l'interface le fait", () => {
    const r = repondre(manifeste, "GET", url("/__dev/remplir"), undefined, etat(), "en");
    expect(r.corps).toBe(DEMO_EN["refus.remplir"]);
  });

  it("dit qu'il n'y a pas de relais plutôt que de répondre en erreur", () => {
    expect(repondre(manifeste, "POST", url("/__dev/relais/pays"), "{}", etat(), "fr")).toMatchObject({
      status: 200,
      corps: "null",
    });
    expect(repondre(manifeste, "POST", url("/__dev/relais/tarifs-offerts"), "{}", etat(), "fr")).toMatchObject({
      corps: "false",
    });
  });

  it("l'aperçu de couverture est déjà enregistré : le POST répond ok", () => {
    expect(repondre(manifeste, "POST", url("/__dev/couverture?profil=cloudprinter"), "", etat(), "fr")).toMatchObject({
      status: 200,
      corps: "ok",
    });
  });
});

describe("les textes de la démo", () => {
  it("ont les mêmes clés en français et en anglais", () => {
    expect(Object.keys(DEMO_EN).sort()).toEqual(Object.keys(DEMO_FR).sort());
  });

  it("ne nomment pas le logiciel Colophon, ne portent aucun tiret cadratin", () => {
    for (const dico of [DEMO_FR, DEMO_EN]) {
      for (const [cle, texte] of Object.entries(dico)) {
        expect(texte, cle).not.toMatch(/Colophon/);
        expect(texte, cle).not.toContain("—");
      }
    }
  });

  it("envoient vers l'ancre de téléchargement, dans la langue de l'interface", () => {
    expect(lienTelecharger("fr")).toBe("https://booklibre.app/fr/#get");
    expect(lienTelecharger("en")).toBe("https://booklibre.app/#get");
  });

  it("remplissent les trous du crédit", () => {
    expect(texteDemo("bandeau.credit", "en", { auteur: "A B", licence: "CC BY-SA 4.0" })).toBe("Photos: A B, CC BY-SA 4.0");
  });

  it("n'annoncent plus de vidéo sur l'écran mobile", () => {
    expect(Object.keys(DEMO_FR)).not.toContain("mobile.video");
    expect(DEMO_FR["mobile.texte"]).not.toMatch(/vidéo/);
  });
});

describe("premierCoupDOeil", () => {
  const p = (...n: number[]) => n.map((k) => ({ slots: new Array(k).fill(0) }));
  it("saute la page vide et prend la première planche de deux photos ou plus", () => {
    expect(premierCoupDOeil(p(0, 1, 3, 2))).toBe(2);
  });
  it("se rabat sur la première planche à une photo, puis sur la première", () => {
    expect(premierCoupDOeil(p(0, 1, 1))).toBe(1);
    expect(premierCoupDOeil(p(0, 0))).toBe(0);
  });
});

describe("fileDernierArrive", () => {
  // Les vignettes de la planche à l'écran sont demandées après celles que la
  // légende de qualité réclame toutes à l'arrivée : elles doivent passer devant.
  const differe = () => {
    let fin!: () => void;
    const p = new Promise<void>((r) => (fin = r));
    return { p, fin };
  };

  it("ne lance jamais plus de tâches que permis", async () => {
    const file = fileDernierArrive(2);
    const d = [differe(), differe(), differe()];
    let enCours = 0;
    let max = 0;
    const tous = d.map((x) =>
      file(async () => {
        enCours++;
        max = Math.max(max, enCours);
        await x.p;
        enCours--;
      }),
    );
    d.forEach((x) => x.fin());
    await Promise.all(tous);
    expect(max).toBe(2);
  });

  it("sert la dernière demande arrivée en premier quand une place se libère", async () => {
    const file = fileDernierArrive(1);
    const ordre: string[] = [];
    const bloque = differe();
    const premier = file(async () => {
      ordre.push("a");
      await bloque.p;
    });
    const b = file(async () => void ordre.push("b"));
    const c = file(async () => void ordre.push("c"));
    bloque.fin();
    await Promise.all([premier, b, c]);
    expect(ordre).toEqual(["a", "c", "b"]);
  });

  it("fait passer devant une tâche déjà en attente qu'on redemande", async () => {
    // La légende demande la vignette 3 tôt, elle est au fond de la pile ; la
    // planche à l'écran la veut ensuite, mais `loadThumb` ne la redemande
    // pas (même promesse) : sans `devant`, la 3e page tournée attendait 3 s.
    const file = fileDernierArrive(1);
    const ordre: string[] = [];
    const bloque = differe();
    const premier = file(async () => {
      ordre.push("a");
      await bloque.p;
    });
    const t = ["v3", "v2", "v1"].map((v) => file(async () => void ordre.push(v), v));
    file.devant(["v3", "v2"]);
    bloque.fin();
    await Promise.all([premier, ...t]);
    expect(ordre).toEqual(["a", "v3", "v2", "v1"]);
  });

  it("ignore ce qui n'est pas en attente", async () => {
    const file = fileDernierArrive(1);
    file.devant(["absent"]);
    await expect(file(async () => 7, "x")).resolves.toBe(7);
  });

  it("libère la place même quand une tâche échoue", async () => {
    const file = fileDernierArrive(1);
    await expect(file(() => Promise.reject(new Error("x")))).rejects.toThrow("x");
    await expect(file(async () => 7)).resolves.toBe(7);
  });
});
