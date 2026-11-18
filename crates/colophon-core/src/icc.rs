//! The destination colour profile, embedded in every exported PDF.
//!
//! A PDF/X file says which colour it was made for, and says it by carrying
//! the profile rather than by naming it: `sRGB IEC61966-2.1` as a string is
//! an assertion, an OutputIntent with a `DestOutputProfile` is a measurement
//! anyone downstream can redo. Same reasoning as the face in [`crate::font`]:
//! what the file needs travels inside the file.
//!
//! sRGB2014.icc, published by the International Color Consortium and
//! redistributable without restriction (see `assets/sRGB2014-LICENSE.md`).
//! It is the ICC's own v2 encoding of sRGB IEC 61966-2.1, which is exactly
//! what [`crate::printer::Espace::Rgb`] names.
//!
//! The header is read by hand, the way [`crate::font`] reads the TrueType
//! tables: an ICC profile is a 128-byte header of big-endian fields at known
//! offsets, and the point here is not to interpret colour but to refuse an
//! asset that is not the profile we think it is.

use anyhow::{ensure, Result};

/// The profile itself, ~3 kB, embedded whole and unmodified. The licence only
/// constrains altered copies, so this file is never rewritten: it goes into
/// the PDF byte for byte.
pub const ICC_DATA: &[u8] = include_bytes!("../assets/sRGB2014.icc");

/// The name the OutputIntent carries, and the one printers recognise. Kept
/// next to the bytes so the string and the profile cannot drift apart.
pub const CONDITION: &str = "sRGB IEC61966-2.1";

/// The ICC registry, for `/RegistryName`.
pub const REGISTRY: &str = "http://www.color.org";

/// What the PDF needs to know about the profile.
#[derive(Debug, Clone, Copy)]
pub struct Header {
    /// Size the profile declares for itself, which must match the asset.
    pub size: u32,
    /// Major version. PDF/X-4 and PDF/A-2 both refuse a v5 profile.
    pub version_major: u8,
    /// `mntr`, `prtr`, `scnr`, `spac`…
    pub class: [u8; 4],
    /// Data colour space: `RGB `, `CMYK`, `GRAY`.
    pub space: [u8; 4],
    /// Profile connection space.
    pub pcs: [u8; 4],
}

impl Header {
    /// Number of colorants, for the stream's `/N`. A reader needs it to size
    /// the alternate space without parsing the profile.
    pub fn components(&self) -> Result<i64> {
        Ok(match &self.space {
            b"GRAY" => 1,
            b"RGB " => 3,
            b"CMYK" => 4,
            other => anyhow::bail!(
                "espace couleur {} non géré par l'export",
                String::from_utf8_lossy(other)
            ),
        })
    }
}

/// Read the bundled profile.
pub fn header() -> Result<Header> {
    parse(ICC_DATA)
}

fn parse(d: &[u8]) -> Result<Header> {
    ensure!(d.len() >= 132, "profil ICC tronqué : {} octets", d.len());
    // Bytes 36..40 are the mandatory `acsp` signature. Its absence means the
    // asset is not an ICC profile at all: an HTML error page saved under the
    // right name, which is exactly how this one nearly shipped.
    ensure!(&d[36..40] == b"acsp", "signature acsp absente : ce n'est pas un profil ICC");

    let size = u32::from_be_bytes([d[0], d[1], d[2], d[3]]);
    ensure!(
        size as usize == d.len(),
        "le profil annonce {size} octets et en fait {}",
        d.len()
    );

    Ok(Header {
        size,
        version_major: d[8],
        class: [d[12], d[13], d[14], d[15]],
        space: [d[16], d[17], d[18], d[19]],
        pcs: [d[20], d[21], d[22], d[23]],
    })
}

/// How far a colorant may sit from sRGB's and still be sRGB, per XYZ
/// component. Every sRGB profile met in the test sets (ICC v2, ICC v4,
/// Apple's, the camera's « sRGB IEC61966-2.1 ») matches sRGB2014 at 0,0;
/// the nearest profile that is not sRGB, Apple's Generic RGB, sits 0,032
/// away, Display P3 0,093. A tenth of the first gap leaves room for an
/// encoder that rounds differently and none for another gamut.
pub const TOLERANCE_SRGB: f64 = 0.002;

