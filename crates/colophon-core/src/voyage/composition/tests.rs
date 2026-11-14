//! La composition d'un voyage synthétique (P-s2). Aucune photo réelle, aucune
//! donnée d'un tiers : des images générées, des étapes inventées.

use super::*;
use crate::build::{build_album, BuildOptions};
use crate::model::Album;
use crate::voyage::{ecrire, Etape, Lieu, Meteo, Photo, Provenance, Voyage, VERSION};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

fn jour() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 12).unwrap()
}

fn dossier(nom: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("booklibre-voyage-compo-{nom}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let photos = base.join("photos");
    fs::create_dir_all(&photos).unwrap();
    (photos, base.join("album"))
}

/// Une photo de `w × h` pixels en grandes cases de couleur tirées de `graine`
/// (des empreintes éloignées : aucun doublon).
fn photo(chemin: &Path, graine: u32, w: u32, h: u32) {
    let mut rng = graine.wrapping_mul(2654435761).wrapping_add(97);
    let mut cases = [[0u8; 3]; 64];
    for c in cases.iter_mut() {
        for ch in c.iter_mut() {
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            *ch = (rng >> 24) as u8;
        }
    }
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb(cases[((y * 8 / h).min(7) * 8 + (x * 8 / w).min(7)) as usize])
    });
    img.save(chemin).unwrap();
}

fn instant(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

const FORMES: [(u32, u32); 5] = [(3000, 2000), (2000, 3000), (2400, 2400), (3000, 2000), (2000, 3000)];

/// Les étapes d'un voyage inventé : `(nom, nombre de photos, récit)`.
fn etapes_inventees() -> Vec<(&'static str, usize, Option<String>)> {
    let long = "Une longue journée de marche dans la vallée, sous un ciel changeant. ".repeat(40);
    vec![
        ("Lyon", 7, Some("Départ de la gare, valises lourdes et café tiède.".into())),
        ("Turin", 6, None),
        ("Gênes 🌊", 8, Some("Le port, la focaccia 🍞 et le vent 🌬️ sur la jetée. 🇮🇹".into())),
        ("Nice", 6, Some(long.clone())),
        ("Le train de nuit", 0, Some(format!("{0} {0} {0}", "Dix heures de rails, un thé, un livre, et le jour qui se lève sur la mer. Le contrôleur passe deux fois, la lumière baisse, une femme s'endort contre la vitre et un enfant compte les tunnels à voix basse. À l'aube, la mer apparaît d'un coup, plate et grise, puis argentée, et tout le wagon se réveille en même temps pour la regarder."))),
        ("Marseille", 7, Some("Un récit assez court.".into())),
        ("Une étape vide", 0, None),
        ("Arles", 6, Some("x".repeat(10).to_string())),
        ("Escale", 0, Some("Une halte, deux mots.".into())),
        ("Halte floue", 2, Some("Deux vignettes, aucune planche pour elles.".into())),
        ("Avignon", 9, None),
        ("Orange", 6, Some("Les gradins vides à l'aube, et personne pour nous dire où aller.".into())),
        ("Valence", 7, Some("Des heures dans une salle d'attente, à regarder les horaires changer. ".repeat(110))),
        ("Retour à Lyon", 6, None),
    ]
}

/// Écrit le voyage inventé : les photos, puis `voyage.json`. Les récits sont
/// ceux de `etapes_inventees()`.
fn ecrire_voyage(photos: &Path) -> Voyage {
    let mut graine = 1;
    let mut etapes = Vec::new();
    for (i, (nom, n, recit)) in etapes_inventees().into_iter().enumerate() {
        let mut ph = Vec::new();
        for j in 0..n {
            let (w, h) = if nom == "Halte floue" { (300, 200) } else { FORMES[(i + j) % FORMES.len()] };
            let fichier = format!("e{:03}-p{:02}.jpg", i + 1, j + 1);
            photo(&photos.join(&fichier), graine, w, h);
            graine += 1;
            ph.push(Photo { fichier, largeur: w, hauteur: h });
        }
        etapes.push(Etape {
            nom: nom.into(),
            recit,
            debut: instant(&format!("2025-06-{:02}T09:30:00Z", i + 1)),
            fin: None,
            fuseau: "Europe/Paris".into(),
            lieu: Lieu { lat: 45.0, lon: 4.0, localite: Some(nom.replace(" 🌊", "")), pays: Some("France".into()), code_pays: Some("FR".into()) },
            meteo: Some(Meteo { condition: Some("rain".into()), temperature: Some(18.0) }),
            photos: ph,
        });
    }
    let v = Voyage {
        version: VERSION,
        titre: "Un tour du sud 🌍".into(),
        resume: None,
        debut: Some(instant("2025-06-01T09:30:00Z")),
        fin: Some(instant("2025-06-12T09:30:00Z")),
        fuseau: Some("Europe/Paris".into()),
        distance_km: Some(1200.0),
        pays: vec!["FR".into()],
        etapes,
        segments: vec![],
        trace: vec![],
        videos_ignorees: 0,
        couverture: None,
        provenance: Provenance { source: "test".into(), reference: Some("1".into()), format: None },
    };
    ecrire(photos, &v).unwrap();
    v
}


/// Ce que les tests lisent d'une composition, sans la refaire : le voyage
/// inventé se compose **une fois** par processus (80 photos de 3000 px, lent en
/// debug) ; un test qui écrit dans l'album travaille sur une copie.
#[derive(Clone)]
struct Rapport {
    album: Album,
    voyage: Option<Bilan>,
    chapters: usize,
    album_json: PathBuf,
}

struct Partage {
    photos: PathBuf,
    out: PathBuf,
    voyage: Voyage,
    rapport: Rapport,
}

fn partage() -> &'static Partage {
    static P: std::sync::OnceLock<Partage> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        let (photos, out) = dossier("partage");
        let voyage = ecrire_voyage(&photos);
        let r = build_album(&photos, &out, options()).expect("un voyage se compose");
        let rapport = Rapport { album: r.album, voyage: r.voyage, chapters: r.chapters, album_json: r.album_json };
        Partage { photos, out, voyage, rapport }
    })
}

