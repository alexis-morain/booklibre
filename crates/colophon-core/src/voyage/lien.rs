//! Le lecteur du lien public d'un voyage (P-s1, plan D2, D16 à D18).
//!
//! Il prend l'adresse qu'une personne colle (`polarsteps.com/<qui>/<id>-<nom>`,
//! avec ou sans `?s=<jeton de partage>`), demande **un** voyage à la même API
//! JSON que le site public interroge, et écrit un dossier : les photos et un
//! `voyage.json` ([`super::Voyage`]). Le moteur ne voit que le dossier.
//!
//! **Ce que ce module est, et n'est pas.** Il lit un voyage que la personne a
//! sous les yeux, collé à la main. Jamais d'énumération, jamais de parcours
//! d'un profil, aucun cookie, aucun jeton d'API, une requête à la fois, un
//! `User-Agent` qui dit Booklibre, une pause entre deux requêtes, des plafonds
//! de taille partout. **Le seul jeton qui part est celui que la personne a
//! collé dans son lien**, tel quel, vers l'API et vers elle seule : il n'entre
//! ni dans `voyage.json`, ni dans un message d'erreur, ni dans une adresse de
//! photo.
//!
//! **Jamais lu : `user`, `trip_buddies`, `users`** (plan D6). La conversion
//! cueille ses champs un à un dans la réponse ; ce qu'elle ne nomme pas ne
//! peut pas fuir. Le test `ni_l_utilisateur_ni_les_compagnons_ne_sortent`
//! tient la promesse.
//!
//! **La version de l'API est figée** ([`VERSION_API`] = 62, celle du site, la
//! dernière à rendre la trace GPS). Une réponse dont la forme n'est plus
//! celle-là donne une [`Erreur::Format`] franche : jamais un voyage à moitié
//! rempli. Les photos sont les `large_thumbnail_path` (960 px au plus) : la
//! pleine résolution est refusée à qui n'est pas connecté (mesure du 12/11).
//!
//! **Absent de la version Mac App Store.** Cette adresse réseau n'existe pas
//! là-bas : l'app n'appelle ce module que derrière `cfg(not(feature = "mas"))`
//! (P-s3), le linker ne garde rien de ce que personne n'appelle, et `mas.sh`
//! le prouve sur le binaire.

use super::{Etape, Lieu, Meteo, Photo, Point, PointDeTrace, Provenance, Segment, Voyage, VERSION};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use std::cell::Cell;
use std::fmt;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

/// L'API que le site public interroge. Seule adresse réseau du module.
pub const API: &str = "https://api.polarsteps.com";

/// La version de l'API, figée.
pub const VERSION_API: u32 = 62;

/// Les seuls hôtes d'où une photo se télécharge. Une réponse qui en nomme un
/// autre n'est plus celle qu'on sait lire : l'import ne va pas chercher
/// n'importe où ce qu'un JSON lui dit.
const HOTES_PHOTOS: [&str; 2] = ["media.prod.polarsteps.com", "polarsteps.s3.amazonaws.com"];

const SOURCE: &str = "polarsteps-lien";

// ---- les plafonds ------------------------------------------------------------

/// Ce qu'un import accepte de recevoir. Les valeurs par défaut sont larges
/// pour un vrai voyage (93 étapes, 1 546 photos de 200 Ko, une réponse de
/// 2,8 Mo) et serrées pour le reste.
#[derive(Debug, Clone)]
pub struct Limites {
    /// Octets de la réponse du voyage.
    pub reponse: u64,
    pub etapes: usize,
    /// Photos du voyage entier.
    pub photos: usize,
    /// Octets d'une photo.
    pub photo: u64,
    /// Octets de toutes les photos reçues.
    pub total: u64,
    /// Points de la trace.
    pub trace: usize,
}

