//! Le survol d'un dossier, avant toute composition : combien de photos il
//! porte, et une poignée de miniatures pour garnir l'écran de création.
//!
//! Rien ici ne décode un original, et rien ne passe par le cache de vignettes
//! d'un album, qui n'existe pas encore. Une photo JPEG porte presque toujours,
//! dans son bloc EXIF, une miniature que l'appareil a écrite (160 × 120 en
//! général) : on ne lit que l'en-tête du fichier, jusqu'à ce bloc, et on en
//! sort ces quelques kilo-octets. Un HEIC ou un RAW passe par le décodeur du
//! système, qui rend ce que le fichier embarque sans jamais calculer de pixels
//! (`SystemDecoder::preview`). Une photo sans miniature est sautée au profit
//! de sa voisine : l'écran reste garni d'aplats plutôt que d'attendre un
//! décodage.
//!
//! L'écran doit s'afficher en moins de 300 ms sur le plus gros jeu de
//! référence. Le budget de temps est donc un argument, et le relevé s'arrête
//! là où il en est quand il est dépassé.

use image::DynamicImage;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};

/// Une miniature, orientée, réencodée en JPEG.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Miniature {
    /// Le chemin relatif au dossier, pour l'ordre et le débogage.
    pub src: String,
    pub w: u32,
    pub h: u32,
    /// `data:image/jpeg;base64,…`, prêt pour un `src` d'image : la CSP de la
    /// fenêtre accepte `data:`, et aucun fichier n'est écrit nulle part.
    pub url: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Survol {
    /// Les photos que `scan` retiendrait, toutes.
    pub photos: usize,
    /// Sous `pipeline::PETIT_DOSSIER`, la curation se limite aux rejets
    /// certains et le livre prend toutes les photos : l'estimation de l'écran
    /// en dépend.
    pub petit: bool,
    pub miniatures: Vec<Miniature>,
}

/// Au plus `n` miniatures, réparties sur le dossier dans l'ordre de `scan`.
/// Chaque place essaie sa photo, puis les suivantes, `ESSAIS` au plus, tant
/// que `limite` n'est pas dépassée.
pub fn survol(dir: &Path, n: usize, limite: Duration) -> Survol {
    let debut = Instant::now();
    let images = crate::scan::scan(dir).images;
    let photos = images.len();
    let mut miniatures = Vec::new();
    let mut prises = std::collections::BTreeSet::new();
    'places: for k in 0..n.min(photos) {
        let depart = (2 * k + 1) * photos / (2 * n);
        for i in depart..(depart + ESSAIS).min(photos) {
            if debut.elapsed() > limite {
                break 'places;
            }
            if !prises.insert(i) {
                continue;
            }
            if let Some(img) = miniature(&images[i]) {
                let src = images[i]
                    .strip_prefix(dir)
                    .unwrap_or(&images[i])
                    .to_string_lossy()
                    .to_string();
                if let Some(v) = encoder(src, &img) {
                    miniatures.push(v);
                    continue 'places;
                }
            }
        }
    }
    Survol { photos, petit: photos < crate::pipeline::PETIT_DOSSIER, miniatures }
}

/// Les places que l'écran de création demande : six cases d'aperçu et une
/// couverture, plus une pour qu'une couverture paysage se trouve plus souvent.
pub const PLACES: usize = 8;

/// Le budget du relevé, sous les 300 ms où l'écran doit s'afficher.
pub const BUDGET: Duration = Duration::from_millis(200);

/// Combien de voisines une place essaie avant de rester un aplat.
const ESSAIS: usize = 3;

/// Le plus long côté demandé au décodeur du système : assez pour une case de
/// l'aperçu, et c'est l'ordre de grandeur de ce qu'un HEIC embarque.
const COTE_SYSTEME: u32 = 320;

/// La miniature que le fichier porte, orientation appliquée, ou rien. Jamais
/// un décodage de l'original.
pub fn miniature(path: &Path) -> Option<DynamicImage> {
    if crate::heic::systeme_requis(path) {
        let d = crate::heic::system()?;
        let img = d.preview(path, Some(COTE_SYSTEME)).ok()?;
        let orientation = d.exif(path).and_then(|e| e.orientation).unwrap_or(1);
        return Some(crate::thumb::apply_orientation(img, orientation));
    }
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if ext != "jpg" && ext != "jpeg" {
        return None;
    }
    let mut lecteur = BufReader::new(File::open(path).ok()?);
    let exif = exif::Reader::new().read_from_container(&mut lecteur).ok()?;
    let entier = |tag| {
        exif.get_field(tag, exif::In::THUMBNAIL)
            .and_then(|f| f.value.get_uint(0))
            .map(|v| v as usize)
    };
    let debut = entier(exif::Tag::JPEGInterchangeFormat)?;
    let longueur = entier(exif::Tag::JPEGInterchangeFormatLength)?;
    let octets = exif.buf().get(debut..debut.checked_add(longueur)?)?;
    let img = image::load_from_memory_with_format(octets, image::ImageFormat::Jpeg).ok()?;
    let orientation = exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1);
    Some(crate::thumb::apply_orientation(img, orientation))
}

fn encoder(src: String, img: &DynamicImage) -> Option<Miniature> {
    let mut jpeg = Vec::new();
    img.to_rgb8()
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .ok()?;
    Some(Miniature {
        src,
        w: img.width(),
        h: img.height(),
        url: format!("data:image/jpeg;base64,{}", base64(&jpeg)),
    })
}

