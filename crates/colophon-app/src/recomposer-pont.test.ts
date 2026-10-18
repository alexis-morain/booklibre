// Ce que `recomposeAlbum` demande au moteur, dans la coquille : les deux
// choix du panneau, et rien quand rien n'a changé ; puis la géométrie de
// l'album recomposé, qui peut être d'un autre format.

import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

const appels: { cmd: string; args: Record<string, unknown> | undefined }[] = [];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    appels.push({ cmd, args });
    return null;
  }),
}));

describe("recomposeAlbum", () => {
  beforeAll(() => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  });
  afterAll(() => {
    vi.unstubAllGlobals();
  });

  it("sans choix, n'envoie ni format ni rythme", async () => {
    const b = await import("./bridge");
    appels.length = 0;
    await b.recomposeAlbum().catch(() => {});
    const a = appels.find((x) => x.cmd === "recompose_album");
    expect(a?.args).toMatchObject({ format: null, densite: null });
  });

  it("envoie le format et le rythme choisis, puis recharge la géométrie", async () => {
    const b = await import("./bridge");
    appels.length = 0;
    await b
      .recomposeAlbum({ format: { w: 297, h: 210 }, densite: "aeree" })
      .catch(() => {});
    const i = appels.findIndex((x) => x.cmd === "recompose_album");
    expect(appels[i].args).toMatchObject({ format: { w: 297, h: 210 }, densite: "aeree" });
    // Sans elle, un album passé en A4 se dessinerait sur le dump du carré,
    // ou ne se dessinerait pas : une fenêtre blanche.
    expect(appels.slice(i + 1).some((x) => x.cmd === "geometrie")).toBe(true);
  });
});