impl Default for Limites {
    fn default() -> Self {
        Limites {
            reponse: 32 << 20,
            // Le rang d'étape s'écrit sur trois chiffres dans les noms de
            // fichiers : au-delà, l'ordre alphabétique ne serait plus l'ordre.
            etapes: 999,
            photos: 5_000,
            photo: 8 << 20,
            total: 2 << 30,
            trace: 200_000,
        }
    }
}

/// Les plafonds et la politesse d'un import.
#[derive(Debug, Clone)]
pub struct Reglages {
    pub limites: Limites,
    /// L'attente entre deux requêtes.
    pub pause: Duration,
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages { limites: Limites::default(), pause: Duration::from_millis(120) }
    }
}

/// Le plafond de photos d'une seule étape : le rang s'écrit sur deux chiffres.
const PHOTOS_PAR_ETAPE: usize = 99;

// ---- les erreurs -------------------------------------------------------------

/// Ce qui arrête un import. Aucune ne porte le jeton de partage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Erreur {
    /// L'adresse collée n'est pas celle d'un voyage Polarsteps.
    Url(String),
    /// Le réseau, avant tout statut.
    Reseau(String),
    /// Un statut autre que 200, pour le voyage ou pour une photo.
    Statut { statut: u16, quoi: &'static str },
    /// Un plafond atteint.
    TropGros(&'static str),
    /// La réponse n'a plus la forme qu'on sait lire.
    Format(String),
    /// Un fichier reçu n'est pas une image lisible.
    Photo(String),
    /// Le dossier de sortie.
    Disque(String),
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Url(m) => write!(f, "ce n'est pas l'adresse d'un voyage Polarsteps ({m}) : colle le lien tel que le site ou l'app le donne"),
            Erreur::Reseau(m) => write!(f, "le réseau a échoué : {m}"),
            Erreur::Statut { statut, quoi: "voyage" } if matches!(statut, 401 | 403 | 404) => write!(
                f,
                "Polarsteps ne montre pas ce voyage ({statut}) : il est privé ou n'existe plus. Pour un voyage privé, colle le lien de partage complet, avec son « ?s=… »"
            ),
            Erreur::Statut { statut, quoi } => write!(f, "Polarsteps a répondu {statut} pour {quoi}"),
            Erreur::TropGros(quoi) => write!(f, "{quoi} dépasse ce que Booklibre accepte de recevoir"),
            Erreur::Format(m) => write!(f, "Polarsteps a changé son format ({m}) : passe par l'export de tes données"),
            Erreur::Photo(m) => write!(f, "une photo reçue n'est pas lisible : {m}"),
            Erreur::Disque(m) => write!(f, "écriture impossible : {m}"),
        }
    }
}

impl std::error::Error for Erreur {}

fn inconnu(msg: impl Into<String>) -> Erreur {
    Erreur::Format(msg.into())
}

// ---- l'adresse ---------------------------------------------------------------

/// Ce que l'adresse collée désigne : un voyage, et son jeton de partage si
/// elle en porte un. Le nom d'utilisateur de l'adresse n'est pas gardé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lien {
    pub id: u64,
    pub jeton: Option<String>,
}

const JETON_MAX: usize = 128;

