//! Les tests du lecteur du lien. Aucun ne touche le réseau : un faux
//! transport rend des réponses écrites à la main (fixture synthétique, D14).

use super::*;
use serde_json::json;
use std::cell::RefCell;
use std::collections::HashMap;

const V62: &str = include_str!("../fixtures/synthetique-v62.json");

fn fixture() -> Value {
    serde_json::from_str(V62).unwrap()
}

fn jpeg(l: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(l, h, image::Rgb([120, 80, 40]));
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
    out.into_inner()
}

/// Un transport qui rend ce qu'on lui a posé et note chaque appel.
struct Faux {
    reponses: HashMap<String, (u16, Vec<u8>)>,
    appels: RefCell<Vec<String>>,
}

impl Faux {
    fn avec_voyage(json: &Value) -> Self {
        let mut f = Faux { reponses: HashMap::new(), appels: RefCell::new(Vec::new()) };
        for s in json["steps"].as_array().into_iter().flatten() {
            for m in s["media"].as_array().into_iter().flatten() {
                if m["type"] == 0 {
                    f.reponses.insert(m["large_thumbnail_path"].as_str().unwrap().to_string(), (200, jpeg(12, 8)));
                }
            }
        }
        f
    }
    fn api(mut self, id: u64, jeton: Option<&str>, statut: u16, corps: Vec<u8>) -> Self {
        self.reponses.insert(url_api(id, jeton), (statut, corps));
        self
    }
}

fn url_api(id: u64, jeton: Option<&str>) -> String {
    let mut u = format!("{API}/trips/{id}?polarsteps_api_version={VERSION_API}");
    if let Some(j) = jeton {
        u.push_str(&format!("&s={j}"));
    }
    u
}

impl Transport for Faux {
    fn get(&self, url: &str, _plafond: u64) -> Result<Reponse, String> {
        self.appels.borrow_mut().push(url.to_string());
        match self.reponses.get(url) {
            Some((statut, corps)) => Ok(Reponse { statut: *statut, corps: corps.clone() }),
            None => Err("pas de route".into()),
        }
    }
}

