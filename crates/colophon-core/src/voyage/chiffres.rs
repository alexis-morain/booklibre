//! La page « Le voyage en chiffres » (P-s6, D11).
//!
//! Facultative, décochée par défaut, **dérivée** : le réglage est
//! `Album::chiffres`, la page est une planche `texte` posée en fin de livre
//! (avant le colophon, qui reste la dernière), et la composition la repose.
//! Une planche `texte` plutôt qu'un gabarit de plus : l'éditeur, le PDF et le
//! prévol la savent déjà, et une main qui la retouche la fait passer pour une
//! planche gardée.
//!
//! Les chiffres sont ceux que l'app **recompte**, jamais ceux que la source
//! affiche, sauf la distance (voir [`distance_km`]).

use super::composition::jour_local;
use super::Voyage;
use crate::carte::geometrie::haversine_km;
use crate::langue::Langue;
use crate::model::{Album, Spread};
use chrono::NaiveDate;

/// Le titre de la page, première ligne de son texte : c'est aussi ce qui la
/// fait reconnaître dans l'album, dans l'une ou l'autre langue.
pub fn titre(langue: Langue) -> &'static str {
    match langue {
        Langue::Fr => "Le voyage en chiffres",
        Langue::En => "The trip in numbers",
    }
}

/// Cette planche est-elle la page de chiffres ?
pub fn est_la_page(s: &Spread) -> bool {
    s.template == "texte"
        && s.slots.is_empty()
        && s.text
            .as_deref()
            .and_then(|t| t.lines().next())
            .is_some_and(|l| l == titre(Langue::Fr) || l == titre(Langue::En))
}

/// Les pays comptés sur les étapes : des codes à deux lettres, `00` exclu (ce
/// n'est pas un pays), chacun une fois. Jamais le compteur de la source, qui
/// vaut 0 sur un tour du monde.
pub fn pays(v: &Voyage) -> usize {
    let mut vus: Vec<String> = Vec::new();
    for c in v.etapes.iter().filter_map(|e| e.lieu.code_pays.as_deref()) {
        let c = c.trim().to_ascii_uppercase();
        if c.len() == 2 && c.chars().all(|x| x.is_ascii_alphabetic()) && !vus.contains(&c) {
            vus.push(c);
        }
    }
    vus.len()
}

/// La distance imprimée, en km, et d'où elle vient. **Le `total_km` de la
/// source quand il existe** : c'est le chiffre que le voyageur a vu, mesuré sur
/// sa trace, et il passe le trajet réel (une route n'est pas une droite). Sans
/// lui, la somme à vol d'oiseau (haversine) des étapes, qui n'en est qu'un
/// plancher : la page le dit alors (« au moins »).
pub fn distance_km(v: &Voyage) -> Option<(f64, bool)> {
    if let Some(d) = v.distance_km.filter(|d| d.is_finite() && *d > 0.0) {
        return Some((d, true));
    }
    let somme: f64 = v
        .etapes
        .windows(2)
        .map(|w| {
            haversine_km(
                (w[0].lieu.lat, w[0].lieu.lon),
                (w[1].lieu.lat, w[1].lieu.lon),
            )
        })
        .sum();
    (somme >= 1.0).then_some((somme, false))
}

/// Premier et dernier jour des étapes, dans leur fuseau.
fn jours(v: &Voyage) -> Option<(NaiveDate, NaiveDate)> {
    let tous: Vec<NaiveDate> = v.etapes.iter().filter_map(jour_local).collect();
    Some((*tous.iter().min()?, *tous.iter().max()?))
}

/// Les températures extrêmes des étapes qui en ont une.
fn extremes(v: &Voyage) -> Option<(i64, i64)> {
    let t: Vec<f64> = v
        .etapes
        .iter()
        .filter_map(|e| e.meteo.as_ref()?.temperature)
        .filter(|t| t.is_finite())
        .collect();
    let min = t.iter().copied().fold(f64::INFINITY, f64::min);
    let max = t.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (!t.is_empty()).then(|| (min.round() as i64, max.round() as i64))
}

