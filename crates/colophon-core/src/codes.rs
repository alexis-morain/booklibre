//! Les codes que le moteur donne à l'application pour qu'elle écrive, dans sa
//! langue, ce que le moteur a dit en français.
//!
//! Le français reste celui du moteur : la ligne de commande le lit, les tests
//! le citent, et l'écran français l'affiche tel quel. L'anglais vit dans
//! `crates/colophon-app/src/moteur.ts`, rangé par ces codes. Ce module en
//! tient la liste, et `moteur.codes.json` en est la copie que lit l'app : un
//! test de ce module rougit quand elle a vieilli, un test de l'app rougit
//! quand un code n'a pas d'anglais ou qu'une phrase anglaise attend une valeur
//! que le moteur ne donne pas.

use crate::{build, format, layout, prevol, printer};
use std::collections::BTreeMap;

/// La liste, en JSON stable : formats, rythmes, propositions, les règles du
/// prévol, ses phrases avec les valeurs que chacune lit, et les textes fixes — notes du
/// rapport et réserves des profils — en entier, puisque l'app les reconnaît
/// mot pour mot.
pub fn codes_json() -> String {
    // Des `BTreeMap`, jamais `serde_json::Map` : l'ordre de ce dernier dépend
    // d'une option que n'importe quel crate du workspace peut allumer, et la
    // fixture changerait selon ce qu'on compile à côté.
    #[derive(serde::Serialize)]
    struct Codes {
        densites: Vec<&'static str>,
        formats: Vec<&'static str>,
        notes: [&'static str; 2],
        phrases: BTreeMap<&'static str, &'static [&'static str]>,
        propositions: [&'static str; 3],
        regles: &'static [&'static str],
        reserves: BTreeMap<&'static str, &'static [&'static str]>,
    }
    let codes = Codes {
        densites: layout::Densite::toutes().iter().map(|d| d.id()).collect(),
        formats: format::FORMATS.iter().map(|f| f.0).collect(),
        notes: [prevol::NOTE_PDF_X4, prevol::NOTE_PHOTOS_INTROUVABLES],
        phrases: prevol::PHRASES.iter().copied().collect(),
        propositions: build::IDS_PROPOSITIONS,
        regles: prevol::REGLES,
        reserves: printer::PrinterProfile::tous().iter().map(|p| (p.id, p.reserves)).collect(),
    };
    let mut s = serde_json::to_string_pretty(&codes).expect("du JSON fait de chaînes");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../colophon-app/src/moteur.codes.json");

    /// La copie de l'app suit le moteur. Comparée en contenu, pas en fins de
    /// ligne : git rend un `.json` en CRLF sur Windows (voir `LICENCES.md`).
    #[test]
    fn la_liste_des_codes_de_l_app_est_a_jour() {
        let sur_disque = std::fs::read_to_string(FIXTURE)
            .expect("moteur.codes.json manquant")
            .replace("\r\n", "\n");
        assert_eq!(
            sur_disque,
            codes_json(),
            "moteur.codes.json a vieilli : le régénérer avec \
             cargo test -p colophon-core ecrire_la_liste_des_codes -- --ignored"
        );
    }

    /// Réécrit `moteur.codes.json`. Ignoré : c'est un outil, pas une
    /// vérification.
    #[test]
    #[ignore]
    fn ecrire_la_liste_des_codes() {
        std::fs::write(FIXTURE, codes_json()).unwrap();
    }

    /// Un code de la table que plus rien n'émet est une phrase anglaise que
    /// personne ne lira jamais, et c'est le signe qu'une règle a changé de
    /// mots sans changer de code. Chaque code doit apparaître dans `prevol.rs`
    /// hors de sa table.
    #[test]
    fn chaque_code_de_phrase_est_emis() {
        let source = include_str!("prevol.rs");
        let corps = source.split("pub const PHRASES").next().unwrap().to_string()
            + source.split("fn phrase(").nth(1).unwrap();
        for (code, _) in prevol::PHRASES {
            assert!(corps.contains(&format!("\"{code}\"")), "{code} n'est émis nulle part");
        }
    }

    /// Les règles du prévol sont une liste fermée, que l'app lit pour donner
    /// à chacune sa destination à l'écran (`envoi.ts::DESTINATIONS`). Chaque
    /// règle de la liste est émise, et chaque `regle: "…"` écrit dans le
    /// prévol est dans la liste : une règle neuve sans entrée rougit ici, puis
    /// dans l'app tant qu'elle n'a nulle part où mener. `check` le vérifie
    /// aussi à chaque défaut qu'il pose, en debug.
    #[test]
    fn chaque_regle_du_prevol_est_listee_et_emise() {
        let source = include_str!("prevol.rs");
        let corps = source.split("pub const REGLES").next().unwrap().to_string()
            + source.split("fn phrase(").nth(1).unwrap().split("#[cfg(test)]").next().unwrap();
        for regle in prevol::REGLES {
            assert!(corps.contains(&format!("\"{regle}\"")), "{regle} n'est émise nulle part");
        }
        for (i, _) in corps.match_indices("regle: \"") {
            let reste = &corps[i + "regle: \"".len()..];
            let regle = &reste[..reste.find('"').unwrap()];
            assert!(prevol::REGLES.contains(&regle), "{regle} manque à prevol::REGLES");
        }
        let codes: serde_json::Value = serde_json::from_str(&codes_json()).unwrap();
        assert_eq!(codes["regles"].as_array().map(Vec::len), Some(prevol::REGLES.len()));
    }
}
