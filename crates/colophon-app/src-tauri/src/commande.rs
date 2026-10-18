//! Commander depuis Envoi, côté app (S-s3) : ce que l'app garde des
//! intentions de commande qu'elle a ouvertes au relais.
//!
//! **Le secret vit ici, et nulle part ailleurs.** Chaque intention garde son
//! identifiant, son secret de lecture, son mode, sa date et son dernier état
//! dans `commande.json`, au dossier de données de l'app, en `0600` sur Unix.
//! La fenêtre ne relit jamais ce fichier : elle reçoit des [`IntentionVue`],
//! où le secret n'a pas de champ. Rien de ce module n'écrit au journal une
//! valeur ni le nom du fichier : le rapport de Signaler cite le journal.
//!
//! **Une seule écriture à la fois.** Toute lecture-modification-écriture du
//! fichier passe par [`modifier`], sous le verrou que tient `AppState` ; une
//! relecture au relais ne réécrit jamais un cliché pris avant ses appels
//! réseau, elle relit le fichier sous le verrou et ne pose que les champs de
//! l'intention relue ([`relire`]).

use colophon_core::relais::{Abandon, Mode, Relais, Secret};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Le nom du fichier, au dossier de données de l'app (celui de K-s3).
pub const FICHIER: &str = "commande.json";

/// Ce que l'app garde d'une intention. Le `Debug` du secret rend `***`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentionGardee {
    pub id: String,
    pub secret: Secret,
    pub mode: Mode,
    /// Le jour de la création, `AAAA-MM-JJ`.
    pub date: String,
    /// Le dernier état que le relais a rendu.
    pub etat: String,
    /// Le dernier numéro d'état de Cloudprinter, une fois commandée.
    #[serde(default)]
    pub dernier_code: Option<i64>,
    /// Le niveau d'expédition choisi, gardé dès le choix : la liste rouvre
    /// le paiement avec lui. Ce n'est pas un secret.
    #[serde(default)]
    pub niveau: Option<String>,
}

/// Le contenu de `commande.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Stock {
    #[serde(default)]
    pub intentions: Vec<IntentionGardee>,
}

/// Une intention, telle que la fenêtre la voit : ni secret, ni URL.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IntentionVue {
    pub id: String,
    pub mode: Mode,
    pub etat: String,
    pub dernier_code: Option<i64>,
    pub niveau: Option<String>,
    /// Le jour de la création, `AAAA-MM-JJ` : la carte de la commande le dit.
    /// Ce n'est pas un secret.
    pub date: String,
}

/// Les quatre états que plus rien ne fait bouger (la machine du relais).
pub fn terminal(etat: &str) -> bool {
    matches!(etat, "echec_remboursee" | "annulee" | "purgee" | "abandonnee")
}

impl Stock {
    /// Lit `commande.json` dans `dir`. Absent, le stock est vide.
    pub fn lire(dir: &Path) -> Result<Stock, String> {
        match std::fs::read_to_string(dir.join(FICHIER)) {
            Ok(t) => serde_json::from_str(&t).map_err(|e| format!("commandes illisibles : {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Stock::default()),
            Err(e) => Err(format!("commandes illisibles : {e}")),
        }
    }

    /// Écrit `commande.json` dans `dir` : un fichier temporaire créé en
    /// `0600`, puis renommé. Le fichier n'existe jamais lisible par
    /// d'autres, même un instant.
    pub fn ecrire(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        // Un nom par écriture : deux écritures ne partagent jamais leur
        // fichier temporaire.
        static RANG: AtomicU64 = AtomicU64::new(0);
        let tmp = dir.join(format!("{FICHIER}.{}.{}.tmp", std::process::id(), RANG.fetch_add(1, Ordering::Relaxed)));
        let texte = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let ecrit = (|| {
            use std::io::Write;
            let mut f = o.open(&tmp)?;
            f.write_all(texte.as_bytes())?;
            drop(f);
            std::fs::rename(&tmp, dir.join(FICHIER))
        })();
        // Un échec ne laisse pas traîner son fichier temporaire.
        ecrit.map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("écriture des commandes : {e}")
        })
    }

    pub fn trouver(&self, id: &str) -> Result<&IntentionGardee, String> {
        self.intentions.iter().find(|i| i.id == id).ok_or_else(|| "commande inconnue".to_string())
    }

