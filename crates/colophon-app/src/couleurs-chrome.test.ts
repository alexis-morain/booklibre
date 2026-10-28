import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

/**
 * Les paires de couleurs du chrome, lues dans la feuille de style elle-même :
 * texte à 4,5:1, frontière de contrôle à 3:1, en clair et en sombre. Une paire
 * neuve s'ajoute ici avant de partir (DESIGN.md, « The Tested Pair Rule »).
 */
const css = readFileSync(new URL("./styles.css", import.meta.url), "utf8");

function jetons(bloc: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const m of bloc.matchAll(/--([a-z0-9-]+):\s*(#[0-9a-fA-F]{6})\s*;/g)) out[m[1]] = m[2];
  return out;
}

const clair = jetons(css.slice(css.indexOf(":root {"), css.indexOf("@media (prefers-color-scheme: dark)")));
const sombre = {
  ...clair,
  ...jetons(css.slice(css.indexOf("@media (prefers-color-scheme: dark)"), css.indexOf("\n* {"))),
};

function luminance(hex: string): number {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const [r, g, b] = c.map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
export function rapport(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** [texte, fond] à tenir à 4,5:1 dans les deux thèmes. */
const TEXTES: [string, string][] = [
  ["ink", "ground"], ["ink", "sidebar"], ["ink", "wash"], ["ink", "table"], ["ink", "action-tint"],
  ["ink-muted", "ground"], ["ink-muted", "sidebar"], ["ink-muted", "wash"], ["ink-muted", "table"],
  ["ink-muted", "action-tint"],
  ["on-action", "action"], ["on-action", "action-hover"],
  ["link", "ground"], ["link", "wash"], ["link", "sidebar"],
  ["error", "ground"], ["error", "error-tint"], ["error", "wash"],
  ["chap-ink", "chap"], ["qual-ink", "qual"], ["main-ink", "main"], ["pret-ink", "pret"], ["gris-ink", "gris"],
  ["ink", "qual"], ["ink", "chap"], ["ink", "main"], ["ink", "pret"],
  ["chap-ink", "sidebar"], ["pret-ink", "ground"], ["qual-ink", "ground"], ["main-ink", "ground"],
];

/** Frontières et marques à tenir à 3:1. */
const FRONTIERES: [string, string][] = [
  ["field-border", "ground"], ["field-border", "wash"], ["field-border", "sidebar"],
  ["link", "ground"], ["link", "table"], ["link", "sidebar"],
  ["ink", "ground"], ["ink", "table"], ["action", "ground"],
];

describe.each([
  ["clair", clair],
  ["sombre", sombre],
])("le chrome, thème %s", (_nom, t) => {
  it.each(TEXTES)("texte %s sur %s à 4,5:1", (fg, bg) => {
    expect(rapport(t[fg], t[bg])).toBeGreaterThanOrEqual(4.5);
  });
  it.each(FRONTIERES)("frontière %s sur %s à 3:1", (fg, bg) => {
    expect(rapport(t[fg], t[bg])).toBeGreaterThanOrEqual(3);
  });
});

describe("ce qui est dessiné sur le papier ne change pas de thème", () => {
  it.each([
    ["paper-ink", "paper", 4.5],
    ["paper-ink-soft", "paper", 4.5],
    ["paper-warn", "paper", 4.5],
    ["link-paper", "paper", 3],
  ])("%s sur %s à %s:1", (fg, bg, mini) => {
    expect(clair[fg]).toBe(sombre[fg]);
    expect(rapport(clair[fg], clair[bg])).toBeGreaterThanOrEqual(mini as number);
  });
});
