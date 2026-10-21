//! Le client du relais marchand (S-s3) : ce que l'app demande au relais
//! d'Alexis pour commander un album, et rien d'autre.
//!
//! **L'app ne tient aucune clé.** Le relais devise, encaisse par Stripe et
//! commande chez Cloudprinter avec ses propres secrets. L'app ne reçoit, par
//! intention de commande, qu'un identifiant et un **secret de lecture**
//! ([`Secret`]) qui part dans `Authorization: Bearer`, jamais dans une URL.
//!
//! **Tout passe par le trait [`Transport`]**, qu'un faux remplace en test :
//! aucun test du gate ne touche le réseau. Il est écrit sur le modèle de
//! [`crate::commande::Http`], qui ne sait poser ni en-tête ni `GET` ni un
//! fichier en flux, et que la CLI garde tel quel.
//!
//! **Les deux PDF montent directement au dépôt**, par les deux URL PUT que la
//! création d'intention rend, avec les en-têtes signés qu'elle rend à côté
//! (dont `Content-MD5`, que R2 vérifie). Un seul PUT par fichier, en flux,
//! avec sa longueur annoncée : c'est ce qui fait que l'ETag rendu est le MD5,
//! et on le compare.

use crate::export::{md5_du_fichier, LIVRAISON_COUVERTURE, LIVRAISON_INTERIEUR};
pub use crate::prevol::{PageSousSeuil, QuelFichier};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::time::Duration;

/// L'adresse du relais sandbox.
const SANDBOX: &str = "https://colophon-relais-sandbox.alexis-c1f.workers.dev";

/// L'adresse du relais pour un build de debug ou de release : le sandbox en
/// debug ; en release, le sandbox si la compilation l'a demandé (`sandbox`,
/// le produit final qu'on teste à la carte de test), sinon le relais réel
/// `reel` qu'elle a reçu, et **rien** sans l'un ni l'autre. Une fonction
/// plutôt que des `cfg` : le gate tourne en debug, et c'est ainsi qu'un test
/// y tient aussi la valeur de la release.
pub const fn url_pour(debug: bool, sandbox: bool, reel: Option<&'static str>) -> Option<&'static str> {
    if debug || sandbox {
        Some(SANDBOX)
    } else {
        reel
    }
}

/// Le mode des intentions que ce relais rend : sandbox pour le sandbox, réel
/// pour le relais réel, aucun sans relais. L'app ne liste que celles-là :
/// debug et release partagent `commande.json`, et une intention du sandbox
/// rangée sous un autre mode sortirait de la liste dès sa création.
pub const fn mode_pour(debug: bool, sandbox: bool, reel: Option<&'static str>) -> Option<Mode> {
    match url_pour(debug, sandbox, reel) {
        None => None,
        Some(_) if debug || sandbox => Some(Mode::Sandbox),
        Some(_) => Some(Mode::Reel),
    }
}

/// `COLOPHON_RELAIS_SANDBOX` : absent, ou `1` exactement. Toute autre valeur
/// arrête le build, comme une adresse réelle refusée : un `0` qui
/// brancherait quand même le sandbox serait le piège.
pub const fn sandbox_valide(v: Option<&'static str>) -> Result<bool, &'static str> {
    match v {
        None => Ok(false),
        Some(v) if v.len() == 1 && v.as_bytes()[0] == b'1' => Ok(true),
        Some(_) => Err("COLOPHON_RELAIS_SANDBOX : 1, ou ne pas la poser"),
    }
}

/// Un seul relais par build : le sandbox et le réel à la fois arrêtent la
/// compilation, plutôt que de laisser l'un gagner en silence.
pub const fn relais_unique(sandbox: bool, reel: Option<&'static str>) -> Result<(), &'static str> {
    if sandbox && reel.is_some() {
        Err("COLOPHON_RELAIS_SANDBOX et COLOPHON_RELAIS_REEL : l'un ou l'autre")
    } else {
        Ok(())
    }
}

/// Une adresse de relais réel : `https://` puis un hôte seul, sans chemin
/// (pas même `/`), port, identifiants, requête ni fragment ; des étiquettes
/// non vides de lettres, chiffres et tirets. Une fonction `const` : la
/// compilation la juge, et une adresse refusée arrête le build ([`REEL`]).
pub const fn reel_valide(reel: Option<&'static str>) -> Result<Option<&'static str>, &'static str> {
    let Some(url) = reel else {
        return Ok(None);
    };
    let o = url.as_bytes();
    let prefixe = b"https://";
    if o.len() <= prefixe.len() {
        return Err("COLOPHON_RELAIS_REEL : https:// suivi d'un hôte");
    }
    let mut i = 0;
    while i < prefixe.len() {
        if o[i] != prefixe[i] {
            return Err("COLOPHON_RELAIS_REEL : https:// suivi d'un hôte");
        }
        i += 1;
    }
    // `i` est au premier octet de l'hôte. Un point ne commence ni ne finit
    // l'hôte, et ne suit pas un autre point.
    let mut precedent_point = true;
    while i < o.len() {
        let c = o[i];
        if c == b'.' {
            if precedent_point {
                return Err("COLOPHON_RELAIS_REEL : étiquette vide dans l'hôte");
            }
            precedent_point = true;
        } else if c.is_ascii_alphanumeric() || c == b'-' {
            precedent_point = false;
        } else {
            return Err("COLOPHON_RELAIS_REEL : un hôte seul, sans chemin, port, identifiants ni requête");
        }
        i += 1;
    }
    if precedent_point {
        return Err("COLOPHON_RELAIS_REEL : étiquette vide dans l'hôte");
    }
    Ok(Some(url))
}

/// Le relais réel que la compilation a reçu par `COLOPHON_RELAIS_REEL`, jugé
/// par [`reel_valide`] : une adresse refusée fait échouer le build, ici, à
/// l'évaluation de la constante. La CI publique ne la pose pas : les builds
/// publiés n'ont aucun relais. Cargo relit la variable à chaque build
/// (`option_env!` entre dans les dépendances du crate) : la changer
/// recompile le moteur.
pub const REEL: Option<&str> = match reel_valide(option_env!("COLOPHON_RELAIS_REEL")) {
    Ok(r) => r,
    Err(raison) => panic!("{}", raison),
};

/// Une release branchée sur le sandbox, par `COLOPHON_RELAIS_SANDBOX=1` à la
/// compilation : le produit final à tester, paiement à la carte de test. La
/// CI publique ne la pose pas plus que le relais réel.
pub const SANDBOX_EN_RELEASE: bool = match sandbox_valide(option_env!("COLOPHON_RELAIS_SANDBOX")) {
    Ok(s) => match relais_unique(s, REEL) {
        Ok(()) => s,
        Err(raison) => panic!("{}", raison),
    },
    Err(raison) => panic!("{}", raison),
};

/// L'adresse du relais compilé (décision 3). Sans adresse, l'app n'offre pas
/// Commander.
pub const RELAIS_URL: Option<&str> = url_pour(cfg!(debug_assertions), SANDBOX_EN_RELEASE, REEL);

/// Le mode du relais compilé.
pub const RELAIS_MODE: Option<Mode> = mode_pour(cfg!(debug_assertions), SANDBOX_EN_RELEASE, REEL);

/// Les seuls en-têtes de la réponse de création que le PUT recopie. Le relais
/// n'est pas cru sur parole : `Host`, `Authorization`, `Cookie`,
/// `Transfer-Encoding`, `Content-Length` ou tout autre nom est ignoré.
const ENTETES_PERMIS: [&str; 2] = ["content-md5", "content-type"];

/// L'hôte de tout dépôt R2.
const HOTE_R2: &str = ".r2.cloudflarestorage.com";

/// Le seul papier intérieur que le relais imprime (`produits.ts` du relais) :
/// celui dont la couverture du dossier préparé porte le dos. Un dossier
/// préparé pour un autre papier ne se commande pas.
pub const PAPIER: &str = "pageblock_150mcs";

/// Une progression émise au plus tous les `PAS_DE_PROGRESSION` octets.
const PAS_DE_PROGRESSION: u64 = 1 << 20;

// ---- le secret ---------------------------------------------------------------

/// Le secret de lecture d'une intention. Il ouvre l'état, le paiement et
/// l'annulation de sa commande : `Debug` et `Display` rendent `***`. Il se
/// sérialise, parce que `commande.json` doit le garder, et c'est le seul
/// fichier qui le reçoit ; ce que la fenêtre voit n'en porte aucun champ.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(valeur: impl Into<String>) -> Self {
        Secret(valeur.into())
    }

    /// Retire le secret d'un texte venu d'ailleurs avant qu'il entre dans une
    /// [`Erreur`], sur le modèle de `Cle::masquer`.
    fn masquer(&self, texte: &str) -> String {
        if self.0.is_empty() {
            texte.to_string()
        } else {
            texte.replace(&self.0, "***")
        }
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

// ---- le transport ------------------------------------------------------------

/// Un fichier à poser par PUT : où, avec quels en-têtes signés, et combien
/// d'octets il fait.
pub struct Envoi<'a> {
    pub url: &'a str,
    pub entetes: &'a BTreeMap<String, String>,
    pub chemin: &'a Path,
    pub octets: u64,
}

/// La seule porte vers le réseau. Un statut HTTP, même 4xx, n'est pas une
/// erreur à ce niveau ; une erreur de transport est un texte.
pub trait Transport {
    /// Un GET, rendu en statut et corps.
    fn get(&self, url: &str, bearer: Option<&Secret>) -> Result<(u16, String), String>;
    /// Un POST JSON, rendu en statut et corps.
    fn post_json(&self, url: &str, bearer: Option<&Secret>, corps: &Value) -> Result<(u16, String), String>;
    /// Un PUT du fichier en flux, `Content-Length` annoncé. Rend le statut et
    /// l'ETag. `progression` reçoit les octets déjà partis.
    fn put_fichier(&self, envoi: &Envoi<'_>, progression: &dyn Fn(u64)) -> Result<(u16, Option<String>), String>;
}

/// Le transport réel, synchrone, TLS par rustls : le même `ureq` que
/// `commande` et `depot`.
pub struct Reseau {
    /// Les appels JSON : soixante secondes en tout.
    json: ureq::Agent,
    /// Les PUT : pas de délai global, 95 Mo sur une ligne lente dépassent
    /// n'importe quel chiffre qu'on choisirait. La connexion se borne.
    envoi: ureq::Agent,
}

impl Default for Reseau {
    fn default() -> Self {
        let json = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(60)))
            .build()
            .into();
        let envoi = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(30)))
            .build()
            .into();
        Reseau { json, envoi }
    }
}

