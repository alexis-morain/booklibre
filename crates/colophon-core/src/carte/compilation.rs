//! La compilation de Natural Earth vers l'actif (`actif.rs`), reproductible à
//! l'octet : `scripts/carte.sh` la lance, le dépôt ne garde que le résultat.
//!
//! Un lecteur de `.shp` (polygones et polylignes, rien d'autre : la `.dbf`
//! n'est pas lue), puis pour chaque couche :
//! 1. les coordonnées passent en dix-millièmes de degré (11 m) ;
//! 2. les surfaces sont simplifiées en anneau fermé, les lignes en ligne
//!    ouverte, par Douglas-Peucker à la tolérance de la couche ;
//! 3. les **frontières** sont les arêtes que deux anneaux de pays se
//!    partagent (une côte n'a qu'un voisin : la mer), reliées en chaînes
//!    puis simplifiées une fois, si bien que deux pays voisins ne se
//!    contredisent jamais ;
//! 4. les arêtes posées sur l'antiméridien ou sur un pôle sont des coupures
//!    du fichier, pas des traits : aucune ligne ne les dessine.
//!
//! Aucune table de hachage ne se parcourt ici, aucune date n'est écrite.

use super::actif::{ecrire_partie, Couche, ECHELLE, MAGIE};
use super::geometrie::douglas_peucker;
use std::collections::{BTreeMap, BTreeSet};

type Pt = (i32, i32);

/// Les tolérances de simplification, en degrés.
#[derive(Debug, Clone, Copy)]
pub struct Reglages {
    /// Le monde et les continents (couches de 1:50m).
    pub tolerance_monde: f64,
    /// Un pays, une région, une ville (couches de 1:10m).
    pub tolerance_detail: f64,
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages {
            tolerance_monde: TOLERANCE_MONDE,
            tolerance_detail: TOLERANCE_DETAIL,
        }
    }
}

/// 0,01° (1,1 km) : à 3 000 km de large sur 180 mm, la limite de ce niveau,
/// 0,2 mm valent 3,3 km ; le monde entier est bien plus grossier encore.
pub const TOLERANCE_MONDE: f64 = 0.01;
/// 0,001° (110 m) : 0,2 mm à 100 km de large sur 180 mm, soit le bas de ce
/// niveau (60 km en diagonale), et sous la précision propre du 1:10m (autour
/// d'un kilomètre). Mesuré le 14/11 : 0,0005° coûte 5,3 Mo, 0,002° 4,5 Mo,
/// 0,004° 3,6 Mo ; le plafond est de 6 Mo.
pub const TOLERANCE_DETAIL: f64 = 0.001;

/// Le contenu des `.shp` de Natural Earth.
pub struct Sources<'a> {
    pub land50: &'a [u8],
    pub countries50: &'a [u8],
    pub countries10: &'a [u8],
    pub coastline10: &'a [u8],
    pub lakes10: &'a [u8],
    pub rivers10: &'a [u8],
}

