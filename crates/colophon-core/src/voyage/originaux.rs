//! Remplacer les vignettes d'un voyage par leurs originaux (P-s8, plan D17).
//!
//! Un voyage importé par son lien n'a que des vignettes de 960 px au côté
//! court, presque toutes sous 250 ppi. Cette étape les apparie aux photos
//! d'origine d'un dossier que la personne désigne, et ne remplace que ce
//! dont elle est **sûre**.
//!
//! **L'appariement.** Une vignette n'a aucune date de prise de vue (la
//! source retire l'EXIF : 0 fichier sur 1 546 mesurés) : seule son étape en
//! a une. L'heure ne peut donc que **borner** le choix, et c'est
//! l'image qui tranche.
//! 1. *Par l'heure.* Les originaux dont l'heure de prise de vue tombe dans la
//!    fenêtre de l'étape. L'heure de la source est en UTC avec un fuseau
//!    d'étape ; celle de l'EXIF est une heure locale **sans fuseau**, lue dans
//!    le fuseau de l'étape (ou dans `OffsetTimeOriginal` quand le boîtier
//!    l'écrit). Ce décalage se règle ici, pas dans une marge.
//! 2. *Par l'image seule*, plus sévère, quand l'heure ne trouve personne (une
//!    photo posée dans une autre étape que celle de sa prise de vue, un
//!    boîtier resté à l'heure de la maison).
//!
//! L'empreinte est maison et déterministe, jamais une IA : la luminance
//! réduite à 24 × 24, centrée et réduite (la luminosité et le contraste ne
//! comptent pas), comparée par l'écart quadratique moyen. Le ratio d'aspect
//! doit être compatible. Un original est **sûr** s'il est sous un seuil de
//! distance **et** si le suivant est nettement plus loin ; une rafale, où
//! deux originaux se ressemblent, n'apparie donc jamais rien.
//!
//! **Le remplacement garde le nom.** Le fichier de l'album reste
//! `e001-p01-…jpg` : place, recadrage, légende, réglages, curation, tout
//! ce qui désigne la photo par son nom la désigne encore, et `album.json`
//! n'est pas touché. La vignette n'est pas effacée : elle part dans un
//! dossier voisin (`<dossier>-vignettes`), et `originaux.json` écrit à côté
//! de `voyage.json` dit qui est devenu quoi, de quel fichier, à quelle
//! distance : [`retablir`] rend les vignettes.
//!
//! **Les originaux qui ne sont pas des JPEG** (HEIC d'un iPhone, PNG)
//! remplacent aussi, sous le même nom : décodés par le chemin du reste de
//! l'app (le décodeur système pour un HEIC, jamais une crate), redressés par
//! leur EXIF, réécrits en JPEG pleine définition ([`QUALITE_JPEG`]) avec la
//! date de prise de vue (et son décalage) quand ils la donnaient. Un JPEG est
//! copié à l'octet, EXIF compris. Un RAW apparié est nommé au bilan, il ne
//! remplace rien ; un HEIC qu'aucun décodeur du système ne lit (Windows,
//! Linux) est compté et dit au bilan, avec la même formule.