/// Une copie du dossier d'album partagé, pour un test qui l'écrit. Les photos
/// et `voyage.json` restent les mêmes : personne ne les modifie.
fn copie_de_l_album(nom: &str) -> PathBuf {
    fn copier(de: &Path, vers: &Path) {
        fs::create_dir_all(vers).unwrap();
        for e in fs::read_dir(de).unwrap() {
            let e = e.unwrap();
            let cible = vers.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copier(&e.path(), &cible);
            } else {
                fs::copy(e.path(), &cible).unwrap();
            }
        }
    }
    let vers = std::env::temp_dir().join(format!("booklibre-voyage-copie-{nom}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&vers);
    copier(&partage().out, &vers);
    vers
}

fn options() -> BuildOptions {
    BuildOptions { aujourdhui: Some(jour()), ..Default::default() }
}

fn noms_de_chapitres(album: &Album) -> Vec<String> {
    album.spreads.iter().filter_map(|s| s.caption.clone()).collect()
}

#[test]
fn douze_etapes_composent_un_album_a_zero_defaut() {
    let c = partage();
    let out = c.out.clone();
    let r = c.rapport.clone();
    let audit = crate::audit::audit(&out).unwrap();
    let c = &audit.compteurs;
    let comptes: Vec<(&str, usize)> = [
        ("visage_coupe", c.visage_coupe.count),
        ("orientation_trahie", c.orientation_trahie.count),
        ("doublon_planche", c.doublon_planche.count),
        ("sous_resolution", c.sous_resolution.count),
        ("chapitre_orphelin", c.chapitre_orphelin.count),
        ("ouverture_faible", c.ouverture_faible.count),
        ("rythme_plat", c.rythme_plat.count),
        ("legende_manquante", c.legende_manquante.count),
        ("legende_sur_photo", c.legende_sur_photo.count),
        ("repetition_gabarit", c.repetition_gabarit.count),
        ("objet_hors_marge", c.objet_hors_marge.count),
        ("objet_deborde", c.objet_deborde.count),
        ("ornement_sur_photo", c.ornement_sur_photo.count),
        ("caractere_absent", c.caractere_absent.count),
    ]
    .into_iter()
    .collect();
    // Le récit de « Valence » dépasse ses pages, c'est voulu par ce jeu et
    // dit au bilan : le linter le voit aussi, en `objet_deborde` (qui avertit).
    let autres: Vec<_> = comptes
        .iter()
        .filter(|(k, n)| *n > 0 && *k != "objet_deborde" && *k != "objet_hors_marge")
        .collect();
    // Les deux avertissements des objets ne portent que sur ce récit-là.
    for f in c.objet_deborde.details.iter().chain(&c.objet_hors_marge.details) {
        let chapitre = r.album.spreads[..f.planche].iter().rev().find_map(|s| s.caption.clone());
        assert_eq!(chapitre.as_deref(), Some("Valence"), "{f:?}");
    }
    assert!(audit.ok && autres.is_empty(), "défauts : {autres:?}\n{:#?}", c.objet_deborde.details);
    assert_eq!(r.chapters, 14);
}

#[test]
fn un_chapitre_est_une_etape_dans_l_ordre_du_voyage() {
    let c = partage();
    let v = c.voyage.clone();
    let r = c.rapport.clone();
    // Les titres sont les noms d'étape, sans émoji, dans l'ordre ; l'étape
    // sans photo ni récit n'en a pas.
    let attendus: Vec<String> = v
        .etapes
        .iter()
        .filter(|e| !["Une étape vide", "Le train de nuit", "Escale", "Halte floue"].contains(&e.nom.as_str()))
        .map(|e| sans_emojis(&e.nom).0)
        .collect();
    assert_eq!(noms_de_chapitres(&r.album), attendus);
    // Les photos d'une étape sont toutes dans les planches de cette étape.
    let mut etape_en_cours = None;
    for s in &r.album.spreads {
        if let Some(c) = &s.caption {
            etape_en_cours = v.etapes.iter().position(|e| sans_emojis(&e.nom).0 == *c);
        }
        for sl in &s.slots {
            let i = etape_en_cours.expect("une photo avant tout chapitre");
            assert!(
                v.etapes[i].photos.iter().any(|p| p.fichier == sl.src),
                "{} n'est pas une photo de l'étape {}",
                sl.src,
                v.etapes[i].nom
            );
        }
    }
}

#[test]
fn le_recit_trop_long_est_nomme_au_bilan_et_reste_entier() {
    let c = partage();
    let v = c.voyage.clone();
    let r = c.rapport.clone();
    let b = r.voyage.expect("un voyage a son bilan");
    assert_eq!(b.recits_debordants, vec!["Valence".to_string()]);
    assert!(b.phrases().iter().any(|p| p.contains("Valence") && p.contains("dépasse")));
    // Jamais coupé : toutes les lignes du récit sont encore dans les blocs.
    let attendu = v.etapes.iter().find(|e| e.nom == "Valence").unwrap().recit.clone().unwrap();
    let mots = |t: &str| t.split_whitespace().count();
    let dans_les_blocs: usize = r
        .album
        .spreads
        .iter()
        .filter(|s| s.caption.as_deref() == Some("Valence"))
        .flat_map(|s| s.objets.iter())
        .map(|o| match &o.contenu {
            // Le titre (18 pt) et la mention (9 pt) ne sont pas du récit.
            crate::model::Contenu::Texte { texte, taille_pt, .. } if (*taille_pt - TAILLE_RECIT_PT).abs() < 1e-9 => mots(texte),
            _ => 0,
        })
        .sum();
    assert_eq!(dans_les_blocs, mots(&attendu));
    // Le long de « Nice » a sa page de récit mais tient dans ses deux pages.
    assert!(!b.recits_debordants.contains(&"Nice".to_string()));
    assert!(b.recits_en_page >= 2);
}

#[test]
fn les_emojis_sont_comptes_et_ne_s_impriment_pas() {
    let c = partage();
    let (photos, out) = (c.photos.clone(), c.out.clone());
    let r = c.rapport.clone();
    let b = r.voyage.unwrap();
    // 🌊 dans un nom, 🍞 🌬️ 🇮🇹 dans un récit : quatre ; le titre n'est pas compté ici.
    assert_eq!(b.emojis_non_imprimes, 4);
    assert!(b.phrases().iter().any(|p| p.contains("4 émojis non imprimés")));
    let json = fs::read_to_string(&r.album_json).unwrap();
    assert!(!json.chars().any(est_pictogramme), "un émoji est resté dans album.json");
    // Le titre de la couverture aussi, et le PDF n'écrit aucun « ? » de
    // substitution : le linter ne voit aucun caractère absent.
    assert_eq!(r.album.cover.as_ref().unwrap().title, "Un tour du sud");
    let audit = crate::audit::audit(&out).unwrap();
    assert_eq!(audit.compteurs.caractere_absent.count, 0);
    // Et voyage.json les garde.
    let gardes = fs::read_to_string(photos.join("voyage.json")).unwrap();
    assert!(gardes.contains('🌊') && gardes.contains('🍞'));
}

#[test]
fn une_etape_sans_photo_est_une_page_de_texte_ou_rien() {
    let c = partage();
    let r = c.rapport.clone();
    // Sans photo, le texte du train est en tête de l'ouverture de « Marseille » :
    // aucune planche de texte à lui, aucune page blanche.
    let train = r
        .album
        .spreads
        .iter()
        .find(|s| textes(s).iter().any(|(t, _)| t == "Le train de nuit"))
        .expect("l'étape sans photo a son texte");
    assert!(!noms_de_chapitres(&r.album).contains(&"Le train de nuit".to_string()));
    assert_eq!(train.caption.as_deref(), Some("Marseille"));
    // Sa mention (date locale, lieu) est sur la page, dérivée de l'étape.
    let textes: Vec<String> = train
        .objets
        .iter()
        .filter_map(|o| match &o.contenu {
            crate::model::Contenu::Texte { texte, .. } => Some(texte.clone()),
            _ => None,
        })
        .collect();
    assert!(textes.iter().any(|t| t == "Le train de nuit, 5 juin 2025"), "{textes:?}");
    assert!(textes.iter().any(|t| t.contains("Dix heures de rails")));
    // Ni photo ni récit : rien, et le bilan le compte.
    assert!(!noms_de_chapitres(&r.album).contains(&"Une étape vide".to_string()));
    let b = r.voyage.unwrap();
    assert_eq!((b.etapes_vides, b.pages_de_texte, b.etapes_groupees), (1, 0, 3));
}

#[test]
fn la_mention_est_la_date_locale_et_le_lieu_de_l_etape() {
    let c = partage();
    let r = c.rapport.clone();
    // « Lyon » : sa date, dans son fuseau, et son lieu, dans le texte de
    // l'ouverture (récit court) : rien ne se superpose sous les photos.
    let lyon = r.album.spreads.iter().find(|s| s.caption.as_deref() == Some("Lyon")).unwrap();
    assert!(textes(lyon).iter().any(|(t, _)| t == "Lyon, 1 juin 2025"), "{:?}", textes(lyon));
    assert!(lyon.slots.iter().all(|sl| sl.mention.is_none()));
    // « Turin » n'a pas de récit : sa mention est sous sa première photo.
    let turin = r.album.spreads.iter().flat_map(|s| &s.slots).find(|sl| sl.mention.as_deref() == Some("Turin, 2 juin 2025"));
    assert!(turin.is_some());
    // Une seule mention par étape.
    let n = r.album.spreads.iter().flat_map(|s| &s.slots).filter(|sl| sl.mention.is_some()).count();
    assert!(n <= 11, "{n} mentions pour 11 étapes à photos");
}

#[test]
fn la_date_est_celle_du_fuseau_de_l_etape() {
    let mut e = ecrire_voyage_etape();
    // 23 h UTC le 1er juin : déjà le 2 à Tokyo, encore le 1er à Paris.
    e.debut = instant("2025-06-01T23:30:00Z");
    e.fuseau = "Asia/Tokyo".into();
    assert_eq!(jour_local(&e), NaiveDate::from_ymd_opt(2025, 6, 2));
    e.fuseau = "Europe/Paris".into();
    assert_eq!(jour_local(&e), NaiveDate::from_ymd_opt(2025, 6, 2)); // 01 h 30 le 2 en été
    e.fuseau = "America/Chicago".into();
    assert_eq!(jour_local(&e), NaiveDate::from_ymd_opt(2025, 6, 1));
    e.fuseau = "Nulle/Part".into();
    assert_eq!(jour_local(&e), None);
}

fn ecrire_voyage_etape() -> Etape {
    Etape {
        nom: "x".into(),
        recit: None,
        debut: instant("2025-06-01T09:30:00Z"),
        fin: None,
        fuseau: "Europe/Paris".into(),
        lieu: Lieu { lat: 0.0, lon: 0.0, localite: None, pays: None, code_pays: None },
        meteo: None,
        photos: vec![],
    }
}

#[test]
fn sans_emojis_compte_les_suites_et_laisse_le_reste_intact() {
    assert_eq!(sans_emojis("Bonjour à tous, ça va ? → oui ©"), ("Bonjour à tous, ça va ? → oui ©".into(), 0));
    assert_eq!(sans_emojis("Plage 🏖️ et soleil ☀️"), ("Plage et soleil".into(), 2));
    assert_eq!(sans_emojis("En famille 👨‍👩‍👧 !"), ("En famille !".into(), 1));
    assert_eq!(sans_emojis("🇫🇷🇮🇹 voyage"), ("voyage".into(), 2));
    assert_eq!(sans_emojis("Salut 👋🏽"), ("Salut".into(), 1));
    assert_eq!(sans_emojis("ligne un 🎒\nligne deux"), ("ligne un\nligne deux".into(), 1));
}

#[test]
fn un_dossier_sans_voyage_json_ou_avec_un_voyage_illisible_se_compose_pareil() {
    let (photos, out) = dossier("identite");
    for i in 0..30u32 {
        let (w, h) = FORMES[i as usize % 5];
        let chemin = photos.join(format!("p{i:02}.jpg"));
        photo(&chemin, i, w, h);
        let f = fs::File::options().write(true).open(&chemin).unwrap();
        f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000 + u64::from(i) * 7200)).unwrap();
    }
    build_album(&photos, &out, options()).unwrap();
    let sans = fs::read(out.join("album.json")).unwrap();
    // Un voyage.json trop récent : le dossier se compose comme un dossier
    // ordinaire, au même octet, et le bilan dit pourquoi.
    fs::write(photos.join("voyage.json"), r#"{"version": 99}"#).unwrap();
    let out2 = out.with_file_name("album2");
    let r = build_album(&photos, &out2, options()).unwrap();
    let avec = fs::read(out2.join("album.json")).unwrap();
    // Seul le chemin du dossier d'album diffère ; `root` est le même.
    assert_eq!(sans, avec, "un voyage.json refusé ne doit pas changer l'album");
    let b = r.voyage.expect("le refus se dit");
    assert!(b.refus.as_deref().is_some_and(|m| m.contains("version 99")));
    assert!(b.phrases()[0].contains("dossier ordinaire"));
}