pub fn analyser_url(url: &str) -> Result<Lien, Erreur> {
    let refus = |m: &str| Erreur::Url(m.to_string());
    let reste = url.trim().strip_prefix("https://").ok_or_else(|| refus("https:// attendu"))?;
    let reste = reste.split('#').next().unwrap_or("");
    let (chemin, requete) = match reste.split_once('?') {
        Some((c, r)) => (c, r),
        None => (reste, ""),
    };
    let mut parties = chemin.split('/');
    let hote = parties.next().unwrap_or("");
    if hote != "polarsteps.com" && hote != "www.polarsteps.com" {
        return Err(refus("l'adresse n'est pas celle de polarsteps.com"));
    }
    let segments: Vec<&str> = parties.collect();
    // `<qui>/<id>-<nom>`, avec une barre finale permise.
    let voyage: &str = match segments.as_slice() {
        [qui, voyage] | [qui, voyage, ""] if !qui.is_empty() => voyage,
        _ => return Err(refus("attendu : polarsteps.com/<qui>/<numéro>-<nom>")),
    };
    let chiffres: String = voyage.chars().take_while(char::is_ascii_digit).collect();
    let suite = &voyage[chiffres.len()..];
    if chiffres.is_empty() || !(suite.is_empty() || suite.starts_with('-')) {
        return Err(refus("le numéro du voyage manque"));
    }
    let id: u64 = chiffres.parse().map_err(|_| refus("numéro de voyage trop grand"))?;
    let jeton = requete
        .split('&')
        .find_map(|p| p.strip_prefix("s="))
        .map(|j| {
            if j.is_empty() || j.len() > JETON_MAX || !j.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
                Err(refus("le jeton de partage (s=) est illisible"))
            } else {
                Ok(j.to_string())
            }
        })
        .transpose()?;
    Ok(Lien { id, jeton })
}

// ---- le transport ------------------------------------------------------------

/// Ce qu'un GET rend : le statut et au plus `plafond + 1` octets (de quoi
/// savoir qu'on le dépasse, sans le lire en entier).
pub struct Reponse {
    pub statut: u16,
    pub corps: Vec<u8>,
}

/// La seule porte vers le réseau. Un faux la remplace en test : aucun test du
/// gate ne touche le réseau.
pub trait Transport {
    fn get(&self, url: &str, plafond: u64) -> Result<Reponse, String>;
}

/// `Booklibre/<version> (+https://booklibre.app)` : qui frappe à la porte.
pub fn agent_utilisateur() -> String {
    format!("Booklibre/{} (+https://booklibre.app)", env!("CARGO_PKG_VERSION"))
}

/// Le transport réel : le même `ureq` et le même TLS que le relais. Pas de
/// cookie (la fonctionnalité n'est pas compilée), pas de redirection suivie,
/// pas de compression demandée.
pub struct Reseau {
    agent: ureq::Agent,
}

impl Default for Reseau {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .user_agent(agent_utilisateur())
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(90)))
            .build()
            .into();
        Reseau { agent }
    }
}

impl Transport for Reseau {
    fn get(&self, url: &str, plafond: u64) -> Result<Reponse, String> {
        let mut r = self.agent.get(url).call().map_err(|e| e.to_string())?;
        let statut = r.status().as_u16();
        let mut corps = Vec::new();
        r.body_mut()
            .as_reader()
            .take(plafond.saturating_add(1))
            .read_to_end(&mut corps)
            .map_err(|e| e.to_string())?;
        Ok(Reponse { statut, corps })
    }
}

// ---- la conversion -----------------------------------------------------------

/// Une photo à télécharger : de quelle étape, quel rang dans l'étape, quel
/// nom de fichier, quelle adresse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct APrendre {
    pub etape: usize,
    pub photo: usize,
    pub fichier: String,
    pub url: String,
}

/// Le voyage converti, ses photos encore sans dimensions, et la liste de ce
/// qu'il reste à télécharger.
#[derive(Debug)]
pub struct Converti {
    pub voyage: Voyage,
    pub a_prendre: Vec<APrendre>,
}

fn objet<'a>(v: &'a Value, chemin: &str) -> Result<&'a Map<String, Value>, Erreur> {
    v.as_object().ok_or_else(|| inconnu(format!("{chemin} n'est plus un objet")))
}

/// Une chaîne optionnelle : absente ou nulle, c'est `None` ; d'un autre type,
/// c'est un format changé. Rognée, vide vaut absente.
fn texte_opt(o: &Map<String, Value>, cle: &str, chemin: &str) -> Result<Option<String>, Erreur> {
    match o.get(cle) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.trim().to_string()).filter(|s| !s.is_empty())),
        Some(_) => Err(inconnu(format!("{chemin}.{cle} n'est plus un texte"))),
    }
}

