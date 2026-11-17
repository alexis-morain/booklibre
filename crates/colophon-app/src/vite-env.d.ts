/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** « 1 » dans la version Mac App Store (`scripts/mas.sh`), absent ailleurs. */
  readonly VITE_COLOPHON_MAS?: string;
  /** "1" rallume l'import Polarsteps, en pause depuis le 17/11 (`voyage.ts`). */
  readonly VITE_COLOPHON_VOYAGE?: string;
}
