// La carte dans la scène et dans les deux rendus (P-s5).
//
// La parité complète avec le moteur est celle de `parity.test.ts`, sur la
// fixture : un objet `carte` tourné y est posé exprès (planche 10, 23°), donc
// la boîte, l'angle et l'identité sont comparés au dump du moteur sur six
// formats. Ce fichier tient ce que la fixture ne peut pas dire : la forme de
// la boîte retournée, ce que `estLibre` accepte, et les attributs que le DOM
// pose, les mêmes valeurs que `peindreCarte` pose au canvas.

import { describe, expect, it } from "vitest";
import fixture from "./geometrie.fixture.json";
import sceneFixture from "./scene.fixture.json";
import { Album, Objet, Spread, spreadGeometry } from "./album";
import { attributsCalque, Calque, carteDe, cleCarte, setCartes } from "./carte";
import { Dump, setGeometrie } from "./geometrie";
import { estLibre, sceneOf } from "./scene";

setGeometrie(fixture as unknown as Dump);
const g = spreadGeometry({ trim_mm: fixture.trim_mm, bleed_mm: fixture.bleed_mm } as Album);
const mesure = (s: string, mm: number) => s.length * mm * 0.5;

const carte: Objet = { x: 120, y: 40, w: 84, h: 50.4, angle: 23, type: "carte", debut: 2, fin: 9 };
const planche: Spread = { template: "vide", slots: [], objets: [carte] };

describe("la carte dans la scène", () => {
  it("porte son identité, sa boîte retournée et son angle, sans dessin", () => {
    const [o] = sceneOf(planche, g, mesure).objects;
    expect(o.role).toEqual({ role: "carte", index: 0, debut: 2, fin: 9 });
    expect(o.angle).toBe(23);
    // Le repère moteur est en bas à gauche, celui de la scène en haut.
    expect(o.rect).toEqual({ x: 120, y: g.h - (40 + 50.4), w: 84, h: 50.4 });
    expect(estLibre(o.role)).toBe(true);
  });

  it("est dans la fixture du moteur avec le même angle et les mêmes plages", () => {
    const parFormat = Object.values(sceneFixture as unknown as Record<string, any[][]>);
    for (const scenes of parFormat) {
      const cartes = scenes.flatMap((s: any) => s.objects).filter((o: any) => o.role.role === "carte");
      expect(cartes).toHaveLength(1);
      expect(cartes[0].angle).toBe(23);
      expect([cartes[0].role.debut, cartes[0].role.fin]).toEqual([2, 9]);
    }
  });

  it("une plage sans dessin n'en a pas, et se retrouve par sa clé", () => {
    setCartes({});
    expect(carteDe(2, 9)).toBeUndefined();
    setCartes({ [cleCarte(2, 9)]: { viewbox: [0, 0, 100, 50], niveau: "monde", calques: [] } });
    expect(carteDe(2, 9)?.viewbox[2]).toBe(100);
    setCartes({});
  });
});

describe("les attributs d'un calque", () => {
  const base = { role: "x", chemins: [] };
  it("remplit sans trait, ou avec le contour de même encre qui recolle les moitiés", () => {
    const terre: Calque = { ...base, style: { style: "remplir" }, encre: "#f1ead8", contour_mm: 0.12 };
    expect(attributsCalque(terre)).toEqual({ fill: "#f1ead8", stroke: "#f1ead8", strokeWidth: 0.12 });
    expect(attributsCalque({ ...terre, contour_mm: undefined })).toEqual({ fill: "#f1ead8", stroke: "none" });
  });
  it("trace en mm du dessin, et pointille le vol", () => {
    const vol: Calque = {
      ...base,
      style: { style: "tracer", epaisseur_mm: 0.4, pointille: [1.2, 0.9] },
      encre: "#c0392b",
    };
    expect(attributsCalque(vol)).toEqual({
      fill: "none",
      stroke: "#c0392b",
      strokeWidth: 0.4,
      strokeDasharray: "1.2 0.9",
    });
    const sol = { ...vol, style: { style: "tracer" as const, epaisseur_mm: 0.5, pointille: null } };
    expect(attributsCalque(sol).strokeDasharray).toBeUndefined();
  });
});
