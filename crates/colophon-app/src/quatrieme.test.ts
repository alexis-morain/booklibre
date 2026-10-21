import { describe, expect, it } from "vitest";
import type { Cover } from "./album";
import {
  avecCadre,
  avecPhotoQuatrieme,
  avecPoliceQuatrieme,
  avecTailleTexte,
  cadreQuatrieme,
  ecartsQuatrieme,
  quatrieme,
  sansPhotoQuatrieme,
  tailleQuatrieme,
} from "./quatrieme";

const panneau = { x: 0, y: 0, w: 210, h: 210 };
const mesure = (s: string, pt: number) => [...s].length * pt * 0.2;

describe("la quatrième", () => {
  it("garde la taille du format sans champ, et borne la sienne", () => {
    const c: Cover = { title: "t" };
    expect(tailleQuatrieme(c, 210)).toBeCloseTo(10.5, 12);
    expect(tailleQuatrieme(c, 300)).toBeCloseTo(15, 12);
    expect(tailleQuatrieme({ ...c, back_taille_pt: 2 }, 210)).toBe(6);
    expect(tailleQuatrieme({ ...c, back_taille_pt: 99 }, 210)).toBe(48);
  });

  it("pose un cadre carré dans la boîte intérieure", () => {
    const p = { photo: { src: "a", focal: [0.5, 0.5] as [number, number] } };
    const haut = cadreQuatrieme(panneau, { ...p, position: "haut", taille: "grande" });
    expect(haut.w).toBeCloseTo(haut.h, 12);
    expect(haut.y + haut.h).toBeCloseTo(210 * 0.91, 9);
    const bas = cadreQuatrieme(panneau, { ...p, position: "bas", taille: "petite" });
    expect(bas.y).toBeCloseTo(210 * 0.09, 9);
    expect(bas.w).toBeCloseTo((210 * 0.82) / 3, 9);
  });

  it("met le texte sous une photo du haut, au-dessus d'une photo du bas", () => {
    const c: Cover = { title: "t", back_text: "Pour Marie." };
    const haut = quatrieme(panneau, 7, avecPhotoQuatrieme(c, "a.jpg"), 10.5, mesure);
    expect(haut.lignes[0].y).toBeLessThan(haut.cadre!.y);
    const bas = quatrieme(
      panneau,
      7,
      avecCadre(avecPhotoQuatrieme(c, "a.jpg"), { position: "bas" }),
      10.5,
      mesure,
    );
    expect(bas.lignes[0].y).toBeGreaterThan(bas.cadre!.y + bas.cadre!.h);
    expect(ecartsQuatrieme(bas, panneau, 7)).toEqual([0, 0]);
  });

  it("compte ce qui ne tient pas", () => {
    const c: Cover = {
      title: "t",
      back_text: Array(60).fill("ligne").join("\n"),
      back_photo: { photo: { src: "a", focal: [0.5, 0.5] }, taille: "grande" },
    };
    const q = quatrieme(panneau, 7, c, 10.5, mesure);
    expect(q.coupees).toBeGreaterThan(0);
    expect(q.lignes.length + q.coupees).toBe(60);
  });

  it("retire chaque champ au lieu de le vider", () => {
    const c: Cover = { title: "t" };
    const avec = avecPhotoQuatrieme(c, "a.jpg");
    expect(avec.back_photo).toEqual({
      photo: { src: "a.jpg", focal: [0.5, 0.5] },
      position: "haut",
      taille: "moyenne",
    });
    // Changer de photo garde le cadre.
    const autre = avecPhotoQuatrieme(avecCadre(avec, { position: "bas" }), "b.jpg");
    expect(autre.back_photo?.position).toBe("bas");
    expect(sansPhotoQuatrieme(avec)).toEqual(c);
    expect(avecCadre(c, { taille: "grande" })).toBe(c);
    const police = { fichier: "objet-X.ttf", postscript: "X", nom: "X" };
    expect(avecPoliceQuatrieme(avecPoliceQuatrieme(c, police), null)).toEqual(c);
    expect(avecTailleTexte(avecTailleTexte(c, 14), null)).toEqual(c);
    expect(avecTailleTexte(c, 14).back_taille_pt).toBe(14);
  });
});
