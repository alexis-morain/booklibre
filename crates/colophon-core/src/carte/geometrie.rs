//! La géométrie plane et sphérique de la carte, écrite à la main : aucune
//! crate de géométrie ni de projection (plan D8).
//!
//! Tout ce qui décide d'un tracé est fait ici sur des `f64` ordinaires, puis
//! **ramené sur une grille** par l'appelant : c'est la grille, jamais le
//! flottant, qui fait l'empreinte d'un dessin (les fonctions `sin`, `atan2`
//! ou `ln` ne rendent pas le même dernier bit sur les trois systèmes).

/// Un point du plan (x, y), ou d'une longitude et d'une latitude en degrés.
pub type P = (f64, f64);

/// Le rayon moyen de la Terre, en km.
pub const RAYON_KM: f64 = 6371.0088;

/// Distance du grand cercle entre deux points `(lat, lon)` en degrés, en km.
pub fn haversine_km(a: P, b: P) -> f64 {
    let (la1, lo1) = (a.0.to_radians(), a.1.to_radians());
    let (la2, lo2) = (b.0.to_radians(), b.1.to_radians());
    let h = ((la2 - la1) / 2.0).sin().powi(2) + la1.cos() * la2.cos() * ((lo2 - lo1) / 2.0).sin().powi(2);
    2.0 * RAYON_KM * h.sqrt().min(1.0).asin()
}

/// Ramène un angle en degrés dans `(-180, 180]`.
pub fn envelopper(deg: f64) -> f64 {
    let mut d = deg % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    d
}

/// Rend les longitudes d'une suite `(lat, lon)` continues : un saut de plus
/// de 180° entre deux points voisins est un passage de l'antiméridien, pas un
/// détour par l'autre côté du monde.
pub fn derouler(points: &mut [P]) {
    for i in 1..points.len() {
        let mut lon = points[i].1;
        while lon - points[i - 1].1 > 180.0 {
            lon -= 360.0;
        }
        while lon - points[i - 1].1 < -180.0 {
            lon += 360.0;
        }
        points[i].1 = lon;
    }
}

/// Les points d'un arc de grand cercle de `a` à `b` (`(lat, lon)`), extrémités
/// comprises, un tous les deux degrés environ. Les longitudes sont continues
/// (`derouler`). Deux points confondus, ou diamétralement opposés (aucun grand
/// cercle unique), rendent la corde.
pub fn arc_grand_cercle(a: P, b: P) -> Vec<P> {
    let v = |p: P| {
        let (la, lo) = (p.0.to_radians(), p.1.to_radians());
        [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
    };
    let (va, vb) = (v(a), v(b));
    let cos_d = (va[0] * vb[0] + va[1] * vb[1] + va[2] * vb[2]).clamp(-1.0, 1.0);
    let delta = cos_d.acos();
    let sin_d = delta.sin();
    if delta < 1e-9 || sin_d < 1e-9 {
        return vec![a, b];
    }
    let n = ((delta.to_degrees() / 2.0).ceil() as usize).clamp(1, 180);
    let mut sortie = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let (ca, cb) = (((1.0 - t) * delta).sin() / sin_d, (t * delta).sin() / sin_d);
        let p = [ca * va[0] + cb * vb[0], ca * va[1] + cb * vb[1], ca * va[2] + cb * vb[2]];
        sortie.push((p[2].clamp(-1.0, 1.0).asin().to_degrees(), p[1].atan2(p[0]).to_degrees()));
    }
    // Les extrémités exactes, pas leur reconstruction.
    sortie[0] = a;
    sortie[n] = b;
    derouler(&mut sortie);
    sortie
}

fn distance_au_segment(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    if l2 == 0.0 {
        return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt();
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0);
    ((p.0 - (a.0 + t * dx)).powi(2) + (p.1 - (a.1 + t * dy)).powi(2)).sqrt()
}

/// Douglas-Peucker sur une ligne ouverte : les deux extrémités restent, un
/// point reste s'il s'écarte de plus de `tol` du segment qui le contourne.
/// Itératif (une carte a des côtes de cent mille points), et le premier des
/// plus éloignés l'emporte : le résultat ne dépend que des coordonnées.
pub fn douglas_peucker(points: &[P], tol: f64) -> Vec<P> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let mut garde = vec![false; n];
    garde[0] = true;
    garde[n - 1] = true;
    let mut pile = vec![(0usize, n - 1)];
    while let Some((i, j)) = pile.pop() {
        let (mut pire, mut rang) = (0.0, 0);
        for k in i + 1..j {
            let d = distance_au_segment(points[k], points[i], points[j]);
            if d > pire {
                (pire, rang) = (d, k);
            }
        }
        if pire > tol {
            garde[rang] = true;
            pile.push((i, rang));
            pile.push((rang, j));
        }
    }
    points.iter().zip(&garde).filter(|(_, g)| **g).map(|(p, _)| *p).collect()
}