use super::Voyage;
use crate::{heic, meta, scan, thumb};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use image::{imageops::FilterType, DynamicImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// Le fichier de correspondance, à côté de `voyage.json`.
pub const FICHIER: &str = "originaux.json";
pub const VERSION: u32 = 1;

/// Le côté de l'empreinte : 24 × 24 valeurs de luminance.
pub const COTE_EMPREINTE: u32 = 24;

/// Distance (écart quadratique moyen de deux empreintes centrées-réduites,
/// 0 = identiques, ~1,4 = sans rapport) sous laquelle un original **de la
/// fenêtre horaire** est le bon. Mesure : `docs/mesures/2026-11-15-les-originaux.json`.
pub const SEUIL_HEURE: f32 = 0.12;
/// Avance minimale du meilleur sur le suivant, même fenêtre.
pub const MARGE_HEURE: f32 = 0.10;
/// Sans l'heure, tout le dossier concourt : seuil et marge plus sévères.
pub const SEUIL_IMAGE: f32 = 0.08;
pub const MARGE_IMAGE: f32 = 0.15;
/// Écart de ratio d'aspect toléré (arrondis de la source compris).
pub const TOLERANCE_RATIO: f64 = 0.025;

/// Avant l'étape et après sa fin : un boîtier mal réglé de quelques heures.
pub const MARGE_HORAIRE_S: i64 = 6 * 3600;
/// Une étape sans fin dure au moins un jour pour la fenêtre : la photo posée
/// dans une étape est souvent prise plus tard dans la journée.
pub const DUREE_ETAPE_MIN_S: i64 = 24 * 3600;
/// La qualité du JPEG écrit pour un original HEIC ou PNG : celle de
/// l'éditeur (`reglage.rs`) en plus haut, pour que le remplacement ne perde
/// rien de visible.
pub const QUALITE_JPEG: u8 = 95;

// ---------------------------------------------------------------- l'empreinte

#[derive(Debug, Clone)]
pub struct Empreinte {
    v: Vec<f32>,
    /// Image sans relief (noir, blanc, ciel nu) : elle ne ressemble à rien en
    /// particulier, donc à tout. Elle n'apparie jamais.
    plate: bool,
}

/// L'empreinte d'une image déjà orientée. Même chemin pour une vignette et un
/// original : réduite à 128 px, puis à 24 × 24.
pub fn empreinte(img: &DynamicImage) -> Empreinte {
    let petit = img.thumbnail(128, 128).to_luma8();
    let s = image::imageops::resize(&petit, COTE_EMPREINTE, COTE_EMPREINTE, FilterType::Triangle);
    let n = s.len() as f32;
    let moyenne = s.iter().map(|p| *p as f32).sum::<f32>() / n;
    let variance = s.iter().map(|p| (*p as f32 - moyenne).powi(2)).sum::<f32>() / n;
    let ecart = variance.sqrt();
    if ecart < 2.0 {
        return Empreinte {
            v: vec![0.0; s.len()],
            plate: true,
        };
    }
    Empreinte {
        v: s.iter().map(|p| (*p as f32 - moyenne) / ecart).collect(),
        plate: false,
    }
}

/// 0 pour deux images identiques, ~1,4 pour deux images sans rapport.
/// Une image plate n'est proche de rien.
pub fn distance(a: &Empreinte, b: &Empreinte) -> f32 {
    if a.plate || b.plate {
        return f32::INFINITY;
    }
    let somme: f32 = a.v.iter().zip(&b.v).map(|(x, y)| (x - y) * (x - y)).sum();
    (somme / a.v.len() as f32).sqrt()
}

fn ratio_compatible(l1: u32, h1: u32, l2: u32, h2: u32) -> bool {
    if l1 == 0 || h1 == 0 || l2 == 0 || h2 == 0 {
        return false;
    }
    let r1 = l1 as f64 / h1 as f64;
    let r2 = l2 as f64 / h2 as f64;
    (r1 / r2 - 1.0).abs() <= TOLERANCE_RATIO
}

// ------------------------------------------------------------------ l'index

/// Une photo du dossier d'originaux, lue une fois.
#[derive(Debug, Clone)]
pub struct Original {
    pub chemin: PathBuf,
    /// Dimensions orientées, comme la page les verra.
    pub largeur: u32,
    pub hauteur: u32,
    /// L'heure locale de la prise de vue, sans fuseau, quand l'EXIF la donne.
    pub heure: Option<NaiveDateTime>,
    /// Le décalage à UTC en secondes que le boîtier a écrit
    /// (`OffsetTimeOriginal`), quand il l'a écrit.
    pub decalage_s: Option<i32>,
    pub empreinte: Empreinte,
}

/// Le décalage `+02:00` d'`OffsetTimeOriginal`, en secondes.
fn decalage_exif(chemin: &Path) -> Option<i32> {
    let f = std::fs::File::open(chemin).ok()?;
    let exif = exif::Reader::new()
        .read_from_container(&mut std::io::BufReader::new(f))
        .ok()?;
    let champ = exif.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY)?;
    let texte = champ.display_value().to_string();
    let texte = texte.trim().trim_matches('"');
    let (signe, reste) = match texte.as_bytes().first()? {
        b'+' => (1, &texte[1..]),
        b'-' => (-1, &texte[1..]),
        _ => return None,
    };
    let (h, m) = reste.split_once(':')?;
    let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
    (h <= 14 && m < 60).then_some(signe * (h * 3600 + m * 60))
}

fn lire_original(chemin: &Path) -> Result<Original> {
    let m = meta::read(chemin);
    let img = heic::open(chemin)?;
    let img = thumb::apply_orientation(img, m.orientation);
    Ok(Original {
        chemin: chemin.to_path_buf(),
        largeur: img.width(),
        hauteur: img.height(),
        heure: m.taken_reliable.then_some(m.taken),
        decalage_s: decalage_exif(chemin),
        empreinte: empreinte(&img),
    })
}

