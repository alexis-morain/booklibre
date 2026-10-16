//! The language of the words the engine writes into an album.
//!
//! The engine writes a handful of phrases into the book itself: the dates
//! and towns of the half-title, the colophon page, the line that opens each
//! chapter, the cover's subtitle, a caption's date. They follow the language
//! of the interface **at the moment they are written**: an album composed in
//! English carries English pages, and switching the interface afterwards
//! changes nothing in it, because the words are already in `album.json` as
//! text. No field records the language: a page regenerated later (the Envoi
//! screen puts the colophon back, a recomposition rewrites the chapters)
//! speaks the language of the interface that regenerates it.
//!
//! Everything else the engine says stays in English for a developer and a
//! log, as before (`i18n.ts`). This module is only about the paper.
//!
//! Town names come from the gazetteer and print as they are in both
//! languages; camera models too.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// French first: the language everything was written in, and the default of
/// every caller that does not say, the command line included.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Langue {
    #[default]
    Fr,
    En,
}

const MOIS_FR: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août",
    "septembre", "octobre", "novembre", "décembre",
];

const MONTHS_EN: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December",
];

impl Langue {
    /// `fr` or `en`, the codes `i18n.ts` stores. Anything else is refused
    /// rather than quietly read as French.
    pub fn par_code(code: &str) -> Option<Self> {
        match code {
            "fr" => Some(Langue::Fr),
            "en" => Some(Langue::En),
            _ => None,
        }
    }

    /// « 27 octobre 2013 », « 27 October 2013 »: day first in both, the way
    /// a book printed in Europe dates itself.
    pub fn date(self, d: NaiveDate, with_year: bool) -> String {
        let m = match self {
            Langue::Fr => MOIS_FR[d.month0() as usize],
            Langue::En => MONTHS_EN[d.month0() as usize],
        };
        if with_year {
            format!("{} {} {}", d.day(), m, d.year())
        } else {
            format!("{} {}", d.day(), m)
        }
    }

    /// The span an album covers, as a sentence fragment in lower case:
    /// « le 21 octobre 2013 » / « on 21 October 2013 » for one day,
    /// « du 21 au 29 octobre 2013 » / « from 21 to 29 October 2013 » when one
    /// month holds both ends, and both months (and years) named when it does
    /// not. `None` without a trusted date at either end.
    pub fn periode(self, debut: Option<NaiveDate>, fin: Option<NaiveDate>) -> Option<String> {
        let (d, e) = (debut?, fin?);
        let (un_jour, de, a) = match self {
            Langue::Fr => ("le", "du", "au"),
            Langue::En => ("on", "from", "to"),
        };
        if d == e {
            return Some(format!("{un_jour} {}", self.date(d, true)));
        }
        let depart = if d.year() != e.year() {
            self.date(d, true)
        } else if d.month() != e.month() {
            self.date(d, false)
        } else {
            d.day().to_string()
        };
        Some(format!("{de} {depart} {a} {}", self.date(e, true)))
    }

    /// The dates of a chapter line: « 27 octobre – 3 novembre 2013 ». A dash,
    /// not a sentence: the line is a heading.
    pub fn intervalle(self, d: NaiveDate, e: NaiveDate) -> String {
        if d == e {
            self.date(d, true)
        } else {
            format!("{} \u{2013} {}", self.date(d, false), self.date(e, true))
        }
    }

    /// « a, b et c », « a, b and c ». The last separator is a word, the way a
    /// sentence joins things.
    pub fn liste(self, v: &[String]) -> String {
        let et = match self {
            Langue::Fr => "et",
            Langue::En => "and",
        };
        match v.len() {
            0 => String::new(),
            1 => v[0].clone(),
            _ => format!("{} {et} {}", v[..v.len() - 1].join(", "), v[v.len() - 1]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn les_codes_de_l_interface_et_rien_d_autre() {
        assert_eq!(Langue::par_code("fr"), Some(Langue::Fr));
        assert_eq!(Langue::par_code("en"), Some(Langue::En));
        assert_eq!(Langue::par_code("de"), None);
        assert_eq!(Langue::par_code(""), None);
        assert_eq!(Langue::default(), Langue::Fr);
    }

    #[test]
    fn une_date_dans_chaque_langue() {
        assert_eq!(Langue::Fr.date(j(2013, 10, 27), true), "27 octobre 2013");
        assert_eq!(Langue::En.date(j(2013, 10, 27), true), "27 October 2013");
        assert_eq!(Langue::En.date(j(2013, 11, 3), false), "3 November");
    }

    #[test]
    fn la_periode_en_anglais() {
        let en = Langue::En;
        assert_eq!(
            en.periode(Some(j(2013, 10, 27)), Some(j(2013, 11, 3))).as_deref(),
            Some("from 27 October to 3 November 2013")
        );
        assert_eq!(
            en.periode(Some(j(2013, 10, 21)), Some(j(2013, 10, 29))).as_deref(),
            Some("from 21 to 29 October 2013")
        );
        assert_eq!(
            en.periode(Some(j(2013, 12, 29)), Some(j(2014, 1, 2))).as_deref(),
            Some("from 29 December 2013 to 2 January 2014")
        );
        assert_eq!(
            en.periode(Some(j(2013, 10, 21)), Some(j(2013, 10, 21))).as_deref(),
            Some("on 21 October 2013")
        );
        assert_eq!(en.periode(None, Some(j(2013, 10, 21))), None);
    }

    #[test]
    fn la_periode_en_francais_ne_bouge_pas() {
        let fr = Langue::Fr;
        assert_eq!(
            fr.periode(Some(j(2013, 10, 27)), Some(j(2013, 11, 3))).as_deref(),
            Some("du 27 octobre au 3 novembre 2013")
        );
        assert_eq!(
            fr.periode(Some(j(2013, 10, 21)), Some(j(2013, 10, 21))).as_deref(),
            Some("le 21 octobre 2013")
        );
    }

    #[test]
    fn l_intervalle_d_un_chapitre() {
        assert_eq!(
            Langue::En.intervalle(j(2013, 10, 27), j(2013, 11, 3)),
            "27 October \u{2013} 3 November 2013"
        );
        assert_eq!(Langue::En.intervalle(j(2013, 10, 27), j(2013, 10, 27)), "27 October 2013");
    }

    #[test]
    fn une_liste_se_joint_par_un_mot() {
        let v: Vec<String> = ["Calvi", "Lumio", "Algajola"].iter().map(|s| s.to_string()).collect();
        assert_eq!(Langue::Fr.liste(&v), "Calvi, Lumio et Algajola");
        assert_eq!(Langue::En.liste(&v), "Calvi, Lumio and Algajola");
        assert_eq!(Langue::En.liste(&v[..1]), "Calvi");
    }
}