/// Lit un `.shp` de polygones (5) ou de polylignes (3) : les parties de chaque
/// enregistrement, dans l'ordre du fichier, en degrés `(lon, lat)`.
pub fn lire_shp(octets: &[u8]) -> Result<Vec<Vec<(f64, f64)>>, String> {
    let i32_le = |p: usize| {
        octets
            .get(p..p + 4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or("shapefile tronqué")
    };
    let f64_le = |p: usize| {
        octets
            .get(p..p + 8)
            .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
            .ok_or("shapefile tronqué")
    };
    if octets.len() < 100 || octets[..4] != [0, 0, 0x27, 0x0a] {
        return Err("pas un shapefile".into());
    }
    let genre = i32_le(32)?;
    if genre != 3 && genre != 5 {
        return Err(format!(
            "shapefile de genre {genre} : seuls 3 (polylignes) et 5 (polygones) sont lus"
        ));
    }
    let mut parties = Vec::new();
    let mut pos = 100;
    while pos + 8 <= octets.len() {
        let mots = i32::from_be_bytes([
            octets[pos + 4],
            octets[pos + 5],
            octets[pos + 6],
            octets[pos + 7],
        ]) as usize;
        let fin = pos + 8 + mots * 2;
        let c = pos + 8;
        if fin > octets.len() {
            return Err("enregistrement de shapefile tronqué".into());
        }
        if i32_le(c)? == genre {
            let (np, nt) = (i32_le(c + 36)? as usize, i32_le(c + 40)? as usize);
            let (debuts, pts) = (c + 44, c + 44 + np * 4);
            let rang = |k: usize| -> Result<usize, String> {
                if k == np {
                    Ok(nt)
                } else {
                    Ok(i32_le(debuts + k * 4)? as usize)
                }
            };
            for k in 0..np {
                let (a, b) = (rang(k)?, rang(k + 1)?);
                if b > nt || a > b {
                    return Err("parties de shapefile incohérentes".into());
                }
                let mut ligne = Vec::with_capacity(b - a);
                for i in a..b {
                    ligne.push((f64_le(pts + i * 16)?, f64_le(pts + i * 16 + 8)?));
                }
                parties.push(ligne);
            }
        }
        pos = fin;
    }
    Ok(parties)
}

fn quantifier(partie: &[(f64, f64)]) -> Vec<Pt> {
    let mut sortie: Vec<Pt> = Vec::with_capacity(partie.len());
    for &(x, y) in partie {
        let p = ((x * ECHELLE).round() as i32, (y * ECHELLE).round() as i32);
        if sortie.last() != Some(&p) {
            sortie.push(p);
        }
    }
    sortie
}

/// Un anneau sans point répété : le dernier point d'un anneau shapefile est
/// le premier.
fn anneau(partie: &[(f64, f64)]) -> Vec<Pt> {
    let mut p = quantifier(partie);
    while p.len() > 1 && p.first() == p.last() {
        p.pop();
    }
    p
}

fn en_f(p: &[Pt]) -> Vec<(f64, f64)> {
    p.iter().map(|&(x, y)| (x as f64, y as f64)).collect()
}

fn en_i(p: &[(f64, f64)]) -> Vec<Pt> {
    p.iter().map(|&(x, y)| (x as i32, y as i32)).collect()
}

fn simplifier_ligne(p: &[Pt], tol: f64) -> Vec<Pt> {
    en_i(&douglas_peucker(&en_f(p), tol * ECHELLE))
}

/// Un anneau : le point le plus éloigné du premier coupe l'anneau en deux
/// lignes simplifiées chacune, extrémités gardées.
fn simplifier_anneau(p: &[Pt], tol: f64) -> Vec<Pt> {
    if p.len() < 4 {
        return p.to_vec();
    }
    let d2 =
        |q: Pt| ((q.0 as i64 - p[0].0 as i64).pow(2) + (q.1 as i64 - p[0].1 as i64).pow(2)) as f64;
    let mut loin = 1;
    for i in 2..p.len() {
        if d2(p[i]) > d2(p[loin]) {
            loin = i;
        }
    }
    let mut a = simplifier_ligne(&p[..=loin], tol);
    let mut retour: Vec<Pt> = p[loin..].to_vec();
    retour.push(p[0]);
    let b = simplifier_ligne(&retour, tol);
    a.pop();
    a.extend_from_slice(&b[..b.len() - 1]);
    a
}

/// Une arête posée sur l'antiméridien ou sur un pôle : une coupure du fichier.
fn arete_artificielle(a: Pt, b: Pt) -> bool {
    let m = 180 * ECHELLE as i32;
    let p = 90 * ECHELLE as i32;
    (a.0.abs() == m && b.0 == a.0) || (a.1.abs() == p && b.1 == a.1)
}

/// Les surfaces d'une couche, simplifiées ; les anneaux qui s'effondrent
/// (une île plus petite que la tolérance) disparaissent.
fn surfaces(parties: &[Vec<(f64, f64)>], tol: f64) -> Vec<Vec<Pt>> {
    parties
        .iter()
        .map(|p| simplifier_anneau(&anneau(p), tol))
        .filter(|a| a.len() >= 3)
        .collect()
}

/// Les lignes d'une couche, simplifiées.
fn lignes(parties: &[Vec<(f64, f64)>], tol: f64) -> Vec<Vec<Pt>> {
    parties
        .iter()
        .map(|p| simplifier_ligne(&quantifier(p), tol))
        .filter(|l| l.len() >= 2)
        .collect()
}

/// Les contours d'anneaux comme lignes, sans les arêtes artificielles : une
/// côte tirée d'une surface (`ne_50m_land`).
fn contours(parties: &[Vec<(f64, f64)>], tol: f64) -> Vec<Vec<Pt>> {
    let mut sortie = Vec::new();
    for p in parties {
        let a = anneau(p);
        let n = a.len();
        if n < 3 {
            continue;
        }
        let coupures: Vec<usize> = (0..n)
            .filter(|&i| arete_artificielle(a[i], a[(i + 1) % n]))
            .collect();
        if coupures.is_empty() {
            let mut l = a.clone();
            l.push(a[0]);
            sortie.push(l);
            continue;
        }
        // Repart juste après une coupure et s'arrête à la suivante.
        let depart = (coupures[0] + 1) % n;
        let mut courant: Vec<Pt> = vec![a[depart]];
        for k in 0..n {
            let i = (depart + k) % n;
            let j = (i + 1) % n;
            if arete_artificielle(a[i], a[j]) {
                if courant.len() >= 2 {
                    sortie.push(std::mem::take(&mut courant));
                }
                courant = vec![a[j]];
            } else {
                courant.push(a[j]);
            }
        }
        if courant.len() >= 2 {
            sortie.push(courant);
        }
    }
    sortie
        .iter()
        .map(|l| simplifier_ligne(l, tol))
        .filter(|l| l.len() >= 2)
        .collect()
}

/// Les arêtes que deux anneaux se partagent, reliées en chaînes. Les sommets
/// d'un degré autre que deux (le point où trois pays se touchent, la fin
/// d'une frontière à la côte) bornent les chaînes ; un cycle sans borne est
/// une enclave et se parcourt depuis son plus petit sommet.
fn frontieres(parties: &[Vec<(f64, f64)>], tol: f64) -> Vec<Vec<Pt>> {
    let mut vues: BTreeMap<(Pt, Pt), u32> = BTreeMap::new();
    for p in parties {
        let a = anneau(p);
        let n = a.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (u, v) = (a[i], a[(i + 1) % n]);
            if arete_artificielle(u, v) {
                continue;
            }
            *vues.entry((u.min(v), u.max(v))).or_insert(0) += 1;
        }
    }
    let mut voisins: BTreeMap<Pt, Vec<Pt>> = BTreeMap::new();
    for (&(u, v), &n) in &vues {
        if n >= 2 {
            voisins.entry(u).or_default().push(v);
            voisins.entry(v).or_default().push(u);
        }
    }
    let mut faites: BTreeSet<(Pt, Pt)> = BTreeSet::new();
    let cle = |u: Pt, v: Pt| (u.min(v), u.max(v));
    let mut chaines: Vec<Vec<Pt>> = Vec::new();
    let suivre = |depart: Pt, premier: Pt, faites: &mut BTreeSet<(Pt, Pt)>| -> Vec<Pt> {
        let mut chaine = vec![depart, premier];
        faites.insert(cle(depart, premier));
        let (mut prec, mut cur) = (depart, premier);
        while voisins[&cur].len() == 2 {
            let suite = voisins[&cur].iter().copied().find(|&w| w != prec);
            let Some(w) = suite else { break };
            if faites.contains(&cle(cur, w)) {
                break;
            }
            faites.insert(cle(cur, w));
            chaine.push(w);
            (prec, cur) = (cur, w);
        }
        chaine
    };
    let bornes: Vec<Pt> = voisins
        .iter()
        .filter(|(_, v)| v.len() != 2)
        .map(|(k, _)| *k)
        .collect();
    for b in bornes {
        let mut vs = voisins[&b].clone();
        vs.sort();
        for v in vs {
            if !faites.contains(&cle(b, v)) {
                chaines.push(suivre(b, v, &mut faites));
            }
        }
    }
    let tous: Vec<Pt> = voisins.keys().copied().collect();
    for u in tous {
        let mut vs = voisins[&u].clone();
        vs.sort();
        for v in vs {
            if !faites.contains(&cle(u, v)) {
                chaines.push(suivre(u, v, &mut faites));
            }
        }
    }
    chaines
        .iter()
        .map(|c| simplifier_ligne(c, tol))
        .filter(|c| c.len() >= 2)
        .collect()
}