fn nombre_opt(o: &Map<String, Value>, cle: &str, chemin: &str) -> Result<Option<f64>, Erreur> {
    match o.get(cle) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(Some).ok_or_else(|| inconnu(format!("{chemin}.{cle} n'est plus un nombre"))),
    }
}

fn instant(v: &Value, chemin: &str) -> Result<DateTime<Utc>, Erreur> {
    let s = v.as_str().ok_or_else(|| inconnu(format!("{chemin} n'est plus une date")))?;
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| inconnu(format!("{chemin} n'est plus une date lisible")))
}

fn instant_opt(o: &Map<String, Value>, cle: &str, chemin: &str) -> Result<Option<DateTime<Utc>>, Erreur> {
    match o.get(cle) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => instant(v, &format!("{chemin}.{cle}")).map(Some),
    }
}

fn coordonnees(l: &Map<String, Value>, chemin: &str) -> Result<(f64, f64), Erreur> {
    let lat = nombre_opt(l, "lat", chemin)?.ok_or_else(|| inconnu(format!("{chemin}.lat manque")))?;
    let lon = nombre_opt(l, "lon", chemin)?.ok_or_else(|| inconnu(format!("{chemin}.lon manque")))?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(inconnu(format!("{chemin} sort du globe")));
    }
    Ok((lat, lon))
}

/// Une adresse de photo : https, un hôte de la liste, une extension d'image
/// connue. Rend l'extension.
fn extension_photo(url: &str, chemin: &str) -> Result<&'static str, Erreur> {
    let reste = url.strip_prefix("https://").ok_or_else(|| inconnu(format!("{chemin} n'est pas en https")))?;
    let (hote, suite) = reste.split_once('/').unwrap_or((reste, ""));
    if !HOTES_PHOTOS.contains(&hote) {
        return Err(inconnu(format!("{chemin} vient d'un hôte inconnu")));
    }
    let chemin_url = suite.split(['?', '#']).next().unwrap_or("");
    let nom = chemin_url.rsplit('/').next().unwrap_or("");
    match nom.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).as_deref() {
        Some("jpg") | Some("jpeg") => Ok("jpg"),
        Some("png") => Ok("png"),
        _ => Err(inconnu(format!("{chemin} ne dit pas son format"))),
    }
}

/// Les huit premiers caractères alphanumériques d'un uuid de média : de quoi
/// rendre le nom stable sans jamais y mettre un séparateur.
fn court(uuid: &str) -> String {
    let c: String = uuid.chars().filter(char::is_ascii_alphanumeric).take(8).collect::<String>().to_ascii_lowercase();
    if c.is_empty() {
        "x".to_string()
    } else {
        c
    }
}

struct EtapeBrute {
    uuid: Option<String>,
    /// L'uuid du média que la source met en avant pour l'étape.
    cover: Option<String>,
    etape: Etape,
    /// (ordre, uuid de média, adresse, extension)
    photos: Vec<(i64, String, String, &'static str)>,
}

pub fn convertir(json: &Value, id: u64, limites: &Limites) -> Result<Converti, Erreur> {
    let racine = objet(json, "la réponse")?;
    match racine.get("id").and_then(Value::as_u64) {
        Some(lu) if lu == id => {}
        Some(_) => return Err(inconnu("la réponse est celle d'un autre voyage")),
        None => return Err(inconnu("id du voyage manque")),
    }
    let titre = texte_opt(racine, "name", "voyage")?
        .or(texte_opt(racine, "display_name", "voyage")?)
        .ok_or_else(|| inconnu("le titre du voyage manque"))?;
    let etapes_json = racine
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| inconnu("steps n'est plus une liste"))?;
    if etapes_json.len() > limites.etapes {
        return Err(Erreur::TropGros("le nombre d'étapes"));
    }

