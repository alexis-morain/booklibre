//! Le fond de carte et le tracé d'un voyage (P-s4) : `carte::dessiner`.
//!
//! **Aucun objet de la scène, aucun rendu** : ce module rend un [`Dessin`], des
//! chemins en millimètres rangés par calque, et P-s5 le posera dans l'album.
//! Les chemins sont ceux des ornements (`ornement::Chemin`, droites
//! seulement) ; ce que l'ornement ne sait pas (le trait, le pointillé) est
//! dans [`Style`], par calque.
//!
//! Le fond est Natural Earth (domaine public), simplifié en un actif unique
//! (`assets/carte/carte.bin`, `compilation.rs`, `scripts/carte.sh`). Aucune
//! tuile, aucun réseau, aucune crate de projection : les trois projections
//! sont écrites à la main (`projection.rs`).
//!
//! ## Ce que `dessiner` décide
//!
//! - **Le centre** : la longitude *médiane circulaire* des étapes (celle des
//!   étapes qui minimise la somme des écarts angulaires). La couture du
//!   monde est à son antipode, loin de ce que le voyage a visité ; ce que la
//!   couture traverse malgré tout (un tracé, un continent) est rogné et
//!   ramené de chaque côté (`geometrie::decouper_fuseau`).
//! - **Le niveau et la projection**, selon l'étendue des points (étapes et
//!   tracé, en diagonale) : moins de 60 km, une *ville* (Mercator) ; moins de
//!   2 500 km, une *région* (équirectangulaire, corrigée par le cosinus de la
//!   latitude du milieu) ; au-delà, le *monde* (Equal Earth). Le fond est le
//!   1:10m des deux premiers, le 1:50m du dernier.
//! - **L'emprise** : la boîte des points plus 10 % de marge, jamais plus
//!   étroite que 10 km, jamais plus allongée que 2,1 pour 1 ; le plus grand
//!   côté vaut [`COTE_MM`]. Tout ce que le dessin porte tient dans cette
//!   boîte : le fond, le tracé et les étapes sont **rognés à ses bords**.
//! - **Le tracé** : Douglas-Peucker à [`TOLERANCE_TRACE_MM`], au plus
//!   [`SOMMETS_MAX`] sommets (la tolérance grandit jusqu'à y tenir).
//!
//! ## Une empreinte sur une grille
//!
//! Chaque coordonnée est ramenée au micromètre avant toute simplification ou
//! rognage : `sin`, `ln` et `atan2` ne rendent pas le même dernier bit sur les
//! trois systèmes, mais personne ne les voit au micromètre. [`Dessin::empreinte`]
//! lit cette grille, pas le flottant.

pub mod actif;
pub mod compilation;
pub mod geometrie;
pub mod projection;
pub mod trace;

use crate::ornement::{Chemin, Segment};
use crate::voyage::Voyage;
use actif::{Actif, Couche, ECHELLE};
use geometrie::{decouper_fuseau, douglas_peucker, envelopper, Rect, P};
use projection::Projection;
use serde::Serialize;
use std::ops::Range;
use std::sync::OnceLock;

/// Le voyage d'un album : `voyage.json` à côté de ses photos (`album.root`),
/// lu seulement si une planche porte une carte. `None` quand l'album n'en
/// porte aucune, ou quand le fichier manque ou ne se lit pas : une carte sans
/// voyage ne dessine rien, et le prévol le dit.
pub fn voyage_de(album: &crate::model::Album) -> Option<Voyage> {
    let porte = album.spreads.iter().any(|s| s.objets.iter().any(|o| matches!(o.contenu, crate::model::Contenu::Carte { .. })));
    if !porte {
        return None;
    }
    crate::voyage::lire(std::path::Path::new(&album.root)).ok()
}

