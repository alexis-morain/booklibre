//! La carte d'un voyage synthétique (P-s4). Aucune donnée d'un tiers : des
//! étapes inventées, aux coordonnées de villes publiques.

use super::*;
use crate::voyage::{Etape, Lieu, PointDeTrace, Provenance, Segment as Trajet, Voyage, VERSION};
use chrono::{DateTime, Utc};

fn instant(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// Une étape par jour à partir du 1er mai 2024, à midi UTC.
fn etape(rang: usize, nom: &str, lat: f64, lon: f64) -> Etape {
    Etape {
        nom: nom.into(),
        recit: None,
        debut: instant(&format!("2024-05-{:02}T12:00:00Z", rang + 1)),
        fin: None,
        fuseau: "UTC".into(),
        lieu: Lieu { lat, lon, localite: None, pays: None, code_pays: None },
        meteo: None,
        photos: vec![],
    }
}

fn voyage(etapes: Vec<(&str, f64, f64)>, segments: Vec<Trajet>, trace: Vec<PointDeTrace>) -> Voyage {
    Voyage {
        version: VERSION,
        titre: "Test".into(),
        resume: None,
        debut: None,
        fin: None,
        fuseau: None,
        distance_km: None,
        pays: vec![],
        etapes: etapes.into_iter().enumerate().map(|(i, (n, la, lo))| etape(i, n, la, lo)).collect(),
        segments,
        trace,
        videos_ignorees: 0,
        couverture: None,
        provenance: Provenance { source: "test".into(), reference: None, format: None },
    }
}

fn vol(de: usize, a: usize) -> Trajet {
    Trajet { mode: Some("flight".into()), de: Some(de), a: Some(a), polyligne: vec![] }
}

/// USA, Fidji, Nouvelle-Zélande : le voyage passe l'antiméridien.
fn pacifique() -> Voyage {
    voyage(
        vec![("Los Angeles", 34.05, -118.24), ("Nadi", -17.80, 177.42), ("Auckland", -36.85, 174.76)],
        vec![vol(0, 1), vol(1, 2)],
        vec![],
    )
}

/// Quatre villes de France, d'un jour à l'autre, sans trajet : la suite des étapes.
fn france() -> Voyage {
    voyage(
        vec![("Paris", 48.857, 2.352), ("Lyon", 45.764, 4.836), ("Marseille", 43.296, 5.370), ("Bordeaux", 44.838, -0.579)],
        vec![],
        vec![],
    )
}

/// Trois points de Lisbonne, à quelques kilomètres l'un de l'autre.
fn lisbonne() -> Voyage {
    voyage(vec![("Belém", 38.697, -9.206), ("Alfama", 38.711, -9.130), ("Parc des Nations", 38.768, -9.094)], vec![], vec![])
}

/// Le tour du monde : la couture passe par la Russie, sur la route Paris-Tokyo.
fn tour_du_monde() -> Voyage {
    voyage(
        vec![("Paris", 48.857, 2.352), ("Tokyo", 35.68, 139.69), ("Nadi", -17.80, 177.42), ("Los Angeles", 34.05, -118.24), ("New York", 40.71, -74.0)],
        vec![],
        vec![],
    )
}

fn dans_le_cadre(d: &Dessin) {
    let (w, h) = (d.viewbox[2], d.viewbox[3]);
    for (x, y) in d.points() {
        assert!(x >= 0.0 && x <= w && y >= 0.0 && y <= h, "({x}, {y}) hors de {w} × {h}");
    }
}

fn plus_long_segment_de(d: &Dessin, role: Role) -> f64 {
    let mut plus = 0.0f64;
    for c in d.calque(role).iter().flat_map(|c| c.chemins.iter()) {
        let mut prec: Option<(f64, f64)> = None;
        for s in &c.segments {
            match *s {
                Segment::Vers { x, y } => prec = Some((x, y)),
                Segment::Ligne { x, y } => {
                    if let Some(p) = prec {
                        plus = plus.max((x - p.0).hypot(y - p.1));
                    }
                    prec = Some((x, y));
                }
                _ => {}
            }
        }
    }
    plus
}

#[test]
fn le_monde_qui_traverse_l_antimeridien() {
    let v = pacifique();
    let d = dessiner(&v, 0..3).unwrap();
    assert_eq!(d.niveau, Niveau::Monde);
    assert!(matches!(d.projection, Projection::EqualEarth { .. }));
    // Centré sur le Pacifique, pas sur Greenwich : la couture est dans l'Atlantique.
    let lon0 = d.projection.lon0();
    assert!(lon0 > 170.0 || lon0 < -110.0, "centre {lon0}");
    dans_le_cadre(&d);
    // Le vol Los Angeles - Nadi franchit les 180° : un seul trait continu qui
    // va toujours vers l'ouest, pas une ligne qui revient d'un bord de la carte
    // à l'autre.
    let vol = d.calque(Role::Vol).unwrap();
    assert_eq!(vol.chemins.len(), 1, "un seul tracé aérien, de Los Angeles à Auckland");
    let xs: Vec<f64> = vol.chemins[0].segments.iter().filter_map(|s| match *s {
        Segment::Vers { x, .. } | Segment::Ligne { x, .. } => Some(x),
        _ => None,
    }).collect();
    assert!(xs.len() > 3);
    assert!(xs.windows(2).all(|w| w[1] <= w[0] + 0.01), "le tracé revient en arrière : {xs:?}");
    assert_eq!(d.reperes.len(), 3);
    assert_eq!(d.empreinte(), dessiner(&v, 0..3).unwrap().empreinte());
    assert_eq!(d.empreinte(), "6bf935774a4ec8b3");
}

#[test]
fn un_pays() {
    let v = france();
    let d = dessiner(&v, 0..4).unwrap();
    assert_eq!(d.niveau, Niveau::Region);
    assert!(matches!(d.projection, Projection::Equirectangulaire { .. }));
    dans_le_cadre(&d);
    // Le fond 1:10m : terre, côtes et frontières (l'Espagne, la Suisse…) y sont.
    for r in [Role::Terre, Role::Cotes, Role::Frontieres] {
        assert!(d.calque(r).is_some_and(|c| !c.chemins.is_empty()), "{r:?} manque");
    }
    // Des étapes à un jour d'écart, à moins de 250 km/h : le sol, jamais un vol.
    assert!(d.calque(Role::Vol).is_none());
    assert_eq!(d.calque(Role::Trace).unwrap().chemins.len(), 1, "les trois jambes se recollent en un trait");
    assert_eq!(d.empreinte(), dessiner(&v, 0..4).unwrap().empreinte());
    assert_eq!(d.empreinte(), "c65d750d7c011b68");
}

#[test]
fn une_ville() {
    let v = lisbonne();
    let d = dessiner(&v, 0..3).unwrap();
    assert_eq!(d.niveau, Niveau::Ville);
    assert!(matches!(d.projection, Projection::Mercator { .. }));
    dans_le_cadre(&d);
    assert!(d.calque(Role::Terre).is_some());
    assert!(d.viewbox[2].max(d.viewbox[3]) == COTE_MM);
    assert_eq!(d.empreinte(), dessiner(&v, 0..3).unwrap().empreinte());
    assert_eq!(d.empreinte(), "a02cc706e997db4e");
}

#[test]
fn une_seule_etape_donne_une_carte_d_au_moins_trente_kilometres() {
    let v = voyage(vec![("Lisbonne", 38.711, -9.13)], vec![], vec![]);
    let d = dessiner(&v, 0..1).unwrap();
    assert_eq!(d.niveau, Niveau::Ville);
    assert_eq!(d.reperes.len(), 1);
    // Un repère au milieu, et un fond qui n'est pas vide.
    assert!((d.reperes[0].x - d.viewbox[2] / 2.0).abs() < 0.01 && (d.reperes[0].y - d.viewbox[3] / 2.0).abs() < 0.01);
    assert!(d.calque(Role::Terre).is_some());
    dans_le_cadre(&d);
}

#[test]
fn la_plage_choisit_les_etapes_et_une_plage_vide_se_refuse() {
    let v = tour_du_monde();
    let d = dessiner(&v, 1..3).unwrap();
    assert_eq!(d.reperes.iter().map(|r| r.etape).collect::<Vec<_>>(), vec![1, 2]);
    // Une plage qui dépasse le voyage est rognée à ses étapes.
    assert_eq!(dessiner(&v, 3..99).unwrap().reperes.len(), 2);
    assert_eq!(dessiner(&v, 5..9).unwrap_err(), Erreur::PlageVide);
    assert_eq!(dessiner(&v, 2..2).unwrap_err(), Erreur::PlageVide);
    // Le dessin d'une sous-plage n'est pas celui du tout.
    assert_ne!(d.empreinte(), dessiner(&v, 0..5).unwrap().empreinte());
}

#[test]
fn le_vol_suit_le_grand_cercle() {
    // Paris - Tokyo : le grand cercle monte vers la Sibérie. Une corde droite
    // entre les deux villes ne passerait jamais au nord des deux.
    let v = voyage(vec![("Paris", 48.857, 2.352), ("Tokyo", 35.68, 139.69)], vec![vol(0, 1)], vec![]);
    let d = dessiner(&v, 0..2).unwrap();
    assert_eq!(d.niveau, Niveau::Monde);
    let haut = d.calque(Role::Vol).unwrap().chemins.iter().flat_map(|c| c.segments.iter()).filter_map(|s| match *s {
        Segment::Vers { y, .. } | Segment::Ligne { y, .. } => Some(y),
        _ => None,
    }).fold(f64::MAX, f64::min);
    let villes = d.reperes.iter().map(|r| r.y).fold(f64::MAX, f64::min);
    assert!(villes - haut > 5.0, "l'arc monte de {} mm au-dessus de la plus septentrionale des deux villes", villes - haut);
}

#[test]
fn sans_mode_la_vitesse_fait_l_avion() {
    // Un jour entre Paris et Tokyo : 400 km/h. Un jour entre Paris et Lyon : 20 km/h.
    let v = voyage(vec![("Paris", 48.857, 2.352), ("Tokyo", 35.68, 139.69), ("Kyoto", 35.01, 135.77)], vec![], vec![]);
    let d = dessiner(&v, 0..3).unwrap();
    assert!(d.calque(Role::Vol).is_some(), "Paris-Tokyo en un jour est un vol");
    assert!(d.calque(Role::Trace).is_some(), "Tokyo-Kyoto est un trajet au sol");
}

#[test]
fn une_couture_qui_traverse_le_trace_le_coupe_et_le_ramene() {
    let v = tour_du_monde();
    let d = dessiner(&v, 0..5).unwrap();
    assert_eq!(d.niveau, Niveau::Monde);
    // Le centre est Los Angeles : la couture passe par 62° E, sur la route Paris-Tokyo.
    assert!((d.projection.lon0() - -118.24).abs() < 1e-9, "{}", d.projection.lon0());
    dans_le_cadre(&d);
    assert!(plus_long_segment_de(&d, Role::Vol) < 0.2 * d.viewbox[2], "un trait a traversé la couture : {}", plus_long_segment_de(&d, Role::Vol));
    assert_eq!(d.calque(Role::Vol).unwrap().chemins.len(), 2, "Paris-couture, puis couture-Tokyo-Nadi-Los Angeles");
}

#[test]
fn les_trajets_de_la_source_gardent_leur_mode_et_la_plage_les_trie() {
    let pl = |a: (f64, f64), b: (f64, f64)| vec![crate::voyage::Point { lat: a.0, lon: a.1 }, crate::voyage::Point { lat: b.0, lon: b.1 }];
    let v = voyage(
        vec![("Paris", 48.857, 2.352), ("Lyon", 45.764, 4.836), ("Genève", 46.204, 6.143), ("Tokyo", 35.68, 139.69)],
        vec![
            Trajet { mode: Some("car".into()), de: Some(0), a: Some(1), polyligne: pl((48.857, 2.352), (45.764, 4.836)) },
            // Sans `de` ni `a` : l'étape la plus proche de chaque bout les donne.
            Trajet { mode: None, de: None, a: None, polyligne: pl((45.77, 4.84), (46.2, 6.14)) },
            Trajet { mode: Some("flight".into()), de: Some(2), a: Some(3), polyligne: pl((46.204, 6.143), (35.68, 139.69)) },
        ],
        vec![],
    );
    // Les trois premières étapes : deux trajets au sol, le vol vers Tokyo est hors plage.
    let d = dessiner(&v, 0..3).unwrap();
    assert!(d.calque(Role::Vol).is_none(), "le vol n'est pas de cette plage");
    assert!(d.calque(Role::Trace).is_some());
    // Tout le voyage : le vol en est.
    assert!(dessiner(&v, 0..4).unwrap().calque(Role::Vol).is_some());
}

#[test]
fn la_trace_ne_sert_que_sans_trajet_et_se_coupe_sur_la_fenetre_de_la_plage() {
    let t = |h: u32, lat: f64, lon: f64| PointDeTrace { lat, lon, instant: instant(&format!("2024-05-01T{h:02}:00:00Z")) };
    let trace = vec![t(8, 48.0, 2.0), t(9, 48.2, 2.2), t(10, 48.4, 2.4), t(11, 48.6, 2.6)];
    let mut v = voyage(vec![("Départ", 48.0, 2.0), ("Arrivée", 48.6, 2.6)], vec![], trace);
    v.etapes[0].debut = instant("2024-05-01T08:00:00Z");
    v.etapes[1].debut = instant("2024-05-01T11:00:00Z");
    let d = dessiner(&v, 0..2).unwrap();
    // Quatre points de trace alignés se réduisent à deux sommets.
    assert_eq!(d.sommets_du_trace(), 2);
    assert_eq!(d.calque(Role::Trace).unwrap().chemins.len(), 1);
}

#[test]
fn un_trace_de_milliers_de_points_tient_sous_le_plafond() {
    // Une marche au hasard (congruentielle, donc la même partout) de 5 000 points.
    let mut graine = 12345u64;
    let mut alea = || {
        graine = graine.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((graine >> 33) as f64 / (1u64 << 31) as f64) - 0.5
    };
    let (mut lat, mut lon) = (45.0, 5.0);
    let mut trace = Vec::new();
    for i in 0..5000u32 {
        lat += alea() * 0.04;
        lon += alea() * 0.04 + 0.0004;
        trace.push(PointDeTrace { lat, lon, instant: instant("2024-05-01T00:00:00Z") + chrono::Duration::seconds(i as i64 * 60) });
    }
    let mut v = voyage(vec![("A", 45.0, 5.0), ("B", lat, lon)], vec![], trace);
    v.etapes[1].debut = instant("2024-06-01T00:00:00Z");
    let d = dessiner(&v, 0..2).unwrap();
    let n = d.sommets_du_trace();
    assert!(n <= SOMMETS_MAX, "{n} sommets");
    assert!(n > 100, "le tracé n'est pas écrasé : {n} sommets");
    dans_le_cadre(&d);
    // Et sans le plafond, ce tracé n'y tiendrait pas : la tolérance seule en laisse plus.
    let seul = trace_simplifie_sans_plafond(&v);
    assert!(seul > SOMMETS_MAX, "le plafond ne sert à rien ici : {seul}");
}

/// Les sommets du tracé de ce voyage à la seule tolérance de 0,2 mm.
fn trace_simplifie_sans_plafond(v: &Voyage) -> usize {
    let d = dessiner(v, 0..v.etapes.len()).unwrap();
    // Le cadre du dessin plafonné est celui du dessin libre : on reprend les
    // points projetés tels que `dessiner` les a posés, en retirant la boucle.
    let pieces = trace::pieces(v, &(0..v.etapes.len()));
    let lon0 = longitude_mediane(&v.etapes.iter().map(|e| e.lieu.lon).collect::<Vec<_>>());
    let proj = d.projection;
    let _ = lon0;
    let pts: Vec<P> = pieces[0].points.iter().map(|&(la, lo)| proj.projeter(envelopper(lo - proj.lon0()), la)).collect();
    // Échelle du dessin : 180 mm sur le grand côté de la boîte de ses points.
    let (x0, x1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (y0, y1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let echelle = COTE_MM / ((x1 - x0).max(y1 - y0) * (1.0 + 2.0 * MARGE));
    let mm: Vec<P> = pts.iter().map(|p| (p.0 * echelle, p.1 * echelle)).collect();
    douglas_peucker(&mm, TOLERANCE_TRACE_MM).len()
}

/// La plus grande distance entre deux repères, en mm.
fn ecart_des_reperes(d: &Dessin) -> f64 {
    let mut m = 0.0f64;
    for a in &d.reperes {
        for b in &d.reperes {
            m = m.max((a.x - b.x).hypot(a.y - b.y));
        }
    }
    m
}

#[test]
fn un_trajet_au_sol_par_dessus_l_antimeridien_reste_court() {
    // Nadi et Apia, à 1 100 km de part et d'autre des 180°, un jour d'écart : un trajet
    // au sol. Si la corde faisait le tour du monde, la boîte s'étendrait à 350° et les
    // deux repères se toucheraient.
    let v = voyage(vec![("Nadi", -17.80, 177.42), ("Apia", -13.83, -171.76)], vec![], vec![]);
    let d = dessiner(&v, 0..2).unwrap();
    assert_eq!(d.niveau, Niveau::Region);
    assert!(ecart_des_reperes(&d) > 0.6 * COTE_MM, "repères à {} mm : le tracé a fait le tour du monde", ecart_des_reperes(&d));
    assert_eq!(d.calque(Role::Trace).unwrap().chemins.len(), 1);
}

#[test]
fn une_polyligne_de_vol_a_plusieurs_points_passe_l_antimeridien_sans_saut() {
    let p = |lat: f64, lon: f64| crate::voyage::Point { lat, lon };
    let v = voyage(
        vec![("Nadi", -17.80, 177.42), ("Apia", -13.83, -171.76), ("Tongatapu", -21.14, -175.20)],
        vec![Trajet { mode: Some("flight".into()), de: Some(0), a: Some(2), polyligne: vec![p(-17.80, 177.42), p(-13.83, -171.76), p(-21.14, -175.20)] }],
        vec![],
    );
    let d = dessiner(&v, 0..3).unwrap();
    assert_eq!(d.niveau, Niveau::Region);
    assert!(ecart_des_reperes(&d) > 0.6 * COTE_MM, "repères à {} mm", ecart_des_reperes(&d));
    assert_eq!(d.calque(Role::Vol).unwrap().chemins.len(), 1, "un seul tracé aérien continu");
}

#[test]
fn la_longitude_mediane_est_circulaire() {
    assert_eq!(longitude_mediane(&[10.0, 20.0, 30.0]), 20.0);
    assert_eq!(longitude_mediane(&[42.0]), 42.0);
    let c = longitude_mediane(&[179.0, -179.0]);
    assert!(c == 179.0 || c == -179.0, "{c}");
    // Le Pacifique : la médiane est dans le Pacifique, pas à Greenwich.
    assert!(longitude_mediane(&[-118.0, 177.0, 175.0]) > 170.0);
    // Égalité : la plus petite.
    assert_eq!(longitude_mediane(&[0.0, 90.0]), 0.0);
}

#[test]
fn le_cadre_garde_la_marge_et_le_rapport() {
    let d = dessiner(&france(), 0..4).unwrap();
    let (w, h) = (d.viewbox[2], d.viewbox[3]);
    assert!((w.max(h) - COTE_MM).abs() < 0.01);
    assert!(w.max(h) / w.min(h) <= RAPPORT_MAX + 1e-6);
    // Les repères ne touchent pas les bords : la marge est de 10 %.
    for r in &d.reperes {
        assert!(r.x > 0.05 * w && r.x < 0.95 * w && r.y > 0.05 * h && r.y < 0.95 * h, "{r:?} dans {w} × {h}");
    }
}

#[test]
fn le_dessin_se_serialise() {
    let d = dessiner(&lisbonne(), 0..3).unwrap();
    let j = serde_json::to_value(&d).unwrap();
    assert_eq!(j["niveau"], "ville");
    assert_eq!(j["projection"]["projection"], "mercator");
    assert!(j["calques"][0]["chemins"].is_array());
}

// ── l'actif ────────────────────────────────────────────────────────────────

#[test]
fn l_actif_embarque_est_lisible_et_sous_le_plafond() {
    assert!(ACTIF.len() < 6_000_000, "{} octets : le plafond du plan est de 6 Mo", ACTIF.len());
    let a = Actif::lire(ACTIF).unwrap();
    for c in Couche::TOUTES {
        let n = a.nombre_de_parties(c);
        assert!(n > 300, "{c:?} : {n} parties");
        let mut vues = 0;
        for p in a.parties(c) {
            let p = p.unwrap();
            let pts = p.points().unwrap();
            assert!(pts.len() >= if c.est_surface() { 3 } else { 2 });
            assert!(pts.iter().all(|&(x, y)| (-180.0..=180.0).contains(&x) && (-90.0..=90.0).contains(&y)));
            vues += 1;
        }
        assert_eq!(vues, n);
    }
}

#[test]
fn l_actif_correspond_a_sa_provenance() {
    let source = include_str!("../../assets/carte/SOURCE.md");
    let attendue = source.lines().find_map(|l| l.split("`").nth(1).filter(|_| l.contains("Empreinte"))).expect("SOURCE.md porte l'empreinte");
    assert_eq!(compilation::empreinte(ACTIF), attendue, "carte.bin a changé sans que SOURCE.md le dise : scripts/carte.sh compiler");
    for mot in ["domaine public", "Natural Earth", "ne_10m_coastline", "SHA-256"] {
        assert!(source.contains(mot), "{mot}");
    }
}

#[test]
#[ignore = "recompile Natural Earth (hors dépôt) : scripts/carte.sh verifier le fait dans le gate local"]
fn l_actif_se_recompile_a_l_identique() {
    let dossier = std::env::var("COLOPHON_NATURAL_EARTH").unwrap_or_else(|_| {
        format!("{}/Documents/08_IA/02_Outputs/colophon/travail/natural-earth", std::env::var("HOME").unwrap())
    });
    let lire = |n: &str| std::fs::read(format!("{dossier}/{n}/{n}.shp")).unwrap();
    let (a, b, c, d, e, f) = (
        lire("ne_50m_land"),
        lire("ne_50m_admin_0_countries"),
        lire("ne_10m_admin_0_countries"),
        lire("ne_10m_coastline"),
        lire("ne_10m_lakes"),
        lire("ne_10m_rivers_lake_centerlines"),
    );
    let src = compilation::Sources { land50: &a, countries50: &b, countries10: &c, coastline10: &d, lakes10: &e, rivers10: &f };
    let une = compilation::compiler(&src, &compilation::Reglages::default()).unwrap();
    assert!(une == ACTIF, "l'actif versionné n'est pas ce que ce code compile");
    assert!(une == compilation::compiler(&src, &compilation::Reglages::default()).unwrap(), "deux compilations diffèrent");
}

// ── le rendu, pour les yeux ────────────────────────────────────────────────

fn svg(d: &Dessin) -> String {
    let (w, h) = (d.viewbox[2], d.viewbox[3]);
    let mut s = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {w} {h}' width='{}' height='{}'>\n<rect width='{w}' height='{h}' fill='#cfe3ee'/>\n", w * 6.0, h * 6.0);
    for c in &d.calques {
        let mut dd = String::new();
        for ch in &c.chemins {
            for sg in &ch.segments {
                match *sg {
                    Segment::Vers { x, y } => dd += &format!("M{x} {y}"),
                    Segment::Ligne { x, y } => dd += &format!("L{x} {y}"),
                    Segment::Ferme => dd += "Z",
                    Segment::Courbe { .. } => {}
                }
            }
        }
        let attrs = match (c.role, c.style) {
            (Role::Terre, _) => "fill='#f1ead8' stroke='none'".to_string(),
            (Role::Lacs, _) => "fill='#cfe3ee' stroke='none'".to_string(),
            (Role::Rivieres, Style::Tracer { epaisseur_mm, .. }) => format!("fill='none' stroke='#8fb4cc' stroke-width='{epaisseur_mm}'"),
            (Role::Frontieres, Style::Tracer { epaisseur_mm, .. }) => format!("fill='none' stroke='#b9a98f' stroke-width='{epaisseur_mm}'"),
            (Role::Cotes, Style::Tracer { epaisseur_mm, .. }) => format!("fill='none' stroke='#6b7f8c' stroke-width='{epaisseur_mm}'"),
            (Role::Trace, Style::Tracer { epaisseur_mm, .. }) => format!("fill='none' stroke='#c0392b' stroke-width='{epaisseur_mm}' stroke-linejoin='round'"),
            (Role::Vol, Style::Tracer { epaisseur_mm, pointille }) => {
                let p = pointille.unwrap_or([1.0, 1.0]);
                format!("fill='none' stroke='#c0392b' stroke-width='{epaisseur_mm}' stroke-dasharray='{} {}'", p[0], p[1])
            }
            _ => "fill='none' stroke='black'".to_string(),
        };
        s += &format!("<path d='{dd}' {attrs}/>\n");
    }
    for r in &d.reperes {
        s += &format!("<circle cx='{}' cy='{}' r='1.1' fill='#222' stroke='white' stroke-width='0.3'/>\n", r.x, r.y);
    }
    s + "</svg>\n"
}

fn ecrire_svg(nom: &str, d: &Dessin) {
    if let Ok(dir) = std::env::var("COLOPHON_CARTE_RENDU") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{nom}.svg"), svg(d)).unwrap();
    }
}

#[test]
#[ignore = "écrit les SVG des cas dans $COLOPHON_CARTE_RENDU, pour les yeux"]
fn le_rendu_des_cas() {
    ecrire_svg("1-monde-antimeridien", &dessiner(&pacifique(), 0..3).unwrap());
    ecrire_svg("2-pays-france", &dessiner(&france(), 0..4).unwrap());
    ecrire_svg("3-ville-lisbonne", &dessiner(&lisbonne(), 0..3).unwrap());
    ecrire_svg("4-tour-du-monde-couture", &dessiner(&tour_du_monde(), 0..5).unwrap());
}

#[test]
#[ignore = "le voyage réel d'Alexis (hors dépôt) : $COLOPHON_VOYAGE_REEL"]
fn le_voyage_reel() {
    let dossier = std::path::PathBuf::from(std::env::var("COLOPHON_VOYAGE_REEL").expect("COLOPHON_VOYAGE_REEL"));
    let v = crate::voyage::lire(&dossier).unwrap();
    let t0 = std::time::Instant::now();
    let d = dessiner(&v, 0..v.etapes.len()).unwrap();
    eprintln!("tout le voyage : {:?}, niveau {:?}, {} sommets de tracé, {} × {} mm, empreinte {}", t0.elapsed(), d.niveau, d.sommets_du_trace(), d.viewbox[2], d.viewbox[3], d.empreinte());
    dans_le_cadre(&d);
    assert!(d.sommets_du_trace() <= SOMMETS_MAX);
    ecrire_svg("5-voyage-reel-entier", &d);
    // Sous-plages : un pays à la fois, comme P-s5 le demandera.
    let mut debut = 0;
    while debut < v.etapes.len() {
        let code = v.etapes[debut].lieu.code_pays.clone();
        let mut fin = debut + 1;
        while fin < v.etapes.len() && v.etapes[fin].lieu.code_pays == code {
            fin += 1;
        }
        let t = std::time::Instant::now();
        let p = dessiner(&v, debut..fin).unwrap();
        eprintln!("{:?} {debut}..{fin} : {:?} {:?}, {} sommets, {:?}", code, p.niveau, p.viewbox, p.sommets_du_trace(), t.elapsed());
        dans_le_cadre(&p);
        if fin - debut >= 3 {
            ecrire_svg(&format!("6-pays-{}-{debut}", code.unwrap_or_default()), &p);
        }
        debut = fin;
    }
}