    let mut videos = 0u32;
    let mut brutes: Vec<EtapeBrute> = Vec::with_capacity(etapes_json.len());
    let mut nb_photos = 0usize;
    for (i, e) in etapes_json.iter().enumerate() {
        let chemin = format!("steps[{i}]");
        let o = objet(e, &chemin)?;
        let nom = texte_opt(o, "name", &chemin)?
            .or(texte_opt(o, "display_name", &chemin)?)
            .ok_or_else(|| inconnu(format!("{chemin} n'a pas de nom")))?;
        let debut = instant(o.get("start_time").ok_or_else(|| inconnu(format!("{chemin}.start_time manque")))?, &format!("{chemin}.start_time"))?;
        let fin = instant_opt(o, "end_time", &chemin)?;
        let fuseau = texte_opt(o, "timezone_id", &chemin)?.ok_or_else(|| inconnu(format!("{chemin}.timezone_id manque")))?;
        let lieu_json = objet(o.get("location").ok_or_else(|| inconnu(format!("{chemin}.location manque")))?, &format!("{chemin}.location"))?;
        let (lat, lon) = coordonnees(lieu_json, &format!("{chemin}.location"))?;
        let lieu = Lieu {
            lat,
            lon,
            localite: texte_opt(lieu_json, "locality", &format!("{chemin}.location"))?,
            pays: texte_opt(lieu_json, "country", &format!("{chemin}.location"))?,
            code_pays: texte_opt(lieu_json, "country_code", &format!("{chemin}.location"))?
                .filter(|c| c.len() == 2 && c.bytes().all(|b| b.is_ascii_alphabetic()))
                .map(|c| c.to_ascii_uppercase()),
        };
        let condition = texte_opt(o, "weather_condition", &chemin)?;
        let temperature = nombre_opt(o, "weather_temperature", &chemin)?;
        let meteo = (condition.is_some() || temperature.is_some()).then_some(Meteo { condition, temperature });

        let medias = match o.get("media") {
            None | Some(Value::Null) => &[][..],
            Some(Value::Array(a)) => a.as_slice(),
            Some(_) => return Err(inconnu(format!("{chemin}.media n'est plus une liste"))),
        };
        let mut photos = Vec::new();
        for (j, m) in medias.iter().enumerate() {
            let mchemin = format!("{chemin}.media[{j}]");
            let mo = objet(m, &mchemin)?;
            match mo.get("type").and_then(Value::as_i64) {
                Some(1) => videos += 1,
                Some(0) => {
                    let url = texte_opt(mo, "large_thumbnail_path", &mchemin)?
                        .ok_or_else(|| inconnu(format!("{mchemin}.large_thumbnail_path manque")))?;
                    let ext = extension_photo(&url, &format!("{mchemin}.large_thumbnail_path"))?;
                    let uuid = texte_opt(mo, "uuid", &mchemin)?.unwrap_or_default();
                    let ordre = mo.get("order").and_then(Value::as_i64).unwrap_or(j as i64);
                    photos.push((ordre, uuid, url, ext));
                }
                _ => return Err(inconnu(format!("{mchemin}.type n'est ni photo ni vidéo"))),
            }
        }
        if photos.len() > PHOTOS_PAR_ETAPE {
            return Err(Erreur::TropGros("le nombre de photos d'une étape"));
        }
        nb_photos += photos.len();
        if nb_photos > limites.photos {
            return Err(Erreur::TropGros("le nombre de photos"));
        }
        // L'ordre du média, à égalité l'ordre de la réponse (le tri est stable).
        photos.sort_by_key(|p| p.0);
        brutes.push(EtapeBrute {
            uuid: texte_opt(o, "uuid", &chemin)?,
            cover: texte_opt(o, "cover_media_uuid", &chemin)?,
            etape: Etape {
                nom,
                recit: texte_opt(o, "description", &chemin)?,
                debut,
                fin,
                fuseau,
                lieu,
                meteo,
                photos: Vec::new(),
            },
            photos,
        });
    }
    brutes.sort_by_key(|b| b.etape.debut);

