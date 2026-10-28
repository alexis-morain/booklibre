//! La table des tarifs (1.1-f) : ce que l'app lit pour dire « À partir de
//! 46 € livré » pendant qu'on construit l'album.
//!
//! **Un Worker à part, `colophon-tarifs`**, publie une seule ressource,
//! `GET /v1/prices` : par format et par pays, le prix de vente TTC en
//! centimes pour chaque nombre pair de pages, déjà calculé par le relais.
//! L'app lit une case et ne recalcule rien. Aucune intention n'est créée,
//! aucun en-tête propre ne part, aucun identifiant : un GET nu, par le même
//! [`Transport`] que le relais.
//!
//! **Quand l'appel part, c'est l'app qui le décide** (C59 de la revue
//! 1.1-c-f) : jamais avant que quelqu'un ait répondu « Par Booklibre » à
//! « Comment l'imprimer ? », jamais dans la version Mac App Store. Ce module
//! ne sait que lire ; la porte est dans `src-tauri` et `impression.ts`.
//!
//! **L'adresse est compilée comme celle du relais** : le sandbox en debug ou
//! avec `COLOPHON_RELAIS_SANDBOX=1`, sinon `COLOPHON_TARIFS_REEL` (un hôte
//! seul, jugé par [`relais::reel_valide`]), et rien sans elle.

use crate::relais::{self, Mode, Transport};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// L'adresse du Worker des tarifs au sandbox.
const SANDBOX: &str = "https://colophon-tarifs-sandbox.alexis-c1f.workers.dev";

/// Le chemin de la seule ressource.
pub const CHEMIN: &str = "/v1/prices";

/// Un instantané plus vieux que sept jours ne dit plus un prix (C52) : le
/// Worker le refuse déjà, l'app aussi, au cas où une réponse traînerait.
pub const PEREMPTION_S: i64 = 7 * 24 * 3600;

/// Le plus grand nombre de pages que le relais vend (`PAGES_MAX`).
pub const PAGES_MAX: u32 = 200;

/// L'adresse pour un build : le sandbox en debug ou si la compilation l'a
/// demandé, sinon l'adresse réelle reçue, et rien sans elle. Le même choix
/// que [`relais::url_pour`], pour que les deux visent le même mode.
pub const fn url_pour(debug: bool, sandbox: bool, reel: Option<&'static str>) -> Option<&'static str> {
    if debug || sandbox {
        Some(SANDBOX)
    } else {
        reel
    }
}

/// Le Worker réel reçu par `COLOPHON_TARIFS_REEL`, jugé à la compilation :
/// une adresse refusée arrête le build, comme celle du relais.
pub const REEL: Option<&str> = match relais::reel_valide(option_env!("COLOPHON_TARIFS_REEL")) {
    Ok(r) => r,
    Err(_) => panic!("COLOPHON_TARIFS_REEL : https:// suivi d'un hôte seul, sans chemin, port, identifiants ni requête"),
};

/// Le sandbox et le réel à la fois arrêtent la compilation.
const _: () = match relais::relais_unique(relais::SANDBOX_EN_RELEASE, REEL) {
    Ok(()) => (),
    Err(_) => panic!("COLOPHON_RELAIS_SANDBOX et COLOPHON_TARIFS_REEL : l'un ou l'autre"),
};

/// L'adresse compilée du Worker des tarifs. Sans elle, l'app ne pose pas la
/// question et n'affiche aucun prix.
pub const TARIFS_URL: Option<&str> = url_pour(cfg!(debug_assertions), relais::SANDBOX_EN_RELEASE, REEL);

/// Une ligne de la table : les prix de `de` pages à 200, de `pas` en `pas`.
/// `None` quand aucun niveau de livraison ne tient dans les bornes de prix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ligne {
    pub de: u32,
    pub pas: u32,
    pub ttc_centimes: Vec<Option<u32>>,
}

/// La table telle que le Worker la publie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tarifs {
    pub mode: Mode,
    pub genere_le: String,
    pub devise: String,
    pub pays: Vec<String>,
    pub formats: BTreeMap<String, BTreeMap<String, Ligne>>,
}

/// Ce qui empêche de lire un prix. Aucune ne recopie un corps reçu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Erreur {
    /// Le réseau, avant tout statut.
    Reseau,
    /// Le Worker répond hors 200 : `pas_encore`, `perime`, `indisponible`…
    Statut { statut: u16, erreur: String },
    /// Un corps que l'app ne sait pas lire, ou hors des bornes.
    Illisible(&'static str),
    /// L'instantané a plus de sept jours.
    Perime,
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Reseau => f.write_str("tarifs injoignables"),
            Erreur::Statut { statut, erreur } => write!(f, "tarifs : {erreur} ({statut})"),
            Erreur::Illisible(r) => write!(f, "tarifs illisibles : {r}"),
            Erreur::Perime => f.write_str("tarifs périmés"),
        }
    }
}

