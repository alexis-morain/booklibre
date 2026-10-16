// Les mots que le moteur écrit dans le livre suivent la langue de
// l'interface au moment où il les écrit (`colophon_core::langue`). Ce test
// tient le bout du front : chaque commande qui fait écrire du texte au moteur
// lui passe la langue courante, et un oubli se voit ici plutôt que sur papier.

import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

const appels: { cmd: string; args: Record<string, unknown> | undefined }[] = [];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    appels.push({ cmd, args });
    return null;
  }),
}));

describe("la langue voyage jusqu'au moteur", () => {
  beforeAll(() => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  });
  afterAll(() => {
    vi.unstubAllGlobals();
  });

  it("dans chaque commande qui écrit des mots du livre", async () => {
    const { setLangue } = await import("./i18n");
    const b = await import("./bridge");
    const album = { title: "Corsica", spreads: [] } as never;
    for (const l of ["en", "fr"] as const) {
      setLangue(l);
      appels.length = 0;
      await b.buildAlbum("/p", "carre-21", 24, "equilibree", null).catch(() => {});
      await b.recomposeAlbum().catch(() => {});
      await b.captionSuggestion("a.jpg");
      await b.datesFiables(album);
      await b.legendeProposee(0);
      await b.colophonSpread(album);
      await b.gardeSpread(album);
      for (const cmd of [
        "build_album_from_folder",
        "recompose_album",
        "caption_suggestion",
        "dates_fiables",
        "proposition_legende",
        "colophon_spread",
        "garde_spread",
      ]) {
        const a = appels.find((x) => x.cmd === cmd);
        expect(a, cmd).toBeDefined();
        expect(a?.args?.langue, cmd).toBe(l);
      }
    }
  });
});

describe("les noms de touches", () => {
  it("suivent la langue de l'interface", async () => {
    const { setLangue, t } = await import("./i18n");
    setLangue("fr");
    expect(t("touche.entree")).toBe("Entrée");
    expect(t("touche.echap")).toBe("Échap");
    setLangue("en");
    expect(t("touche.entree")).toBe("Return");
    expect(t("touche.echap")).toBe("Esc");
    setLangue("fr");
  });

  it("ne mélangent pas les langues : l'anglais dit Return, comme un Mac", async () => {
    const { EN, FR } = await import("./i18n");
    for (const [cle, texte] of Object.entries(EN)) {
      expect(/\b(Entrée|Échap|Enter)\b/.test(texte), cle).toBe(false);
    }
    for (const [cle, texte] of Object.entries(FR)) {
      expect(/\b(Enter|Return|Esc)\b/.test(texte), cle).toBe(false);
    }
  });

  /** Une touche nommée par un mot vient du dictionnaire. Ce qui reste écrit
   *  en dur dans un `<kbd>` est un symbole ou une lettre seule : ⌘, ←, R. */
  it("ne s'écrivent jamais en dur dans un <kbd>", () => {
    const dossier = fileURLToPath(new URL(".", import.meta.url));
    for (const f of readdirSync(dossier).filter((n) => n.endsWith(".tsx"))) {
      const source = readFileSync(dossier + f, "utf8");
      for (const m of source.matchAll(/<kbd>([^<{]*)<\/kbd>/g)) {
        expect(/\p{L}{2,}/u.test(m[1]), `${f} : <kbd>${m[1]}</kbd>`).toBe(false);
      }
    }
  });
});
