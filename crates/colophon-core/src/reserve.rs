//! La réserve classée : les photos du dossier qu'aucune planche ne montre,
//! rangées pour une planche donnée (chantier E, décision E3).
//!
//! Le classement vit ici parce que les mesures y vivent : le dHash et le
//! score d'une photo se lisent dans le relevé quand l'album en porte un, et
//! sur ses vignettes sinon — les deux chemins de `audit::mesure_photos`, et
//! pas un troisième chiffre. L'app ne reçoit que des codes (`Raison`) : le
//! libellé est à elle.
//!
//! **C'est une aide à la main, et la main décide** (E5) : rien d'ici n'entre
//! dans le Composer, et une photo écartée par la curation reste candidate.

use crate::analyze;
use crate::audit::DUP_HAMMING;
use crate::model::{default_focal, Album, Discard, Slot};
use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveDateTime};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Douze au plus : un popover de vignettes, pas un second tiroir.
pub const AU_PLUS: usize = 12;

/// Pourquoi une photo est là où elle est dans la liste. Un code, jamais une
/// phrase : la phrase est à l'app, dans sa langue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Raison {
    /// Prise le jour de la planche.
    MemeJour,
    /// Prise un autre jour ; les plus proches d'abord.
    Proche,
    /// Sans date à comparer : classée sur son score seul.
    Nette,
}

/// Une photo de la réserve, telle que le popover la montre.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    /// Relative à la racine, comme `Slot::src`.
    pub src: String,
    /// `Analysis::score`, la formule du relevé.
    pub score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taken: Option<NaiveDateTime>,
    pub raison: Raison,
    /// Le point focal qu'elle gardera une fois posée : celui que la curation
    /// lui a trouvé (un visage), sinon le défaut du Composer. Sans lui l'app
    /// devrait relire `curation.json` pour construire la case.
    pub focal: [f64; 2],
    /// Ce qu'elle imprimerait dans la case qu'on remplace, en ppi, au cadrage
    /// par défaut (`print::effective_ppi`, zoom 1). Absent sans case cible,
    /// et quand la taille de l'original n'est pas connue : rien ne se devine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ppi: Option<f64>,
}

/// Ce que la commande rend : la liste, et ce qu'il faut savoir d'elle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Reserve {
    pub candidats: Vec<Candidate>,
    /// Ce qui a manqué pour classer tout à fait (pas de date, vignette
    /// illisible), dit plutôt que tu. Absent quand rien n'a manqué.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Ce que le classement lit d'une photo. Une fiche réduite à trois
/// nombres, pour que le classement se teste sans vignette ni relevé.
#[derive(Debug, Clone, PartialEq)]
pub struct Fait {
    pub dhash: u64,
    pub score: f64,
    /// La prise de vue quand elle est fiable (EXIF ou sidecar), jamais un
    /// mtime : une date de fichier classerait sur l'ordre de copie.
    pub taken: Option<NaiveDateTime>,
    /// La taille de l'original, orientée (`Photo::orig`), quand on la sait.
    pub orig: Option<(u32, u32)>,
}

/// Une photo que l'album ne montre pas, avec le focal qu'elle emportera.
#[derive(Debug, Clone, PartialEq)]
pub struct EnReserve {
    pub src: String,
    pub focal: [f64; 2],
}

/// Le jour qu'une planche raconte : la date médiane de ses photos datées.
/// Une planche sans photo datée (page de texte, planche vide) prend le jour
/// de ses voisines, les plus proches d'abord, les deux côtés confondus. Rien
/// nulle part : `None`, et la réserve se classe sur le score seul.
pub fn jour_de_reference(
    album: &Album,
    planche: usize,
    date_de: &dyn Fn(&str) -> Option<NaiveDateTime>,
) -> Option<NaiveDate> {
    let n = album.spreads.len();
    if planche >= n {
        return None;
    }
    let jours = |i: usize| -> Vec<NaiveDate> {
        album.spreads[i]
            .slots
            .iter()
            .filter_map(|s| date_de(&s.src).map(|d| d.date()))
            .collect()
    };
    for distance in 0..n {
        let mut dates = Vec::new();
        if let Some(i) = planche.checked_sub(distance) {
            dates.extend(jours(i));
        }
        if distance > 0 && planche + distance < n {
            dates.extend(jours(planche + distance));
        }
        if let Some(m) = mediane(dates) {
            return Some(m);
        }
    }
    None
}

fn mediane(mut dates: Vec<NaiveDate>) -> Option<NaiveDate> {
    if dates.is_empty() {
        return None;
    }
    dates.sort();
    Some(dates[dates.len() / 2])
}

/// Le classement lui-même, pur et en mémoire. Dans l'ordre : les photos du
/// jour de la planche, puis par écart de date croissant, à égalité de jour
/// par score décroissant ; les photos sans date ferment la marche, par score.
/// Écartées : les quasi-doublons d'une photo déjà posée sur la planche, au
/// seuil dHash de la curation, et les photos dont rien n'a été mesuré (une
/// illisible n'a pas de dHash, donc pas de place).
///
/// Avec une case cible (`cellule`, « Remplacer par… »), celles qui la
/// tiennent à 250 ppi passent devant, dans ce même ordre, et les autres
/// suivent : aucune n'est écartée, si rien ne tient la main choisit encore.
/// Douze au plus, comptées après ce rangement.
pub fn classer(
    album: &Album,
    planche: usize,
    cellule: Option<usize>,
    reserve: &[EnReserve],
    faits: &HashMap<String, Fait>,
    date_de: &dyn Fn(&str) -> Option<NaiveDateTime>,
) -> Vec<Candidate> {
    let mut rangees = ranger(album, planche, cellule, reserve, faits, date_de);
    rangees.truncate(AU_PLUS);
    rangees
}