/// Base64 standard, avec bourrage (RFC 4648 § 4). Écrit ici plutôt qu'emprunté :
/// le moteur n'a pas de crate d'encodage et n'en gagne pas un pour vingt lignes.
pub fn base64(octets: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(octets.len().div_ceil(3) * 4);
    for bloc in octets.chunks(3) {
        let b = [bloc[0], *bloc.get(1).unwrap_or(&0), *bloc.get(2).unwrap_or(&0)];
        let v = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= bloc.len() {
                out.push(ALPHABET[(v >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use exif::experimental::Writer;
    use exif::{Field, In, Tag, Value};

    #[test]
    fn base64_rend_les_vecteurs_de_la_rfc_4648() {
        for (entree, attendu) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(entree.as_bytes()), attendu, "{entree:?}");
        }
        assert_eq!(base64(&[0xfb, 0xff]), "+/8=");
    }

    /// Un JPEG de `w` × `h` en miniature, dans un bloc EXIF d'orientation
    /// `orientation`, suivi de **déchets** à la place de l'image principale :
    /// si quoi que ce soit lisait l'original, la lecture échouerait.
    fn jpeg_a_miniature(w: u32, h: u32, orientation: u32) -> Vec<u8> {
        let mut petite = Vec::new();
        DynamicImage::new_rgb8(w, h)
            .write_to(&mut std::io::Cursor::new(&mut petite), image::ImageFormat::Jpeg)
            .unwrap();
        let champ = Field {
            tag: Tag::Orientation,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![orientation as u16]),
        };
        let mut tiff = std::io::Cursor::new(Vec::new());
        let mut w = Writer::new();
        w.push_field(&champ);
        w.set_jpeg(&petite, In::THUMBNAIL);
        w.write(&mut tiff, false).unwrap();
        let tiff = tiff.into_inner();
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut out = vec![0xff, 0xd8, 0xff, 0xe1];
        out.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&app1);
        out.extend_from_slice(&[0xff, 0xda, 0x00, 0x02]);
        out.extend_from_slice(b"ceci n'est pas une image principale");
        out
    }

    fn dossier(nom: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("colophon-survol-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// La miniature sort du seul bloc EXIF, tournée comme la photo : une
    /// miniature 32 × 16 d'une photo à l'orientation 6 se lit 16 × 32.
    #[test]
    fn la_miniature_sort_de_l_en_tete_et_suit_l_orientation() {
        let dir = dossier("orientation");
        let p = dir.join("a.jpg");
        std::fs::write(&p, jpeg_a_miniature(32, 16, 6)).unwrap();
        let img = miniature(&p).expect("miniature lue");
        assert_eq!((img.width(), img.height()), (16, 32));
        std::fs::write(&p, jpeg_a_miniature(32, 16, 1)).unwrap();
        let img = miniature(&p).unwrap();
        assert_eq!((img.width(), img.height()), (32, 16));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Le dossier compte toutes ses photos ; une photo sans miniature laisse la
    /// place à sa voisine, une vidéo ne compte pas.
    #[test]
    fn le_dossier_compte_tout_et_ne_garnit_que_de_miniatures() {
        let dir = dossier("compte");
        std::fs::write(dir.join("a.jpg"), jpeg_a_miniature(32, 16, 1)).unwrap();
        std::fs::write(dir.join("b.jpg"), b"\xff\xd8 sans exif").unwrap();
        DynamicImage::new_rgb8(4, 4).save(dir.join("c.png")).unwrap();
        std::fs::write(dir.join("d.mov"), b"video").unwrap();
        let a = survol(&dir, 3, Duration::from_secs(5));
        assert_eq!(a.photos, 3);
        assert!(a.petit);
        assert_eq!(a.miniatures.len(), 1);
        assert_eq!(a.miniatures[0].src, "a.jpg");
        assert!(a.miniatures[0].url.starts_with("data:image/jpeg;base64,/9j/"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Les places se répartissent sur le dossier, et une place sans miniature
    /// prend la photo suivante, jamais une déjà prise.
    #[test]
    fn les_places_se_repartissent_et_glissent_a_la_voisine() {
        let dir = dossier("places");
        for i in 0..10 {
            let octets = if i == 5 { b"\xff\xd8 rien".to_vec() } else { jpeg_a_miniature(8, 8, 1) };
            std::fs::write(dir.join(format!("{i:02}.jpg")), octets).unwrap();
        }
        let a = survol(&dir, 3, Duration::from_secs(5));
        let srcs: Vec<_> = a.miniatures.iter().map(|v| v.src.as_str()).collect();
        assert_eq!(srcs, ["01.jpg", "06.jpg", "08.jpg"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Un budget épuisé rend ce qui est lu, et le compte reste entier.
    #[test]
    fn un_budget_epuise_rend_le_compte_sans_miniature() {
        let dir = dossier("budget");
        for i in 0..4 {
            std::fs::write(dir.join(format!("{i}.jpg")), jpeg_a_miniature(8, 8, 1)).unwrap();
        }
        let a = survol(&dir, 3, Duration::ZERO);
        assert_eq!(a.photos, 4);
        assert!(a.miniatures.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
