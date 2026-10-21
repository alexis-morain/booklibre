//! Preflight. The gate between a composed album and a print order.
//!
//! Every check is read against a [`PrinterProfile`], never against a constant:
//! the same album passes at one supplier and fails at the next, and that is
//! the point. A blocking defect stops the export: a print run costs real
//! money and a reprint costs it twice.
//!
//! Every message names **the spread and the cause**, in words, and says what
//! to do about it. A preflight that answers `ERR_RES_LOW` sends the user to a
//! forum instead of to the crop editor.

use crate::export::{Artefact, Manifeste, LIVRAISON_COUVERTURE, LIVRAISON_INTERIEUR};
use crate::model::Album;
use crate::printer::{Certitude, Dos, Espace, Fichiers, PdfX, PrinterProfile, GRAMMAGE_DEFAUT};
use crate::{cover, heic, imposition, meta, pdf, print, scene};
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// One thing wrong with the file, named the way a human would name it.
#[derive(Debug, Serialize)]
pub struct Defaut {
    /// Rule that fired, for grouping. The human reads `cause`, not this.
    pub regle: &'static str,
    /// A blocking defect stops the export. A warning is worth knowing.
    pub bloquant: bool,
    /// 1-based, as the ruler shows it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planche: Option<usize>,
    #[serde(rename = "case", skip_serializing_if = "Option::is_none")]
    pub case_idx: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    pub cause: String,
    /// The gesture that fixes it, in the editor.
    pub remede: String,
    /// La même chose en codes, pour qui écrit dans une autre langue.
    pub phrase: Phrase,
}

/// Ce que disent `cause` et `remede`, en codes et en valeurs.
///
/// Le français du moteur reste la phrase de référence, celle que lisent la
/// ligne de commande et les tests ; l'application en écrit l'anglais depuis
/// ces codes (`moteur.ts`), sans jamais relire une phrase française pour en
/// tirer un nombre. Les valeurs sont **déjà formatées**, à la précision de la
/// phrase française : un nombre arrondi deux fois, une fois par langue, finit
/// par ne pas dire la même chose dans les deux.
#[derive(Debug, Serialize)]
pub struct Phrase {
    pub cause: &'static str,
    pub remede: &'static str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub valeurs: BTreeMap<&'static str, String>,
}

/// Chaque code de phrase, et les valeurs qu'il lit. La table est la seule
/// liste : [`phrase`] refuse en test un code qu'elle ne porte pas ou une
/// valeur qu'elle n'annonce pas, et `codes::codes_json` la donne à l'app, dont
/// un test exige une phrase anglaise par code et aucun trou sans valeur.
pub const PHRASES: &[(&str, &[&str])] = &[
    ("format_sans_produit", &["imprimeur", "w_mm", "h_mm"]),
    ("format_sans_produit.choisir", &[]),
    ("pagination.multiple", &["pages", "imprimeur", "pas"]),
    ("pagination.bornes", &["pages", "imprimeur", "min", "max"]),
    ("pagination.decocher_colophon", &[]),
    ("pagination.decocher_garde", &[]),
    ("pagination.ajouter", &["planches_min"]),
    ("pagination.ajuster", &["planches_min", "planches_max"]),
    ("imposition.objet", &["imprimeur"]),
    ("imposition.objets", &["imprimeur", "objets"]),
    ("imposition.garde", &[]),
    ("fond_perdu", &["album_mm", "imprimeur", "requis_mm"]),
    ("fond_perdu.recomposer", &[]),
    ("espace", &["imprimeur", "intent"]),
    ("espace.choisir", &[]),
    ("conformite", &["imprimeur"]),
    ("conformite.choisir", &[]),
    ("resolution", &["src", "ppi", "imprimeur", "min_ppi"]),
    ("resolution.case", &[]),
    ("zone_sure", &["marge_mm", "imprimeur", "sure_mm"]),
    ("zone_sure.retirer", &[]),
    ("objet_coupe", &["n", "depasse_mm"]),
    ("objet_coupe.deplacer", &[]),
    ("objet_pli", &["n"]),
    ("objet_pli.glisser", &[]),
    ("couverture_resolution", &["src", "ppi", "w_mm", "h_mm", "imprimeur", "min_ppi"]),
    ("couverture_resolution.choisir", &[]),
    ("couverture_vide.titre", &[]),
    ("couverture_vide.dossier", &[]),
    ("couverture_vide.choisir", &[]),
    ("quatrieme_deborde", &["n"]),
    ("quatrieme_deborde.raccourcir", &[]),
    ("quatrieme_zone_sure", &["imprimeur", "sure_mm"]),
    ("quatrieme_zone_sure.reduire", &[]),
    ("quatrieme_resolution", &["src", "ppi", "imprimeur", "min_ppi"]),
    ("quatrieme_resolution.choisir", &[]),
    ("dos_nu", &["dos_mm", "min_mm"]),
    ("dos_nu.ajouter", &[]),
    ("dos", &["dos_mm", "imprimeur"]),
    ("dos.confirmer", &[]),
    ("fichier.illisible", &["fichier", "pourquoi"]),
    ("fichier.page", &["fichier", "n", "imprimeur", "attendues"]),
    ("fichier.pages", &["fichier", "n", "imprimeur", "attendues"]),
    ("fichier.taille", &["fichier", "page", "w_mm", "h_mm", "imprimeur", "attendu_w_mm", "attendu_h_mm"]),
    ("fichier_profil", &["fichier", "rendu", "profil", "imprimeur"]),
    ("fichier_perime.album", &["fichier", "avant", "maintenant"]),
    ("fichier_perime.octets", &["fichier", "octets", "rendu"]),
    ("fichier.refaire", &["commande", "profil"]),
];

/// Chaque règle du prévol, sous le nom que porte `Defaut::regle`. La liste
/// est fermée : `check` refuse en debug un défaut d'une règle qu'elle ne porte
/// pas, et `codes::codes_json` la donne à l'app, dont un test exige une
/// destination à l'écran par règle (`envoi.ts::DESTINATIONS`).
pub const REGLES: &[&str] = &[
    "conformite",
    "couverture_resolution",
    "couverture_vide",
    "dos",
    "dos_nu",
    "espace",
    "fichier_couverture",
    "fichier_interieur",
    "fichier_perime",
    "fichier_profil",
    "fond_perdu",
    "format_sans_produit",
    "imposition",
    "objet_coupe",
    "objet_pli",
    "pagination",
    "quatrieme_deborde",
    "quatrieme_resolution",
    "quatrieme_zone_sure",
    "resolution",
    "zone_sure",
];

/// Les règles que l'imprimeur ne refuse pas et que l'auteur tranche (1.1-c) :
/// une photo sous le plancher imprime, moins nette. Elles avertissent au lieu
/// de bloquer, et leurs pages entrent dans [`PrevolReport::acquittement`].
/// `codes::codes_json` donne la liste à l'app, qui les range à part.
pub const A_VOUS_DE_VOIR: &[&str] = &["couverture_resolution", "resolution"];

/// Lequel des deux fichiers de la livraison porte une page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QuelFichier {
    Interieur,
    Couverture,
}

/// Une page sous le plancher, telle que l'auteur l'acquitte : la page du
/// livre (planche k : gauche 2k − 2, droite 2k − 1 ; la couverture : 1) et le
/// ppi le plus bas qu'on y trouve. Des entiers, jamais un nom de photo : c'est
/// ce qui voyage au relais avec une commande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PageSousSeuil {
    pub fichier: QuelFichier,
    pub page: u32,
    pub ppi: u32,
}

/// Une entrée par page, le ppi le plus bas, arrondi vers le bas et tenu dans
/// `[1, plancher − 1]`, triées. La page 0 — la moitié gauche de la première
/// planche, que personne n'imprime — n'a rien à acquitter.
fn pages_sous_seuil(mesures: Vec<(QuelFichier, u32, f64)>, min_ppi: f64) -> Vec<PageSousSeuil> {
    let plafond = (min_ppi.ceil() as u32).saturating_sub(1).max(1);
    let mut par_page: BTreeMap<(QuelFichier, u32), u32> = BTreeMap::new();
    for (fichier, page, ppi) in mesures {
        if page == 0 {
            continue;
        }
        let ppi = (ppi.floor().max(0.0) as u32).clamp(1, plafond);
        par_page
            .entry((fichier, page))
            .and_modify(|p| *p = (*p).min(ppi))
            .or_insert(ppi);
    }
    par_page.into_iter().map(|((fichier, page), ppi)| PageSousSeuil { fichier, page, ppi }).collect()
}

/// Les deux notes du rapport. Des textes fixes : l'app les écrit en anglais
/// en les reconnaissant mot pour mot, et `codes::codes_json` les lui donne
/// pour qu'un mot changé ici rougisse là-bas.
pub const NOTE_PDF_X4: &str = "conformité PDF/X-4 déclarée : polices incorporées, OutputIntent \
     sRGB avec profil ICC, XMP et TrimBox vérifiés à chaque export. Le contrôle indépendant \
     disponible est PDF/A-2b (veraPDF), qui couvre le même socle ; le verdict PDF/X-4 lui-même \
     revient au prévol de l'imprimeur.";
pub const NOTE_PHOTOS_INTROUVABLES: &str =
    "dossier de photos introuvable : la résolution effective n'a pas pu être vérifiée";

/// Une phrase, ses valeurs vérifiées contre [`PHRASES`] en test : une valeur
/// oubliée laisserait un trou dans l'anglais, une valeur de trop n'irait nulle
/// part.
fn phrase(
    cause: &'static str,
    remede: &'static str,
    valeurs: Vec<(&'static str, String)>,
) -> Phrase {
    let valeurs: BTreeMap<&'static str, String> = valeurs.into_iter().collect();
    if cfg!(debug_assertions) {
        let mut attendues: Vec<&str> = [cause, remede]
            .iter()
            .flat_map(|code| {
                PHRASES
                    .iter()
                    .find(|(c, _)| c == code)
                    .unwrap_or_else(|| panic!("code de phrase inconnu : {code}"))
                    .1
                    .iter()
                    .copied()
            })
            .collect();
        attendues.sort_unstable();
        attendues.dedup();
        let portees: Vec<&str> = valeurs.keys().copied().collect();
        assert_eq!(portees, attendues, "valeurs de {cause} / {remede}");
    }
    Phrase { cause, remede, valeurs }
}

/// The sheet handed to whoever receives the PDF. Everything a printer asks
/// on the phone, written down once.
#[derive(Debug, Serialize)]
pub struct Fiche {
    pub imprimeur: &'static str,
    pub format_page_mm: [f64; 2],
    /// Le produit de l'imprimeur pour ce format, quand l'app sait le
    /// commander (Cloudprinter seul, `commande::produit_pour`). Absent sinon :
    /// un 28 × 21 se prépare, mais aucun produit ne l'imprime à ses cotes, et
    /// l'écran ne propose pas Commander.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produit: Option<&'static str>,
    pub planches: usize,
    pub pages_interieur: usize,
    /// Pages in the delivered PDF: the interior, plus the two cover leaves
    /// when the supplier binds a single file. This is the number that gets
    /// declared at the order, and Prodigi holds an order whose declared count
    /// disagrees with the file it received.
    pub pages_fichier: usize,
    pub fond_perdu_mm: crate::printer::Bleed,
    pub zone_sure_mm: f64,
    /// The flat cover sheet, in millimetres, when the supplier expects one
    /// from us. Absent when they bind a single file and build their own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feuille_couverture_mm: Option<[f64; 2]>,
    pub espace: Espace,
    pub output_intent: &'static str,
    pub conformite: PdfX,
    pub fichiers: Fichiers,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dos_mm: Option<f64>,
    pub grammage_g_m2: f64,
    pub resolution_cible_dpi: f64,
}

#[derive(Debug, Serialize)]
pub struct PrevolReport {
    pub album: String,
    pub profil: &'static str,
    /// False as soon as one blocking defect stands.
    pub ok: bool,
    pub bloquants: usize,
    pub avertissements: usize,
    pub fiche: Fiche,
    /// What the profile itself is still waiting on, copied through so a
    /// provisional number never travels silently.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reserves: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub defauts: Vec<Defaut>,
    /// Les pages sous le plancher, celles que l'auteur acquitte pour
    /// commander (1.1-c). Absent quand il n'y en a pas.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub acquittement: Vec<PageSousSeuil>,
}

/// Ce qu'un PDF posé à côté d'`album.json` a rendu quand on l'a ouvert.
///
/// Un fichier qu'on n'a pas écrit dans la seconde se lit sans jamais paniquer :
/// tronqué, chiffré ou dont le `MediaBox` n'est pas un tableau de nombres, il
/// rend [`Lecture::Illisible`] avec sa raison, et c'est la règle qui décide
/// quoi en faire. C'est la leçon de l'analyseur SVG de 6.3 s1, appliquée à
/// quelqu'un qui s'apprête à commander.
#[derive(Debug)]
pub enum Lecture {
    /// Les media box de ses pages, en millimètres, dans l'ordre du fichier.
    Pages(Vec<[f64; 2]>),
    /// Le fichier est là et ne se lit pas. La phrase dit pourquoi.
    Illisible(String),
}

/// Un fichier trouvé à côté d'`album.json` : ce qu'il porte, et ce qu'il pèse.
///
/// Les octets sont là pour le manifeste, qui les a notés au rendu : ils
/// attrapent un fichier remplacé depuis, pour le prix d'un `stat`, là où
/// rehacher 95 Mo à chaque prévol ne se justifierait pas.
#[derive(Debug)]
pub struct Pose {
    pub lecture: Lecture,
    pub octets: u64,
}

/// Ce que l'export a posé à côté de l'album : les deux PDF, et le manifeste
/// qui dit d'où ils sortent.
///
/// `None` veut dire absent, et **absent n'est pas un défaut** : la plupart des
/// albums n'ont jamais été exportés, et le prévol tourne dans l'app à tout
/// moment. La règle ne parle que de ce qu'elle trouve.
#[derive(Debug, Default)]
pub struct FichiersPoses {
    pub interieur: Option<Pose>,
    pub couverture: Option<Pose>,
    /// `export.json`, vide quand il n'y en a pas. Voir [`crate::export`] : une
    /// entrée absente ne dit rien du tout, elle ne rend rien suspect.
    pub manifeste: Manifeste,
}

impl FichiersPoses {
    /// Ouvre ce qui est là, à côté d'`album.json`. La lecture du disque vit
    /// ici et jamais dans [`check`], qui reste une fonction de valeurs.
    ///
    /// Les deux noms sont ceux de la **livraison**. Les aperçus posés dans le
    /// même dossier — `album.pdf` et `album-cover.apercu.pdf` — ne se lisent
    /// pas : un aperçu change de profil au gré de qui regarde, et le juger
    /// refuserait un album pour un fichier que personne n'envoie.
    pub fn lire(dir: &Path) -> Self {
        FichiersPoses {
            interieur: lire_pdf(&dir.join(LIVRAISON_INTERIEUR)),
            couverture: lire_pdf(&dir.join(LIVRAISON_COUVERTURE)),
            manifeste: Manifeste::lire(dir),
        }
    }
}

/// Le fichier s'il existe, sa raison s'il ne se lit pas, rien s'il est absent.
fn lire_pdf(path: &Path) -> Option<Pose> {
    if !path.is_file() {
        return None;
    }
    Some(Pose {
        lecture: match media_boxes_mm(path) {
            Ok(pages) => Lecture::Pages(pages),
            Err(pourquoi) => Lecture::Illisible(pourquoi),
        },
        octets: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
    })
}

/// Les media box du fichier, page par page, en millimètres.
fn media_boxes_mm(path: &Path) -> Result<Vec<[f64; 2]>, String> {
    let doc = lopdf::Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err("le fichier ne porte aucune page".into());
    }
    let mut v = Vec::with_capacity(pages.len());
    for (n, id) in pages {
        v.push(
            media_box_mm(&doc, id)
                .ok_or_else(|| format!("page {n} : le MediaBox ne se lit pas"))?,
        );
    }
    Ok(v)
}

