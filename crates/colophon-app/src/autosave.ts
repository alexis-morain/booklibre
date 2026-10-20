// L'écriture automatique de l'album (1.1-b). Une file d'un seul rang : une
// écriture à la fois, la suivante attend et part avec le dernier état, et
// une valeur déjà sur le disque ne repart pas. Sans React et sans disque,
// donc testée seule (`autosave.test.ts`).

export type Ecrivain<T> = {
  /** Écrire `v` après `delai` sans autre modification. */
  planifier(v: T): void;
  /** Écrire tout de suite ce qui attend ; vrai quand le disque porte le dernier état. */
  maintenant(): Promise<boolean>;
  /** `v` est déjà sur le disque (ouverture, ⌘S) : rien à écrire pour lui. */
  connu(v: T): void;
  /** Ce qui attend, le délai compris, est abandonné. */
  oublier(): void;
  /** La dernière écriture a échoué et rien n'a réussi depuis. */
  echoue(): boolean;
  /** Une autre écriture (⌘S), dans la même file : jamais deux à la fois. */
  enFile<R>(tache: () => Promise<R>): Promise<R>;
};

export function creerEcrivain<T>(
  ecrire: (v: T) => Promise<void>,
  opts: { delai: number; ecrit?: (v: T) => void; echec?: (e: unknown) => void },
): Ecrivain<T> {
  let disque: T | undefined;
  let suivant: { v: T } | null = null;
  let minuterie: ReturnType<typeof setTimeout> | null = null;
  let file: Promise<void> = Promise.resolve();
  let enEchec = false;

  const arreter = () => {
    if (minuterie !== null) clearTimeout(minuterie);
    minuterie = null;
  };

  // Ne rejette jamais : un échec se dit par `echec` et laisse la file vivante.
  const vidange = async () => {
    while (suivant) {
      const { v } = suivant;
      suivant = null;
      if (v === disque) continue;
      try {
        await ecrire(v);
        disque = v;
        enEchec = false;
        opts.ecrit?.(v);
      } catch (e) {
        enEchec = true;
        opts.echec?.(e);
      }
    }
  };
  const lancer = () => (file = file.then(vidange));

  return {
    planifier(v) {
      arreter();
      if (v === disque) {
        suivant = null;
        // Un ⌘Z qui ramène ce que porte le disque efface l'échec : plus rien ne manque.
        if (enEchec) {
          enEchec = false;
          opts.ecrit?.(v);
        }
        return;
      }
      suivant = { v };
      minuterie = setTimeout(() => {
        minuterie = null;
        void lancer();
      }, opts.delai);
    },
    maintenant() {
      arreter();
      return lancer().then(() => !enEchec);
    },
    connu(v) {
      disque = v;
      enEchec = false;
      if (suivant?.v === v) suivant = null;
    },
    oublier() {
      arreter();
      suivant = null;
    },
    echoue: () => enEchec,
    enFile(tache) {
      const p = file.then(tache);
      file = p.then(
        () => {},
        () => {},
      );
      return p;
    },
  };
}