    /// Pose l'état relu d'une intention. Un état terminal ne revient jamais
    /// à un état qui bouge : une relecture en retard ne le défait pas.
    pub fn poser(&mut self, id: &str, etat: &str, dernier_code: Option<i64>) {
        if let Some(i) = self.intentions.iter_mut().find(|i| i.id == id) {
            if terminal(&i.etat) && !terminal(etat) {
                return;
            }
            i.etat = etat.to_string();
            if dernier_code.is_some() {
                i.dernier_code = dernier_code;
            }
        }
    }

    /// Ce que la fenêtre a le droit de voir, la plus récente d'abord, et
    /// seulement les intentions du relais compilé : debug et release
    /// partagent ce fichier, et un bundle réel n'a rien à dire d'une
    /// commande du sandbox. Sans relais, rien.
    pub fn vue(&self, mode: Option<Mode>) -> Vec<IntentionVue> {
        self.intentions
            .iter()
            .rev()
            .filter(|i| Some(i.mode) == mode)
            .map(|i| IntentionVue {
                id: i.id.clone(),
                mode: i.mode,
                etat: i.etat.clone(),
                dernier_code: i.dernier_code,
                niveau: i.niveau.clone(),
                date: i.date.clone(),
            })
            .collect()
    }

    /// Oublie une intention, localement. Le relais purge à 24 h ce qui n'a
    /// pas été payé.
    pub fn oublier(&mut self, id: &str) {
        self.intentions.retain(|i| i.id != id);
    }
}

/// Lit, change et réécrit `commande.json` sous `verrou`.
pub fn modifier<T>(verrou: &Mutex<()>, dir: &Path, f: impl FnOnce(&mut Stock) -> Result<T, String>) -> Result<T, String> {
    let _tenu = verrou.lock().unwrap_or_else(|e| e.into_inner());
    let mut s = Stock::lire(dir)?;
    let r = f(&mut s)?;
    s.ecrire(dir)?;
    Ok(r)
}

/// Relit au relais l'intention `id`, ou toutes celles qui peuvent encore
/// bouger, et pose ce qu'il rend. Les appels réseau se font hors du verrou ;
/// l'écriture relit le fichier sous lui et ne touche que l'état et le dernier
/// code des intentions relues : une intention créée pendant ce temps survit.
/// Rend la première erreur de lecture.
/// Seules les intentions du `mode` du relais compilé partent : un secret ne
/// va jamais au relais de l'autre mode.
pub fn relire(
    verrou: &Mutex<()>,
    dir: &Path,
    relais: &Relais<'_>,
    mode: Option<Mode>,
    id: Option<&str>,
) -> Result<Option<String>, String> {
    let a_lire: Vec<IntentionGardee> = {
        let _tenu = verrou.lock().unwrap_or_else(|e| e.into_inner());
        Stock::lire(dir)?
            .intentions
            .into_iter()
            .filter(|i| Some(i.mode) == mode)
            .filter(|i| match id {
                Some(id) => i.id == id,
                None => !terminal(&i.etat) && i.etat != "attente_fichiers",
            })
            .collect()
    };
    let lus: Vec<_> = a_lire.iter().map(|i| (i.id.clone(), relais.etat(&i.id, &i.secret))).collect();
    let mut erreur = None;
    modifier(verrou, dir, |s| {
        for (id, lu) in lus {
            match lu {
                Ok(e) => s.poser(&id, &e.etat, e.dernier_code),
                Err(e) => {
                    erreur.get_or_insert(e.to_string());
                }
            }
        }
        Ok(())
    })?;
    Ok(erreur)
}

/// Pourquoi « Abandonner » est refusé. Les codes voyagent jusqu'à l'écran,
/// qui les dit avec sa phrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusAbandon {
    /// Les fichiers de l'intention partent en ce moment.
    EnvoiEnCours,
    /// Un autre abandon de la même intention attend le relais.
    AbandonEnCours,
    /// Le relais ou Stripe ne répond pas, ou hors contrat : rien ne prouve
    /// que la session ne peut plus être payée.
    Injoignable,
    /// Le relais rend un état payé ou au-delà, ou l'état a bougé ici pendant
    /// l'appel.
    Avancee(String),
    Inconnue,
}

impl std::fmt::Display for RefusAbandon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefusAbandon::EnvoiEnCours => f.write_str("abandon_envoi_en_cours"),
            RefusAbandon::AbandonEnCours => f.write_str("abandon_en_cours"),
            RefusAbandon::Injoignable => f.write_str("abandon_injoignable"),
            RefusAbandon::Avancee(e) => write!(f, "abandon_refuse {e}"),
            RefusAbandon::Inconnue => f.write_str("commande inconnue"),
        }
    }
}