/// Le dessin de rendu de chaque carte de l'album, par plage : `"debut-fin"`.
/// C'est ce que l'écran lit, du même `dessiner` que le PDF, et il n'en dessine
/// jamais d'autre. Une plage que le voyage ne tient plus est absente : l'écran
/// la dit vide, comme le prévol.
pub fn dessins_de_l_album(album: &crate::model::Album) -> std::collections::BTreeMap<String, Dessin> {
    let mut sortie = std::collections::BTreeMap::new();
    let Some(voyage) = voyage_de(album) else { return sortie };
    for o in album.spreads.iter().flat_map(|s| s.objets.iter()) {
        if let crate::model::Contenu::Carte { debut, fin } = o.contenu {
            let cle = format!("{debut}-{fin}");
            if sortie.contains_key(&cle) {
                continue;
            }
            if let Ok(d) = dessiner(&voyage, debut..fin) {
                sortie.insert(cle, d.pour_le_rendu());
            }
        }
    }
    sortie
}

/// L'actif compilé, embarqué.
pub const ACTIF: &[u8] = include_bytes!("../../assets/carte/carte.bin");

/// Le plus grand côté du dessin, en mm : une carte pleine page de l'album.
pub const COTE_MM: f64 = 180.0;
/// Le plafond de sommets du tracé (plan D9).
pub const SOMMETS_MAX: usize = 500;
/// La tolérance du tracé, en mm à l'échelle d'impression (plan D9).
pub const TOLERANCE_TRACE_MM: f64 = 0.2;

const MARGE: f64 = 0.10;
const RAPPORT_MAX: f64 = 2.1;
/// Plus près que cela, la carte dépasse ce que le 1:10m sait dessiner (un
/// kilomètre) : un voyage dans un seul quartier reste une carte de 30 km.
const ETENDUE_MIN_KM: f64 = 30.0;
const VILLE_KM: f64 = 60.0;
const REGION_KM: f64 = 2500.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Niveau {
    Ville,
    Region,
    Monde,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Terre,
    Lacs,
    Rivieres,
    Frontieres,
    Cotes,
    /// Le tracé au sol.
    Trace,
    /// Le tracé aérien : arcs de grand cercle.
    Vol,
    /// Les étapes, en petits disques (P-s5) : jamais produit par
    /// [`dessiner`], ajouté au dessin de rendu par [`Dessin::pour_le_rendu`].
    Reperes,
}

/// Comment un calque se pose. Les mesures sont en mm **à la taille du dessin**
/// (`COTE_MM`) : P-s5 les met à l'échelle de la boîte.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum Style {
    /// Les chemins sont fermés, à remplir.
    Remplir,
    /// Les chemins sont ouverts (ou fermés), à tracer ; `pointille` : plein, vide.
    Tracer { epaisseur_mm: f64, pointille: Option<[f64; 2]> },
}

impl Role {
    pub fn style(self) -> Style {
        match self {
            Role::Terre | Role::Lacs => Style::Remplir,
            Role::Rivieres => Style::Tracer { epaisseur_mm: 0.08, pointille: None },
            Role::Frontieres => Style::Tracer { epaisseur_mm: 0.10, pointille: None },
            Role::Cotes => Style::Tracer { epaisseur_mm: 0.15, pointille: None },
            Role::Trace => Style::Tracer { epaisseur_mm: 0.50, pointille: None },
            Role::Vol => Style::Tracer { epaisseur_mm: 0.40, pointille: Some([1.2, 0.9]) },
            Role::Reperes => Style::Remplir,
        }
    }

    /// L'encre du calque, `#rrggbb`. **Elle est ici et nulle part ailleurs** :
    /// le PDF la lit, et l'écran la lit du dump, comme les chemins.
    pub fn encre(self) -> &'static str {
        match self {
            Role::Terre => "#f1ead8",
            Role::Lacs => "#cfe3ee",
            Role::Rivieres => "#8fb4cc",
            Role::Frontieres => "#b9a98f",
            Role::Cotes => "#6b7f8c",
            Role::Trace | Role::Vol | Role::Reperes => "#c0392b",
        }
    }

    /// Le contour de même encre qu'un calque rempli porte, en mm à la taille
    /// du dessin. La terre est coupée à l'antiméridien en deux moitiés qui se
    /// touchent : un lecteur anti-crénelé laisse entre elles un filet de
    /// papier, et un contour de la couleur de la terre le recouvre.
    pub fn contour_mm(self) -> Option<f64> {
        match self {
            Role::Terre => Some(0.12),
            _ => None,
        }
    }

    fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Calque {
    pub role: Role,
    pub style: Style,
    /// `#rrggbb`, celle de [`Role::encre`].
    pub encre: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contour_mm: Option<f64>,
    pub chemins: Vec<Chemin>,
}

