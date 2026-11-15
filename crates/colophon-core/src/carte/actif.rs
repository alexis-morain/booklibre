//! Le format de l'actif de carte : Natural Earth, simplifié, en un seul fichier
//! que le binaire embarque (`assets/carte/carte.bin`).
//!
//! ```text
//! "BLCARTE1"
//! u8 nombre de couches
//! par couche : u8 code, u32 nombre de parties, u32 octets      (little-endian)
//! puis les octets des couches, dans le même ordre
//! une partie : varint longueur · varint n · zz x0 · zz y0
//!              · varint (x0-minx) (maxx-x0) (y0-miny) (maxy-y0)
//!              · (n-1) fois : zz dx, zz dy
//! ```
//!
//! Les coordonnées sont des entiers en dix-millièmes de degré (11 m). Une
//! partie est un anneau (couches de surface, fermé sans répéter son premier
//! point) ou une ligne ouverte. Sa **boîte** est écrite avant ses points, et
//! sa longueur avant tout : une carte de ville saute les quinze mille parties
//! qui ne la touchent pas sans en décoder un point.
//!
//! Aucun ordre de hachage, aucune date, aucun flottant : deux compilations des
//! mêmes fichiers donnent les mêmes octets (`compilation.rs`).

pub const MAGIE: &[u8; 8] = b"BLCARTE1";

/// Dix-millièmes de degré par degré.
pub const ECHELLE: f64 = 10_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Couche {
    /// `ne_50m_land` : le monde et les continents.
    Terre50 = 0,
    /// Les contours de `ne_50m_land`, sans les arêtes posées sur l'antiméridien.
    Cotes50 = 1,
    /// Les arêtes que deux pays de `ne_50m_admin_0_countries` se partagent.
    Frontieres50 = 2,
    /// `ne_10m_admin_0_countries` : un pays, une région.
    Terre10 = 3,
    /// `ne_10m_coastline`.
    Cotes10 = 4,
    Frontieres10 = 5,
    /// `ne_10m_lakes`.
    Lacs10 = 6,
    /// `ne_10m_rivers_lake_centerlines`.
    Rivieres10 = 7,
}

impl Couche {
    pub const TOUTES: [Couche; 8] = [
        Couche::Terre50,
        Couche::Cotes50,
        Couche::Frontieres50,
        Couche::Terre10,
        Couche::Cotes10,
        Couche::Frontieres10,
        Couche::Lacs10,
        Couche::Rivieres10,
    ];

    fn depuis_code(c: u8) -> Option<Couche> {
        Couche::TOUTES.iter().copied().find(|k| *k as u8 == c)
    }

    /// Une surface (anneaux à remplir) ou des lignes (traits).
    pub fn est_surface(self) -> bool {
        matches!(self, Couche::Terre50 | Couche::Terre10 | Couche::Lacs10)
    }
}

pub fn ecrire_varint(sortie: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        sortie.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
    sortie.push(v as u8);
}

pub fn zigzag(v: i32) -> u64 {
    ((v << 1) ^ (v >> 31)) as u32 as u64
}

fn dezigzag(v: u64) -> i32 {
    let v = v as u32;
    ((v >> 1) as i32) ^ -((v & 1) as i32)
}

fn lire_varint(octets: &[u8], pos: &mut usize) -> Result<u64, String> {
    let (mut v, mut decalage) = (0u64, 0u32);
    loop {
        let b = *octets.get(*pos).ok_or("actif de carte tronqué")?;
        *pos += 1;
        v |= ((b & 0x7f) as u64) << decalage;
        if b < 0x80 {
            return Ok(v);
        }
        decalage += 7;
        if decalage > 56 {
            return Err("entier trop long dans l'actif de carte".into());
        }
    }
}

/// Écrit une partie : les points sont des entiers en dix-millièmes de degré.
pub fn ecrire_partie(sortie: &mut Vec<u8>, points: &[(i32, i32)]) {
    let (mut mnx, mut mny, mut mxx, mut mxy) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in points {
        mnx = mnx.min(p.0);
        mxx = mxx.max(p.0);
        mny = mny.min(p.1);
        mxy = mxy.max(p.1);
    }
    let mut corps = Vec::new();
    ecrire_varint(&mut corps, points.len() as u64);
    ecrire_varint(&mut corps, zigzag(points[0].0));
    ecrire_varint(&mut corps, zigzag(points[0].1));
    for v in [points[0].0 - mnx, mxx - points[0].0, points[0].1 - mny, mxy - points[0].1] {
        ecrire_varint(&mut corps, v as u64);
    }
    for w in points.windows(2) {
        ecrire_varint(&mut corps, zigzag(w[1].0 - w[0].0));
        ecrire_varint(&mut corps, zigzag(w[1].1 - w[0].1));
    }
    ecrire_varint(sortie, corps.len() as u64);
    sortie.extend_from_slice(&corps);
}

/// Un actif lu : les couches, sans copie.
pub struct Actif<'a> {
    couches: Vec<(Couche, u32, &'a [u8])>,
}

