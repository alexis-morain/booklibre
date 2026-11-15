//! Les cartes d'un album de voyage (P-s5) : où la composition les pose, et ce
//! que la recomposition en fait. Un seul voyage, composé **une fois** par
//! processus (patron de `partage()`) : le job Windows en debug est à
//! 42 minutes sur 45.

use super::*;
use crate::model::Contenu;

/// Huit étapes, une photo chacune : trois en France, deux en Italie (pas
/// assez pour une carte), trois en Grèce. Trois cartes : le voyage, la
/// France, la Grèce.
fn etapes_en_trois_pays() -> Vec<(&'static str, &'static str, f64, f64)> {
    vec![
        ("Paris", "FR", 48.86, 2.35),
        ("Lyon", "FR", 45.76, 4.84),
        ("Marseille", "FR", 43.30, 5.37),
        ("Rome", "IT", 41.90, 12.50),
        ("Naples", "IT", 40.85, 14.27),
        ("Athènes", "GR", 37.98, 23.73),
        ("Delphes", "GR", 38.48, 22.50),
        ("Corfou", "GR", 39.62, 19.92),
    ]
}

fn ecrire_voyage_en_trois_pays(photos: &Path) -> Voyage {
    let mut etapes = Vec::new();
    for (i, (nom, pays, lat, lon)) in etapes_en_trois_pays().into_iter().enumerate() {
        let (w, h) = PETITES[i % PETITES.len()];
        let fichier = format!("t{:03}-p01.jpg", i + 1);
        photo(&photos.join(&fichier), 4000 + i as u32, w, h);
        etapes.push(Etape {
            nom: nom.into(),
            recit: (i % 3 == 0).then(|| "Un récit assez court pour tenir à côté de la photo.".into()),
            debut: instant(&format!("2025-06-{:02}T09:30:00Z", i + 1)),
            fin: None,
            fuseau: "Europe/Paris".into(),
            lieu: Lieu { lat, lon, localite: Some(nom.into()), pays: None, code_pays: Some(pays.into()) },
            meteo: None,
            photos: vec![Photo { fichier, largeur: w, hauteur: h }],
        });
    }
    let v = Voyage {
        version: VERSION,
        titre: "Trois pays".into(),
        resume: None,
        debut: None,
        fin: None,
        fuseau: None,
        distance_km: None,
        pays: vec!["FR".into(), "IT".into(), "GR".into()],
        etapes,
        segments: vec![],
        trace: vec![],
        videos_ignorees: 0,
        couverture: None,
        provenance: Provenance { source: "test".into(), reference: None, format: None },
    };
    ecrire(photos, &v).unwrap();
    v
}

struct PartageCartes {
    photos: PathBuf,
    out: PathBuf,
    voyage: Voyage,
    album: Album,
    bilan: Bilan,
}

fn partage_cartes() -> &'static PartageCartes {
    static P: std::sync::OnceLock<PartageCartes> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        let (photos, out) = dossier("cartes");
        let voyage = ecrire_voyage_en_trois_pays(&photos);
        // Les options par défaut : la règle des cartes s'applique.
        let r = build_album(&photos, &out, BuildOptions { aujourdhui: Some(jour()), ..Default::default() })
            .expect("un voyage se compose");
        PartageCartes { photos, out, voyage, album: r.album, bilan: r.voyage.unwrap() }
    })
}

/// Les plages de carte d'un album, dans l'ordre du livre.
fn plages(album: &Album) -> Vec<(usize, usize)> {
    album
        .spreads
        .iter()
        .flat_map(|s| s.objets.iter())
        .filter_map(|o| match o.contenu {
            Contenu::Carte { debut, fin } => Some((debut, fin)),
            _ => None,
        })
        .collect()
}

#[test]
fn la_regle_pose_le_voyage_puis_chaque_pays_a_partir_de_trois_etapes() {
    let c = partage_cartes();
    assert_eq!(plages_de_cartes(&c.voyage), [(0, 8), (0, 3), (5, 8)]);
    assert_eq!(plages(&c.album), [(0, 8), (0, 3), (5, 8)], "dans l'ordre du livre");
    assert_eq!(c.bilan.cartes, 3);
}

#[test]
fn le_colophon_credite_natural_earth_quand_l_album_porte_une_carte() {
    let c = partage_cartes();
    assert!(c.album.colophon.as_ref().unwrap().carte);
    let page = c.album.spreads.iter().find(|s| s.template == crate::colophon::TEMPLATE).unwrap();
    assert!(page.text.as_deref().unwrap().contains("Natural Earth"), "{:?}", page.text);
}

