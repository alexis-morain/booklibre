//! Le tracé d'un voyage sur la carte : quelles lignes, de quel mode (plan D9,
//! D18). Géographique seulement : `(lat, lon)` en degrés, longitudes
//! déroulées ; la projection est affaire de `mod.rs`.
//!
//! Trois sources, la première qui existe l'emporte :
//! 1. les **trajets** de la source (`route_segments`), avec leur mode de
//!    transport ; ceux de la plage sont les trajets dont les deux bouts sont
//!    des étapes de la plage (`de`, `a`, sinon l'étape la plus proche du bout) ;
//! 2. à défaut de tout trajet dans le voyage, la **trace** du tracker, sur la
//!    fenêtre de temps de la plage ;
//! 3. à défaut, la **suite des étapes**.
//!
//! Un trajet sans mode et un saut de la trace se jugent à la **vitesse** : au-
//! delà de 250 km/h c'est un avion, dessiné en arc de grand cercle. Jamais une
//! IA, jamais une table de villes.

use super::geometrie::{arc_grand_cercle, derouler, haversine_km, P};
use crate::voyage::{Etape, Voyage};
use chrono::{DateTime, Utc};
use std::ops::Range;

/// Au-delà, c'est un avion.
pub const VITESSE_AVION_KMH: f64 = 250.0;

/// Un morceau de tracé : `(lat, lon)`, longitudes continues.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub vol: bool,
    pub points: Vec<P>,
}

fn est_vol(mode: &str) -> bool {
    matches!(mode, "flight" | "plane" | "air")
}

fn position(e: &Etape) -> P {
    (e.lieu.lat, e.lieu.lon)
}

fn longueur_km(p: &[P]) -> f64 {
    p.windows(2).map(|w| haversine_km(w[0], w[1])).sum()
}

fn plus_proche(etapes: &[Etape], p: P) -> usize {
    let mut meilleur = (f64::MAX, 0);
    for (i, e) in etapes.iter().enumerate() {
        let d = haversine_km(p, position(e));
        if d < meilleur.0 {
            meilleur = (d, i);
        }
    }
    meilleur.1
}

/// Les heures entre deux étapes `i < j` : de la fin de la première (si elle
/// est connue et plausible) au début de la seconde.
fn heures_entre(etapes: &[Etape], i: usize, j: usize) -> Option<f64> {
    let (a, b) = (&etapes[i.min(j)], &etapes[i.max(j)]);
    let depuis = a
        .fin
        .filter(|f| *f >= a.debut && *f <= b.debut)
        .unwrap_or(a.debut);
    let h = (b.debut - depuis).num_seconds() as f64 / 3600.0;
    (h > 0.0).then_some(h)
}

fn plus_vite_qu_un_avion(km: f64, heures: Option<f64>) -> bool {
    heures.is_some_and(|h| km / h > VITESSE_AVION_KMH)
}

/// Un trajet de `a` à `b` : l'arc de grand cercle d'un vol, la corde sinon.
fn jambe(a: P, b: P, vol: bool) -> Piece {
    let mut points = if vol {
        arc_grand_cercle(a, b)
    } else {
        vec![a, b]
    };
    derouler(&mut points);
    Piece { vol, points }
}

/// Une polyligne de la source, déroulée ; un vol se densifie en arcs.
fn polyligne(points: &[P], vol: bool) -> Piece {
    if !vol {
        let mut p = points.to_vec();
        derouler(&mut p);
        return Piece { vol, points: p };
    }
    let mut sortie: Vec<P> = Vec::new();
    for w in points.windows(2) {
        let arc = arc_grand_cercle(w[0], w[1]);
        sortie.extend(if sortie.is_empty() {
            &arc[..]
        } else {
            &arc[1..]
        });
    }
    derouler(&mut sortie);
    Piece {
        vol,
        points: sortie,
    }
}