impl Calque {
    pub fn de(role: Role, chemins: Vec<Chemin>) -> Self {
        Calque { role, style: role.style(), encre: role.encre(), contour_mm: role.contour_mm(), chemins }
    }
}

/// Une étape posée sur la carte, en mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Repere {
    /// Le rang de l'étape dans `Voyage::etapes`.
    pub etape: usize,
    pub x: f64,
    pub y: f64,
}

/// Une carte dessinée : `viewbox` est `[0, 0, largeur, hauteur]` en mm, **y
/// vers le bas** comme celui d'un ornement.
#[derive(Debug, Clone, Serialize)]
pub struct Dessin {
    pub viewbox: [f64; 4],
    pub niveau: Niveau,
    pub projection: Projection,
    /// Dans l'ordre où ils se posent : la terre d'abord, le tracé en dernier.
    pub calques: Vec<Calque>,
    pub reperes: Vec<Repere>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Erreur {
    /// La plage ne contient aucune étape du voyage.
    PlageVide,
    /// L'actif embarqué est illisible : un défaut de fabrication, pas d'usage.
    Actif(String),
}

impl std::fmt::Display for Erreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Erreur::PlageVide => write!(f, "carte : aucune étape dans la plage"),
            Erreur::Actif(e) => write!(f, "carte : {e}"),
        }
    }
}

impl std::error::Error for Erreur {}

fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// Le rayon d'un repère d'étape, en mm à la taille du dessin.
pub const REPERE_RAYON_MM: f64 = 0.9;

impl Dessin {
    /// Le dessin tel que les deux rendus le posent : celui de [`dessiner`],
    /// plus un dernier calque, les étapes en disques. Les repères restent des
    /// points dans `dessiner` (et dans ses empreintes d'or) ; ils deviennent
    /// des chemins ici, **une seule fois**, pour que le PDF et l'écran
    /// n'aient à interpréter rien d'autre qu'un calque.
    pub fn pour_le_rendu(mut self) -> Dessin {
        const K: f64 = 0.552_284_749_8; // le quart de cercle en cubiques
        let r = REPERE_RAYON_MM;
        let chemins: Vec<Chemin> = self
            .reperes
            .iter()
            .map(|p| {
                let (x, y) = (p.x, p.y);
                let c = K * r;
                Chemin {
                    segments: vec![
                        Segment::Vers { x: x + r, y },
                        Segment::Courbe { x1: x + r, y1: y + c, x2: x + c, y2: y + r, x, y: y + r },
                        Segment::Courbe { x1: x - c, y1: y + r, x2: x - r, y2: y + c, x: x - r, y },
                        Segment::Courbe { x1: x - r, y1: y - c, x2: x - c, y2: y - r, x, y: y - r },
                        Segment::Courbe { x1: x + c, y1: y - r, x2: x + r, y2: y - c, x: x + r, y },
                        Segment::Ferme,
                    ],
                    evenodd: false,
                }
            })
            .collect();
        if !chemins.is_empty() {
            self.calques.push(Calque::de(Role::Reperes, chemins));
        }
        self
    }

    /// Le rapport largeur/hauteur du dessin, que la boîte d'une carte garde.
    pub fn rapport(&self) -> f64 {
        self.viewbox[2] / self.viewbox[3]
    }

    /// Tous les points de tous les chemins, et les repères.
    pub fn points(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        let chemins = self.calques.iter().flat_map(|c| c.chemins.iter()).flat_map(|c| c.segments.iter()).filter_map(|s| match *s {
            Segment::Vers { x, y } | Segment::Ligne { x, y } | Segment::Courbe { x, y, .. } => Some((x, y)),
            Segment::Ferme => None,
        });
        chemins.chain(self.reperes.iter().map(|r| (r.x, r.y)))
    }

    /// Les sommets du tracé (au sol et aérien).
    pub fn sommets_du_trace(&self) -> usize {
        self.calques
            .iter()
            .filter(|c| matches!(c.role, Role::Trace | Role::Vol))
            .flat_map(|c| c.chemins.iter())
            .map(|c| c.segments.iter().filter(|s| !matches!(s, Segment::Ferme)).count())
            .sum()
    }

