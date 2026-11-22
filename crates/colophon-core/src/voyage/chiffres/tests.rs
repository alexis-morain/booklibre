//! La page de chiffres sur un voyage synthétique, en mémoire : aucune photo.

use super::*;
use crate::voyage::{ecrire, Etape, Lieu, Meteo, Provenance, VERSION};
use chrono::{DateTime, Utc};

fn instant(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn etape(nom: &str, jour: &str, code: Option<&str>, lat: f64, lon: f64, t: Option<f64>) -> Etape {
    Etape {
        nom: nom.into(),
        recit: None,
        debut: instant(&format!("{jour}T09:30:00Z")),
        fin: None,
        fuseau: "Europe/Paris".into(),
        lieu: Lieu {
            lat,
            lon,
            localite: Some(nom.into()),
            pays: None,
            code_pays: code.map(String::from),
        },
        meteo: t.map(|t| Meteo {
            condition: Some("rain".into()),
            temperature: Some(t),
        }),
        photos: vec![],
    }
}

/// Quatre étapes, du 1er au 10 juin. Les pays : `FR` deux fois (dont une en
/// minuscules), `00` (pas un pays), un code absent et `DE` : **deux** pays,
/// alors que la source en déclare 9.
fn voyage() -> Voyage {
    Voyage {
        version: VERSION,
        titre: "Un voyage".into(),
        resume: None,
        debut: None,
        fin: None,
        fuseau: None,
        distance_km: Some(1234.5),
        pays: vec!["A".into(); 9],
        etapes: vec![
            etape("Paris", "2025-06-01", Some("FR"), 48.85, 2.35, Some(31.4)),
            etape("Lyon", "2025-06-03", Some("fr"), 45.76, 4.84, Some(3.0)),
            etape("Mer", "2025-06-05", Some("00"), 40.0, 5.0, None),
            etape("Sans code", "2025-06-07", None, 41.0, 6.0, None),
            etape("Berlin", "2025-06-10", Some("DE"), 52.52, 13.4, Some(12.6)),
        ],
        segments: vec![],
        trace: vec![],
        videos_ignorees: 0,
        couverture: None,
        provenance: Provenance {
            source: "test".into(),
            reference: None,
            format: None,
        },
    }
}

#[test]
fn les_chiffres_sont_justes_et_les_pays_recomptes() {
    let v = voyage();
    assert_eq!(pays(&v), 2, "FR, fr, 00, rien, DE");
    let t = texte(&v, Langue::Fr);
    assert_eq!(
        t,
        "Le voyage en chiffres\n\nDu 1 au 10 juin 2025\n10 jours\n5 étapes\n2 pays\n1 235 km\n\nTempératures relevées : de 3 °C à 31 °C"
    );
}

#[test]
fn la_distance_vient_de_la_source_sinon_de_la_somme_des_etapes() {
    let mut v = voyage();
    assert_eq!(distance_km(&v), Some((1234.5, true)));
    v.distance_km = None;
    let (km, de_la_source) = distance_km(&v).unwrap();
    assert!(!de_la_source);
    // Paris-Lyon-mer-… à vol d'oiseau : Paris-Lyon seul fait déjà 390 km.
    assert!(km > 1500.0 && km < 4000.0, "{km}");
    assert!(texte(&v, Langue::Fr).contains(&format!(
        "au moins {} km",
        groupes(km.round() as u64, Langue::Fr)
    )));
    // Zéro n'est pas une distance : la source qui dit 0 laisse la somme parler.
    v.distance_km = Some(0.0);
    assert!(!distance_km(&v).unwrap().1);
}

#[test]
fn en_anglais_les_mots_et_les_milliers_changent() {
    let t = texte(&voyage(), Langue::En);
    assert!(t.starts_with("The trip in numbers\n\nFrom 1 to 10 June 2025\n10 days\n5 steps\n2 countries\n1,235 km"), "{t}");
    assert!(t.ends_with("Temperatures recorded: 3 °C to 31 °C"), "{t}");
}

#[test]
fn une_ligne_sans_son_chiffre_n_est_pas_ecrite() {
    let mut v = voyage();
    v.etapes.truncate(1);
    v.etapes[0].meteo = None;
    v.etapes[0].lieu.code_pays = None;
    v.distance_km = None;
    let t = texte(&v, Langue::Fr);
    assert_eq!(
        t, "Le voyage en chiffres\n\nLe 1 juin 2025\n1 jour\n1 étape",
        "{t}"
    );
    // Une seule température : « Température relevée », pas « de 3 à 3 ».
    v.etapes[0].meteo = Some(Meteo {
        condition: None,
        temperature: Some(3.0),
    });
    assert!(texte(&v, Langue::Fr).ends_with("Température relevée : 3 °C"));
}

fn album_du_voyage(nom: &str) -> Album {
    let d = std::env::temp_dir().join(format!("booklibre-chiffres-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    ecrire(&d, &voyage()).unwrap();
    let mut a = Album::new("t", &d, crate::model::Size { w: 210.0, h: 210.0 });
    for t in ["vide", "colophon"] {
        a.spreads.push(Spread {
            template: t.into(),
            ..Spread::vide()
        });
    }
    a
}

#[test]
fn la_page_se_coche_se_decoche_et_le_colophon_reste_le_dernier() {
    let mut a = album_du_voyage("coche");
    assert!(
        !a.chiffres && !a.spreads.iter().any(est_la_page),
        "décochée par défaut"
    );

    assert!(regler(&mut a, true, Langue::Fr));
    assert!(a.chiffres);
    let rangs: Vec<usize> = a
        .spreads
        .iter()
        .enumerate()
        .filter(|(_, s)| est_la_page(s))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(rangs, vec![1], "une seule page, avant le colophon");
    assert_eq!(
        a.spreads.last().unwrap().template,
        crate::colophon::TEMPLATE
    );

    // Cocher deux fois ne la double pas ; la changer de langue la récrit.
    regler(&mut a, true, Langue::En);
    assert_eq!(a.spreads.iter().filter(|s| est_la_page(s)).count(), 1);
    assert!(a.spreads[1]
        .text
        .as_deref()
        .unwrap()
        .starts_with("The trip in numbers"));

    assert!(regler(&mut a, false, Langue::Fr));
    assert!(!a.chiffres && !a.spreads.iter().any(est_la_page));
    assert_eq!(a.spreads.len(), 2);

    // L'album s'écrit sans trace du réglage quand il est décoché.
    assert!(!serde_json::to_string(&a).unwrap().contains("\"chiffres\""));
}

#[test]
fn sans_voyage_json_rien_ne_change() {
    let d = std::env::temp_dir().join(format!("booklibre-chiffres-sans-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut a = Album::new("t", &d, crate::model::Size { w: 210.0, h: 210.0 });
    a.spreads.push(Spread {
        template: "colophon".into(),
        ..Spread::vide()
    });
    assert!(!regler(&mut a, true, Langue::Fr));
    assert!(!a.chiffres && a.spreads.len() == 1);
}