fn ecrire_couche(couche: Couche, parties: &[Vec<Pt>], index: &mut Vec<u8>, corps: &mut Vec<u8>) {
    let mut octets = Vec::new();
    for p in parties {
        ecrire_partie(&mut octets, p);
    }
    index.push(couche as u8);
    index.extend_from_slice(&(parties.len() as u32).to_le_bytes());
    index.extend_from_slice(&(octets.len() as u32).to_le_bytes());
    corps.extend_from_slice(&octets);
}

/// Compile les six fichiers en un actif. Mêmes fichiers, mêmes réglages :
/// mêmes octets.
pub fn compiler(src: &Sources, reg: &Reglages) -> Result<Vec<u8>, String> {
    let (m, d) = (reg.tolerance_monde, reg.tolerance_detail);
    let land50 = lire_shp(src.land50).map_err(|e| format!("ne_50m_land : {e}"))?;
    let pays50 =
        lire_shp(src.countries50).map_err(|e| format!("ne_50m_admin_0_countries : {e}"))?;
    let pays10 =
        lire_shp(src.countries10).map_err(|e| format!("ne_10m_admin_0_countries : {e}"))?;
    let cote10 = lire_shp(src.coastline10).map_err(|e| format!("ne_10m_coastline : {e}"))?;
    let lacs10 = lire_shp(src.lakes10).map_err(|e| format!("ne_10m_lakes : {e}"))?;
    let rivieres10 =
        lire_shp(src.rivers10).map_err(|e| format!("ne_10m_rivers_lake_centerlines : {e}"))?;

    let mut index = Vec::new();
    let mut corps = Vec::new();
    ecrire_couche(
        Couche::Terre50,
        &surfaces(&land50, m),
        &mut index,
        &mut corps,
    );
    ecrire_couche(
        Couche::Cotes50,
        &contours(&land50, m),
        &mut index,
        &mut corps,
    );
    ecrire_couche(
        Couche::Frontieres50,
        &frontieres(&pays50, m),
        &mut index,
        &mut corps,
    );
    ecrire_couche(
        Couche::Terre10,
        &surfaces(&pays10, d),
        &mut index,
        &mut corps,
    );
    ecrire_couche(Couche::Cotes10, &lignes(&cote10, d), &mut index, &mut corps);
    ecrire_couche(
        Couche::Frontieres10,
        &frontieres(&pays10, d),
        &mut index,
        &mut corps,
    );
    ecrire_couche(
        Couche::Lacs10,
        &surfaces(&lacs10, d),
        &mut index,
        &mut corps,
    );
    ecrire_couche(
        Couche::Rivieres10,
        &lignes(&rivieres10, d),
        &mut index,
        &mut corps,
    );

    let mut sortie = MAGIE.to_vec();
    sortie.push(8);
    sortie.extend_from_slice(&index);
    sortie.extend_from_slice(&corps);
    Ok(sortie)
}