#[test]
fn la_regle_ne_pose_rien_de_trop() {
    let mut v = partage_cartes().voyage.clone();
    // Un voyage d'une étape : pas de carte.
    let seule = Voyage { etapes: v.etapes[..1].to_vec(), ..v.clone() };
    assert!(plages_de_cartes(&seule).is_empty());
    // Un voyage qui tient en un seul pays : la carte du voyage, pas de seconde
    // qui serait la même.
    let france = Voyage { etapes: v.etapes[..3].to_vec(), ..v.clone() };
    assert_eq!(plages_de_cartes(&france), [(0, 3)]);
    // Un pays visité en deux fois : une carte par séjour de trois étapes, pas
    // une plage qui avale les étapes d'entre-deux.
    for i in [3, 4] {
        v.etapes[i].lieu.code_pays = Some("FR".into());
    }
    assert_eq!(plages_de_cartes(&v), [(0, 8), (0, 5), (5, 8)]);
    // Pas de code, ou le « 00 » de la source : aucun pays, aucune carte de pays.
    for e in &mut v.etapes {
        e.lieu.code_pays = Some("00".into());
    }
    assert_eq!(plages_de_cartes(&v), [(0, 8)]);
}

#[test]
fn une_carte_garde_le_rapport_de_son_dessin_et_tient_dans_sa_page() {
    let c = partage_cartes();
    let g = crate::pdf::geometry(&c.album);
    for (s, o) in c.album.spreads.iter().flat_map(|s| s.objets.iter().map(move |o| (s, o))) {
        let Contenu::Carte { debut, fin } = o.contenu else { continue };
        let d = crate::carte::dessiner(&c.voyage, debut..fin).unwrap();
        assert!((o.w / o.h - d.rapport()).abs() < 1e-9, "{debut}..{fin}");
        let page = [crate::pdf::page_box(false, &g), crate::pdf::page_box(true, &g)];
        assert!(
            page.iter().any(|p| o.x >= p.x - 1e-9 && o.x + o.w <= p.x + p.w + 1e-9 && o.y >= p.y - 1e-9 && o.y + o.h <= p.y + p.h + 1e-9),
            "la carte {debut}..{fin} sort de sa page : {o:?}"
        );
        assert_eq!(o.angle, 0.0);
        let _ = s;
    }
}

#[test]
fn une_planche_a_carte_n_a_pas_de_page_blanche() {
    let c = partage_cartes();
    let g = crate::pdf::geometry(&c.album);
    let milieu = g.media_w / 2.0;
    let n = c.album.spreads.len();
    let mut vues = 0;
    for (i, s) in c.album.spreads.iter().enumerate() {
        if !s.objets.iter().any(|o| matches!(o.contenu, Contenu::Carte { .. })) {
            continue;
        }
        vues += 1;
        let rects = crate::pdf::slots_for(&s.template, s.slots.len(), &g);
        let scene = crate::scene::Scene::of(s, &g);
        let gauche = scene.objects.iter().any(|o| o.rect.x + o.rect.w / 2.0 < milieu)
            || rects.iter().any(|r| r.x + r.w / 2.0 < milieu);
        let droite = scene.objects.iter().any(|o| o.rect.x + o.rect.w / 2.0 >= milieu)
            || rects.iter().any(|r| r.x + r.w / 2.0 >= milieu);
        // Seule la dernière planche du livre peut rester à moitié vide ; une
        // planche à carte l'est quand le colophon la suit, pas avant.
        assert!(gauche && (droite || i + 2 >= n), "planche {} : gauche {gauche}, droite {droite}", i + 1);
    }
    // La carte du voyage et celle de la France s'ouvrent devant la même étape :
    // elles se font face sur une planche, deux pages de carte et aucune blanche.
    assert_eq!(vues, 2, "deux planches portent les trois cartes");
}

#[test]
fn la_carte_d_un_pays_ouvre_devant_la_premiere_etape_du_pays() {
    let c = partage_cartes();
    let place = |debut: usize, fin: usize| -> usize {
        c.album
            .spreads
            .iter()
            .position(|s| s.objets.iter().any(|o| matches!(o.contenu, Contenu::Carte { debut: d, fin: f } if (d, f) == (debut, fin))))
            .unwrap()
    };
    let titre = |nom: &str| c.album.spreads.iter().position(|s| s.caption.as_deref() == Some(nom));
    // Athènes ouvre la Grèce : sa carte est avant ou sur la planche de ce titre,
    // et après celle de Naples.
    let grece = place(5, 8);
    assert!(titre("Athènes").is_none_or(|t| grece <= t), "la carte de Grèce est devant Athènes");
    assert!(titre("Naples").is_none_or(|t| t < grece), "Naples est avant la carte de Grèce");
    // La carte du voyage est la première planche à carte.
    assert!(place(0, 8) <= place(0, 3) && place(0, 3) < grece);
}

