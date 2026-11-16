use super::*;
use crate::voyage::{Etape, Lieu, Photo, Provenance};
use image::{Rgb, RgbImage};

// ---- de quoi fabriquer, en petit, un cas réaliste -------------------------------

/// Un tirage déterministe : pas de dépendance, pas d'horloge.
struct Tirage(u32);
impl Tirage {
    fn suivant(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Une « scène » : des taches de couleur sur un dégradé, propre à sa graine.
/// Deux graines donnent deux images sans rapport.
fn scene(graine: u32, l: u32, h: u32) -> RgbImage {
    let mut t = Tirage(graine.wrapping_mul(2_654_435_761).wrapping_add(7));
    let taches: Vec<(f32, f32, f32, [f32; 3])> = (0..7)
        .map(|_| (t.suivant(), t.suivant(), 0.08 + 0.25 * t.suivant(), [t.suivant() * 255.0, t.suivant() * 255.0, t.suivant() * 255.0]))
        .collect();
    let (gx, gy) = (t.suivant(), t.suivant());
    RgbImage::from_fn(l, h, |x, y| {
        let (u, v) = (x as f32 / l as f32, y as f32 / h as f32);
        let mut c = [60.0 + 80.0 * (gx * u + gy * v); 3];
        for (cx, cy, r, couleur) in &taches {
            let d = ((u - cx).powi(2) + (v - cy).powi(2)) / (r * r);
            let w = (-d).exp();
            for k in 0..3 {
                c[k] = c[k] * (1.0 - w) + couleur[k] * w;
            }
        }
        Rgb([c[0] as u8, c[1] as u8, c[2] as u8])
    })
}

fn jpeg(img: &RgbImage, qualite: u8) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, qualite).encode_image(img).unwrap();
    out
}