/// Lit un fichier en signalant les octets lus, par paliers, sur un canal :
/// le corps part dans le fil de l'envoi.
struct Compteur<R> {
    lecteur: R,
    lus: u64,
    signale: u64,
    /// La longueur annoncée : le transport s'arrête dessus sans demander la
    /// lecture vide qui dirait la fin.
    total: u64,
    canal: std::sync::mpsc::Sender<u64>,
}

impl<R: std::io::Read> std::io::Read for Compteur<R> {
    fn read(&mut self, tampon: &mut [u8]) -> std::io::Result<usize> {
        let n = self.lecteur.read(tampon)?;
        self.lus += n as u64;
        if self.lus != self.signale && (self.lus - self.signale >= PAS_DE_PROGRESSION || self.lus >= self.total || n == 0) {
            self.signale = self.lus;
            let _ = self.canal.send(self.lus);
        }
        Ok(n)
    }
}

fn texte_de(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<(u16, String), String> {
    let mut r = r.map_err(|e| e.to_string())?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok((statut, texte))
}

impl Transport for Reseau {
    fn get(&self, url: &str, bearer: Option<&Secret>) -> Result<(u16, String), String> {
        let mut req = self.json.get(url);
        if let Some(s) = bearer {
            req = req.header("Authorization", format!("Bearer {}", s.0));
        }
        texte_de(req.call())
    }

    fn post_json(&self, url: &str, bearer: Option<&Secret>, corps: &Value) -> Result<(u16, String), String> {
        let mut req = self.json.post(url).header("Content-Type", "application/json");
        if let Some(s) = bearer {
            req = req.header("Authorization", format!("Bearer {}", s.0));
        }
        texte_de(req.send(corps.to_string()))
    }

    fn put_fichier(&self, envoi: &Envoi<'_>, progression: &dyn Fn(u64)) -> Result<(u16, Option<String>), String> {
        let fichier = std::fs::File::open(envoi.chemin).map_err(|e| e.to_string())?;
        // Le compteur est possédé par le corps, qui part dans un autre fil :
        // les octets lus reviennent par un canal, relevé ici pendant l'envoi.
        let (canal, rx) = std::sync::mpsc::channel::<u64>();
        let corps = Compteur { lecteur: fichier, lus: 0, signale: 0, total: envoi.octets, canal };
        let mut req = self.envoi.put(envoi.url).header("Content-Length", envoi.octets.to_string());
        for (nom, valeur) in envoi.entetes {
            req = req.header(nom.as_str(), valeur.as_str());
        }
        let r = std::thread::scope(|s| {
            let envoi = s.spawn(move || req.send(ureq::SendBody::from_owned_reader(corps)));
            // Le corps meurt avec la requête : le canal se ferme, le relevé
            // finit.
            for n in rx.iter() {
                progression(n);
            }
            envoi.join().map_err(|_| "envoi interrompu".to_string())
        })?;
        let mut r = r.map_err(|e| e.to_string())?;
        let statut = r.status().as_u16();
        let etag = r.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_string);
        let _ = r.body_mut().read_to_string();
        Ok((statut, etag))
    }
}

// ---- les erreurs -------------------------------------------------------------

/// Ce qui empêche un geste. Aucune ne recopie le secret ni une URL de dépôt :
/// tout texte venu de dehors passe par un masque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Erreur {
    /// Le relais répond hors 2xx, avec le champ `erreur` de son corps, et
    /// rien d'autre (`illisible` quand le corps ne le porte pas).
    Relais { statut: u16, erreur: String },
    /// Le réseau, avant tout statut.
    Reseau(String),
    /// Un corps 2xx que l'app ne sait pas lire.
    Reponse(String),
    /// Le dépôt refuse le PUT d'un fichier.
    Depot { fichier: &'static str, statut: u16 },
    /// L'ETag que le dépôt rend n'est pas le MD5 du fichier envoyé.
    Etag { fichier: &'static str, md5: String, etag: Option<String> },
    /// Un fichier du dossier préparé ne se lit pas.
    Fichier(String),
    /// Le dossier préparé est fait pour un autre papier que celui du relais.
    Papier,
    /// L'URL de dépôt rendue n'est pas une URL R2 en https, sans port ni
    /// identifiants. Rien n'est envoyé.
    UrlDeDepot { fichier: &'static str },
    /// Des pages sont sous le plancher et la case qui les nomme n'est pas
    /// cochée.
    AcquittementManquant,
    /// Plus de pages sous le plancher que le relais n'en acquitte.
    AcquittementTropLong(usize),
}

impl Erreur {
    /// Le code du relais, quand c'est lui qui refuse : `trop_tard`, `pays`…
    pub fn code(&self) -> Option<&str> {
        match self {
            Erreur::Relais { erreur, .. } => Some(erreur),
            _ => None,
        }
    }
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Relais { statut, erreur } => write!(f, "relais : {erreur} ({statut})"),
            Erreur::Reseau(e) => write!(f, "relais injoignable : {e}"),
            Erreur::Reponse(e) => write!(f, "réponse du relais illisible : {e}"),
            Erreur::Depot { fichier, statut } => write!(f, "le dépôt refuse {fichier} : statut {statut}"),
            Erreur::Etag { fichier, md5, etag } => write!(
                f,
                "le dépôt rend pour {fichier} l'ETag {}, pas le MD5 {md5}",
                etag.as_deref().unwrap_or("(aucun)")
            ),
            Erreur::Fichier(e) => write!(f, "dossier préparé illisible : {e}"),
            Erreur::Papier => write!(f, "ce dossier est préparé pour un autre papier que celui du relais ({PAPIER})"),
            Erreur::UrlDeDepot { fichier } => write!(f, "le relais rend pour {fichier} une adresse de dépôt refusée"),
            Erreur::AcquittementManquant => {
                write!(f, "des pages sont sous {SEUIL_PPI} ppi : cochez la case qui les nomme pour commander")
            }
            Erreur::AcquittementTropLong(n) => write!(
                f,
                "{n} pages sont sous {SEUIL_PPI} ppi, au-delà des {ACQUITTEMENT_MAX} qu'une commande peut acquitter"
            ),
        }
    }
}

impl std::error::Error for Erreur {}

// ---- ce que le relais rend ---------------------------------------------------

/// Sandbox ou réel, déclaré par le relais dans chaque réponse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Sandbox,
    Reel,
}

/// Un État ou une région, quand Cloudprinter en exige un dans l'adresse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatRegion {
    pub code: String,
    pub nom: String,
}

/// Un pays ouvert à la commande.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pays {
    pub code: String,
    pub nom: String,
    pub require_state: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub etats: Vec<EtatRegion>,
}

/// Un niveau d'expédition de la grille : un prix de vente TTC, en centimes,
/// livraison comprise. Jamais le prix de Cloudprinter, jamais la marge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Niveau {
    pub niveau: String,
    pub transporteur: Option<String>,
    pub prix_ttc_centimes: u32,
    pub delai_jours: Option<u32>,
}

/// Une URL PUT et les en-têtes qu'elle a signés. L'URL porte une signature :
/// son `Debug` la tait, et elle ne quitte jamais le moteur.
#[derive(Clone, Deserialize)]
pub struct Televersement {
    url: String,
    #[serde(default)]
    entetes: BTreeMap<String, String>,
}

impl fmt::Debug for Televersement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Televersement").field("url", &"***").field("entetes", &self.entetes).finish()
    }
}

/// Les deux URL de dépôt d'une intention.
#[derive(Debug, Clone, Deserialize)]
pub struct Televersements {
    pub interieur: Televersement,
    pub couverture: Televersement,
}

/// Une intention de commande, telle que sa création la rend.
#[derive(Debug, Clone, Deserialize)]
pub struct Intention {
    pub id: String,
    pub secret: Secret,
    pub mode: Mode,
    pub televersements: Televersements,
    pub grille: Vec<Niveau>,
    pub expire: String,
}