    pub fn calque(&self, role: Role) -> Option<&Calque> {
        self.calques.iter().find(|c| c.role == role)
    }

    /// FNV-1a 64 de ce que le dessin porte, lu au micromètre : deux dessins
    /// d'une même entrée portent la même empreinte, sur tous les systèmes.
    pub fn empreinte(&self) -> String {
        let mut h = Fnv(0xcbf29ce484222325);
        let g = |v: f64| (v * 1000.0).round() as i64;
        for v in self.viewbox {
            h.i64(g(v));
        }
        h.i64(self.niveau as i64);
        for c in &self.calques {
            h.i64(c.role.code() as i64);
            h.i64(c.chemins.len() as i64);
            for ch in &c.chemins {
                h.i64(ch.evenodd as i64);
                h.i64(ch.segments.len() as i64);
                for s in &ch.segments {
                    match *s {
                        Segment::Vers { x, y } => (h.i64(1), h.i64(g(x)), h.i64(g(y))),
                        Segment::Ligne { x, y } => (h.i64(2), h.i64(g(x)), h.i64(g(y))),
                        Segment::Courbe { x, y, .. } => (h.i64(3), h.i64(g(x)), h.i64(g(y))),
                        Segment::Ferme => (h.i64(4), (), ()),
                    };
                }
            }
        }
        for r in &self.reperes {
            h.i64(r.etape as i64);
            h.i64(g(r.x));
            h.i64(g(r.y));
        }
        format!("{:016x}", h.0)
    }
}

struct Fnv(u64);

impl Fnv {
    fn i64(&mut self, v: i64) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}

fn actif() -> Result<&'static Actif<'static>, Erreur> {
    static LU: OnceLock<Result<Actif<'static>, String>> = OnceLock::new();
    LU.get_or_init(|| Actif::lire(ACTIF)).as_ref().map_err(|e| Erreur::Actif(e.clone()))
}

/// La longitude médiane circulaire : parmi les longitudes données, celle qui
/// minimise la somme des écarts angulaires aux autres ; à égalité, la plus
/// petite. Pour des longitudes qui tiennent sur un côté du monde, c'est la
/// médiane ordinaire ; pour `[179, -179]`, l'une ou l'autre, jamais 0.
pub fn longitude_mediane(lons: &[f64]) -> f64 {
    let somme = |c: f64| lons.iter().map(|l| envelopper(l - c).abs()).sum::<f64>();
    let mut meilleur = (f64::MAX, 0.0);
    for &c in lons {
        let s = somme(c);
        if s < meilleur.0 - 1e-9 || ((s - meilleur.0).abs() <= 1e-9 && c < meilleur.1) {
            meilleur = (s, c);
        }
    }
    meilleur.1
}

/// Le centre d'une carte du monde : l'antipode du milieu du plus grand vide de
/// longitudes entre les points. Les points tiennent alors du même côté de la
/// couture, qui tombe là où personne n'a dormi ; à égalité de deux vides, le
/// plus à l'ouest.
pub fn centre_du_monde(points: &[P]) -> f64 {
    let mut lons: Vec<f64> = points.iter().map(|&(_, lon)| envelopper(lon)).collect();
    lons.sort_by(|a, b| a.partial_cmp(b).unwrap());
    lons.dedup();
    let Some(&premier) = lons.first() else { return 0.0 };
    // Le vide qui suit chaque longitude, en tournant ; le dernier boucle sur le premier.
    let (mut meilleur, mut debut) = (f64::MIN, premier);
    for (i, &l) in lons.iter().enumerate() {
        let suivante = lons.get(i + 1).copied().unwrap_or(premier + 360.0);
        let vide = suivante - l;
        if vide > meilleur + 1e-9 {
            meilleur = vide;
            debut = l;
        }
    }
    let couture = debut + meilleur / 2.0;
    envelopper(couture + 180.0)
}