/// Le segment APP1 d'un EXIF minimal : orientation, heure de prise de vue
/// (`AAAA:MM:JJ HH:MM:SS`, locale sans fuseau) et, si on le donne, `+02:00`.
fn exif(heure: Option<&str>, orientation: u16, decalage: Option<&str>) -> Vec<u8> {
    let mut sous: Vec<(u16, u16, u32, Vec<u8>)> = Vec::new(); // tag, type, nombre, octets
    if let Some(h) = heure {
        let mut o = h.as_bytes().to_vec();
        o.push(0);
        sous.push((0x9003, 2, o.len() as u32, o));
    }
    if let Some(d) = decalage {
        let mut o = d.as_bytes().to_vec();
        o.push(0);
        sous.push((0x9011, 2, o.len() as u32, o));
    }
    let ifd0_fin = 8 + 2 + 2 * 12 + 4;
    let exif_ifd = ifd0_fin;
    let mut donnees_debut = exif_ifd + 2 + sous.len() as u32 * 12 + 4;
    let mut tiff = b"II*\0".to_vec();
    tiff.extend(8u32.to_le_bytes());
    tiff.extend(2u16.to_le_bytes());
    for (tag, typ, n, val) in [(0x0112u16, 3u16, 1u32, orientation as u32), (0x8769, 4, 1, exif_ifd)] {
        tiff.extend(tag.to_le_bytes());
        tiff.extend(typ.to_le_bytes());
        tiff.extend(n.to_le_bytes());
        tiff.extend(val.to_le_bytes());
    }
    tiff.extend(0u32.to_le_bytes());
    tiff.extend((sous.len() as u16).to_le_bytes());
    let mut donnees: Vec<u8> = Vec::new();
    for (tag, typ, n, octets) in &sous {
        tiff.extend(tag.to_le_bytes());
        tiff.extend(typ.to_le_bytes());
        tiff.extend(n.to_le_bytes());
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

/// Un JPEG d'« appareil » : l'image, son EXIF.
fn original_jpeg(img: &RgbImage, heure: Option<&str>, orientation: u16, decalage: Option<&str>) -> Vec<u8> {
    let corps = jpeg(img, 92);
    let mut out = vec![0xFF, 0xD8];
    out.extend(exif(heure, orientation, decalage));
    out.extend(&corps[2..]);
    out
}

/// Ce que ferait la source : réduite, recompressée, sans EXIF.
fn vignette_jpeg(img: &RgbImage, l: u32) -> (Vec<u8>, u32, u32) {
    let h = (img.height() as f32 * l as f32 / img.width() as f32).round() as u32;
    let petite = image::imageops::resize(img, l, h, FilterType::Triangle);
    (jpeg(&petite, 60), l, h)
}

fn dossier(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("booklibre-originaux-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(dossier_des_vignettes(&d.join("voyage")).unwrap());
    std::fs::create_dir_all(d.join("voyage")).unwrap();
    std::fs::create_dir_all(d.join("originaux")).unwrap();
    d
}

fn instant(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn etape(nom: &str, debut: &str, fuseau: &str, photos: &[(&str, u32, u32)]) -> Etape {
    Etape {
        nom: nom.into(),
        recit: None,
        debut: instant(debut),
        fin: None,
        fuseau: fuseau.into(),
        lieu: Lieu { lat: 0.0, lon: 0.0, localite: None, pays: None, code_pays: None },
        meteo: None,
        photos: photos.iter().map(|(f, l, h)| Photo { fichier: (*f).into(), largeur: *l, hauteur: *h }).collect(),
    }
}

fn voyage(etapes: Vec<Etape>) -> Voyage {
    Voyage {
        version: crate::voyage::VERSION,
        titre: "Essai".into(),
        resume: None,
        debut: None,
        fin: None,
        fuseau: None,
        distance_km: None,
        pays: vec![],
        etapes,
        segments: vec![],
        trace: vec![],
        videos_ignorees: 0,
        couverture: None,
        provenance: Provenance { source: "polarsteps-lien".into(), reference: None, format: None },
    }
}

fn pas_de_progres() -> (impl Fn(usize, usize) + Sync, impl Fn() -> bool + Sync) {
    (|_: usize, _: usize| {}, || false)
}

/// Un original en mémoire, sans disque : ce que l'appariement lit.
fn en_memoire(nom: &str, img: &RgbImage, heure: Option<&str>) -> Original {
    let decode = image::load_from_memory(&jpeg(img, 92)).unwrap();
    Original {
        chemin: PathBuf::from(nom),
        largeur: img.width(),
        hauteur: img.height(),
        heure: heure.map(|h| NaiveDateTime::parse_from_str(h, "%Y-%m-%d %H:%M:%S").unwrap()),
        decalage_s: None,
        empreinte: empreinte(&decode),
    }
}

fn vignette_en_memoire(fichier: &str, img: &RgbImage, debut: &str, fuseau: &str) -> Vignette {
    let (octets, l, h) = vignette_jpeg(img, 60);
    let decode = image::load_from_memory(&octets).unwrap();
    Vignette {
        fichier: fichier.into(),
        largeur: l,
        hauteur: h,
        empreinte: Some(empreinte(&decode)),
        fenetre: fenetre_de(instant(debut), None),
        fuseau: fuseau.into(),
    }
}

/// Le banc du cas fabriqué (hors dépôt) : pour chaque vignette, la distance au
/// bon original et au meilleur des mauvais. À la main :
/// `COLOPHON_ORIGINAUX_CAS=<cas> cargo test -p colophon-core --release banc_du_cas -- --ignored --nocapture`
#[test]
#[ignore]
fn banc_du_cas_fabrique() {
    let Ok(cas) = std::env::var("COLOPHON_ORIGINAUX_CAS") else { return };
    let cas = PathBuf::from(cas);
    let verite: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(cas.join("verite.json")).unwrap()).unwrap();
    let voyage = crate::voyage::lire(&cas.join("voyage")).unwrap();
    let (index, _) = indexer(&cas.join("originaux"), &[], &|_, _| {}, &|| false).unwrap();
    let vignettes = vignettes_du_voyage(&cas.join("voyage"), &voyage, &[]);
    let mut lignes = Vec::new();
    for (v, _) in &vignettes {
        let t = &verite[&v.fichier];
        let vrai = t["original"].as_str().or(t["vrai"].as_str());
        let emp = v.empreinte.as_ref().unwrap();
        let mut bon = f32::NAN;
        let mut meilleur_autre = f32::INFINITY;
        for o in &index {
            if !ratio_compatible(v.largeur, v.hauteur, o.largeur, o.hauteur) {
                continue;
            }
            let d = distance(emp, &o.empreinte);
            if Some(o.chemin.file_name().unwrap().to_str().unwrap()) == vrai {
                bon = d;
            } else {
                meilleur_autre = meilleur_autre.min(d);
            }
        }
        lignes.push(format!("{}\t{}\t{:.3}\t{:.3}", t["piege"].as_str().unwrap(), v.fichier, bon, meilleur_autre));
    }
    lignes.sort();
    println!("piege\tfichier\tbon\tmeilleur_autre\n{}", lignes.join("\n"));
}

// ---- l'appariement, sans disque -------------------------------------------------

const GRAND: (u32, u32) = (240, 320);

#[test]
fn un_original_net_est_apparie_par_l_heure() {
    let (a, b) = (scene(1, GRAND.0, GRAND.1), scene(2, GRAND.0, GRAND.1));
    let originaux = [en_memoire("a.jpg", &a, Some("2024-06-10 10:00:00")), en_memoire("b.jpg", &b, Some("2024-06-10 11:00:00"))];
    let v = vignette_en_memoire("e001.jpg", &b, "2024-06-10T09:00:00Z", "UTC");
    match apparier_une(&v, &originaux) {
        Verdict::Sur(m) => assert_eq!((m.original, m.methode), (1, Methode::Heure)),
        autre => panic!("{autre:?}"),
    }
}

/// La rafale : deux originaux qui se ressemblent. Aucun n'est sûr, donc rien
/// ne se remplace, même quand l'un des deux est le bon.
#[test]
fn une_rafale_n_apparie_jamais_rien() {
    let a = scene(3, GRAND.0, GRAND.1);
    let mut jumeau = a.clone();
    for (i, p) in jumeau.pixels_mut().enumerate().step_by(97) {
        p.0[0] = p.0[0].saturating_add((i % 5) as u8); // du bruit de capteur
    }
    let originaux = [en_memoire("a1.jpg", &a, Some("2024-06-10 10:00:00")), en_memoire("a2.jpg", &jumeau, Some("2024-06-10 10:00:01"))];
    let v = vignette_en_memoire("e001.jpg", &a, "2024-06-10T09:00:00Z", "UTC");
    assert_eq!(apparier_une(&v, &originaux), Verdict::Refus(Raison::Ambigu));
    // Et sans l'heure, le dossier entier a les mêmes deux candidats.
    let sans_heure = [en_memoire("a1.jpg", &a, None), en_memoire("a2.jpg", &jumeau, None)];
    assert_eq!(apparier_une(&v, &sans_heure), Verdict::Refus(Raison::Ambigu));
}

/// L'heure de la source est UTC avec un fuseau d'étape ; celle de l'EXIF est
/// locale, sans fuseau. À Los Angeles (UTC−7 en juin), 10 h locales sont
/// 17 h UTC : lues comme de l'UTC, elles tombent avant la fenêtre de l'étape.
#[test]
fn le_decalage_de_fuseau_est_absorbe() {
    let a = scene(4, GRAND.0, GRAND.1);
    let autre = scene(5, GRAND.0, GRAND.1);
    let originaux = [en_memoire("a.jpg", &a, Some("2024-06-10 10:00:00")), en_memoire("x.jpg", &autre, Some("2024-06-10 10:30:00"))];
    let v = vignette_en_memoire("e001.jpg", &a, "2024-06-10T17:00:00Z", "America/Los_Angeles");
    match apparier_une(&v, &originaux) {
        Verdict::Sur(m) => assert_eq!((m.original, m.methode), (0, Methode::Heure)),
        autre => panic!("{autre:?}"),
    }
    // L'instant lui-même, sans passer par l'appariement.
    assert_eq!(instant_de(&originaux[0], "America/Los_Angeles"), Some(instant("2024-06-10T17:00:00Z")));
    assert_eq!(instant_de(&originaux[0], "Asia/Tokyo"), Some(instant("2024-06-10T01:00:00Z")));
    // Un fuseau inconnu se lit en UTC, il ne fait pas échouer.
    assert_eq!(instant_de(&originaux[0], "Mars/Olympus"), Some(instant("2024-06-10T10:00:00Z")));
    // Le décalage que le boîtier a écrit l'emporte sur le fuseau de l'étape.
    let mut ecrit = originaux[0].clone();
    ecrit.decalage_s = Some(2 * 3600);
    assert_eq!(instant_de(&ecrit, "America/Los_Angeles"), Some(instant("2024-06-10T08:00:00Z")));
}

#[test]
fn hors_de_la_fenetre_l_image_seule_est_plus_severe() {
    let a = scene(6, GRAND.0, GRAND.1);
    let originaux = [en_memoire("a.jpg", &a, Some("2024-06-20 10:00:00")), en_memoire("x.jpg", &scene(7, GRAND.0, GRAND.1), Some("2024-06-20 10:00:05"))];
    let v = vignette_en_memoire("e001.jpg", &a, "2024-06-10T09:00:00Z", "UTC");
    // Dix jours plus tôt : la fenêtre ne couvre rien, l'image tranche.
    match apparier_une(&v, &originaux) {
        Verdict::Sur(m) => assert_eq!((m.original, m.methode), (0, Methode::Image)),
        autre => panic!("{autre:?}"),
    }
    // Entre les deux seuils : sûre dans la fenêtre, refusée hors d'elle.
    let proche = {
        let b = scene(8, GRAND.0, GRAND.1);
        let mut m = a.clone();
        for (p, q) in m.pixels_mut().zip(b.pixels()) {
            for k in 0..3 {
                p.0[k] = (p.0[k] as f32 * 0.90 + q.0[k] as f32 * 0.10) as u8;
            }
        }
        m
    };
    let v = vignette_en_memoire("e001.jpg", &proche, "2024-06-10T09:00:00Z", "UTC");
    let d = distance(v.empreinte.as_ref().unwrap(), &en_memoire("a.jpg", &a, None).empreinte);
    assert!(d > SEUIL_IMAGE && d < SEUIL_HEURE, "le montage ne vise pas l'entre-deux : {d}");
    let dedans = [en_memoire("a.jpg", &a, Some("2024-06-10 10:00:00"))];
    assert!(matches!(apparier_une(&v, &dedans), Verdict::Sur(m) if m.methode == Methode::Heure));
    let dehors = [en_memoire("a.jpg", &a, Some("2024-06-20 10:00:00"))];
    assert_eq!(apparier_une(&v, &dehors), Verdict::Refus(Raison::SansOriginal));
}

#[test]
fn une_vignette_sans_original_est_refusee() {
    let originaux = [en_memoire("a.jpg", &scene(9, GRAND.0, GRAND.1), Some("2024-06-10 10:00:00"))];
    let v = vignette_en_memoire("e001.jpg", &scene(10, GRAND.0, GRAND.1), "2024-06-10T09:00:00Z", "UTC");
    assert_eq!(apparier_une(&v, &originaux), Verdict::Refus(Raison::SansOriginal));
}

/// Une photo recadrée en carré n'a plus le ratio de l'original : même image,
/// pas le même cadre, rien n'est remplacé.
#[test]
fn un_ratio_incompatible_n_apparie_pas() {
    let a = scene(11, GRAND.0, GRAND.1);
    let originaux = [en_memoire("a.jpg", &a, Some("2024-06-10 10:00:00"))];
    let carre = image::imageops::crop_imm(&a, 0, 40, 240, 240).to_image();
    let v = vignette_en_memoire("e001.jpg", &carre, "2024-06-10T09:00:00Z", "UTC");
    assert_eq!(apparier_une(&v, &originaux), Verdict::Refus(Raison::SansOriginal));
    assert!(ratio_compatible(960, 1280, 3024, 4032) && ratio_compatible(1705, 960, 1920, 1080));
    assert!(!ratio_compatible(960, 1280, 1000, 1000));
}

/// Un recadrage qui garde le ratio garde le cadre approximativement : son
/// image s'éloigne de l'original au-delà du seuil.
#[test]
fn un_recadrage_de_meme_ratio_n_apparie_pas() {
    let a = scene(12, GRAND.0, GRAND.1);
    let originaux = [en_memoire("a.jpg", &a, Some("2024-06-10 10:00:00"))];
    let centre = image::imageops::crop_imm(&a, 24, 32, 192, 256).to_image(); // 80 %
    let v = vignette_en_memoire("e001.jpg", &centre, "2024-06-10T09:00:00Z", "UTC");
    assert_eq!(apparier_une(&v, &originaux), Verdict::Refus(Raison::SansOriginal));
}

#[test]
fn deux_vignettes_pour_un_meme_original_ne_remplacent_ni_l_une_ni_l_autre() {
    let a = scene(13, GRAND.0, GRAND.1);
    let originaux = [en_memoire("a.jpg", &a, Some("2024-06-10 10:00:00"))];
    let v1 = vignette_en_memoire("e001-p01.jpg", &a, "2024-06-10T09:00:00Z", "UTC");
    let v2 = vignette_en_memoire("e002-p01.jpg", &a, "2024-06-10T09:00:00Z", "UTC");
    assert_eq!(apparier(&[v1, v2], &originaux), vec![Verdict::Refus(Raison::Conflit); 2]);
}

#[test]
fn un_original_pas_plus_grand_ou_pas_jpeg_ne_remplace_rien() {
    let a = scene(14, GRAND.0, GRAND.1);
    let v = vignette_en_memoire("e001.jpg", &a, "2024-06-10T09:00:00Z", "UTC");
    let petit = en_memoire("a.jpg", &image::imageops::resize(&a, v.largeur, v.hauteur, FilterType::Triangle), Some("2024-06-10 10:00:00"));
    assert_eq!(apparier(std::slice::from_ref(&v), &[petit]), vec![Verdict::Refus(Raison::PasPlusGrand)]);
    let heic = en_memoire("a.heic", &a, Some("2024-06-10 10:00:00"));
    assert_eq!(apparier(std::slice::from_ref(&v), &[heic]), vec![Verdict::Refus(Raison::Format)]);
}

#[test]
fn une_image_plate_ne_ressemble_a_rien() {
    let noir = RgbImage::from_pixel(240, 320, Rgb([0, 0, 0]));
    let originaux = [en_memoire("n.jpg", &noir, Some("2024-06-10 10:00:00"))];
    let v = vignette_en_memoire("e001.jpg", &noir, "2024-06-10T09:00:00Z", "UTC");
    assert_eq!(apparier_une(&v, &originaux), Verdict::Refus(Raison::SansOriginal));
}

#[test]
fn la_fenetre_d_une_etape_sans_fin_dure_un_jour_plus_les_marges() {
    let (de, a) = fenetre_de(instant("2024-06-10T12:00:00Z"), None);
    assert_eq!((de, a), (instant("2024-06-10T06:00:00Z"), instant("2024-06-11T18:00:00Z")));
    let (_, a) = fenetre_de(instant("2024-06-10T12:00:00Z"), Some(instant("2024-06-15T00:00:00Z")));
    assert_eq!(a, instant("2024-06-15T06:00:00Z"));
}

// ---- le remplacement, sur un vrai dossier ----------------------------------------

struct Cas {
    racine: PathBuf,
    vignette_b: Vec<u8>,
    original_b: Vec<u8>,
}

/// Trois photos, trois étapes : A (portrait de l'appareil, tag 6, couché dans
/// le fichier), B (une rafale B1/B2 : jamais remplacée), C (sans original).
/// Les originaux sont en plus : D (une photo que personne n'a postée).
fn cas(nom: &str) -> Cas {
    let racine = dossier(nom);
    let v = racine.join("voyage");
    let o = racine.join("originaux");
    let (a, b, c, d) = (scene(21, GRAND.0, GRAND.1), scene(22, GRAND.0, GRAND.1), scene(23, GRAND.0, GRAND.1), scene(24, GRAND.0, GRAND.1));
    // A : l'appareil l'a couchée (320 × 240) avec le tag 6 ; la page la verra debout.
    let couchee = image::imageops::rotate270(&a);
    std::fs::write(o.join("IMG_A.jpg"), original_jpeg(&couchee, Some("2024:06:10 10:00:00"), 6, None)).unwrap();
    let original_b = original_jpeg(&b, Some("2024:06:11 15:00:00"), 1, None);
    std::fs::write(o.join("IMG_B.jpg"), &original_b).unwrap();
    std::fs::write(o.join("IMG_D.jpg"), original_jpeg(&d, Some("2024:06:12 09:00:00"), 1, None)).unwrap();
    let mut b2 = b.clone();
    b2.pixels_mut().step_by(131).for_each(|p| p.0[1] = p.0[1].saturating_add(3));
    std::fs::write(o.join("IMG_B2.jpg"), original_jpeg(&b2, Some("2024:06:11 15:00:01"), 1, None)).unwrap();
    // Les vignettes : sans EXIF, redressées, 120 px de large.
    let (va, la, ha) = vignette_jpeg(&a, 120);
    let (vb, lb, hb) = vignette_jpeg(&b, 120);
    let (vc, lc, hc) = vignette_jpeg(&c, 120);
    std::fs::write(v.join("e001-p01-aaaa.jpg"), va).unwrap();
    std::fs::write(v.join("e002-p01-bbbb.jpg"), &vb).unwrap();
    std::fs::write(v.join("e003-p01-cccc.jpg"), vc).unwrap();
    crate::voyage::ecrire(
        &v,
        &voyage(vec![
            // Paris en juin : UTC+2.
            etape("Un", "2024-06-10T08:00:00Z", "Europe/Paris", &[("e001-p01-aaaa.jpg", la, ha)]),
            etape("Deux", "2024-06-11T13:00:00Z", "Europe/Paris", &[("e002-p01-bbbb.jpg", lb, hb)]),
            etape("Trois", "2024-06-12T08:00:00Z", "Europe/Paris", &[("e003-p01-cccc.jpg", lc, hc)]),
        ]),
    )
    .unwrap();
    Cas { racine, vignette_b: vb, original_b }
}

#[test]
fn le_remplacement_garde_le_nom_la_place_et_ne_efface_rien() {
    let c = cas("remplacement");
    let v = c.racine.join("voyage");
    // Un album qui pointe sur ces trois photos : légende, recadrage.
    let album = r#"{"slots":[{"src":"e001-p01-aaaa.jpg","focal":[0.3,0.4],"zoom":1.5,"caption":"Le départ"}]}"#;
    std::fs::write(c.racine.join("album.json"), album).unwrap();
    let vignette_a = std::fs::read(v.join("e001-p01-aaaa.jpg")).unwrap();
    let (p, a) = pas_de_progres();
    let r = remplacer(&v, &c.racine.join("originaux"), None, &p, &a).unwrap();

    // A est remplacée (heure + fuseau + rotation) ; B est une rafale ; C n'a pas d'original.
    assert_eq!(r.remplacees, 1, "{r:?}");
    let raisons: Vec<_> = r.refus.iter().map(|x| (x.fichier.as_str(), x.raison)).collect();
    assert_eq!(raisons, [("e002-p01-bbbb.jpg", Raison::Ambigu), ("e003-p01-cccc.jpg", Raison::SansOriginal)]);

    // Le nom est resté, le contenu est celui de l'original (l'octet), l'album n'a pas bougé.
    let nom = v.join("e001-p01-aaaa.jpg");
    assert_eq!(std::fs::read(&nom).unwrap(), std::fs::read(c.racine.join("originaux/IMG_A.jpg")).unwrap());
    assert_eq!(std::fs::read_to_string(c.racine.join("album.json")).unwrap(), album);
    // Les perdantes sont intactes à l'octet.
    assert_eq!(std::fs::read(v.join("e002-p01-bbbb.jpg")).unwrap(), c.vignette_b);
    // Les originaux n'ont pas été touchés.
    assert_eq!(std::fs::read(c.racine.join("originaux/IMG_B.jpg")).unwrap(), c.original_b);

    // Rien n'est effacé : la vignette est rangée à côté, et dite.
    let rangee = dossier_des_vignettes(&v).unwrap().join("e001-p01-aaaa.jpg");
    assert_eq!(std::fs::read(&rangee).unwrap(), vignette_a);
    let corr = lire_correspondance(&v).unwrap();
    assert_eq!(corr.remplacements.len(), 1);
    let m = &corr.remplacements[0];
    assert_eq!((m.fichier.as_str(), m.methode), ("e001-p01-aaaa.jpg", Methode::Heure));
    assert!(m.original.ends_with("IMG_A.jpg") && m.vignette.ends_with("-vignettes/e001-p01-aaaa.jpg"));
    // Dimensions orientées : la page la voit debout (240 × 320), pas couchée.
    assert_eq!(m.apres, [240, 320]);
    let lu = crate::voyage::lire(&v).unwrap();
    assert_eq!((lu.etapes[0].photos[0].largeur, lu.etapes[0].photos[0].hauteur), (240, 320));
    assert!(r.sous_seuil_avant >= r.sous_seuil_apres);
}

#[test]
fn rien_n_est_refait_au_second_passage_et_tout_se_retablit() {
    let c = cas("retablir");
    let v = c.racine.join("voyage");
    let avant_voyage = std::fs::read_to_string(v.join("voyage.json")).unwrap();
    let vignette_a = std::fs::read(v.join("e001-p01-aaaa.jpg")).unwrap();
    let (p, a) = pas_de_progres();
    let un = remplacer(&v, &c.racine.join("originaux"), None, &p, &a).unwrap();
    let deux = remplacer(&v, &c.racine.join("originaux"), None, &p, &a).unwrap();
    assert_eq!((un.remplacees, deux.remplacees, deux.deja), (1, 0, 1));
    assert_eq!(lire_correspondance(&v).unwrap().remplacements.len(), 1);

    let r = retablir(&v, None).unwrap();
    assert_eq!(r.vignettes_rendues, 1);
    assert_eq!(std::fs::read(v.join("e001-p01-aaaa.jpg")).unwrap(), vignette_a);
    assert_eq!(std::fs::read_to_string(v.join("voyage.json")).unwrap(), avant_voyage);
    assert!(lire_correspondance(&v).is_none());
    // Rien à rétablir deux fois.
    assert!(retablir(&v, None).is_err());
}

#[test]
fn un_nom_de_correspondance_qui_sort_du_dossier_est_refuse() {
    let c = cas("nom-refuse");
    let v = c.racine.join("voyage");
    let (p, a) = pas_de_progres();
    remplacer(&v, &c.racine.join("originaux"), None, &p, &a).unwrap();
    let t = std::fs::read_to_string(v.join(FICHIER)).unwrap().replace("\"e001-p01-aaaa.jpg\",\n      \"vignette\"", "\"../evil.jpg\",\n      \"vignette\"");
    std::fs::write(v.join(FICHIER), t).unwrap();
    assert!(retablir(&v, None).is_err());
}

/// Le dossier d'originaux peut contenir le voyage (« mes Images ») : ses
/// vignettes ne sont pas des originaux, et ne s'apparient pas à elles-mêmes.
#[test]
fn le_dossier_du_voyage_n_est_pas_un_original() {
    let c = cas("exclusion");
    let (p, a) = pas_de_progres();
    let (tous, _) = indexer(&c.racine, &[], &p, &a).unwrap();
    let (sans, _) = indexer(&c.racine, &[c.racine.join("voyage")], &p, &a).unwrap();
    assert!(tous.len() > sans.len() && sans.len() == 4);
    assert!(sans.iter().all(|o| o.chemin.starts_with(c.racine.join("originaux"))));
}

#[test]
fn les_vignettes_en_cache_de_l_album_suivent_le_remplacement() {
    let c = cas("cache");
    let v = c.racine.join("voyage");
    let album = c.racine.join("album");
    std::fs::create_dir_all(&album).unwrap();
    // Un cache d'album tel que la composition l'a laissé : thumbs.json pointe la vignette.
    let cache = thumb::ThumbCache::new(&album).unwrap();
    let avant = cache.get(&v.join("e001-p01-aaaa.jpg"), 1).unwrap();
    let nom_avant = cache.path_for(&v.join("e001-p01-aaaa.jpg")).file_name().unwrap().to_string_lossy().to_string();
    std::fs::write(album.join("thumbs.json"), format!("{{\"e001-p01-aaaa.jpg\":\"{nom_avant}\"}}")).unwrap();
    assert_eq!(avant.width(), 120);
    let (p, a) = pas_de_progres();
    remplacer(&v, &c.racine.join("originaux"), Some(&album), &p, &a).unwrap();
    let index: BTreeMap<String, String> = serde_json::from_str(&std::fs::read_to_string(album.join("thumbs.json")).unwrap()).unwrap();
    let nom_apres = &index["e001-p01-aaaa.jpg"];
    assert_ne!(*nom_apres, nom_avant);
    // La nouvelle vignette est celle de l'original (240 × 320 : sous 1600, sa taille exacte).
    let neuve = image::open(album.join(".cache/thumbs").join(nom_apres)).unwrap();
    assert_eq!((neuve.width(), neuve.height()), (240, 320));
}

/// Les photos que la composition avait laissées à la réserve faute de
/// définition sont comptées, jamais ramenées d'office.
#[test]
fn les_photos_de_la_reserve_qui_peuvent_revenir_sont_comptees() {
    let c = cas("reserve");
    let v = c.racine.join("voyage");
    let album = c.racine.join("album");
    std::fs::create_dir_all(&album).unwrap();
    std::fs::write(
        album.join("curation.json"),
        r#"[{"src":"e001-p01-aaaa.jpg","reason":"hors_budget","focal":[0.5,0.42]},{"src":"e003-p01-cccc.jpg","reason":"definition","focal":[0.5,0.42]}]"#,
    )
    .unwrap();
    let (p, a) = pas_de_progres();
    let r = remplacer(&v, &c.racine.join("originaux"), Some(&album), &p, &a).unwrap();
    // A revient (remplacée), C reste (sans original) : un seul chiffre vrai.
    assert_eq!(r.revenues_possibles, 1);
    assert_eq!(std::fs::read_to_string(album.join("curation.json")).unwrap().matches("reason").count(), 2);
}

/// Un jumeau écarté reste écarté : l'original ne change pas ce choix.
#[test]
fn un_jumeau_ecarte_ne_compte_pas_parmi_ceux_qui_peuvent_revenir() {
    let c = cas("jumeau");
    let v = c.racine.join("voyage");
    let album = c.racine.join("album");
    std::fs::create_dir_all(&album).unwrap();
    std::fs::write(album.join("curation.json"), r#"[{"src":"e001-p01-aaaa.jpg","reason":"jumeau","focal":[0.5,0.42]}]"#).unwrap();
    let (p, a) = pas_de_progres();
    let r = remplacer(&v, &c.racine.join("originaux"), Some(&album), &p, &a).unwrap();
    assert_eq!((r.remplacees, r.revenues_possibles), (1, 0));
}

#[test]
fn un_original_png_sur_est_nomme_pas_remplace() {
    let c = cas("png");
    let v = c.racine.join("voyage");
    // A n'existe plus qu'en PNG.
    std::fs::remove_file(c.racine.join("originaux/IMG_A.jpg")).unwrap();
    let a = scene(21, GRAND.0, GRAND.1);
    std::fs::write(c.racine.join("originaux/IMG_A.png"), {
        let mut o = std::io::Cursor::new(Vec::new());
        a.write_to(&mut o, image::ImageFormat::Png).unwrap();
        o.into_inner()
    })
    .unwrap();
    let avant = std::fs::read(v.join("e001-p01-aaaa.jpg")).unwrap();
    let (p, an) = pas_de_progres();
    let r = remplacer(&v, &c.racine.join("originaux"), None, &p, &an).unwrap();
    // Sans EXIF le PNG est hors fenêtre : c'est l'image seule qui le trouve, puis le format qui le refuse.
    assert_eq!(r.remplacees, 0);
    assert!(r.refus.iter().any(|x| x.fichier == "e001-p01-aaaa.jpg" && x.raison == Raison::Format), "{r:?}");
    assert_eq!(std::fs::read(v.join("e001-p01-aaaa.jpg")).unwrap(), avant);
}

#[test]
fn l_annulation_ne_laisse_rien_a_moitie() {
    let c = cas("annule");
    let v = c.racine.join("voyage");
    let avant = std::fs::read_to_string(v.join("voyage.json")).unwrap();
    let r = remplacer(&v, &c.racine.join("originaux"), None, &|_, _| {}, &|| true);
    assert!(r.is_err());
    assert_eq!(std::fs::read_to_string(v.join("voyage.json")).unwrap(), avant);
    assert!(lire_correspondance(&v).is_none());
}
