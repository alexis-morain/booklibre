// L'écriture automatique (1.1-b) : une file qui attend que la main se pose,
// n'écrit jamais deux fois en même temps, et part toujours avec le dernier
// état. Un faux disque compte les écritures et les retient à volonté.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { creerEcrivain } from "./autosave";

/** Un disque dont chaque écriture attend qu'on la libère. */
function disque() {
  const ecrits: string[] = [];
  const enVol: Array<{ v: string; finir: (ok: boolean) => void }> = [];
  const ecrire = (v: string) =>
    new Promise<void>((ok, ko) => {
      enVol.push({
        v,
        finir: (reussi) => {
          if (reussi) {
            ecrits.push(v);
            ok();
          } else ko(new Error("disque plein"));
        },
      });
    });
  return { ecrits, enVol, ecrire };
}

/** Laisse passer les promesses en attente. */
const vider = () => vi.advanceTimersByTimeAsync(0);

describe("creerEcrivain", () => {
  beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
  afterEach(() => vi.useRealTimers());

  it("deux modifications rapides font une écriture, avec le dernier état", async () => {
    const d = disque();
    const e = creerEcrivain(d.ecrire, { delai: 1000 });
    e.planifier("a");
    vi.advanceTimersByTime(500);
    e.planifier("b");
    await vi.advanceTimersByTimeAsync(999);
    expect(d.enVol).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(1);
    expect(d.enVol.map((x) => x.v)).toEqual(["b"]);
    d.enVol[0].finir(true);
    await vi.runAllTimersAsync();
    expect(d.ecrits).toEqual(["b"]);
  });

  it("une modification pendant une écriture lente part après elle, jamais en même temps", async () => {
    const d = disque();
    const e = creerEcrivain(d.ecrire, { delai: 1000 });
    e.planifier("a");
    await vi.advanceTimersByTimeAsync(1000);
    expect(d.enVol).toHaveLength(1);
    e.planifier("b");
    e.planifier("c");
    await vi.advanceTimersByTimeAsync(1000);
    expect(d.enVol).toHaveLength(1);
    d.enVol[0].finir(true);
    await vi.runOnlyPendingTimersAsync();
    expect(d.enVol.map((x) => x.v)).toEqual(["a", "c"]);
    d.enVol[1].finir(true);
    await vi.runAllTimersAsync();
    expect(d.ecrits).toEqual(["a", "c"]);
  });

  it("maintenant n'attend pas le délai, et rend vrai une fois écrit", async () => {
    const d = disque();
    const e = creerEcrivain(d.ecrire, { delai: 1000 });
    e.planifier("a");
    const p = e.maintenant();
    await vi.runOnlyPendingTimersAsync();
    expect(d.enVol.map((x) => x.v)).toEqual(["a"]);
    d.enVol[0].finir(true);
    expect(await p).toBe(true);
    vi.advanceTimersByTime(5000);
    expect(d.enVol).toHaveLength(1);
  });

  it("ce qui est déjà sur le disque ne s'écrit pas", async () => {
    const d = disque();
    const ecrit = vi.fn();
    const e = creerEcrivain(d.ecrire, { delai: 1000, ecrit });
    e.connu("a");
    e.planifier("a");
    vi.advanceTimersByTime(1000);
    expect(await e.maintenant()).toBe(true);
    expect(d.enVol).toHaveLength(0);
    expect(ecrit).not.toHaveBeenCalled();
  });

  it("un échec se dit, rend faux, et la modification suivante retente", async () => {
    const d = disque();
    const echec = vi.fn();
    const ecrit = vi.fn();
    const e = creerEcrivain(d.ecrire, { delai: 1000, echec, ecrit });
    e.planifier("a");
    const p = e.maintenant();
    await vider();
    d.enVol[0].finir(false);
    expect(await p).toBe(false);
    expect(echec).toHaveBeenCalledTimes(1);
    expect(e.echoue()).toBe(true);

    e.planifier("b");
    vi.advanceTimersByTime(1000);
    await vider();
    d.enVol[1].finir(true);
    expect(await e.maintenant()).toBe(true);
    expect(ecrit).toHaveBeenCalledWith("b");
    expect(e.echoue()).toBe(false);
  });

  it("une tâche en file (⌘S) attend l'écriture en vol, et connu lève l'échec", async () => {
    const d = disque();
    const e = creerEcrivain(d.ecrire, { delai: 1000 });
    e.planifier("a");
    void e.maintenant();
    await vider();
    let partie = false;
    const t = e.enFile(async () => {
      partie = true;
      e.connu("s");
    });
    await vider();
    expect(partie).toBe(false);
    d.enVol[0].finir(false);
    await t;
    expect(partie).toBe(true);
    expect(e.echoue()).toBe(false);
  });

  it("revenir à ce que porte le disque lève l'échec sans écrire", async () => {
    const d = disque();
    const ecrit = vi.fn();
    const e = creerEcrivain(d.ecrire, { delai: 1000, ecrit });
    e.connu("a");
    e.planifier("b");
    const p = e.maintenant();
    await vider();
    d.enVol[0].finir(false);
    expect(await p).toBe(false);
    e.planifier("a");
    expect(e.echoue()).toBe(false);
    expect(ecrit).toHaveBeenCalledWith("a");
    expect(d.enVol).toHaveLength(1);
  });
});
