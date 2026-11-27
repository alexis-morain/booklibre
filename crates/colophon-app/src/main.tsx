import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { installDemo } from "./demo";
import { Frontiere } from "./Frontiere";

const racine = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
const app = (
  <React.StrictMode>
    <Frontiere>
      <App />
    </Frontiere>
  </React.StrictMode>
);

// La démo (`vite build --mode demo`) : le `fetch` du harnais est remplacé
// avant le premier rendu, l'app est posée dans son cadre. La condition est
// écrite en clair pour que le build normal ne garde rien de ce qu'elle porte.
if (import.meta.env.VITE_DEMO === "1") {
  void import("./DemoCadre").then(async ({ DemoCadre }) => {
    await installDemo();
    racine.render(<DemoCadre>{app}</DemoCadre>);
  });
} else {
  racine.render(app);
}