/// A source profile worth declaring in the PDF: the bytes as the file gave
/// them, once they have been read as a valid RGB profile (v4 at most, which
/// is what PDF/A-2 accepts) whose colorants are not sRGB's. Everything else
/// — sRGB, absent, truncated, CMYK, grey, unreadable — is `None`, and the
/// image goes out as `DeviceRGB` exactly as before: a profile is never a
/// reason for an export to fail.
pub fn a_declarer(profil: Vec<u8>) -> Option<std::sync::Arc<[u8]>> {
    let h = parse(&profil).ok()?;
    if &h.space != b"RGB " || h.version_major > 4 || !matches!(&h.pcs, b"XYZ " | b"Lab ") {
        return None;
    }
    let srgb = colorants(ICC_DATA)?;
    let est_srgb = colorants(&profil).is_some_and(|c| {
        c.iter().flatten().zip(srgb.iter().flatten()).all(|(a, b)| (a - b).abs() <= TOLERANCE_SRGB)
    });
    (!est_srgb).then(|| profil.into())
}

/// The profile a JPEG declares, to declare again in the PDF.
pub fn du_jpeg(data: &[u8]) -> Option<std::sync::Arc<[u8]>> {
    a_declarer(jpeg_icc(data)?)
}

/// The profile a photo file declares, read from its bytes: APP2 for a JPEG,
/// `iCCP` for a PNG. No system API, so it reads the same on every OS.
pub fn du_fichier(path: &std::path::Path) -> Option<std::sync::Arc<[u8]>> {
    use image::ImageDecoder;
    let data = std::fs::read(path).ok()?;
    if data.starts_with(&[0xFF, 0xD8]) {
        return du_jpeg(&data);
    }
    let mut png = image::codecs::png::PngDecoder::new(std::io::Cursor::new(&data)).ok()?;
    a_declarer(png.icc_profile().ok()??)
}

/// `rXYZ`, `gXYZ`, `bXYZ` from the tag table, `None` when one is missing
/// (a LUT-only profile): then nothing says it is sRGB, and it is declared.
fn colorants(d: &[u8]) -> Option<[[f64; 3]; 3]> {
    let be = |o: usize| d.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let n = be(128)? as usize;
    let mut out = [[0.0; 3]; 3];
    for (i, sig) in [b"rXYZ", b"gXYZ", b"bXYZ"].iter().enumerate() {
        let entree = (0..n.min(1024)).map(|k| 132 + 12 * k).find(|&e| d.get(e..e + 4) == Some(&sig[..]))?;
        let o = be(entree + 4)? as usize;
        if d.get(o..o + 4)? != b"XYZ " {
            return None;
        }
        for (j, v) in out[i].iter_mut().enumerate() {
            *v = f64::from(be(o + 8 + 4 * j)? as i32) / 65536.0;
        }
    }
    Some(out)
}

