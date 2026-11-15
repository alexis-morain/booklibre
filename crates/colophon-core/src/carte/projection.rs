//! Trois projections, écrites à la main (plan D8) : Mercator pour une ville,
//! équirectangulaire corrigée par cos(latitude) pour une région, Equal Earth
//! pour le monde.
//!
//! Elles reçoivent des longitudes **relatives** au centre (`-180..=180`, la
//! couture déjà coupée par `geometrie::decouper_fuseau`) et rendent des
//! coordonnées planes en rayons terrestres, **y vers le haut**. L'échelle en
//! millimètres est affaire de l'emprise, pas de la projection.

use serde::Serialize;

const A1: f64 = 1.340264;
const A2: f64 = -0.081106;
const A3: f64 = 0.000893;
const A4: f64 = 0.003796;

/// La latitude au-delà de laquelle Mercator s'envole.
const LAT_MAX_MERCATOR: f64 = 85.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "projection", rename_all = "snake_case")]
pub enum Projection {
    Mercator { lon0: f64 },
    /// `x = Δλ · cos(lat_ref)`, `y = φ` : la latitude de référence est celle
    /// du milieu de l'emprise.
    Equirectangulaire { lon0: f64, lat_ref: f64 },
    EqualEarth { lon0: f64 },
}

impl Projection {
    pub fn lon0(&self) -> f64 {
        match *self {
            Projection::Mercator { lon0 } | Projection::Equirectangulaire { lon0, .. } | Projection::EqualEarth { lon0 } => lon0,
        }
    }

    /// `(x, y)` en rayons terrestres, `lon` relative au centre.
    pub fn projeter(&self, lon: f64, lat: f64) -> (f64, f64) {
        let l = lon.to_radians();
        match *self {
            Projection::Mercator { .. } => {
                let phi = lat.clamp(-LAT_MAX_MERCATOR, LAT_MAX_MERCATOR).to_radians();
                (l, (std::f64::consts::FRAC_PI_4 + phi / 2.0).tan().ln())
            }
            Projection::Equirectangulaire { lat_ref, .. } => (l * lat_ref.to_radians().cos(), lat.to_radians()),
            Projection::EqualEarth { .. } => {
                let theta = (3f64.sqrt() / 2.0 * lat.to_radians().sin()).asin();
                let (t2, t6) = (theta * theta, theta.powi(6));
                let x = 2.0 * 3f64.sqrt() * l * theta.cos() / (3.0 * (9.0 * A4 * t6 * t2 + 7.0 * A3 * t6 + 3.0 * A2 * t2 + A1));
                let y = theta * (A1 + A2 * t2 + t6 * (A3 + A4 * t2));
                (x, y)
            }
        }
    }

    /// `(lon relative, lat)` pour un point du plan, quand l'inverse est
    /// fermée (les deux premières) : sert à borner ce qu'il faut lire du fond.
    pub fn inverse(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        match *self {
            Projection::Mercator { .. } => Some((x.to_degrees(), y.sinh().atan().to_degrees())),
            Projection::Equirectangulaire { lat_ref, .. } => Some(((x / lat_ref.to_radians().cos()).to_degrees(), y.to_degrees())),
            Projection::EqualEarth { .. } => None,
        }
    }