/// L'état d'une intention, relu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Etat {
    /// `attente_fichiers`, `attente_paiement`, `payee`, `commandee`… Une
    /// chaîne et pas une `enum` : un état que l'app ne connaît pas encore se
    /// lit quand même, et l'écran le dit d'une phrase neutre.
    pub etat: String,
    pub mode: Mode,
    /// Le numéro d'état de Cloudprinter, quand la commande est passée.
    #[serde(default)]
    pub dernier_code: Option<i64>,
}

/// Ce que `POST /v1/intentions/{id}/abandon` rend, réponse par réponse du
/// contrat. Tout le reste est une [`Erreur`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Abandon {
    /// 200 : chaque session Checkout de l'intention est expirée chez Stripe,
    /// elle ne peut plus être payée. L'état rendu, `abandonnee` ou `purgee`.
    Fait(String),
    /// 409 `avancee` : payée ou au-delà, avec l'état que le relais a relu.
    Avancee(String),
    /// 502 `stripe_injoignable` : Stripe ne répond pas, rien n'est prouvé.
    StripeInjoignable,
    /// 404 : intention inconnue du relais (ou secret faux).
    Inconnue,
}

/// Un état venu du relais, tel qu'il peut entrer dans `commande.json` et
/// dans une phrase : court, en minuscules ASCII et tirets bas.
fn etat_lisible(etat: &str) -> bool {
    !etat.is_empty() && etat.len() <= 64 && etat.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

// ---- ce que l'app demande ----------------------------------------------------

/// Un fichier déclaré : son MD5 et sa taille.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fichier {
    pub md5: String,
    pub octets: u64,
}

/// Ce qu'une intention chiffre, lu du dossier préparé. Pas de papier : le
/// relais le fixe lui-même, et refuse une clé qu'il ne connaît pas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Demande {
    pub format: String,
    pub pages: u32,
    pub pays: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etat_region: Option<String>,
    pub interieur: Fichier,
    pub couverture: Fichier,
    /// Les pages sous le plancher que l'auteur a acquittées en cochant la
    /// case (1.1-c). Absent quand il n'y en a pas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquittement: Option<Acquittement>,
}

/// La version du libellé de la case, archivé mot pour mot dans
/// `docs/acquittement-v1.md`. Changer un mot du libellé, c'est changer de
/// version.
pub const ACQUITTEMENT_VERSION: u32 = 1;

/// Le plancher que le relais acquitte, le sien : le `min_ppi` de Cloudprinter.
pub const SEUIL_PPI: u32 = 250;

/// Le relais refuse un acquittement de plus de pages.
pub const ACQUITTEMENT_MAX: usize = 200;

/// Ce que l'auteur a reconnu en cochant : des pages du livre et leur ppi, le
/// seuil, l'heure. Construit ici depuis le rapport de prévol relu sur le
/// dossier préparé, jamais depuis la fenêtre, qui n'envoie qu'un booléen :
/// aucun nom de photo, aucun chemin, aucune légende ne peut y entrer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Acquittement {
    pub version: u32,
    pub seuil_ppi: u32,
    /// UTC à la seconde, `AAAA-MM-JJTHH:MM:SSZ` : l'heure où la déclaration
    /// cochée part au relais.
    pub coche_le: String,
    /// `[page, ppi]`, triées.
    pub interieur: Vec<[u32; 2]>,
    pub couverture: Vec<[u32; 2]>,
}

/// L'acquittement des `pages`, quand il y en a. Sans pages, rien, case cochée
/// ou non. Avec, la case est exigée, et au-delà de [`ACQUITTEMENT_MAX`] la
/// commande se refuse ici plutôt qu'au relais.
pub fn acquittement_pour(
    pages: &[PageSousSeuil],
    seuil_ppi: u32,
    acquitte: bool,
    coche_le: &str,
) -> Result<Option<Acquittement>, Erreur> {
    if pages.is_empty() {
        return Ok(None);
    }
    if !acquitte {
        return Err(Erreur::AcquittementManquant);
    }
    if pages.len() > ACQUITTEMENT_MAX {
        return Err(Erreur::AcquittementTropLong(pages.len()));
    }
    let de = |q: QuelFichier| pages.iter().filter(|p| p.fichier == q).map(|p| [p.page, p.ppi]).collect();
    Ok(Some(Acquittement {
        version: ACQUITTEMENT_VERSION,
        seuil_ppi,
        coche_le: coche_le.to_string(),
        interieur: de(QuelFichier::Interieur),
        couverture: de(QuelFichier::Couverture),
    }))
}

/// L'heure présente, dans la forme que le relais accepte pour `coche_le`.
pub fn coche_le_maintenant() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

impl Demande {
    /// La demande pour le dossier préparé `dossier` : les MD5 et tailles de
    /// ses deux fichiers, lus une fois. `papier` est celui du profil qui l'a
    /// préparé ; un autre que [`PAPIER`] est refusé avant tout appel.
    pub fn du_dossier(
        dossier: &Path,
        papier: Option<&str>,
        format: &str,
        pages: u32,
        pays: &str,
        etat_region: Option<String>,
    ) -> Result<Demande, Erreur> {
        if papier != Some(PAPIER) {
            return Err(Erreur::Papier);
        }
        let fichier = |nom: &str| -> Result<Fichier, Erreur> {
            let chemin = dossier.join(nom);
            let octets = std::fs::metadata(&chemin).map_err(|e| Erreur::Fichier(format!("{nom} : {e}")))?.len();
            let md5 = md5_du_fichier(&chemin).map_err(|e| Erreur::Fichier(format!("{nom} : {e:#}")))?;
            Ok(Fichier { md5, octets })
        };
        Ok(Demande {
            format: format.to_string(),
            pages,
            pays: pays.to_string(),
            etat_region,
            interieur: fichier(LIVRAISON_INTERIEUR)?,
            couverture: fichier(LIVRAISON_COUVERTURE)?,
            acquittement: None,
        })
    }
}

/// Lequel des deux fichiers, pour la progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Quel {
    Interieur,
    Couverture,
}

// ---- les appels ----------------------------------------------------------------

/// Le relais à l'adresse `base`, et le transport qui le joint.
pub struct Relais<'a> {
    pub base: &'a str,
    pub transport: &'a dyn Transport,
}