/// Rogne une ligne ouverte contre le demi-plan `f(p) >= 0` (`f` affine) : les
/// morceaux qui restent, chacun d'au moins deux points. Une arête qui sort et
/// revient coupe la ligne en deux morceaux ; rien ne les relie.
pub fn rogner_ligne(points: &[P], f: impl Fn(P) -> f64) -> Vec<Vec<P>> {
    let mut morceaux = Vec::new();
    let mut courant: Vec<P> = Vec::new();
    for i in 0..points.len() {
        let (p, fp) = (points[i], f(points[i]));
        if i > 0 {
            let (q, fq) = (points[i - 1], f(points[i - 1]));
            if (fp >= 0.0) != (fq >= 0.0) {
                let t = fq / (fq - fp);
                let x = (q.0 + t * (p.0 - q.0), q.1 + t * (p.1 - q.1));
                if fq >= 0.0 {
                    courant.push(x);
                    morceaux.push(std::mem::take(&mut courant));
                } else {
                    courant.push(x);
                }
            }
        }
        if fp >= 0.0 {
            courant.push(p);
        }
    }
    morceaux.push(courant);
    morceaux.retain(|m| m.len() >= 2);
    morceaux
}

/// Sutherland-Hodgman contre le demi-plan `f(p) >= 0`. Un polygone concave
/// rogné peut garder des arêtes à double sens le long de la coupe : elles
/// n'ont aucune aire, donc rien à remplir, et le polygone n'est jamais tracé
/// en contour (les traits sont des lignes à part).
pub fn rogner_polygone(points: &[P], f: impl Fn(P) -> f64) -> Vec<P> {
    let n = points.len();
    let mut sortie = Vec::with_capacity(n + 4);
    for i in 0..n {
        let (p, q) = (points[i], points[(i + n - 1) % n]);
        let (fp, fq) = (f(p), f(q));
        if (fp >= 0.0) != (fq >= 0.0) {
            let t = fq / (fq - fp);
            sortie.push((q.0 + t * (p.0 - q.0), q.1 + t * (p.1 - q.1)));
        }
        if fp >= 0.0 {
            sortie.push(p);
        }
    }
    if sortie.len() < 3 {
        sortie.clear();
    }
    sortie
}

/// Le rectangle `[x0, x1] × [y0, y1]`.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn polygone(&self, points: &[P]) -> Vec<P> {
        let mut p = rogner_polygone(points, |p| p.0 - self.x0);
        for f in [|r: &Rect, p: P| r.x1 - p.0, |r: &Rect, p: P| p.1 - r.y0, |r: &Rect, p: P| r.y1 - p.1] {
            if p.is_empty() {
                break;
            }
            p = rogner_polygone(&p, |q| f(self, q));
        }
        p
    }

    pub fn ligne(&self, points: &[P]) -> Vec<Vec<P>> {
        let mut morceaux = rogner_ligne(points, |p| p.0 - self.x0);
        for f in [|r: &Rect, p: P| r.x1 - p.0, |r: &Rect, p: P| p.1 - r.y0, |r: &Rect, p: P| r.y1 - p.1] {
            morceaux = morceaux.iter().flat_map(|m| rogner_ligne(m, |q| f(self, q))).collect();
        }
        morceaux
    }
}

/// Un pas de plus de ce nombre de degrés, posé sur une coupure, est subdivisé :
/// la couture est une courbe dans la plupart des projections (Equal Earth), et
/// la corde entre deux points d'intersection mangerait de la terre.
const PAS_DE_COUPE_DEG: f64 = 1.0;

/// Subdivise les arêtes d'un polygone qui longent la coupure `x = lo` ou
/// `x = hi`.
fn densifier_la_coupe(p: &[P], lo: f64, hi: f64) -> Vec<P> {
    let sur = |q: P| (q.0 - lo).abs() < 1e-9 || (q.0 - hi).abs() < 1e-9;
    let mut sortie = Vec::with_capacity(p.len());
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        sortie.push(a);
        if sur(a) && sur(b) && (a.0 - b.0).abs() < 1e-9 && (b.1 - a.1).abs() > PAS_DE_COUPE_DEG {
            let n = ((b.1 - a.1).abs() / PAS_DE_COUPE_DEG).ceil() as usize;
            for k in 1..n {
                sortie.push((a.0, a.1 + (b.1 - a.1) * k as f64 / n as f64));
            }
        }
    }
    sortie
}