/// « Abandonner » : demande au relais d'abandonner l'intention `id`
/// (`POST /abandon`, qui expire d'abord chez Stripe toute session ouverte),
/// puis l'oublie localement sur son 200 ou son 404. Aucune horloge : seul le
/// relais dit qu'une session ne peut plus être payée. Sur 409, l'intention
/// reste avec l'état relu ; sur 502, une panne ou une réponse hors contrat,
/// elle reste telle quelle. Refusé avant tout appel pendant l'envoi de ses
/// fichiers, et pendant un autre abandon d'elle.
///
/// L'oubli se fait dans la fermeture de [`modifier`], sous le verrou, et
/// seulement si l'état local n'a pas bougé depuis la lecture d'avant l'appel :
/// une relecture qui a posé `payee` entre-temps garde l'intention.
pub fn abandonner(
    verrou: &Mutex<()>,
    dir: &Path,
    relais: &Relais<'_>,
    mode: Option<Mode>,
    envois: &EnCours,
    abandons: &EnCours,
    id: &str,
) -> Result<(), RefusAbandon> {
    let Some(_abandon) = abandons.prendre(id) else {
        return Err(RefusAbandon::AbandonEnCours);
    };
    // Le marqueur d'envoi reste pris jusqu'à l'oubli : aucun envoi ne
    // démarre entre.
    let Some(_envoi) = envois.prendre(id) else {
        return Err(RefusAbandon::EnvoiEnCours);
    };
    let i = {
        let _tenu = verrou.lock().unwrap_or_else(|e| e.into_inner());
        Stock::lire(dir)
            .ok()
            .and_then(|s| s.intentions.into_iter().find(|i| i.id == id && Some(i.mode) == mode))
            .ok_or(RefusAbandon::Inconnue)?
    };
    match relais.abandonner(&i.id, &i.secret) {
        Ok(Abandon::Fait(_) | Abandon::Inconnue) => modifier(verrou, dir, |s| {
            Ok(match s.intentions.iter().find(|x| x.id == id) {
                Some(x) if x.etat != i.etat => Err(RefusAbandon::Avancee(x.etat.clone())),
                _ => {
                    s.oublier(id);
                    Ok(())
                }
            })
        })
        .map_err(|_| RefusAbandon::Injoignable)?,
        Ok(Abandon::Avancee(etat)) => {
            let _ = modifier(verrou, dir, |s| {
                s.poser(id, &etat, None);
                Ok(())
            });
            Err(RefusAbandon::Avancee(etat))
        }
        Ok(Abandon::StripeInjoignable) | Err(_) => Err(RefusAbandon::Injoignable),
    }
}

/// Les intentions sur lesquelles un geste est en cours : l'envoi de leurs
/// fichiers, ou leur abandon. Un second geste de la même sorte sur la même
/// intention est refusé sans rien toucher.
#[derive(Default)]
pub struct EnCours(Mutex<HashSet<String>>);

/// Le marqueur d'un geste en cours ; il se retire en tombant.
pub struct Jeton<'a> {
    en_cours: &'a EnCours,
    id: String,
}

impl EnCours {
    pub fn prendre(&self, id: &str) -> Option<Jeton<'_>> {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        g.insert(id.to_string()).then(|| Jeton { en_cours: self, id: id.to_string() })
    }
}

impl Drop for Jeton<'_> {
    fn drop(&mut self) {
        self.en_cours.0.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.id);
    }
}

/// Ce que la commande écrit au journal. Des phrases fixes : aucune valeur,
/// aucun nom de fichier, rien qu'un rapport de Signaler ne puisse citer.
#[derive(Debug, Clone, Copy)]
pub enum Evenement {
    IntentionCreee,
    FichiersEnvoyes,
    EnvoiEchoue,
    PaiementOuvert,
    PaiementRefuse,
    Annulee,
}