/// Le champ `erreur` d'un corps de refus, et rien d'autre.
fn code_d_erreur(texte: &str) -> String {
    serde_json::from_str::<Value>(texte)
        .ok()
        .and_then(|v| v["erreur"].as_str().map(str::to_string))
        .filter(|e| !e.is_empty() && e.len() <= 64 && e.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or_else(|| "illisible".to_string())
}

fn lire<T: serde::de::DeserializeOwned>(texte: &str, secret: Option<&Secret>) -> Result<T, Erreur> {
    serde_json::from_str(texte).map_err(|e| {
        let m = e.to_string();
        Erreur::Reponse(secret.map_or(m.clone(), |s| s.masquer(&m)))
    })
}

impl Relais<'_> {
    fn url(&self, chemin: &str) -> String {
        format!("{}{chemin}", self.base.trim_end_matches('/'))
    }

    /// Rend le corps d'un 2xx, ou l'erreur typée de tout autre statut.
    fn verdict(r: Result<(u16, String), String>, secret: Option<&Secret>) -> Result<String, Erreur> {
        let masque = |t: &str| secret.map_or(t.to_string(), |s| s.masquer(t));
        let (statut, texte) = r.map_err(|e| Erreur::Reseau(masque(&e)))?;
        if (200..300).contains(&statut) {
            Ok(texte)
        } else {
            Err(Erreur::Relais { statut, erreur: masque(&code_d_erreur(&texte)) })
        }
    }

    fn get(&self, chemin: &str, secret: Option<&Secret>) -> Result<String, Erreur> {
        Self::verdict(self.transport.get(&self.url(chemin), secret), secret)
    }

    fn post(&self, chemin: &str, secret: Option<&Secret>, corps: &Value) -> Result<String, Erreur> {
        Self::verdict(self.transport.post_json(&self.url(chemin), secret, corps), secret)
    }

    /// `GET /v1/pays` : les pays ouverts à la commande.
    pub fn pays(&self) -> Result<Vec<Pays>, Erreur> {
        #[derive(Deserialize)]
        struct R {
            pays: Vec<Pays>,
        }
        Ok(lire::<R>(&self.get("/v1/pays", None)?, None)?.pays)
    }

    /// `POST /v1/intentions` : la grille des prix, les deux URL de dépôt, et
    /// l'identifiant et le secret de l'intention.
    pub fn creer(&self, demande: &Demande) -> Result<Intention, Erreur> {
        let corps = serde_json::to_value(demande).map_err(|e| Erreur::Reponse(e.to_string()))?;
        let texte = self.post("/v1/intentions", None, &corps)?;
        // Le corps porte le secret : une erreur de lecture ne doit pas le
        // recopier, et on ne le connaît qu'une fois lu.
        serde_json::from_str(&texte).map_err(|_| Erreur::Reponse("intention".into()))
    }

    /// `POST /v1/intentions/{id}/fichiers` : le relais vérifie et fige les
    /// deux objets déposés. Rend le nouvel état.
    pub fn fichiers(&self, id: &str, secret: &Secret) -> Result<String, Erreur> {
        #[derive(Deserialize)]
        struct R {
            etat: String,
        }
        let texte = self.post(&format!("/v1/intentions/{id}/fichiers"), Some(secret), &json!({}))?;
        Ok(lire::<R>(&texte, Some(secret))?.etat)
    }

    /// `POST /v1/intentions/{id}/paiement` : l'URL de la session Checkout du
    /// niveau choisi. L'app la garde (`ouvrir_paiement`) et ne l'envoie qu'au
    /// navigateur du système.
    pub fn paiement(&self, id: &str, secret: &Secret, niveau: &str) -> Result<String, Erreur> {
        #[derive(Deserialize)]
        struct R {
            url: String,
        }
        let texte = self.post(&format!("/v1/intentions/{id}/paiement"), Some(secret), &json!({ "niveau": niveau }))?;
        Ok(lire::<R>(&texte, Some(secret))?.url)
    }

    /// `GET /v1/intentions/{id}` : l'état, le mode, le dernier code.
    pub fn etat(&self, id: &str, secret: &Secret) -> Result<Etat, Erreur> {
        lire(&self.get(&format!("/v1/intentions/{id}"), Some(secret))?, Some(secret))
    }

    /// `POST /v1/intentions/{id}/annulation`. Rend le nouvel état
    /// (`annulee`, ou `annulation_en_cours` quand le remboursement attend) ;
    /// trop tard, le relais répond 409 `trop_tard`.
    pub fn annuler(&self, id: &str, secret: &Secret) -> Result<String, Erreur> {
        #[derive(Deserialize)]
        struct R {
            etat: String,
        }
        let texte = self.post(&format!("/v1/intentions/{id}/annulation"), Some(secret), &json!({}))?;
        Ok(lire::<R>(&texte, Some(secret))?.etat)
    }

    /// `POST /v1/intentions/{id}/abandon` : le relais expire chez Stripe
    /// toute session ouverte de l'intention, puis l'abandonne. Les quatre
    /// réponses du contrat deviennent un [`Abandon`] ; un 200 qui ne dit ni
    /// `abandonnee` ni `purgee`, un 409 qui n'est pas `avancee` avec un état
    /// lisible, tout autre statut, sont des erreurs : rien n'y permet l'oubli.
    pub fn abandonner(&self, id: &str, secret: &Secret) -> Result<Abandon, Erreur> {
        #[derive(Deserialize)]
        struct R {
            etat: String,
        }
        let masque = |t: &str| secret.masquer(t);
        let (statut, texte) = self
            .transport
            .post_json(&self.url(&format!("/v1/intentions/{id}/abandon")), Some(secret), &json!({}))
            .map_err(|e| Erreur::Reseau(masque(&e)))?;
        let hors_contrat = || Erreur::Reponse(format!("abandon : réponse hors contrat ({statut})"));
        match statut {
            200 => {
                let etat = lire::<R>(&texte, Some(secret))?.etat;
                match etat.as_str() {
                    "abandonnee" | "purgee" => Ok(Abandon::Fait(etat)),
                    _ => Err(hors_contrat()),
                }
            }
            409 if code_d_erreur(&texte) == "avancee" => {
                let v: Value = serde_json::from_str(&texte).map_err(|_| hors_contrat())?;
                match v["etat"].as_str() {
                    Some(e) if etat_lisible(e) => Ok(Abandon::Avancee(e.to_string())),
                    _ => Err(hors_contrat()),
                }
            }
            502 if code_d_erreur(&texte) == "stripe_injoignable" => Ok(Abandon::StripeInjoignable),
            404 => Ok(Abandon::Inconnue),
            _ => Err(Erreur::Relais { statut, erreur: masque(&code_d_erreur(&texte)) }),
        }
    }
}

/// Pose un fichier par son URL de dépôt et vérifie que l'ETag rendu est son
/// MD5. Un écart est une erreur : le relais le refuserait de toute façon.
pub fn televerser(
    transport: &dyn Transport,
    quel: Quel,
    t: &Televersement,
    chemin: &Path,
    fichier: &Fichier,
    progression: &dyn Fn(u64),
) -> Result<(), Erreur> {
    let nom = match quel {
        Quel::Interieur => LIVRAISON_INTERIEUR,
        Quel::Couverture => LIVRAISON_COUVERTURE,
    };
    if !url_de_depot(&t.url) {
        return Err(Erreur::UrlDeDepot { fichier: nom });
    }
    let entetes: BTreeMap<String, String> = t
        .entetes
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
        .filter(|(k, _)| ENTETES_PERMIS.contains(&k.as_str()))
        .collect();
    let envoi = Envoi { url: &t.url, entetes: &entetes, chemin, octets: fichier.octets };
    // Une erreur de transport peut recopier l'URL, qui porte une signature.
    let (statut, etag) = transport
        .put_fichier(&envoi, progression)
        .map_err(|e| Erreur::Reseau(e.replace(&t.url, "<url de dépôt>")))?;
    if !(200..300).contains(&statut) {
        return Err(Erreur::Depot { fichier: nom, statut });
    }
    let lu = etag.as_deref().map(|e| e.trim_matches('"').to_ascii_lowercase());
    if lu.as_deref() != Some(fichier.md5.as_str()) {
        return Err(Erreur::Etag { fichier: nom, md5: fichier.md5.clone(), etag: lu });
    }
    Ok(())
}