fn dossier(nom: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("booklibre-voyage-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn reglages() -> Reglages {
    Reglages { limites: Limites { reponse: 1 << 20, etapes: 100, photos: 100, photo: 1 << 20, total: 1 << 24, trace: 1000 }, pause: Duration::ZERO }
}

fn format_attendu<T: std::fmt::Debug>(r: Result<T, Erreur>) -> String {
    match r {
        Err(Erreur::Format(m)) => m,
        autre => panic!("attendu Erreur::Format, reçu {autre:?}"),
    }
}

// ---- l'adresse ---------------------------------------------------------------

#[test]
fn l_adresse_se_lit_avec_et_sans_jeton() {
    let a = analyser_url("https://www.polarsteps.com/quelquun/11306874-clark-family-2024").unwrap();
    assert_eq!(a, Lien { id: 11306874, jeton: None });
    let b = analyser_url("https://polarsteps.com/quelquun/4242-un-voyage?s=ab12-CD_34").unwrap();
    assert_eq!(b, Lien { id: 4242, jeton: Some("ab12-CD_34".into()) });
    let c = analyser_url("  https://www.polarsteps.com/q/4242?utm=x&s=zz9  ").unwrap();
    assert_eq!(c, Lien { id: 4242, jeton: Some("zz9".into()) });
}

#[test]
fn l_adresse_refuse_ce_qui_n_est_pas_un_voyage_polarsteps() {
    for mauvaise in [
        "http://www.polarsteps.com/q/4242-v",
        "https://www.polarsteps.com.evil.example/q/4242-v",
        "https://evil.example/www.polarsteps.com/q/4242-v",
        "https://user@www.polarsteps.com/q/4242-v",
        "https://www.polarsteps.com:8443/q/4242-v",
        "https://www.polarsteps.com/q",
        "https://www.polarsteps.com/q/abc-v",
        "https://www.polarsteps.com/q/4242-v?s=a b",
        "https://www.polarsteps.com/q/4242-v?s=%0d%0aX",
        "https://www.polarsteps.com/q/4242-v?s=",
        "https://www.polarsteps.com/q/99999999999999999999-v",
        "polarsteps.com/q/4242-v",
        "",
    ] {
        assert!(matches!(analyser_url(mauvaise), Err(Erreur::Url(_))), "{mauvaise:?} aurait dû être refusée");
    }
}

#[test]
fn l_agent_dit_qui_l_envoie() {
    let ua = agent_utilisateur();
    assert_eq!(ua, format!("Booklibre/{} (+https://booklibre.app)", env!("CARGO_PKG_VERSION")));
}

// ---- la conversion -----------------------------------------------------------

#[test]
fn la_conversion_v62_remplit_le_modele() {
    let c = convertir(&fixture(), 4242, &limites_larges()).unwrap();
    let v = c.voyage;
    assert_eq!(v.version, 1);
    assert_eq!(v.titre, "Voyage synthétique");
    assert_eq!(v.resume.as_deref(), Some("Trois étapes écrites à la main 🌍"));
    assert_eq!(v.fuseau.as_deref(), Some("Europe/Paris"));
    assert_eq!(v.distance_km, Some(3210.5));
    assert_eq!(v.provenance.source, "polarsteps-lien");
    assert_eq!(v.provenance.reference.as_deref(), Some("4242"));
    assert_eq!(v.provenance.format.as_deref(), Some("api-62"));
    assert_eq!(v.etapes.len(), 3);

    let lyon = &v.etapes[0];
    assert_eq!(lyon.nom, "Lyon");
    assert_eq!(lyon.recit.as_deref(), Some("Départ 🎒 par la gare de Perrache."), "les émojis restent dans voyage.json");
    assert_eq!(lyon.fuseau, "Europe/Paris");
    assert_eq!(lyon.lieu.code_pays.as_deref(), Some("FR"));
    assert_eq!(lyon.lieu.localite.as_deref(), Some("Lyon"));
    assert_eq!(lyon.meteo.as_ref().unwrap().condition.as_deref(), Some("rain"));
    assert_eq!(lyon.meteo.as_ref().unwrap().temperature, Some(12.5));
    assert_eq!(v.videos_ignorees, 1);

    // Le nom choisi par la personne l'emporte sur le nom d'affichage.
    assert_eq!(v.etapes[1].nom, "Arrivée à Tunis");
    // Une condition inconnue est gardée telle quelle ; la température absente
    // ne devient pas zéro.
    let tunis = v.etapes[1].meteo.as_ref().unwrap();
    assert_eq!(tunis.condition.as_deref(), Some("tornado-of-frogs"));
    assert_eq!(tunis.temperature, None);
    // Ni condition ni température : pas de météo du tout.
    assert!(v.etapes[2].meteo.is_none());
    assert!(v.etapes[2].recit.is_none());
}

#[test]
fn les_photos_suivent_l_ordre_du_media_et_les_noms_sont_stables() {
    let c = convertir(&fixture(), 4242, &limites_larges()).unwrap();
    let lyon = &c.voyage.etapes[0];
    let noms: Vec<&str> = lyon.photos.iter().map(|p| p.fichier.as_str()).collect();
    // `order` 0 puis 1, alors que la réponse les donne à l'envers.
    assert_eq!(noms, ["e001-p01-aaaaaaaa.jpg", "e001-p02-bbbbbbbb.jpg"]);
    assert_eq!(c.voyage.etapes[1].photos[0].fichier, "e002-p01-dddddddd.jpg");
    assert_eq!(c.a_prendre.len(), 3, "la vidéo n'est pas à prendre");
    assert!(c.a_prendre[0].url.ends_with("/a1.jpg"));
    // Deux conversions, les mêmes noms.
    let bis = convertir(&fixture(), 4242, &limites_larges()).unwrap();
    assert_eq!(bis.voyage.etapes[0].photos, lyon.photos);
}

#[test]
fn un_uuid_de_media_ne_sort_jamais_du_dossier() {
    let mut j = fixture();
    j["steps"][0]["media"][1]["uuid"] = json!("../../etc/passwd");
    let c = convertir(&j, 4242, &limites_larges()).unwrap();
    for p in c.voyage.etapes.iter().flat_map(|e| &e.photos) {
        assert!(!p.fichier.contains('/') && !p.fichier.contains(".."), "{}", p.fichier);
        assert!(p.fichier.starts_with('e') && p.fichier.ends_with(".jpg"));
    }
}

#[test]
fn les_pays_se_recomptent_depuis_les_etapes() {
    let c = convertir(&fixture(), 4242, &limites_larges()).unwrap();
    // `country_count` vaut 0 dans la réponse, comme sur le tour du monde mesuré.
    assert_eq!(c.voyage.pays, ["FR", "TN"]);
    assert_eq!(c.voyage.bilan().pays, 2);

    let mut j = fixture();
    j["country_count"] = json!(99);
    j["steps"][2]["location"]["country_code"] = json!("it");
    let c = convertir(&j, 4242, &limites_larges()).unwrap();
    assert_eq!(c.voyage.pays, ["FR", "TN", "IT"], "le compteur de la source est ignoré, les codes sont mis en majuscules");

    // Le voyage réel mesuré rend « 00 » pour un lieu sans pays (en mer, en vol) :
    // ce n'est pas un pays.
    j["steps"][1]["location"]["country_code"] = json!("00");
    let c = convertir(&j, 4242, &limites_larges()).unwrap();
    assert_eq!(c.voyage.pays, ["FR", "IT"]);
}

#[test]
fn les_segments_gardent_le_mode_et_decodent_la_polyligne() {
    let v = convertir(&fixture(), 4242, &limites_larges()).unwrap().voyage;
    // Le trajet prévu (`future_directions`) n'est pas un trajet fait.
    assert_eq!(v.segments.len(), 2);
    let vol = &v.segments[0];
    assert_eq!(vol.mode.as_deref(), Some("flight"));
    assert_eq!((vol.de, vol.a), (Some(0), Some(1)));
    assert_eq!(vol.polyligne.len(), 3);
    assert!((vol.polyligne[0].lat - 38.5).abs() < 1e-9 && (vol.polyligne[0].lon + 120.2).abs() < 1e-9);
    assert!((vol.polyligne[2].lat - 43.252).abs() < 1e-9 && (vol.polyligne[2].lon + 126.453).abs() < 1e-9);
    // Le mode « none » ne dit rien ; une route illisible n'invente pas de points.
    let deuxieme = &v.segments[1];
    assert_eq!(deuxieme.mode, None);
    assert_eq!((deuxieme.de, deuxieme.a), (Some(1), None));
    assert!(deuxieme.polyligne.is_empty());
}

#[test]
fn la_trace_vient_des_zelda_steps() {
    let v = convertir(&fixture(), 4242, &limites_larges()).unwrap().voyage;
    assert_eq!(v.trace.len(), 4);
    assert_eq!(v.trace[2].lat, 36.8);
    let b = v.bilan();
    assert!(b.trace_presente);
    assert_eq!(b.points_de_trace, 4);
}

#[test]
fn la_variante_70_sans_trace_se_signale() {
    let mut j = fixture();
    j.as_object_mut().unwrap().remove("zelda_steps");
    j["visited_countries"] = json!(["FR", "TN"]);
    j["most_recent_zelda_step"] = json!(null);
    let v = convertir(&j, 4242, &limites_larges()).unwrap().voyage;
    assert!(v.trace.is_empty());
    let b = v.bilan();
    assert!(!b.trace_presente);
    assert_eq!(b.points_de_trace, 0);
    assert!(b.to_string().contains("trace : absente"));
}

#[test]
fn ni_l_utilisateur_ni_les_compagnons_ne_sortent() {
    let c = convertir(&fixture(), 4242, &limites_larges()).unwrap();
    let texte = serde_json::to_string(&c.voyage).unwrap();
    for piege in ["PIEGE", "777", "778", "user", "buddies", "likes"] {
        assert!(!texte.contains(piege), "« {piege} » est sorti dans voyage.json : {texte}");
    }
}

#[test]
fn bilan_des_photos_sous_le_repere() {
    let dir = dossier("bilan");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    let v = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|_, _| {}).unwrap();
    let b = v.bilan();
    assert_eq!((b.etapes, b.etapes_sans_photo, b.etapes_avec_meteo), (3, 1, 2));
    assert_eq!((b.photos, b.photos_sous_seuil, b.videos_ignorees), (3, 3, 1));
    assert_eq!(v.etapes[0].photos[0].largeur, 12);
    assert_eq!(v.etapes[0].photos[0].hauteur, 8);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- le format inconnu : un échec franc, jamais un voyage à moitié rempli ----

#[test]
fn un_format_inconnu_est_refuse_franchement() {
    let cas: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("pas un objet", Box::new(|j| *j = json!([1, 2]))),
        ("steps absent", Box::new(|j| {
            j.as_object_mut().unwrap().remove("steps");
        })),
        ("steps n'est plus une liste", Box::new(|j| j["steps"] = json!({"a": 1}))),
        ("id d'un autre voyage", Box::new(|j| j["id"] = json!(1))),
        ("titre absent", Box::new(|j| {
            j.as_object_mut().unwrap().remove("name");
            j.as_object_mut().unwrap().remove("display_name");
        })),
        ("étape sans start_time", Box::new(|j| {
            j["steps"][1].as_object_mut().unwrap().remove("start_time");
        })),
        ("start_time devenu un nombre", Box::new(|j| j["steps"][1]["start_time"] = json!(1714730400))),
        ("étape sans lieu", Box::new(|j| {
            j["steps"][0].as_object_mut().unwrap().remove("location");
        })),
        ("latitude hors du monde", Box::new(|j| j["steps"][0]["location"]["lat"] = json!(123.0))),
        ("étape sans fuseau", Box::new(|j| {
            j["steps"][0].as_object_mut().unwrap().remove("timezone_id");
        })),
        ("étape sans nom", Box::new(|j| {
            let e = j["steps"][0].as_object_mut().unwrap();
            e.remove("name");
            e.remove("display_name");
        })),
        ("media n'est plus une liste", Box::new(|j| j["steps"][0]["media"] = json!("x"))),
        ("photo sans vignette", Box::new(|j| {
            j["steps"][0]["media"][0].as_object_mut().unwrap().remove("large_thumbnail_path");
        })),
        ("type de média inconnu", Box::new(|j| j["steps"][0]["media"][0]["type"] = json!(7))),
        ("vignette hors de Polarsteps", Box::new(|j| {
            j["steps"][0]["media"][0]["large_thumbnail_path"] = json!("https://evil.example/a.jpg")
        })),
        ("vignette en http", Box::new(|j| {
            j["steps"][0]["media"][0]["large_thumbnail_path"] = json!("http://media.prod.polarsteps.com/a.jpg")
        })),
        ("vignette qui ne dit pas son format", Box::new(|j| {
            j["steps"][0]["media"][0]["large_thumbnail_path"] = json!("https://media.prod.polarsteps.com/a")
        })),
        ("point de trace sans heure", Box::new(|j| {
            j["zelda_steps"][1].as_object_mut().unwrap().remove("time");
        })),
        ("zelda_steps n'est plus une liste", Box::new(|j| j["zelda_steps"] = json!("x"))),
    ];
    for (nom, mutation) in cas {
        let mut j = fixture();
        mutation(&mut j);
        match convertir(&j, 4242, &limites_larges()) {
            Err(Erreur::Format(m)) => {
                let phrase = Erreur::Format(m).to_string();
                assert!(phrase.contains("passe par l'export"), "{nom} : {phrase}");
            }
            autre => panic!("{nom} : attendu Erreur::Format, reçu {:?}", autre.map(|c| c.voyage.titre)),
        }
    }
}