/// Le rangement de `classer`, sans le plafond : le remplissage lit la liste
/// entière, pour qu'une case ne reste pas vide parce que les douze
/// premières avaient la mauvaise forme.
pub fn ranger(
    album: &Album,
    planche: usize,
    cellule: Option<usize>,
    reserve: &[EnReserve],
    faits: &HashMap<String, Fait>,
    date_de: &dyn Fn(&str) -> Option<NaiveDateTime>,
) -> Vec<Candidate> {
    let Some(spread) = album.spreads.get(planche) else {
        return Vec::new();
    };
    let jour = jour_de_reference(album, planche, date_de);
    // La case telle que l'émetteur la dessine : la scène, pas le gabarit.
    let case = cellule.and_then(|c| {
        crate::scene::Scene::of(spread, &crate::pdf::geometry(album))
            .objects
            .into_iter()
            .find(|o| matches!(o.role, crate::scene::Role::Photo { cell, .. } if cell == c))
            .map(|o| o.rect)
    });
    let posees: Vec<u64> = spread
        .slots
        .iter()
        .filter_map(|s| faits.get(&s.src).map(|f| f.dhash))
        .collect();

    let mut rangees: Vec<(Option<i64>, Candidate)> = reserve
        .iter()
        .filter_map(|r| {
            let fait = faits.get(&r.src)?;
            if posees.iter().any(|&d| analyze::hamming(d, fait.dhash) <= DUP_HAMMING) {
                return None;
            }
            let ecart = match (jour, fait.taken) {
                (Some(j), Some(t)) => Some((t.date() - j).num_days().abs()),
                _ => None,
            };
            let raison = match ecart {
                Some(0) => Raison::MemeJour,
                Some(_) => Raison::Proche,
                None => Raison::Nette,
            };
            Some((
                ecart,
                Candidate {
                    src: r.src.clone(),
                    score: fait.score,
                    taken: fait.taken,
                    raison,
                    focal: r.focal,
                    ppi: case.as_ref().zip(fait.orig).and_then(|(rect, (w, h))| {
                        (w > 0 && h > 0).then(|| crate::print::effective_ppi(rect, w, h, 1.0))
                    }),
                },
            ))
        })
        .collect();
    // Stable : deux photos au même jour et au même score gardent l'ordre de
    // la réserve, qui est celui du dossier.
    rangees.sort_by(|(ea, a), (eb, b)| {
        let ka = (ea.is_none(), ea.unwrap_or(0));
        let kb = (eb.is_none(), eb.unwrap_or(0));
        ka.cmp(&kb).then_with(|| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    // Stable aussi : qui tient et qui ne tient pas gardent chacun leur ordre.
    rangees.sort_by_key(|(_, c)| c.ppi.is_some_and(|p| p < crate::audit::MIN_EFFECTIVE_PPI));
    rangees.into_iter().map(|(_, c)| c).collect()
}

/// Le rectangle de la case `cellule` d'un gabarit, qu'une photo l'occupe ou
/// non : les cases d'un gabarit ne dépendent pas du nombre de photos posées,
/// la scène ne fait que s'arrêter à la dernière. `None` au-delà de la
/// capacité.
fn rect_de_case(template: &str, cellule: usize, g: &crate::pdf::SpreadGeometry) -> Option<crate::pdf::Rect> {
    if cellule >= crate::pdf::template_capacity(template) {
        return None;
    }
    crate::pdf::slots_for(template, cellule + 1, g).get(cellule).copied()
}

/// La réserve d'un dossier d'album pour une planche : ce que l'album ne
/// montre pas, mesuré, puis classé. Rend toujours une liste, vide au pire,
/// et dit dans `note` ce qui lui a manqué.
///
/// L'album voyage en argument et non par le fichier : l'éditeur tient une
/// version que le disque n'a pas encore (une photo retirée à l'instant est
/// déjà de la réserve), comme `gabarits_compatibles` reçoit ses `srcs`.
pub fn reserve_classee(
    dir: &Path,
    album: &Album,
    planche: usize,
    cellule: Option<usize>,
) -> Result<Reserve> {
    anyhow::ensure!(
        planche < album.spreads.len(),
        "planche {planche} hors de l'album ({} planches)",
        album.spreads.len()
    );
    let posees: Vec<String> = album.spreads[planche].slots.iter().map(|s| s.src.clone()).collect();
    let Some(Prepare { reserve, mesure, .. }) = preparer(dir, album, &posees)? else {
        return Ok(Reserve { candidats: Vec::new(), note: Some(SANS_MESURE.into()) });
    };
    let date_de = |src: &str| mesure.date(src);
    let candidats = classer(album, planche, cellule, &reserve, &mesure.faits, &date_de);
    let note = (!mesure.notes.is_empty()).then(|| mesure.notes.join(" ; "));
    Ok(Reserve { candidats, note })
}

const SANS_MESURE: &str = "ni relevé ni vignettes dans le dossier de l'album : la réserve \
                           est vide, recomposez l'album";

/// Ce que la réserve d'un album demande avant tout classement : les photos
/// qu'il ne montre pas, avec leur focal, mesurées avec `posees`, et la
/// curation qui dit pourquoi chacune est restée dehors. `None` quand le
/// dossier n'a ni relevé ni vignettes : rien ne se mesure.
struct Prepare {
    reserve: Vec<EnReserve>,
    mesure: Mesure,
    curation: Vec<Discard>,
}

fn preparer(dir: &Path, album: &Album, posees: &[String]) -> Result<Option<Prepare>> {
    let montrees: HashSet<&str> = album
        .spreads
        .iter()
        .flat_map(|s| s.slots.iter().map(|sl| sl.src.as_str()))
        .collect();
    let curation = lire_curation(dir)?;
    let focal_de: HashMap<&str, [f64; 2]> =
        curation.iter().map(|d| (d.src.as_str(), d.focal)).collect();

    let releve = crate::releve::Releve::dans_album(dir)?;
    let univers: Vec<String> = match &releve {
        Some(r) => r.photos.iter().map(|p| r.src(&p.path)).collect(),
        None => match lire_thumbs(dir)? {
            Some(thumbs) => {
                let mut srcs: Vec<String> = thumbs.into_keys().collect();
                srcs.sort();
                srcs
            }
            None => return Ok(None),
        },
    };
    let reserve: Vec<EnReserve> = univers
        .into_iter()
        .filter(|src| !montrees.contains(src.as_str()))
        .map(|src| {
            let focal = focal_de.get(src.as_str()).copied().unwrap_or_else(default_focal);
            EnReserve { src, focal }
        })
        .collect();

    let mut cibles: Vec<String> = reserve.iter().map(|r| r.src.clone()).collect();
    cibles.extend(posees.iter().cloned());
    let mesure = mesurer(dir, album, releve.as_ref(), &cibles);
    Ok(Some(Prepare { reserve, mesure, curation }))
}

/// Les raisons de curation que le remplissage reconduit, et elles seules :
/// celles où la photo est sortie faute de place, pas pour ce qu'elle est.
/// Un doublon, un jumeau, un refus, un parasite, une originale éditée, une
/// photo retirée à la main (qui n'a pas d'entrée) restent à la main : la
/// curation les a jugés avec plus que ce que la réserve sait mesurer.
pub const RECONDUITES: [&str; 2] = ["hors_budget", "meme_moment"];

/// Une case que le remplissage vise : la planche, et le rang de la case
/// dans son gabarit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Case {
    pub planche: usize,
    pub cellule: usize,
}

/// Une photo que le remplissage pose, et ce qu'elle imprime dans sa case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pose {
    pub planche: usize,
    pub cellule: usize,
    pub src: String,
    pub focal: [f64; 2],
    pub ppi: f64,
    /// Le gabarit que la planche prend, quand la pose le change : une
    /// planche vide devient le `solo` le plus proche de la forme de sa photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gabarit: Option<String>,
}

/// Ce que « Remplir les cases vides » rend : les poses, dans l'ordre où
/// l'app les applique, et les cases qu'aucune photo n'a tenues.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Remplissage {
    pub poses: Vec<Pose>,
    pub restees: Vec<Case>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Les cases vides d'un album, planche par planche, case par case. Deux
/// sortes et pas une de plus : une case du gabarit qui n'a pas de photo (un
/// `album.json` réparé à la main en porte), et la planche vide, qui compte
/// pour une case. Une page blanche dessinée par son gabarit (la gauche d'un
/// `solo`) n'est pas un trou ; une page de texte, une planche figée et une
/// planche vide qui porte un objet libre ne se touchent pas.
pub fn cases_vides(album: &Album) -> Vec<Case> {
    let mut cases = Vec::new();
    for (planche, s) in album.spreads.iter().enumerate() {
        if s.locked || s.text.as_deref().is_some_and(|t| !t.is_empty()) {
            continue;
        }
        if s.template == "vide" {
            if s.slots.is_empty() && s.objets.is_empty() {
                cases.push(Case { planche, cellule: 0 });
            }
            continue;
        }
        let Some(spec) = crate::gabarit::spec(&s.template) else { continue };
        cases.extend((s.slots.len()..spec.capacite).map(|cellule| Case { planche, cellule }));
    }
    cases
}

/// Le remplissage, pur et en mémoire. Chaque case reçoit la première photo,
/// dans l'ordre de `classer` (jour, puis score, sans les quasi-doublons de
/// la planche), qui la tient vraiment : 250 ppi au cadrage par défaut et
/// l'aspect à 1,4 au plus de celui de la case — les garanties du Composer,
/// qu'une photo posée par la machine doit tenir comme les siennes.
/// Prendre la première qui tient dans cet ordre, c'est prendre la tête du
/// rangement de 1.1-a, qui fait passer devant ce qui tient. Une taille
/// inconnue ne se pose pas. Une planche vide devient le `solo` le plus
/// proche de la forme de la photo, la règle du Composer pour une photo
/// seule (`layout::solo_pour_aspect`). Chaque pose entre dans l'album de
/// travail avant la case suivante. Une case ne se remplit qu'en ajoutant au
/// bout : la première qui ne trouve rien arrête sa planche.
pub fn remplir_avec(
    album: &Album,
    reserve: &[EnReserve],
    faits: &HashMap<String, Fait>,
    date_de: &dyn Fn(&str) -> Option<NaiveDateTime>,
) -> (Vec<Pose>, Vec<Case>) {
    let g = crate::pdf::geometry(album);
    let mut w = album.clone();
    let mut reste: Vec<EnReserve> = reserve.to_vec();
    let (mut poses, mut restees) = (Vec::new(), Vec::new());
    let mut arretee: Option<usize> = None;
    for case in cases_vides(album) {
        let Case { planche: p, cellule: c } = case;
        if arretee == Some(p) {
            restees.push(case);
            continue;
        }
        let vide = w.spreads[p].template == "vide";
        // Pour chaque candidate : le gabarit que la planche prendrait, et
        // ce que la photo imprime dans sa case.
        let tenue = |cand: &Candidate| -> Option<(String, f64)> {
            let fait = faits.get(&cand.src)?;
            let (iw, ih) = fait.orig.filter(|&(a, b)| a > 0 && b > 0)?;
            let aspect = iw as f64 / ih as f64;
            let gabarit = if vide {
                crate::layout::solo_pour_aspect(aspect).to_string()
            } else {
                w.spreads[p].template.clone()
            };
            let rect = rect_de_case(&gabarit, c, &g)?;
            let forme = rect.w / rect.h;
            let ppi = crate::print::effective_ppi(&rect, iw, ih, 1.0);
            (ppi >= crate::audit::MIN_EFFECTIVE_PPI
                && (aspect / forme).max(forme / aspect) <= crate::audit::ASPECT_BETRAYAL)
                .then_some((gabarit, ppi))
        };
        let choisie = ranger(&w, p, None, &reste, faits, date_de)
            .into_iter()
            .find_map(|cand| tenue(&cand).map(|t| (cand, t)));
        match choisie {
            Some((cand, (gabarit, ppi))) => {
                w.spreads[p].template = gabarit.clone();
                w.spreads[p].slots.push(Slot::new(cand.src.clone(), cand.focal));
                reste.retain(|r| r.src != cand.src);
                poses.push(Pose {
                    planche: p,
                    cellule: c,
                    src: cand.src,
                    focal: cand.focal,
                    ppi,
                    gabarit: vide.then_some(gabarit),
                });
            }
            None => {
                restees.push(case);
                arretee = Some(p);
            }
        }
    }
    (poses, restees)
}

/// La part de la réserve que la machine peut poser : ce que la curation a
/// sorti pour une raison de `RECONDUITES`.
pub fn reconduites(reserve: Vec<EnReserve>, curation: &[Discard]) -> Vec<EnReserve> {
    let permises: HashSet<&str> = curation
        .iter()
        .filter(|d| RECONDUITES.contains(&d.reason.as_str()))
        .map(|d| d.src.as_str())
        .collect();
    reserve.into_iter().filter(|r| permises.contains(r.src.as_str())).collect()
}

/// « Remplir les cases vides » sur un dossier d'album : la réserve mesurée
/// une fois, puis `remplir_avec`. L'album voyage en argument, comme pour
/// `reserve_classee`. Sans case vide, rien ne se mesure.
pub fn remplir(dir: &Path, album: &Album) -> Result<Remplissage> {
    let cases = cases_vides(album);
    if cases.is_empty() {
        return Ok(Remplissage::default());
    }
    let posees: Vec<String> =
        album.spreads.iter().flat_map(|s| s.slots.iter().map(|sl| sl.src.clone())).collect();
    let Some(Prepare { reserve, mesure, curation }) = preparer(dir, album, &posees)? else {
        return Ok(Remplissage { poses: Vec::new(), restees: cases, note: Some(SANS_MESURE.into()) });
    };
    let reserve = reconduites(reserve, &curation);
    let date_de = |src: &str| mesure.date(src);
    let (poses, restees) = remplir_avec(album, &reserve, &mesure.faits, &date_de);
    let note = (!mesure.notes.is_empty()).then(|| mesure.notes.join(" ; "));
    Ok(Remplissage { poses, restees, note })
}

/// D'où les dates des photos posées se lisent quand elles n'ont pas été
/// mesurées avec la réserve : du relevé, des originaux, ou de nulle part.
enum Racine {
    Releve(HashMap<String, NaiveDateTime>),
    Dossier(std::path::PathBuf),
    Absente,
}

struct Mesure {
    faits: HashMap<String, Fait>,
    notes: Vec<String>,
    racine: Racine,
}

impl Mesure {
    /// La prise de vue fiable d'une photo : mesurée avec la réserve, sinon
    /// relue du relevé ou de l'original.
    fn date(&self, src: &str) -> Option<NaiveDateTime> {
        if let Some(f) = self.faits.get(src) {
            return f.taken;
        }
        match &self.racine {
            Racine::Releve(dates) => dates.get(src).copied(),
            Racine::Dossier(root) => {
                let m = crate::meta::read(&root.join(src));
                m.taken_reliable.then_some(m.taken)
            }
            Racine::Absente => None,
        }
    }
}

/// Les faits des `srcs` demandées : du relevé quand l'album en porte un
/// (ses fiches sont les mesures mêmes des vignettes), des vignettes sinon,
/// la date venant alors de l'original quand le dossier de photos est là.
/// Une vignette illisible ou absente ne fait pas échouer : elle manque, et
/// la note le dit.
fn mesurer(dir: &Path, album: &Album, releve: Option<&crate::releve::Releve>, srcs: &[String]) -> Mesure {
    if let Some(r) = releve {
        let fiches: HashMap<String, &crate::pipeline::Photo> =
            r.photos.iter().map(|p| (r.src(&p.path), p)).collect();
        let faits = srcs
            .iter()
            .filter_map(|src| {
                let p = fiches.get(src)?;
                Some((
                    src.clone(),
                    Fait {
                        dhash: p.analysis.dhash,
                        score: p.analysis.score(),
                        taken: p.meta.taken_reliable.then_some(p.meta.taken),
                        orig: Some(p.orig),
                    },
                ))
            })
            .collect();
        let dates = fiches
            .iter()
            .filter_map(|(src, p)| p.meta.taken_reliable.then(|| (src.clone(), p.meta.taken)))
            .collect();
        return Mesure { faits, notes: Vec::new(), racine: Racine::Releve(dates) };
    }

    let root = std::path::PathBuf::from(&album.root);
    let root_ok = root.is_dir();
    let mut notes = Vec::new();
    let thumbs = match lire_thumbs(dir) {
        Ok(Some(t)) => t,
        Ok(None) => HashMap::new(),
        Err(e) => {
            notes.push(format!("{e:#}"));
            HashMap::new()
        }
    };
    let mesures: Vec<Option<(String, Fait)>> = srcs
        .par_iter()
        .map(|src| {
            let name = thumbs.get(src)?;
            let chemin = crate::thumb::chemin(dir, name)?;
            let (dhash, score) = analyse_de_vignette(&chemin)?;
            let original = root.join(src);
            let meta = root_ok.then(|| crate::meta::read(&original));
            let taken = meta.as_ref().and_then(|m| m.taken_reliable.then_some(m.taken));
            // La seule règle de taille du projet, celle de l'alerte (`qualite.rs`).
            let orig = meta.and_then(|m| crate::heic::oriented_dimensions(&original, m.orientation).ok());
            Some((src.clone(), Fait { dhash, score, taken, orig }))
        })
        .collect();
    let manquantes = mesures.iter().filter(|m| m.is_none()).count();
    let faits: HashMap<String, Fait> = mesures.into_iter().flatten().collect();
    if !root_ok {
        notes.push(format!(
            "dossier de photos introuvable ({}) : sans date, la réserve se classe \
             sur la netteté seule",
            root.display()
        ));
    }
    if manquantes > 0 {
        notes.push(format!(
            "{manquantes} photo(s) sans vignette lisible, hors réserve : recomposez \
             l'album pour les retrouver"
        ));
    }
    let racine = if root_ok { Racine::Dossier(root) } else { Racine::Absente };
    Mesure { faits, notes, racine }
}

/// Le dHash et le score d'une vignette, mesurés une fois par processus.
///
/// Mesuré le 02/10 sur corse-2013 (572 vignettes, 131 Mo) : rouvrir toute la
/// réserve coûte 3,7 s sur dix cœurs, et le popover qui la montre s'ouvre
/// au clic droit. La vignette d'une photo ne change qu'à la recomposition,
/// qui réécrit le fichier : la clé est son chemin, sa date et sa taille, et
/// une vignette réécrite se remesure d'elle-même. Rien n'est écrit sur le
/// disque : un cache de mesures à côté de l'album serait une seconde source
/// de vérité pour ce que le relevé sait déjà écrire.
fn analyse_de_vignette(chemin: &Path) -> Option<(u64, f64)> {
    static MESURES: std::sync::OnceLock<
        std::sync::RwLock<HashMap<std::path::PathBuf, (std::time::SystemTime, u64, u64, f64)>>,
    > = std::sync::OnceLock::new();
    let meta = std::fs::metadata(chemin).ok()?;
    let (modifie, taille) = (meta.modified().ok()?, meta.len());
    let cache = MESURES.get_or_init(Default::default);
    if let Some((m, t, dhash, score)) = cache.read().ok().and_then(|c| c.get(chemin).copied()) {
        if m == modifie && t == taille {
            return Some((dhash, score));
        }
    }
    let img = image::open(chemin).ok()?;
    let analysis = analyze::analyze(&img);
    let (dhash, score) = (analysis.dhash, analysis.score());
    if let Ok(mut c) = cache.write() {
        c.insert(chemin.to_path_buf(), (modifie, taille, dhash, score));
    }
    Some((dhash, score))
}

fn lire_curation(dir: &Path) -> Result<Vec<Discard>> {
    let path = dir.join("curation.json");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let texte = std::fs::read_to_string(&path)
        .with_context(|| format!("lecture de {}", path.display()))?;
    serde_json::from_str(&texte).context("curation.json illisible")
}

/// `thumbs.json`, ou `None` quand le dossier n'en a pas (album composé sans
/// ses photos). Un fichier présent et illisible est une erreur, pas un vide.
fn lire_thumbs(dir: &Path) -> Result<Option<HashMap<String, String>>> {
    let path = dir.join("thumbs.json");
    if !path.is_file() {
        return Ok(None);
    }
    let texte = std::fs::read_to_string(&path)
        .with_context(|| format!("lecture de {}", path.display()))?;
    serde_json::from_str(&texte).map(Some).context("thumbs.json illisible")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Size, Spread};

    fn date(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn planche(srcs: &[&str]) -> Spread {
        Spread {
            template: "duo".into(),
            slots: srcs.iter().map(|s| Slot::new(s.to_string(), [0.5, 0.42])).collect(),
            caption: None,
            text: None,
            edited: false,
            locked: false,
            objets: Vec::new(),
        }
    }

    fn album(planches: Vec<Spread>) -> Album {
        let mut a = Album::new("test", Path::new("/nulle-part"), Size { w: 210.0, h: 210.0 });
        a.spreads = planches;
        a
    }

    /// Des dHash tous à plus de 24 bits les uns des autres, sauf ceux qu'on
    /// rapproche exprès : chaque photo prend un motif de 8 bits répété.
    fn dhash(n: u64) -> u64 {
        let motif = [0x00, 0xff, 0x0f, 0xf0, 0x33, 0xcc, 0x55, 0xaa, 0x3c, 0xc3][n as usize % 10];
        (0..8).fold(0u64, |acc, i| acc | (motif << (8 * i)))
    }

    fn fait(n: u64, score: f64, taken: Option<&str>) -> Fait {
        Fait { dhash: dhash(n), score, taken: taken.map(date), orig: None }
    }

    /// Une photo dont on connaît la taille de l'original, orientée.
    fn fait_de(n: u64, score: f64, taken: &str, orig: (u32, u32)) -> Fait {
        Fait { orig: Some(orig), ..fait(n, score, Some(taken)) }
    }

    /// Une pleine page : la case qui demande le plus de pixels.
    fn pleine_page(srcs: &[&str]) -> Spread {
        Spread { template: "full1".into(), ..planche(srcs) }
    }

    fn reserve(srcs: &[&str]) -> Vec<EnReserve> {
        srcs.iter().map(|s| EnReserve { src: s.to_string(), focal: [0.5, 0.42] }).collect()
    }

    fn date_de(faits: &HashMap<String, Fait>) -> impl Fn(&str) -> Option<NaiveDateTime> + '_ {
        move |src| faits.get(src).and_then(|f| f.taken)
    }

    fn srcs(c: &[Candidate]) -> Vec<&str> {
        c.iter().map(|c| c.src.as_str()).collect()
    }

    #[test]
    fn le_meme_jour_gagne_sur_un_meilleur_score_d_un_autre_jour() {
        let a = album(vec![planche(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait(0, 5.0, Some("2013-10-27 10:00:00"))),
            ("loin.jpg".to_string(), fait(1, 9.0, Some("2013-10-29 10:00:00"))),
            ("jour.jpg".to_string(), fait(2, 2.0, Some("2013-10-27 18:00:00"))),
            ("veille.jpg".to_string(), fait(3, 8.0, Some("2013-10-26 18:00:00"))),
        ]
        .into_iter()
        .collect();
        let c = classer(&a, 0, None, &reserve(&["loin.jpg", "jour.jpg", "veille.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["jour.jpg", "veille.jpg", "loin.jpg"]);
        assert_eq!(c[0].raison, Raison::MemeJour);
        assert_eq!(c[1].raison, Raison::Proche);
        assert_eq!(c[2].raison, Raison::Proche);
    }

    #[test]
    fn un_quasi_doublon_d_une_photo_de_la_planche_est_ecarte() {
        let a = album(vec![planche(&["p.jpg"])]);
        let mut faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait(0, 5.0, Some("2013-10-27 10:00:00"))),
            ("autre.jpg".to_string(), fait(1, 5.0, Some("2013-10-27 10:00:00"))),
        ]
        .into_iter()
        .collect();
        // À 23 bits de la photo posée : doublon au seuil de la curation.
        let proche = Fait { dhash: dhash(0) ^ ((1u64 << 23) - 1), ..fait(0, 9.0, Some("2013-10-27 11:00:00")) };
        assert_eq!(analyze::hamming(proche.dhash, dhash(0)), 23);
        faits.insert("doublon.jpg".into(), proche);
        let c = classer(&a, 0, None, &reserve(&["doublon.jpg", "autre.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["autre.jpg"]);
    }

    #[test]
    fn a_egalite_de_jour_le_score_departage() {
        let a = album(vec![planche(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait(0, 5.0, Some("2013-10-27 10:00:00"))),
            ("faible.jpg".to_string(), fait(1, 1.0, Some("2013-10-27 12:00:00"))),
            ("forte.jpg".to_string(), fait(2, 7.0, Some("2013-10-27 13:00:00"))),
            ("moyenne.jpg".to_string(), fait(3, 4.0, Some("2013-10-27 14:00:00"))),
        ]
        .into_iter()
        .collect();
        let c = classer(&a, 0, None, &reserve(&["faible.jpg", "forte.jpg", "moyenne.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["forte.jpg", "moyenne.jpg", "faible.jpg"]);
        assert!(c.iter().all(|c| c.raison == Raison::MemeJour));
    }

    #[test]
    fn une_planche_sans_photo_prend_le_jour_de_ses_voisines() {
        // Une page de texte entre deux planches : la plus proche datée gagne,
        // et la voisine de gauche à distance 1 l'emporte sur celle à distance 2.
        let a = album(vec![planche(&["a.jpg"]), planche(&[]), planche(&["b.jpg"]), planche(&["c.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("a.jpg".to_string(), fait(0, 5.0, Some("2013-10-20 10:00:00"))),
            ("b.jpg".to_string(), fait(1, 5.0, None)),
            ("c.jpg".to_string(), fait(2, 5.0, Some("2013-10-29 10:00:00"))),
            ("r20.jpg".to_string(), fait(3, 1.0, Some("2013-10-20 15:00:00"))),
            ("r29.jpg".to_string(), fait(4, 9.0, Some("2013-10-29 15:00:00"))),
        ]
        .into_iter()
        .collect();
        assert_eq!(jour_de_reference(&a, 1, &date_de(&faits)), Some(date("2013-10-20 00:00:00").date()));
        let c = classer(&a, 1, None, &reserve(&["r29.jpg", "r20.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["r20.jpg", "r29.jpg"]);
        assert_eq!(c[0].raison, Raison::MemeJour);
    }

    #[test]
    fn la_mediane_des_dates_d_une_planche_fait_son_jour() {
        let a = album(vec![planche(&["a.jpg", "b.jpg", "c.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("a.jpg".to_string(), fait(0, 1.0, Some("2013-10-01 10:00:00"))),
            ("b.jpg".to_string(), fait(1, 1.0, Some("2013-10-27 10:00:00"))),
            ("c.jpg".to_string(), fait(2, 1.0, Some("2013-10-28 10:00:00"))),
        ]
        .into_iter()
        .collect();
        assert_eq!(jour_de_reference(&a, 0, &date_de(&faits)), Some(date("2013-10-27 00:00:00").date()));
    }

    #[test]
    fn sans_aucune_date_tout_est_nette_par_score() {
        let a = album(vec![planche(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait(0, 5.0, None)),
            ("x.jpg".to_string(), fait(1, 2.0, None)),
            ("y.jpg".to_string(), fait(2, 8.0, Some("2013-10-27 10:00:00"))),
        ]
        .into_iter()
        .collect();
        let c = classer(&a, 0, None, &reserve(&["x.jpg", "y.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["y.jpg", "x.jpg"]);
        assert!(c.iter().all(|c| c.raison == Raison::Nette));
    }

    #[test]
    fn les_photos_sans_date_ferment_la_marche() {
        let a = album(vec![planche(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait(0, 5.0, Some("2013-10-27 10:00:00"))),
            ("sans.jpg".to_string(), fait(1, 9.0, None)),
            ("loin.jpg".to_string(), fait(2, 1.0, Some("2013-12-01 10:00:00"))),
        ]
        .into_iter()
        .collect();
        let c = classer(&a, 0, None, &reserve(&["sans.jpg", "loin.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["loin.jpg", "sans.jpg"]);
        assert_eq!(c[1].raison, Raison::Nette);
    }

    #[test]
    fn douze_au_plus_et_une_photo_sans_fait_n_entre_pas() {
        let a = album(vec![planche(&["p.jpg"])]);
        let mut faits: HashMap<String, Fait> =
            [("p.jpg".to_string(), fait(0, 5.0, Some("2013-10-27 10:00:00")))].into_iter().collect();
        let mut noms: Vec<String> = Vec::new();
        for i in 1..=20u64 {
            let nom = format!("r{i:02}.jpg");
            // Des dHash éloignés les uns des autres et de la photo posée.
            faits.insert(nom.clone(), Fait { dhash: dhash(i % 9 + 1) ^ (i << 40), score: i as f64, taken: Some(date("2013-10-27 12:00:00")), orig: None });
            noms.push(nom);
        }
        noms.push("illisible.jpg".into());
        let en_reserve: Vec<EnReserve> = noms.iter().map(|s| EnReserve { src: s.clone(), focal: [0.5, 0.42] }).collect();
        let c = classer(&a, 0, None, &en_reserve, &faits, &date_de(&faits));
        assert_eq!(c.len(), AU_PLUS);
        assert_eq!(c[0].src, "r20.jpg");
        assert!(c.iter().all(|c| c.src != "illisible.jpg"));
    }

    #[test]
    fn avec_une_case_celle_qui_tient_passe_devant_une_plus_proche_qui_ne_tient_pas() {
        let a = album(vec![pleine_page(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait_de(0, 5.0, "2013-10-27 10:00:00", (6000, 4000))),
            ("petite.jpg".to_string(), fait_de(1, 9.0, "2013-10-27 12:00:00", (1200, 900))),
            ("grande.jpg".to_string(), fait_de(2, 1.0, "2013-10-29 12:00:00", (6000, 4000))),
        ]
        .into_iter()
        .collect();
        let r = reserve(&["petite.jpg", "grande.jpg"]);
        let c = classer(&a, 0, Some(0), &r, &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["grande.jpg", "petite.jpg"]);
        let (grande, petite) = (c[0].ppi.unwrap(), c[1].ppi.unwrap());
        assert!(grande >= crate::audit::MIN_EFFECTIVE_PPI, "{grande}");
        assert!(petite < crate::audit::MIN_EFFECTIVE_PPI, "{petite}");
        // La raison de date ne change pas : c'est l'app qui montre le signe.
        assert_eq!(c[1].raison, Raison::MemeJour);
    }

    #[test]
    fn sans_case_l_ordre_et_le_contrat_sont_ceux_d_hier() {
        let a = album(vec![pleine_page(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait_de(0, 5.0, "2013-10-27 10:00:00", (6000, 4000))),
            ("petite.jpg".to_string(), fait_de(1, 9.0, "2013-10-27 12:00:00", (1200, 900))),
            ("grande.jpg".to_string(), fait_de(2, 1.0, "2013-10-29 12:00:00", (6000, 4000))),
        ]
        .into_iter()
        .collect();
        let c = classer(&a, 0, None, &reserve(&["petite.jpg", "grande.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["petite.jpg", "grande.jpg"]);
        assert!(c.iter().all(|c| c.ppi.is_none()));
        // Sans case, le JSON n'a pas un champ de plus.
        assert!(!serde_json::to_string(&c).unwrap().contains("ppi"));
    }

    #[test]
    fn le_plafond_tombe_apres_le_rangement() {
        // Treize petites du jour devant une grande d'un autre jour : coupées
        // avant le rangement, les douze premières auraient chassé la seule
        // qui tient.
        let a = album(vec![pleine_page(&["p.jpg"])]);
        let mut faits: HashMap<String, Fait> =
            [("p.jpg".to_string(), fait_de(0, 5.0, "2013-10-27 10:00:00", (6000, 4000)))].into_iter().collect();
        let mut noms: Vec<String> = Vec::new();
        for i in 1..=13u64 {
            let nom = format!("petite{i:02}.jpg");
            faits.insert(nom.clone(), Fait { dhash: dhash(i % 9 + 1) ^ (i << 40), score: i as f64, taken: Some(date("2013-10-27 12:00:00")), orig: Some((1200, 900)) });
            noms.push(nom);
        }
        faits.insert("grande.jpg".into(), fait_de(5, 0.5, "2013-11-30 12:00:00", (6000, 4000)));
        noms.push("grande.jpg".into());
        let r: Vec<EnReserve> = noms.iter().map(|s| EnReserve { src: s.clone(), focal: [0.5, 0.42] }).collect();
        let c = classer(&a, 0, Some(0), &r, &faits, &date_de(&faits));
        assert_eq!(c.len(), AU_PLUS);
        assert_eq!(c[0].src, "grande.jpg");
        assert_eq!(c[1].src, "petite13.jpg", "les autres gardent l'ordre d'hier");
    }

    #[test]
    fn une_taille_inconnue_ne_se_devine_pas() {
        // Sans original mesuré, pas de ppi : la candidate garde sa place.
        let a = album(vec![pleine_page(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait(0, 5.0, Some("2013-10-27 10:00:00"))),
            ("inconnue.jpg".to_string(), fait(1, 9.0, Some("2013-10-27 12:00:00"))),
            ("grande.jpg".to_string(), fait_de(2, 1.0, "2013-10-27 12:00:00", (6000, 4000))),
        ]
        .into_iter()
        .collect();
        let c = classer(&a, 0, Some(0), &reserve(&["inconnue.jpg", "grande.jpg"]), &faits, &date_de(&faits));
        assert_eq!(srcs(&c), ["inconnue.jpg", "grande.jpg"]);
        assert_eq!(c[0].ppi, None);
    }

    #[test]
    fn le_focal_de_la_curation_voyage_avec_la_candidate() {
        let a = album(vec![planche(&["p.jpg"])]);
        let faits: HashMap<String, Fait> = [
            ("p.jpg".to_string(), fait(0, 5.0, None)),
            ("v.jpg".to_string(), fait(1, 5.0, None)),
        ]
        .into_iter()
        .collect();
        let r = vec![EnReserve { src: "v.jpg".into(), focal: [0.3, 0.2] }];
        let c = classer(&a, 0, None, &r, &faits, &date_de(&faits));
        assert_eq!(c[0].focal, [0.3, 0.2]);
    }

    fn vide() -> Spread {
        Spread { template: "vide".into(), ..planche(&[]) }
    }

    fn cases(c: &[(usize, usize)]) -> Vec<Case> {
        c.iter().map(|&(planche, cellule)| Case { planche, cellule }).collect()
    }

    #[test]
    fn les_cases_vides_sont_la_planche_vide_et_la_case_sans_photo() {
        let mut figee = vide();
        figee.locked = true;
        let mut texte = Spread { template: "texte".into(), ..planche(&[]) };
        texte.text = Some("Un mot".into());
        let mut ornee = vide();
        ornee.objets.push(serde_json::from_value(serde_json::json!({
            "x": 10.0, "y": 10.0, "w": 20.0, "h": 20.0,
            "type": "ornement", "pack": "colophon", "id": "fleuron-1"
        })).unwrap());
        // Un trio réparé à la main avec une seule photo : deux cases sans photo.
        let trou = Spread { template: "trio".into(), ..planche(&["t.jpg"]) };
        let a = album(vec![
            vide(),
            pleine_page(&["p.jpg"]),
            planche(&["a.jpg", "b.jpg"]),
            figee,
            texte,
            ornee,
            trou,
            Spread { template: "colophon".into(), ..planche(&[]) },
        ]);
        assert_eq!(cases_vides(&a), cases(&[(0, 0), (6, 1), (6, 2)]));
    }

    /// Des faits de photos de 6000 × 4000 : elles tiennent toute case.
    fn grandes(noms: &[(&str, u64, f64, &str)]) -> HashMap<String, Fait> {
        noms.iter()
            .map(|&(nom, n, score, jour)| (nom.to_string(), fait_de(n, score, jour, (6000, 4000))))
            .collect()
    }

    #[test]
    fn une_planche_vide_recoit_la_meilleure_du_jour_de_ses_voisines() {
        let a = album(vec![pleine_page(&["p.jpg"]), vide(), pleine_page(&["q.jpg"])]);
        let faits = grandes(&[
            ("p.jpg", 0, 5.0, "2013-10-27 10:00:00"),
            ("q.jpg", 1, 5.0, "2013-10-29 10:00:00"),
            ("loin.jpg", 2, 9.0, "2013-11-20 10:00:00"),
            ("jour.jpg", 3, 2.0, "2013-10-27 18:00:00"),
        ]);
        let (poses, restees) =
            remplir_avec(&a, &reserve(&["loin.jpg", "jour.jpg"]), &faits, &date_de(&faits));
        assert!(restees.is_empty());
        assert_eq!(poses.len(), 1);
        assert_eq!((poses[0].planche, poses[0].cellule, poses[0].src.as_str()), (1, 0, "jour.jpg"));
        assert!(poses[0].ppi >= crate::audit::MIN_EFFECTIVE_PPI, "{}", poses[0].ppi);
    }

    #[test]
    fn une_photo_n_est_posee_qu_une_fois() {
        let a = album(vec![pleine_page(&["p.jpg"]), vide(), vide()]);
        let faits = grandes(&[
            ("p.jpg", 0, 5.0, "2013-10-27 10:00:00"),
            ("x.jpg", 1, 9.0, "2013-10-27 11:00:00"),
            ("y.jpg", 2, 4.0, "2013-10-27 12:00:00"),
        ]);
        let (poses, restees) = remplir_avec(&a, &reserve(&["x.jpg", "y.jpg"]), &faits, &date_de(&faits));
        assert!(restees.is_empty());
        assert_eq!(poses.iter().map(|p| p.src.as_str()).collect::<Vec<_>>(), ["x.jpg", "y.jpg"]);
        assert_eq!(poses.iter().map(|p| p.planche).collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn une_petite_ou_une_inconnue_laissent_la_case_vide_et_le_disent() {
        let a = album(vec![pleine_page(&["p.jpg"]), vide()]);
        let mut faits = grandes(&[("p.jpg", 0, 5.0, "2013-10-27 10:00:00")]);
        faits.insert("petite.jpg".into(), fait_de(1, 9.0, "2013-10-27 11:00:00", (1200, 900)));
        faits.insert("inconnue.jpg".into(), fait(3, 9.0, Some("2013-10-27 11:00:00")));
        let r = reserve(&["petite.jpg", "inconnue.jpg"]);
        let (poses, restees) = remplir_avec(&a, &r, &faits, &date_de(&faits));
        assert!(poses.is_empty(), "{poses:?}");
        assert_eq!(restees, cases(&[(1, 0)]));
    }

    #[test]
    fn une_mal_formee_ne_prend_pas_la_case_d_un_gabarit() {
        // La case d'un quad est loin d'un panorama : il la trahit, la
        // photo droite d'un autre jour passe devant lui.
        let a = album(vec![Spread { template: "quad".into(), ..planche(&["p.jpg", "q.jpg", "r.jpg"]) }]);
        let mut faits = grandes(&[
            ("p.jpg", 0, 5.0, "2013-10-27 10:00:00"),
            ("q.jpg", 1, 5.0, "2013-10-27 10:00:00"),
            ("r.jpg", 2, 5.0, "2013-10-27 10:00:00"),
            ("droite.jpg", 3, 1.0, "2013-11-27 10:00:00"),
        ]);
        faits.insert("pano.jpg".into(), fait_de(4, 9.0, "2013-10-27 11:00:00", (12000, 3000)));
        let (poses, _) = remplir_avec(&a, &reserve(&["pano.jpg", "droite.jpg"]), &faits, &date_de(&faits));
        assert_eq!(poses.iter().map(|p| p.src.as_str()).collect::<Vec<_>>(), ["droite.jpg"]);
        assert_eq!(poses[0].gabarit, None, "une case d'un gabarit ne change pas le gabarit");
    }

    #[test]
    fn une_planche_vide_prend_le_solo_de_la_forme_de_sa_photo() {
        let a = album(vec![pleine_page(&["p.jpg"]), vide(), vide()]);
        let mut faits = grandes(&[
            ("p.jpg", 0, 5.0, "2013-10-27 10:00:00"),
            ("paysage.jpg", 1, 9.0, "2013-10-27 11:00:00"),
        ]);
        faits.insert("portrait.jpg".into(), fait_de(2, 8.0, "2013-10-27 12:00:00", (4000, 6000)));
        let (poses, _) = remplir_avec(&a, &reserve(&["paysage.jpg", "portrait.jpg"]), &faits, &date_de(&faits));
        let pris: Vec<_> = poses.iter().map(|p| (p.src.as_str(), p.gabarit.as_deref())).collect();
        assert_eq!(pris, [("paysage.jpg", Some("solo_paysage")), ("portrait.jpg", Some("solo"))]);
    }

    #[test]
    fn une_quasi_doublon_de_la_planche_n_est_pas_posee() {
        // Un quad réparé à la main : la case libre ne reçoit pas le quasi-
        // doublon d'une photo qu'il porte déjà.
        let a = album(vec![Spread { template: "quad".into(), ..planche(&["p.jpg", "q.jpg", "r.jpg"]) }]);
        let mut faits = grandes(&[
            ("p.jpg", 0, 5.0, "2013-10-27 10:00:00"),
            ("q.jpg", 1, 5.0, "2013-10-27 10:00:00"),
            ("r.jpg", 2, 5.0, "2013-10-27 10:00:00"),
            ("autre.jpg", 3, 1.0, "2013-10-27 12:00:00"),
        ]);
        let proche = Fait { dhash: dhash(0) ^ 0b111, ..fait_de(0, 9.0, "2013-10-27 11:00:00", (6000, 4000)) };
        faits.insert("doublon.jpg".into(), proche);
        let (poses, _) = remplir_avec(&a, &reserve(&["doublon.jpg", "autre.jpg"]), &faits, &date_de(&faits));
        assert_eq!(poses.iter().map(|p| p.src.as_str()).collect::<Vec<_>>(), ["autre.jpg"]);
    }

    #[test]
    fn les_cases_d_une_planche_se_remplissent_dans_l_ordre_et_la_premiere_ratee_l_arrete() {
        // Un quad réparé à la main avec une photo : trois cases, deux photos
        // qui tiennent. La troisième case reste vide, et le dit.
        let a = album(vec![Spread { template: "quad".into(), ..planche(&["p.jpg"]) }]);
        let faits = grandes(&[
            ("p.jpg", 0, 5.0, "2013-10-27 10:00:00"),
            ("x.jpg", 1, 9.0, "2013-10-27 11:00:00"),
            ("y.jpg", 2, 4.0, "2013-10-27 12:00:00"),
        ]);
        let (poses, restees) = remplir_avec(&a, &reserve(&["x.jpg", "y.jpg"]), &faits, &date_de(&faits));
        assert_eq!(poses.iter().map(|p| (p.cellule, p.src.as_str())).collect::<Vec<_>>(), [(1, "x.jpg"), (2, "y.jpg")]);
        assert_eq!(restees, cases(&[(0, 3)]));
    }

    #[test]
    fn la_douzieme_mal_formee_ne_cache_pas_la_treizieme() {
        // Treize panoramas du jour devant une photo d'un autre jour : coupée
        // à douze, la liste ne laisserait rien à poser.
        let a = album(vec![pleine_page(&["p.jpg"]), vide()]);
        let mut faits = grandes(&[("p.jpg", 0, 5.0, "2013-10-27 10:00:00")]);
        let mut noms: Vec<String> = Vec::new();
        for i in 1..=13u64 {
            let nom = format!("pano{i:02}.jpg");
            faits.insert(nom.clone(), Fait { dhash: dhash(i % 9 + 1) ^ (i << 40), score: i as f64, taken: Some(date("2013-10-27 12:00:00")), orig: Some((12000, 3000)) });
            noms.push(nom);
        }
        faits.insert("loin.jpg".into(), fait_de(5, 0.5, "2013-12-01 12:00:00", (4000, 6000)));
        noms.push("loin.jpg".into());
        let r: Vec<EnReserve> = noms.iter().map(|s| EnReserve { src: s.clone(), focal: [0.5, 0.42] }).collect();
        let (poses, _) = remplir_avec(&a, &r, &faits, &date_de(&faits));
        assert_eq!(poses.iter().map(|p| p.src.as_str()).collect::<Vec<_>>(), ["loin.jpg"]);
        assert_eq!(poses[0].gabarit.as_deref(), Some("solo"));
    }

    #[test]
    fn la_machine_ne_reconduit_que_ce_qui_est_sorti_faute_de_place() {
        let curation: Vec<Discard> = [
            ("budget.jpg", "hors_budget"),
            ("moment.jpg", "meme_moment"),
            ("non.jpg", "rejetee"),
            ("orig.jpg", "originale_editee"),
            ("dbl.jpg", "doublon"),
            ("jum.jpg", "jumeau"),
            ("para.jpg", "parasite"),
        ]
        .iter()
        .map(|&(src, reason)| Discard { src: src.into(), reason: reason.into(), kept: None, focal: [0.5, 0.42] })
        .collect();
        // `main.jpg` n'a pas d'entrée : retirée à la main, elle reste à la main.
        let r = reconduites(
            reserve(&["budget.jpg", "moment.jpg", "non.jpg", "orig.jpg", "dbl.jpg", "jum.jpg", "para.jpg", "main.jpg"]),
            &curation,
        );
        assert_eq!(r.iter().map(|r| r.src.as_str()).collect::<Vec<_>>(), ["budget.jpg", "moment.jpg"]);
    }

    #[test]
    fn sans_case_vide_rien_ne_se_mesure() {
        // Un dossier sans rien : sans case vide, aucune lecture et aucune note.
        let a = album(vec![pleine_page(&["p.jpg"])]);
        let r = remplir(Path::new("/nulle-part/colophon-remplir"), &a).unwrap();
        assert!(r.poses.is_empty() && r.restees.is_empty() && r.note.is_none());
    }

    #[test]
    fn un_dossier_sans_mesure_laisse_les_cases_vides_et_le_dit() {
        let dir = std::env::temp_dir().join(format!("colophon-remplir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = album(vec![pleine_page(&["p.jpg"]), vide()]);
        let r = remplir(&dir, &a).unwrap();
        assert!(r.poses.is_empty());
        assert_eq!(r.restees, cases(&[(1, 0)]));
        assert!(r.note.unwrap_or_default().contains("ni relevé ni vignettes"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Un dossier d'album réduit à son `album.json` : ni relevé, ni vignettes,
    /// ni dossier de photos. La commande rend une liste vide et le dit, jamais
    /// une panique ni une erreur — c'est un état atteignable à la main.
    #[test]
    fn un_dossier_sans_releve_ni_vignettes_rend_une_liste_vide_et_le_dit() {
        let dir = std::env::temp_dir().join(format!("colophon-reserve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = album(vec![planche(&["p.jpg"])]);
        let r = reserve_classee(&dir, &a, 0, None).unwrap();
        assert!(r.candidats.is_empty());
        assert!(r.note.as_deref().unwrap_or("").contains("ni relevé ni vignettes"));
        assert!(reserve_classee(&dir, &a, 7, None).is_err(), "une planche hors de l'album se refuse");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Des vignettes sans le dossier de photos : la réserve se classe sur la
    /// netteté et la note dit l'absence de date.
    #[test]
    fn des_vignettes_sans_originaux_classent_sur_la_nettete_et_le_disent() {
        let dir = std::env::temp_dir().join(format!("colophon-reserve-vignettes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = dir.join(".cache").join("thumbs");
        std::fs::create_dir_all(&cache).unwrap();
        // Trois vignettes synthétiques aux gradients opposés, donc à des
        // dHash éloignés (un dHash lit le gradient horizontal : un dégradé
        // horizontal et un vertical sont à 64 bits ; un motif « pseudo-
        // aléatoire » au pixel, lui, se moyenne en gris au rééchantillonnage
        // et tombe à zéro comme le vertical), et une entrée dont le fichier
        // manque.
        let mut thumbs = HashMap::new();
        let dessins: [(&str, Box<dyn Fn(u32, u32) -> u8>); 3] = [
            ("a.jpg", Box::new(|x, _| (x * 4) as u8)),
            ("b.jpg", Box::new(|_, y| (y * 4) as u8)),
            // Un dégradé horizontal en haut, son inverse en bas : 32 bits du
            // dHash de a (tout à un) comme de celui de b (tout à zéro).
            ("c.jpg", Box::new(|x, y| if y < 32 { (x * 4) as u8 } else { 255 - (x * 4) as u8 })),
        ];
        for (nom, f) in dessins {
            let img = image::ImageBuffer::from_fn(64, 64, |x, y| {
                let v = f(x, y);
                image::Rgb([v, 255 - v, v / 2])
            });
            let fichier = format!("v-{nom}");
            img.save(cache.join(&fichier)).unwrap();
            thumbs.insert(nom.to_string(), fichier);
        }
        thumbs.insert("perdue.jpg".to_string(), "v-perdue.jpg".to_string());
        std::fs::write(dir.join("thumbs.json"), serde_json::to_string(&thumbs).unwrap()).unwrap();
        let a = album(vec![planche(&["a.jpg"])]);
        let r = reserve_classee(&dir, &a, 0, None).unwrap();
        let note = r.note.clone().unwrap_or_default();
        assert!(note.contains("sans date"), "{note}");
        assert!(note.contains("1 photo(s) sans vignette lisible"), "{note}");
        assert!(r.candidats.iter().all(|c| c.raison == Raison::Nette));
        let noms = srcs(&r.candidats);
        assert_eq!(noms.len(), 2, "{noms:?}");
        assert!(noms.contains(&"b.jpg") && noms.contains(&"c.jpg"), "{noms:?}");
        assert!(r.candidats[0].score >= r.candidats[1].score);

        // La vignette de la photo posée est réécrite en copie de b : le cache
        // par processus la remesure sur sa date et sa taille, et b devient un
        // doublon de la planche. Un cache qui rendrait l'ancienne mesure
        // laisserait b dans la liste.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let b = image::ImageBuffer::from_fn(64, 64, |_, y| {
            let v = (y * 4) as u8;
            image::Rgb([v, 255 - v, v / 2])
        });
        b.save(cache.join("v-a.jpg")).unwrap();
        let r = reserve_classee(&dir, &a, 0, None).unwrap();
        assert_eq!(srcs(&r.candidats), ["c.jpg"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Le banc de l'œil : la réserve d'un vrai album, pour chaque planche
    /// demandée, et ce que coûte le second appel une fois les vignettes
    /// mesurées. `COLOPHON_ALBUM=.albums/corse-2013 cargo test -p
    /// colophon-core --release banc_reserve_du_mac -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn banc_reserve_du_mac() {
        let dir = std::path::PathBuf::from(
            std::env::var("COLOPHON_ALBUM").expect("COLOPHON_ALBUM=<dossier d'album>"),
        );
        let album: Album =
            serde_json::from_str(&std::fs::read_to_string(dir.join("album.json")).unwrap())
                .unwrap();
        for planche in [0, album.spreads.len() / 2, album.spreads.len() - 1] {
            let t0 = std::time::Instant::now();
            let r = reserve_classee(&dir, &album, planche, None).unwrap();
            let premier = t0.elapsed();
            let t1 = std::time::Instant::now();
            let _ = reserve_classee(&dir, &album, planche, None).unwrap();
            let second = t1.elapsed();
            eprintln!(
                "planche {} : {} candidates en {:.2} s, puis {:.3} s ; note = {:?}",
                planche + 1,
                r.candidats.len(),
                premier.as_secs_f64(),
                second.as_secs_f64(),
                r.note
            );
            for c in r.candidats.iter().take(4) {
                eprintln!("    {:?} {:.2} {:?} {}", c.raison, c.score, c.taken, c.src);
            }
        }
    }
}