pub fn journal(e: Evenement) {
    colophon_core::log::line(match e {
        Evenement::IntentionCreee => "commande : intention créée au relais",
        Evenement::FichiersEnvoyes => "commande : fichiers envoyés et vérifiés",
        Evenement::EnvoiEchoue => "commande : envoi des fichiers en échec",
        Evenement::PaiementOuvert => "commande : paiement ouvert dans le navigateur",
        Evenement::PaiementRefuse => "commande : adresse de paiement refusée",
        Evenement::Annulee => "commande : annulation demandée",
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use colophon_core::relais::Transport;

    const SECRET: &str = "secret-de-lecture-du-stock-0123456789";

    fn gardee(id: &str, etat: &str) -> IntentionGardee {
        IntentionGardee {
            id: id.into(),
            secret: Secret::new(SECRET),
            mode: Mode::Sandbox,
            date: "2026-10-09".into(),
            etat: etat.into(),
            dernier_code: None,
            niveau: None,
        }
    }

    fn dossier(nom: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("colophon-stock-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn le_stock_s_ecrit_en_0600_et_se_relit() {
        let dir = dossier("0600");
        assert!(Stock::lire(&dir).unwrap().intentions.is_empty(), "absent, le stock est vide");
        let mut s = Stock { intentions: vec![gardee("a", "attente_paiement")] };
        s.poser("a", "commandee", Some(501));
        s.ecrire(&dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(FICHIER)).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let relu = Stock::lire(&dir).unwrap();
        assert_eq!(relu.intentions, s.intentions);
        assert_eq!(relu.intentions[0].secret, Secret::new(SECRET), "le secret survit à la relecture");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn la_fenetre_ne_voit_jamais_le_secret() {
        let s = Stock { intentions: vec![gardee("a", "commandee"), gardee("b", "attente_paiement")] };
        let vue = serde_json::to_string(&s.vue(Some(Mode::Sandbox))).unwrap();
        assert!(!vue.contains(SECRET), "{vue}");
        assert!(vue.find("\"b\"").unwrap() < vue.find("\"a\"").unwrap(), "la plus récente d'abord");
        assert!(!format!("{s:?}").contains(SECRET));
        assert!(!format!("{:?}", s.trouver("a").unwrap()).contains(SECRET));
        assert_eq!(s.trouver("z").unwrap_err(), "commande inconnue");
    }

    #[test]
    fn quatre_etats_sont_terminaux() {
        for e in ["echec_remboursee", "annulee", "purgee", "abandonnee"] {
            assert!(terminal(e), "{e}");
        }
        for e in ["attente_fichiers", "attente_paiement", "payee", "commandee", "remboursement_en_cours", "annulation_en_cours", "inconnu"] {
            assert!(!terminal(e), "{e}");
        }
    }

    #[test]
    fn la_liste_ne_montre_que_le_mode_du_relais_compile() {
        let mut reel = gardee("r", "commandee");
        reel.mode = Mode::Reel;
        let s = Stock { intentions: vec![gardee("s", "commandee"), reel] };
        let ids = |m| s.vue(m).into_iter().map(|v| v.id).collect::<Vec<_>>();
        assert_eq!(ids(Some(Mode::Sandbox)), ["s"]);
        assert_eq!(ids(Some(Mode::Reel)), ["r"]);
        assert!(ids(None).is_empty(), "sans relais, rien");
        // Le gate tourne en debug : le relais compilé est le sandbox.
        assert_eq!(colophon_core::relais::RELAIS_MODE, Some(Mode::Sandbox));
    }

    #[test]
    fn le_niveau_se_garde_et_une_intention_s_oublie() {
        let dir = dossier("niveau");
        let verrou = Mutex::new(());
        modifier(&verrou, &dir, |s| {
            s.intentions.push(gardee("a", "attente_paiement"));
            s.intentions.push(gardee("b", "attente_fichiers"));
            Ok(())
        })
        .unwrap();
        // Un fichier de S-s3 sans niveau se relit.
        assert_eq!(Stock::lire(&dir).unwrap().intentions[0].niveau, None);
        modifier(&verrou, &dir, |s| {
            s.intentions.iter_mut().find(|i| i.id == "a").unwrap().niveau = Some("cp_ground".into());
            s.oublier("b");
            Ok(())
        })
        .unwrap();
        let v = Stock::lire(&dir).unwrap().vue(Some(Mode::Sandbox));
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].niveau.as_deref(), Some("cp_ground"));
        let restes: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(restes, [std::ffi::OsString::from(FICHIER)], "aucun fichier temporaire ne traîne");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Un relais qui tient la relecture : il signale qu'il est entré, puis
    /// attend le feu vert avant de répondre.
    struct Bloque {
        entre: std::sync::mpsc::Sender<()>,
        feu: std::sync::mpsc::Receiver<()>,
    }

    impl Transport for Bloque {
        fn get(&self, _: &str, _: Option<&Secret>) -> Result<(u16, String), String> {
            self.entre.send(()).unwrap();
            self.feu.recv().unwrap();
            Ok((200, r#"{"etat": "commandee", "mode": "sandbox", "dernier_code": 1}"#.into()))
        }
        fn post_json(&self, _: &str, _: Option<&Secret>, _: &serde_json::Value) -> Result<(u16, String), String> {
            unreachable!()
        }
        fn put_fichier(
            &self,
            _: &colophon_core::relais::Envoi<'_>,
            _: &dyn Fn(u64),
        ) -> Result<(u16, Option<String>), String> {
            unreachable!()
        }
    }

    /// Le défaut de l'écriture perdue : une relecture lente croise une
    /// création. Le relais de la relecture est tenu tant que la création
    /// n'est pas écrite, donc l'ordre est forcé, pas espéré.
    #[test]
    fn une_relecture_lente_ne_perd_pas_une_intention_creee_pendant() {
        let dir = dossier("croisee");
        let verrou = Mutex::new(());
        modifier(&verrou, &dir, |s| {
            s.intentions.push(gardee("a", "attente_paiement"));
            Ok(())
        })
        .unwrap();
        let (entre_tx, entre_rx) = std::sync::mpsc::channel();
        let (feu_tx, feu_rx) = std::sync::mpsc::channel();
        let bloque = Bloque { entre: entre_tx, feu: feu_rx };
        std::thread::scope(|sc| {
            let (verrou, dir) = (&verrou, &dir);
            let lecture = sc.spawn(move || {
                let relais = Relais { base: "https://relais.exemple", transport: &bloque };
                relire(verrou, dir, &relais, Some(Mode::Sandbox), None)
            });
            entre_rx.recv().unwrap();
            // Ce que fait `relais_creer`, pendant que la relecture attend.
            modifier(verrou, dir, |s| {
                s.intentions.push(gardee("b", "attente_fichiers"));
                Ok(())
            })
            .unwrap();
            feu_tx.send(()).unwrap();
            assert_eq!(lecture.join().unwrap().unwrap(), None);
        });
        let s = Stock::lire(&dir).unwrap();
        let etats: Vec<(&str, &str, Option<i64>)> =
            s.intentions.iter().map(|i| (i.id.as_str(), i.etat.as_str(), i.dernier_code)).collect();
        assert_eq!(etats, [("a", "commandee", Some(1)), ("b", "attente_fichiers", None)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn un_second_envoi_de_la_meme_intention_est_refuse() {
        let envois = EnCours::default();
        let premier = envois.prendre("a").expect("le premier passe");
        assert!(envois.prendre("a").is_none(), "le second est refusé");
        assert!(envois.prendre("b").is_some(), "une autre intention passe");
        drop(premier);
        assert!(envois.prendre("a").is_some(), "le marqueur tombe avec l'envoi");
    }

    /// Un relais qui rend une réponse fixée, ou rien, et compte ses appels :
    /// les relectures (GET) et les abandons (POST) à part.
    struct Fixe {
        reponse: Result<(u16, String), String>,
        appels: std::cell::Cell<u32>,
        abandons: std::cell::Cell<u32>,
    }

    impl Fixe {
        fn etat(etat: &str) -> Fixe {
            Fixe::rend(Ok((200, format!(r#"{{"etat": "{etat}", "mode": "sandbox"}}"#))))
        }
        fn rend(reponse: Result<(u16, String), String>) -> Fixe {
            Fixe { reponse, appels: std::cell::Cell::new(0), abandons: std::cell::Cell::new(0) }
        }
    }

    impl Transport for Fixe {
        fn get(&self, _: &str, _: Option<&Secret>) -> Result<(u16, String), String> {
            self.appels.set(self.appels.get() + 1);
            self.reponse.clone()
        }
        fn post_json(&self, url: &str, _: Option<&Secret>, _: &serde_json::Value) -> Result<(u16, String), String> {
            assert!(url.ends_with("/v1/intentions/a/abandon"), "{url}");
            self.abandons.set(self.abandons.get() + 1);
            self.reponse.clone()
        }
        fn put_fichier(
            &self,
            _: &colophon_core::relais::Envoi<'_>,
            _: &dyn Fn(u64),
        ) -> Result<(u16, Option<String>), String> {
            unreachable!()
        }
    }

    /// Un stock d'une intention `a` dans l'état local `etat`, et son dossier.
    fn stock_a(nom: &str, etat: &str) -> std::path::PathBuf {
        let dir = dossier(nom);
        Stock { intentions: vec![gardee("a", etat)] }.ecrire(&dir).unwrap();
        dir
    }

    fn abandon_avec(dir: &Path, t: &dyn Transport, envois: &EnCours, abandons: &EnCours) -> Result<(), RefusAbandon> {
        let relais = Relais { base: "https://relais.exemple", transport: t };
        abandonner(&Mutex::new(()), dir, &relais, Some(Mode::Sandbox), envois, abandons, "a")
    }

    fn abandon(dir: &Path, t: &Fixe) -> Result<(), RefusAbandon> {
        abandon_avec(dir, t, &EnCours::default(), &EnCours::default())
    }

    fn reste(dir: &Path) -> Option<String> {
        Stock::lire(dir).unwrap().intentions.into_iter().find(|i| i.id == "a").map(|i| i.etat)
    }

    #[test]
    fn abandonner_demande_au_relais_et_n_oublie_que_sur_200_ou_404() {
        // 200 : la session ne peut plus être payée, l'intention quitte le
        // stock. 404 : le relais ne la connaît plus.
        for (nom, reponse) in [
            ("abandonnee", (200, r#"{"etat": "abandonnee"}"#)),
            ("purgee", (200, r#"{"etat": "purgee"}"#)),
            ("404", (404, r#"{"erreur": "introuvable"}"#)),
        ] {
            let dir = stock_a(&format!("abandon-{nom}"), "attente_paiement");
            let t = Fixe::rend(Ok((reponse.0, reponse.1.into())));
            assert_eq!(abandon(&dir, &t), Ok(()), "{nom}");
            assert_eq!((t.abandons.get(), t.appels.get()), (1, 0), "la route, et aucune relecture");
            assert_eq!(reste(&dir), None, "{nom}");
        }
        // 409 : payée ou au-delà, gardée, avec l'état relu posé.
        let dir = stock_a("abandon-409", "attente_paiement");
        let t = Fixe::rend(Ok((409, r#"{"erreur": "avancee", "etat": "payee"}"#.into())));
        assert_eq!(abandon(&dir, &t), Err(RefusAbandon::Avancee("payee".into())));
        assert_eq!(reste(&dir).as_deref(), Some("payee"));
        // 502, le réseau, tout le reste : rien n'est prouvé, gardée telle quelle.
        for (nom, reponse) in [
            ("502", Ok((502, r#"{"erreur": "stripe_injoignable"}"#.to_string()))),
            ("502-html", Ok((502, "<html>Bad gateway</html>".to_string()))),
            ("500", Ok((500, r#"{"erreur": "interne"}"#.to_string()))),
            ("200-hors-contrat", Ok((200, r#"{"etat": "attente_paiement"}"#.to_string()))),
            ("dns", Err("dns".to_string())),
        ] {
            let dir = stock_a(&format!("abandon-{nom}"), "attente_paiement");
            assert_eq!(abandon(&dir, &Fixe::rend(reponse)), Err(RefusAbandon::Injoignable), "{nom}");
            assert_eq!(reste(&dir).as_deref(), Some("attente_paiement"), "{nom}");
        }
        assert_eq!(RefusAbandon::Injoignable.to_string(), "abandon_injoignable");
        assert_eq!(RefusAbandon::Avancee("payee".into()).to_string(), "abandon_refuse payee");
    }

    #[test]
    fn un_stock_de_s_s3_se_relit_et_la_fenetre_ne_recoit_plus_d_heure() {
        // `paiement_ouvert_le` n'existe plus : un fichier qui le porte se
        // relit, et la réécriture le perd.
        let dir = dossier("s-s3");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(FICHIER),
            format!(
                r#"{{"intentions": [{{"id": "a", "secret": "{SECRET}", "mode": "sandbox", "date": "2026-10-10",
                    "etat": "attente_paiement", "niveau": "cp_ground", "paiement_ouvert_le": "2026-10-10T12:00:00+00:00"}}]}}"#
            ),
        )
        .unwrap();
        let s = Stock::lire(&dir).unwrap();
        assert_eq!(s.intentions[0].niveau.as_deref(), Some("cp_ground"));
        // Le jour de la création voyage (la carte de la commande le dit),
        // jamais une heure.
        let vue = serde_json::to_value(s.vue(Some(Mode::Sandbox))).unwrap();
        assert_eq!(
            vue,
            serde_json::json!([{"id": "a", "mode": "sandbox", "etat": "attente_paiement", "dernier_code": null, "niveau": "cp_ground", "date": "2026-10-10"}])
        );
        s.ecrire(&dir).unwrap();
        assert!(!std::fs::read_to_string(dir.join(FICHIER)).unwrap().contains("paiement_ouvert_le"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn abandonner_est_refuse_pendant_l_envoi_des_fichiers() {
        let envois = EnCours::default();
        let dir = stock_a("abandon-envoi", "attente_fichiers");
        let jeton = envois.prendre("a").unwrap();
        let t = Fixe::rend(Ok((200, r#"{"etat": "abandonnee"}"#.into())));
        assert_eq!(abandon_avec(&dir, &t, &envois, &EnCours::default()), Err(RefusAbandon::EnvoiEnCours));
        assert_eq!(t.abandons.get(), 0);
        assert_eq!(reste(&dir).as_deref(), Some("attente_fichiers"));
        drop(jeton);
        assert_eq!(abandon_avec(&dir, &t, &envois, &EnCours::default()), Ok(()));
    }

    #[test]
    fn un_second_abandon_concurrent_dit_son_propre_refus() {
        let abandons = EnCours::default();
        let envois = EnCours::default();
        let dir = stock_a("abandon-double", "attente_paiement");
        let premier = abandons.prendre("a").unwrap();
        let t = Fixe::rend(Ok((200, r#"{"etat": "abandonnee"}"#.into())));
        let refus = abandon_avec(&dir, &t, &envois, &abandons).unwrap_err();
        assert_eq!(refus, RefusAbandon::AbandonEnCours);
        assert_eq!(refus.to_string(), "abandon_en_cours");
        assert_eq!(t.abandons.get(), 0, "le second ne part pas au relais");
        assert!(envois.prendre("a").is_some(), "il ne tient pas le marqueur d'envoi");
        drop(premier);
        assert_eq!(abandon_avec(&dir, &t, &envois, &abandons), Ok(()));
        assert!(abandons.prendre("a").is_some(), "le marqueur tombe avec l'abandon");
    }

    /// Un relais qui tient l'abandon : il signale qu'il est entré, attend le
    /// feu vert, puis rend 200.
    struct AbandonTenu {
        entre: std::sync::mpsc::Sender<()>,
        feu: std::sync::mpsc::Receiver<()>,
    }

    impl Transport for AbandonTenu {
        fn get(&self, _: &str, _: Option<&Secret>) -> Result<(u16, String), String> {
            unreachable!()
        }
        fn post_json(&self, _: &str, _: Option<&Secret>, _: &serde_json::Value) -> Result<(u16, String), String> {
            self.entre.send(()).unwrap();
            self.feu.recv().unwrap();
            Ok((200, r#"{"etat": "abandonnee"}"#.into()))
        }
        fn put_fichier(
            &self,
            _: &colophon_core::relais::Envoi<'_>,
            _: &dyn Fn(u64),
        ) -> Result<(u16, Option<String>), String> {
            unreachable!()
        }
    }

    /// L'oubli se fait sous le verrou, et seulement si l'intention n'a pas
    /// changé d'état depuis l'appel. Le relais est tenu pendant que ce test
    /// prend le verrou : l'abandon ne peut pas finir tant qu'il le tient, et
    /// ce qui s'écrit pendant (une relecture qui pose `payee`) est vu.
    #[test]
    fn l_oubli_se_fait_sous_le_verrou_et_jamais_sur_un_etat_qui_a_bouge() {
        let dir = stock_a("abandon-verrou", "attente_paiement");
        let verrou = Mutex::new(());
        let (entre_tx, entre_rx) = std::sync::mpsc::channel();
        let (feu_tx, feu_rx) = std::sync::mpsc::channel();
        let tenu = AbandonTenu { entre: entre_tx, feu: feu_rx };
        std::thread::scope(|sc| {
            let (verrou, dir) = (&verrou, &dir);
            let abandon = sc.spawn(move || {
                let relais = Relais { base: "https://relais.exemple", transport: &tenu };
                abandonner(verrou, dir, &relais, Some(Mode::Sandbox), &EnCours::default(), &EnCours::default(), "a")
            });
            entre_rx.recv().unwrap();
            let garde = verrou.lock().unwrap();
            feu_tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(!abandon.is_finished(), "l'oubli n'attend pas le verrou");
            let mut s = Stock::lire(dir).unwrap();
            s.poser("a", "payee", None);
            s.ecrire(dir).unwrap();
            drop(garde);
            assert_eq!(abandon.join().unwrap(), Err(RefusAbandon::Avancee("payee".into())));
        });
        assert_eq!(reste(&dir).as_deref(), Some("payee"), "gardée");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn la_relecture_n_envoie_jamais_un_secret_au_relais_de_l_autre_mode() {
        let dir = dossier("relire-mode");
        let mut reel = gardee("r", "attente_paiement");
        reel.mode = Mode::Reel;
        Stock { intentions: vec![gardee("s", "attente_paiement"), reel] }.ecrire(&dir).unwrap();
        let t = Fixe::etat("payee");
        let relais = Relais { base: "https://relais.exemple", transport: &t };
        let verrou = Mutex::new(());
        relire(&verrou, &dir, &relais, Some(Mode::Sandbox), None).unwrap();
        assert_eq!(t.appels.get(), 1, "seule l'intention du sandbox part");
        relire(&verrou, &dir, &relais, Some(Mode::Sandbox), Some("r")).unwrap();
        assert_eq!(t.appels.get(), 1, "même nommée, celle du réel ne part pas");
        relire(&verrou, &dir, &relais, None, None).unwrap();
        assert_eq!(t.appels.get(), 1, "sans relais, rien ne part");
        let etats: Vec<String> = Stock::lire(&dir).unwrap().intentions.into_iter().map(|i| i.etat).collect();
        assert_eq!(etats, ["payee", "attente_paiement"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn un_etat_terminal_ne_revient_jamais_en_arriere() {
        for terminal in ["annulee", "echec_remboursee", "purgee"] {
            let mut s = Stock { intentions: vec![gardee("a", terminal)] };
            s.poser("a", "commandee", Some(15));
            assert_eq!(s.intentions[0].etat, terminal);
            s.poser("a", "purgee", None);
            assert_eq!(s.intentions[0].etat, "purgee", "d'un terminal à un autre, oui");
        }
        let mut s = Stock { intentions: vec![gardee("a", "commandee")] };
        s.poser("a", "annulee", None);
        assert_eq!(s.intentions[0].etat, "annulee");
    }

    /// Le verrou de `modifier` est tenu pendant toute la modification, ce qui
    /// se voit sans dépendre d'un ordonnancement : `try_lock` échoue dedans.
    /// Puis deux écritures croisées, la seconde lancée pendant que la
    /// première tient son cliché : sans verrou, la première écrase la
    /// seconde (100 ms laissent à la seconde le temps de finir) ; avec, la
    /// seconde attend, et rien ne se perd quel que soit l'ordonnancement.
    #[test]
    fn modifier_tient_son_verrou_et_deux_ecritures_croisees_ne_perdent_rien() {
        let dir = dossier("verrou");
        let verrou = Mutex::new(());
        modifier(&verrou, &dir, |_| {
            assert!(verrou.try_lock().is_err(), "le verrou est tenu pendant la modification");
            Ok(())
        })
        .unwrap();
        let (dedans_tx, dedans_rx) = std::sync::mpsc::channel();
        let (feu_tx, feu_rx) = std::sync::mpsc::channel::<()>();
        std::thread::scope(|sc| {
            let (verrou, dir) = (&verrou, &dir);
            let premiere = sc.spawn(move || {
                modifier(verrou, dir, |s| {
                    dedans_tx.send(()).unwrap();
                    feu_rx.recv().unwrap();
                    s.intentions.push(gardee("a", "attente_fichiers"));
                    Ok(())
                })
            });
            dedans_rx.recv().unwrap();
            let seconde = sc.spawn(move || {
                modifier(verrou, dir, |s| {
                    s.intentions.push(gardee("b", "attente_fichiers"));
                    Ok(())
                })
            });
            std::thread::sleep(std::time::Duration::from_millis(100));
            feu_tx.send(()).unwrap();
            premiere.join().unwrap().unwrap();
            seconde.join().unwrap().unwrap();
        });
        let mut ids: Vec<String> = Stock::lire(&dir).unwrap().intentions.into_iter().map(|i| i.id).collect();
        ids.sort();
        assert_eq!(ids, ["a", "b"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn une_ecriture_ratee_ne_laisse_pas_de_fichier_temporaire() {
        let dir = dossier("ecriture-ratee");
        // Le fichier final est un dossier non vide : le `rename` échoue.
        std::fs::create_dir_all(dir.join(FICHIER).join("plein")).unwrap();
        assert!(Stock::default().ecrire(&dir).is_err());
        let restes: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(restes, [std::ffi::OsString::from(FICHIER)], "seul le dossier piège reste");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