/// La media box d'une page, largeur et hauteur en millimètres.
///
/// Le `MediaBox` s'hérite du nœud parent : une page qui ne le porte pas prend
/// celui de l'arbre au-dessus d'elle, et refuser là refuserait un fichier
/// parfaitement légitime. La remontée est bornée, un `/Parent` circulaire
/// n'étant pas un cas qu'on peut exclure d'un fichier qu'on n'a pas écrit.
fn media_box_mm(doc: &lopdf::Document, page: lopdf::ObjectId) -> Option<[f64; 2]> {
    const PT_MM: f64 = 25.4 / 72.0;
    let mut courant = page;
    for _ in 0..32 {
        let dict = doc.get_dictionary(courant).ok()?;
        if let Ok(o) = dict.get(b"MediaBox") {
            let a = doc.dereference(o).ok()?.1.as_array().ok()?;
            if a.len() != 4 {
                return None;
            }
            let n = |i: usize| -> Option<f64> {
                Some(f64::from(doc.dereference(&a[i]).ok()?.1.as_float().ok()?))
            };
            return Some([
                (n(2)? - n(0)?).abs() * PT_MM,
                (n(3)? - n(1)?).abs() * PT_MM,
            ]);
        }
        courant = dict.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

/// Run the preflight over a composed album folder.
pub fn prevol(dir: &Path, profil: &'static PrinterProfile) -> Result<PrevolReport> {
    prevol_dossier(dir, dir, profil)
}

/// Le prévol de l'album seul, sans regarder un seul fichier posé : celui de
/// la fenêtre.
///
/// Les PDF restés à côté d'`album.json` sont ceux d'un export d'avant, peut-être
/// d'un autre format ou d'un autre imprimeur, et personne ne les envoie depuis
/// l'app : elle prépare un dossier neuf et le relit (`prevol_dossier`). Les
/// juger ici refusait un album juste pour un fichier oublié, avec un conseil
/// de ligne de commande dans la fenêtre. La CLI garde [`prevol`] : dans un
/// terminal, ce dossier est celui qu'on envoie.
pub fn prevol_album(dir: &Path, profil: &'static PrinterProfile) -> Result<PrevolReport> {
    let (album, dims) = lire_album(dir)?;
    let face = face_de_la_quatrieme(dir, &album);
    let mesure = |s: &str, pt: f64| face.largeur_mm(s, pt);
    Ok(check_mesure(&album, profil, &dims, &FichiersPoses::default(), &mesure))
}

/// `album.json` et les tailles de ses originaux.
fn lire_album(dir: &Path) -> Result<(Album, HashMap<String, (u32, u32)>)> {
    let json = dir.join("album.json");
    let album: Album = serde_json::from_str(
        &fs::read_to_string(&json).with_context(|| format!("lecture de {}", json.display()))?,
    )
    .context("album.json illisible")?;
    let dims = original_dimensions(&album);
    Ok((album, dims))
}

/// Le même prévol, les fichiers lus dans `fichiers` plutôt qu'à côté
/// d'`album.json`.
///
/// C'est le dossier que « Préparer pour Cloudprinter » vient d'écrire hors du
/// dossier de l'album : les deux PDF, leur `export.json`, la fiche. Le relire
/// ici est ce qui fait mordre `fichier_interieur`, `fichier_couverture`,
/// `fichier_profil` et `fichier_perime` sur les fichiers réellement écrits,
/// et non sur ceux qu'un export d'avant aurait laissés à côté de l'album.
/// L'album, lui, se lit toujours dans `dir` : c'est contre lui qu'on juge.
pub fn prevol_dossier(
    dir: &Path,
    fichiers: &Path,
    profil: &'static PrinterProfile,
) -> Result<PrevolReport> {
    let (album, dims) = lire_album(dir)?;
    let face = face_de_la_quatrieme(dir, &album);
    let mesure = |s: &str, pt: f64| face.largeur_mm(s, pt);
    Ok(check_mesure(&album, profil, &dims, &FichiersPoses::lire(fichiers), &mesure))
}

/// La face où la quatrième s'imprime : la sienne si elle en a une et que le
/// dossier la porte, celle du livre sinon — le repli de l'émetteur, mot pour
/// mot.
fn face_de_la_quatrieme(
    dir: &Path,
    album: &Album,
) -> std::borrow::Cow<'static, crate::font::Embarquee> {
    if let Some(p) = album.cover.as_ref().and_then(|c| c.back_police.as_ref()) {
        let propre = crate::font::face_objet(dir, p);
        if propre.defaut.is_none() {
            return propre.face;
        }
    }
    crate::font::face_album(dir, album.police.as_ref().map(|p| p.fichier.as_str())).face
}

/// The page the machine writes that the album could do without, named as the
/// Envoi screen names it. Both are ordinary spreads costing two pages like
/// any other, which is exactly why the pagination rule has to know about
/// them: they are the two spreads a user can drop in a single click.
fn page_decochable(album: &Album) -> Option<&'static str> {
    let a = |t: &str| album.spreads.iter().any(|s| s.template == t);
    if a(crate::colophon::TEMPLATE) {
        Some("de colophon")
    } else if a(crate::garde::TEMPLATE) {
        Some("de garde")
    } else {
        None
    }
}

/// Original pixel sizes, EXIF orientation applied, keyed by slot source.
/// Absent entries mean the folder moved: resolution then goes unchecked and
/// the report says so rather than passing quietly.
fn original_dimensions(album: &Album) -> HashMap<String, (u32, u32)> {
    let root = PathBuf::from(&album.root);
    if !root.is_dir() {
        return HashMap::new();
    }
    let mut srcs: Vec<&str> = album
        .spreads
        .iter()
        .flat_map(|s| s.slots.iter().map(|sl| sl.src.as_str()))
        .collect();
    // La photo de couverture est un slot de plus, et pas un slot de planche :
    // elle n'apparaît nulle part dans `spreads`, donc sans cette ligne le
    // prévol de la couverture n'aurait jamais de dimensions à mesurer et se
    // tairait exactement là où il vient d'apprendre à parler.
    if let Some(slot) = album.cover.as_ref().and_then(|c| c.photo.as_ref()) {
        srcs.push(slot.src.as_str());
    }
    // Et la photo de quatrième (1.2-d), pour la même raison.
    if let Some(p) = album.cover.as_ref().and_then(|c| c.back_photo.as_ref()) {
        srcs.push(p.photo.src.as_str());
    }
    srcs.sort_unstable();
    srcs.dedup();
    srcs.par_iter()
        .filter_map(|src| {
            let p = root.join(src);
            let m = meta::read(&p);
            let taille = heic::oriente(heic::dimensions(&p).ok()?, m.orientation);
            Some(((*src).to_string(), taille))
        })
        .collect()
}

/// The whole preflight on values already in memory, so the tests need no
/// photo folder.
pub fn check(
    album: &Album,
    profil: &'static PrinterProfile,
    dims: &HashMap<String, (u32, u32)>,
    poses: &FichiersPoses,
) -> PrevolReport {
    check_mesure(album, profil, dims, poses, &crate::font::text_width_mm)
}