/// Le cadre : du plan de la projection aux millimètres.
struct Cadre {
    x0: f64,
    y1: f64,
    echelle: f64,
    largeur: f64,
    hauteur: f64,
    /// Les bornes du cadre dans le plan : `(x0, y0, x1, y1)`.
    plan: (f64, f64, f64, f64),
}

impl Cadre {
    fn mm(&self, p: P) -> P {
        (r3((p.0 - self.x0) * self.echelle), r3((self.y1 - p.1) * self.echelle))
    }

    fn rect(&self) -> Rect {
        Rect { x0: 0.0, y0: 0.0, x1: self.largeur, y1: self.hauteur }
    }
}

fn cadre(proj: &Projection, points: &[P]) -> Cadre {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in points {
        x0 = x0.min(p.0);
        x1 = x1.max(p.0);
        y0 = y0.min(p.1);
        y1 = y1.max(p.1);
    }
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let minimum = ETENDUE_MIN_KM / geometrie::RAYON_KM;
    let mut w = (x1 - x0).max(minimum);
    let mut h = (y1 - y0).max(minimum);
    let marge = MARGE * w.max(h);
    w += 2.0 * marge;
    h += 2.0 * marge;
    if w > RAPPORT_MAX * h {
        h = w / RAPPORT_MAX;
    } else if h > RAPPORT_MAX * w {
        w = h / RAPPORT_MAX;
    }
    let (mut ox, mut oy) = (cx - w / 2.0, cy - h / 2.0);
    if let Some((dw, dh)) = proj.demi_monde() {
        w = w.min(2.0 * dw);
        h = h.min(2.0 * dh);
        ox = ox.clamp(-dw, dw - w);
        oy = oy.clamp(-dh, dh - h);
    }
    let echelle = COTE_MM / w.max(h);
    Cadre { x0: ox, y1: oy + h, echelle, largeur: r3(w * echelle), hauteur: r3(h * echelle), plan: (ox, oy, ox + w, oy + h) }
}

/// Le fond d'une couche : ce que l'emprise touche, coupé aux coutures, projeté,
/// rogné au cadre.
struct Fond<'a> {
    proj: &'a Projection,
    cadre: &'a Cadre,
    /// `(lon min, lon max, lat min, lat max)` relatifs, `None` : tout lire.
    fenetre: Option<(f64, f64, f64, f64)>,
}

impl Fond<'_> {
    fn touche(&self, boite: [i32; 4]) -> bool {
        let Some((lo, hi, la0, la1)) = self.fenetre else { return true };
        let e = ECHELLE;
        let (a, b, c, d) = (boite[0] as f64 / e, boite[2] as f64 / e, boite[1] as f64 / e, boite[3] as f64 / e);
        if d < la0 || c > la1 {
            return false;
        }
        let lon0 = self.proj.lon0();
        (-1..=1).any(|m| {
            let s = 360.0 * m as f64 - lon0;
            b + s >= lo && a + s <= hi
        })
    }

    fn couche(&self, actif: &Actif, couche: Couche, role: Role) -> Result<Option<Calque>, Erreur> {
        let surface = couche.est_surface();
        let mut chemins = Vec::new();
        for partie in actif.parties(couche) {
            let partie = partie.map_err(Erreur::Actif)?;
            if !self.touche(partie.boite) {
                continue;
            }
            let pts = partie.points().map_err(Erreur::Actif)?;
            for morceau in decouper_fuseau(&pts, self.proj.lon0(), surface) {
                let plan: Vec<P> = morceau.iter().map(|&(lon, lat)| self.cadre.mm(self.proj.projeter(lon, lat))).collect();
                let rect = self.cadre.rect();
                if surface {
                    let p = arrondir(&rect.polygone(&plan));
                    if p.len() >= 3 && aire(&p) != 0.0 {
                        chemins.push(chemin(&p, true));
                    }
                } else {
                    for l in rect.ligne(&plan) {
                        let l = dedoublonner(&arrondir(&l));
                        if l.len() >= 2 {
                            chemins.push(chemin(&l, false));
                        }
                    }
                }
            }
        }
        Ok((!chemins.is_empty()).then(|| Calque::de(role, chemins)))
    }
}

fn arrondir(p: &[P]) -> Vec<P> {
    p.iter().map(|&(x, y)| (r3(x), r3(y))).collect()
}