impl std::error::Error for Erreur {}

fn code_pays(c: &str) -> bool {
    c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase())
}

fn nom_de_format(c: &str) -> bool {
    !c.is_empty() && c.len() <= 40 && c.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl Tarifs {
    /// Les bornes de la lecture : ce qu'un Worker honnête rend, et rien de
    /// plus. Un document hors bornes ne donne aucun prix.
    fn verifier(&self, maintenant_s: i64) -> Result<(), Erreur> {
        if self.devise != "EUR" {
            return Err(Erreur::Illisible("devise"));
        }
        if self.pays.is_empty() || self.pays.len() > 60 || !self.pays.iter().all(|p| code_pays(p)) {
            return Err(Erreur::Illisible("pays"));
        }
        if self.formats.len() > 20 {
            return Err(Erreur::Illisible("formats"));
        }
        for (format, par_pays) in &self.formats {
            if !nom_de_format(format) || par_pays.len() > 60 {
                return Err(Erreur::Illisible("formats"));
            }
            for (pays, l) in par_pays {
                let n = l.ttc_centimes.len() as u32;
                let fin = l.de.checked_add(l.pas.saturating_mul(n.saturating_sub(1)));
                if !code_pays(pays)
                    || l.pas != 2
                    || l.de < 2
                    || l.de % 2 != 0
                    || n == 0
                    || fin.is_none_or(|f| f > PAGES_MAX)
                    || l.ttc_centimes.iter().flatten().any(|&c| c == 0 || c > 10_000_000)
                {
                    return Err(Erreur::Illisible("ligne"));
                }
            }
        }
        let genere = chrono::DateTime::parse_from_rfc3339(&self.genere_le).map_err(|_| Erreur::Illisible("genere_le"))?;
        if maintenant_s - genere.timestamp() > PEREMPTION_S {
            return Err(Erreur::Perime);
        }
        Ok(())
    }
}

/// Lit `GET {base}/v1/prices` : un GET nu, sans bearer, sans en-tête propre.
/// `maintenant_s` est l'heure Unix, pour la péremption.
pub fn lire(base: &str, transport: &dyn Transport, maintenant_s: i64) -> Result<Tarifs, Erreur> {
    let url = format!("{}{CHEMIN}", base.trim_end_matches('/'));
    let (statut, texte) = transport.get(&url, None).map_err(|_| Erreur::Reseau)?;
    if statut != 200 {
        let erreur = serde_json::from_str::<serde_json::Value>(&texte)
            .ok()
            .and_then(|v| v["erreur"].as_str().map(str::to_string))
            .filter(|e| !e.is_empty() && e.len() <= 32 && e.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
            .unwrap_or_else(|| "illisible".to_string());
        return Err(Erreur::Statut { statut, erreur });
    }
    if texte.len() > 512 * 1024 {
        return Err(Erreur::Illisible("taille"));
    }
    let t: Tarifs = serde_json::from_str(&texte).map_err(|_| Erreur::Illisible("json"))?;
    t.verifier(maintenant_s)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relais::{Envoi, Secret};
    use serde_json::{json, Value};
    use std::cell::RefCell;

    /// Un faux Worker : un statut et un corps, et tout ce qu'on lui a demandé.
    struct Faux {
        statut: u16,
        corps: String,
        vus: RefCell<Vec<(String, bool)>>,
    }

    impl Faux {
        fn new(statut: u16, corps: Value) -> Self {
            Faux { statut, corps: corps.to_string(), vus: RefCell::new(vec![]) }
        }
    }

    impl Transport for Faux {
        fn get(&self, url: &str, bearer: Option<&Secret>) -> Result<(u16, String), String> {
            self.vus.borrow_mut().push((url.to_string(), bearer.is_some()));
            Ok((self.statut, self.corps.clone()))
        }
        fn post_json(&self, _: &str, _: Option<&Secret>, _: &Value) -> Result<(u16, String), String> {
            panic!("les tarifs ne postent rien")
        }
        fn put_fichier(&self, _: &Envoi<'_>, _: &dyn Fn(u64)) -> Result<(u16, Option<String>), String> {
            panic!("les tarifs ne posent rien")
        }
    }

    /// Le 19/10 à 4 h UTC.
    const LUNDI: i64 = 1_792_382_400;

    fn document() -> Value {
        json!({
            "mode": "sandbox",
            "genere_le": "2026-10-19T04:00:00.000Z",
            "devise": "EUR",
            "pays": ["FR"],
            "formats": {
                "carre-21": { "FR": { "de": 24, "pas": 2, "ttc_centimes": [4600, 4650, null] } }
            }
        })
    }

    #[test]
    fn l_adresse_suit_celle_du_relais() {
        assert_eq!(url_pour(true, false, None), Some(SANDBOX));
        assert_eq!(url_pour(false, true, None), Some(SANDBOX));
        assert_eq!(url_pour(false, false, Some("https://prices.booklibre.app")), Some("https://prices.booklibre.app"));
        // La release publique sans adresse : aucun prix, aucun appel.
        assert_eq!(url_pour(false, false, None), None);
        // Le gate tourne en debug : le sandbox, jamais une autre adresse.
        assert_eq!(TARIFS_URL, Some(SANDBOX));
    }

    #[test]
    fn un_get_nu_vers_la_seule_ressource() {
        let f = Faux::new(200, document());
        let t = lire("https://tarifs.example/", &f, LUNDI + 3600).unwrap();
        assert_eq!(*f.vus.borrow(), vec![("https://tarifs.example/v1/prices".to_string(), false)]);
        assert_eq!(t.pays, vec!["FR"]);
        assert_eq!(t.formats["carre-21"]["FR"].ttc_centimes, vec![Some(4600), Some(4650), None]);
    }

    #[test]
    fn un_refus_du_worker_se_nomme_sans_recopier_le_corps() {
        let f = Faux::new(503, json!({ "erreur": "pas_encore" }));
        assert_eq!(lire("https://t", &f, LUNDI), Err(Erreur::Statut { statut: 503, erreur: "pas_encore".into() }));
        let f = Faux::new(500, json!({ "erreur": "<script>alert(1)</script>" }));
        assert_eq!(lire("https://t", &f, LUNDI), Err(Erreur::Statut { statut: 500, erreur: "illisible".into() }));
    }

    #[test]
    fn au_dela_de_sept_jours_aucun_prix() {
        let f = Faux::new(200, document());
        assert!(lire("https://t", &f, LUNDI + PEREMPTION_S).is_ok());
        assert_eq!(lire("https://t", &f, LUNDI + PEREMPTION_S + 1), Err(Erreur::Perime));
    }

    #[test]
    fn un_document_hors_bornes_ne_donne_aucun_prix() {
        let cas: Vec<(&str, Value)> = vec![
            ("/devise", json!("USD")),
            ("/pays", json!(["fr"])),
            ("/pays", json!([])),
            ("/genere_le", json!("hier")),
            ("/formats/carre-21/FR/pas", json!(4)),
            ("/formats/carre-21/FR/de", json!(23)),
            ("/formats/carre-21/FR/de", json!(198)),
            ("/formats/carre-21/FR/ttc_centimes", json!([0])),
            ("/formats/carre-21/FR/ttc_centimes", json!([-1])),
            ("/formats/carre-21/FR/ttc_centimes", json!([])),
        ];
        for (chemin, valeur) in cas {
            let mut d = document();
            *d.pointer_mut(chemin).unwrap() = valeur.clone();
            let f = Faux::new(200, d);
            assert!(lire("https://t", &f, LUNDI).is_err(), "{chemin} = {valeur}");
        }
        let mut d = document();
        d["formats"]["../x"] = json!({});
        assert!(lire("https://t", &Faux::new(200, d), LUNDI).is_err());
        let mut d = document();
        d.as_object_mut().unwrap().remove("mode");
        assert!(lire("https://t", &Faux::new(200, d), LUNDI).is_err());
    }

    #[test]
    fn le_reseau_en_panne_ne_recopie_rien() {
        struct Coupe;
        impl Transport for Coupe {
            fn get(&self, _: &str, _: Option<&Secret>) -> Result<(u16, String), String> {
                Err("tls: https://t/v1/prices".into())
            }
            fn post_json(&self, _: &str, _: Option<&Secret>, _: &Value) -> Result<(u16, String), String> {
                unreachable!()
            }
            fn put_fichier(&self, _: &Envoi<'_>, _: &dyn Fn(u64)) -> Result<(u16, Option<String>), String> {
                unreachable!()
            }
        }
        assert_eq!(lire("https://t", &Coupe, LUNDI), Err(Erreur::Reseau));
    }
}