/// Les mots de `SOURCE.md` : l'empreinte FNV-1a 64 d'un fichier, en hexadécimal.
pub fn empreinte(octets: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in octets {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
pub(crate) mod fabrique {
    //! Des shapefiles minuscules, écrits à la main, pour tester sans Natural Earth.

    /// Un `.shp` de genre `genre` : un enregistrement par élément de `formes`,
    /// chaque forme étant une liste de parties.
    pub fn shp(genre: i32, formes: &[Vec<Vec<(f64, f64)>>]) -> Vec<u8> {
        let mut corps = Vec::new();
        for (i, parties) in formes.iter().enumerate() {
            let pts: Vec<(f64, f64)> = parties.iter().flatten().copied().collect();
            let mut e = Vec::new();
            e.extend_from_slice(&genre.to_le_bytes());
            for v in [0.0f64; 4] {
                e.extend_from_slice(&v.to_le_bytes());
            }
            e.extend_from_slice(&(parties.len() as i32).to_le_bytes());
            e.extend_from_slice(&(pts.len() as i32).to_le_bytes());
            let mut a = 0;
            for p in parties {
                e.extend_from_slice(&(a as i32).to_le_bytes());
                a += p.len();
            }
            for (x, y) in &pts {
                e.extend_from_slice(&x.to_le_bytes());
                e.extend_from_slice(&y.to_le_bytes());
            }
            corps.extend_from_slice(&(i as i32 + 1).to_be_bytes());
            corps.extend_from_slice(&((e.len() / 2) as i32).to_be_bytes());
            corps.extend_from_slice(&e);
        }
        let mut f = vec![0, 0, 0x27, 0x0a];
        f.resize(32, 0);
        f.extend_from_slice(&genre.to_le_bytes());
        f.resize(100, 0);
        f.extend_from_slice(&corps);
        f
    }

    /// Un carré de côté `c` degrés dont le coin bas-gauche est `(x, y)`,
    /// fermé comme un anneau de shapefile.
    pub fn carre(x: f64, y: f64, c: f64) -> Vec<(f64, f64)> {
        vec![(x, y), (x, y + c), (x + c, y + c), (x + c, y), (x, y)]
    }
}

#[cfg(test)]
mod tests {
    use super::fabrique::*;
    use super::*;
    use crate::carte::actif::Actif;

    fn sources_minuscules() -> Vec<Vec<u8>> {
        // Deux pays voisins (un mur commun en x = 1) et un lac.
        let pays = shp(5, &[vec![carre(0.0, 0.0, 1.0)], vec![carre(1.0, 0.0, 1.0)]]);
        let terre = shp(5, &[vec![carre(0.0, 0.0, 2.0)]]);
        let cote = shp(3, &[vec![vec![(0.0, 0.0), (0.5, 0.0001), (1.0, 0.0)]]]);
        let lac = shp(5, &[vec![carre(0.2, 0.2, 0.3)]]);
        let riviere = shp(3, &[vec![vec![(0.0, 0.5), (2.0, 0.5)]]]);
        vec![terre, pays.clone(), pays, cote, lac, riviere]
    }

    fn compiler_minuscule() -> Vec<u8> {
        let s = sources_minuscules();
        let src = Sources {
            land50: &s[0],
            countries50: &s[1],
            countries10: &s[2],
            coastline10: &s[3],
            lakes10: &s[4],
            rivers10: &s[5],
        };
        compiler(&src, &Reglages::default()).unwrap()
    }

    #[test]
    fn le_lecteur_rend_les_parties_d_un_shapefile() {
        let f = shp(
            5,
            &[
                vec![carre(0.0, 0.0, 1.0), carre(5.0, 5.0, 1.0)],
                vec![carre(9.0, 9.0, 1.0)],
            ],
        );
        let p = lire_shp(&f).unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[1][0], (5.0, 5.0));
        assert_eq!(p[2].len(), 5);
        assert!(lire_shp(b"nope").is_err());
        assert!(
            lire_shp(&shp(1, &[])).is_err(),
            "un genre qu'on ne lit pas se refuse"
        );
        let mut coupe = f.clone();
        coupe.truncate(coupe.len() - 10);
        assert!(lire_shp(&coupe).is_err());
    }

    #[test]
    fn deux_compilations_donnent_les_memes_octets() {
        assert_eq!(compiler_minuscule(), compiler_minuscule());
        assert_eq!(empreinte(b"abc"), "e71fa2190541574b");
    }

    #[test]
    fn la_frontiere_est_le_mur_commun_et_pas_la_cote() {
        let o = compiler_minuscule();
        let a = Actif::lire(&o).unwrap();
        let f: Vec<_> = a
            .parties(Couche::Frontieres10)
            .map(|p| p.unwrap().points().unwrap())
            .collect();
        assert_eq!(
            f,
            vec![vec![(1.0, 0.0), (1.0, 1.0)]],
            "une seule chaîne : le mur x=1"
        );
        assert_eq!(a.nombre_de_parties(Couche::Terre10), 2);
        assert_eq!(a.nombre_de_parties(Couche::Lacs10), 1);
        // La côte de 1:10m est simplifiée : le point à 0,0001° du segment tombe.
        let c: Vec<_> = a
            .parties(Couche::Cotes10)
            .map(|p| p.unwrap().points().unwrap())
            .collect();
        assert_eq!(c, vec![vec![(0.0, 0.0), (1.0, 0.0)]]);
    }

    #[test]
    fn les_aretes_sur_l_antimeridien_ou_un_pole_ne_sont_pas_des_traits() {
        // Un anneau qui longe l'antiméridien : sa côte est trois arêtes, pas quatre.
        let anneau_de_bord = vec![
            (170.0, 0.0),
            (180.0, 0.0),
            (180.0, 10.0),
            (170.0, 10.0),
            (170.0, 0.0),
        ];
        let terre = shp(5, &[vec![anneau_de_bord]]);
        let c = contours(&lire_shp(&terre).unwrap(), 0.0);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].len(), 4, "trois arêtes : {:?}", c[0]);
        assert!(!c[0].windows(2).any(|w| arete_artificielle(w[0], w[1])));
        // Un anneau libre est une ligne fermée sur elle-même.
        let libre = contours(
            &lire_shp(&shp(5, &[vec![carre(0.0, 0.0, 1.0)]])).unwrap(),
            0.0,
        );
        assert_eq!((libre.len(), libre[0].len()), (1, 5));
        assert_eq!(libre[0][0], libre[0][4]);
    }

    #[test]
    fn la_simplification_retire_ce_qui_est_sous_la_tolerance_et_garde_le_reste() {
        let ile_minuscule = carre(10.0, 10.0, 0.0005);
        let ile = carre(20.0, 20.0, 1.0);
        let s = surfaces(&[ile_minuscule, ile], 0.0015);
        assert_eq!(s.len(), 1, "l'îlot de 55 m disparaît");
        assert_eq!(s[0].len(), 4);
        // Un contour en dents de scie plus fines que la tolérance devient droit.
        let dents: Vec<(f64, f64)> = (0..100)
            .map(|i| (i as f64 * 0.01, if i % 2 == 0 { 0.0 } else { 0.0005 }))
            .collect();
        assert_eq!(lignes(&[dents], 0.0015)[0].len(), 2);
    }

    #[test]
    fn deux_pays_voisins_simplifies_ne_se_contredisent_pas() {
        // Un mur commun en zigzag : les deux anneaux le portent, la chaîne le simplifie une fois.
        let mur: Vec<(f64, f64)> = (0..=20)
            .map(|i| (1.0 + if i % 2 == 0 { 0.0 } else { 0.0003 }, i as f64 * 0.05))
            .collect();
        let mut gauche = vec![(0.0, 0.0)];
        gauche.extend(mur.iter().copied());
        gauche.push((0.0, 1.0));
        gauche.push((0.0, 0.0));
        let mut droite = vec![(2.0, 0.0), (2.0, 1.0)];
        droite.extend(mur.iter().rev().copied());
        droite.push((2.0, 0.0));
        let f = frontieres(&[gauche, droite], 0.0015);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].len(), 2, "le zigzag de 30 m tombe : {:?}", f[0]);
    }
}