#[test]
fn la_recomposition_garde_les_pages_touchees_a_leur_etape() {
    let c = partage();
    let out = copie_de_l_album("recompo");
    let v = c.voyage.clone();
    let r = c.rapport.clone();
    // Une main touche la deuxième planche de photos de « Marseille » (5e étape à photos).
    let debut = r.album.spreads.iter().position(|s| s.caption.as_deref() == Some("Marseille")).unwrap();
    let suivante = r.album.spreads[debut + 1..].iter().position(|s| s.caption.is_some()).map(|i| debut + 1 + i).unwrap();
    assert!(suivante > debut + 1, "Marseille a au moins deux planches");
    let touchee = debut + 1;
    let mut album = r.album.clone();
    album.spreads[touchee].edited = true;
    album.spreads[touchee].slots[0].caption = Some("Notre table au port".into());
    let sources: Vec<String> = album.spreads[touchee].slots.iter().map(|s| s.src.clone()).collect();
    fs::write(out.join("album.json"), serde_json::to_string_pretty(&album).unwrap()).unwrap();

    let apres = crate::recomposition::recomposer(
        &out,
        crate::recomposition::Recomposition { aujourdhui: Some(jour()), ..Default::default() },
    )
    .unwrap()
    .album;
    // La planche touchée est là, telle quelle, une seule fois.
    let trouvees: Vec<usize> = apres
        .spreads
        .iter()
        .enumerate()
        .filter(|(_, s)| s.slots.first().map(|sl| sl.caption.as_deref()) == Some(Some("Notre table au port")))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(trouvees.len(), 1, "la planche touchée est gardée une fois");
    let a = &apres.spreads[trouvees[0]];
    assert!(a.edited);
    assert_eq!(a.slots.iter().map(|s| s.src.clone()).collect::<Vec<_>>(), sources);
    // Chaque photo est posée au plus une fois dans le livre recomposé.
    let mut vues = std::collections::HashSet::new();
    for sl in apres.spreads.iter().flat_map(|s| &s.slots) {
        assert!(vues.insert(sl.src.clone()), "{} posée deux fois", sl.src);
    }
    // Et elle reste à son étape : entre le titre de Marseille et celui d'Arles.
    let marseille = apres.spreads.iter().position(|s| s.caption.as_deref() == Some("Marseille")).unwrap();
    let arles = apres.spreads.iter().position(|s| s.caption.as_deref() == Some("Arles")).unwrap();
    assert!(marseille < trouvees[0] && trouvees[0] < arles, "{marseille} < {} < {arles}", trouvees[0]);
    let _ = v;
    // Les étapes se retrouvent dans l'ordre, une fois chacune.
    assert_eq!(noms_de_chapitres(&apres).iter().filter(|c| *c == "Marseille").count(), 1);
}

