//! La recomposition d'un album déjà composé, au même format ou à un autre.
//!
//! Elle vivait dans la coquille Tauri (`recompose_album`), où aucun test ne
//! l'atteignait ; elle est ici pour que l'app et la CLI passent par la même
//! fonction, et qu'un test tienne la promesse qui la rend sûre : les planches
//! éditées à la main ou verrouillées passent telles quelles, tout le reste
//! se recompose.
//!
//! **Deux paramètres, tous deux facultatifs.** Absents, c'est la recomposition
//! d'avant au bit près : le format et le rythme sont relus dans `album.json`.
//! Un format donné remplace `trim_mm`, et alors les planches épinglées passent
//! **d'abord par la bascule** ([`crate::bascule::bascule`], pas une seconde
//! implémentation) : mêmes photos, même ordre, et seul le gabarit d'une
//! planche que le nouveau format trahirait se replie. Le reste se recompose au
//! nouveau format. Un rythme donné remplace `album.densite`.

use crate::bascule;
use crate::build::{build_album, BuildOptions, BuildReport};
use crate::layout::Densite;
use crate::model::{Album, Size};
use crate::printer::PrinterProfile;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Ce qu'une recomposition peut changer, et de quoi elle rend compte.
pub struct Recomposition {
    /// Le nouveau format de page, en millimètres ; `None` garde celui de
    /// l'album.
    pub format: Option<Size>,
    /// Le nouveau rythme ; `None` garde celui de l'album.
    pub densite: Option<Densite>,
    pub langue: crate::langue::Langue,
    /// Le jour que le colophon imprime ; `None` est l'horloge, comme pour
    /// toute composition (`BuildOptions::aujourdhui`).
    pub aujourdhui: Option<chrono::NaiveDate>,
    pub progress: Box<dyn Fn(&str) + Send + Sync>,
    pub cancel: Box<dyn Fn() -> bool + Send + Sync>,
}

impl Default for Recomposition {
    fn default() -> Self {
        Self {
            format: None,
            densite: None,
            langue: crate::langue::Langue::Fr,
            aujourdhui: None,
            progress: Box::new(|_| {}),
            cancel: Box::new(|| false),
        }
    }
}

/// Combien de planches une recomposition garde telles quelles : celles
/// qu'une main a éditées ou verrouillées. Le panneau Recomposer le dit en
/// chiffre ; c'est la même règle que celle qui les retient.
pub fn planches_gardees(album: &Album) -> usize {
    album.spreads.iter().filter(|s| s.pinned()).count()
}