impl<'a> Actif<'a> {
    pub fn lire(octets: &'a [u8]) -> Result<Actif<'a>, String> {
        if octets.len() < 9 || &octets[..8] != MAGIE {
            return Err("actif de carte : en-tête inconnu".into());
        }
        let n = octets[8] as usize;
        let mut pos = 9 + n * 9;
        let mut couches = Vec::new();
        for i in 0..n {
            let e = octets.get(9 + i * 9..18 + i * 9).ok_or("actif de carte tronqué")?;
            let couche = Couche::depuis_code(e[0]).ok_or("actif de carte : couche inconnue")?;
            let parties = u32::from_le_bytes([e[1], e[2], e[3], e[4]]);
            let longueur = u32::from_le_bytes([e[5], e[6], e[7], e[8]]) as usize;
            let corps = octets.get(pos..pos + longueur).ok_or("actif de carte tronqué")?;
            couches.push((couche, parties, corps));
            pos += longueur;
        }
        if pos != octets.len() {
            return Err("actif de carte : octets en trop".into());
        }
        Ok(Actif { couches })
    }

    pub fn nombre_de_parties(&self, c: Couche) -> usize {
        self.couches.iter().find(|k| k.0 == c).map_or(0, |k| k.1 as usize)
    }

    pub fn octets(&self, c: Couche) -> usize {
        self.couches.iter().find(|k| k.0 == c).map_or(0, |k| k.2.len())
    }

    /// Les parties d'une couche, dans l'ordre où elles ont été écrites.
    pub fn parties(&self, c: Couche) -> Parties<'a> {
        Parties { reste: self.couches.iter().find(|k| k.0 == c).map_or(&[], |k| k.2) }
    }
}

pub struct Parties<'a> {
    reste: &'a [u8],
}

/// Une partie dont seule la boîte est lue ; les points se décodent à la demande.
pub struct Partie<'a> {
    /// `[min_x, min_y, max_x, max_y]`, en dix-millièmes de degré.
    pub boite: [i32; 4],
    n: usize,
    depart: (i32, i32),
    deltas: &'a [u8],
}

impl<'a> Iterator for Parties<'a> {
    type Item = Result<Partie<'a>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reste.is_empty() {
            return None;
        }
        let lire = || -> Result<(Partie<'a>, usize), String> {
            let mut pos = 0;
            let longueur = lire_varint(self.reste, &mut pos)? as usize;
            let fin = pos + longueur;
            let corps = self.reste.get(pos..fin).ok_or("actif de carte tronqué")?;
            let mut p = 0;
            let n = lire_varint(corps, &mut p)? as usize;
            let x0 = dezigzag(lire_varint(corps, &mut p)?);
            let y0 = dezigzag(lire_varint(corps, &mut p)?);
            let (a, b, c, d) = (
                lire_varint(corps, &mut p)? as i32,
                lire_varint(corps, &mut p)? as i32,
                lire_varint(corps, &mut p)? as i32,
                lire_varint(corps, &mut p)? as i32,
            );
            Ok((Partie { boite: [x0 - a, y0 - c, x0 + b, y0 + d], n, depart: (x0, y0), deltas: &corps[p..] }, fin))
        };
        match lire() {
            Ok((partie, fin)) => {
                self.reste = &self.reste[fin..];
                Some(Ok(partie))
            }
            Err(e) => {
                self.reste = &[];
                Some(Err(e))
            }
        }
    }
}

impl Partie<'_> {
    /// Les points en degrés `(lon, lat)`.
    pub fn points(&self) -> Result<Vec<(f64, f64)>, String> {
        let mut pts = Vec::with_capacity(self.n);
        let (mut x, mut y) = self.depart;
        pts.push((x as f64 / ECHELLE, y as f64 / ECHELLE));
        let mut pos = 0;
        for _ in 1..self.n {
            x += dezigzag(lire_varint(self.deltas, &mut pos)?);
            y += dezigzag(lire_varint(self.deltas, &mut pos)?);
            pts.push((x as f64 / ECHELLE, y as f64 / ECHELLE));
        }
        Ok(pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_partie_fait_l_aller_retour_avec_sa_boite() {
        let pts = vec![(1_800_000, -900_000), (-1_800_000, 900_000), (5, 7), (5, 7), (-3, 0)];
        let mut corps = Vec::new();
        ecrire_partie(&mut corps, &pts);
        ecrire_partie(&mut corps, &[(10, 20), (11, 21)]);
        let mut it = Parties { reste: &corps };
        let p = it.next().unwrap().unwrap();
        assert_eq!(p.boite, [-1_800_000, -900_000, 1_800_000, 900_000]);
        let relus: Vec<(i32, i32)> = p.points().unwrap().iter().map(|(x, y)| ((x * ECHELLE).round() as i32, (y * ECHELLE).round() as i32)).collect();
        assert_eq!(relus, pts);
        assert_eq!(it.next().unwrap().unwrap().boite, [10, 20, 11, 21]);
        assert!(it.next().is_none());
    }

    #[test]
    fn un_actif_tronque_ou_inconnu_se_refuse() {
        assert!(Actif::lire(b"pas un actif").is_err());
        let mut o = MAGIE.to_vec();
        o.push(1);
        o.extend_from_slice(&[0, 1, 0, 0, 0, 9, 0, 0, 0]);
        o.extend_from_slice(&[1, 2, 3]);
        assert!(Actif::lire(&o).is_err());
    }

    #[test]
    fn le_zigzag_se_defait() {
        for v in [0, 1, -1, 63, -64, i32::MAX, i32::MIN + 1, 1_800_000] {
            assert_eq!(dezigzag(zigzag(v)), v);
        }
    }
}