#[test]
fn rien_est_un_choix_qu_une_recomposition_ne_defait_pas() {
    let c = partage();
    let out = copie_de_l_album("rien");
    let r = c.rapport.clone();
    // À la première composition : date + lieu, posés par défaut.
    assert!(r.album.legendes.is_some());
    assert!(r.album.spreads.iter().flat_map(|s| &s.slots).any(|sl| sl.mention.is_some()));
    // L'utilisateur choisit « Rien » : plus de réglage, plus de mention.
    let mut album = r.album.clone();
    album.legendes = None;
    for sl in album.spreads.iter_mut().flat_map(|s| s.slots.iter_mut()) {
        sl.mention = None;
    }
    fs::write(out.join("album.json"), serde_json::to_string_pretty(&album).unwrap()).unwrap();
    let apres = crate::recomposition::recomposer(
        &out,
        crate::recomposition::Recomposition { aujourdhui: Some(jour()), ..Default::default() },
    )
    .unwrap()
    .album;
    assert!(apres.legendes.is_none(), "la machine a remis un réglage que l'utilisateur avait ôté");
    assert!(apres.spreads.iter().flat_map(|s| &s.slots).all(|sl| sl.mention.is_none()));
}

fn orphelins(dir: &Path) -> usize {
    crate::audit::audit(dir).unwrap().compteurs.chapitre_orphelin.count
}