/// Une copie de l'album composé, pour le test qui l'écrit.
fn copie(nom: &str) -> PathBuf {
    let c = partage_cartes();
    let vers = std::env::temp_dir().join(format!("booklibre-cartes-copie-{nom}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&vers);
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
    copier(&c.out, &vers);
    vers
}

fn ecrire_album(dir: &Path, album: &Album) {
    fs::write(dir.join("album.json"), serde_json::to_string_pretty(album).unwrap()).unwrap();
}

fn recomposer(dir: &Path) -> Album {
    crate::recomposition::recomposer(
        dir,
        crate::recomposition::Recomposition { aujourdhui: Some(jour()), ..Default::default() },
    )
    .unwrap()
    .album
}

#[test]
fn une_recomposition_ne_remet_pas_une_carte_supprimee_par_l_utilisateur() {
    let dir = copie("supprimee");
    let mut album = partage_cartes().album.clone();
    // La main supprime la carte de la Grèce : la planche est touchée.
    let (si, oi) = album
        .spreads
        .iter()
        .enumerate()
        .find_map(|(si, s)| {
            s.objets.iter().position(|o| matches!(o.contenu, Contenu::Carte { debut: 5, .. })).map(|oi| (si, oi))
        })
        .unwrap();
    album.spreads[si].objets.remove(oi);
    album.spreads[si].edited = true;
    ecrire_album(&dir, &album);

    let apres = recomposer(&dir);
    assert_eq!(plages(&apres), [(0, 8), (0, 3)], "la carte de Grèce n'est pas revenue");
    assert!(apres.spreads.iter().any(|s| s.edited), "la planche touchée est gardée");
    // Et une seconde recomposition n'y change rien.
    assert_eq!(plages(&recomposer(&dir)), [(0, 8), (0, 3)]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn une_recomposition_garde_une_carte_deplacee_et_ne_la_double_pas() {
    let dir = copie("deplacee");
    let mut album = partage_cartes().album.clone();
    let (si, oi) = album
        .spreads
        .iter()
        .enumerate()
        .find_map(|(si, s)| {
            s.objets.iter().position(|o| matches!(o.contenu, Contenu::Carte { debut: 0, fin: 3 })).map(|oi| (si, oi))
        })
        .unwrap();
    album.spreads[si].objets[oi].x += 7.0;
    album.spreads[si].objets[oi].angle = 15.0;
    album.spreads[si].edited = true;
    let deplacee = album.spreads[si].objets[oi].clone();
    ecrire_album(&dir, &album);

    let apres = recomposer(&dir);
    assert_eq!(plages(&apres), [(0, 8), (0, 3), (5, 8)], "pas de carte en double");
    let retrouvee = apres
        .spreads
        .iter()
        .flat_map(|s| s.objets.iter())
        .find(|o| matches!(o.contenu, Contenu::Carte { debut: 0, fin: 3 }))
        .unwrap();
    assert_eq!(*retrouvee, deplacee, "la carte déplacée est telle que la main l'a laissée");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn un_album_de_voyage_d_avant_les_cartes_n_en_gagne_pas_a_la_recomposition() {
    let dir = copie("ancien");
    let mut album = partage_cartes().album.clone();
    // Un album composé par P-s2 : aucune carte, aucune planche n'en porte.
    for s in &mut album.spreads {
        s.objets.retain(|o| !matches!(o.contenu, Contenu::Carte { .. }));
    }
    ecrire_album(&dir, &album);
    assert!(plages(&recomposer(&dir)).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn un_dossier_sans_voyage_json_ne_pose_aucune_carte() {
    // Le chemin d'un dossier ordinaire n'a pas bougé : jamais un `Carte` sans
    // voyage (l'identité octet pour octet est mesurée sur les vrais dossiers,
    // `scripts/` et le rapport de la PR).
    let c = partage_cartes();
    let ordinaire = c.photos.join("..").join("ordinaire");
    let _ = fs::remove_dir_all(&ordinaire);
    fs::create_dir_all(&ordinaire).unwrap();
    for i in 0..4 {
        photo(&ordinaire.join(format!("o{i}.jpg")), 7000 + i, 2400, 1600);
    }
    let r = build_album(&ordinaire, &c.photos.join("..").join("album-ordinaire"), options()).unwrap();
    assert!(plages(&r.album).is_empty());
    assert!(r.voyage.is_none());
}