/// Lit tout le dossier d'originaux (récursif, formats de l'album) sauf ce qui
/// est dans `exclus` : le dossier du voyage et ses vignettes ne sont pas des
/// originaux. Rend aussi le nombre de fichiers illisibles et celui des HEIC
/// que le système ne sait pas lire (le scan les écarte, ils ne sont jamais
/// des originaux possibles).
pub fn indexer(
    dossier: &Path,
    exclus: &[PathBuf],
    progres: &(dyn Fn(usize, usize) + Sync),
    annuler: &(dyn Fn() -> bool + Sync),
) -> Result<(Vec<Original>, usize, usize)> {
    let exclus: Vec<PathBuf> = exclus
        .iter()
        .filter_map(|e| e.canonicalize().ok())
        .collect();
    let balayage = scan::scan(dossier);
    let heic_illisibles = balayage.skipped_heic;
    let fichiers: Vec<PathBuf> = balayage
        .images
        .into_iter()
        .filter(|p| match p.canonicalize() {
            Ok(c) => !exclus.iter().any(|e| c.starts_with(e)),
            Err(_) => false,
        })
        .collect();
    let total = fichiers.len();
    let faits = std::sync::atomic::AtomicUsize::new(0);
    let lus: Vec<Option<Original>> = fichiers
        .par_iter()
        .map(|p| {
            if annuler() {
                return None;
            }
            let r = lire_original(p).ok();
            let n = faits.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n % 25 == 0 || n == total {
                progres(n, total);
            }
            r
        })
        .collect();
    if annuler() {
        bail!("annulé");
    }
    let illisibles = lus.iter().filter(|o| o.is_none()).count();
    let mut originaux: Vec<Original> = lus.into_iter().flatten().collect();
    originaux.sort_by(|a, b| a.chemin.cmp(&b.chemin));
    Ok((originaux, illisibles, heic_illisibles))
}

// ----------------------------------------------------------- l'appariement

/// Une vignette à apparier : ce qu'on sait d'elle et de son étape.
#[derive(Debug, Clone)]
pub struct Vignette {
    pub fichier: String,
    pub largeur: u32,
    pub hauteur: u32,
    /// `None` : fichier illisible.
    pub empreinte: Option<Empreinte>,
    /// La fenêtre de prise de vue de l'étape, en UTC.
    pub fenetre: (DateTime<Utc>, DateTime<Utc>),
    /// Le fuseau de l'étape.
    pub fuseau: String,
}

/// La fenêtre d'une étape : de son début à sa fin (au moins un jour), plus la
/// marge d'un boîtier mal réglé.
pub fn fenetre_de(
    debut: DateTime<Utc>,
    fin: Option<DateTime<Utc>>,
) -> (DateTime<Utc>, DateTime<Utc>) {
    let marge = chrono::Duration::seconds(MARGE_HORAIRE_S);
    let au_moins = debut + chrono::Duration::seconds(DUREE_ETAPE_MIN_S);
    let fin = fin.map_or(au_moins, |f| f.max(au_moins));
    (debut - marge, fin + marge)
}