/// Ajoute à la fin de l'album une planche de texte légendée, un chapitre d'une
/// planche sans photo.
fn ajouter_page_de_texte(dir: &Path) {
    let mut album: Album = serde_json::from_str(&fs::read_to_string(dir.join("album.json")).unwrap()).unwrap();
    let mut s = crate::model::Spread::vide();
    s.caption = Some("Ajout".into());
    album.spreads.push(s);
    fs::write(dir.join("album.json"), serde_json::to_string_pretty(&album).unwrap()).unwrap();
}

#[test]
fn chapitre_orphelin_ne_passe_sur_les_pages_de_texte_que_dans_un_voyage() {
    // Un album ordinaire : la page de texte légendée compte, comme avant.
    let (photos, out) = dossier("orphelin-ordinaire");
    for i in 0..30u32 {
        let (w, h) = FORMES[i as usize % 5];
        photo(&photos.join(format!("p{i:02}.jpg")), i, w, h);
    }
    build_album(&photos, &out, options()).unwrap();
    let avant = orphelins(&out);
    ajouter_page_de_texte(&out);
    assert_eq!(orphelins(&out), avant + 1, "album ordinaire : le compteur est inchangé");
    // Un album de voyage : une page de texte est voulue, elle ne compte pas.
    let out = copie_de_l_album("orphelin");
    let avant = orphelins(&out);
    ajouter_page_de_texte(&out);
    assert_eq!(orphelins(&out), avant, "album de voyage : la page de texte n'est pas un orphelin");
}

