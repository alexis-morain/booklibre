//! Un voyage, sans savoir d'où il vient (P-s1).
//!
//! Même patron que la photothèque du Mac : une source **produit un dossier**
//! (les photos et un `voyage.json`), et le moteur n'apprend pas le nom de la
//! source. Il apprend « voyage » : des étapes, des lieux, des dates, des
//! textes, une trace. Le seul mot propre à un service est le champ
//! [`Provenance`], une simple étiquette de lecture.
//!
//! `voyage.json` est versionné ([`VERSION`]), lisible et réparable à la main.
//! Un fichier d'une version plus récente est refusé plutôt que lu de travers,
//! comme `album.json` (`SchemaTropRecent`). `scan.rs` ignore déjà `json` : le
//! fichier vit à côté des photos sans les gêner.
//!
//! Le lecteur du lien public est à part ([`lien`]) : lui seul porte une
//! adresse réseau, et la version Mac App Store ne l'appelle jamais.

pub mod composition;
pub mod lien;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// La version de `voyage.json` que ce Booklibre écrit et sait lire.
pub const VERSION: u32 = 1;

/// Le nom du fichier, à côté des photos.
pub const FICHIER: &str = "voyage.json";

/// Le côté court, en pixels, qu'une photo pleine page demande à 250 ppi sur
/// une page de 216 mm fond perdu compris (plan D7). Un repère connu dès
/// l'import ; le ppi réel se calcule à la composition, boîte en main.
pub const SEUIL_COTE_COURT_PX: u32 = 2126;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Voyage {
    pub version: u32,
    pub titre: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debut: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fin: Option<DateTime<Utc>>,
    /// Le fuseau du voyage (identifiant IANA), celui de son départ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuseau: Option<String>,
    /// La distance parcourue, en km, telle que la source l'affiche.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_km: Option<f64>,
    /// Les codes pays des étapes, dans l'ordre de première apparition :
    /// recomptés, jamais repris d'un compteur de la source (plan D18).
    #[serde(default)]
    pub pays: Vec<String>,
    pub etapes: Vec<Etape>,
    #[serde(default)]
    pub segments: Vec<Segment>,
    /// La trace du tracker. Vide quand la source ne la donne pas.
    #[serde(default)]
    pub trace: Vec<PointDeTrace>,
    /// Les vidéos que l'import n'a pas prises : comptées, jamais perdues
    /// en silence.
    #[serde(default)]
    pub videos_ignorees: u32,
    /// Le nom de fichier de la photo de couverture de la source, quand elle
    /// est dans le dossier (P-s2). Additif : un `voyage.json` d'avant ne
    /// l'a pas, et la composition choisit alors comme pour tout dossier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub couverture: Option<String>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// Qui a produit ce fichier (`polarsteps-lien`).
    pub source: String,
    /// L'identifiant du voyage chez la source. Jamais un nom d'utilisateur ni
    /// un jeton de partage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// La forme que la source a rendue (`api-62`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Etape {
    pub nom: String,
    /// Le récit, émojis compris (retirés à l'impression, pas ici).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recit: Option<String>,
    pub debut: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fin: Option<DateTime<Utc>>,
    /// Le fuseau de l'étape : l'heure locale exacte s'en déduit.
    pub fuseau: String,
    pub lieu: Lieu,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meteo: Option<Meteo>,
    /// Les noms de fichiers, dans l'ordre de l'étape, relatifs au dossier.
    #[serde(default)]
    pub photos: Vec<Photo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lieu {
    pub lat: f64,
    pub lon: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub localite: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pays: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_pays: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meteo {
    /// La condition telle que la source la nomme. Une condition inconnue est
    /// gardée : la table de correspondance est celle de P-s6, pas de l'import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// En °C.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Photo {
    pub fichier: String,
    /// Les dimensions du fichier écrit, relevées à l'import.
    pub largeur: u32,
    pub hauteur: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    /// Le mode de transport (`flight`, `car`…), absent quand la source ne le
    /// sait pas ou n'en nomme aucun.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Les étapes aux deux bouts, par rang dans [`Voyage::etapes`], quand le
    /// bout est une étape et non un point de la trace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<usize>,
    /// La polyligne décodée, vide quand la source n'en donne pas de lisible.
    #[serde(default)]
    pub polyligne: Vec<Point>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointDeTrace {
    pub lat: f64,
    pub lon: f64,
    pub instant: DateTime<Utc>,
}

/// Ce que l'import a pris et laissé, dit avant que l'album se compose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bilan {
    pub etapes: usize,
    pub etapes_sans_photo: usize,
    pub etapes_avec_meteo: usize,
    pub pays: usize,
    pub photos: usize,
    pub videos_ignorees: u32,
    /// Les photos dont le côté court est sous [`SEUIL_COTE_COURT_PX`].
    pub photos_sous_seuil: usize,
    pub segments: usize,
    pub points_de_trace: usize,
    pub trace_presente: bool,
}

impl Voyage {
    pub fn bilan(&self) -> Bilan {
        let photos = self.etapes.iter().flat_map(|e| e.photos.iter());
        Bilan {
            etapes: self.etapes.len(),
            etapes_sans_photo: self.etapes.iter().filter(|e| e.photos.is_empty()).count(),
            etapes_avec_meteo: self.etapes.iter().filter(|e| e.meteo.is_some()).count(),
            pays: self.pays.len(),
            photos: photos.clone().count(),
            videos_ignorees: self.videos_ignorees,
            photos_sous_seuil: photos
                .filter(|p| p.largeur.min(p.hauteur) < SEUIL_COTE_COURT_PX)
                .count(),
            segments: self.segments.len(),
            points_de_trace: self.trace.len(),
            trace_presente: !self.trace.is_empty(),
        }
    }
}

impl std::fmt::Display for Bilan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "étapes : {} ({} sans photo, {} avec météo)", self.etapes, self.etapes_sans_photo, self.etapes_avec_meteo)?;
        writeln!(f, "pays : {}", self.pays)?;
        writeln!(f, "photos : {} ({} sous le repère de {} px de côté court)", self.photos, self.photos_sous_seuil, SEUIL_COTE_COURT_PX)?;
        writeln!(f, "vidéos ignorées : {}", self.videos_ignorees)?;
        writeln!(f, "trajets : {}", self.segments)?;
        if self.trace_presente {
            write!(f, "trace : {} points", self.points_de_trace)
        } else {
            write!(f, "trace : absente (la carte se repliera sur la suite des étapes)")
        }
    }
}