fn dedoublonner(p: &[P]) -> Vec<P> {
    let mut s: Vec<P> = Vec::with_capacity(p.len());
    for &q in p {
        if s.last() != Some(&q) {
            s.push(q);
        }
    }
    s
}

fn aire(p: &[P]) -> f64 {
    (0..p.len()).map(|i| p[i].0 * p[(i + 1) % p.len()].1 - p[(i + 1) % p.len()].0 * p[i].1).sum::<f64>() / 2.0
}

fn chemin(p: &[P], ferme: bool) -> Chemin {
    let mut segments = Vec::with_capacity(p.len() + 1);
    for (i, &(x, y)) in p.iter().enumerate() {
        segments.push(if i == 0 { Segment::Vers { x, y } } else { Segment::Ligne { x, y } });
    }
    if ferme {
        segments.push(Segment::Ferme);
    }
    Chemin { segments, evenodd: false }
}

/// La carte des étapes `plage` du voyage (les rangs hors du voyage sont
/// ignorés). Même voyage, même plage : même dessin, à la même empreinte.
pub fn dessiner(voyage: &Voyage, plage: Range<usize>) -> Result<Dessin, Erreur> {
    let plage = plage.start..plage.end.min(voyage.etapes.len());
    if plage.start >= plage.end {
        return Err(Erreur::PlageVide);
    }
    let etapes = &voyage.etapes[plage.clone()];
    let actif = actif()?;

    // Le centre, puis le tracé en géographie.
    let lons: Vec<f64> = etapes.iter().map(|e| e.lieu.lon).collect();
    let pieces = trace::pieces(voyage, &plage);
    let mut geo: Vec<P> = etapes.iter().map(|e| (e.lieu.lat, e.lieu.lon)).collect();
    geo.extend(pieces.iter().flat_map(|p| p.points.iter().copied()));

    // L'étendue, en kilomètres, sur le monde enroulé autour d'un centre.
    let etendue = |lon0: f64| -> (Niveau, f64) {
        let (mut lmin, mut lmax, mut amin, mut amax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(lat, lon) in &geo {
            let rel = envelopper(lon - lon0);
            lmin = lmin.min(rel);
            lmax = lmax.max(rel);
            amin = amin.min(lat);
            amax = amax.max(lat);
        }
        let lat_mid = (amin + amax) / 2.0;
        let largeur_km = (lmax - lmin) * 111.195 * lat_mid.to_radians().cos();
        let hauteur_km = (amax - amin) * 111.195;
        let diagonale = largeur_km.hypot(hauteur_km);
        let niveau = if diagonale < VILLE_KM {
            Niveau::Ville
        } else if diagonale < REGION_KM {
            Niveau::Region
        } else {
            Niveau::Monde
        };
        (niveau, lat_mid)
    };
    // Une ville ou une région se centre sur la médiane des étapes. Le monde
    // place sa couture au milieu du plus grand vide de longitudes **entre les
    // étapes** (P-s5) : la médiane, elle, tombait au milieu des États-Unis sur
    // le voyage réel, entre deux villes visitées. Les étapes seules : un tour
    // du monde a des vols dans tous les vides, et si le tracé comptait, la
    // couture tomberait n'importe où ; ce qu'elle traverse alors est un
    // trajet, coupé et ramené de chaque côté, jamais une ville.
    let mut lon0 = longitude_mediane(&lons);
    let (mut niveau, mut lat_mid) = etendue(lon0);
    if niveau == Niveau::Monde {
        lon0 = centre_du_monde(&etapes.iter().map(|e| (e.lieu.lat, e.lieu.lon)).collect::<Vec<P>>());
        (niveau, lat_mid) = etendue(lon0);
    }
    let proj = match niveau {
        Niveau::Ville => Projection::Mercator { lon0 },
        Niveau::Region => Projection::Equirectangulaire { lon0, lat_ref: lat_mid },
        Niveau::Monde => Projection::EqualEarth { lon0 },
    };

    // Le tracé dans le plan, coupé aux coutures.
    let mut plan_pieces: Vec<(Role, Vec<P>)> = Vec::new();
    for p in &pieces {
        let contour: Vec<P> = p.points.iter().map(|&(lat, lon)| (lon, lat)).collect();
        for m in decouper_fuseau(&contour, lon0, false) {
            let pts = m.iter().map(|&(lon, lat)| proj.projeter(lon, lat)).collect();
            plan_pieces.push((if p.vol { Role::Vol } else { Role::Trace }, pts));
        }
    }
    let plan_etapes: Vec<P> = etapes.iter().map(|e| proj.projeter(envelopper(e.lieu.lon - lon0), e.lieu.lat)).collect();

    // L'emprise : les étapes et le tracé.
    let tous: Vec<P> = plan_etapes.iter().copied().chain(plan_pieces.iter().flat_map(|p| p.1.iter().copied())).collect();
    let cadre = cadre(&proj, &tous);
    let rect = cadre.rect();

    // Le fond.
    let fenetre = match (&proj, niveau) {
        (_, Niveau::Monde) => None,
        _ => {
            let (x0, y0, x1, y1) = cadre.plan;
            let (a, b) = (proj.inverse(x0, y0), proj.inverse(x1, y1));
            a.zip(b).map(|((lo0, la0), (lo1, la1))| {
                let pad = 0.02 * (lo1 - lo0).abs().max(la1 - la0);
                (lo0.min(lo1) - pad, lo0.max(lo1) + pad, la0.min(la1) - pad, la0.max(la1) + pad)
            })
        }
    };
    let fond = Fond { proj: &proj, cadre: &cadre, fenetre };
    let couches: &[(Couche, Role)] = if niveau == Niveau::Monde {
        &[(Couche::Terre50, Role::Terre), (Couche::Frontieres50, Role::Frontieres), (Couche::Cotes50, Role::Cotes)]
    } else {
        &[
            (Couche::Terre10, Role::Terre),
            (Couche::Lacs10, Role::Lacs),
            (Couche::Rivieres10, Role::Rivieres),
            (Couche::Frontieres10, Role::Frontieres),
            (Couche::Cotes10, Role::Cotes),
        ]
    };
    let mut calques = Vec::new();
    for &(couche, role) in couches {
        calques.extend(fond.couche(actif, couche, role)?);
    }

    // Le tracé : en mm, recollé, simplifié sous le plafond, rogné.
    let mut en_mm: Vec<(Role, Vec<P>)> = Vec::new();
    for (role, pts) in &plan_pieces {
        let pts = dedoublonner(&pts.iter().map(|&p| cadre.mm(p)).collect::<Vec<_>>());
        if pts.len() < 2 {
            continue;
        }
        match en_mm.last_mut() {
            Some((r, dernier)) if r == role && dist(*dernier.last().unwrap(), pts[0]) <= 0.5 => dernier.extend_from_slice(&pts[1..]),
            _ => en_mm.push((*role, pts)),
        }
    }
    let mut tolerance = TOLERANCE_TRACE_MM;
    let simplifie = loop {
        let s: Vec<(Role, Vec<P>)> = en_mm
            .iter()
            .flat_map(|(role, pts)| {
                let simple = arrondir(&douglas_peucker(pts, tolerance));
                rect.ligne(&simple).into_iter().map(move |l| (*role, arrondir(&l)))
            })
            .collect();
        let n: usize = s.iter().map(|p| p.1.len()).sum();
        if n <= SOMMETS_MAX || tolerance > COTE_MM {
            break s;
        }
        tolerance *= 1.5;
    };
    for role in [Role::Trace, Role::Vol] {
        let chemins: Vec<Chemin> = simplifie.iter().filter(|(r, _)| *r == role).map(|(_, p)| chemin(p, false)).collect();
        if !chemins.is_empty() {
            calques.push(Calque::de(role, chemins));
        }
    }

    let reperes = plan_etapes
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let (x, y) = cadre.mm(p);
            Repere { etape: plage.start + i, x: x.clamp(0.0, cadre.largeur), y: y.clamp(0.0, cadre.hauteur) }
        })
        .collect();

    Ok(Dessin { viewbox: [0.0, 0.0, cadre.largeur, cadre.hauteur], niveau, projection: proj, calques, reperes })
}

fn dist(a: P, b: P) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

#[cfg(test)]
mod tests;
