use super::*;

/// Le banc du cas fabriqué (hors dépôt) : pour chaque vignette, la distance au
/// bon original et au meilleur des mauvais. À la main :
/// `COLOPHON_ORIGINAUX_CAS=<cas> cargo test -p colophon-core --release banc_du_cas -- --ignored --nocapture`
#[test]
#[ignore]
fn banc_du_cas_fabrique() {
    let Ok(cas) = std::env::var("COLOPHON_ORIGINAUX_CAS") else { return };
    let cas = PathBuf::from(cas);
    let verite: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(cas.join("verite.json")).unwrap()).unwrap();
    let voyage = crate::voyage::lire(&cas.join("voyage")).unwrap();
    let (index, _) = indexer(&cas.join("originaux"), &[], &|_, _| {}, &|| false).unwrap();
    let vignettes = vignettes_du_voyage(&cas.join("voyage"), &voyage, &[]);
    let mut lignes = Vec::new();
    for (v, _) in &vignettes {
        let t = &verite[&v.fichier];
        let vrai = t["original"].as_str().or(t["vrai"].as_str());
        let emp = v.empreinte.as_ref().unwrap();
        let mut bon = f32::NAN;
        let mut meilleur_autre = f32::INFINITY;
        for o in &index {
            if !ratio_compatible(v.largeur, v.hauteur, o.largeur, o.hauteur) {
                continue;
            }
            let d = distance(emp, &o.empreinte);
            if Some(o.chemin.file_name().unwrap().to_str().unwrap()) == vrai {
                bon = d;
            } else {
                meilleur_autre = meilleur_autre.min(d);
            }
        }
        lignes.push(format!("{}\t{}\t{:.3}\t{:.3}", t["piege"].as_str().unwrap(), v.fichier, bon, meilleur_autre));
    }
    lignes.sort();
    println!("piege\tfichier\tbon\tmeilleur_autre\n{}", lignes.join("\n"));
}