pub fn pieces(voyage: &Voyage, plage: &Range<usize>) -> Vec<Piece> {
    let etapes = &voyage.etapes;
    let mut sortie = Vec::new();
    let mut couvert = vec![false; plage.end.saturating_sub(plage.start).saturating_sub(1)];

    for seg in &voyage.segments {
        let bout = |explicite: Option<usize>, p: Option<&crate::voyage::Point>| {
            explicite
                .filter(|i| *i < etapes.len())
                .or_else(|| p.map(|p| plus_proche(etapes, (p.lat, p.lon))))
        };
        let (Some(s), Some(e)) = (
            bout(seg.de, seg.polyligne.first()),
            bout(seg.a, seg.polyligne.last()),
        ) else {
            continue;
        };
        if !(plage.contains(&s) && plage.contains(&e)) {
            continue;
        }
        let pts: Vec<P> = if seg.polyligne.len() >= 2 {
            seg.polyligne.iter().map(|p| (p.lat, p.lon)).collect()
        } else if s != e && seg.polyligne.is_empty() {
            vec![position(&etapes[s]), position(&etapes[e])]
        } else {
            continue;
        };
        let vol = match seg.mode.as_deref() {
            Some(m) => est_vol(m),
            None => s != e && plus_vite_qu_un_avion(longueur_km(&pts), heures_entre(etapes, s, e)),
        };
        for k in s.min(e)..s.max(e) {
            if let Some(c) = couvert.get_mut(k.wrapping_sub(plage.start)) {
                *c = true;
            }
        }
        sortie.push(polyligne(&pts, vol));
    }

    if voyage.segments.is_empty() && !voyage.trace.is_empty() {
        let t0 = (plage.start > 0).then(|| etapes[plage.start].debut);
        let t1 = (plage.end < etapes.len()).then(|| etapes[plage.end].debut);
        let dedans: Vec<(P, DateTime<Utc>)> = voyage
            .trace
            .iter()
            .filter(|p| t0.is_none_or(|t| p.instant >= t) && t1.is_none_or(|t| p.instant < t))
            .map(|p| ((p.lat, p.lon), p.instant))
            .collect();
        return decouper_la_trace(&dedans);
    }

    // Les écarts que rien ne couvre : la suite des étapes.
    for (i, c) in couvert.iter().enumerate() {
        if !*c {
            let k = plage.start + i;
            let (a, b) = (position(&etapes[k]), position(&etapes[k + 1]));
            let vol = plus_vite_qu_un_avion(haversine_km(a, b), heures_entre(etapes, k, k + 1));
            sortie.push(jambe(a, b, vol));
        }
    }
    sortie
}

/// La trace du tracker : une ligne tant que la vitesse reste celle d'un
/// véhicule, un arc dès qu'elle ne l'est plus.
fn decouper_la_trace(points: &[(P, DateTime<Utc>)]) -> Vec<Piece> {
    let mut sortie = Vec::new();
    let mut courant: Vec<P> = Vec::new();
    for (i, (p, t)) in points.iter().enumerate() {
        if i > 0 {
            let (q, tq) = points[i - 1];
            let km = haversine_km(q, *p);
            let h = (*t - tq).num_seconds() as f64 / 3600.0;
            // Un saut sans durée lisible, mais de plus de 500 km, est un vol aussi.
            let vol = if h > 0.0 {
                km / h > VITESSE_AVION_KMH && km > 50.0
            } else {
                km > 500.0
            };
            if vol {
                if courant.len() >= 2 {
                    sortie.push(polyligne(&std::mem::take(&mut courant), false));
                }
                courant.clear();
                sortie.push(jambe(q, *p, true));
                courant.push(*p);
                continue;
            }
        }
        courant.push(*p);
    }
    if courant.len() >= 2 {
        sortie.push(polyligne(&courant, false));
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_vitesse_fait_l_avion() {
        assert!(plus_vite_qu_un_avion(900.0, Some(2.0)));
        assert!(!plus_vite_qu_un_avion(400.0, Some(4.0)));
        assert!(
            !plus_vite_qu_un_avion(9000.0, None),
            "sans durée, rien ne se conclut"
        );
        assert!(est_vol("flight") && !est_vol("car"));
    }

    #[test]
    fn la_trace_se_coupe_sur_un_saut_de_vitesse() {
        let t = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        let pts = vec![
            ((48.0, 2.0), t("2024-01-01T08:00:00Z")),
            ((48.1, 2.1), t("2024-01-01T08:10:00Z")),
            ((35.7, 139.7), t("2024-01-01T20:00:00Z")),
            ((35.8, 139.8), t("2024-01-01T20:10:00Z")),
        ];
        let p = decouper_la_trace(&pts);
        assert_eq!(
            p.iter().map(|p| p.vol).collect::<Vec<_>>(),
            vec![false, true, false]
        );
        assert!(p[1].points.len() > 10, "l'arc est densifié");
    }
}