/// L'heure d'un original en UTC : son décalage s'il l'a écrit, sinon l'heure
/// locale lue dans le fuseau de l'étape. Un fuseau inconnu se lit en UTC.
pub fn instant_de(o: &Original, fuseau: &str) -> Option<DateTime<Utc>> {
    let h = o.heure?;
    if let Some(s) = o.decalage_s {
        return DateTime::from_timestamp(h.and_utc().timestamp() - i64::from(s), 0);
    }
    use chrono::{Datelike, Timelike};
    let civil = jiff::civil::DateTime::new(
        i16::try_from(h.year()).ok()?,
        h.month() as i8,
        h.day() as i8,
        h.hour() as i8,
        h.minute() as i8,
        h.second() as i8,
        0,
    )
    .ok()?;
    let ts = civil
        .in_tz(fuseau)
        .or_else(|_| civil.in_tz("UTC"))
        .ok()?
        .timestamp()
        .as_second();
    DateTime::from_timestamp(ts, 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Methode {
    Heure,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Raison {
    /// Rien dans le dossier ne ressemble à cette vignette.
    SansOriginal,
    /// Plusieurs originaux se ressemblent : une rafale, rien n'est sûr.
    Ambigu,
    /// Deux vignettes ont désigné le même original.
    Conflit,
    /// L'original n'est pas plus grand que la vignette : rien à gagner.
    PasPlusGrand,
    /// L'original est sûr mais c'est un RAW : le nom de la photo dans l'album
    /// ne peut pas changer d'extension, et un RAW ne se réécrit pas en JPEG
    /// sans un développement que la personne n'a pas choisi.
    Format,
    /// La vignette elle-même ne se lit pas.
    Illisible,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appariement {
    /// Le rang de l'original dans l'index.
    pub original: usize,
    pub distance: f32,
    /// L'avance sur le suivant ; `None` quand il n'y en avait pas.
    pub marge: Option<f32>,
    pub methode: Methode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    Sur(Appariement),
    Refus(Raison),
}

fn est_jpeg(p: &Path) -> bool {
    matches!(
        p.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .as_deref(),
        Some("jpg" | "jpeg")
    )
}

/// Ce qui remplace : un JPEG (copié) ou un format qu'on réécrit en JPEG
/// (HEIC, PNG). Un RAW, non.
fn remplacable(p: &Path) -> bool {
    !heic::is_raw(p)
}

/// Meilleur et suivant parmi des candidats (index, distance).
fn deux_meilleurs(mut c: Vec<(usize, f32)>) -> Option<((usize, f32), Option<f32>)> {
    c.retain(|(_, d)| d.is_finite());
    c.sort_by(|a, b| a.1.total_cmp(&b.1));
    let premier = *c.first()?;
    Some((premier, c.get(1).map(|s| s.1)))
}

/// Le verdict d'une vignette. Pur : ni disque, ni horloge.
pub fn apparier_une(v: &Vignette, originaux: &[Original]) -> Verdict {
    let Some(emp) = &v.empreinte else {
        return Verdict::Refus(Raison::Illisible);
    };
    let compatibles = |o: &Original| ratio_compatible(v.largeur, v.hauteur, o.largeur, o.hauteur);
    let dist = |i: usize, o: &Original| (i, distance(emp, &o.empreinte));

    // 1. L'heure borne, l'image tranche.
    let dans_la_fenetre: Vec<(usize, f32)> = originaux
        .iter()
        .enumerate()
        .filter(|(_, o)| compatibles(o))
        .filter(|(_, o)| {
            instant_de(o, &v.fuseau).is_some_and(|t| t >= v.fenetre.0 && t <= v.fenetre.1)
        })
        .map(|(i, o)| dist(i, o))
        .collect();
    let mut dernier_refus = Raison::SansOriginal;
    if let Some(((i, d), suivant)) = deux_meilleurs(dans_la_fenetre) {
        if d <= SEUIL_HEURE {
            let marge = suivant.map(|s| s - d);
            if marge.is_none_or(|m| m >= MARGE_HEURE) {
                return Verdict::Sur(Appariement {
                    original: i,
                    distance: d,
                    marge,
                    methode: Methode::Heure,
                });
            }
            // Deux candidats se ressemblent : le dossier entier ne fera pas
            // mieux, il aura les mêmes.
            return Verdict::Refus(Raison::Ambigu);
        }
    }

    // 2. Sans l'heure : tout le dossier, plus sévère.
    let tous: Vec<(usize, f32)> = originaux
        .iter()
        .enumerate()
        .filter(|(_, o)| compatibles(o))
        .map(|(i, o)| dist(i, o))
        .collect();
    if let Some(((i, d), suivant)) = deux_meilleurs(tous) {
        if d <= SEUIL_IMAGE {
            let marge = suivant.map(|s| s - d);
            if marge.is_none_or(|m| m >= MARGE_IMAGE) {
                return Verdict::Sur(Appariement {
                    original: i,
                    distance: d,
                    marge,
                    methode: Methode::Image,
                });
            }
            dernier_refus = Raison::Ambigu;
        }
    }
    Verdict::Refus(dernier_refus)
}

/// Tous les verdicts : l'appariement, puis les garde-fous qui ne dépendent
/// que d'un original (remplaçable, plus grand) et celui qui dépend des autres
/// vignettes (deux vignettes, un même original : aucune des deux).
pub fn apparier(vignettes: &[Vignette], originaux: &[Original]) -> Vec<Verdict> {
    let mut verdicts: Vec<Verdict> = vignettes
        .par_iter()
        .map(|v| apparier_une(v, originaux))
        .collect();
    for (v, verdict) in vignettes.iter().zip(verdicts.iter_mut()) {
        if let Verdict::Sur(a) = *verdict {
            let o = &originaux[a.original];
            if o.largeur.min(o.hauteur) <= v.largeur.min(v.hauteur) {
                *verdict = Verdict::Refus(Raison::PasPlusGrand);
            } else if !remplacable(&o.chemin) {
                *verdict = Verdict::Refus(Raison::Format);
            }
        }
    }
    let mut par_original: HashMap<usize, usize> = HashMap::new();
    for v in &verdicts {
        if let Verdict::Sur(a) = v {
            *par_original.entry(a.original).or_default() += 1;
        }
    }
    for v in verdicts.iter_mut() {
        if let Verdict::Sur(a) = *v {
            if par_original[&a.original] > 1 {
                *v = Verdict::Refus(Raison::Conflit);
            }
        }
    }
    verdicts
}

// ------------------------------------------------------ la correspondance

/// Ce qui est devenu quoi : lisible, réparable à la main.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Remplacement {
    /// Le nom dans l'album, qui ne change pas.
    pub fichier: String,
    /// Où la vignette a été rangée, relatif au dossier du voyage.
    pub vignette: String,
    pub original: String,
    pub methode: Methode,
    pub distance: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marge: Option<f32>,
    /// Largeur, hauteur de la vignette puis de l'original.
    pub avant: [u32; 2],
    pub apres: [u32; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Refus {
    pub fichier: String,
    pub etape: String,
    pub raison: Raison,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Correspondance {
    pub version: u32,
    /// Le dossier d'originaux désigné la dernière fois.
    pub dossier: String,
    pub remplacements: Vec<Remplacement>,
    /// Les vignettes laissées telles quelles, au dernier passage.
    pub refus: Vec<Refus>,
}

pub fn lire_correspondance(dossier: &Path) -> Option<Correspondance> {
    let texte = std::fs::read_to_string(dossier.join(FICHIER)).ok()?;
    serde_json::from_str(&texte)
        .ok()
        .filter(|c: &Correspondance| c.version <= VERSION)
}

fn ecrire_correspondance(dossier: &Path, c: &Correspondance) -> Result<()> {
    let tmp = dossier.join(format!("{FICHIER}.part"));
    std::fs::write(&tmp, serde_json::to_string_pretty(c)?)?;
    std::fs::rename(&tmp, dossier.join(FICHIER))?;
    Ok(())
}

/// Le dossier voisin où partent les vignettes remplacées.
pub fn dossier_des_vignettes(dossier: &Path) -> Result<PathBuf> {
    let nom = dossier
        .file_name()
        .context("dossier sans nom")?
        .to_string_lossy()
        .to_string();
    Ok(dossier.with_file_name(format!("{nom}-vignettes")))
}

/// Ce que l'écran dit : combien, lesquelles, et ce que ça a changé.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Rapport {
    /// Remplacées à ce passage.
    pub remplacees: usize,
    /// Remplacées à un passage précédent, non redemandées.
    pub deja: usize,
    /// Les vignettes sans original sûr, avec la raison, étape comprise.
    pub refus: Vec<Refus>,
    /// Les photos du dossier d'originaux qu'on n'a pas pu lire.
    pub illisibles: usize,
    /// Les HEIC du dossier que ce système ne sait pas lire : ils ne peuvent
    /// remplacer personne, et le bilan le dit.
    pub heic_illisibles: usize,
    /// Parmi les remplacées, celles qu'on a réécrites en JPEG (HEIC, PNG).
    pub reecrites: usize,
    /// Photos sous le repère de 2 126 px, avant puis après.
    pub sous_seuil_avant: usize,
    pub sous_seuil_apres: usize,
    /// Parmi les remplacées, celles que la composition avait laissées à la
    /// réserve (`curation.json` : `definition` ou `hors_budget`) : une
    /// recomposition peut en ramener dans le livre, jamais d'office.
    pub revenues_possibles: usize,
}

/// Ce que `retablir` rend.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Retablissement {
    pub vignettes_rendues: usize,
}

fn deplacer(de: &Path, vers: &Path) -> std::io::Result<()> {
    if std::fs::rename(de, vers).is_ok() {
        return Ok(());
    }
    std::fs::copy(de, vers)?;
    std::fs::remove_file(de)
}

/// Les vignettes d'un voyage : celles de `voyage.json`, lues une fois.
fn vignettes_du_voyage(
    dossier: &Path,
    voyage: &Voyage,
    deja: &[String],
) -> Vec<(Vignette, String)> {
    let mut v = Vec::new();
    for e in &voyage.etapes {
        let fenetre = fenetre_de(e.debut, e.fin);
        for p in &e.photos {
            if deja.contains(&p.fichier) {
                continue;
            }
            let emp = image::open(dossier.join(&p.fichier))
                .ok()
                .map(|i| empreinte(&i));
            v.push((
                Vignette {
                    fichier: p.fichier.clone(),
                    largeur: p.largeur,
                    hauteur: p.hauteur,
                    empreinte: emp,
                    fenetre,
                    fuseau: e.fuseau.clone(),
                },
                e.nom.clone(),
            ));
        }
    }
    v
}

/// Les entrées de `thumbs.json` des photos changées pointent encore sur la
/// vignette en cache : on les refait, sinon l'éditeur montrerait l'ancienne
/// image et croirait la photo encore petite.
fn rafraichir_cache(album: &Path, dossier: &Path, fichiers: &[String]) -> Result<()> {
    let chemin = album.join("thumbs.json");
    let Ok(texte) = std::fs::read_to_string(&chemin) else {
        return Ok(());
    };
    let mut index: BTreeMap<String, String> =
        serde_json::from_str(&texte).context("thumbs.json illisible")?;
    let cache = thumb::ThumbCache::new(album)?;
    for f in fichiers {
        if !index.contains_key(f) {
            continue;
        }
        let p = dossier.join(f);
        let orientation = meta::read(&p).orientation;
        cache.get(&p, orientation)?;
        if let Some(nom) = cache.path_for(&p).file_name() {
            index.insert(f.clone(), nom.to_string_lossy().to_string());
        }
    }
    std::fs::write(&chemin, serde_json::to_string_pretty(&index)?)?;
    Ok(())
}

/// Les photos que la composition a laissées à la réserve faute de place pour
/// leur taille : `definition` (trop petites pour la plus petite case) et
/// `hors_budget` (où le composer range aussi les « trop petites » d'un voyage,
/// `layout::Composer::trop_petites`). Les jumeaux, panoramas, doublons et
/// rejets sont des décisions que l'original ne change pas.
fn ecartees_pour_la_taille(album: Option<&Path>) -> Vec<String> {
    let Some(a) = album else { return Vec::new() };
    let Ok(t) = std::fs::read_to_string(a.join("curation.json")) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<crate::model::Discard>>(&t)
        .map(|d| {
            d.into_iter()
                .filter(|x| matches!(x.reason.as_str(), "definition" | "hors_budget"))
                .map(|x| x.src)
                .collect()
        })
        .unwrap_or_default()
}

/// Le segment APP1 d'un EXIF minimal : la date de prise de vue
/// (`DateTimeOriginal`, heure locale sans fuseau) et, si le boîtier l'avait
/// écrit, son décalage (`OffsetTimeOriginal`). Rien d'autre : l'orientation
/// est déjà appliquée aux pixels.
fn segment_exif(heure: NaiveDateTime, decalage_s: Option<i32>) -> Vec<u8> {
    let mut champs: Vec<(u16, Vec<u8>)> = Vec::new();
    let mut h = heure.format("%Y:%m:%d %H:%M:%S").to_string().into_bytes();
    h.push(0);
    champs.push((0x9003, h));
    if let Some(s) = decalage_s {
        let (signe, a) = if s < 0 { ('-', -s) } else { ('+', s) };
        let mut o = format!("{signe}{:02}:{:02}", a / 3600, a % 3600 / 60).into_bytes();
        o.push(0);
        champs.push((0x9011, o));
    }
    // TIFF petit-boutiste : IFD0 (un pointeur vers l'IFD Exif), puis l'IFD Exif.
    let exif_ifd = 8 + 2 + 12 + 4;
    let mut donnees_debut = exif_ifd + 2 + champs.len() as u32 * 12 + 4;
    let mut tiff = b"II*\0".to_vec();
    tiff.extend(8u32.to_le_bytes());
    tiff.extend(1u16.to_le_bytes());
    for v in [
        0x8769u16.to_le_bytes().as_slice(),
        &4u16.to_le_bytes(),
        &1u32.to_le_bytes(),
        &exif_ifd.to_le_bytes(),
    ] {
        tiff.extend(v);
    }
    tiff.extend(0u32.to_le_bytes());
    tiff.extend((champs.len() as u16).to_le_bytes());
    let mut donnees: Vec<u8> = Vec::new();
    for (tag, octets) in &champs {
        tiff.extend(tag.to_le_bytes());
        tiff.extend(2u16.to_le_bytes());
        tiff.extend((octets.len() as u32).to_le_bytes());
        tiff.extend(donnees_debut.to_le_bytes());
        donnees_debut += octets.len() as u32;
        donnees.extend(octets.iter());
    }
    tiff.extend(0u32.to_le_bytes());
    tiff.extend(donnees);
    let mut seg = vec![0xFF, 0xE1];
    seg.extend(((2 + 6 + tiff.len()) as u16).to_be_bytes());
    seg.extend(b"Exif\0\0");
    seg.extend(tiff);
    seg
}

/// Écrit l'original (HEIC, PNG…) en JPEG pleine définition à `vers` : décodé
/// comme partout dans l'app, redressé par son EXIF, un PNG transparent posé sur
/// du blanc. Aucun profil n'est écrit : les pixels sont ceux que le décodeur
/// rend, lus comme du sRGB par le reste de la chaîne, comme pour tout HEIC.
fn ecrire_jpeg(o: &Original, vers: &Path) -> Result<()> {
    let m = meta::read(&o.chemin);
    let img = thumb::apply_orientation(heic::open(&o.chemin)?, m.orientation);
    let rgba = img.to_rgba8();
    let (l, h) = rgba.dimensions();
    let mut rgb = image::RgbImage::new(l, h);
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = u32::from(p.0[3]);
        let sur_blanc = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
        rgb.put_pixel(
            x,
            y,
            image::Rgb([sur_blanc(p.0[0]), sur_blanc(p.0[1]), sur_blanc(p.0[2])]),
        );
    }
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, QUALITE_JPEG)
        .encode_image(&rgb)?;
    let mut out = vec![0xFF, 0xD8];
    if let Some(h) = o.heure {
        out.extend(segment_exif(h, o.decalage_s));
    }
    out.extend(&jpeg[2..]);
    std::fs::write(vers, out).with_context(|| format!("écrire {}", vers.display()))
}

/// Apparie les vignettes de `dossier` aux photos de `originaux` et remplace,
/// sur place et sans rien effacer, celles dont l'original est sûr.
/// `album` : le dossier de l'album, pour rafraîchir ses vignettes en cache et
/// dire combien de photos de la réserve peuvent revenir ; absent, rien de
/// cela. Un voyage déjà passé n'est pas refait : seul ce qui reste se tente.
pub fn remplacer(
    dossier: &Path,
    originaux: &Path,
    album: Option<&Path>,
    progres: &(dyn Fn(usize, usize) + Sync),
    annuler: &(dyn Fn() -> bool + Sync),
) -> Result<Rapport> {
    let mut voyage = super::lire(dossier).map_err(|e| anyhow::anyhow!("{e}"))?;
    let archive = dossier_des_vignettes(dossier)?;
    let mut corr = lire_correspondance(dossier).unwrap_or(Correspondance {
        version: VERSION,
        dossier: String::new(),
        remplacements: Vec::new(),
        refus: Vec::new(),
    });
    let deja: Vec<String> = corr
        .remplacements
        .iter()
        .map(|r| r.fichier.clone())
        .collect();
    let sous_seuil_avant = voyage.bilan().photos_sous_seuil;

    let (index, illisibles, heic_illisibles) = indexer(
        originaux,
        &[dossier.to_path_buf(), archive.clone()],
        progres,
        annuler,
    )?;
    let vignettes = vignettes_du_voyage(dossier, &voyage, &deja);
    let seules: Vec<Vignette> = vignettes.iter().map(|(v, _)| v.clone()).collect();
    let verdicts = apparier(&seules, &index);

    std::fs::create_dir_all(&archive).with_context(|| format!("créer {}", archive.display()))?;
    let ecartees = ecartees_pour_la_taille(album);
    let mut rapport = Rapport {
        deja: deja.len(),
        illisibles,
        heic_illisibles,
        sous_seuil_avant,
        ..Rapport::default()
    };
    let mut refus = Vec::new();
    let mut changes = Vec::new();
    for ((v, etape), verdict) in vignettes.iter().zip(&verdicts) {
        if annuler() {
            break;
        }
        match verdict {
            Verdict::Refus(raison) => refus.push(Refus {
                fichier: v.fichier.clone(),
                etape: etape.clone(),
                raison: *raison,
            }),
            Verdict::Sur(a) => {
                let o = &index[a.original];
                let cible = dossier.join(&v.fichier);
                let part = dossier.join(format!("{}.part", v.fichier));
                let rangee = archive.join(&v.fichier);
                if est_jpeg(&o.chemin) {
                    std::fs::copy(&o.chemin, &part)
                        .with_context(|| format!("copier {}", o.chemin.display()))?;
                } else {
                    if let Err(e) = ecrire_jpeg(o, &part) {
                        let _ = std::fs::remove_file(&part);
                        return Err(e.context(format!("réécrire {}", o.chemin.display())));
                    }
                    rapport.reecrites += 1;
                }
                deplacer(&cible, &rangee).with_context(|| format!("ranger {}", cible.display()))?;
                std::fs::rename(&part, &cible)?;
                corr.remplacements.push(Remplacement {
                    fichier: v.fichier.clone(),
                    vignette: format!(
                        "../{}/{}",
                        archive.file_name().unwrap_or_default().to_string_lossy(),
                        v.fichier
                    ),
                    original: o.chemin.to_string_lossy().to_string(),
                    methode: a.methode,
                    distance: a.distance,
                    marge: a.marge,
                    avant: [v.largeur, v.hauteur],
                    apres: [o.largeur, o.hauteur],
                });
                for e in voyage.etapes.iter_mut() {
                    for p in e.photos.iter_mut().filter(|p| p.fichier == v.fichier) {
                        p.largeur = o.largeur;
                        p.hauteur = o.hauteur;
                    }
                }
                if ecartees.contains(&v.fichier) {
                    rapport.revenues_possibles += 1;
                }
                changes.push(v.fichier.clone());
                rapport.remplacees += 1;
                // Une interruption ne perd pas la trace : le fichier de
                // correspondance suit les remplacements, par lots.
                if changes.len() % 25 == 0 {
                    super::ecrire(dossier, &voyage).map_err(|e| anyhow::anyhow!("{e}"))?;
                    ecrire_correspondance(dossier, &corr)?;
                }
            }
        }
    }
    corr.dossier = originaux.to_string_lossy().to_string();
    corr.refus = refus.clone();
    super::ecrire(dossier, &voyage).map_err(|e| anyhow::anyhow!("{e}"))?;
    ecrire_correspondance(dossier, &corr)?;
    if let Some(a) = album {
        rafraichir_cache(a, dossier, &changes)?;
    }
    rapport.sous_seuil_apres = voyage.bilan().photos_sous_seuil;
    rapport.refus = refus;
    if annuler() {
        bail!("annulé");
    }
    Ok(rapport)
}

/// Rend les vignettes : chaque fichier remplacé redevient ce qu'il était,
/// `voyage.json` retrouve ses dimensions, la correspondance disparaît.
pub fn retablir(dossier: &Path, album: Option<&Path>) -> Result<Retablissement> {
    let corr = lire_correspondance(dossier)
        .context("aucun remplacement à rétablir (originaux.json absent)")?;
    let mut voyage = super::lire(dossier).map_err(|e| anyhow::anyhow!("{e}"))?;
    let archive = dossier_des_vignettes(dossier)?;
    let mut rendues = 0;
    let mut fichiers = Vec::new();
    for r in &corr.remplacements {
        // Le nom vient d'un fichier qu'une main a pu changer : un nom, rien d'autre.
        if !thumb::nom_propre(&r.fichier) {
            bail!("originaux.json : nom refusé ({})", r.fichier);
        }
        let rangee = archive.join(&r.fichier);
        if !rangee.exists() {
            bail!(
                "la vignette {} n'est plus dans {}",
                r.fichier,
                archive.display()
            );
        }
        let cible = dossier.join(&r.fichier);
        std::fs::rename(&rangee, &cible).or_else(|_| {
            std::fs::copy(&rangee, &cible)?;
            std::fs::remove_file(&rangee)
        })?;
        for e in voyage.etapes.iter_mut() {
            for p in e.photos.iter_mut().filter(|p| p.fichier == r.fichier) {
                p.largeur = r.avant[0];
                p.hauteur = r.avant[1];
            }
        }
        fichiers.push(r.fichier.clone());
        rendues += 1;
    }
    super::ecrire(dossier, &voyage).map_err(|e| anyhow::anyhow!("{e}"))?;
    std::fs::remove_file(dossier.join(FICHIER))?;
    let _ = std::fs::remove_dir(&archive);
    if let Some(a) = album {
        rafraichir_cache(a, dossier, &fichiers)?;
    }
    Ok(Retablissement {
        vignettes_rendues: rendues,
    })
}