/// [`check`], la quatrième mesurée par `mesure` : dans la face où elle
/// s'imprime, que [`face_de_la_quatrieme`] lit sur le disque. La mesure entre
/// en valeur comme `dims`, et `check` reste une fonction de valeurs.
pub fn check_mesure(
    album: &Album,
    profil: &'static PrinterProfile,
    dims: &HashMap<String, (u32, u32)>,
    poses: &FichiersPoses,
    mesure: &dyn Fn(&str, f64) -> f64,
) -> PrevolReport {
    let mut defauts: Vec<Defaut> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let g = pdf::geometry(album);
    // Zero unless the supplier cuts the block page by page, and then every
    // rectangle below is the one the composition produced.
    let pli = imposition::pli_mm(profil);
    let pages = album.spreads.len() * 2;
    // What the supplier counts is the file, not the book block. A supplier
    // who binds one file finds the cover in it, two pages the interior does
    // not have, and their bounds are written against that total. Two is also
    // the number that has to be declared at the order, and Prodigi puts an
    // order on hold when it disagrees with the file.
    let pages_fichier = pages + if profil.fichiers == Fichiers::Un { 2 } else { 0 };

    // 0. Le format. Chez qui l'app commande, un format sans produit ne se
    // relie nulle part : le 13/10 un 28 × 21 se préparait vert pour
    // Cloudprinter, puis le relais refusait la commande. Rien à recadrer, rien
    // à retirer : le seul geste est de changer de format, et il se dit avant
    // tout le reste. Ailleurs on ne sait pas quels produits existent, donc on
    // se tait plutôt que d'inventer un refus.
    let produit = profil
        .commande
        .and_then(|_| crate::commande::produit_pour(&crate::format::nom(album.trim_mm)));
    if profil.commande.is_some() && produit.is_none() {
        let (w, h) = (format!("{:.0}", album.trim_mm.w), format!("{:.0}", album.trim_mm.h));
        defauts.push(Defaut {
            regle: "format_sans_produit",
            bloquant: true,
            planche: None,
            case_idx: None,
            src: None,
            cause: format!(
                "{} n'imprime pas ce format : aucun de ses livres ne mesure {w} × {h} mm",
                profil.nom
            ),
            remede: "choisissez un des formats qu'il imprime dans Format".into(),
            phrase: phrase(
                "format_sans_produit",
                "format_sans_produit.choisir",
                vec![("imprimeur", profil.nom.into()), ("w_mm", w), ("h_mm", h)],
            ),
        });
    }

    // 1. Pagination. A binding folds sheets: an odd count does not exist, and
    // every press has a range it will not leave.
    if !profil.pagination_ok(pages_fichier) {
        let multiple = pages_fichier % profil.pas_pagination != 0;
        let decochable =
            page_decochable(album).filter(|_| profil.pagination_ok(pages_fichier - 2));
        let mut valeurs = vec![("pages", pages_fichier.to_string()), ("imprimeur", profil.nom.into())];
        let code_cause = if multiple {
            valeurs.push(("pas", profil.pas_pagination.to_string()));
            "pagination.multiple"
        } else {
            valeurs.push(("min", profil.pages_min.to_string()));
            valeurs.push(("max", profil.pages_max.to_string()));
            "pagination.bornes"
        };
        let code_remede = match decochable {
            Some("de colophon") => "pagination.decocher_colophon",
            Some(_) => "pagination.decocher_garde",
            None if pages_fichier < profil.pages_min => {
                valeurs.push(("planches_min", profil.pages_min.div_ceil(2).to_string()));
                "pagination.ajouter"
            }
            None => {
                valeurs.push(("planches_min", (profil.pages_min / 2).to_string()));
                valeurs.push(("planches_max", (profil.pages_max / 2).to_string()));
                "pagination.ajuster"
            }
        };
        let cause = if multiple {
            format!(
                "le fichier fait {pages_fichier} pages, or {} ne relie que des multiples de {}",
                profil.nom, profil.pas_pagination
            )
        } else {
            format!(
                "le fichier fait {pages_fichier} pages, hors des bornes de {} ({} à {})",
                profil.nom, profil.pages_min, profil.pages_max
            )
        };
        defauts.push(Defaut {
            regle: "pagination",
            bloquant: true,
            planche: None,
            case_idx: None,
            src: None,
            cause,
            // The colophon page is two of those pages, and an album sitting
            // exactly on the upper bound is pushed over by it. Say so where
            // it is read, rather than letting somebody hunt for two pages
            // through a book of a hundred spreads.
            remede: if let Some(page) = decochable {
                format!("décochez la page {page} dans l'écran Envoi : elle vaut deux pages, et sans elle le compte tombe juste")
            } else if pages_fichier < profil.pages_min {
                // Sous le plancher, retirer n'a aucun sens. Un rythme plus
                // aéré pose moins de photos par planche, donc allonge le livre
                // sans en ajouter une seule.
                format!(
                    "ajoutez des planches ou choisissez un rythme plus aéré : une planche vaut deux pages, il en faut au moins {}",
                    profil.pages_min.div_ceil(2)
                )
            } else {
                format!(
                    "ajoutez ou retirez des planches : une planche vaut deux pages, il en faut entre {} et {}",
                    profil.pages_min / 2,
                    profil.pages_max / 2
                )
            },
            phrase: phrase(code_cause, code_remede, valeurs),
        });
    }

    // 1 bis. The imposition. A supplier who reads one PDF page as one page of
    // the book gets the interior cut in two, and the cut has a price: the left
    // half of the first spread is the page that faces the inside of the cover,
    // and it is not printed. That is what puts the half-title on page one,
    // where a book puts it. It only holds if that half is empty — which it is
    // of a half-title, and is not of a spread of photographs — so a spread
    // carrying something there is refused rather than quietly truncated.
    if profil.pages_simples {
        if let Some(premiere) = album.spreads.first() {
            let objets = imposition::objets_a_gauche(&scene::Scene::of(premiere, &g), &g);
            if objets > 0 {
                defauts.push(Defaut {
                    regle: "imposition",
                    bloquant: true,
                    planche: Some(1),
                    case_idx: None,
                    src: None,
                    cause: format!(
                        "{} relie une page de PDF par page de livre, et la page de gauche de la \
                         première planche ne s'imprime pas : elle porte {objets} objet{} qui \
                         seraient perdus",
                        profil.nom,
                        if objets > 1 { "s" } else { "" }
                    ),
                    remede: "déplacez ce qui est sur la page de gauche, ou ajoutez la page de garde \
                             dans l'écran Envoi : elle se pose en tête et sa page de gauche reste \
                             blanche"
                        .into(),
                    phrase: if objets > 1 {
                        phrase(
                            "imposition.objets",
                            "imposition.garde",
                            vec![("imprimeur", profil.nom.into()), ("objets", objets.to_string())],
                        )
                    } else {
                        phrase(
                            "imposition.objet",
                            "imposition.garde",
                            vec![("imprimeur", profil.nom.into())],
                        )
                    },
                });
            }
        }
    }

    // 2. Bleed. The album carries one value; the profile wants a value per
    // edge. Rendering less than the printer trims puts white on the cut.
    let requis = profil.bleed_mm.max();
    if album.bleed_mm + 1e-9 < requis {
        defauts.push(Defaut {
            regle: "fond_perdu",
            bloquant: true,
            planche: None,
            case_idx: None,
            src: None,
            cause: format!(
                "l'album est composé avec {:.1} mm de fond perdu, {} en demande {:.1}",
                album.bleed_mm, profil.nom, requis
            ),
            remede: "recomposez l'album au fond perdu du profil : le fond perdu est fixé à la composition".into(),
            phrase: phrase(
                "fond_perdu",
                "fond_perdu.recomposer",
                vec![
                    ("album_mm", format!("{:.1}", album.bleed_mm)),
                    ("imprimeur", profil.nom.into()),
                    ("requis_mm", format!("{requis:.1}")),
                ],
            ),
        });
    }

    // 3. Colour space. We render RGB and never convert: a CMYK conversion done
    // blind is worse than no conversion at all, and lcms2 is not in yet.
    if profil.espace == Espace::Fogra39 {
        defauts.push(Defaut {
            regle: "espace",
            bloquant: true,
            planche: None,
            case_idx: None,
            src: None,
            cause: format!(
                "{} imprime en CMJN {} et Booklibre n'exporte qu'en RVB",
                profil.nom,
                profil.espace.output_intent()
            ),
            remede: "choisissez un profil qui accepte le RVB, ou demandez à l'imprimeur de convertir".into(),
            phrase: phrase(
                "espace",
                "espace.choisir",
                vec![
                    ("imprimeur", profil.nom.into()),
                    ("intent", profil.espace.output_intent().into()),
                ],
            ),
        });
    }

    // 4. PDF/X conformance. Read from the renderer rather than restated here,
    // so nobody has to remember this line the day the declaration changes.
    if profil.pdf_x == PdfX::X4 && !pdf::EMITS_PDF_X {
        defauts.push(Defaut {
            regle: "conformite",
            bloquant: true,
            planche: None,
            case_idx: None,
            src: None,
            cause: format!(
                "{} demande du PDF/X-4 et le fichier ne se déclare pas comme tel : les polices \
                 sont incorporées, mais l'OutputIntent et les métadonnées XMP manquent encore",
                profil.nom
            ),
            remede: "choisissez un profil sans conformité PDF/X, ou demandez à l'imprimeur s'il accepte un PDF simple en RVB".into(),
            phrase: phrase("conformite", "conformite.choisir", vec![("imprimeur", profil.nom.into())]),
        });
    } else if profil.pdf_x == PdfX::X4 {
        // The declaration is there and measured, but no free validator
        // certifies PDF/X-4: veraPDF ships PDF/A profiles only. Saying which
        // is which here costs one line and stops the spec sheet from
        // promising a verdict nobody delivered.
        notes.push(NOTE_PDF_X4.into());
    }

    // 5. Resolution, cell by cell. The one defect nobody sees on screen and
    // everybody sees on paper.
    //
    // Since 1.1-c this warns: the printer prints it, less sharp, and the
    // author decides. The pages go into the acquittal list.
    if dims.is_empty() && album.spreads.iter().any(|s| !s.slots.is_empty()) {
        notes.push(NOTE_PHOTOS_INTROUVABLES.into());
    }
    let mut sous_seuil: Vec<(QuelFichier, u32, f64)> = Vec::new();
    for (si, spread) in album.spreads.iter().enumerate() {
        let scene = scene::Scene::of(spread, &g);
        for (ci, object) in scene.objects.iter().enumerate() {
            let scene::Role::Photo { src, zoom, .. } = &object.role else { continue };
            let Some(&(ow, oh)) = dims.get(src) else { continue };
            // The rectangle **this supplier** receives. A photograph that runs
            // to the fold gains the fold bleed on the way out, so it is
            // cover-cropped a little wider for the same pixels: measuring the
            // composed rectangle would promise 250 ppi where the press gets
            // less. Identical to `object.rect` for everyone who binds spreads.
            let rect = &imposition::rect_exporte(object, &g, pli);
            let slot = &spread.slots[ci];
            let scale = print::print_scale(rect, ow, oh) * zoom.max(1.0);
            let ppi = print::PRINT_DPI / scale;
            if ppi < profil.min_ppi {
                let page = 2 * si + usize::from(imposition::cote_de(object, &g) == imposition::Cote::Droite);
                sous_seuil.push((QuelFichier::Interieur, u32::try_from(page).unwrap_or(u32::MAX), ppi));
                defauts.push(Defaut {
                    regle: "resolution",
                    bloquant: false,
                    planche: Some(si + 1),
                    case_idx: Some(ci),
                    src: Some(slot.src.clone()),
                    cause: format!(
                        "{} imprimerait à {ppi:.0} ppi dans cette case, {} exige {:.0}",
                        slot.src, profil.nom, profil.min_ppi
                    ),
                    remede: "mettez-la dans une case plus petite, remplacez-la, ou retirez-la".into(),
                    phrase: phrase(
                        "resolution",
                        "resolution.case",
                        vec![
                            ("src", slot.src.clone()),
                            ("ppi", format!("{ppi:.0}")),
                            ("imprimeur", profil.nom.into()),
                            ("min_ppi", format!("{:.0}", profil.min_ppi)),
                        ],
                    ),
                });
            }
        }
    }

    // 6. Safe zone. Photos bleed on purpose; text must not — and this warns
    // rather than refuses, which is the doctrine already written for the free
    // objects in 6.4: the fold stops, the margin warns. What the guillotine
    // goes through comes back mutilated and is refused; a caption a supplier
    // would rather see further in is a preference, and preferences differ by
    // a factor of two between the four profiles here.
    //
    // It had to change, and the measurement is what changed it. The caption's
    // baseline is `bleed + margin × CAPTION_SAFE`, half the margin of the
    // format: 7,00 mm on five formats and 6,75 on `portrait-20x25`. Blocking,
    // it made every captioned album fail at Prodigi and at Lulu — 8 spreads of
    // `corse-2013`, on `main`, for a rule nobody could satisfy — and the only
    // thing keeping Cloudprinter quiet was a `safe_mm` of 5 that nobody had
    // read off their specification. A threshold set to silence a rule measures
    // nothing. Soft, the three real numbers can finally be carried: 7, 10 and
    // 12,7, each the supplier's own.
    //
    // The real fix is upstream and is not this session's: a caption at
    // `max(0,5 × marge, plancher)` would move every caption of every album,
    // which is its own wave.
    for (si, spread) in album.spreads.iter().enumerate() {
        // Every caption is measured at its own baseline, read off the scene:
        // the chapter line at its anchor, each photo caption — typed, or the
        // date and place the machine sets under it (1.2-b) — under its own
        // cell. Until 1.2-b a spread that carried photo captions only was
        // measured at the chapter anchor it did not print. What is measured
        // stays the baseline, not the ink around it, and one warning per
        // spread names the closest.
        let marge = scene::Scene::of(spread, &g)
            .objects
            .iter()
            .filter_map(|o| match &o.role {
                scene::Role::PhotoCaption { at, .. } | scene::Role::ChapterCaption { at, .. } => {
                    Some(distance_au_rognage(at.x, at.y, &g, album.bleed_mm))
                }
                _ => None,
            })
            .fold(f64::INFINITY, f64::min);
        if !marge.is_finite() {
            continue;
        }
        if marge + 1e-9 < profil.safe_mm {
            defauts.push(Defaut {
                regle: "zone_sure",
                bloquant: false,
                planche: Some(si + 1),
                case_idx: None,
                src: None,
                cause: format!(
                    "la légende passe à {marge:.1} mm du rognage, {} garde {:.1} mm libres",
                    profil.nom, profil.safe_mm
                ),
                // L'ancien remède — « raccourcissez la légende ou changez le
                // gabarit » — était faux : ni l'un ni l'autre ne déplace une
                // ligne de base calculée depuis la marge du format.
                remede: "retirez la légende de cette planche si le risque ne vous \
                         convient pas : sa ligne de base vient de la marge du format, ni la \
                         raccourcir ni changer de gabarit ne la déplace"
                    .into(),
                phrase: phrase(
                    "zone_sure",
                    "zone_sure.retirer",
                    vec![
                        ("marge_mm", format!("{marge:.1}")),
                        ("imprimeur", profil.nom.into()),
                        ("sure_mm", format!("{:.1}", profil.safe_mm)),
                    ],
                ),
            });
        }
    }

    // 6 bis. Free objects. Two refusals and two only: the cut, and the fold.
    // The margin stays soft — a block laid to the bleed on purpose is a
    // choice, the editor said so at the gesture and the linter counts it —
    // but a block the guillotine goes through comes back mutilated, and one
    // across the fold comes back split by the binding. Neither is measured
    // here: `distance_to_trim` and `traverse_le_pli` are the doctrine, and
    // this reads them with the object's own angle.
    //
    // **Whatever fills the object.** A fleuron the cut goes through is a
    // mutilated fleuron; the box is what is measured, and an ornament's box
    // is its ink.
    for (si, spread) in album.spreads.iter().enumerate() {
        for objet in scene::Scene::of(spread, &g).objects.iter() {
            let Some(index) = objet.role.index_libre() else { continue };
            let marge = scene::distance_to_trim(&objet.rect, objet.angle, &g);
            if marge < 0.0 {
                defauts.push(Defaut {
                    regle: "objet_coupe",
                    bloquant: true,
                    planche: Some(si + 1),
                    case_idx: None,
                    src: None,
                    cause: format!(
                        "l'objet libre n° {} dépasse de {:.1} mm au-delà du rognage : \
                         la coupe passe dedans",
                        index + 1,
                        -marge
                    ),
                    remede: "déplacez l'objet vers l'intérieur de la page, ou réduisez-le".into(),
                    phrase: phrase(
                        "objet_coupe",
                        "objet_coupe.deplacer",
                        vec![("n", (index + 1).to_string()), ("depasse_mm", format!("{:.1}", -marge))],
                    ),
                });
            }
            if scene::traverse_le_pli(&objet.rect, objet.angle, &g) {
                defauts.push(Defaut {
                    regle: "objet_pli",
                    bloquant: true,
                    planche: Some(si + 1),
                    case_idx: None,
                    src: None,
                    cause: format!(
                        "l'objet libre n° {} est à cheval sur le pli : la reliure le \
                         coupera en deux",
                        index + 1
                    ),
                    remede: "glissez l'objet d'un seul côté du pli : l'éditeur y bute tout seul".into(),
                    phrase: phrase("objet_pli", "objet_pli.glisser", vec![("n", (index + 1).to_string())]),
                });
            }
        }
    }

    // 7. La couverture, mesurée sur la feuille que ce fournisseur reçoit.
    //
    // Les trois cotes du cartonné — rempli, débord, mors — n'agrandissent pas
    // le livre, elles agrandissent le carton autour de lui : chez Cloudprinter
    // la première passe de 213 × 216 à 239 × 258 mm. La photo qu'on y pose
    // remplit donc une surface un huitième plus grande avec les mêmes pixels,
    // et son ppi effectif tombe d'autant. Mesuré en session A sur les 98
    // photographies de corse-2013 : 2 étaient sous le plancher en couverture
    // souple, 36 le sont en cartonné. C'est la page que tout le monde regarde,
    // et le prévol n'en disait rien du tout.
    //
    // Le rectangle vient de `cover`, jamais reconstruit ici : une feuille à
    // plat et un feuillet volant ne sont pas la même surface, et le prévol qui
    // choisirait à leur place mesurerait une couverture que personne n'imprime.
    let cg = cover::geometry(album, profil);
    if let Some(slot) = album.cover.as_ref().and_then(|c| c.photo.as_ref()) {
        if let Some(&(ow, oh)) = dims.get(slot.src.as_str()) {
            let rect = cover::photo_rect_du_profil(album, profil);
            let scale = print::print_scale(&rect, ow, oh) * slot.zoom.max(1.0);
            let ppi = print::PRINT_DPI / scale;
            if ppi < profil.min_ppi {
                sous_seuil.push((QuelFichier::Couverture, 1, ppi));
                defauts.push(Defaut {
                    regle: "couverture_resolution",
                    bloquant: false,
                    planche: None,
                    case_idx: None,
                    src: Some(slot.src.clone()),
                    cause: format!(
                        "{} imprimerait à {ppi:.0} ppi sur la couverture de {:.0} × {:.0} mm, \
                         {} exige {:.0}",
                        slot.src, rect.w, rect.h, profil.nom, profil.min_ppi
                    ),
                    remede: "choisissez une photo plus définie pour la couverture : la \
                             feuille est plus grande que la page, elle en demande davantage"
                        .into(),
                    phrase: phrase(
                        "couverture_resolution",
                        "couverture_resolution.choisir",
                        vec![
                            ("src", slot.src.clone()),
                            ("ppi", format!("{ppi:.0}")),
                            ("w_mm", format!("{:.0}", rect.w)),
                            ("h_mm", format!("{:.0}", rect.h)),
                            ("imprimeur", profil.nom.into()),
                            ("min_ppi", format!("{:.0}", profil.min_ppi)),
                        ],
                    ),
                });
            }
        }
    }

    // La couverture vide. Le 21/09 la feuille partie chez Cloudprinter était
    // blanche : `album.json` n'avait pas de clé `cover`, le rendu est retombé
    // sur le nom du dossier sans photo, 0,12 % d'encre, aux cotes exactes et
    // au prévol vert. C'est l'imprimeur qui l'a vu. La règle lit l'album, pas
    // le raster : une couverture sans photo sort blanche, qu'elle ait un
    // titre ou non, et chez qui relie deux fichiers c'est un fichier qu'on
    // envoie. Ailleurs la couverture voyage dans l'intérieur, la même
    // frontière que `fichier_couverture`.
    if profil.fichiers == Fichiers::Deux {
        let photo = album.cover.as_ref().and_then(|c| c.photo.as_ref());
        if photo.is_none() {
            let titree = album.cover.as_ref().is_some_and(|c| !c.title.trim().is_empty());
            defauts.push(Defaut {
                regle: "couverture_vide",
                bloquant: true,
                planche: None,
                case_idx: None,
                src: None,
                cause: if titree {
                    "la couverture n'a pas de photo, elle sortirait blanche avec son seul titre"
                        .into()
                } else {
                    "la couverture n'a pas de photo, elle sortirait blanche avec le nom du dossier"
                        .into()
                },
                remede: "choisissez une photo dans Couverture".into(),
                phrase: phrase(
                    if titree { "couverture_vide.titre" } else { "couverture_vide.dossier" },
                    "couverture_vide.choisir",
                    vec![],
                ),
            });
        }
    }

    // Et le dos, qui est une surface ou n'en est pas une. Sous le plancher de
    // `cover`, le titre n'est pas dessiné du tout — un titre sur un dos de
    // 6 mm rate le pli de plus que sa propre hauteur. Un dos nu est un livre
    // légitime, donc c'est un avertissement ; le taire serait laisser partir
    // une tranche vide sans que personne l'ait choisi.
    if profil.fichiers == Fichiers::Deux {
        if let Some(spine) = &cg.spine {
            if spine.w < cover::SPINE_TEXT_MIN_MM {
                defauts.push(Defaut {
                    regle: "dos_nu",
                    bloquant: false,
                    planche: None,
                    case_idx: None,
                    src: None,
                    cause: format!(
                        "le dos fait {:.1} mm : sous {:.0} mm il ne porte pas de titre, la tranche \
                         sortira nue",
                        spine.w,
                        cover::SPINE_TEXT_MIN_MM
                    ),
                    remede: "ajoutez des planches : le dos s'épaissit avec le livre, et le \
                             titre s'y pose tout seul"
                        .into(),
                    phrase: phrase(
                        "dos_nu",
                        "dos_nu.ajouter",
                        vec![
                            ("dos_mm", format!("{:.1}", spine.w)),
                            ("min_mm", format!("{:.0}", cover::SPINE_TEXT_MIN_MM)),
                        ],
                    ),
                });
            }
        }
    }

    // La quatrième (1.2-d) : sa photo, son texte, dans la page que ce
    // fournisseur reçoit. Le cadre se pose dans la boîte intérieure, donc la
    // photo ne passe jamais le pli ; le texte, lui, peut ne pas tenir — un
    // mot plus large que la page, un texte trop long sous la photo — et ces
    // lignes-là ne s'impriment pas, ou la coupe les mange. Le pli bute, la
    // marge avertit.
    if let Some(c) = &album.cover {
        let panneau = match profil.fichiers {
            Fichiers::Deux => cg.back.clone(),
            Fichiers::Un => cover::page_geometry(album, profil, cover::Face::Quatrieme).back,
        };
        let taille = cover::taille_quatrieme(c, album.trim_mm.w);
        let q = cover::quatrieme(&panneau, profil.safe_mm, c, taille, mesure);
        let (dehors, zone) = q.ecarts(&panneau, profil.safe_mm);
        if dehors > 0 {
            defauts.push(Defaut {
                regle: "quatrieme_deborde",
                bloquant: true,
                planche: None,
                case_idx: None,
                src: None,
                cause: format!(
                    "{dehors} ligne(s) de la quatrième ne tiennent pas dans la page : elles \
                     sortent du panneau, montent sur la photo ou ne s'impriment pas"
                ),
                remede: "raccourcissez le texte de la quatrième, ou réduisez sa taille ou celle \
                         de sa photo"
                    .into(),
                phrase: phrase(
                    "quatrieme_deborde",
                    "quatrieme_deborde.raccourcir",
                    vec![("n", dehors.to_string())],
                ),
            });
        }
        if zone > 0 {
            defauts.push(Defaut {
                regle: "quatrieme_zone_sure",
                bloquant: false,
                planche: None,
                case_idx: None,
                src: None,
                cause: format!(
                    "le texte ou la photo de la quatrième entre dans la marge de sécurité de \
                     {:.1} mm que demande {}",
                    profil.safe_mm, profil.nom
                ),
                remede: "réduisez la taille du texte de la quatrième, ou coupez le mot trop long"
                    .into(),
                phrase: phrase(
                    "quatrieme_zone_sure",
                    "quatrieme_zone_sure.reduire",
                    vec![
                        ("imprimeur", profil.nom.into()),
                        ("sure_mm", format!("{:.1}", profil.safe_mm)),
                    ],
                ),
            });
        }
        if let (Some(p), Some(cadre)) = (&c.back_photo, &q.cadre) {
            if let Some(&(ow, oh)) = dims.get(p.photo.src.as_str()) {
                let ppi = print::PRINT_DPI / (print::print_scale(cadre, ow, oh) * p.photo.zoom.max(1.0));
                if ppi < profil.min_ppi {
                    defauts.push(Defaut {
                        regle: "quatrieme_resolution",
                        bloquant: true,
                        planche: None,
                        case_idx: None,
                        src: Some(p.photo.src.clone()),
                        cause: format!(
                            "{} imprimerait à {ppi:.0} ppi sur la quatrième, {} exige {:.0}",
                            p.photo.src, profil.nom, profil.min_ppi
                        ),
                        remede: "choisissez une photo plus définie pour la quatrième, ou un \
                                 cadre plus petit"
                            .into(),
                        phrase: phrase(
                            "quatrieme_resolution",
                            "quatrieme_resolution.choisir",
                            vec![
                                ("src", p.photo.src.clone()),
                                ("ppi", format!("{ppi:.0}")),
                                ("imprimeur", profil.nom.into()),
                                ("min_ppi", format!("{:.0}", profil.min_ppi)),
                            ],
                        ),
                    });
                }
            }
        }
    }

    // 8. The spine. Not a defect: a number that travels, and must travel with
    // its provenance attached.
    let dos_mm = profil.dos_mm(pages, GRAMMAGE_DEFAUT);
    if let Dos::Calcule { certitude: Certitude::Provisoire, .. } = profil.dos {
        if let Some(d) = dos_mm {
            defauts.push(Defaut {
                regle: "dos",
                bloquant: false,
                planche: None,
                case_idx: None,
                src: None,
                cause: format!(
                    "dos calculé à {d:.1} mm avec un coefficient provisoire, non confirmé par {}",
                    profil.nom
                ),
                remede: "confirmez la formule auprès de l'imprimeur avant un tirage, ou mesurez sur le premier album reçu".into(),
                phrase: phrase(
                    "dos",
                    "dos.confirmer",
                    vec![("dos_mm", format!("{d:.1}")), ("imprimeur", profil.nom.into())],
                ),
            });
        }
    }

    // 9. Les fichiers posés à côté d'`album.json`, quand il y en a.
    //
    // Le prévol mesurait l'album contre un profil et n'avait jamais regardé ce
    // qui dormait dans le dossier. Le 08/09 la couverture posée pour la
    // commande était le rendu `generique` — 426 × 216, sans dos, sans rempli —
    // pendant que la fiche annonçait la feuille de Cloudprinter, et le rapport
    // disait `ok: true`. Quarante-quatre octets séparaient les deux, sous le
    // même nom, et ce qui a rattrapé l'erreur est une empreinte notée à la
    // main dans un journal.
    //
    // Absent ne produit rien : la plupart des albums n'ont jamais été
    // exportés. Présent et faux bloque : une feuille au mauvais format part à
    // la presse et revient fausse, et aucune main ne la rattrape après coup.
    // C'est la doctrine de 6.4 telle quelle — ce que la presse imprime bute,
    // ce qu'un fournisseur préfère avertit.
    if let Some(pose) = &poses.interieur {
        let attendues = pages_attendues(album, profil, &g);
        if let Some((cause, remede, phrase)) =
            confronter(&pose.lecture, &attendues, LIVRAISON_INTERIEUR, "--print", profil)
        {
            defauts.push(Defaut {
                regle: "fichier_interieur",
                bloquant: true,
                planche: None,
                case_idx: None,
                src: None,
                cause,
                remede,
                phrase,
            });
        }
    }
    // La couverture ne se contrôle que chez qui en attend une, c'est-à-dire
    // chez qui relie deux fichiers. Chez les autres elle voyage **dans**
    // l'intérieur, et `album-cover.pdf` n'est qu'un sous-produit que personne
    // n'envoie : refuser dessus refuserait un fichier dont la livraison ne
    // veut pas.
    if profil.fichiers == Fichiers::Deux {
        if let Some(pose) = &poses.couverture {
            let attendues = vec![[cg.media_w, cg.media_h]];
            if let Some((cause, remede, phrase)) =
                confronter(&pose.lecture, &attendues, LIVRAISON_COUVERTURE, "--cover", profil)
            {
                defauts.push(Defaut {
                    regle: "fichier_couverture",
                    bloquant: true,
                    planche: None,
                    case_idx: None,
                    src: None,
                    cause,
                    remede,
                    phrase,
                });
            }
        }
    }

    // 10. Le manifeste : de quel album ce fichier sort, et pour quel
    // imprimeur.
    //
    // La règle 9 juge une géométrie, et une géométrie ne dit pas tout : un
    // fichier au bon format, au bon compte de pages, rendu avant la dernière
    // légende, passait vert et la presse imprimait le livre d'hier. Une date
    // de fichier ne le rattraperait pas — un `mtime` ne se défend pas —, mais
    // ce que le fichier **dit de sa provenance**, si.
    //
    // Sans entrée au manifeste, silence complet : voir [`crate::export`].
    // C'est ce qui laisse passer tout export antérieur à ce module, à
    // commencer par ceux qui sont déjà vérifiés et prêts à partir.
    let empreinte = crate::export::empreinte_album(album);
    let manifeste = [
        (&poses.interieur, LIVRAISON_INTERIEUR, "--print", true),
        (
            &poses.couverture,
            LIVRAISON_COUVERTURE,
            "--cover",
            profil.fichiers == Fichiers::Deux,
        ),
    ];
    for (pose, fichier, commande, livre) in manifeste {
        // La même frontière que la règle 9, et pour la même raison : chez qui
        // relie un seul fichier, la couverture voyage dans l'intérieur et
        // `album-cover.pdf` est un sous-produit que personne n'envoie.
        if !livre {
            continue;
        }
        // Rien de posé : rien à envoyer, donc rien à refuser, même si le
        // manifeste garde le souvenir d'un rendu.
        let Some(pose) = pose else { continue };
        let Some(note) = poses.manifeste.artefact(fichier) else { continue };
        if let Some((regle, cause, remede, phrase)) =
            confronter_manifeste(pose, note, &empreinte, fichier, commande, profil)
        {
            defauts.push(Defaut {
                regle,
                bloquant: true,
                planche: None,
                case_idx: None,
                src: None,
                cause,
                remede,
                phrase,
            });
        }
    }

    if cfg!(debug_assertions) {
        for d in &defauts {
            assert!(REGLES.contains(&d.regle), "règle absente de REGLES : {}", d.regle);
            assert!(!(d.bloquant && A_VOUS_DE_VOIR.contains(&d.regle)), "{} ne bloque pas", d.regle);
        }
    }
    let bloquants = defauts.iter().filter(|d| d.bloquant).count();
    let avertissements = defauts.len() - bloquants;

    PrevolReport {
        album: album.title.clone(),
        profil: profil.id,
        ok: bloquants == 0,
        bloquants,
        avertissements,
        fiche: Fiche {
            imprimeur: profil.nom,
            format_page_mm: [album.trim_mm.w, album.trim_mm.h],
            produit,
            planches: album.spreads.len(),
            pages_interieur: pages,
            pages_fichier,
            fond_perdu_mm: profil.bleed_mm,
            zone_sure_mm: profil.safe_mm,
            // La feuille à plat, quand c'est nous qui la livrons. C'est la
            // première question d'un imprimeur qui relie du cartonné, et
            // jusqu'ici la fiche ne savait pas y répondre : les trois cotes
            // vivaient dans le profil et n'atteignaient aucun humain. Absente
            // chez qui relie un seul fichier, où la couverture est deux
            // feuillets de l'intérieur et n'a pas de feuille à elle.
            feuille_couverture_mm: (profil.fichiers == Fichiers::Deux)
                .then(|| [cg.media_w, cg.media_h]),
            espace: profil.espace,
            output_intent: profil.espace.output_intent(),
            conformite: profil.pdf_x,
            fichiers: profil.fichiers,
            dos_mm,
            grammage_g_m2: GRAMMAGE_DEFAUT,
            resolution_cible_dpi: print::PRINT_DPI,
        },
        reserves: profil.reserves.iter().map(|s| (*s).to_string()).collect(),
        notes,
        defauts,
        acquittement: pages_sous_seuil(sous_seuil, profil.min_ppi),
    }
}