/// Recompose l'album de `dir` depuis son dossier de photos, et l'écrit.
///
/// `album.origin.json` ne bouge pas (la composition ne l'écrit qu'une fois),
/// et l'`album.json` d'avant survit en `.bak`, comme à chaque écriture.
pub fn recomposer(dir: &Path, opts: Recomposition) -> Result<BuildReport> {
    let json = dir.join("album.json");
    let text = std::fs::read_to_string(&json)
        .with_context(|| format!("lecture de {}", json.display()))?;
    let mut album: Album = serde_json::from_str(&text).context("album.json illisible")?;
    let root = PathBuf::from(&album.root);
    if !root.is_dir() {
        anyhow::bail!(
            "dossier de photos introuvable ({}) : impossible de recomposer",
            root.display()
        );
    }

    // Un autre format : la bascule d'abord, sur tout l'album, parce que c'est
    // elle qui sait replier un gabarit sans perdre une photo. Seules ses
    // planches épinglées comptent ensuite, le reste se recompose. Le profil
    // ne sert qu'au bilan de couverture, que personne ne lit ici.
    if let Some(trim) = opts.format {
        let tailles = bascule::tailles_du_dossier(dir, &album)?;
        let profil = PrinterProfile::par_id("generique").expect("le profil générique existe");
        let (basculee, bilan) = bascule::bascule(&album, trim, &tailles, profil);
        if !bilan.epinglees_touchees.is_empty() {
            (opts.progress)(&format!(
                "bascule : {} planches gardées changent de gabarit",
                bilan.epinglees_touchees.len()
            ));
        }
        album = basculee;
    }

    // Pinned spreads keep their place through a capture-time anchor. A
    // photo-less spread (texte, vide) inherits the time of the spread
    // before it, so it stays glued to its chapter.
    let mut pinned = Vec::new();
    let mut last: Option<chrono::NaiveDateTime> = None;
    for spread in &album.spreads {
        let t = spread
            .slots
            .first()
            .map(|sl| crate::meta::read(&root.join(&sl.src)).taken)
            .or(last);
        if spread.pinned() {
            pinned.push((spread.clone(), t));
        }
        last = t;
    }
    let target = album.spreads.len().saturating_sub(pinned.len()).max(4);

    let build = BuildOptions {
        title: Some(album.title.clone()),
        spreads: target,
        trim: album.trim_mm,
        progress: opts.progress,
        cancel: opts.cancel,
        pinned,
        cover: album.cover.clone(),
        // The pace the album was built at, read back from the file unless
        // the caller asks for another: a recomposition never quietly
        // reverts to the default one.
        densite: opts.densite.unwrap_or(album.densite),
        // Same reasoning for the colophon page: somebody who took it away
        // must not have to take it away again after every recomposition.
        colophon: album.spreads.iter().any(|s| s.template == crate::colophon::TEMPLATE),
        // And the half-title at the head of the book, on the same rule.
        garde: album.spreads.iter().any(|s| s.template == crate::garde::TEMPLATE),
        // A recomposition is not a choice of album: the user already made
        // that one, and a second offer would throw away every hand edit that
        // the pinned spreads were kept for.
        variantes: Vec::new(),
        // The adjustments follow the photographs, not the spreads: a field
        // BuildOptions did not carry would be destroyed by the rebuild.
        reglages: album.reglages.clone(),
        // And the face, on the same rule: the file stays beside album.json,
        // which the rebuild writes into, so only the album's record of it
        // could be lost — silently, and in the one place nobody would look.
        police: album.police.clone(),
        // Le colophon dit le jour où le livre a été composé, et une
        // recomposition le compose : la date du jour, sauf à la donner.
        aujourdhui: opts.aujourdhui,
        // Une recomposition récrit les lignes de chapitre, la garde et le
        // colophon : dans la langue de qui la lance. Les planches épinglées
        // gardent leurs mots, elles passent telles quelles.
        langue: opts.langue,
    };
    build_album(&root, dir, build)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model;
    use std::fs;

    fn jour_fixe() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2023, 11, 14).unwrap()
    }

    /// Un dossier de photos jetable et son dossier d'album, pas encore créé.
    fn dossier(nom: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir()
            .join(format!("colophon-recomposition-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let photos = base.join("photos");
        fs::create_dir_all(&photos).unwrap();
        (photos, base.join("album"))
    }

    /// Une photo imprimable de `w × h` pixels, en grandes cases de couleur
    /// tirées de `graine` (des dHash éloignés : aucun doublon), datée à
    /// l'heure `graine` pour que le livre ait un ordre.
    fn photo(path: &Path, graine: u32, w: u32, h: u32) {
        let mut rng = graine.wrapping_mul(2654435761).wrapping_add(97);
        let mut cases = [[0u8; 3]; 64];
        for c in cases.iter_mut() {
            for ch in c.iter_mut() {
                rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
                *ch = (rng >> 24) as u8;
            }
        }
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            let cx = (x * 8 / w).min(7);
            let cy = (y * 8 / h).min(7);
            image::Rgb(cases[(cy * 8 + cx) as usize])
        });
        img.save(path).unwrap();
        let f = fs::File::options().write(true).open(path).unwrap();
        let t = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(1_700_000_000 + u64::from(graine) * 3600);
        f.set_modified(t).unwrap();
    }

    /// Un petit voyage aux trois formes : des paysages, des portraits, des
    /// carrés, assez pour plusieurs planches de photographies.
    fn voyage(photos: &Path) {
        for i in 0..12u32 {
            let (w, h) = match i % 3 {
                0 => (2400, 1600),
                1 => (1600, 2400),
                _ => (2000, 2000),
            };
            photo(&photos.join(format!("p-{i:02}.jpg")), i, w, h);
        }
    }

    /// Le dossier d'un album recopié entier, vignettes comprises : les
    /// planches gardées ne repassent pas par l'analyse, donc leurs vignettes
    /// doivent être là.
    fn copier(de: &Path, vers: &Path) {
        fs::create_dir_all(vers).unwrap();
        for e in fs::read_dir(de).unwrap() {
            let e = e.unwrap();
            let cible = vers.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copier(&e.path(), &cible);
            } else {
                fs::copy(e.path(), cible).unwrap();
            }
        }
    }

    fn composer(photos: &Path, album: &Path) -> Album {
        let opts = BuildOptions { aujourdhui: Some(jour_fixe()), ..Default::default() };
        build_album(photos, album, opts).expect("un album").album
    }

    fn lire(dir: &Path) -> Album {
        serde_json::from_str(&fs::read_to_string(dir.join("album.json")).unwrap()).unwrap()
    }

    /// Verrouille une planche de photos sur deux, et l'écrit : les planches
    /// que la recomposition doit garder.
    fn verrouiller(dir: &Path) -> Vec<usize> {
        let mut album = lire(dir);
        let mut verrouillees = Vec::new();
        let mut n = 0;
        for (i, s) in album.spreads.iter_mut().enumerate() {
            if s.slots.is_empty() {
                continue;
            }
            if n % 2 == 0 {
                s.locked = true;
                verrouillees.push(i);
            }
            n += 1;
        }
        crate::build::write_album_json(dir, &album).unwrap();
        verrouillees
    }

    /// Pose sur une planche de photos non verrouillée un gabarit de même
    /// capacité, apte au format de l'album et trahi au format `vers`, et la
    /// marque éditée. Rend son indice.
    fn editer_un_gabarit_trahi_en(dir: &Path, vers: Size) -> usize {
        let mut album = lire(dir);
        let tailles = bascule::tailles_du_dossier(dir, &album).unwrap();
        let ici = crate::pdf::geometry(&album);
        let mut ailleurs = album.clone();
        ailleurs.trim_mm = vers;
        let la_bas = crate::pdf::geometry(&ailleurs);
        for (i, s) in album.spreads.iter_mut().enumerate() {
            if s.slots.is_empty() || s.pinned() {
                continue;
            }
            let aspects: Vec<f64> = s
                .slots
                .iter()
                .map(|sl| {
                    let (w, h) = tailles[&sl.src];
                    f64::from(w) / f64::from(h)
                })
                .collect();
            let trahi = crate::gabarit::compatibles(&aspects, &ici).into_iter().find(|nom| {
                crate::gabarit::spec(nom).is_some_and(|sp| {
                    sp.capacite == aspects.len()
                        && crate::gabarit::trahison(sp, &aspects, &la_bas)
                            > crate::audit::ASPECT_BETRAYAL
                })
            });
            if let Some(nom) = trahi {
                s.template = nom.to_string();
                s.edited = true;
                crate::build::write_album_json(dir, &album).unwrap();
                return i;
            }
        }
        panic!("aucune planche du jeu ne peut porter un gabarit trahi en {vers:?}");
    }

    /// Les options que `recompose_album` assemblait dans la coquille avant
    /// cette session, recopiées ligne pour ligne : la référence du « au bit
    /// près ».
    fn options_d_avant(album: &Album) -> BuildOptions {
        let root = PathBuf::from(&album.root);
        let mut pinned = Vec::new();
        let mut last: Option<chrono::NaiveDateTime> = None;
        for spread in &album.spreads {
            let t = spread
                .slots
                .first()
                .map(|sl| crate::meta::read(&root.join(&sl.src)).taken)
                .or(last);
            if spread.pinned() {
                pinned.push((spread.clone(), t));
            }
            last = t;
        }
        let target = album.spreads.len().saturating_sub(pinned.len()).max(4);
        BuildOptions {
            title: Some(album.title.clone()),
            spreads: target,
            trim: album.trim_mm,
            pinned,
            cover: album.cover.clone(),
            densite: album.densite,
            colophon: album.spreads.iter().any(|s| s.template == crate::colophon::TEMPLATE),
            garde: album.spreads.iter().any(|s| s.template == crate::garde::TEMPLATE),
            variantes: Vec::new(),
            reglages: album.reglages.clone(),
            police: album.police.clone(),
            aujourdhui: Some(jour_fixe()),
            langue: crate::langue::Langue::Fr,
            ..Default::default()
        }
    }

    /// Sans paramètres, la recomposition rend l'`album.json` d'avant cette
    /// session, à l'octet : même format, même rythme, mêmes planches gardées.
    #[test]
    fn sans_parametres_le_meme_album_json_qu_avant() {
        let (photos, a) = dossier("sans-a");
        voyage(&photos);
        composer(&photos, &a);
        verrouiller(&a);
        // Le même album dans un second dossier, pour recomposer par les deux
        // chemins sans que l'un lise ce que l'autre a écrit.
        let b = a.with_file_name("album-b");
        copier(&a, &b);
        let avant = lire(&a);

        build_album(&photos, &a, options_d_avant(&avant)).expect("l'ancien chemin");
        recomposer(
            &b,
            Recomposition { aujourdhui: Some(jour_fixe()), ..Default::default() },
        )
        .expect("le nouveau chemin");

        assert_eq!(
            fs::read_to_string(a.join("album.json")).unwrap(),
            fs::read_to_string(b.join("album.json")).unwrap(),
            "sans format ni rythme, la recomposition d'avant au bit près"
        );
    }

    /// Le rythme donné remplace celui de l'album, et il est écrit.
    #[test]
    fn le_rythme_donne_remplace_celui_de_l_album() {
        let (photos, dir) = dossier("rythme");
        voyage(&photos);
        assert_eq!(composer(&photos, &dir).densite, Densite::Equilibree);
        let r = recomposer(
            &dir,
            Recomposition {
                densite: Some(Densite::Aeree),
                aujourdhui: Some(jour_fixe()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.album.densite, Densite::Aeree);
        assert_eq!(lire(&dir).densite, Densite::Aeree);
    }

    /// 21 × 21 vers A4 paysage : `trim_mm` change, chaque planche verrouillée
    /// est gardée — mêmes photos, mêmes recadrages, même légende — avec le
    /// gabarit que la bascule lui donne à ce format, et le prévol n'y voit
    /// aucun défaut de géométrie qu'il ne voyait pas avant.
    #[test]
    fn carre_21_vers_a4_paysage_garde_les_verrouillees_repliees() {
        let (photos, dir) = dossier("a4");
        voyage(&photos);
        composer(&photos, &dir);
        let verrouillees = verrouiller(&dir);
        assert!(verrouillees.len() >= 2, "un jeu qui verrouille quelque chose");
        let a4 = Size { w: 297.0, h: 210.0 };
        // Une planche éditée à la main : un gabarit que le sélecteur offre au
        // carré et que l'A4 paysage trahit. C'est elle que la bascule doit
        // replier ; sans elle, le Composer ne pose que des gabarits qui
        // tiennent dans les deux formats et le test ne prouverait rien.
        let editee = editer_un_gabarit_trahi_en(&dir, a4);
        let avant = lire(&dir);
        assert_eq!(planches_gardees(&avant), verrouillees.len() + 1);

        // Ce que la bascule seule ferait des planches gardées : la référence
        // du repli, calculée par le module qui en a la définition.
        let tailles = bascule::tailles_du_dossier(&dir, &avant).unwrap();
        let profil = PrinterProfile::par_id("cloudprinter").unwrap();
        let (basculee, bilan) = bascule::bascule(&avant, a4, &tailles, profil);
        assert!(
            bilan.epinglees_touchees.contains(&(editee + 1)),
            "la planche éditée doit se replier : {:?}",
            bilan.replis
        );
        let attendues: Vec<model::Spread> =
            basculee.spreads.iter().filter(|s| s.pinned()).cloned().collect();
        assert_ne!(
            basculee.spreads[editee].template, avant.spreads[editee].template,
            "repliée"
        );

        let geometrie = ["fond_perdu", "imposition", "objet_coupe", "objet_pli", "format_sans_produit"];
        let codes = |dir: &Path| -> std::collections::BTreeSet<&'static str> {
            crate::prevol::prevol_album(dir, profil)
                .unwrap()
                .defauts
                .iter()
                .map(|d| d.regle)
                .filter(|r| geometrie.contains(r))
                .collect()
        };
        let defauts_avant = codes(&dir);

        let r = recomposer(
            &dir,
            Recomposition {
                format: Some(a4),
                aujourdhui: Some(jour_fixe()),
                ..Default::default()
            },
        )
        .unwrap();

        let cotes = |s: Size| (s.w, s.h);
        assert_eq!(cotes(r.album.trim_mm), cotes(a4));
        assert_eq!(cotes(lire(&dir).trim_mm), cotes(a4), "le format est écrit");
        // Comparées comme ensembles : une planche gardée revient à sa date,
        // et la recomposition peut la poser à un autre rang.
        let json = |s: &model::Spread| serde_json::to_string(s).unwrap();
        let mut gardees: Vec<String> =
            r.album.spreads.iter().filter(|s| s.pinned()).map(json).collect();
        let mut attendues: Vec<String> = attendues.iter().map(json).collect();
        gardees.sort();
        attendues.sort();
        assert_eq!(gardees, attendues, "les planches gardées sont celles que la bascule rend");
        let nouveaux: Vec<_> = codes(&dir).difference(&defauts_avant).copied().collect();
        assert!(nouveaux.is_empty(), "défauts de géométrie nouveaux : {nouveaux:?}");
    }

    /// Un format donné égal au format courant ne change rien de ce qu'un
    /// format absent aurait rendu : la bascule sur place est l'identité.
    #[test]
    fn le_format_courant_donne_vaut_le_format_absent() {
        let (photos, a) = dossier("courant");
        voyage(&photos);
        composer(&photos, &a);
        verrouiller(&a);
        let b = a.with_file_name("album-b");
        copier(&a, &b);
        let trim = lire(&a).trim_mm;
        recomposer(&a, Recomposition { aujourdhui: Some(jour_fixe()), ..Default::default() })
            .unwrap();
        recomposer(
            &b,
            Recomposition {
                format: Some(trim),
                aujourdhui: Some(jour_fixe()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(a.join("album.json")).unwrap(),
            fs::read_to_string(b.join("album.json")).unwrap()
        );
    }
}