#[test]
fn un_format_inconnu_n_ecrit_rien() {
    let dir = dossier("format");
    let mut j = fixture();
    j["steps"][1].as_object_mut().unwrap().remove("start_time");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, serde_json::to_vec(&j).unwrap());
    let r = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|_, _| {});
    format_attendu(r);
    // Ni photo téléchargée, ni voyage.json.
    assert_eq!(faux.appels.borrow().len(), 1, "seule la demande du voyage est partie");
    assert!(!dir.join("voyage.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn une_reponse_qui_n_est_pas_du_json_est_un_format_inconnu() {
    let dir = dossier("pasjson");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, b"<html>maintenance</html>".to_vec());
    format_attendu(importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|_, _| {}));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- les plafonds ------------------------------------------------------------

#[test]
fn la_reponse_trop_grosse_est_refusee() {
    let dir = dossier("plafond-reponse");
    let corps = V62.as_bytes().to_vec();
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, corps.clone());
    let mut r = reglages();
    r.limites.reponse = corps.len() as u64 - 1;
    let e = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &r, &|_, _| {}).unwrap_err();
    assert_eq!(e, Erreur::TropGros("la réponse du voyage"));
    // Pile à la taille, elle passe.
    r.limites.reponse = corps.len() as u64;
    importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &r, &|_, _| {}).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn la_photo_trop_grosse_est_refusee() {
    let dir = dossier("plafond-photo");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    let taille = jpeg(12, 8).len() as u64;
    let mut r = reglages();
    r.limites.photo = taille - 1;
    let e = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &r, &|_, _| {}).unwrap_err();
    assert_eq!(e, Erreur::TropGros("une photo"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn le_total_des_photos_est_plafonne() {
    let dir = dossier("plafond-total");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    let taille = jpeg(12, 8).len() as u64;
    let mut r = reglages();
    r.limites.total = taille * 2;
    let e = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &r, &|_, _| {}).unwrap_err();
    assert_eq!(e, Erreur::TropGros("le total des photos"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn trop_d_etapes_ou_de_photos_ou_de_points_sont_refuses_avant_toute_photo() {
    let mut l = limites_larges();
    l.etapes = 2;
    assert_eq!(convertir(&fixture(), 4242, &l).err(), Some(Erreur::TropGros("le nombre d'étapes")));
    let mut l = limites_larges();
    l.photos = 2;
    assert_eq!(convertir(&fixture(), 4242, &l).err(), Some(Erreur::TropGros("le nombre de photos")));
    let mut l = limites_larges();
    l.trace = 3;
    assert_eq!(convertir(&fixture(), 4242, &l).err(), Some(Erreur::TropGros("la trace")));
}

// ---- le réseau et le disque --------------------------------------------------

#[test]
fn le_jeton_part_tel_quel_et_ne_reste_nulle_part() {
    let dir = dossier("jeton");
    let jeton = "Zx-9_secretJETON";
    let faux = Faux::avec_voyage(&fixture()).api(4242, Some(jeton), 200, V62.as_bytes().to_vec());
    let v = importer(&format!("https://www.polarsteps.com/q/4242-v?s={jeton}"), &dir, &faux, &reglages(), &|_, _| {}).unwrap();
    assert_eq!(faux.appels.borrow()[0], url_api(4242, Some(jeton)));
    // Les photos sont publiques : aucun jeton ne les accompagne.
    assert!(faux.appels.borrow()[1..].iter().all(|u| !u.contains(jeton)));
    let ecrit = std::fs::read_to_string(dir.join("voyage.json")).unwrap();
    assert!(!ecrit.contains(jeton) && !serde_json::to_string(&v).unwrap().contains(jeton));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn les_statuts_se_disent() {
    let dir = dossier("statuts");
    for (statut, attendu) in [(404u16, true), (403, true), (500, false)] {
        let faux = Faux::avec_voyage(&fixture()).api(4242, None, statut, b"{}".to_vec());
        let e = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|_, _| {}).unwrap_err();
        assert_eq!(e, Erreur::Statut { statut, quoi: "voyage" });
        assert_eq!(e.to_string().contains("lien de partage"), attendu, "{e}");
    }
    // Une photo qui répond 403 arrête l'import : pas de voyage à moitié rempli.
    let mut faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    faux.reponses.insert("https://media.prod.polarsteps.com/user_images/steps/large_thumb/u_777/b2.jpg".into(), (403, vec![]));
    let e = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|_, _| {}).unwrap_err();
    assert_eq!(e, Erreur::Statut { statut: 403, quoi: "une photo" });
    assert!(!dir.join("voyage.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn une_photo_qui_n_est_pas_une_image_est_refusee() {
    let dir = dossier("pasimage");
    let mut faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    faux.reponses.insert("https://media.prod.polarsteps.com/user_images/steps/large_thumb/u_777/a1.jpg".into(), (200, b"<html>".to_vec()));
    let e = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|_, _| {}).unwrap_err();
    assert!(matches!(e, Erreur::Photo(_)), "{e:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn l_import_ecrit_les_photos_et_un_voyage_json_qui_se_relit() {
    let dir = dossier("complet");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    let progres = RefCell::new(Vec::new());
    let v = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|f, t| progres.borrow_mut().push((f, t))).unwrap();
    assert_eq!(*progres.borrow(), [(1, 3), (2, 3), (3, 3)]);
    for p in v.etapes.iter().flat_map(|e| &e.photos) {
        assert!(dir.join(&p.fichier).is_file(), "{}", p.fichier);
    }
    assert!(!std::fs::read_dir(&dir).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().ends_with(".part")));
    // L'aller-retour : ce qui est écrit, relu, est le voyage rendu.
    assert_eq!(crate::voyage::lire(&dir).unwrap(), v);
    // Une requête à la fois : la demande du voyage, puis les trois photos, dans l'ordre.
    let appels = faux.appels.borrow();
    assert_eq!(appels.len(), 4);
    assert_eq!(appels[0], url_api(4242, None));
    assert!(appels[1].ends_with("/a1.jpg") && appels[2].ends_with("/b2.jpg") && appels[3].ends_with("/d4.jpg"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reprendre_un_import_ne_retelecharge_pas_les_photos_deja_la() {
    let dir = dossier("reprise");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    let un = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &reglages(), &|_, _| {}).unwrap();
    let faux2 = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    let deux = importer("https://www.polarsteps.com/q/4242-v", &dir, &faux2, &reglages(), &|_, _| {}).unwrap();
    assert_eq!(un, deux);
    assert_eq!(faux2.appels.borrow().len(), 1, "seule la demande du voyage repart");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn la_pause_separe_les_requetes() {
    let dir = dossier("pause");
    let faux = Faux::avec_voyage(&fixture()).api(4242, None, 200, V62.as_bytes().to_vec());
    let mut r = reglages();
    r.pause = Duration::from_millis(40);
    let t0 = std::time::Instant::now();
    importer("https://www.polarsteps.com/q/4242-v", &dir, &faux, &r, &|_, _| {}).unwrap();
    // 4 requêtes, 3 pauses entre elles.
    assert!(t0.elapsed() >= Duration::from_millis(120), "{:?}", t0.elapsed());
    let _ = std::fs::remove_dir_all(&dir);
}

fn limites_larges() -> Limites {
    reglages().limites
}

// ---- le vrai réseau : à la main -----------------------------------------------

/// `COLOPHON_LIEN_POLARSTEPS=<url> COLOPHON_LIEN_VERS=/tmp/... cargo test -p
/// colophon-core --release lien_reel -- --ignored --nocapture`
#[test]
#[ignore]
fn lien_reel() {
    let url = std::env::var("COLOPHON_LIEN_POLARSTEPS").expect("COLOPHON_LIEN_POLARSTEPS");
    let vers = std::path::PathBuf::from(std::env::var("COLOPHON_LIEN_VERS").expect("COLOPHON_LIEN_VERS"));
    assert!(vers.starts_with(std::env::temp_dir()) || vers.starts_with("/tmp"), "écrire hors du dépôt, sous /tmp");
    let v = importer(&url, &vers, &Reseau::default(), &Reglages::default(), &|f, t| {
        if f % 100 == 0 || f == t {
            eprintln!("photo {f}/{t}");
        }
    })
    .unwrap();
    println!("{}", v.bilan());
}
