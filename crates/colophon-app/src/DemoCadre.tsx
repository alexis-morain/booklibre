// Le cadre de la démo : le bandeau permanent, la boîte « disponible dans
// l'app » que `demoRefus` ouvre, et l'écran qui remplace l'éditeur sur un
// téléphone. Monté par `main.tsx` seulement quand `VITE_DEMO` est posé.

import { useEffect, useRef, useState } from "react";
import { Dialogue } from "./Dialogue";
import { LARGEUR_MIN, lienTelecharger, Quoi, surRefus, texteDemo } from "./demo";
import { useLangue } from "./i18n";
import "./demo.css";

/** Trop étroit pour l'éditeur, ou seulement tactile : pas de souris à
 *  l'horizon. Réévalué à chaque changement de fenêtre. */
function ecranInadapte(): boolean {
  const seulTactile =
    window.matchMedia("(pointer: coarse)").matches && !window.matchMedia("(any-pointer: fine)").matches;
  return window.innerWidth < LARGEUR_MIN || seulTactile;
}

function useInadapte(): boolean {
  const [v, setV] = useState(ecranInadapte);
  useEffect(() => {
    const f = () => setV(ecranInadapte());
    window.addEventListener("resize", f);
    return () => window.removeEventListener("resize", f);
  }, []);
  return v;
}

/** Le crédit des photos que `scripts/demo.sh` écrit quand on lui donne un
 *  fichier de sources ; absent, le bandeau n'en dit rien. */
function useCredit(): { auteur: string; licence: string } | null {
  const [credit, setCredit] = useState<{ auteur: string; licence: string } | null>(null);
  useEffect(() => {
    fetch(`${import.meta.env.BASE_URL}demo-data/credits.json`)
      .then((r) => (r.ok ? r.json() : null))
      .then(setCredit, () => {});
  }, []);
  return credit;
}

export function DemoCadre({ children }: { children: React.ReactNode }) {
  const l = useLangue();
  const lien = lienTelecharger(l);
  const credit = useCredit();
  const [refus, setRefus] = useState<Quoi | null>(null);
  const inadapte = useInadapte();
  // L'éditeur qui s'est montré ne se retire plus : réduire la fenêtre ne doit
  // pas jeter les retouches de la séance.
  const montre = useRef(false);
  if (!inadapte) montre.current = true;

  useEffect(() => surRefus(setRefus), []);
  useEffect(() => {
    if (!refus) return;
    const f = (e: KeyboardEvent) => e.key === "Escape" && setRefus(null);
    window.addEventListener("keydown", f);
    return () => window.removeEventListener("keydown", f);
  }, [refus]);

  if (inadapte && !montre.current) {
    return (
      <main className="demo-mobile">
        <h1>{texteDemo("mobile.titre")}</h1>
        <p>{texteDemo("mobile.texte")}</p>
        <a className="cta" href={lien} target="_blank" rel="noopener">
          {texteDemo("mobile.telecharger")}
        </a>
      </main>
    );
  }

  return (
    <div className="demo-cadre">
      <div className="demo-bandeau" role="note">
        <span>{texteDemo("bandeau.texte")}</span>
        <span aria-hidden="true">·</span>
        <a href={lien} target="_blank" rel="noopener">
          {texteDemo("bandeau.lien")}
        </a>
        {credit && (
          <>
            <span aria-hidden="true">·</span>
            <span>{texteDemo("bandeau.credit", l, credit)}</span>
            <span aria-hidden="true">·</span>
            <a href={`${import.meta.env.BASE_URL}credits.html`} target="_blank" rel="noopener">
              {texteDemo("bandeau.credits")}
            </a>
          </>
        )}
      </div>
      <div className="demo-app">{children}</div>
      {refus && (
        <Dialogue titre={texteDemo("refus.titre")} onClose={() => setRefus(null)} panelClassName="demo-refus">
          <p>{texteDemo(`refus.${refus}`)}</p>
          <a className="cta" href={lien} target="_blank" rel="noopener">
            {texteDemo("refus.lien")}
          </a>
        </Dialogue>
      )}
    </div>
  );
}