    // Les noms de fichiers : rang d'étape, rang de photo, bout d'uuid. Stables
    // d'un import à l'autre, et dans l'ordre alphabétique du livre.
    let mut a_prendre = Vec::with_capacity(nb_photos);
    let mut rang_par_uuid = std::collections::HashMap::new();
    let mut etapes = Vec::with_capacity(brutes.len());
    let mut couverture: Option<String> = None;
    let mut cover_par_etape: Option<String> = None;
    // La couverture du voyage chez la source : un objet média, qu'on retrouve
    // par l'adresse de sa vignette (son `uuid` n'est pas celui du média de
    // l'étape). Mesuré le 13/11 : c'est la première photo de la première étape.
    let couverture_url: Option<String> = racine
        .get("cover_photo")
        .and_then(Value::as_object)
        .and_then(|c| c.get("large_thumbnail_path"))
        .and_then(Value::as_str)
        .map(str::to_string);
    for (i, mut b) in brutes.into_iter().enumerate() {
        if let Some(u) = &b.uuid {
            rang_par_uuid.insert(u.clone(), i);
        }
        for (n, (_, uuid, url, ext)) in b.photos.into_iter().enumerate() {
            let fichier = format!("e{:03}-p{:02}-{}.{ext}", i + 1, n + 1, court(&uuid));
            // La couverture du voyage : la photo que la source met en avant
            // pour la première étape qui en nomme une et la porte.
            if couverture.is_none() && couverture_url.as_deref() == Some(url.as_str()) {
                couverture = Some(fichier.clone());
            }
            if cover_par_etape.is_none() && !uuid.is_empty() && b.cover.as_deref() == Some(uuid.as_str()) {
                cover_par_etape = Some(fichier.clone());
            }
            b.etape.photos.push(Photo { fichier: fichier.clone(), largeur: 0, hauteur: 0 });
            a_prendre.push(APrendre { etape: i, photo: n, fichier, url });
        }
        etapes.push(b.etape);
    }

    // À défaut, la photo que la source met en avant pour la première étape
    // qui en nomme une (`cover_media_uuid`).
    if couverture.is_none() {
        couverture = cover_par_etape;
    }

    // Les pays, recomptés (D18) : le compteur de la source vaut 0 sur un tour
    // du monde mesuré.
    let mut pays: Vec<String> = Vec::new();
    for c in etapes.iter().filter_map(|e| e.lieu.code_pays.clone()) {
        if !pays.contains(&c) {
            pays.push(c);
        }
    }

    let segments = segments(racine, &rang_par_uuid)?;
    let trace = trace(racine, limites)?;

    let debut = instant_opt(racine, "start_date", "voyage")?.or_else(|| etapes.first().map(|e| e.debut));
    let fin = instant_opt(racine, "end_date", "voyage")?.or_else(|| etapes.last().map(|e| e.fin.unwrap_or(e.debut)));
    let fuseau = texte_opt(racine, "timezone_id", "voyage")?.or_else(|| etapes.first().map(|e| e.fuseau.clone()));
    let distance_km = nombre_opt(racine, "total_km", "voyage")?.filter(|d| *d >= 0.0);

    let voyage = Voyage {
        version: VERSION,
        titre,
        resume: texte_opt(racine, "summary", "voyage")?,
        debut,
        fin,
        fuseau,
        distance_km,
        pays,
        etapes,
        segments,
        trace,
        videos_ignorees: videos,
        couverture,
        provenance: Provenance {
            source: SOURCE.to_string(),
            reference: Some(id.to_string()),
            format: Some(format!("api-{VERSION_API}")),
        },
    };
    Ok(Converti { voyage, a_prendre })
}