    /// Les bornes du monde entier, pour Equal Earth : `(demi-largeur, demi-hauteur)`.
    pub fn demi_monde(&self) -> Option<(f64, f64)> {
        match self {
            Projection::EqualEarth { .. } => {
                let (x, _) = self.projeter(180.0, 0.0);
                let (_, y) = self.projeter(0.0, 90.0);
                Some((x, y))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercator_est_conforme_et_s_inverse() {
        let p = Projection::Mercator { lon0: 0.0 };
        let (x, y) = p.projeter(10.0, 45.0);
        assert!((x - 10f64.to_radians()).abs() < 1e-12);
        // y(45°) = ln(tan(67,5°)) = 0,881373587…
        assert!((y - 0.881373587).abs() < 1e-8, "{y}");
        let (lon, lat) = p.inverse(x, y).unwrap();
        assert!((lon - 10.0).abs() < 1e-9 && (lat - 45.0).abs() < 1e-9);
        let (x0, y0) = p.projeter(0.0, 0.0);
        assert!(x0 == 0.0 && y0.abs() < 1e-12);
    }

    #[test]
    fn l_equirectangulaire_corrige_la_largeur_par_le_cosinus() {
        let p = Projection::Equirectangulaire { lon0: 0.0, lat_ref: 60.0 };
        let (x, y) = p.projeter(10.0, 30.0);
        assert!((x - 10f64.to_radians() * 0.5).abs() < 1e-12);
        assert!((y - 30f64.to_radians()).abs() < 1e-12);
        let (lon, lat) = p.inverse(x, y).unwrap();
        assert!((lon - 10.0).abs() < 1e-9 && (lat - 30.0).abs() < 1e-9);
    }

    #[test]
    fn equal_earth_a_ses_valeurs_de_reference() {
        // Šavrič, Patterson, Jenny (2018) : le monde fait 2,7066 de demi-largeur
        // et 1,3173 de demi-hauteur en rayons, soit 2,054 : 1.
        let p = Projection::EqualEarth { lon0: 0.0 };
        let (w, h) = p.demi_monde().unwrap();
        assert!((w - 2.7066).abs() < 1e-3, "{w}");
        assert!((h - 1.3173).abs() < 1e-3, "{h}");
        assert!(((w / h) - 2.054).abs() < 5e-3);
        assert_eq!(p.projeter(0.0, 0.0), (0.0, 0.0));
        // Symétrique, et le méridien central est droit.
        let (x1, y1) = p.projeter(100.0, 40.0);
        let (x2, y2) = p.projeter(-100.0, -40.0);
        assert!((x1 + x2).abs() < 1e-12 && (y1 + y2).abs() < 1e-12);
        assert_eq!(p.projeter(0.0, 50.0).0, 0.0);
    }

    #[test]
    fn equal_earth_conserve_les_aires() {
        // Trois quadrilatères de la sphère, loin les uns des autres : l'aire du
        // plan, intégrée par le jacobien, suit celle de la sphère.
        let p = Projection::EqualEarth { lon0: 0.0 };
        let aire = |lon0: f64, lon1: f64, lat0: f64, lat1: f64| {
            let n = 40;
            let mut s = 0.0;
            for i in 0..n {
                for j in 0..n {
                    let (a, b) = (lon0 + (lon1 - lon0) * (i as f64 + 0.5) / n as f64, lat0 + (lat1 - lat0) * (j as f64 + 0.5) / n as f64);
                    let h = 1e-4;
                    let (xa, ya) = p.projeter(a + h, b);
                    let (xb, yb) = p.projeter(a - h, b);
                    let (xc, yc) = p.projeter(a, b + h);
                    let (xd, yd) = p.projeter(a, b - h);
                    let j11 = (xa - xb) / (2.0 * h);
                    let j21 = (ya - yb) / (2.0 * h);
                    let j12 = (xc - xd) / (2.0 * h);
                    let j22 = (yc - yd) / (2.0 * h);
                    s += (j11 * j22 - j12 * j21).abs() * ((lon1 - lon0) / n as f64) * ((lat1 - lat0) / n as f64);
                }
            }
            s
        };
        // Sphère : aire d'un fuseau [lat0, lat1] sur Δλ = Δλ·(sin lat1 − sin lat0) (en radians).
        let attendu = |lon0: f64, lon1: f64, lat0: f64, lat1: f64| (lon1 - lon0).to_radians() * (lat1.to_radians().sin() - lat0.to_radians().sin());
        // Le rapport entre l'aire du plan et celle de la sphère est le même
        // partout : c'est ce que « équivalente » veut dire.
        let r1 = aire(0.0, 10.0, 0.0, 10.0) / attendu(0.0, 10.0, 0.0, 10.0);
        let r2 = aire(20.0, 50.0, 50.0, 70.0) / attendu(20.0, 50.0, 50.0, 70.0);
        let r3 = aire(-90.0, -60.0, -40.0, -10.0) / attendu(-90.0, -60.0, -40.0, -10.0);
        assert!((r1 - r2).abs() < 2e-3 * r1 && (r1 - r3).abs() < 2e-3 * r1, "{r1} {r2} {r3}");
    }
}