/// Coupe un contour `(lon, lat)` aux coutures du monde centré sur `lon0` et
/// le rend en longitudes **relatives** : de `-180` à `180` autour du centre.
/// La couture est à l'antipode du centre ; ce qu'elle traverse est rogné en
/// autant de morceaux qu'il faut, chacun ramené de son côté. C'est ce qui règle
/// l'antiméridien : un contour brut en `[-180, 180]` (Fidji, la Tchoukotka)
/// et un tracé déroulé au-delà se coupent de la même façon.
///
/// `ferme` : un polygone (rogné par Sutherland-Hodgman) ou une ligne ouverte.
pub fn decouper_fuseau(contour: &[P], lon0: f64, ferme: bool) -> Vec<Vec<P>> {
    if contour.is_empty() {
        return Vec::new();
    }
    let (mn, mx) = contour.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let kmin = ((mn - lon0 - 180.0) / 360.0).ceil() as i64;
    let kmax = ((mx - lon0 + 180.0) / 360.0).floor() as i64;
    let mut sortie = Vec::new();
    for k in kmin..=kmax {
        let (lo, hi) = (lon0 - 180.0 + 360.0 * k as f64, lon0 + 180.0 + 360.0 * k as f64);
        let decalage = -lon0 - 360.0 * k as f64;
        let ramener = |m: Vec<P>| m.into_iter().map(|p| (p.0 + decalage, p.1)).collect::<Vec<P>>();
        if ferme {
            let p = if mn >= lo && mx <= hi {
                contour.to_vec()
            } else {
                let moitie = rogner_polygone(contour, |p| p.0 - lo);
                if moitie.is_empty() {
                    continue;
                }
                rogner_polygone(&moitie, |p| hi - p.0)
            };
            if p.len() >= 3 {
                sortie.push(ramener(densifier_la_coupe(&p, lo, hi)));
            }
        } else if mn >= lo && mx <= hi {
            sortie.push(ramener(contour.to_vec()));
        } else {
            for m in rogner_ligne(contour, |p| p.0 - lo) {
                sortie.extend(rogner_ligne(&m, |p| hi - p.0).into_iter().map(ramener));
            }
        }
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn douglas_peucker_garde_les_extremites_et_ce_qui_depasse() {
        let l = vec![(0.0, 0.0), (1.0, 0.05), (2.0, 0.0), (3.0, 2.0), (4.0, 0.0)];
        let s = douglas_peucker(&l, 0.1);
        assert_eq!(s, vec![(0.0, 0.0), (2.0, 0.0), (3.0, 2.0), (4.0, 0.0)]);
        assert_eq!(douglas_peucker(&l, 10.0), vec![(0.0, 0.0), (4.0, 0.0)]);
    }

    #[test]
    fn le_grand_cercle_reste_dans_son_plan_et_ses_pas_sont_egaux() {
        let (a, b) = ((48.9, 2.3), (35.7, 139.7));
        let arc = arc_grand_cercle(a, b);
        assert_eq!((arc[0], *arc.last().unwrap()), (a, b));
        let v = |p: P| {
            let (la, lo) = (p.0.to_radians(), p.1.to_radians());
            [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
        };
        let (va, vb) = (v(a), v(b));
        let n = [va[1] * vb[2] - va[2] * vb[1], va[2] * vb[0] - va[0] * vb[2], va[0] * vb[1] - va[1] * vb[0]];
        let norme = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        for p in &arc {
            let w = v(*p);
            assert!((w[0] * n[0] + w[1] * n[1] + w[2] * n[2]).abs() / norme < 1e-9, "{p:?} hors du plan");
        }
        // Le milieu de Paris-Tokyo passe bien au nord des deux villes.
        let haut = arc.iter().map(|p| p.0).fold(f64::MIN, f64::max);
        assert!(haut > 55.0, "le grand cercle Paris-Tokyo monte à {haut}°");
        let pas: Vec<f64> = arc.windows(2).map(|w| haversine_km(w[0], w[1])).collect();
        let (mn, mx) = pas.iter().fold((f64::MAX, f64::MIN), |(a, b), d| (a.min(*d), b.max(*d)));
        assert!(mx - mn < 1.0, "pas inégaux : {mn} à {mx}");
    }

    #[test]
    fn un_arc_a_travers_l_antimeridien_garde_des_longitudes_continues() {
        let arc = arc_grand_cercle((34.0, -118.0), (-17.7, 177.4));
        assert!(arc.windows(2).all(|w| (w[1].1 - w[0].1).abs() < 10.0), "saut de longitude : {arc:?}");
        assert!(arc.last().unwrap().1 < -180.0, "la fin est déroulée au-delà de -180");
    }

    #[test]
    fn rogner_une_ligne_et_un_polygone_contre_un_demi_plan() {
        let l = vec![(-1.0, 0.0), (3.0, 0.0), (3.0, 1.0), (-1.0, 1.0)];
        let m = rogner_ligne(&l, |p| p.0);
        assert_eq!(m, vec![vec![(0.0, 0.0), (3.0, 0.0), (3.0, 1.0), (0.0, 1.0)]]);
        // La ligne sort par la droite et revient : deux morceaux, rien ne les relie.
        let m = rogner_ligne(&l, |p| 2.0 - p.0);
        assert_eq!(m, vec![vec![(-1.0, 0.0), (2.0, 0.0)], vec![(2.0, 1.0), (-1.0, 1.0)]]);
        let c = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        let r = rogner_polygone(&c, |p| 2.0 - p.0);
        assert_eq!(r.iter().map(|p| p.0).fold(f64::MIN, f64::max), 2.0);
        assert!(rogner_polygone(&c, |p| p.0 - 9.0).is_empty());
    }

    #[test]
    fn la_couture_coupe_un_trace_qui_la_traverse_et_ramene_chaque_morceau() {
        // Centré sur 0, la couture est à ±180 : un trace de 170 à 190 (déroulé) se coupe en deux.
        let trace = vec![(170.0, 10.0), (190.0, 20.0)];
        let m = decouper_fuseau(&trace, 0.0, false);
        assert_eq!(m.len(), 2, "{m:?}");
        assert!(m.iter().flatten().all(|p| p.0.abs() <= 180.0 + 1e-9));
        // Centré sur 175, le même trace ne traverse plus rien.
        let m = decouper_fuseau(&trace, 175.0, false);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0], vec![(-5.0, 10.0), (15.0, 20.0)]);
    }

    #[test]
    fn un_anneau_brut_de_part_et_d_autre_de_180_se_recolle_quand_le_centre_est_proche() {
        // Deux moitiés d'île, de 178 à 180 et de -180 à -178, centrées sur 179.
        let est = vec![(178.0, 0.0), (180.0, 0.0), (180.0, 1.0), (178.0, 1.0)];
        let ouest = vec![(-180.0, 0.0), (-178.0, 0.0), (-178.0, 1.0), (-180.0, 1.0)];
        let a = decouper_fuseau(&est, 179.0, true);
        let b = decouper_fuseau(&ouest, 179.0, true);
        let bornes = |v: &Vec<Vec<P>>| {
            let p: Vec<&P> = v.iter().flatten().collect();
            (p.iter().map(|p| p.0).fold(f64::MAX, f64::min), p.iter().map(|p| p.0).fold(f64::MIN, f64::max))
        };
        assert_eq!(bornes(&a), (-1.0, 1.0));
        assert_eq!(bornes(&b), (1.0, 3.0));
    }

    #[test]
    fn la_coupe_d_un_polygone_est_subdivisee_pour_suivre_la_couture_courbe() {
        // Un grand carré de 20° de côté, coupé en son milieu par la couture (centre 0, couture ±180).
        let c = vec![(170.0, 0.0), (190.0, 0.0), (190.0, 20.0), (170.0, 20.0)];
        let m = decouper_fuseau(&c, 0.0, true);
        assert_eq!(m.len(), 2);
        for morceau in &m {
            let sur_la_coupe: Vec<f64> = morceau.iter().filter(|p| (p.0.abs() - 180.0).abs() < 1e-9).map(|p| p.1).collect();
            assert!(sur_la_coupe.len() >= 20, "la coupe de 20° doit porter un point par degré : {sur_la_coupe:?}");
        }
    }

    #[test]
    fn le_rectangle_rogne_et_ne_laisse_rien_dehors() {
        let r = Rect { x0: 0.0, y0: 0.0, x1: 10.0, y1: 10.0 };
        let grand = vec![(-5.0, -5.0), (15.0, -5.0), (15.0, 15.0), (-5.0, 15.0)];
        let p = r.polygone(&grand);
        assert!(p.iter().all(|q| (0.0..=10.0).contains(&q.0) && (0.0..=10.0).contains(&q.1)));
        let aire: f64 = (0..p.len()).map(|i| p[i].0 * p[(i + 1) % p.len()].1 - p[(i + 1) % p.len()].0 * p[i].1).sum::<f64>() / 2.0;
        assert_eq!(aire.abs(), 100.0);
        let l = r.ligne(&[(-5.0, 5.0), (15.0, 5.0)]);
        assert_eq!(l, vec![vec![(0.0, 5.0), (10.0, 5.0)]]);
        assert!(r.ligne(&[(-5.0, 20.0), (15.0, 20.0)]).is_empty());
    }
}