/// Les trajets faits (`past_zelda_steps`) ; les trajets prévus ne sont pas
/// un voyage vécu.
fn segments(racine: &Map<String, Value>, rang_par_uuid: &std::collections::HashMap<String, usize>) -> Result<Vec<Segment>, Erreur> {
    let liste = match racine.get("route_segments") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(a)) => a,
        Some(_) => return Err(inconnu("route_segments n'est plus une liste")),
    };
    let mut out = Vec::new();
    for (i, s) in liste.iter().enumerate() {
        let chemin = format!("route_segments[{i}]");
        let o = objet(s, &chemin)?;
        if o.get("type").and_then(Value::as_str) != Some("past_zelda_steps") {
            continue;
        }
        let mode = texte_opt(o, "transport_mode", &chemin)?.filter(|m| m != "none");
        let bout = |type_cle: &str, uuid_cle: &str| -> Option<usize> {
            if o.get(type_cle).and_then(Value::as_str) != Some("step") {
                return None;
            }
            o.get(uuid_cle).and_then(Value::as_str).and_then(|u| rang_par_uuid.get(u)).copied()
        };
        let polyligne = texte_opt(o, "route", &chemin)?.and_then(|r| decoder_polyligne(&r)).unwrap_or_default();
        out.push(Segment {
            mode,
            de: bout("start_object_type", "start_object_uuid"),
            a: bout("end_object_type", "end_object_uuid"),
            polyligne,
        });
    }
    Ok(out)
}

/// La trace du tracker, `zelda_steps` : présente en version 62, retirée plus
/// haut. Absente, elle est vide, et le bilan le dit.
fn trace(racine: &Map<String, Value>, limites: &Limites) -> Result<Vec<PointDeTrace>, Erreur> {
    let liste = match racine.get("zelda_steps") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(a)) => a,
        Some(_) => return Err(inconnu("zelda_steps n'est plus une liste")),
    };
    if liste.len() > limites.trace {
        return Err(Erreur::TropGros("la trace"));
    }
    liste
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let chemin = format!("zelda_steps[{i}]");
            let o = objet(p, &chemin)?;
            let lieu = objet(o.get("location").ok_or_else(|| inconnu(format!("{chemin}.location manque")))?, &format!("{chemin}.location"))?;
            let (lat, lon) = coordonnees(lieu, &format!("{chemin}.location"))?;
            let t = o.get("time").ok_or_else(|| inconnu(format!("{chemin}.time manque")))?;
            Ok(PointDeTrace { lat, lon, instant: instant(t, &format!("{chemin}.time"))? })
        })
        .collect()
}

/// La polyligne encodée au format de Google, précision 1e-5. Les routes
/// préfixées `off_` (hors-piste) ne se lisent pas : `None`, jamais des points
/// inventés.
fn decoder_polyligne(s: &str) -> Option<Vec<Point>> {
    if s.starts_with("off_") || s.len() > (1 << 20) {
        return None;
    }
    let o = s.as_bytes();
    let (mut i, mut lat, mut lon) = (0usize, 0i64, 0i64);
    let mut out = Vec::new();
    while i < o.len() {
        for axe in 0..2 {
            let (mut r, mut decalage) = (0i64, 0u32);
            loop {
                let b = *o.get(i)?;
                i += 1;
                if !(63..=126).contains(&b) {
                    return None;
                }
                let v = i64::from(b - 63);
                r |= (v & 31) << decalage;
                decalage += 5;
                if decalage > 60 {
                    return None;
                }
                if v < 32 {
                    break;
                }
            }
            let d = if r & 1 == 1 { !(r >> 1) } else { r >> 1 };
            if axe == 0 {
                lat += d;
            } else {
                lon += d;
            }
        }
        let (la, lo) = (lat as f64 / 1e5, lon as f64 / 1e5);
        if !(-90.0..=90.0).contains(&la) || !(-180.0..=180.0).contains(&lo) {
            return None;
        }
        out.push(Point { lat: la, lon: lo });
    }
    Some(out)
}

// ---- l'import ----------------------------------------------------------------