fn textes(s: &crate::model::Spread) -> Vec<(String, f64)> {
    s.objets
        .iter()
        .filter_map(|o| match &o.contenu {
            crate::model::Contenu::Texte { texte, taille_pt, .. } => Some((texte.clone(), *taille_pt)),
            _ => None,
        })
        .collect()
}

#[test]
fn le_nom_de_l_etape_ouvre_son_texte_en_titre() {
    let r = partage().rapport.clone();
    let titre = crate::garde::TITRE_PT;
    // Une planche de récit (Nice), une ouverture en tête de page (Lyon) et le
    // texte d'une étape sans photo (Le train de nuit) portent le nom en grand.
    for nom in ["Nice", "Lyon", "Marseille", "Le train de nuit"] {
        assert!(
            r.album.spreads.iter().any(|s| textes(s).iter().any(|(t, pt)| t == nom && (*pt - titre).abs() < 1e-9)),
            "{nom} n'a pas son titre en {titre} pt"
        );
    }
}

#[test]
fn un_recit_court_partage_sa_page_avec_des_photos() {
    let r = partage().rapport.clone();
    let s = r.album.spreads.iter().find(|s| s.caption.as_deref() == Some("Lyon")).unwrap();
    // Le bloc titre + mention + récit est en tête de la page de gauche, les
    // photos dessous et à droite : une bande haute, aucune page blanche.
    assert!(s.template.starts_with("g_") && s.template.contains("_t"), "{}", s.template);
    assert!(s.slots.len() >= 2);
    assert!(textes(s).iter().any(|(t, _)| t.contains("Départ de la gare")));
}