/// The ICC profile of a JPEG, reassembled from its `ICC_PROFILE` APP2
/// segments. A profile over 64 kB is cut into numbered chunks
/// (sequence, count); every chunk from 1 to count must be there once, in
/// any order, or the profile is unreadable and the answer is `None`.
pub fn jpeg_icc(data: &[u8]) -> Option<Vec<u8>> {
    if !data.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut morceaux: Vec<(u8, &[u8])> = Vec::new();
    let mut total = None;
    let mut i = 2;
    while i + 4 <= data.len() && data[i] == 0xFF {
        let marqueur = data[i + 1];
        if marqueur == 0xFF {
            i += 1; // fill byte
            continue;
        }
        if marqueur == 0xDA || marqueur == 0xD9 {
            break; // the profile lives in the header, before the scan
        }
        let longueur = usize::from(u16::from_be_bytes([data[i + 2], data[i + 3]]));
        let segment = data.get(i + 4..i + 2 + longueur)?;
        if marqueur == 0xE2 && segment.len() >= 14 && segment.starts_with(b"ICC_PROFILE\0") {
            let (seq, n) = (segment[12], segment[13]);
            if seq == 0 || seq > n || *total.get_or_insert(n) != n {
                return None;
            }
            morceaux.push((seq, &segment[14..]));
        }
        i += 2 + longueur;
    }
    let n = total?;
    morceaux.sort_by_key(|(seq, _)| *seq);
    let complet = morceaux.len() == usize::from(n)
        && morceaux.iter().enumerate().all(|(k, (seq, _))| usize::from(*seq) == k + 1);
    complet.then(|| morceaux.iter().flat_map(|(_, m)| m.iter().copied()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A JPEG header carrying `profil` in APP2 chunks of at most `taille`
    /// bytes, in the given chunk order (1-based).
    fn jpeg_avec(profil: &[u8], taille: usize, ordre: &[usize]) -> Vec<u8> {
        let morceaux: Vec<&[u8]> = profil.chunks(taille).collect();
        let mut d = vec![0xFF, 0xD8];
        for &k in ordre {
            let m = morceaux[k - 1];
            d.extend_from_slice(&[0xFF, 0xE2]);
            d.extend_from_slice(&((m.len() + 16) as u16).to_be_bytes());
            d.extend_from_slice(b"ICC_PROFILE\0");
            d.extend_from_slice(&[k as u8, morceaux.len() as u8]);
            d.extend_from_slice(m);
        }
        d.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9]);
        d
    }

    /// The profile of a phone: sRGB2014 with its three colorants moved to
    /// Display P3's, which is all `a_declarer` reads beyond the header.
    fn faux_p3() -> Vec<u8> {
        let p3 = [[0.5151, 0.2412, -0.0011], [0.2920, 0.6922, 0.0419], [0.1571, 0.0666, 0.7841]];
        poser_colorants(ICC_DATA.to_vec(), p3)
    }

    fn poser_colorants(mut d: Vec<u8>, c: [[f64; 3]; 3]) -> Vec<u8> {
        let n = u32::from_be_bytes(d[128..132].try_into().unwrap()) as usize;
        for (i, sig) in [b"rXYZ", b"gXYZ", b"bXYZ"].iter().enumerate() {
            let e = (0..n).map(|k| 132 + 12 * k).find(|&e| &d[e..e + 4] == &sig[..]).unwrap();
            let o = u32::from_be_bytes(d[e + 4..e + 8].try_into().unwrap()) as usize;
            for j in 0..3 {
                let v = ((c[i][j] * 65536.0).round() as i32).to_be_bytes();
                d[o + 8 + 4 * j..o + 12 + 4 * j].copy_from_slice(&v);
            }
        }
        d
    }

    #[test]
    fn un_profil_en_plusieurs_segments_se_recolle_dans_l_ordre() {
        let p = faux_p3();
        assert_eq!(jpeg_icc(&jpeg_avec(&p, p.len(), &[1])).unwrap(), p);
        let n = p.len().div_ceil(1000);
        let dans_l_ordre: Vec<usize> = (1..=n).collect();
        assert_eq!(jpeg_icc(&jpeg_avec(&p, 1000, &dans_l_ordre)).unwrap(), p);
        let renverse: Vec<usize> = (1..=n).rev().collect();
        assert_eq!(jpeg_icc(&jpeg_avec(&p, 1000, &renverse)).unwrap(), p, "segments dans le désordre");
        let manquant: Vec<usize> = (2..=n).collect();
        assert_eq!(jpeg_icc(&jpeg_avec(&p, 1000, &manquant)), None, "un segment manque");
        let double: Vec<usize> = std::iter::once(1).chain(1..n).collect();
        assert_eq!(jpeg_icc(&jpeg_avec(&p, 1000, &double)), None, "un segment en double");
        assert_eq!(jpeg_icc(&[0xFF, 0xD8, 0xFF, 0xDA, 0, 2]), None, "aucun profil");
        assert_eq!(jpeg_icc(b"\x89PNG"), None);
    }

    #[test]
    fn srgb_ne_se_declare_pas_et_p3_si() {
        assert!(a_declarer(ICC_DATA.to_vec()).is_none(), "sRGB2014");
        let p3 = faux_p3();
        assert_eq!(a_declarer(p3.clone()).as_deref(), Some(&p3[..]));
        // Un sRGB arrondi par un autre encodeur reste sRGB, au bord de la tolérance.
        let srgb = colorants(ICC_DATA).unwrap();
        let mut proche = srgb;
        // Absolute, not a share of the constant: a tolerance set to zero must fail here.
        proche[0][0] += 0.0015;
        assert!(a_declarer(poser_colorants(ICC_DATA.to_vec(), proche)).is_none());
        let mut loin = srgb;
        loin[1][0] += 0.02;
        assert!(a_declarer(poser_colorants(ICC_DATA.to_vec(), loin)).is_some());
        // Non RVB, v5, tronqué : rien à déclarer, jamais une erreur.
        let mut cmjn = p3.clone();
        cmjn[16..20].copy_from_slice(b"CMYK");
        assert!(a_declarer(cmjn).is_none());
        let mut v5 = p3.clone();
        v5[8] = 5;
        assert!(a_declarer(v5).is_none());
        assert!(a_declarer(p3[..p3.len() - 1].to_vec()).is_none());
        assert!(a_declarer(b"pas un profil".to_vec()).is_none());
    }

    /// The system's own profiles, read the way a photo's would be: the five
    /// profiles with sRGB's primaries stay `DeviceRGB` (ITU-709 too: the rule
    /// reads colorants, not the tone curve), the other gamuts do not.
    #[cfg(target_os = "macos")]
    #[test]
    fn les_profils_du_systeme_se_classent() {
        let lire = |n: &str| std::fs::read(format!("/System/Library/ColorSync/Profiles/{n}")).unwrap();
        for n in ["sRGB Profile.icc", "ITU-709.icc"] {
            assert!(a_declarer(lire(n)).is_none(), "{n}");
        }
        for n in ["Display P3.icc", "AdobeRGB1998.icc", "Generic RGB Profile.icc"] {
            assert!(a_declarer(lire(n)).is_some(), "{n}");
        }
        assert!(a_declarer(lire("Generic CMYK Profile.icc")).is_none());
    }

    /// The bundled asset really is the sRGB profile the OutputIntent claims.
    /// A swap that put a v5 profile, a CMYK one, or a downloaded HTML page in
    /// its place breaks here rather than at a printer.
    #[test]
    fn the_bundled_profile_is_srgb_v2() {
        let h = header().expect("profil lisible");
        assert_eq!(&h.class, b"mntr", "classe {:?}", h.class);
        assert_eq!(&h.space, b"RGB ", "espace {:?}", h.space);
        assert_eq!(&h.pcs, b"XYZ ");
        assert!(h.version_major <= 4, "version ICC {}", h.version_major);
        assert_eq!(h.components().unwrap(), 3);
        assert_eq!(h.size as usize, ICC_DATA.len());
    }

    /// The description tag names the profile in clear text, and the copyright
    /// tag is the one the licence file quotes. Both travel in the PDF, so
    /// both are worth checking before they do.
    #[test]
    fn the_profile_carries_its_identification() {
        let s = String::from_utf8_lossy(ICC_DATA);
        assert!(s.contains("sRGB2014"), "description absente");
        assert!(s.contains("International Color Consortium"), "copyright absent");
    }

    /// A truncated or mislabelled profile is refused, not embedded.
    #[test]
    fn a_broken_profile_is_refused() {
        assert!(parse(b"<!DOCTYPE html>").is_err());
        let mut bad = ICC_DATA.to_vec();
        bad[36] = b'x';
        assert!(parse(&bad).is_err(), "signature acsp non vérifiée");
        let short = &ICC_DATA[..ICC_DATA.len() - 1];
        assert!(parse(short).is_err(), "taille non vérifiée");
    }
}