/// Un entier par tranches de trois chiffres : « 73 451 » en français,
/// « 73,451 » en anglais. Une espace ordinaire : la face de l'album n'a pas
/// forcément l'insécable.
fn groupes(n: u64, langue: Langue) -> String {
    let brut = n.to_string();
    let sep = match langue {
        Langue::Fr => ' ',
        Langue::En => ',',
    };
    let mut sortie = String::new();
    for (i, c) in brut.chars().enumerate() {
        if i > 0 && (brut.len() - i) % 3 == 0 {
            sortie.push(sep);
        }
        sortie.push(c);
    }
    sortie
}

fn majuscule(t: &str) -> String {
    let mut c = t.chars();
    c.next()
        .map(|p| p.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Le texte de la page, une ligne par chiffre, rien d'inventé : une ligne dont
/// le chiffre manque (pas de date, pas de météo) n'est pas écrite.
pub fn texte(v: &Voyage, langue: Langue) -> String {
    let fr = langue == Langue::Fr;
    let mut lignes = vec![titre(langue).to_string(), String::new()];
    if let Some((d, f)) = jours(v) {
        if let Some(p) = langue.periode(Some(d), Some(f)) {
            lignes.push(majuscule(&p));
        }
        let n = (f - d).num_days() + 1;
        lignes.push(match (fr, n) {
            (true, 1) => "1 jour".into(),
            (true, n) => format!("{n} jours"),
            (false, 1) => "1 day".into(),
            (false, n) => format!("{n} days"),
        });
    }
    let e = v.etapes.len();
    if e > 0 {
        lignes.push(match (fr, e) {
            (true, 1) => "1 étape".into(),
            (true, n) => format!("{n} étapes"),
            (false, 1) => "1 step".into(),
            (false, n) => format!("{n} steps"),
        });
    }
    let p = pays(v);
    if p > 0 {
        lignes.push(match (fr, p) {
            (true, 1) => "1 pays".into(),
            (true, n) => format!("{n} pays"),
            (false, 1) => "1 country".into(),
            (false, n) => format!("{n} countries"),
        });
    }
    if let Some((km, de_la_source)) = distance_km(v) {
        let n = groupes(km.round() as u64, langue);
        lignes.push(match (fr, de_la_source) {
            (true, true) => format!("{n} km"),
            (true, false) => format!("au moins {n} km"),
            (false, true) => format!("{n} km"),
            (false, false) => format!("at least {n} km"),
        });
    }
    if let Some((min, max)) = extremes(v) {
        lignes.push(String::new());
        lignes.push(match (fr, min == max) {
            (true, true) => format!("Température relevée : {min} °C"),
            (true, false) => format!("Températures relevées : de {min} °C à {max} °C"),
            (false, true) => format!("Temperature recorded: {min} °C"),
            (false, false) => format!("Temperatures recorded: {min} °C to {max} °C"),
        });
    }
    lignes.join("\n")
}

/// La planche : du texte et rien d'autre, comme le colophon.
pub fn spread(v: &Voyage, langue: Langue) -> Spread {
    Spread {
        text: Some(texte(v, langue)),
        template: "texte".into(),
        ..Spread::vide()
    }
}

/// Pose la page en fin de livre, avant le colophon qui reste la dernière
/// planche, sauf si l'album en porte déjà une (la main l'a retouchée, elle
/// est gardée, donc elle est là). Rend vrai quand elle a été ajoutée.
pub fn poser(album: &mut Album, v: &Voyage, langue: Langue) -> bool {
    if album.spreads.iter().any(est_la_page) {
        return false;
    }
    let au_bout = album
        .spreads
        .last()
        .is_some_and(|s| s.template == crate::colophon::TEMPLATE);
    let rang = album.spreads.len() - usize::from(au_bout);
    album.spreads.insert(rang, spread(v, langue));
    true
}

/// Coche ou décoche la page sur un album **déjà composé** : le réglage et la
/// planche, sans rien recomposer. Décocher retire la page même retouchée (le
/// choix est celui de l'utilisateur). Sans `voyage.json` à côté des photos,
/// rien ne change et la réponse est `Ok(false)`.
pub fn regler(album: &mut Album, actif: bool, langue: Langue) -> bool {
    use super::composition::{lecture, Lecture};
    let Lecture::Voyage(v) = lecture(std::path::Path::new(&album.root)) else {
        return false;
    };
    album.spreads.retain(|s| !est_la_page(s));
    album.chiffres = actif;
    if actif {
        poser(album, &v, langue);
    }
    true
}

#[cfg(test)]
mod tests;