#[test]
fn une_etape_sans_photo_au_texte_court_se_regroupe_avec_la_suivante() {
    let r = partage().rapport.clone();
    let b = r.voyage.clone().unwrap();
    // « Escale » n'a pas sa planche ni son chapitre dans les légendes : son
    // texte est en tête de l'ouverture d'« Avignon », qui est un chapitre.
    assert!(!noms_de_chapitres(&r.album).contains(&"Escale".to_string()));
    let avignon = r.album.spreads.iter().find(|s| s.caption.as_deref() == Some("Avignon")).unwrap();
    assert!(!avignon.slots.is_empty());
    let t = textes(avignon);
    assert!(t.iter().any(|(x, _)| x == "Escale") && t.iter().any(|(x, _)| x.contains("Une halte")), "{t:?}");
    // « Halte floue » a des photos que aucune planche ne prend : même sort.
    assert!(!noms_de_chapitres(&r.album).contains(&"Halte floue".to_string()));
    assert!(t.iter().any(|(x, _)| x == "Halte floue"), "{t:?}");
    assert_eq!(b.etapes_groupees, 3);
    assert!(b.phrases().iter().any(|p| p.contains("regroupées")));
}

/// Un voyage en trois étapes : deux aux vignettes trop petites pour le format
/// et au récit court, puis une aux vraies photos.
fn ecrire_voyage_de_vignettes(photos: &Path) {
    let plan: [(&str, Option<&str>, usize, u32, u32); 3] = [
        ("Départ", Some("Les valises sont prêtes."), 2, 800, 800),
        ("Gare", Some("Un café et le quai."), 1, 800, 800),
        ("Arrivée", None, 6, 3000, 2000),
    ];
    let mut etapes = Vec::new();
    let mut graine = 500;
    for (i, (nom, recit, n, w, h)) in plan.into_iter().enumerate() {
        let mut ph = Vec::new();
        for j in 0..n {
            let fichier = format!("v{:03}-p{:02}.jpg", i + 1, j + 1);
            photo(&photos.join(&fichier), graine, w, h);
            graine += 1;
            ph.push(Photo { fichier, largeur: w, hauteur: h });
        }
        etapes.push(Etape {
            nom: nom.into(),
            recit: recit.map(String::from),
            debut: instant(&format!("2025-06-{:02}T09:30:00Z", i + 1)),
            fin: None,
            fuseau: "Europe/Paris".into(),
            lieu: Lieu { lat: 45.0, lon: 4.0, localite: Some(nom.into()), pays: Some("France".into()), code_pays: Some("FR".into()) },
            meteo: None,
            photos: ph,
        });
    }
    let v = Voyage {
        version: VERSION,
        titre: "Des vignettes".into(),
        resume: None,
        debut: Some(instant("2025-06-01T09:30:00Z")),
        fin: Some(instant("2025-06-03T09:30:00Z")),
        fuseau: Some("Europe/Paris".into()),
        distance_km: None,
        pays: vec!["FR".into()],
        etapes,
        segments: vec![],
        trace: vec![],
        videos_ignorees: 0,
        couverture: None,
        provenance: Provenance { source: "test".into(), reference: Some("2".into()), format: None },
    };
    ecrire(photos, &v).unwrap();
}

#[test]
fn une_etape_aux_photos_trop_petites_se_traite_comme_une_etape_sans_photo() {
    let (photos, out) = dossier("vignettes");
    ecrire_voyage_de_vignettes(&photos);
    let r = build_album(&photos, &out, options()).unwrap();
    let b = r.voyage.clone().unwrap();
    // Ni « Départ » ni « Gare » n'ont leur planche : leurs textes sont en tête
    // de l'ouverture d'« Arrivée ».
    assert_eq!(b.etapes_groupees, 2, "{b:?}");
    let noms = noms_de_chapitres(&r.album);
    assert!(!noms.contains(&"Départ".to_string()) && !noms.contains(&"Gare".to_string()), "{noms:?}");
    let arrivee = r.album.spreads.iter().find(|s| s.caption.as_deref() == Some("Arrivée")).unwrap();
    assert!(!arrivee.slots.is_empty());
    let t = textes(arrivee);
    assert!(t.iter().any(|(x, _)| x == "Départ") && t.iter().any(|(x, _)| x == "Gare"), "{t:?}");
    // Les vignettes trop petites ne s'impriment pas : la réserve les garde.
    let imprimees: Vec<&String> = r.album.spreads.iter().flat_map(|s| &s.slots).map(|sl| &sl.src).collect();
    assert!(!imprimees.iter().any(|f| f.contains("v001") || f.contains("v002")), "{imprimees:?}");
}


