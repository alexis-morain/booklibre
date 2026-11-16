// React exige le même nombre de hooks à chaque rendu d'un composant. `App`
// rend l'accueil, la planche vide et le bilan par des `return` anticipés :
// un hook appelé après eux n'existe qu'une fois l'album ouvert, et React
// tombe (erreur minifiée #310) au passage du bilan à l'album. C'est arrivé
// le 15/11 avec `useLimitePages`, posé sous ces `return` par P-s3.
// Pas d'eslint dans l'app : ce test tient la règle pour `App`.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const source = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

function corpsDeApp(src: string): string[] {
  const lignes = src.split("\n");
  const debut = lignes.findIndex((l) => l.startsWith("export default function App("));
  const fin = lignes.findIndex((l, i) => i > debut && l === "}");
  return lignes.slice(debut, fin);
}

/** Les hooks appelés par `App` après son premier `return` anticipé. */
export function hooksApresRetour(src: string): string[] {
  const corps = corpsDeApp(src);
  const premier = corps.findIndex((l) => /^  if \(.*\) \{$/.test(l) && /^    return/.test(corps[corps.indexOf(l) + 1] ?? ""));
  if (premier < 0) return [];
  return corps
    .slice(premier)
    .filter((l) => !l.trim().startsWith("//") && !l.trim().startsWith("*"))
    .filter((l) => /^  (const .* = )?use[A-Z][A-Za-z]*\(/.test(l))
    .map((l) => l.trim());
}

describe("App et la règle des hooks", () => {
  it("n'appelle aucun hook après un return anticipé", () => {
    expect(hooksApresRetour(source)).toEqual([]);
  });

  it("le test mord : un hook sous le return est vu", () => {
    const fautif = [
      "export default function App() {",
      "  const a = useState(0);",
      "  if (!album || building) {",
      "    return null;",
      "  }",
      "  const limite = useLimitePages();",
      "}",
    ].join("\n");
    expect(hooksApresRetour(fautif)).toEqual(["const limite = useLimitePages();"]);
  });
});