/// Ce qui empêche d'écrire ou de lire un `voyage.json`.
#[derive(Debug)]
pub enum ErreurFichier {
    Io(String),
    Illisible(String),
    /// Écrit par un Booklibre plus récent.
    TropRecent { trouvee: u32 },
}

impl std::fmt::Display for ErreurFichier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurFichier::Io(e) => write!(f, "voyage.json : {e}"),
            ErreurFichier::Illisible(e) => write!(f, "voyage.json illisible : {e}"),
            ErreurFichier::TropRecent { trouvee } => write!(
                f,
                "voyage.json est en version {trouvee}, ce Booklibre lit jusqu'à la {VERSION} : mets-le à jour"
            ),
        }
    }
}

impl std::error::Error for ErreurFichier {}

/// Écrit `voyage.json` dans `dossier`, d'un bloc (fichier voisin puis
/// renommage) : un fichier à moitié écrit ne se lit jamais.
pub fn ecrire(dossier: &Path, voyage: &Voyage) -> Result<(), ErreurFichier> {
    let texte = serde_json::to_string_pretty(voyage).map_err(|e| ErreurFichier::Io(e.to_string()))?;
    let tmp = dossier.join(format!("{FICHIER}.part"));
    std::fs::write(&tmp, texte).map_err(|e| ErreurFichier::Io(e.to_string()))?;
    std::fs::rename(&tmp, dossier.join(FICHIER)).map_err(|e| ErreurFichier::Io(e.to_string()))
}