#[test]
fn la_mention_ne_se_superpose_pas_au_nom_du_chapitre() {
    let r = partage().rapport.clone();
    let a = &r.album;
    let g = crate::pdf::geometry(a);
    let mut vus = 0;
    for s in a.spreads.iter().filter(|s| s.caption.is_some()) {
        let rects = crate::pdf::slots_for(&s.template, s.slots.len(), &g);
        let droite = |k: usize| rects[k].x >= g.media_w / 2.0;
        let deux_cotes = (0..rects.len()).any(droite) && (0..rects.len()).any(|k| !droite(k));
        if !deux_cotes {
            continue;
        }
        for (k, sl) in s.slots.iter().enumerate() {
            if sl.mention.is_some() {
                vus += 1;
                assert!(droite(k), "{} : mention sous la page de gauche, où pend le nom du chapitre", s.template);
            }
        }
    }
    // Le jeu pose ses mentions dans le texte des ouvertures : on rejoue la
    // table des mentions par case, comme le fait l'app, sur l'album composé.
    let reglage = legendes_par_defaut(crate::langue::Langue::Fr);
    let table = crate::legende::mentions(a, &reglage, &[]);
    for s in a.spreads.iter().filter(|s| s.caption.is_some()) {
        let rects = crate::pdf::slots_for(&s.template, s.slots.len(), &g);
        let droite = |k: usize| rects[k].x >= g.media_w / 2.0;
        if !((0..rects.len()).any(droite) && (0..rects.len()).any(|k| !droite(k))) {
            continue;
        }
        for (k, _) in s.slots.iter().enumerate().filter(|(_, sl)| table.contains_key(&sl.src)) {
            assert!(droite(k), "{} : mention en case {k}, sous le nom du chapitre", s.template);
        }
    }
    let _ = vus;
}


/// Ce que chaque page d'une planche porte : `(page de gauche, page de droite)`.
fn pages_pleines(s: &crate::model::Spread, g: &crate::pdf::SpreadGeometry) -> (bool, bool) {
    let milieu = g.media_w / 2.0;
    let rects = crate::pdf::slots_for(&s.template, s.slots.len(), g);
    (
        rects.iter().any(|r| r.x < milieu) || s.objets.iter().any(|o| o.x < milieu),
        rects.iter().any(|r| r.x >= milieu) || s.objets.iter().any(|o| o.x >= milieu),
    )
}

#[test]
fn aucune_planche_de_texte_n_a_une_page_blanche_hors_la_derniere() {
    let r = partage().rapport.clone();
    let g = crate::pdf::geometry(&r.album);
    let avec_texte: Vec<usize> =
        (0..r.album.spreads.len()).filter(|&i| !r.album.spreads[i].objets.is_empty()).collect();
    assert!(avec_texte.len() >= 8, "le jeu n'a presque aucune planche de texte : {}", avec_texte.len());
    let derniere = *avec_texte.last().unwrap();
    for &i in &avec_texte {
        let s = &r.album.spreads[i];
        let (gauche, droite) = pages_pleines(s, &g);
        assert!(gauche, "planche {i} ({:?}) : page de gauche blanche", s.caption);
        assert!(droite || i == derniere, "planche {i} ({:?}, {}) : page de droite blanche", s.caption, s.template);
    }
}

#[test]
fn un_texte_a_gauche_ouvre_sur_des_photos_a_droite() {
    let r = partage().rapport.clone();
    let g = crate::pdf::geometry(&r.album);
    // Les ouvertures : un bloc de texte à gauche, des photos sur les deux
    // pages (bande haute) ou à droite seulement (page de texte).
    let ouvertures: Vec<_> = r.album.spreads.iter().filter(|s| !s.objets.is_empty() && !s.slots.is_empty()).collect();
    assert!(ouvertures.len() >= 5);
    // Un texte court est en tête de la page de gauche, photos dessous (`_t`).
    assert!(ouvertures.iter().any(|s| s.template.contains("_t")), "aucune ouverture en bande haute");
    for s in ouvertures {
        let rects = crate::pdf::slots_for(&s.template, s.slots.len(), &g);
        assert!(rects.iter().any(|r| r.x >= g.media_w / 2.0), "{} : pas de photo à droite", s.template);
        if s.template.starts_with("g_v_") {
            assert!(rects.iter().all(|r| r.x >= g.media_w / 2.0));
        } else {
            assert!(s.template.contains("_t"), "{}", s.template);
        }
    }
}