/// Shortest distance from a point on the spread to the trimmed edge, in
/// millimetres. The media is the trim plus the bleed on all four sides.
/// Distance from a point to the guillotine. One implementation of the
/// doctrine, in [`crate::scene::distance_to_trim`], reached here with a
/// rectangle of no extent: what must survive the cut is measured from the
/// cut, and it is measured the same way for everyone.
fn distance_au_rognage(x: f64, y: f64, g: &pdf::SpreadGeometry, bleed: f64) -> f64 {
    debug_assert!((g.bleed - bleed).abs() < 1e-9, "deux fonds perdus pour une planche");
    scene::distance_to_trim(&pdf::Rect { x, y, w: 0.0, h: 0.0 }, 0.0, g)
}

/// Tolérance de comparaison des cotes, en millimètres. Celle qu'écrivent déjà
/// `cover.rs` et `print.rs` : le binaire rend 490,4811 pour une fiche à
/// 490,48, et comparer strictement refuserait le bon fichier.
const TOLERANCE_MM: f64 = 0.01;

/// Les pages que l'export pose dans `album-print.pdf` pour **ce** profil, dans
/// l'ordre du fichier, chacune par sa media box en millimètres.
///
/// Le compte se calcule ; il ne se lit pas dans `pages_fichier`, qui est le
/// compte **déclaré à la commande**. Les deux se séparent dès que le
/// fournisseur impose nos planches lui-même : Lulu déclare 96 pages pour un
/// fichier qui en porte 48, et brancher la fiche ici rougirait sur un fichier
/// juste.
fn pages_attendues(
    album: &Album,
    profil: &PrinterProfile,
    g: &pdf::SpreadGeometry,
) -> Vec<[f64; 2]> {
    let mut v = Vec::new();
    // Chez qui relie un seul fichier, la première et la quatrième entrent
    // dedans, et ce sont deux feuillets qui n'ont pas la taille d'une page
    // d'intérieur : c'est pourquoi la taille s'attend page par page et non
    // une fois pour tout le fichier.
    let feuillet = |face| {
        let c = cover::page_geometry(album, profil, face);
        [c.media_w, c.media_h]
    };
    if profil.fichiers == Fichiers::Un {
        v.push(feuillet(cover::Face::Premiere));
    }
    if profil.pages_simples {
        let pli = imposition::pli_mm(profil);
        let simple = |cote| {
            let p = imposition::page_simple(album, cote, pli);
            [p.media_w, p.media_h]
        };
        for i in 0..album.spreads.len() {
            v.extend(imposition::faces(i).iter().map(|c| simple(*c)));
        }
        if !album.spreads.is_empty() {
            v.push(simple(imposition::Cote::Gauche));
        }
    } else {
        v.extend(std::iter::repeat([g.media_w, g.media_h]).take(album.spreads.len()));
    }
    if profil.fichiers == Fichiers::Un {
        v.push(feuillet(cover::Face::Quatrieme));
    }
    v
}

/// Ce qu'il y a à dire d'un fichier posé, ou rien s'il est celui qu'on attend.
///
/// Le compte d'abord, puis la taille de chaque page : toutes ont celle du
/// gabarit, donc une page qui sort du lot est le symptôme qu'on cherche. Une
/// seule est nommée — la première — parce qu'un fichier rendu au mauvais
/// profil les a toutes fausses et qu'une liste de quatre-vingt-seize lignes ne
/// dit rien de plus que la première.
fn confronter(
    lecture: &Lecture,
    attendues: &[[f64; 2]],
    fichier: &str,
    commande: &str,
    profil: &PrinterProfile,
) -> Option<(String, String, Phrase)> {
    let repare =
        || format!("relancez {commande} --profil {} pour le refaire, ou retirez-le du dossier", profil.id);
    let refaire = |code, mut valeurs: Vec<(&'static str, String)>| {
        valeurs.push(("fichier", fichier.to_string()));
        valeurs.push(("commande", commande.to_string()));
        valeurs.push(("profil", profil.id.to_string()));
        phrase(code, "fichier.refaire", valeurs)
    };
    let pages = match lecture {
        Lecture::Illisible(pourquoi) => {
            return Some((
                format!("{fichier} est posé à côté de l'album et ne se lit pas : {pourquoi}"),
                repare(),
                refaire("fichier.illisible", vec![("pourquoi", pourquoi.clone())]),
            ))
        }
        Lecture::Pages(pages) => pages,
    };
    if pages.len() != attendues.len() {
        return Some((
            format!(
                "{fichier} porte {} page{}, {} en attend {}",
                pages.len(),
                if pages.len() > 1 { "s" } else { "" },
                profil.nom,
                attendues.len()
            ),
            repare(),
            if pages.len() > 1 {
                refaire(
                    "fichier.pages",
                    vec![
                        ("n", pages.len().to_string()),
                        ("imprimeur", profil.nom.into()),
                        ("attendues", attendues.len().to_string()),
                    ],
                )
            } else {
                refaire(
                    "fichier.page",
                    vec![
                        ("n", pages.len().to_string()),
                        ("imprimeur", profil.nom.into()),
                        ("attendues", attendues.len().to_string()),
                    ],
                )
            },
        ));
    }
    let (i, page) = pages
        .iter()
        .zip(attendues)
        .position(|(p, a)| {
            (p[0] - a[0]).abs() > TOLERANCE_MM || (p[1] - a[1]).abs() > TOLERANCE_MM
        })
        .map(|i| (i, pages[i]))?;
    let a = attendues[i];
    Some((
        format!(
            "{fichier} : la page {} mesure {:.2} × {:.2} mm, {} en attend une de {:.2} × {:.2}",
            i + 1,
            page[0],
            page[1],
            profil.nom,
            a[0],
            a[1]
        ),
        repare(),
        refaire(
            "fichier.taille",
            vec![
                ("page", (i + 1).to_string()),
                ("w_mm", format!("{:.2}", page[0])),
                ("h_mm", format!("{:.2}", page[1])),
                ("imprimeur", profil.nom.into()),
                ("attendu_w_mm", format!("{:.2}", a[0])),
                ("attendu_h_mm", format!("{:.2}", a[1])),
            ],
        ),
    ))
}