/// Une requête, ses garde-fous : la pause avant toute requête sauf la
/// première, le statut 200, le plafond, le jeton masqué dans l'erreur.
struct Demandeur<'a> {
    transport: &'a dyn Transport,
    pause: Duration,
    deja: Cell<bool>,
    jeton: Option<&'a str>,
}

impl Demandeur<'_> {
    fn prendre(&self, url: &str, plafond: u64, quoi: &'static str, depasse: &'static str) -> Result<Vec<u8>, Erreur> {
        if self.deja.replace(true) {
            std::thread::sleep(self.pause);
        }
        let r = self.transport.get(url, plafond).map_err(|m| {
            Erreur::Reseau(match self.jeton {
                Some(j) => m.replace(j, "***"),
                None => m,
            })
        })?;
        if r.statut != 200 {
            return Err(Erreur::Statut { statut: r.statut, quoi });
        }
        if r.corps.len() as u64 > plafond {
            return Err(Erreur::TropGros(depasse));
        }
        Ok(r.corps)
    }
}

fn dimensions(octets: &[u8]) -> Result<(u32, u32), Erreur> {
    image::ImageReader::new(std::io::Cursor::new(octets))
        .with_guessed_format()
        .map_err(|e| Erreur::Photo(e.to_string()))?
        .into_dimensions()
        .map_err(|e| Erreur::Photo(e.to_string()))
}

/// Importe le voyage de `url` dans `vers` : les photos, puis `voyage.json`
/// en dernier. Une photo déjà là (même nom, lisible) n'est pas redemandée :
/// reprendre un import interrompu ne coûte rien à Polarsteps.
/// `progression(faites, total)` suit les photos.
pub fn importer(
    url: &str,
    vers: &Path,
    transport: &dyn Transport,
    reglages: &Reglages,
    progression: &dyn Fn(usize, usize),
) -> Result<Voyage, Erreur> {
    let lien = analyser_url(url)?;
    let limites = &reglages.limites;
    let demandeur = Demandeur { transport, pause: reglages.pause, deja: Cell::new(false), jeton: lien.jeton.as_deref() };

    let mut api = format!("{API}/trips/{}?polarsteps_api_version={VERSION_API}", lien.id);
    if let Some(j) = &lien.jeton {
        api.push_str("&s=");
        api.push_str(j);
    }
    let corps = demandeur.prendre(&api, limites.reponse, "voyage", "la réponse du voyage")?;
    let json: Value = serde_json::from_slice(&corps).map_err(|_| inconnu("la réponse n'est pas du JSON"))?;
    drop(corps);
    let Converti { mut voyage, a_prendre } = convertir(&json, lien.id, limites)?;
    drop(json);

    std::fs::create_dir_all(vers).map_err(|e| Erreur::Disque(e.to_string()))?;
    let total = a_prendre.len();
    let mut recu = 0u64;
    for (n, p) in a_prendre.iter().enumerate() {
        let chemin = vers.join(&p.fichier);
        let (l, h) = match image::image_dimensions(&chemin) {
            Ok(d) => d,
            Err(_) => {
                let octets = demandeur.prendre(&p.url, limites.photo, "une photo", "une photo")?;
                recu += octets.len() as u64;
                if recu > limites.total {
                    return Err(Erreur::TropGros("le total des photos"));
                }
                let d = dimensions(&octets)?;
                let part = vers.join(format!("{}.part", p.fichier));
                std::fs::write(&part, &octets).map_err(|e| Erreur::Disque(e.to_string()))?;
                std::fs::rename(&part, &chemin).map_err(|e| Erreur::Disque(e.to_string()))?;
                d
            }
        };
        let photo = &mut voyage.etapes[p.etape].photos[p.photo];
        photo.largeur = l;
        photo.hauteur = h;
        progression(n + 1, total);
    }
    super::ecrire(vers, &voyage).map_err(|e| Erreur::Disque(e.to_string()))?;
    Ok(voyage)
}

#[cfg(test)]
mod tests;