/// Lit `voyage.json` dans `dossier`.
pub fn lire(dossier: &Path) -> Result<Voyage, ErreurFichier> {
    let texte = std::fs::read_to_string(dossier.join(FICHIER)).map_err(|e| ErreurFichier::Io(e.to_string()))?;
    // La version d'abord, seule : un format plus récent peut avoir changé de
    // forme, et l'erreur à dire est « trop récent », pas « champ manquant ».
    #[derive(Deserialize)]
    struct Entete {
        version: u32,
    }
    let entete: Entete = serde_json::from_str(&texte).map_err(|e| ErreurFichier::Illisible(e.to_string()))?;
    if entete.version > VERSION {
        return Err(ErreurFichier::TropRecent { trouvee: entete.version });
    }
    serde_json::from_str(&texte).map_err(|e| ErreurFichier::Illisible(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemple() -> Voyage {
        let instant = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        Voyage {
            version: VERSION,
            titre: "Un voyage 🌍".into(),
            resume: None,
            debut: Some(instant("2024-05-01T04:00:00+00:00")),
            fin: None,
            fuseau: Some("Europe/Paris".into()),
            distance_km: Some(12.5),
            pays: vec!["FR".into()],
            etapes: vec![Etape {
                nom: "Lyon".into(),
                recit: Some("Départ 🎒".into()),
                debut: instant("2024-05-01T06:15:00.123456+00:00"),
                fin: None,
                fuseau: "Europe/Paris".into(),
                lieu: Lieu { lat: 45.7485, lon: 4.8467, localite: Some("Lyon".into()), pays: None, code_pays: Some("FR".into()) },
                meteo: Some(Meteo { condition: Some("rain".into()), temperature: Some(12.5) }),
                photos: vec![Photo { fichier: "e001-p01-aaaaaaaa.jpg".into(), largeur: 960, hauteur: 1280 }],
            }],
            segments: vec![Segment { mode: Some("flight".into()), de: Some(0), a: None, polyligne: vec![Point { lat: 0.1, lon: -0.2 }] }],
            trace: vec![PointDeTrace { lat: 45.0, lon: 4.0, instant: instant("2024-05-01T06:00:00+00:00") }],
            videos_ignorees: 2,
            couverture: None,
            provenance: Provenance { source: "polarsteps-lien".into(), reference: Some("4242".into()), format: Some("api-62".into()) },
        }
    }

    fn dossier(nom: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("booklibre-voyage-mod-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn voyage_json_fait_l_aller_retour() {
        let d = dossier("aller-retour");
        let v = exemple();
        ecrire(&d, &v).unwrap();
        assert_eq!(lire(&d).unwrap(), v);
        // Lisible à la main : indenté, versionné, les accents tels quels.
        let texte = std::fs::read_to_string(d.join(FICHIER)).unwrap();
        assert!(texte.contains("\"version\": 1") && texte.contains("Départ 🎒") && texte.contains('\n'));
        assert!(!d.join("voyage.json.part").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn une_version_plus_recente_est_refusee_telle_quelle() {
        let d = dossier("recent");
        // Même si le reste a changé de forme : on dit « trop récent », pas « champ manquant ».
        std::fs::write(d.join(FICHIER), r#"{"version": 2, "tout": "autre"}"#).unwrap();
        assert!(matches!(lire(&d), Err(ErreurFichier::TropRecent { trouvee: 2 })));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn un_fichier_illisible_ou_absent_se_dit() {
        let d = dossier("illisible");
        assert!(matches!(lire(&d), Err(ErreurFichier::Io(_))));
        std::fs::write(d.join(FICHIER), "{ pas du json").unwrap();
        assert!(matches!(lire(&d), Err(ErreurFichier::Illisible(_))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn un_champ_optionnel_efface_a_la_main_se_relit() {
        let d = dossier("main");
        let mut v: serde_json::Value = serde_json::to_value(exemple()).unwrap();
        v.as_object_mut().unwrap().remove("trace");
        v.as_object_mut().unwrap().remove("segments");
        v["etapes"][0].as_object_mut().unwrap().remove("meteo");
        std::fs::write(d.join(FICHIER), v.to_string()).unwrap();
        let lu = lire(&d).unwrap();
        assert!(lu.trace.is_empty() && lu.segments.is_empty() && lu.etapes[0].meteo.is_none());
        let b = lu.bilan();
        assert_eq!((b.photos, b.photos_sous_seuil, b.videos_ignorees, b.trace_presente), (1, 1, 2, false));
        let _ = std::fs::remove_dir_all(&d);
    }
}