/// Ce que le manifeste a à dire d'un fichier posé, ou rien s'il est celui
/// qu'on croit.
///
/// Deux bloquants, et deux seulement. `fichier_profil` : ce PDF a été rendu
/// pour un autre imprimeur. `fichier_perime` : il ne sort pas de l'album
/// d'aujourd'hui — soit parce que l'album a changé depuis, soit parce que le
/// fichier lui-même a été remplacé. Ni l'un ni l'autre n'est une préférence de
/// fournisseur : c'est envoyer le mauvais livre, donc les deux butent.
///
/// Le profil d'abord : un fichier rendu ailleurs est faux de toute façon, et
/// son empreinte d'album ne dirait rien de plus.
fn confronter_manifeste(
    pose: &Pose,
    note: &Artefact,
    empreinte: &str,
    fichier: &str,
    commande: &str,
    profil: &PrinterProfile,
) -> Option<(&'static str, String, String, Phrase)> {
    let repare = || {
        format!(
            "relancez {commande} --profil {} pour le refaire, ou retirez-le du dossier",
            profil.id
        )
    };
    let refaire = |code, mut valeurs: Vec<(&'static str, String)>| {
        valeurs.push(("fichier", fichier.to_string()));
        valeurs.push(("commande", commande.to_string()));
        valeurs.push(("profil", profil.id.to_string()));
        phrase(code, "fichier.refaire", valeurs)
    };
    if note.profil != profil.id {
        return Some((
            "fichier_profil",
            format!(
                "{fichier} a été rendu pour le profil {}, et c'est {} ({}) qui attend le \
                 fichier : c'est le livre d'un autre imprimeur",
                note.profil, profil.id, profil.nom
            ),
            repare(),
            refaire(
                "fichier_profil",
                vec![("rendu", note.profil.clone()), ("imprimeur", profil.nom.into())],
            ),
        ));
    }
    if note.album != empreinte {
        return Some((
            "fichier_perime",
            format!(
                "{fichier} a été rendu sur une autre version de l'album : empreinte {} au \
                 rendu, {empreinte} aujourd'hui",
                note.album
            ),
            repare(),
            refaire(
                "fichier_perime.album",
                vec![("avant", note.album.clone()), ("maintenant", empreinte.to_string())],
            ),
        ));
    }
    if note.octets != pose.octets {
        return Some((
            "fichier_perime",
            format!(
                "{fichier} pèse {} octets, le rendu en avait écrit {} : le fichier a été \
                 remplacé depuis",
                pose.octets, note.octets
            ),
            repare(),
            refaire(
                "fichier_perime.octets",
                vec![("octets", pose.octets.to_string()), ("rendu", note.octets.to_string())],
            ),
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Size, Slot, Spread};

    fn album_de(n: usize, bleed: f64) -> Album {
        let mut a = Album::new("t", Path::new("/p"), Size { w: 210.0, h: 210.0 });
        a.bleed_mm = bleed;
        // Une couverture composée, comme le Composer en pose une depuis
        // K-s1 : sans elle, `couverture_vide` bloquerait tout album de test
        // chez qui relie deux fichiers. Sa photo n'a pas de dimensions dans
        // les relevés des tests, donc sa résolution n'est jamais mesurée ici :
        // les tests de couverture posent la leur.
        a.cover = Some(crate::model::Cover {
            title: "t".into(),
            subtitle: String::new(),
            photo: Some(Slot::new("couverture.jpg".into(), [0.5, 0.5])),
            back_text: String::new(),
            fond: None,
            ..Default::default()
        });
        for i in 0..n {
            a.spreads.push(Spread {
                template: "solo".into(),
                slots: vec![Slot::new(format!("{i}.jpg"), [0.5, 0.5])],
                caption: None,
                text: None,
                edited: false,
                locked: false,
                objets: Vec::new(),
            });
        }
        a
    }

    /// La fiche nomme le produit Cloudprinter du format, et rien quand
    /// l'imprimeur n'en a pas : c'est elle qui dit à l'écran si Commander a
    /// un sens. Un 28 × 21 se prépare pour Cloudprinter, mais ne se vend pas.
    #[test]
    fn la_fiche_nomme_le_produit_du_format_ou_rien() {
        let dims: HashMap<String, (u32, u32)> = (0..24).map(|i| (format!("{i}.jpg"), (5000u32, 5000u32))).collect();
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let carre = album_de(12, 3.0);
        assert_eq!(check(&carre, cp, &dims, &FichiersPoses::default()).fiche.produit, Some("photobook_cw_s210_s_fc"));
        let mut paysage = album_de(12, 3.0);
        paysage.trim_mm = Size { w: 280.0, h: 210.0 };
        assert_eq!(check(&paysage, cp, &dims, &FichiersPoses::default()).fiche.produit, None);
        let mut libre = album_de(12, 3.0);
        libre.trim_mm = Size { w: 211.0, h: 211.0 };
        assert_eq!(check(&libre, cp, &dims, &FichiersPoses::default()).fiche.produit, None);
        // Prodigi ne se commande pas depuis l'app : pas de produit, même carré.
        let pr = PrinterProfile::par_id("prodigi").unwrap();
        assert_eq!(check(&carre, pr, &dims, &FichiersPoses::default()).fiche.produit, None);
    }

    /// Un format que l'imprimeur commandé ne relie pas se dit au prévol, et
    /// bloque. Le 13/10 un 28 × 21 se préparait vert pour Cloudprinter, puis
    /// la commande tombait sur un refus du relais : aucun de ses livres n'a
    /// ces cotes, même commandé à la main. Chez qui l'app ne commande pas, la
    /// règle se tait — leurs produits ne sont pas listés ici, et un silence
    /// vaut mieux qu'un refus inventé.
    #[test]
    fn un_format_sans_produit_bloque_chez_qui_se_commande() {
        let dims: HashMap<String, (u32, u32)> = (0..24).map(|i| (format!("{i}.jpg"), (5000u32, 5000u32))).collect();
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let regle = |a: &Album, p: &'static PrinterProfile| {
            check(a, p, &dims, &FichiersPoses::default())
                .defauts
                .into_iter()
                .filter(|d| d.regle == "format_sans_produit")
                .collect::<Vec<_>>()
        };

        let mut paysage = album_de(12, 3.0);
        paysage.trim_mm = Size { w: 280.0, h: 210.0 };
        let d = regle(&paysage, cp);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].bloquant);
        assert_eq!(d[0].planche, None);
        assert!(d[0].cause.starts_with("Cloudprinter n'imprime pas ce format"), "{}", d[0].cause);
        assert!(d[0].cause.contains("280 × 210 mm"), "{}", d[0].cause);
        assert_eq!(d[0].remede, "choisissez un des formats qu'il imprime dans Format");
        assert_eq!(d[0].phrase.cause, "format_sans_produit");
        assert_eq!(d[0].phrase.remede, "format_sans_produit.choisir");
        assert!(!check(&paysage, cp, &dims, &FichiersPoses::default()).ok);

        // Une taille libre n'a pas de produit non plus.
        let mut libre = album_de(12, 3.0);
        libre.trim_mm = Size { w: 211.0, h: 211.0 };
        assert_eq!(regle(&libre, cp).len(), 1);

        // Le carré se commande, et Prodigi ne se commande pas depuis l'app.
        assert!(regle(&album_de(12, 3.0), cp).is_empty());
        let pr = PrinterProfile::par_id("prodigi").unwrap();
        assert!(regle(&paysage, pr).is_empty());
    }

    /// La fenêtre ne juge que l'album. Le 13/10 un 30 × 30 refusait la
    /// commande sur un `album-cover.pdf` resté à côté de l'album d'un export
    /// d'un autre format, avec un conseil de ligne de commande à l'écran. Les
    /// fichiers se jugent là où on les envoie, dans le dossier préparé
    /// (`prevol_dossier`) ; la CLI garde `prevol`, qui les lit à côté.
    #[test]
    fn le_prevol_de_l_album_ne_juge_pas_les_fichiers_d_a_cote() {
        let dir = std::env::temp_dir()
            .join(format!("colophon-prevol-album-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let a = album_de(24, 3.0);
        fs::write(dir.join("album.json"), serde_json::to_string(&a).unwrap()).unwrap();
        // La feuille d'un autre format, sous le nom de la livraison.
        pdf_d_une_page(&dir.join("album-cover.pdf"), 622.2, 306.0);
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();

        let cli = prevol(&dir, cp).unwrap();
        assert!(cli.defauts.iter().any(|d| d.regle == "fichier_couverture"), "{:?}", cli.defauts);

        let app = prevol_album(&dir, cp).unwrap();
        assert!(!app.defauts.iter().any(|d| d.regle.starts_with("fichier")), "{:?}", app.defauts);
        let sans_fichiers = |r: &PrevolReport| {
            r.defauts
                .iter()
                .filter(|d| !d.regle.starts_with("fichier"))
                .map(|d| (d.regle, d.cause.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(sans_fichiers(&cli), sans_fichiers(&app));

        let _ = fs::remove_dir_all(&dir);
    }

    /// Un fichier posé dont on ne juge que la géométrie : le manifeste des
    /// tests de #33 est vide, donc les octets ne sont confrontés à rien.
    fn pose(pages: Vec<[f64; 2]>) -> Option<Pose> {
        Some(Pose { lecture: Lecture::Pages(pages), octets: 0 })
    }

    /// La page que l'imposition n'imprime pas doit être vide, et c'est ici que
    /// ça se refuse. Une première planche de photos en porte une à gauche : la
    /// découpe la perdrait sans un mot, donc elle est bloquante chez qui relie
    /// page par page, et invisible chez qui impose nos planches lui-même.
    #[test]
    fn une_premiere_planche_qui_porte_a_gauche_est_refusee() {
        let mut a = album_de(12, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();

        // `solo` pose sa photo au recto : la page de gauche est blanche et
        // rien ne se perd.
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        assert!(r.ok, "défauts : {:?}", r.defauts);

        // `duo` en pose une de chaque côté.
        a.spreads[0].template = "duo".into();
        a.spreads[0].slots.push(Slot::new("12.jpg".into(), [0.5, 0.5]));
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        let refus: Vec<&Defaut> =
            r.defauts.iter().filter(|d| d.regle == "imposition").collect();
        assert_eq!(refus.len(), 1, "défauts : {:?}", r.defauts);
        assert!(refus[0].bloquant);
        assert_eq!(refus[0].planche, Some(1));
        assert!(refus[0].remede.contains("page de garde"), "{}", refus[0].remede);
        assert!(refus[0].remede.starts_with("déplacez"), "{}", refus[0].remede);
        assert!(!r.ok);

        // La même planche ailleurs dans le livre ne gêne personne : c'est la
        // première, et seulement elle, dont la gauche ne s'imprime pas.
        let mut b = album_de(12, 3.0);
        b.spreads[5].template = "duo".into();
        b.spreads[5].slots.push(Slot::new("12.jpg".into(), [0.5, 0.5]));
        assert!(check(&b, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default()).ok);

        // Et chez un imprimeur qui relie des planches, la question ne se pose
        // pas : il n'y a pas de découpe.
        let r = check(&a, PrinterProfile::par_id("generique").unwrap(), &dims, &FichiersPoses::default());
        assert!(!r.defauts.iter().any(|d| d.regle == "imposition"), "{:?}", r.defauts);
        assert!(r.ok, "{:?}", r.defauts);
    }

    /// Le piège du fond perdu intérieur, mesuré : une pleine page recadrée par
    /// sa largeur tient les 250 ppi sur la planche composée et ne les tient
    /// plus une fois qu'elle saigne trois millimètres de plus vers le pli. Le
    /// prévol lit le rectangle que la presse reçoit, pas celui de la
    /// composition, sinon il promet une résolution que personne n'imprime.
    #[test]
    fn le_fond_perdu_du_pli_fait_tomber_une_pleine_page_sous_le_plancher() {
        let mut a = album_de(12, 3.0);
        for s in &mut a.spreads {
            s.template = "full1".into(); // page de gauche vide, pleine page à droite
        }
        // 2110 px de large pour 213 mm : 251 ppi. Pour 216 : 248. La hauteur
        // ne gouverne rien ici, la photo étant nettement plus haute que large.
        let dims: HashMap<String, (u32, u32)> = (0..12)
            .map(|i| (format!("{i}.jpg"), (2110u32, 3000u32)))
            .collect();

        let r = check(&a, PrinterProfile::par_id("generique").unwrap(), &dims, &FichiersPoses::default());
        assert!(r.ok, "sur la planche composée elle passe : {:?}", r.defauts);

        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        let sous: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "resolution").collect();
        assert_eq!(sous.len(), 12, "une par planche : {:?}", r.defauts);
        assert!(sous[0].cause.contains("248 ppi"), "{}", sous[0].cause);

        // Une case en marge n'atteint pas le pli, donc rien ne bouge pour
        // elle : le rectangle d'export est le rectangle composé.
        let mut b = album_de(12, 3.0);
        for s in &mut b.spreads {
            s.template = "solo".into();
        }
        let dims: HashMap<String, (u32, u32)> = (0..12)
            .map(|i| (format!("{i}.jpg"), (2000u32, 3000u32)))
            .collect();
        let cp = check(&b, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        let gen = check(&b, PrinterProfile::par_id("generique").unwrap(), &dims, &FichiersPoses::default());
        assert_eq!(cp.ok, gen.ok, "cp {:?} / gen {:?}", cp.defauts, gen.defauts);
    }

    /// Un objet libre que la coupe traverse, un autre que le pli traverse :
    /// deux bloquants, chacun nommé, et le même album sans eux passe. Ce sont
    /// les deux seules choses que le prévol refuse à un objet libre — la
    /// marge, elle, est molle, et c'est le linter qui la compte.
    #[test]
    fn un_objet_libre_coupe_ou_a_cheval_sur_le_pli_est_bloquant() {
        use crate::model::{Alignement, Contenu, Objet};

        let bloc = |x: f64, y: f64| Objet {
            couleur: None,
            police: None,
            x,
            y,
            w: 40.0,
            h: 20.0,
            angle: 0.0,
            contenu: Contenu::Texte {
                texte: "Calvi".into(),
                taille_pt: 10.0,
                interligne_mm: Some(5.0),
                alignement: Alignement::Gauche,
            },
        };

        let mut a = album_de(24, 3.0);
        let g = pdf::geometry(&a);
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let profil = PrinterProfile::par_id("generique").unwrap();

        // Bien à l'intérieur d'une page : rien à dire, l'album passe.
        a.spreads[0].objets = vec![bloc(40.0, 60.0)];
        let r = check(&a, profil, &dims, &FichiersPoses::default());
        assert!(r.ok, "défauts : {:?}", r.defauts);

        // Le coin gauche passe sous la coupe.
        a.spreads[0].objets = vec![bloc(g.bleed - 10.0, 60.0)];
        let r = check(&a, profil, &dims, &FichiersPoses::default());
        let coupe: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "objet_coupe").collect();
        assert_eq!(coupe.len(), 1, "défauts : {:?}", r.defauts);
        assert!(coupe[0].bloquant);
        assert_eq!(coupe[0].planche, Some(1));
        assert!(!r.ok);

        // À cheval sur le pli.
        a.spreads[0].objets = vec![bloc(g.media_w / 2.0 - 20.0, 60.0)];
        let r = check(&a, profil, &dims, &FichiersPoses::default());
        let pli: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "objet_pli").collect();
        assert_eq!(pli.len(), 1, "défauts : {:?}", r.defauts);
        assert!(pli[0].bloquant);
        assert!(!r.ok);

        // Et l'angle compte : droit il s'arrête 5 mm avant le pli, tourné son
        // coin le passe de 10. C'est la scène qui le dit, pas la boîte.
        let frole = Objet { w: 40.0, h: 60.0, ..bloc(g.media_w / 2.0 - 45.0, 60.0) };
        a.spreads[0].objets = vec![frole.clone()];
        let droit = check(&a, profil, &dims, &FichiersPoses::default());
        assert!(droit.ok, "droit il ne touche rien : {:?}", droit.defauts);
        a.spreads[0].objets = vec![Objet { angle: 45.0, ..frole }];
        let r = check(&a, profil, &dims, &FichiersPoses::default());
        assert!(
            r.defauts.iter().any(|d| d.regle == "objet_pli"),
            "le coin tourné passe le pli : {:?}",
            r.defauts
        );
    }

    /// Big originals, matching bleed, sane pagination: nothing stops the file,
    /// at the loosest supplier and at the strictest. This is the state the
    /// export has to reach, and the one the whole session was about.
    #[test]
    fn a_clean_album_passes() {
        let a = album_de(24, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();

        let r = check(&a, PrinterProfile::par_id("generique").unwrap(), &dims, &FichiersPoses::default());
        assert!(r.ok, "défauts : {:?}", r.defauts);
        assert_eq!(r.bloquants, 0);

        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        assert!(r.ok, "défauts : {:?}", r.defauts);
        assert_eq!(r.bloquants, 0);
        // The spine is no longer announced: Cloudprinter wrote its bulk down,
        // so the coefficient is theirs and the warning has nothing to warn
        // about. What is still unknown about that spine — it varies between
        // their production sites, by their own answer — is not a property of
        // our arithmetic and travels in the profile's reserves, which the
        // sheet carries and which the test below reads.
        assert_eq!(r.avertissements, 0, "{:?}", r.defauts);
        assert!(r.fiche.dos_mm.unwrap() > 0.0);
        assert!(
            r.reserves.iter().any(|s| s.contains("varient d'un imprimeur")),
            "la variance entre sites a quitté le rapport : {:?}",
            r.reserves
        );
        // And what the declaration rests on is said, not implied.
        assert!(
            r.notes.iter().any(|n| n.contains("PDF/A-2b")),
            "la note sur la mesure de conformité a disparu : {:?}",
            r.notes
        );
    }

    /// The same album fails or passes depending on the supplier. That is the
    /// whole reason the profile is data.
    #[test]
    fn the_same_album_answers_differently_per_supplier() {
        let a = album_de(12, 0.0); // 24 pages, no bleed
        let dims: HashMap<String, (u32, u32)> = (0..12)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();

        // Prodigi generates the bleed itself and takes 24 pages: nothing in
        // the album's own numbers bothers it. The shape of the interior used
        // to, and no longer does — the export cuts the block page by page for
        // whoever binds it that way — so it now passes clean. That is the
        // profile going from unusable to deliverable without a number of its
        // own moving.
        let r = check(&a, PrinterProfile::par_id("prodigi").unwrap(), &dims, &FichiersPoses::default());
        assert!(r.ok, "{:?}", r.defauts);
        assert_eq!(r.bloquants, 0, "{:?}", r.defauts);

        // Cloudprinter wants 3 mm of bleed we did not render: blocked.
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        assert!(!r.ok);
        assert!(r.defauts.iter().any(|d| d.regle == "fond_perdu" && d.bloquant));

        // Lulu wants CMYK, which we do not produce, and 32 pages minimum.
        let r = check(&a, PrinterProfile::par_id("lulu").unwrap(), &dims, &FichiersPoses::default());
        assert!(!r.ok);
        assert!(r.defauts.iter().any(|d| d.regle == "espace"));
        assert!(r.defauts.iter().any(|d| d.regle == "pagination"));
    }

    /// A supplier who binds one file counts the cover in it. Eleven spreads
    /// make a 22-page book block and a 24-page file, which is exactly their
    /// minimum: counting the block instead would refuse an album they accept,
    /// and the same two pages the other way round would put a real order on
    /// hold for a count that disagrees with the file.
    #[test]
    fn a_single_file_supplier_counts_its_cover() {
        let dims: HashMap<String, (u32, u32)> = (0..11)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let a = album_de(11, 0.0); // 22 pages inside, 24 in the file
        let pr = PrinterProfile::par_id("prodigi").unwrap();
        let r = check(&a, pr, &dims, &FichiersPoses::default());
        assert!(
            !r.defauts.iter().any(|d| d.regle == "pagination"),
            "24 pages de fichier tiennent dans les bornes : {:?}",
            r.defauts
        );

        // Cloudprinter binds the cover separately, so 22 stays 22, under its
        // own minimum of 24.
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        let d = r.defauts.iter().find(|d| d.regle == "pagination").unwrap();
        assert!(d.cause.contains("22 pages"), "{}", d.cause);
    }

    /// La quatrième (1.2-d) : un texte trop long sous une grande photo ne
    /// s'imprime pas en entier et bloque ; un mot qui entre dans la marge
    /// avertit ; une photo trop petite pour son cadre bloque. La même
    /// quatrième, courte, ne dit rien.
    #[test]
    fn la_quatrieme_se_juge_sur_sa_page() {
        use crate::model::{PhotoQuatrieme, PositionQuatrieme, Slot, TailleQuatrieme};
        let mut dims: HashMap<String, (u32, u32)> =
            (0..24).map(|i| (format!("{i}.jpg"), (5000u32, 5000u32))).collect();
        dims.insert("couverture.jpg".into(), (6000, 6000));
        dims.insert("dos.jpg".into(), (3000, 3000));
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let regles_avec = |a: &Album, dims: &HashMap<String, (u32, u32)>| -> Vec<(&'static str, bool)> {
            check(a, cp, dims, &FichiersPoses::default())
                .defauts
                .into_iter()
                .filter(|d| d.regle.starts_with("quatrieme_"))
                .map(|d| (d.regle, d.bloquant))
                .collect()
        };
        let regles = |a: &Album| regles_avec(a, &dims);
        let mut a = album_de(24, 3.0);
        let c = a.cover.as_mut().unwrap();
        c.back_text = "Pour Marie.".into();
        c.back_photo = Some(PhotoQuatrieme {
            photo: Slot::new("dos.jpg".into(), [0.5, 0.5]),
            position: PositionQuatrieme::Haut,
            taille: TailleQuatrieme::Grande,
        });
        assert!(regles(&a).is_empty(), "{:?}", regles(&a));

        let mut long = a.clone();
        long.cover.as_mut().unwrap().back_text = vec!["une ligne"; 40].join("\n");
        assert_eq!(regles(&long), vec![("quatrieme_deborde", true)]);

        let mut mot = a.clone();
        let c = mot.cover.as_mut().unwrap();
        c.back_photo = None;
        // Dans la bande sûre : plus large que la page moins ses deux marges,
        // plus étroit que la page.
        c.back_text = "M".repeat(75);
        let largeur = crate::font::text_width_mm(&c.back_text, cover::taille_quatrieme(c, 210.0));
        assert!(largeur > 210.0 - 2.0 * cp.safe_mm && largeur < 210.0, "{largeur}");
        assert_eq!(regles(&mot), vec![("quatrieme_zone_sure", false)]);

        let mut flou = a.clone();
        let mut petites = dims.clone();
        petites.insert("dos.jpg".into(), (800, 800));
        assert_eq!(regles_avec(&flou, &petites), vec![("quatrieme_resolution", true)]);
        // Un cadre plus petit demande moins de pixels.
        flou.cover.as_mut().unwrap().back_photo.as_mut().unwrap().taille = TailleQuatrieme::Petite;
        petites.insert("dos.jpg".into(), (1200, 1200));
        assert!(regles_avec(&flou, &petites).is_empty(), "{:?}", regles_avec(&flou, &petites));
    }

    /// An odd page count cannot be bound, and the message says so in words.
    #[test]
    fn pagination_is_named_in_words() {
        let mut a = album_de(11, 3.0); // 22 pages, under the minimum of 24
        a.bleed_mm = 3.0;
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &HashMap::new(), &FichiersPoses::default());
        let d = r.defauts.iter().find(|d| d.regle == "pagination").unwrap();
        assert!(d.cause.contains("22 pages"), "{}", d.cause);
        assert!(!d.cause.contains("ERR"), "aucun code dans le message");
        assert!(!d.remede.is_empty());
    }

    /// The pages the machine writes are two pages like any spread. An album
    /// sitting on the supplier's upper bound is pushed over it by one of
    /// them, and the remedy names the one click that fixes it rather than
    /// sending somebody hunting through a hundred spreads.
    #[test]
    fn the_machine_pages_are_named_when_they_are_the_two_pages_too_many() {
        let dims: HashMap<String, (u32, u32)> = (0..401)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let pr = PrinterProfile::par_id("cloudprinter").unwrap(); // 24 à 800 pages

        // 400 spreads = 800 pages, exactly the bound: nothing to report.
        let a = album_de(400, 3.0);
        let r = check(&a, pr, &dims, &FichiersPoses::default());
        assert!(!r.defauts.iter().any(|d| d.regle == "pagination"), "{:?}", r.defauts);

        // The colophon page makes 802, and the remedy says which page to drop.
        let mut avec = a.clone();
        avec.spreads.push(crate::colophon::spread(
            &crate::colophon::Faits {
                photos_retenues: 100,
                photos_scannees: 400,
                debut: None,
                fin: None,
                lieux: Vec::new(),
                appareils: Vec::new(),
                compose_le: chrono::NaiveDate::from_ymd_opt(2026, 8, 17).unwrap(),
            },
            avec.trim_mm,
            150.0,
            "0.9.0",
            crate::langue::Langue::Fr,
        ));
        let r = check(&avec, pr, &dims, &FichiersPoses::default());
        let d = r.defauts.iter().find(|d| d.regle == "pagination").unwrap();
        assert!(d.cause.contains("802 pages"), "{}", d.cause);
        assert!(d.remede.contains("colophon"), "{}", d.remede);

        // A book without a colophon but with a half-title: the remedy names
        // the page it can actually drop, not the one it does not have.
        let mut garde = a.clone();
        let f = crate::colophon::Faits {
            photos_retenues: 100,
            photos_scannees: 400,
            debut: None,
            fin: None,
            lieux: Vec::new(),
            appareils: Vec::new(),
            compose_le: chrono::NaiveDate::from_ymd_opt(2026, 8, 17).unwrap(),
        };
        garde.spreads.insert(0, crate::garde::spread("Corse", &f, 190.0, crate::langue::Langue::Fr));
        let d = check(&garde, pr, &dims, &FichiersPoses::default())
            .defauts
            .into_iter()
            .find(|d| d.regle == "pagination")
            .unwrap();
        assert!(d.remede.contains("de garde"), "{}", d.remede);

        // A count that is wrong for another reason keeps the general remedy.
        let court = album_de(4, 3.0);
        let d = check(&court, pr, &dims, &FichiersPoses::default())
            .defauts
            .into_iter()
            .find(|d| d.regle == "pagination")
            .unwrap();
        assert!(!d.remede.contains("colophon"), "{}", d.remede);
    }

    /// Sous le plancher, retirer n'a aucun sens : le remède ne propose que ce
    /// qui allonge le livre, et le rythme en est une manière, parce qu'un
    /// rythme plus aéré pose moins de photos par planche.
    #[test]
    fn sous_le_plancher_le_remede_allonge_le_livre() {
        let pr = PrinterProfile::par_id("cloudprinter").unwrap();
        let d = check(&album_de(11, 3.0), pr, &HashMap::new(), &FichiersPoses::default())
            .defauts
            .into_iter()
            .find(|d| d.regle == "pagination")
            .expect("22 pages, sous les 24 de Cloudprinter");
        assert!(d.bloquant);
        assert!(
            d.remede.starts_with("ajoutez des planches ou choisissez un rythme plus aéré"),
            "{}",
            d.remede
        );
        assert!(!d.remede.contains("retirez"), "{}", d.remede);
    }

    /// La planche blanche du 21/09 : `album.json` sans clé `cover`, rendue
    /// au nom du dossier, sans photo, 0,12 % d'encre, et c'est l'imprimeur
    /// qui l'a vue. Chez qui relie deux fichiers, la couverture est un
    /// fichier qu'on envoie : sans photo, elle bloque.
    #[test]
    fn une_couverture_sans_photo_bloque_chez_qui_en_attend_une() {
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let vide = |a: &Album, p: &'static PrinterProfile| -> Vec<Defaut> {
            check(a, p, &HashMap::new(), &FichiersPoses::default())
                .defauts
                .into_iter()
                .filter(|d| d.regle == "couverture_vide")
                .collect()
        };

        // Pas de clé `cover` du tout : le cas du 21/09.
        let mut sans = album_de(12, 3.0);
        sans.cover = None;
        let d = vide(&sans, cp);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].bloquant);
        assert_eq!(
            d[0].cause,
            "la couverture n'a pas de photo, elle sortirait blanche avec le nom du dossier"
        );
        assert_eq!(d[0].remede, "choisissez une photo dans Couverture");
        assert!(!check(&sans, cp, &HashMap::new(), &FichiersPoses::default()).ok);

        // Une couverture titrée sans photo sort blanche aussi, avec son
        // titre : la règle mord pareil, la phrase ne ment pas sur le titre.
        let mut titree = album_de(12, 3.0);
        titree.cover.as_mut().unwrap().photo = None;
        titree.cover.as_mut().unwrap().title = "Corse".into();
        let d = vide(&titree, cp);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].bloquant);
        assert!(d[0].cause.starts_with("la couverture n'a pas de photo, elle sortirait blanche"));
        assert!(!d[0].cause.contains("nom du dossier"), "{}", d[0].cause);

        // Avec une photo, rien.
        assert!(vide(&album_de(12, 3.0), cp).is_empty());

        // Chez qui relie un seul fichier, la règle se tait : la couverture y
        // voyage dans l'intérieur, la même frontière que `fichier_couverture`.
        for id in ["generique", "prodigi"] {
            let p = PrinterProfile::par_id(id).unwrap();
            assert!(vide(&sans, p).is_empty(), "{id}");
        }
        // Lulu relie deux fichiers, comme Cloudprinter.
        assert_eq!(vide(&sans, PrinterProfile::par_id("lulu").unwrap()).len(), 1);
    }

    /// A small original in a full-page cell is caught, and the finding names
    /// the spread, the cell and the file.
    #[test]
    fn low_resolution_names_the_spread_and_the_file() {
        let a = album_de(12, 3.0);
        let mut dims: HashMap<String, (u32, u32)> = (0..12)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        dims.insert("3.jpg".into(), (600, 600)); // far under print need
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        let d = r.defauts.iter().find(|d| d.regle == "resolution").unwrap();
        assert_eq!(d.planche, Some(4));
        assert_eq!(d.src.as_deref(), Some("3.jpg"));
        assert!(d.cause.contains("ppi"), "{}", d.cause);
        // Vrai pour la CLI comme pour l'app : retirer marche toujours, même
        // quand aucune case n'est assez petite pour cet original.
        assert_eq!(d.remede, "mettez-la dans une case plus petite, remplacez-la, ou retirez-la");
        // 1.1-c : à vous de voir, pas un refus de l'imprimeur.
        assert!(!d.bloquant);
        assert!(r.ok, "{:?}", r.defauts);
    }

    /// La résolution ne bloque plus (1.1-c) : l'album passe, et le rapport
    /// nomme les pages du livre en cause, une entrée par page, le ppi le plus
    /// bas, arrondi vers le bas. La planche k porte les pages 2k − 2 (gauche)
    /// et 2k − 1 (droite) ; la moitié gauche de la planche 1 n'a pas de page.
    #[test]
    fn la_resolution_est_a_vous_de_voir_et_nomme_ses_pages() {
        let mut a = album_de(12, 3.0);
        // Planche 4 : photo à droite (solo), page 7. Planche 6 : à gauche,
        // page 10. Planche 1 : à gauche, aucune page.
        a.spreads[5].template = "solo_verso".into();
        a.spreads[0].template = "solo_verso".into();
        // Planche 8 : un duo, une photo par page, 14 et 15.
        a.spreads[7].template = "duo".into();
        a.spreads[7].slots.push(Slot::new("7b.jpg".into(), [0.5, 0.5]));
        let mut dims = dims_de(12);
        for src in ["3.jpg", "5.jpg", "0.jpg", "7.jpg", "7b.jpg"] {
            dims.insert(src.into(), (600, 600));
        }
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let r = check(&a, cp, &dims, &FichiersPoses::default());
        // La seule chose refusée est la photo de la moitié gauche de la
        // planche 1, et pour l'imposition, pas pour sa résolution.
        let refus: Vec<&str> = r.defauts.iter().filter(|d| d.bloquant).map(|d| d.regle).collect();
        assert_eq!(refus, vec!["imposition"]);
        let res: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "resolution").collect();
        assert_eq!(res.len(), 5, "la planche 1 se dit aussi : {res:?}");
        assert!(res.iter().all(|d| !d.bloquant));
        let pages: Vec<(QuelFichier, u32)> = r.acquittement.iter().map(|p| (p.fichier, p.page)).collect();
        let i = QuelFichier::Interieur;
        assert_eq!(pages, vec![(i, 7), (i, 10), (i, 14), (i, 15)]);
        for p in &r.acquittement {
            assert!(p.ppi >= 1 && p.ppi < 250, "{p:?}");
        }
        // Le ppi est celui du moteur, arrondi vers le bas.
        let d = res.iter().find(|d| d.src.as_deref() == Some("3.jpg")).unwrap();
        let entier: u32 = d.phrase.valeurs["ppi"].parse().unwrap();
        let p7 = r.acquittement.iter().find(|p| p.page == 7).unwrap();
        assert!(p7.ppi == entier || p7.ppi + 1 == entier, "{} / {}", p7.ppi, entier);
    }

    /// Deux photos sur une même page : une seule entrée, la plus basse ; un
    /// ppi arrondi vers le bas et tenu sous le seuil ; la page 0 écartée.
    #[test]
    fn une_page_ne_s_acquitte_qu_une_fois() {
        let i = QuelFichier::Interieur;
        let c = QuelFichier::Couverture;
        let v = pages_sous_seuil(
            vec![(c, 1, 120.4), (i, 9, 201.9), (i, 3, 180.0), (i, 9, 187.2), (i, 0, 90.0), (i, 4, 249.99), (i, 5, 0.3)],
            250.0,
        );
        let t: Vec<(QuelFichier, u32, u32)> = v.iter().map(|p| (p.fichier, p.page, p.ppi)).collect();
        assert_eq!(t, vec![(i, 3, 180), (i, 4, 249), (i, 5, 1), (i, 9, 187), (c, 1, 120)]);
    }

    /// La couverture sous le seuil est à vous de voir aussi, en page 1 de son
    /// fichier.
    #[test]
    fn la_couverture_sous_le_seuil_s_acquitte_en_page_un() {
        let a = album_de(24, 3.0);
        let mut dims = dims_de(24);
        dims.insert("couverture.jpg".into(), (1200, 1200));
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        let d = r.defauts.iter().find(|d| d.regle == "couverture_resolution").unwrap();
        assert!(!d.bloquant);
        assert!(r.ok, "{:?}", r.defauts);
        assert_eq!(r.acquittement.len(), 1);
        assert_eq!((r.acquittement[0].fichier, r.acquittement[0].page), (QuelFichier::Couverture, 1));
        // Sérialisé : absent quand vide, présent sinon, sans aucun nom de fichier.
        let json = serde_json::to_string(&r.acquittement).unwrap();
        assert!(!json.contains("couverture.jpg"), "{json}");
        let propre = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims_de(24), &FichiersPoses::default());
        assert!(!serde_json::to_string(&propre).unwrap().contains("acquittement"));
    }

    /// Ce que l'imprimeur refuse grise toujours, résolution ou pas.
    #[test]
    fn a_vous_de_voir_ne_couvre_que_la_resolution() {
        assert_eq!(A_VOUS_DE_VOIR, &["couverture_resolution", "resolution"]);
        for r in A_VOUS_DE_VOIR {
            assert!(REGLES.contains(r));
        }
        let mut a = album_de(12, 3.0);
        a.trim_mm = Size { w: 280.0, h: 210.0 };
        let mut dims = dims_de(12);
        dims.insert("3.jpg".into(), (600, 600));
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        assert!(!r.ok, "le format sans produit reste refusé");
    }

    /// Missing photos never turn into a pass: the report says the check did
    /// not run.
    #[test]
    fn unreachable_photos_are_said_out_loud() {
        let a = album_de(12, 3.0);
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &HashMap::new(), &FichiersPoses::default());
        assert!(r.notes.iter().any(|n| n.contains("résolution")));
    }

    /// A profile demanding PDF/X-4 is blocked while the writer does not
    /// declare it, and a profile that asks for no conformance goes through.
    /// The check follows the renderer, so this test flips on its own the day
    /// the OutputIntent lands.
    #[test]
    fn pdf_x_conformance_blocks_only_the_profiles_that_ask_for_it() {
        let a = album_de(12, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..12)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();

        let r = check(&a, PrinterProfile::par_id("prodigi").unwrap(), &dims, &FichiersPoses::default());
        assert_eq!(
            r.defauts.iter().any(|d| d.regle == "conformite" && d.bloquant),
            !crate::pdf::EMITS_PDF_X
        );

        let r = check(&a, PrinterProfile::par_id("generique").unwrap(), &dims, &FichiersPoses::default());
        assert!(!r.defauts.iter().any(|d| d.regle == "conformite"));
    }

    /// The face really is in the file, and its licence really does allow it.
    /// The structural blocker that stood before this session.
    #[test]
    fn the_text_face_is_embeddable() {
        let m = crate::font::metrics().expect("police lisible");
        assert!(m.embeddable());
    }

    /// A bascule must not degrade the preflight, and « green » is not the
    /// bar — corse-2013 is red before any bascule, because two photographs
    /// print at 130 ppi and no format change will ever fix a photograph.
    /// The bar is: same defects, same blockers, and the only field of the
    /// whole fiche that moves is `format_page_mm`. Written as a comparison
    /// of reports, not as a list of expectations copied by hand; only the
    /// ppi inside the wording may follow the cells.
    #[test]
    fn une_bascule_ne_degrade_pas_le_prevol() {
        let a = album_de(24, 3.0);
        let mut dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        // A photograph already under the floor before any bascule.
        dims.insert("3.jpg".into(), (600, 600));
        let profil = PrinterProfile::par_id("cloudprinter").unwrap();

        let avant = check(&a, profil, &dims, &FichiersPoses::default());
        assert!(!avant.defauts.is_empty(), "le cas doit avoir quelque chose à dire");

        // Vers le paysage A4, que Cloudprinter imprime : un 28 × 21 n'a pas de
        // produit chez lui, et `format_sans_produit` y ajouterait un défaut
        // qui ne vient pas de la bascule mais du format visé.
        let (basculee, _) =
            crate::bascule::bascule(&a, Size { w: 297.0, h: 210.0 }, &dims, profil);
        let apres = check(&basculee, profil, &dims, &FichiersPoses::default());

        // Same defects, matched by what they are and where they are.
        let identite = |r: &PrevolReport| {
            r.defauts
                .iter()
                .map(|d| (d.regle, d.bloquant, d.planche, d.case_idx, d.src.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(identite(&avant), identite(&apres));
        assert_eq!(avant.bloquants, apres.bloquants);
        assert_eq!(avant.avertissements, apres.avertissements);
        assert_eq!(avant.ok, apres.ok);
        assert_eq!(avant.notes, apres.notes);

        // The whole fiche, compared as data, with the legitimate moves
        // asserted then masked. There are three, and all follow the page:
        // a bascule changes the page, the cover sheet is built around the
        // page, so a sheet that did **not** move would be the bug ; and the
        // printer's product is the page's, so it changes with it.
        let mut fa = serde_json::to_value(&avant.fiche).unwrap();
        let mut fp = serde_json::to_value(&apres.fiche).unwrap();
        for champ in ["format_page_mm", "feuille_couverture_mm", "produit"] {
            assert_ne!(fa[champ], fp[champ], "{champ}");
            fa[champ] = serde_json::Value::Null;
            fp[champ] = serde_json::Value::Null;
        }
        assert_eq!(fa, fp);
    }

    /// La zone sûre avertit et ne bloque plus, et c'est la mesure qui l'a
    /// décidé : la ligne de base d'une légende vient de la marge du format,
    /// aucune main ne la déplace, et la règle refusait donc tout album légendé
    /// chez deux fournisseurs sur quatre. Ce qui bloque est ce que la coupe
    /// traverse ; ce qu'un imprimeur préfère voir plus loin du bord s'écrit et
    /// se lit — les trois chiffres vont du simple au double.
    #[test]
    fn la_zone_sure_avertit_au_lieu_de_bloquer() {
        let mut a = album_de(24, 3.0);
        // Toutes sauf la première, dont la page de gauche porte le faux-titre
        // dans un vrai livre : une légende y tomberait à gauche, et c'est
        // l'imposition qui le refuse, pas la zone sûre.
        for s in &mut a.spreads[1..] {
            s.caption = Some("une légende comme il y en a dans tout album".into());
        }
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();

        // Prodigi garde 10 mm, la légende passe à 7 : il le dit, et il livre.
        let r = check(&a, PrinterProfile::par_id("prodigi").unwrap(), &dims, &FichiersPoses::default());
        let zs: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "zone_sure").collect();
        assert_eq!(zs.len(), 23, "une par planche légendée : {:?}", r.defauts);
        assert!(zs.iter().all(|d| !d.bloquant));
        assert!(r.ok, "un album légendé reste livrable : {:?}", r.defauts);
        assert_eq!(r.bloquants, 0);

        // Et le remède ne promet plus ce qu'il ne peut pas tenir : ni
        // raccourcir la légende ni changer de gabarit ne déplace sa ligne de
        // base. C'est cette phrase-là qui était fausse quoi qu'on décide du
        // reste, donc c'est elle que le test tient.
        assert!(!zs[0].remede.contains("raccourcissez"), "{}", zs[0].remede);
        assert!(zs[0].remede.contains("marge du format"), "{}", zs[0].remede);

        // Cloudprinter porte enfin son vrai chiffre, et la légende tombe
        // exactement dessus : 7,00 mm contre 7,0 demandés, sur un format dont
        // la marge fait 14. Rien à dire, et rien qui ait été tu.
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        assert!(!r.defauts.iter().any(|d| d.regle == "zone_sure"), "{:?}", r.defauts);
    }

    /// Une légende de photo se mesure à sa propre ligne de base, plus à
    /// l'ancre du chapitre que la planche ne porte pas (1.2-b) : la mention
    /// d'une case, sous la case.
    #[test]
    fn la_zone_sure_mesure_la_legende_de_la_photo_a_sa_place() {
        let mut a = album_de(24, 3.0);
        for s in &mut a.spreads[1..] {
            s.slots[0].mention = Some("Calvi, 27 octobre 2013".into());
        }
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let g = pdf::geometry(&a);
        let sc = scene::Scene::of(&a.spreads[1], &g);
        let at = sc
            .objects
            .iter()
            .find_map(|o| match &o.role {
                scene::Role::PhotoCaption { at, .. } => Some(*at),
                _ => None,
            })
            .expect("la mention s'imprime sous la case");
        let d = distance_au_rognage(at.x, at.y, &g, a.bleed_mm);
        assert!((d - 7.0).abs() > 0.5, "la case n'est pas l'ancre du chapitre : {d}");

        let lulu = PrinterProfile::par_id("lulu").unwrap();
        let r = check(&a, lulu, &dims, &FichiersPoses::default());
        let zs: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "zone_sure").collect();
        assert_eq!(zs.len(), usize::from(d < lulu.safe_mm) * 23, "{:?}", r.defauts);
        if let Some(z) = zs.first() {
            assert!(z.cause.contains(&format!("{d:.1} mm")), "{}", z.cause);
        }
        let prodigi = PrinterProfile::par_id("prodigi").unwrap();
        let r = check(&a, prodigi, &dims, &FichiersPoses::default());
        let n = r.defauts.iter().filter(|d| d.regle == "zone_sure").count();
        assert_eq!(n, usize::from(d < prodigi.safe_mm) * 23, "{:?}", r.defauts);
    }

    /// La couverture se mesure sur la feuille que ce fournisseur reçoit, et
    /// les deux feuilles n'ont pas la même taille : le carton habillé court
    /// bien au-delà du livre, le feuillet volant s'arrête au fond perdu. Une
    /// photographie peut donc être une couverture chez l'un et pas chez
    /// l'autre, avec les mêmes pixels.
    #[test]
    fn la_couverture_se_mesure_sur_la_feuille_du_fournisseur() {
        let mut a = album_de(24, 3.0);
        // 2400 px : 282 ppi sur le feuillet de 216 mm de Prodigi, 255 sur les
        // 239 mm de la feuille cartonnée. Les deux passent.
        a.cover = Some(crate::model::Cover {
            title: "t".into(),
            subtitle: String::new(),
            photo: Some(Slot::new("couv.jpg".into(), [0.5, 0.5])),
            back_text: String::new(),
            fond: None,
            ..Default::default()
        });
        let mut dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        dims.insert("couv.jpg".into(), (2400, 2600));
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let pr = PrinterProfile::par_id("prodigi").unwrap();
        let couv = |r: &PrevolReport| -> Option<String> {
            r.defauts
                .iter()
                .find(|d| d.regle == "couverture_resolution")
                .map(|d| d.cause.clone())
        };
        assert!(couv(&check(&a, cp, &dims, &FichiersPoses::default())).is_none());
        assert!(couv(&check(&a, pr, &dims, &FichiersPoses::default())).is_none());

        // 2100 px : 246 ppi sur le cartonné, 247 sur le feuillet. Le premier
        // refuse, le second aussi — mais pas pour la même surface.
        dims.insert("couv.jpg".into(), (2100, 2300));
        let d = couv(&check(&a, cp, &dims, &FichiersPoses::default())).expect("le cartonné refuse");
        assert!(d.contains("239 × 258"), "{d}");
        assert!(check(&a, cp, &dims, &FichiersPoses::default()).defauts.iter().any(|d| d.regle
            == "couverture_resolution"
            && !d.bloquant));

        // Et la surface exacte est celle du fournisseur, pas une moyenne :
        // 2200 px passent chez Prodigi et pas chez Cloudprinter, avec la même
        // photographie et le même album.
        dims.insert("couv.jpg".into(), (2200, 2400));
        assert!(couv(&check(&a, cp, &dims, &FichiersPoses::default())).is_some(), "260 ppi sur 239 mm : non");
        assert!(couv(&check(&a, pr, &dims, &FichiersPoses::default())).is_none(), "258 ppi sur 216 mm : oui");

        // Une couverture sans photo ne se mesure pas, et un album sans
        // couverture non plus.
        a.cover.as_mut().unwrap().photo = None;
        assert!(couv(&check(&a, cp, &dims, &FichiersPoses::default())).is_none());
        a.cover = None;
        assert!(couv(&check(&a, cp, &dims, &FichiersPoses::default())).is_none());
    }

    /// Un dos trop mince ne porte pas de titre, et le prévol le dit au lieu de
    /// laisser partir une tranche nue que personne n'a choisie. Ce n'est pas
    /// un défaut du fichier : c'est un livre trop fin, donc un avertissement.
    #[test]
    fn un_dos_trop_mince_sort_nu_et_le_dit() {
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let nu = |n: usize| -> Option<Defaut> {
            let mut r = check(&album_de(n, 3.0), cp, &dims, &FichiersPoses::default());
            let i = r.defauts.iter().position(|d| d.regle == "dos_nu")?;
            Some(r.defauts.remove(i))
        };

        // 12 planches : 24 pages, leur minimum, et un dos de 7,62 mm.
        let d = nu(12).expect("un dos de 7,6 mm ne porte rien");
        assert!(!d.bloquant, "un dos nu est un livre légitime");
        assert!(d.cause.contains("7.6"), "{}", d.cause);

        // 24 planches : 9,24 mm, le titre tient.
        assert!(nu(24).is_none());

        // Et la question ne se pose pas chez qui fabrique son dos lui-même :
        // il n'y a pas de feuille à plat à décrire.
        let r = check(&album_de(12, 0.0), PrinterProfile::par_id("prodigi").unwrap(), &dims, &FichiersPoses::default());
        assert!(!r.defauts.iter().any(|d| d.regle == "dos_nu"), "{:?}", r.defauts);
    }

    /// The spec sheet carries what a printer asks on the phone.
    #[test]
    fn the_sheet_answers_the_printers_questions() {
        let a = album_de(24, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &FichiersPoses::default());
        let f = &r.fiche;
        assert_eq!(f.format_page_mm, [210.0, 210.0]);
        assert_eq!(f.pages_interieur, 48);
        assert_eq!(f.fichiers, Fichiers::Deux);
        assert_eq!(f.output_intent, "sRGB IEC61966-2.1");
        assert!(f.dos_mm.is_some());
        // La première question d'un imprimeur qui relie du cartonné : de
        // quelle taille est la feuille ? Les trois cotes vivaient dans le
        // profil et n'atteignaient personne ; ici elles sortent en un nombre.
        let dos = f.dos_mm.unwrap();
        assert_eq!(
            f.feuille_couverture_mm,
            Some([210.0 * 2.0 + dos + 5.0 * 2.0 + 24.0 * 2.0, 210.0 + 24.0 * 2.0])
        );
        // Et personne ne la donne à qui fabrique la sienne.
        let r = check(&a, PrinterProfile::par_id("prodigi").unwrap(), &dims, &FichiersPoses::default());
        assert_eq!(r.fiche.feuille_couverture_mm, None);
        // A provisional profile never travels without its reservations.
        assert!(!r.reserves.is_empty());
    }
    /// L'incident du 08/09, rejoué. La couverture posée dans le dossier de
    /// commande était le rendu `generique` — 426 × 216, sans dos, sans rempli
    /// — pendant que la fiche annonçait la feuille de Cloudprinter. Le prévol
    /// disait `ok: true` : il mesurait l'album contre un profil et n'avait
    /// jamais regardé les fichiers posés à côté. Quarante-quatre octets
    /// séparaient la bonne feuille de la mauvaise, sous le même nom.
    #[test]
    fn une_couverture_rendue_pour_un_autre_profil_est_bloquante() {
        // Les 48 planches de l'album de la commande, pour que les deux
        // feuilles soient celles du 08/09 au centième près.
        let mut a = album_de(48, 3.0);
        // La photo de `album_de` reste : une couverture sans photo bloque
        // depuis K-s1 (`couverture_vide`), et ce test regarde la géométrie.
        a.cover.as_mut().unwrap().title = "Corse".into();
        let dims: HashMap<String, (u32, u32)> = (0..48)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let gen = PrinterProfile::par_id("generique").unwrap();

        // La feuille que `--cover --profil generique` pose : deux panneaux et
        // rien autour, là où le cartonné habille un carton.
        let mauvaise = cover::geometry(&a, gen);
        let bonne = cover::geometry(&a, cp);
        assert!((mauvaise.media_w - 426.0).abs() < 0.01, "{}", mauvaise.media_w);
        assert!((mauvaise.media_h - 216.0).abs() < 0.01, "{}", mauvaise.media_h);
        assert!((bonne.media_w - 490.48).abs() < 0.01, "{}", bonne.media_w);
        assert!((bonne.media_h - 258.0).abs() < 0.01, "{}", bonne.media_h);

        let poses = FichiersPoses {
            manifeste: Manifeste::default(),
            interieur: None,
            couverture: pose(vec![[mauvaise.media_w, mauvaise.media_h]]),
        };
        let r = check(&a, cp, &dims, &poses);
        let d: Vec<&Defaut> =
            r.defauts.iter().filter(|d| d.regle == "fichier_couverture").collect();
        assert_eq!(d.len(), 1, "défauts : {:?}", r.defauts);
        assert!(d[0].bloquant);
        assert!(!r.ok);
        // Les deux cotes, celle du fichier et celle attendue : sans les deux,
        // personne ne sait laquelle des deux feuilles il tient.
        assert!(d[0].cause.contains("426.00 × 216.00"), "{}", d[0].cause);
        assert!(d[0].cause.contains("490.48 × 258.00"), "{}", d[0].cause);
        // Et le remède nomme la commande qui répare.
        assert!(d[0].remede.contains("--cover --profil cloudprinter"), "{}", d[0].remede);

        // La bonne feuille au même endroit : plus rien à dire.
        let poses = FichiersPoses {
            manifeste: Manifeste::default(),
            interieur: None,
            couverture: pose(vec![[bonne.media_w, bonne.media_h]]),
        };
        let r = check(&a, cp, &dims, &poses);
        assert!(r.ok, "défauts : {:?}", r.defauts);
        assert_eq!(r.defauts.len(), 0, "{:?}", r.defauts);

        // Et sans aucun fichier posé, la règle se tait : la plupart des albums
        // n'ont jamais été exportés, et le prévol tourne dans l'app à tout
        // moment. La fiche, elle, est celle d'aujourd'hui.
        let r = check(&a, cp, &dims, &FichiersPoses::default());
        assert!(r.ok, "{:?}", r.defauts);
        assert_eq!(r.defauts.len(), 0, "{:?}", r.defauts);
        assert_eq!(
            r.fiche.feuille_couverture_mm,
            Some([bonne.media_w, bonne.media_h])
        );
    }

    /// Le compte de pages attendu se **calcule**, il ne se lit pas dans
    /// `pages_fichier` : celui-là est le compte déclaré à la commande, qui
    /// vaut 96 chez Lulu pour un fichier de 48 planches doubles. Brancher la
    /// fiche ici rougirait sur un fichier juste.
    #[test]
    fn le_compte_de_pages_attendu_se_calcule_et_non_se_declare() {
        let a = album_de(48, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..48)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let lu = PrinterProfile::par_id("lulu").unwrap();

        // Une planche double de 426 × 216 : le fichier que Lulu attend.
        let g = pdf::geometry(&a);
        let doubles = FichiersPoses {
            interieur: pose(vec![[g.media_w, g.media_h]; 48]),
            ..Default::default()
        };

        // Chez Cloudprinter, qui relie page par page, il en faut 96.
        let r = check(&a, cp, &dims, &doubles);
        let d: Vec<&Defaut> =
            r.defauts.iter().filter(|d| d.regle == "fichier_interieur").collect();
        assert_eq!(d.len(), 1, "défauts : {:?}", r.defauts);
        assert!(d[0].bloquant);
        assert!(d[0].cause.contains("48 page"), "{}", d[0].cause);
        assert!(d[0].cause.contains("96"), "{}", d[0].cause);
        assert!(d[0].remede.contains("--print --profil cloudprinter"), "{}", d[0].remede);

        // Chez Lulu, le même fichier de 48 pages est le bon, alors même que sa
        // fiche en déclare 96 à la commande.
        let r = check(&a, lu, &dims, &doubles);
        assert_eq!(r.fiche.pages_fichier, 96);
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_interieur"),
            "{:?}",
            r.defauts
        );

        // Et le fichier de pages simples est le bon chez Cloudprinter : 96
        // pages de 216 × 216.
        let pli = imposition::pli_mm(cp);
        let p = imposition::page_simple(&a, imposition::Cote::Droite, pli);
        assert!((p.media_w - 216.0).abs() < 0.01, "{}", p.media_w);
        let simples = FichiersPoses {
            interieur: pose(vec![[p.media_w, p.media_h]; 96]),
            ..Default::default()
        };
        let r = check(&a, cp, &dims, &simples);
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_interieur"),
            "{:?}",
            r.defauts
        );

        // Une page qui sort du lot est le symptôme qu'on cherche.
        let mut pages = vec![[p.media_w, p.media_h]; 96];
        pages[40] = [g.media_w, g.media_h];
        let r = check(
            &a,
            cp,
            &dims,
            &FichiersPoses { interieur: pose(pages), ..Default::default() },
        );
        let d = r.defauts.iter().find(|d| d.regle == "fichier_interieur").unwrap();
        assert!(d.bloquant);
        assert!(d.cause.contains("page 41"), "{}", d.cause);
    }

    /// La couverture ne se contrôle que chez qui en attend une. Chez un
    /// fournisseur qui relie un seul fichier, elle voyage **dans** l'intérieur
    /// et `album-cover.pdf` n'est qu'un sous-produit que personne n'envoie :
    /// refuser dessus refuserait un fichier dont la livraison ne veut pas.
    #[test]
    fn la_couverture_ne_se_controle_que_chez_qui_en_attend_une() {
        let a = album_de(24, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let pr = PrinterProfile::par_id("prodigi").unwrap();
        assert_eq!(pr.fichiers, Fichiers::Un);

        let poses = FichiersPoses {
            manifeste: Manifeste::default(),
            interieur: None,
            couverture: pose(vec![[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]]),
            ..Default::default()
        };
        let r = check(&a, pr, &dims, &poses);
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_couverture"),
            "{:?}",
            r.defauts
        );

        // Et l'intérieur de ce fournisseur-là porte ses deux feuillets de
        // couverture : 48 pages de livre et 50 dans le fichier.
        let pli = imposition::pli_mm(pr);
        let p = imposition::page_simple(&a, imposition::Cote::Droite, pli);
        let feuillet = cover::page_geometry(&a, pr, cover::Face::Premiere);
        let mut pages = vec![[feuillet.media_w, feuillet.media_h]];
        pages.extend(std::iter::repeat([p.media_w, p.media_h]).take(48));
        pages.push([feuillet.media_w, feuillet.media_h]);
        let r = check(
            &a,
            pr,
            &dims,
            &FichiersPoses { interieur: pose(pages), ..Default::default() },
        );
        assert_eq!(r.fiche.pages_fichier, 50);
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_interieur"),
            "{:?}",
            r.defauts
        );
    }

    /// L'aperçu de la couverture a son propre nom, et le prévol ne le
    /// regarde jamais : un aperçu n'est pas ce qu'on envoie.
    ///
    /// `render_cover_preview` écrivait `album-cover.pdf`, le nom de la
    /// livraison, au profil de l'aperçu. Prévisualiser sous un profil, en
    /// changer, puis préflighter sans re-prévisualiser affichait donc un
    /// bloquant sur un fichier que personne n'avait demandé à exporter.
    ///
    /// La même feuille sous le nom de la livraison bloque, et c'est elle qui
    /// rend ce test mordant : sans elle, un lecteur qui cesserait de lire quoi
    /// que ce soit passerait pour juste.
    #[test]
    fn l_apercu_de_la_couverture_n_est_jamais_juge() {
        let dir = std::env::temp_dir()
            .join(format!("colophon-prevol-apercu-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Une feuille d'une géométrie qui n'est celle d'aucun profil.
        let apercu = dir.join("album-cover.apercu.pdf");
        pdf_d_une_page(&apercu, 999.0, 999.0);

        let a = album_de(24, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        assert_eq!(cp.fichiers, Fichiers::Deux);

        let poses = FichiersPoses::lire(&dir);
        assert!(poses.couverture.is_none(), "{:?}", poses.couverture);
        let r = check(&a, cp, &dims, &poses);
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_couverture"),
            "{:?}",
            r.defauts
        );

        // La même feuille sous le nom de la livraison, et la règle mord.
        fs::rename(&apercu, dir.join("album-cover.pdf")).unwrap();
        let poses = FichiersPoses::lire(&dir);
        assert!(
            matches!(&poses.couverture, Some(Pose { lecture: Lecture::Pages(_), .. })),
            "{:?}",
            poses.couverture
        );
        let r = check(&a, cp, &dims, &poses);
        let d = r.defauts.iter().find(|d| d.regle == "fichier_couverture").unwrap();
        assert!(d.bloquant);
        assert!(d.cause.contains("999.00"), "{}", d.cause);

        let _ = fs::remove_dir_all(&dir);
    }

    /// Un PDF d'une page, d'une géométrie choisie, écrit sur le disque : de
    /// quoi donner au lecteur du prévol un fichier qu'il lit vraiment.
    fn pdf_d_une_page(path: &Path, w_mm: f64, h_mm: f64) {
        use lopdf::{dictionary, Document, Object};
        const MM_PT: f64 = 72.0 / 25.4;
        let mut doc = Document::with_version("1.6");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => Object::Array(vec![
                0.0f32.into(),
                0.0f32.into(),
                ((w_mm * MM_PT) as f32).into(),
                ((h_mm * MM_PT) as f32).into(),
            ]),
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => Object::Array(vec![page_id.into()]),
                "Count" => 1,
            }),
        );
        let catalog = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog);
        doc.save(path).unwrap();
    }

    /// Un intérieur composé pour ce profil-ci, tel que l'export le pose : la
    /// règle de géométrie n'a rien à en dire, et c'est exactement le fichier
    /// sur lequel le manifeste est le seul témoin.
    fn interieur_juste(album: &Album, profil: &'static PrinterProfile) -> Vec<[f64; 2]> {
        pages_attendues(album, profil, &pdf::geometry(album))
    }

    fn note(fichier: &str, profil: &str, album: &str, octets: u64) -> Manifeste {
        Manifeste {
            artefacts: vec![Artefact {
                fichier: fichier.into(),
                profil: profil.into(),
                album: album.into(),
                octets,
                date: "2026-09-08T12:00:00+02:00".into(),
            }],
        }
    }

    fn dims_de(n: usize) -> HashMap<String, (u32, u32)> {
        (0..n).map(|i| (format!("{i}.jpg"), (5000u32, 5000u32))).collect()
    }

    /// Le fichier a la bonne géométrie et sort du mauvais imprimeur.
    ///
    /// C'est le cas que la géométrie ne peut pas voir : deux fournisseurs
    /// peuvent demander la même page, et alors seul le manifeste sait pour
    /// lequel des deux ce PDF a été rendu. Bloquant, parce que ce n'est pas
    /// une préférence de fournisseur — c'est envoyer le mauvais livre.
    #[test]
    fn un_fichier_rendu_pour_un_autre_imprimeur_est_bloquant() {
        let a = album_de(24, 3.0);
        let dims = dims_de(24);
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let empreinte = crate::export::empreinte_album(&a);

        let poses = FichiersPoses {
            interieur: Some(Pose {
                lecture: Lecture::Pages(interieur_juste(&a, cp)),
                octets: 4242,
            }),
            couverture: None,
            manifeste: note(LIVRAISON_INTERIEUR, "generique", &empreinte, 4242),
        };
        let r = check(&a, cp, &dims, &poses);
        // La géométrie, elle, n'a rien à dire de ce fichier : c'est bien le
        // manifeste, et lui seul, qui parle.
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_interieur"),
            "{:?}",
            r.defauts
        );
        let d: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "fichier_profil").collect();
        assert_eq!(d.len(), 1, "défauts : {:?}", r.defauts);
        assert!(d[0].bloquant);
        assert!(!r.ok);
        // Les deux noms, sinon personne ne sait lequel des deux refaire.
        assert!(d[0].cause.contains("generique"), "{}", d[0].cause);
        assert!(d[0].cause.contains("cloudprinter"), "{}", d[0].cause);
        assert!(d[0].remede.contains("--print --profil cloudprinter"), "{}", d[0].remede);

        // Le manifeste qui dit vrai : plus rien.
        let poses = FichiersPoses {
            interieur: Some(Pose {
                lecture: Lecture::Pages(interieur_juste(&a, cp)),
                octets: 4242,
            }),
            couverture: None,
            manifeste: note(LIVRAISON_INTERIEUR, "cloudprinter", &empreinte, 4242),
        };
        let r = check(&a, cp, &dims, &poses);
        assert!(r.ok, "{:?}", r.defauts);
        assert_eq!(r.defauts.len(), 0, "{:?}", r.defauts);

        // Et sans manifeste, le prévol de #33 ne bouge pas d'un défaut : un
        // export d'avant ce module ne devient pas suspect en vieillissant.
        let poses = FichiersPoses {
            interieur: Some(Pose {
                lecture: Lecture::Pages(interieur_juste(&a, cp)),
                octets: 4242,
            }),
            ..Default::default()
        };
        let r = check(&a, cp, &dims, &poses);
        assert!(r.ok, "{:?}", r.defauts);
        assert_eq!(r.defauts.len(), 0, "{:?}", r.defauts);
    }

    /// L'angle mort que ce module ferme : un fichier **juste de géométrie et
    /// vieux de contenu**. Même format, mêmes pages, la légende d'avant.
    ///
    /// Avant le manifeste, ce fichier-là passait vert et la presse imprimait
    /// le livre d'hier. Aucune règle de géométrie ne pouvait le voir, parce
    /// qu'une légende ne déplace pas un `MediaBox`.
    #[test]
    fn un_fichier_rendu_avant_une_retouche_est_perime() {
        let a = album_de(24, 3.0);
        let dims = dims_de(24);
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let empreinte = crate::export::empreinte_album(&a);
        let pages = interieur_juste(&a, cp);

        // La légende qu'on vient d'écrire, après le rendu.
        let mut apres = a.clone();
        apres.spreads[3].slots[0].caption = Some("Bonifacio, le matin".into());

        let poses = FichiersPoses {
            interieur: Some(Pose { lecture: Lecture::Pages(pages.clone()), octets: 4242 }),
            couverture: None,
            manifeste: note(LIVRAISON_INTERIEUR, "cloudprinter", &empreinte, 4242),
        };
        let r = check(&apres, cp, &dims, &poses);
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_interieur"),
            "une légende ne déplace aucun MediaBox : {:?}",
            r.defauts
        );
        let d: Vec<&Defaut> = r.defauts.iter().filter(|d| d.regle == "fichier_perime").collect();
        assert_eq!(d.len(), 1, "défauts : {:?}", r.defauts);
        assert!(d[0].bloquant);
        assert!(!r.ok);
        // Les deux empreintes, celle du rendu et celle d'aujourd'hui.
        assert!(d[0].cause.contains(&empreinte), "{}", d[0].cause);
        assert!(
            d[0].cause.contains(&crate::export::empreinte_album(&apres)),
            "{}",
            d[0].cause
        );
        assert!(d[0].remede.contains("--print --profil cloudprinter"), "{}", d[0].remede);

        // Le même album, non retouché : le manifeste dit vrai et se tait.
        let r = check(&a, cp, &dims, &poses);
        assert!(r.ok, "{:?}", r.defauts);

        // Un fichier remplacé depuis le rendu : la taille suffit à le dire, et
        // c'est tout ce que `octets` est là pour attraper.
        let poses = FichiersPoses {
            interieur: Some(Pose { lecture: Lecture::Pages(pages), octets: 4243 }),
            couverture: None,
            manifeste: note(LIVRAISON_INTERIEUR, "cloudprinter", &empreinte, 4242),
        };
        let r = check(&a, cp, &dims, &poses);
        let d = r.defauts.iter().find(|d| d.regle == "fichier_perime").unwrap();
        assert!(d.bloquant);
        assert!(d.cause.contains("4243") && d.cause.contains("4242"), "{}", d.cause);

        // Une entrée au manifeste sans le fichier qu'elle décrit ne dit rien :
        // il n'y a rien à envoyer, donc rien à refuser.
        let poses = FichiersPoses {
            manifeste: note(LIVRAISON_INTERIEUR, "generique", "0", 1),
            ..Default::default()
        };
        let r = check(&a, cp, &dims, &poses);
        assert!(r.ok, "{:?}", r.defauts);
        assert_eq!(r.defauts.len(), 0, "{:?}", r.defauts);
    }

    /// La couverture est jugée là où elle est livrée, et le manifeste suit la
    /// même frontière que la géométrie : chez qui relie un seul fichier,
    /// `album-cover.pdf` est un sous-produit que personne n'envoie.
    #[test]
    fn le_manifeste_de_la_couverture_suit_la_frontiere_de_la_livraison() {
        let a = album_de(24, 3.0);
        let dims = dims_de(24);
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let pr = PrinterProfile::par_id("prodigi").unwrap();
        let cg = cover::geometry(&a, cp);
        let empreinte = crate::export::empreinte_album(&a);

        let poses = || FichiersPoses {
            interieur: None,
            couverture: Some(Pose {
                lecture: Lecture::Pages(vec![[cg.media_w, cg.media_h]]),
                octets: 7,
            }),
            manifeste: note(LIVRAISON_COUVERTURE, "generique", &empreinte, 7),
        };

        let r = check(&a, cp, &dims, &poses());
        let d = r.defauts.iter().find(|d| d.regle == "fichier_profil").unwrap();
        assert!(d.bloquant);
        assert!(d.cause.contains(LIVRAISON_COUVERTURE), "{}", d.cause);
        assert!(d.remede.contains("--cover --profil cloudprinter"), "{}", d.remede);

        // Chez Prodigi, la couverture voyage dans l'intérieur : ni la
        // géométrie ni le manifeste ne la jugent.
        let r = check(&a, pr, &dims, &poses());
        assert!(
            !r.defauts.iter().any(|d| d.regle == "fichier_profil"),
            "{:?}",
            r.defauts
        );
    }

    /// Un fichier tronqué se dit, et ne panique jamais. C'est la leçon de
    /// l'analyseur SVG de 6.3 s1 : la personne à qui le module doit un message
    /// clair est celle qui s'apprête à commander.
    #[test]
    fn un_fichier_illisible_se_dit_au_lieu_de_paniquer() {
        let dir = std::env::temp_dir()
            .join(format!("colophon-prevol-fichiers-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Rien posé : rien lu, et rien à dire.
        let vide = FichiersPoses::lire(&dir);
        assert!(vide.interieur.is_none() && vide.couverture.is_none());

        fs::write(dir.join("album-print.pdf"), b"%PDF-1.6\n%\xe2\xe3\xcf\xd3\n1 0 obj").unwrap();
        let poses = FichiersPoses::lire(&dir);
        assert!(
            matches!(&poses.interieur, Some(Pose { lecture: Lecture::Illisible(_), .. })),
            "{:?}",
            poses.interieur
        );

        let a = album_de(24, 3.0);
        let dims: HashMap<String, (u32, u32)> = (0..24)
            .map(|i| (format!("{i}.jpg"), (5000u32, 5000u32)))
            .collect();
        let r = check(&a, PrinterProfile::par_id("cloudprinter").unwrap(), &dims, &poses);
        let d = r.defauts.iter().find(|d| d.regle == "fichier_interieur").unwrap();
        assert!(d.bloquant);
        assert!(!r.ok);
        assert!(d.cause.contains("ne se lit pas"), "{}", d.cause);

        let _ = fs::remove_dir_all(&dir);
    }

}