/// Une URL de dépôt R2 : `https`, un hôte qui finit par
/// `.r2.cloudflarestorage.com`, ni port ni identifiants.
fn url_de_depot(url: &str) -> bool {
    let Some(reste) = url.strip_prefix("https://") else {
        return false;
    };
    let hote = reste.split(['/', '?', '#']).next().unwrap_or("");
    hote.len() > HOTE_R2.len()
        && hote.ends_with(HOTE_R2)
        && hote.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// Pose les deux fichiers du dossier préparé, le gros d'abord.
/// `progression` reçoit le fichier, les octets partis et le total.
pub fn envoyer(
    transport: &dyn Transport,
    televersements: &Televersements,
    dossier: &Path,
    demande: &Demande,
    progression: &dyn Fn(Quel, u64, u64),
) -> Result<(), Erreur> {
    for (quel, t, nom, f) in [
        (Quel::Interieur, &televersements.interieur, LIVRAISON_INTERIEUR, &demande.interieur),
        (Quel::Couverture, &televersements.couverture, LIVRAISON_COUVERTURE, &demande.couverture),
    ] {
        progression(quel, 0, f.octets);
        televerser(transport, quel, t, &dossier.join(nom), f, &|n| progression(quel, n, f.octets))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const SECRET: &str = "secret-de-lecture-0123456789abcdefghijkl";
    const BASE: &str = "https://relais.exemple";

    /// Ce que le faux a reçu : méthode, URL, bearer, corps.
    type Recu = (String, String, Option<String>, Value);

    /// Un faux relais : une réponse par appel, dans l'ordre, et tout ce qu'il
    /// a reçu.
    struct Faux {
        reponses: RefCell<Vec<(u16, String)>>,
        recu: RefCell<Vec<Recu>>,
        puts: RefCell<Vec<(String, BTreeMap<String, String>, u64)>>,
        etag: Option<String>,
        statut_put: u16,
    }

    impl Faux {
        fn new(reponses: &[(u16, &str)]) -> Faux {
            Faux {
                reponses: RefCell::new(reponses.iter().rev().map(|(s, t)| (*s, t.to_string())).collect()),
                recu: RefCell::new(vec![]),
                puts: RefCell::new(vec![]),
                etag: None,
                statut_put: 200,
            }
        }

        fn repondre(&self, methode: &str, url: &str, bearer: Option<&Secret>, corps: Value) -> Result<(u16, String), String> {
            self.recu.borrow_mut().push((methode.into(), url.into(), bearer.map(|s| s.0.clone()), corps));
            Ok(self.reponses.borrow_mut().pop().expect("une réponse prévue"))
        }
    }

    impl Transport for Faux {
        fn get(&self, url: &str, bearer: Option<&Secret>) -> Result<(u16, String), String> {
            self.repondre("GET", url, bearer, Value::Null)
        }
        fn post_json(&self, url: &str, bearer: Option<&Secret>, corps: &Value) -> Result<(u16, String), String> {
            self.repondre("POST", url, bearer, corps.clone())
        }
        fn put_fichier(&self, envoi: &Envoi<'_>, progression: &dyn Fn(u64)) -> Result<(u16, Option<String>), String> {
            let nom = envoi.chemin.file_name().unwrap().to_string_lossy().to_string();
            self.puts.borrow_mut().push((nom, envoi.entetes.clone(), envoi.octets));
            progression(envoi.octets);
            Ok((self.statut_put, self.etag.clone()))
        }
    }

    /// Un transport qui tombe, et dont le message recopie tout ce qu'il a
    /// reçu : le secret, l'URL.
    struct Coupe;
    impl Transport for Coupe {
        fn get(&self, url: &str, bearer: Option<&Secret>) -> Result<(u16, String), String> {
            Err(format!("connexion refusée vers {url} avec Bearer {}", bearer.map(|s| s.0.as_str()).unwrap_or("")))
        }
        fn post_json(&self, url: &str, bearer: Option<&Secret>, _: &Value) -> Result<(u16, String), String> {
            self.get(url, bearer)
        }
        fn put_fichier(&self, envoi: &Envoi<'_>, _: &dyn Fn(u64)) -> Result<(u16, Option<String>), String> {
            Err(format!("connexion refusée vers {}", envoi.url))
        }
    }

    fn secret() -> Secret {
        Secret::new(SECRET)
    }

    /// La réponse de création telle que la mesure S-s1 du 09/10 l'a relevée,
    /// URL et secret remplacés.
    fn creation() -> String {
        format!(
            r#"{{"id": "zNRsXsjGDfnOhog10i579Q", "secret": "{SECRET}", "mode": "sandbox",
              "televersements": {{
                "interieur": {{"url": "https://compte.r2.cloudflarestorage.com/depot/i?X-Amz-Signature=aaa", "entetes": {{"content-md5": "J1lHtPmnlYKHgoufZHDT7A=="}}}},
                "couverture": {{"url": "https://compte.r2.cloudflarestorage.com/depot/c?X-Amz-Signature=bbb", "entetes": {{"content-md5": "TrSodpp+zFUsU+pqsFGhqA=="}}}}
              }},
              "grille": [
                {{"niveau": "cp_ground", "transporteur": "Fedex - Regional Economy", "prix_ttc_centimes": 4600, "delai_jours": 4}},
                {{"niveau": "cp_fast", "transporteur": "FedEx - International Priority", "prix_ttc_centimes": 5250, "delai_jours": 4}}
              ],
              "expire": "2026-10-11T16:20:51.901047Z"}}"#
        )
    }

    fn demande() -> Demande {
        Demande {
            format: "carre-21".into(),
            pages: 96,
            pays: "FR".into(),
            etat_region: None,
            interieur: Fichier { md5: "275947b4f9a7958287828b9f6470d3ec".into(), octets: 94_980_105 },
            couverture: Fichier { md5: "4eb4a8769a7ecc552c53ea6bb051a1a8".into(), octets: 4_870_674 },
            acquittement: None,
        }
    }

    fn page(fichier: QuelFichier, page: u32, ppi: u32) -> PageSousSeuil {
        PageSousSeuil { fichier, page, ppi }
    }

    /// L'acquittement (1.1-c) : des entiers et une date, rien d'autre ; rien
    /// sans case cochée quand il y a des pages ; rien du tout quand il n'y en
    /// a pas ; refusé au-delà de 200 pages, avant le réseau.
    #[test]
    fn l_acquittement_se_construit_des_pages_et_de_la_case() {
        // Le seuil du relais est le plancher du profil qu'il commande.
        let cp = crate::printer::PrinterProfile::par_id("cloudprinter").unwrap();
        assert_eq!(cp.min_ppi, f64::from(SEUIL_PPI));
        let i = QuelFichier::Interieur;
        let c = QuelFichier::Couverture;
        let pages = vec![page(i, 5, 180), page(i, 13, 201), page(c, 1, 190)];
        let a = acquittement_pour(&pages, 250, true, "2026-10-20T14:03:12Z").unwrap().unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"version":1,"seuil_ppi":250,"coche_le":"2026-10-20T14:03:12Z","interieur":[[5,180],[13,201]],"couverture":[[1,190]]}"#
        );
        assert_eq!(acquittement_pour(&pages, 250, false, "x"), Err(Erreur::AcquittementManquant));
        assert_eq!(acquittement_pour(&[], 250, false, "x"), Ok(None));
        assert_eq!(acquittement_pour(&[], 250, true, "x"), Ok(None));
        let trop: Vec<PageSousSeuil> = (1..=201).map(|p| page(i, p, 100)).collect();
        assert_eq!(acquittement_pour(&trop, 250, true, "x"), Err(Erreur::AcquittementTropLong(201)));
        assert!(acquittement_pour(&trop[..200], 250, true, "x").unwrap().is_some());

        // Dans la demande : absent sans acquittement, compact avec.
        let mut d = demande();
        assert!(!serde_json::to_string(&d).unwrap().contains("acquittement"));
        d.acquittement = Some(a);
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains(r#""acquittement":{"version":1"#), "{json}");
    }

    /// L'heure de la case, en UTC à la seconde, la forme exacte que le relais
    /// accepte.
    #[test]
    fn l_heure_de_l_acquittement_a_la_forme_du_relais() {
        let t = coche_le_maintenant();
        assert_eq!(t.len(), 20, "{t}");
        assert!(t.ends_with('Z') && t.as_bytes()[10] == b'T', "{t}");
    }

    #[test]
    fn l_adresse_du_relais_est_le_sandbox_en_debug_et_le_reel_compile_en_release() {
        // Sans `COLOPHON_RELAIS_REEL`, la release ne porte rien : c'est ce qui
        // rend Commander inerte dans le bundle publié (décision 3). Le gate
        // tourne en debug, d'où `url_pour`.
        assert_eq!(url_pour(false, false, None), None);
        assert_eq!(mode_pour(false, false, None), None);
        // Avec, la release parle au relais réel, et à lui seul.
        assert_eq!(url_pour(false, false, Some("https://relais.exemple.fr")), Some("https://relais.exemple.fr"));
        assert_eq!(mode_pour(false, false, Some("https://relais.exemple.fr")), Some(Mode::Reel));
        // Le debug reste au sandbox, variables posées ou non.
        for reel in [None, Some("https://relais.exemple.fr")] {
            for sandbox in [false, true] {
                assert_eq!(url_pour(true, sandbox, reel), Some(SANDBOX));
                assert_eq!(mode_pour(true, sandbox, reel), Some(Mode::Sandbox));
            }
        }
        assert_eq!(RELAIS_URL, url_pour(cfg!(debug_assertions), SANDBOX_EN_RELEASE, REEL));
    }

    /// Une release branchée sur le sandbox : le produit final, optimisé et
    /// signé, qu'on teste de bout en bout à la carte de test. Ses intentions
    /// sont en mode sandbox, comme le relais les déclare : en mode réel elles
    /// sortiraient de la liste dès leur création (`Stock::vue`).
    #[test]
    fn une_release_peut_parler_au_sandbox_et_le_dit_en_mode_sandbox() {
        assert_eq!(url_pour(false, true, None), Some(SANDBOX));
        assert_eq!(mode_pour(false, true, None), Some(Mode::Sandbox));
        assert_eq!(sandbox_valide(None), Ok(false));
        assert_eq!(sandbox_valide(Some("1")), Ok(true));
        for refusee in ["", "0", "oui", "true", " 1", "11"] {
            assert!(sandbox_valide(Some(refusee)).is_err(), "{refusee:?} est acceptée");
        }
        // Les deux à la fois : on ne sait plus à qui parle la release.
        assert!(relais_unique(true, Some("https://relais.exemple.fr")).is_err());
        assert!(relais_unique(true, None).is_ok());
        assert!(relais_unique(false, Some("https://relais.exemple.fr")).is_ok());
    }

    #[test]
    fn le_relais_reel_compile_est_https_et_un_hote_seul() {
        assert_eq!(reel_valide(None), Ok(None));
        assert_eq!(reel_valide(Some("https://relais.colophon.fr")), Ok(Some("https://relais.colophon.fr")));
        assert_eq!(reel_valide(Some("https://relais-1.exemple")), Ok(Some("https://relais-1.exemple")));
        for refusee in [
            "http://relais.colophon.fr",
            "https://relais.colophon.fr/",
            "https://relais.colophon.fr/v1",
            "https://relais.colophon.fr:8443",
            "https://moi:mdp@relais.colophon.fr",
            "https://relais.colophon.fr?x=1",
            "https://relais.colophon.fr#x",
            "https://",
            "https://.colophon.fr",
            "https://relais..colophon.fr",
            "https://relais.colophon.fr.",
            " https://relais.colophon.fr",
            "relais.colophon.fr",
        ] {
            assert!(reel_valide(Some(refusee)).is_err(), "{refusee:?} est acceptée");
        }
    }

    #[test]
    fn abandonner_rend_les_quatre_reponses_de_la_route_et_une_erreur_pour_le_reste() {
        let t = Faux::new(&[
            (200, r#"{"etat": "abandonnee"}"#),
            (200, r#"{"etat": "purgee"}"#),
            (409, r#"{"erreur": "avancee", "etat": "payee"}"#),
            // Stripe refuse d'expirer une session complète : le webhook n'est
            // pas encore passé, l'état rendu est encore `attente_paiement`.
            (409, r#"{"erreur": "avancee", "etat": "attente_paiement"}"#),
            (502, r#"{"erreur": "stripe_injoignable"}"#),
            (404, r#"{"erreur": "introuvable"}"#),
            (404, "pas de corps lisible"),
        ]);
        let r = Relais { base: BASE, transport: &t };
        let s = secret();
        assert_eq!(r.abandonner("abc", &s), Ok(Abandon::Fait("abandonnee".into())));
        assert_eq!(r.abandonner("abc", &s), Ok(Abandon::Fait("purgee".into())));
        assert_eq!(r.abandonner("abc", &s), Ok(Abandon::Avancee("payee".into())));
        assert_eq!(r.abandonner("abc", &s), Ok(Abandon::Avancee("attente_paiement".into())));
        assert_eq!(r.abandonner("abc", &s), Ok(Abandon::StripeInjoignable));
        assert_eq!(r.abandonner("abc", &s), Ok(Abandon::Inconnue));
        assert_eq!(r.abandonner("abc", &s), Ok(Abandon::Inconnue));
        let recu = t.recu.borrow();
        assert_eq!((recu[0].0.as_str(), recu[0].1.as_str()), ("POST", "https://relais.exemple/v1/intentions/abc/abandon"));
        assert_eq!(recu[0].2.as_deref(), Some(SECRET), "le secret part en bearer");
        assert_eq!(recu[0].3, json!({}), "corps vide");
        drop(recu);

        // Hors contrat : une erreur typée, jamais une variante qui permettrait
        // l'oubli.
        let t = Faux::new(&[
            (200, r#"{"etat": "attente_paiement"}"#),
            (200, r#"{"etat": 3}"#),
            (409, r#"{"erreur": "etat", "etat": "payee"}"#),
            (409, r#"{"erreur": "avancee"}"#),
            (409, r#"{"erreur": "avancee", "etat": "payée <b>"}"#),
            (502, "<html>Bad gateway</html>"),
            (500, r#"{"erreur": "interne"}"#),
        ]);
        let r = Relais { base: BASE, transport: &t };
        assert!(matches!(r.abandonner("a", &s), Err(Erreur::Reponse(_))));
        assert!(matches!(r.abandonner("a", &s), Err(Erreur::Reponse(_))));
        assert_eq!(r.abandonner("a", &s), Err(Erreur::Relais { statut: 409, erreur: "etat".into() }));
        assert!(matches!(r.abandonner("a", &s), Err(Erreur::Reponse(_))));
        assert!(matches!(r.abandonner("a", &s), Err(Erreur::Reponse(_))));
        assert_eq!(r.abandonner("a", &s), Err(Erreur::Relais { statut: 502, erreur: "illisible".into() }));
        assert_eq!(r.abandonner("a", &s), Err(Erreur::Relais { statut: 500, erreur: "interne".into() }));
        let coupe = Relais { base: BASE, transport: &Coupe };
        let e = coupe.abandonner("a", &s).unwrap_err();
        assert!(matches!(e, Erreur::Reseau(_)));
        assert!(!e.to_string().contains(SECRET), "{e}");
    }

    #[test]
    fn un_put_ne_recopie_que_les_entetes_permis() {
        let mut t: Televersement = serde_json::from_str(
            r#"{"url": "https://compte.r2.cloudflarestorage.com/depot/x?X-Amz-Signature=a",
                "entetes": {"Content-MD5": "J1lHtPmnlYKHgoufZHDT7A==", "content-type": "application/pdf",
                  "Host": "ailleurs.exemple", "Authorization": "Bearer vole", "Cookie": "a=b",
                  "Transfer-Encoding": "chunked", "Content-Length": "1", "X-Autre": "y"}}"#,
        )
        .unwrap();
        let d = demande();
        let mut f = Faux::new(&[]);
        f.etag = Some(format!("\"{}\"", d.interieur.md5));
        televerser(&f, Quel::Interieur, &t, Path::new("album-print.pdf"), &d.interieur, &|_| {}).unwrap();
        let recus: Vec<String> = f.puts.borrow()[0].1.keys().cloned().collect();
        assert_eq!(recus, ["content-md5", "content-type"]);

        for url in [
            "http://compte.r2.cloudflarestorage.com/depot/x",
            "https://exemple.fr/depot/x",
            "https://r2.cloudflarestorage.com/depot/x",
            "https://.r2.cloudflarestorage.com/depot/x",
            "https://x.r2.cloudflarestorage.com.exemple.fr/depot/x",
            "https://x.r2.cloudflarestorage.com:8443/depot/x",
            "https://moi:mdp@x.r2.cloudflarestorage.com/depot/x",
            "https://x.r2.cloudflarestorage.com@exemple.fr/depot/x",
            "file:///etc/passwd",
        ] {
            t.url = url.to_string();
            let f = Faux::new(&[]);
            let e = televerser(&f, Quel::Interieur, &t, Path::new("album-print.pdf"), &d.interieur, &|_| {}).unwrap_err();
            assert_eq!(e, Erreur::UrlDeDepot { fichier: "album-print.pdf" }, "{url}");
            assert!(f.puts.borrow().is_empty(), "{url} a été envoyée");
        }
    }

    #[test]
    fn les_pays_viennent_du_relais_sans_secret() {
        let t = Faux::new(&[(200, r#"{"mode": "sandbox", "pays": [{"code": "FR", "nom": "France", "require_state": false}]}"#)]);
        let p = Relais { base: BASE, transport: &t }.pays().unwrap();
        assert_eq!(p[0].code, "FR");
        assert!(p[0].etats.is_empty());
        let r = t.recu.borrow();
        assert_eq!((r[0].0.as_str(), r[0].1.as_str(), r[0].2.as_deref()), ("GET", "https://relais.exemple/v1/pays", None));
    }

    #[test]
    fn la_creation_envoie_la_liste_blanche_du_relais_et_rien_d_autre() {
        let t = Faux::new(&[(201, &creation())]);
        let i = Relais { base: "https://relais.exemple/", transport: &t }.creer(&demande()).unwrap();
        assert_eq!(i.id, "zNRsXsjGDfnOhog10i579Q");
        assert_eq!(i.mode, Mode::Sandbox);
        assert_eq!(i.grille.len(), 2);
        assert_eq!(i.grille[0].prix_ttc_centimes, 4600);
        let r = t.recu.borrow();
        assert_eq!(r[0].1, "https://relais.exemple/v1/intentions");
        assert_eq!(r[0].2, None, "la création n'a pas encore de secret");
        // Exactement les clés que `lireDemande` du relais permet : il refuse
        // toute autre, `papier` compris.
        assert_eq!(
            r[0].3,
            json!({
                "format": "carre-21", "pages": 96, "pays": "FR",
                "interieur": {"md5": "275947b4f9a7958287828b9f6470d3ec", "octets": 94_980_105},
                "couverture": {"md5": "4eb4a8769a7ecc552c53ea6bb051a1a8", "octets": 4_870_674}
            })
        );
    }

    #[test]
    fn le_secret_part_en_bearer_jamais_dans_l_url() {
        let t = Faux::new(&[
            (200, r#"{"etat": "attente_paiement"}"#),
            (200, r#"{"url": "https://checkout.stripe.com/c/pay/cs_test_x"}"#),
            (200, r#"{"etat": "commandee", "mode": "sandbox", "dernier_code": 501}"#),
            (200, r#"{"etat": "annulee"}"#),
        ]);
        let r = Relais { base: BASE, transport: &t };
        let s = secret();
        assert_eq!(r.fichiers("abc", &s).unwrap(), "attente_paiement");
        assert_eq!(r.paiement("abc", &s, "cp_ground").unwrap(), "https://checkout.stripe.com/c/pay/cs_test_x");
        assert_eq!(
            r.etat("abc", &s).unwrap(),
            Etat { etat: "commandee".into(), mode: Mode::Sandbox, dernier_code: Some(501) }
        );
        assert_eq!(r.annuler("abc", &s).unwrap(), "annulee");
        let recu = t.recu.borrow();
        let chemins: Vec<(&str, &str)> = recu.iter().map(|x| (x.0.as_str(), x.1.as_str())).collect();
        assert_eq!(
            chemins,
            [
                ("POST", "https://relais.exemple/v1/intentions/abc/fichiers"),
                ("POST", "https://relais.exemple/v1/intentions/abc/paiement"),
                ("GET", "https://relais.exemple/v1/intentions/abc"),
                ("POST", "https://relais.exemple/v1/intentions/abc/annulation"),
            ]
        );
        for x in recu.iter() {
            assert_eq!(x.2.as_deref(), Some(SECRET));
            assert!(!x.1.contains(SECRET));
        }
        assert_eq!(recu[1].3, json!({"niveau": "cp_ground"}));
        assert_eq!(recu[0].3, json!({}), "un corps JSON vide : le relais exige le type");
    }

    #[test]
    fn un_refus_devient_le_champ_erreur_du_relais_et_rien_d_autre() {
        let t = Faux::new(&[
            (409, r#"{"erreur": "trop_tard"}"#),
            (422, r#"{"erreur": "fichiers", "raison": "md5", "fichier": "interieur"}"#),
            (404, r#"{"erreur": "introuvable"}"#),
            (502, "<html>Bad gateway</html>"),
        ]);
        let r = Relais { base: BASE, transport: &t };
        let s = secret();
        assert_eq!(r.annuler("a", &s).unwrap_err(), Erreur::Relais { statut: 409, erreur: "trop_tard".into() });
        assert_eq!(r.fichiers("a", &s).unwrap_err(), Erreur::Relais { statut: 422, erreur: "fichiers".into() });
        assert_eq!(r.etat("a", &s).unwrap_err().code(), Some("introuvable"));
        assert_eq!(r.etat("a", &s).unwrap_err(), Erreur::Relais { statut: 502, erreur: "illisible".into() });
        assert!(matches!(r.pays_avec(&Coupe), Err(Erreur::Reseau(_))));
    }

    impl Relais<'_> {
        fn pays_avec(&self, t: &dyn Transport) -> Result<Vec<Pays>, Erreur> {
            Relais { base: self.base, transport: t }.pays()
        }
    }

    #[test]
    fn le_secret_ne_fuit_ni_par_debug_ni_par_une_erreur() {
        let s = secret();
        assert_eq!(format!("{s:?}"), "***");
        assert_eq!(s.to_string(), "***");
        let t = Faux::new(&[(201, &creation())]);
        let i = Relais { base: BASE, transport: &t }.creer(&demande()).unwrap();
        let vu = format!("{i:?}");
        assert!(!vu.contains(SECRET), "{vu}");
        assert!(!vu.contains("X-Amz-Signature"), "l'URL de dépôt se tait aussi : {vu}");

        // Le relais recopie le secret dans son refus, le transport dans son
        // message, un corps illisible le porte : aucune erreur ne le cite.
        let echo = format!(r#"{{"erreur": "{SECRET}"}}"#);
        let t = Faux::new(&[
            (400, &echo),
            (200, &format!("pas du json {SECRET}")),
            (200, &format!(r#"{{"etat": 3, "x": "{SECRET}"}}"#)),
            (200, &format!(r#"{{"id": "{SECRET}"}}"#)),
        ]);
        let r = Relais { base: BASE, transport: &t };
        let mut erreurs = vec![
            r.etat("a", &s).unwrap_err(),
            r.etat("a", &s).unwrap_err(),
            r.fichiers("a", &s).unwrap_err(),
            r.creer(&demande()).unwrap_err(),
        ];
        let coupe = Relais { base: BASE, transport: &Coupe };
        erreurs.push(coupe.etat("a", &s).unwrap_err());
        erreurs.push(coupe.paiement("a", &s, "cp_ground").unwrap_err());
        let tv = i.televersements.interieur.clone();
        erreurs.push(
            televerser(&Coupe, Quel::Interieur, &tv, Path::new("album-print.pdf"), &demande().interieur, &|_| {}).unwrap_err(),
        );
        for e in &erreurs {
            for interdit in [SECRET, "X-Amz-Signature"] {
                assert!(!format!("{e:?}").contains(interdit), "{e:?}");
                assert!(!e.to_string().contains(interdit), "{e}");
            }
        }
    }

    #[test]
    fn un_put_porte_les_entetes_rendus_et_sa_longueur_et_l_etag_est_compare() {
        let i: Intention = serde_json::from_str(&creation()).unwrap();
        let d = demande();
        let mut t = Faux::new(&[]);
        t.etag = Some("\"275947B4F9A7958287828B9F6470D3EC\"".into());
        let dossier = Path::new("/nulle/part");
        televerser(&t, Quel::Interieur, &i.televersements.interieur, &dossier.join("album-print.pdf"), &d.interieur, &|_| {})
            .unwrap();
        let puts = t.puts.borrow();
        assert_eq!(puts[0].0, "album-print.pdf");
        assert_eq!(puts[0].1.get("content-md5").map(String::as_str), Some("J1lHtPmnlYKHgoufZHDT7A=="));
        assert_eq!(puts[0].2, 94_980_105);
        drop(puts);

        // Un autre ETag, ou aucun : refusé.
        t.etag = Some("\"00000000000000000000000000000000\"".into());
        let e = televerser(&t, Quel::Interieur, &i.televersements.interieur, dossier, &d.interieur, &|_| {}).unwrap_err();
        assert!(matches!(e, Erreur::Etag { fichier: "album-print.pdf", .. }), "{e:?}");
        t.etag = None;
        assert!(matches!(
            televerser(&t, Quel::Couverture, &i.televersements.couverture, dossier, &d.couverture, &|_| {}),
            Err(Erreur::Etag { etag: None, .. })
        ));
        // Un ETag de multipart, suffixé, n'est jamais un MD5.
        t.etag = Some("\"275947b4f9a7958287828b9f6470d3ec-2\"".into());
        assert!(televerser(&t, Quel::Interieur, &i.televersements.interieur, dossier, &d.interieur, &|_| {}).is_err());
        // Un refus du dépôt.
        t.statut_put = 403;
        assert_eq!(
            televerser(&t, Quel::Interieur, &i.televersements.interieur, dossier, &d.interieur, &|_| {}).unwrap_err(),
            Erreur::Depot { fichier: "album-print.pdf", statut: 403 }
        );
    }

    /// Le faux qui rend l'ETag du fichier qu'on lui pose, selon son nom.
    struct DepotJuste(Faux);
    impl Transport for DepotJuste {
        fn get(&self, u: &str, b: Option<&Secret>) -> Result<(u16, String), String> {
            self.0.get(u, b)
        }
        fn post_json(&self, u: &str, b: Option<&Secret>, c: &Value) -> Result<(u16, String), String> {
            self.0.post_json(u, b, c)
        }
        fn put_fichier(&self, e: &Envoi<'_>, p: &dyn Fn(u64)) -> Result<(u16, Option<String>), String> {
            self.0.put_fichier(e, p)?;
            let d = demande();
            let md5 = if e.chemin.ends_with("album-print.pdf") { d.interieur.md5 } else { d.couverture.md5 };
            Ok((200, Some(format!("\"{md5}\""))))
        }
    }

    #[test]
    fn le_gros_part_d_abord_et_la_progression_compte_des_octets() {
        let i: Intention = serde_json::from_str(&creation()).unwrap();
        let t = DepotJuste(Faux::new(&[]));
        let vu = RefCell::new(vec![]);
        envoyer(&t, &i.televersements, Path::new("/dossier"), &demande(), &|q, n, total| {
            vu.borrow_mut().push((q, n, total))
        })
        .unwrap();
        let noms: Vec<String> = t.0.puts.borrow().iter().map(|p| p.0.clone()).collect();
        assert_eq!(noms, ["album-print.pdf", "album-cover.pdf"]);
        assert_eq!(
            vu.into_inner(),
            [
                (Quel::Interieur, 0, 94_980_105),
                (Quel::Interieur, 94_980_105, 94_980_105),
                (Quel::Couverture, 0, 4_870_674),
                (Quel::Couverture, 4_870_674, 4_870_674),
            ]
        );
    }

    #[test]
    fn la_demande_lit_les_deux_fichiers_et_refuse_un_autre_papier() {
        let dir = std::env::temp_dir().join(format!("colophon-relais-demande-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(LIVRAISON_INTERIEUR), b"abc").unwrap();
        std::fs::write(dir.join(LIVRAISON_COUVERTURE), b"").unwrap();
        let d = Demande::du_dossier(&dir, Some(PAPIER), "carre-21", 96, "FR", None).unwrap();
        // Les vecteurs de la RFC 1321.
        assert_eq!(d.interieur, Fichier { md5: "900150983cd24fb0d6963f7d28e17f72".into(), octets: 3 });
        assert_eq!(d.couverture, Fichier { md5: "d41d8cd98f00b204e9800998ecf8427e".into(), octets: 0 });
        assert_eq!(Demande::du_dossier(&dir, Some("pageblock_90"), "carre-21", 96, "FR", None), Err(Erreur::Papier));
        assert_eq!(Demande::du_dossier(&dir, None, "carre-21", 96, "FR", None), Err(Erreur::Papier));
        std::fs::remove_file(dir.join(LIVRAISON_COUVERTURE)).unwrap();
        assert!(matches!(
            Demande::du_dossier(&dir, Some(PAPIER), "carre-21", 96, "FR", None),
            Err(Erreur::Fichier(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Le transport réel, contre un serveur local : le corps est le fichier,
    /// à l'octet, avec sa longueur annoncée (R2 refuse un PUT présigné sans
    /// elle) et l'en-tête signé ; l'ETag de la réponse est relu, et la
    /// progression finit sur le total.
    #[test]
    fn le_put_reel_annonce_sa_longueur_et_envoie_le_fichier_en_flux() {
        use std::io::{BufRead, BufReader, Read, Write};
        let dir = std::env::temp_dir().join(format!("colophon-relais-put-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let chemin = dir.join(LIVRAISON_INTERIEUR);
        let contenu: Vec<u8> = (0..(3 * PAS_DE_PROGRESSION + 17)).map(|i| (i % 251) as u8).collect();
        std::fs::write(&chemin, &contenu).unwrap();

        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let serveur = std::thread::spawn(move || {
            let (flux, _) = ecoute.accept().unwrap();
            let mut lecteur = BufReader::new(flux.try_clone().unwrap());
            let mut entetes = vec![];
            loop {
                let mut l = String::new();
                lecteur.read_line(&mut l).unwrap();
                if l == "\r\n" {
                    break;
                }
                entetes.push(l.trim_end().to_string());
            }
            let longueur: usize = entetes
                .iter()
                .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length: ").map(|v| v.parse().unwrap()))
                .expect("Content-Length");
            let mut corps = vec![0u8; longueur];
            lecteur.read_exact(&mut corps).unwrap();
            let mut flux = flux;
            write!(flux, "HTTP/1.1 200 OK\r\nETag: \"cafe\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            (entetes, corps)
        });

        let mut entetes = BTreeMap::new();
        entetes.insert("content-md5".to_string(), "J1lHtPmnlYKHgoufZHDT7A==".to_string());
        let url = format!("http://127.0.0.1:{port}/depot/x");
        let envoi = Envoi { url: &url, entetes: &entetes, chemin: &chemin, octets: contenu.len() as u64 };
        let vu = RefCell::new(vec![]);
        let (statut, etag) = Reseau::default().put_fichier(&envoi, &|n| vu.borrow_mut().push(n)).unwrap();
        let (recu, corps) = serveur.join().unwrap();
        assert_eq!((statut, etag.as_deref()), (200, Some("\"cafe\"")));
        assert!(recu[0].starts_with("PUT /depot/x "), "{recu:?}");
        assert!(recu.iter().any(|l| l.eq_ignore_ascii_case(&format!("content-length: {}", contenu.len()))), "{recu:?}");
        assert!(recu.iter().any(|l| l.eq_ignore_ascii_case("content-md5: J1lHtPmnlYKHgoufZHDT7A==")), "{recu:?}");
        assert!(!recu.iter().any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding")), "{recu:?}");
        assert_eq!(corps, contenu);
        let vu = vu.into_inner();
        assert_eq!(vu.last(), Some(&(contenu.len() as u64)));
        assert!(vu.len() >= 3, "une progression par mébioctet : {vu:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Le transport réel pose ce que la route `/abandon` exige, comme toute
    /// route POST du relais : `Content-Type: application/json` (415 sinon),
    /// le corps `{}` (tout autre champ est un 400), et le secret en bearer.
    #[test]
    fn l_abandon_reel_part_en_json_avec_un_corps_vide_et_le_bearer() {
        use std::io::{BufRead, BufReader, Read, Write};
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let serveur = std::thread::spawn(move || {
            let (flux, _) = ecoute.accept().unwrap();
            let mut lecteur = BufReader::new(flux.try_clone().unwrap());
            let mut entetes = vec![];
            loop {
                let mut l = String::new();
                lecteur.read_line(&mut l).unwrap();
                if l == "\r\n" {
                    break;
                }
                entetes.push(l.trim_end().to_string());
            }
            let longueur: usize = entetes
                .iter()
                .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length: ").map(|v| v.parse().unwrap()))
                .unwrap_or(0);
            let mut corps = vec![0u8; longueur];
            lecteur.read_exact(&mut corps).unwrap();
            let reponse = r#"{"etat":"abandonnee"}"#;
            let mut flux = flux;
            write!(
                flux,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reponse}",
                reponse.len()
            )
            .unwrap();
            (entetes, String::from_utf8(corps).unwrap())
        });
        let base = format!("http://127.0.0.1:{port}");
        let reseau = Reseau::default();
        let rendu = Relais { base: &base, transport: &reseau }.abandonner("abc", &secret());
        let (recu, corps) = serveur.join().unwrap();
        assert_eq!(rendu, Ok(Abandon::Fait("abandonnee".into())));
        assert!(recu[0].starts_with("POST /v1/intentions/abc/abandon "), "{recu:?}");
        assert!(recu.iter().any(|l| l.eq_ignore_ascii_case("content-type: application/json")), "{recu:?}");
        assert!(recu.iter().any(|l| l == &format!("authorization: Bearer {SECRET}") || l == &format!("Authorization: Bearer {SECRET}")), "{recu:?}");
        assert_eq!(corps, "{}");
    }

    /// Le banc du sandbox (`#[ignore]`), à la main : le parcours de l'app
    /// contre le relais sandbox déployé, sur les deux PDF vérifiés de
    /// `.albums/papier` (`COLOPHON_PAPIER` pour un autre dossier). Intention,
    /// les deux PUT (le gros d'abord) avec durée, octets et ETag, `fichiers`,
    /// puis `paiement`, dont on ne garde que l'hôte de l'URL : la session
    /// n'est **ni payée ni ouverte**. Puis deux abandons (S-s4b) : le relais
    /// expire la session chez Stripe, rend 200 deux fois, et l'intention se
    /// relit `abandonnee`. Aucune clé n'est lue.
    /// La mesure, expurgée (ni secret ni URL), va dans `COLOPHON_MESURE`.
    #[test]
    #[ignore]
    fn banc_relais_sandbox() {
        use crate::printer::PrinterProfile;
        use std::time::Instant;
        // Un build de release ne porte aucune adresse : le banc se lance en
        // debug, ou `COLOPHON_RELAIS` la donne.
        let base = std::env::var("COLOPHON_RELAIS")
            .ok()
            .or(RELAIS_URL.map(str::to_string))
            .expect("aucune adresse de relais : build de debug, ou COLOPHON_RELAIS");
        let papier = std::env::var("COLOPHON_PAPIER").map(std::path::PathBuf::from).unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.albums/papier")
        });
        let profil = PrinterProfile::par_id("cloudprinter").unwrap();
        let fiche = crate::prevol::prevol(&papier, profil).unwrap().fiche;
        let format = crate::format::nom(crate::model::Size { w: fiche.format_page_mm[0], h: fiche.format_page_mm[1] });
        let reseau = Reseau::default();
        let r = Relais { base: &base, transport: &reseau };
        let ms = |t: Instant| t.elapsed().as_millis() as u64;
        let debut = Instant::now();
        let mut mesure = json!({
            "date_utc": chrono::Utc::now().format("%Y-%m-%d").to_string(),
            "session": "S-s4b, Commander dans l'app puis Abandonner (banc_relais_sandbox)",
            "relais": base,
            "client": "colophon_core::relais, transport Reseau (ureq, rustls), comme l'app",
            "expurgation": "aucun secret, aucune URL présignée ni de session ; seul l'hôte de l'URL de paiement est gardé ; l'identifiant d'intention reste (référence sandbox)",
        });

        let t = Instant::now();
        let pays = r.pays().unwrap();
        mesure["pays"] = json!({"ms": ms(t), "codes": pays.iter().map(|p| p.code.clone()).collect::<Vec<_>>()});

        let t = Instant::now();
        let demande = Demande::du_dossier(
            &papier,
            profil.commande.map(|c| c.papier),
            &format,
            fiche.pages_fichier as u32,
            "FR",
            None,
        )
        .unwrap();
        mesure["md5_ms"] = json!(ms(t));
        let t = Instant::now();
        let i = r.creer(&demande).unwrap();
        mesure["intention"] = json!({
            "ms": ms(t), "id": i.id, "mode": i.mode, "grille": i.grille, "expire": i.expire,
            "demande": demande,
            "entetes_a_envoyer": i.televersements.interieur.entetes.keys().collect::<Vec<_>>(),
        });

        for (quel, tv, nom, f) in [
            (Quel::Interieur, &i.televersements.interieur, LIVRAISON_INTERIEUR, &demande.interieur),
            (Quel::Couverture, &i.televersements.couverture, LIVRAISON_COUVERTURE, &demande.couverture),
        ] {
            let t = Instant::now();
            let derniere = RefCell::new(0u64);
            let paliers = RefCell::new(0u32);
            televerser(&reseau, quel, tv, &papier.join(nom), f, &|n| {
                *derniere.borrow_mut() = n;
                *paliers.borrow_mut() += 1;
            })
            .unwrap();
            // `televerser` a déjà refusé tout ETag autre que le MD5.
            mesure["puts"][nom] = json!({
                "duree_s": t.elapsed().as_secs_f64(),
                "octets": f.octets,
                "octets_signales": *derniere.borrow(),
                "progressions": *paliers.borrow(),
                "md5": f.md5,
                "etag_egal_md5": true,
            });
        }

        let t = Instant::now();
        let etat = r.fichiers(&i.id, &i.secret).unwrap();
        mesure["fichiers"] = json!({"ms": ms(t), "etat": etat});

        let niveau = i.grille[0].niveau.clone();
        let t = Instant::now();
        let url = r.paiement(&i.id, &i.secret, &niveau).unwrap();
        let hote = url.strip_prefix("https://").and_then(|u| u.split('/').next()).unwrap_or("").to_string();
        mesure["paiement"] = json!({
            "ms": ms(t), "niveau": niveau, "url_https": url.starts_with("https://"), "hote": hote,
            "ouverte": false, "payee": false,
        });
        let t = Instant::now();
        let apres = r.etat(&i.id, &i.secret).unwrap();
        mesure["etat_apres"] = json!({"ms": ms(t), "etat": apres});

        // L'abandon après le paiement ouvert (S-s4b) : le relais expire la
        // session chez Stripe et rend 200 ; un second abandon rend 200 de
        // nouveau ; l'état relu est `abandonnee`. Un 404 ici dit que la
        // route n'est pas déployée : l'intention vient d'être créée.
        let t = Instant::now();
        let premier = r.abandonner(&i.id, &i.secret);
        let premier_ms = ms(t);
        let t = Instant::now();
        let second = r.abandonner(&i.id, &i.secret);
        let second_ms = ms(t);
        let fin = r.etat(&i.id, &i.secret).unwrap();
        mesure["abandon"] = json!({
            "premier": {"ms": premier_ms, "rendu": format!("{premier:?}")},
            "second": {"ms": second_ms, "rendu": format!("{second:?}")},
            "etat_relu": fin.etat,
        });
        mesure["duree_s"] = json!(debut.elapsed().as_secs());

        let mut texte = serde_json::to_string_pretty(&mesure).unwrap();
        texte = texte.replace(&i.secret.0, "***").replace(&url, "***");
        assert!(!texte.contains("X-Amz-") && !texte.contains("cs_test_") && !texte.contains("cs_live_"));
        let sortie = std::env::var("COLOPHON_MESURE").unwrap_or_else(|_| "/dev/null".into());
        std::fs::write(&sortie, texte + "\n").unwrap();
        assert_eq!(etat, "attente_paiement");
        assert_eq!(hote, "checkout.stripe.com");
        assert_eq!(premier, Ok(Abandon::Fait("abandonnee".into())));
        assert_eq!(second, Ok(Abandon::Fait("abandonnee".into())));
        assert_eq!(fin.etat, "abandonnee");
    }
}
