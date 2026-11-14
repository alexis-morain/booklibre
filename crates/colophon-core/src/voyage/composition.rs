//! Composer un album depuis un voyage (P-s2).
//!
//! Le moteur n'apprend pas le nom d'une source : il lit `voyage.json`
//! (`super`) à côté des photos. Ce module est la moitié « décisions » de la
//! composition d'un voyage, pure autant que possible :
//!
//! - **une étape est un chapitre**, dans l'ordre du fichier, et ses photos
//!   suivent l'étape (pas leur date EXIF, souvent absente d'une vignette) ;
//! - **le récit est un bloc de texte libre** (`Contenu::Texte`) dans la face
//!   de l'album : court, il tient sur la page de gauche de l'ouverture, à
//!   côté des premières photos ; long, il a sa page de récit, avant les
//!   photos ; sans photo, l'étape est une page de texte. Ce qui ne tient pas
//!   dans deux pages se **dit** (`Resume::deborde`), jamais coupé ;
//! - **les émojis** restent dans `voyage.json`, ne s'impriment pas, et se
//!   comptent ;
//! - **la mention** d'une étape (sa date locale, son lieu) se dérive des
//!   données de l'étape, sans passer par le gazetteer.

use super::{Etape, Voyage, FICHIER};
use crate::layout::{Composer, TexteOuvert};
use crate::model::{Alignement, Contenu, Legendes, Objet, Spread};
use crate::pdf::{self, SpreadGeometry};
use crate::pipeline::Chapter;
use chrono::NaiveDate;
use std::path::Path;

/// Ce que la composition trouve dans un dossier.
pub enum Lecture {
    /// Pas de `voyage.json` : un dossier ordinaire, composé comme toujours.
    Absent,
    Voyage(Box<Voyage>),
    /// Un `voyage.json` qui ne se lit pas (illisible, d'une version plus
    /// récente) : le dossier se compose comme un dossier ordinaire et le bilan
    /// le dit. Le patron des sidecars Takeout, qui se taisent sur ce qu'ils ne
    /// savent pas lire ; ici le fichier existe, donc on le dit. Rien n'est
    /// perdu : les photos sont là, c'est l'ordre des étapes qui manque.
    Refus(String),
}

pub fn lecture(dossier: &Path) -> Lecture {
    if !dossier.join(FICHIER).is_file() {
        return Lecture::Absent;
    }
    match super::lire(dossier) {
        Ok(v) => Lecture::Voyage(Box::new(v)),
        Err(e) => Lecture::Refus(e.to_string()),
    }
}

/// La date de l'étape dans son fuseau. `None` quand le fuseau est inconnu :
/// une date fausse d'un jour est pire que pas de date.
pub fn jour_local(e: &Etape) -> Option<NaiveDate> {
    let t = jiff::Timestamp::from_second(e.debut.timestamp()).ok()?;
    let d = t.in_tz(&e.fuseau).ok()?.date();
    NaiveDate::from_ymd_opt(i32::from(d.year()), u32::from(d.month() as u8), u32::from(d.day() as u8))
}

/// Le lieu que la mention nomme : la localité de l'étape, sinon son pays.
pub fn lieu_nomme(e: &Etape) -> Option<&str> {
    e.lieu
        .localite
        .as_deref()
        .or(e.lieu.pays.as_deref())
        .filter(|l| !l.trim().is_empty())
}

/// La mention de l'étape : sa date locale et son lieu, selon le réglage de
/// l'album. Dérivée de l'étape, jamais saisie, jamais du gazetteer.
pub fn mention(e: &Etape, r: &Legendes) -> Option<String> {
    let lieu = lieu_nomme(e).map(|l| sans_emojis(l).0);
    crate::legende::texte_de_mention(jour_local(e), lieu.as_deref(), r)
}

/// Le réglage que reçoit un album de voyage qui n'en a pas : la date et le
/// lieu, dans la langue de la composition.
pub fn legendes_par_defaut(langue: crate::langue::Langue) -> Legendes {
    Legendes {
        contenu: crate::model::Mention::DateLieu,
        date: crate::model::FormatDate::Jour,
        langue,
    }
}

// --- les émojis ---------------------------------------------------------------

fn est_pictogramme(c: char) -> bool {
    matches!(c as u32,
        0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B50 | 0x2B55 | 0x2B05..=0x2B07
        | 0x2B1B | 0x2B1C | 0x231A | 0x231B | 0x23E9..=0x23F3 | 0x23F8..=0x23FA
        | 0x203C | 0x2049 | 0x2934 | 0x2935 | 0x3030 | 0x303D | 0x3297 | 0x3299)
}

fn est_regional(c: char) -> bool {
    matches!(c as u32, 0x1F1E6..=0x1F1FF)
}

/// Retire les émojis d'un texte et les compte : une suite jointe (ZWJ, teinte
/// de peau, sélecteur de variante) en vaut un, un drapeau (deux lettres
/// régionales) aussi. Un texte sans émoji revient **à l'identique**.
pub fn sans_emojis(texte: &str) -> (String, usize) {
    let mut sortie = String::with_capacity(texte.len());
    let mut compte = 0usize;
    let (mut apres_zwj, mut drapeau_ouvert) = (false, false);
    for c in texte.chars() {
        match c as u32 {
            0x200D => {
                apres_zwj = true;
                continue;
            }
            0xFE0F | 0x20E3 | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F => continue,
            _ => {}
        }
        if est_pictogramme(c) {
            if apres_zwj {
                apres_zwj = false;
                continue;
            }
            if est_regional(c) {
                if drapeau_ouvert {
                    drapeau_ouvert = false;
                    continue;
                }
                drapeau_ouvert = true;
            } else {
                drapeau_ouvert = false;
            }
            compte += 1;
            continue;
        }
        apres_zwj = false;
        drapeau_ouvert = false;
        sortie.push(c);
    }
    if compte == 0 {
        return (texte.to_string(), 0);
    }
    // L'émoji parti laisse des blancs : on les resserre, ligne par ligne.
    let propre = sortie
        .split('\n')
        .map(|l| l.split(' ').filter(|m| !m.is_empty()).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    (propre.trim_end().to_string(), compte)
}

// --- le texte ---------------------------------------------------------------------

/// La taille du récit : celle d'une page de texte du livre.
pub const TAILLE_RECIT_PT: f64 = pdf::TEXT_SIZE_PT;
/// La taille de la ligne de mention d'une page de texte.
pub const TAILLE_MENTION_PT: f64 = 9.0;
/// La largeur d'un bloc de récit, en mm : une cinquantaine de signes par
/// ligne en 11 pt, au plus ce que la page offre.
pub const LARGEUR_RECIT_MM: f64 = 135.0;
/// Un récit tient **à côté des photos** quand il occupe au plus la moitié de
/// la hauteur d'une page : la page de gauche reste de l'air autour de lui.
/// Mesuré sur le voyage réel (journal de P-s2) ; le point de départ du plan,
/// 450 signes, laissait en page de récit des textes de huit lignes.
pub fn lignes_pour_etre_court(g: &SpreadGeometry) -> usize {
    lignes_par_page(g, 0.0) / 2
}

fn interligne_mm(pt: f64) -> f64 {
    pt / (72.0 / 25.4) * 1.35
}

/// La hauteur de l'encre de `n` lignes, comme la scène la compte.
fn hauteur_de(n: usize, pt: f64) -> f64 {
    n.saturating_sub(1) as f64 * interligne_mm(pt) + pt / (72.0 / 25.4) * 1.35
}

/// Combien de lignes de récit une page porte, `reserve_mm` déjà prises en
/// haut (la ligne de mention).
pub fn lignes_par_page(g: &SpreadGeometry, reserve_mm: f64) -> usize {
    let pb = pdf::page_box(false, g);
    let libre = pb.h - HAUT_MM - BAS_MM - reserve_mm;
    let taille = TAILLE_RECIT_PT / (72.0 / 25.4) * 1.35;
    if libre < taille {
        return 0;
    }
    ((libre - taille) / interligne_mm(TAILLE_RECIT_PT)).floor() as usize + 1
}

/// Le blanc gardé au-dessus du bloc et sous lui (la légende de chapitre vit
/// dans le pied de la page).
const HAUT_MM: f64 = 12.0;
const BAS_MM: f64 = 16.0;

fn bloc(x: f64, haut: f64, w: f64, lignes: usize, texte: String, pt: f64) -> Objet {
    let h = hauteur_de(lignes, pt);
    Objet {
        x,
        y: haut - h,
        w,
        h,
        angle: 0.0,
        contenu: Contenu::Texte {
            texte,
            taille_pt: pt,
            interligne_mm: None,
            alignement: Alignement::Gauche,
        },
        couleur: None,
        police: None,
    }
}

/// Le titre d'une étape en tête de sa page : celui d'une garde, 18 pt.
pub const TITRE_PT: f64 = crate::garde::TITRE_PT;
const ECART_TITRE_MM: f64 = 3.0;
const ECART_BLOCS_MM: f64 = 8.0;
/// Le texte d'une étape, tel qu'il se pose dans une pile : titre, mention,
/// récit.
#[derive(Debug, Clone)]
pub struct Bloc {
    pub titre: String,
    pub mention: Option<String>,
    pub recit: Option<String>,
}

/// Ce que le bloc pèse, en lignes de récit : le titre en vaut deux.
pub fn poids(b: &Bloc, mesure: &dyn Fn(&str, f64) -> f64) -> usize {
    let lignes = |t: &str, pt| crate::scene::replier(t, LARGEUR_RECIT_MM, pt, mesure).0.len();
    2 * lignes(&b.titre, TITRE_PT).max(1)
        + usize::from(b.mention.is_some())
        + b.recit.as_deref().map_or(0, |r| lignes(r, TAILLE_RECIT_PT))
        + 1
}

/// Empile des blocs de haut en bas à partir de `haut`, sur la largeur d'une
/// page de récit. Rend les objets, la hauteur prise et le nombre de lignes de
/// récit.
fn empiler(
    mesure: &dyn Fn(&str, f64) -> f64,
    blocs: &[Bloc],
    x: f64,
    w: f64,
    haut: f64,
) -> (Vec<Objet>, f64, usize) {
    let mut y = haut;
    let mut objets = Vec::new();
    let mut recit_lignes = 0;
    for (i, b) in blocs.iter().enumerate() {
        if i > 0 {
            y -= ECART_BLOCS_MM;
        }
        let (lt, _) = crate::scene::replier(&b.titre, w, TITRE_PT, mesure);
        if !lt.is_empty() {
            objets.push(bloc(x, y, w, lt.len(), lt.join("\n"), TITRE_PT));
            y -= hauteur_de(lt.len(), TITRE_PT);
        }
        if let Some(m) = &b.mention {
            y -= ECART_TITRE_MM;
            objets.push(bloc(x, y, w, 1, m.clone(), TAILLE_MENTION_PT));
            y -= hauteur_de(1, TAILLE_MENTION_PT);
        }
        if let Some(r) = &b.recit {
            let (lr, _) = crate::scene::replier(r, w, TAILLE_RECIT_PT, mesure);
            if !lr.is_empty() {
                y -= ECART_TITRE_MM;
                objets.push(bloc(x, y, w, lr.len(), lr.join("\n"), TAILLE_RECIT_PT));
                y -= hauteur_de(lr.len(), TAILLE_RECIT_PT);
                recit_lignes += lr.len();
            }
        }
    }
    (objets, haut - y, recit_lignes)
}

// --- l'étape ------------------------------------------------------------------------

/// Où le récit d'une étape a fini sur le papier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// Ni photo ni récit : rien à poser, l'étape est comptée au bilan.
    Rien,
    /// Des photos, pas de récit.
    PhotosSeules,
    /// Une page de texte : l'étape n'a aucune photo gardée.
    PageDeTexte,
    /// Le récit est court : à côté des premières photos.
    ACote,
    /// Le récit est long : sa page de récit, avant les photos.
    EnPage,
    /// Le récit était court mais aucune ouverture n'a tenu les photos à côté
    /// de lui (résolution, orientation) : page de récit.
    EnPageFauteDePlace,
    /// Une étape sans photo au texte court, mise en commun avec d'autres ou
    /// glissée en tête de l'ouverture de la suivante.
    Groupee,
}

/// Ce que le bilan retient d'une étape.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Resume {
    pub nom: String,
    pub placement: Placement,
    /// Signes du récit imprimé, émojis retirés.
    pub signes: usize,
    pub lignes: usize,
    pub deborde: bool,
    pub emojis: usize,
}

impl Resume {
    /// Le résumé d'une étape au moment où on la compose : ce que son texte pèse
    /// est mesuré ici, où il se place est dit par l'appelant.
    pub fn neuf(
        nom: &str,
        recit: Option<&str>,
        emojis: usize,
        placement: Placement,
        mesure: &dyn Fn(&str, f64) -> f64,
    ) -> Self {
        Resume {
            nom: nom.to_string(),
            placement,
            signes: recit.map_or(0, |r| r.chars().count()),
            lignes: recit.map_or(0, |r| {
                crate::scene::replier(r, LARGEUR_RECIT_MM, TAILLE_RECIT_PT, mesure).0.len()
            }),
            deborde: false,
            emojis,
        }
    }
}

// --- le flux du texte ---------------------------------------------------------------

/// Un texte qui attend sa page, avec le rang de son résumé.
#[derive(Debug, Clone)]
pub struct Texte {
    pub bloc: Bloc,
    pub rang: usize,
    /// L'étape n'a aucune photo posable : son texte n'est qu'un texte.
    pub sans_photo: bool,
}

/// Ce qu'une page de texte porte de lignes de récit au plus.
pub fn capacite(g: &SpreadGeometry) -> usize {
    lignes_par_page(g, 0.0)
}

fn poids_page(p: &[Texte], mesure: &dyn Fn(&str, f64) -> f64) -> usize {
    p.iter().map(|t| poids(&t.bloc, mesure)).sum()
}

/// L'abscisse du texte de la page de gauche, celle de la page de droite, et sa
/// largeur.
fn colonne(g: &SpreadGeometry) -> (f64, f64, f64) {
    let gauche = pdf::page_box(false, g);
    let droite = pdf::page_box(true, g);
    let w = LARGEUR_RECIT_MM.min(gauche.w - 10.0);
    (gauche.x + (gauche.w - w) / 2.0, droite.x + (droite.w - w) / 2.0, w)
}

fn blocs_de(p: &[Texte]) -> Vec<Bloc> {
    p.iter().map(|t| t.bloc.clone()).collect()
}

/// Une planche de texte : la page de gauche, et la page de droite quand il y
/// en a une.
fn planche_de_texte(
    g: &SpreadGeometry,
    mesure: &dyn Fn(&str, f64) -> f64,
    gauche: &[Texte],
    droite: Option<&[Texte]>,
) -> Spread {
    let (xg, xd, w) = colonne(g);
    let pb = pdf::page_box(false, g);
    let haut = pb.y + pb.h - HAUT_MM;
    let mut s = Spread::vide();
    s.objets = empiler(mesure, &blocs_de(gauche), xg, w, haut).0;
    if let Some(d) = droite {
        s.objets.extend(empiler(mesure, &blocs_de(d), xd, w, haut).0);
    }
    s.caption = gauche
        .iter()
        .chain(droite.into_iter().flatten())
        .map(|t| t.bloc.titre.clone())
        .find(|t| !t.is_empty());
    s
}

/// Dit aux résumés où leur texte a fini : une étape sans photo qui partage sa
/// page est « groupée », seule elle a sa page de texte. Les étapes aux photos
/// gardent leur placement.
fn marquer(pages: &[&[Texte]], resumes: &mut [Resume], partage: bool) {
    let mut rangs: Vec<usize> = pages.iter().flat_map(|p| p.iter().map(|t| t.rang)).collect();
    rangs.sort_unstable();
    rangs.dedup();
    for t in pages.iter().flat_map(|p| p.iter()).filter(|t| t.sans_photo) {
        resumes[t.rang].placement =
            if partage || rangs.len() > 1 { Placement::Groupee } else { Placement::PageDeTexte };
    }
}

/// Les pages de texte d'un voyage, dans l'ordre du livre. Une planche a deux
/// pages et aucune ne reste blanche : un texte qui finit une page attend la
/// suivante, et la planche s'écrit quand elle a ses deux faces, soit deux pages
/// de texte, soit un texte à gauche et les photos de l'étape à droite
/// ([`Flux::ouvrir`]). Seule la dernière page du livre peut rester seule.
#[derive(Debug, Default)]
pub struct Flux {
    courante: Vec<Texte>,
    finie: Option<Vec<Texte>>,
}

impl Flux {
    /// Ferme la page en cours : elle devient la page de gauche en attente, ou
    /// la page de droite d'une planche de texte quand une page attend déjà.
    fn fermer(
        &mut self,
        g: &SpreadGeometry,
        mesure: &dyn Fn(&str, f64) -> f64,
        resumes: &mut [Resume],
        sp: &mut Vec<Spread>,
    ) {
        if self.courante.is_empty() {
            return;
        }
        let page = std::mem::take(&mut self.courante);
        match self.finie.take() {
            Some(gauche) => {
                marquer(&[&gauche, &page], resumes, false);
                sp.push(planche_de_texte(g, mesure, &gauche, Some(&page)));
            }
            None => self.finie = Some(page),
        }
    }

    /// Ajoute le texte d'une étape. Il rejoint la page en cours s'il y tient,
    /// sinon il en ouvre une ; un récit plus long qu'une page se coupe en
    /// lignes entières sur deux pages, et ce qui dépasse la seconde se **dit**
    /// (`Resume::deborde`), jamais coupé.
    #[allow(clippy::too_many_arguments)]
    pub fn ajouter(
        &mut self,
        g: &SpreadGeometry,
        mesure: &dyn Fn(&str, f64) -> f64,
        bloc: Bloc,
        rang: usize,
        sans_photo: bool,
        resumes: &mut [Resume],
        sp: &mut Vec<Spread>,
    ) {
        let cap = capacite(g);
        let w = poids(&bloc, mesure);
        let texte = |bloc: Bloc| Texte { bloc, rang, sans_photo };
        if w <= cap {
            if w + poids_page(&self.courante, mesure) > cap {
                self.fermer(g, mesure, resumes, sp);
            }
            self.courante.push(texte(bloc));
            return;
        }
        // Plus qu'une page : il commence la sienne.
        self.fermer(g, mesure, resumes, sp);
        let recit = bloc.recit.clone().unwrap_or_default();
        let (lignes, _) = crate::scene::replier(&recit, LARGEUR_RECIT_MM, TAILLE_RECIT_PT, mesure);
        let tete = Bloc { titre: bloc.titre.clone(), mention: bloc.mention.clone(), recit: None };
        let n1 = cap.saturating_sub(poids(&tete, mesure)).max(1).min(lignes.len());
        let (a, b) = lignes.split_at(n1);
        self.courante.push(texte(Bloc { recit: Some(a.join("\n")), ..tete }));
        self.fermer(g, mesure, resumes, sp);
        if !b.is_empty() {
            let reste = Bloc { titre: String::new(), mention: None, recit: Some(b.join("\n")) };
            self.courante.push(texte(reste));
            if b.len() > cap {
                resumes[rang].deborde = true;
                self.fermer(g, mesure, resumes, sp);
            }
        }
    }

    /// Vide ce qui attend : la dernière planche du livre, ou une planche gardée
    /// à la main que le texte ne peut pas ouvrir.
    pub fn vider(
        &mut self,
        g: &SpreadGeometry,
        mesure: &dyn Fn(&str, f64) -> f64,
        resumes: &mut [Resume],
        sp: &mut Vec<Spread>,
    ) {
        self.fermer(g, mesure, resumes, sp);
        if let Some(gauche) = self.finie.take() {
            marquer(&[&gauche], resumes, false);
            sp.push(planche_de_texte(g, mesure, &gauche, None));
        }
    }

    /// Ouvre une étape aux photos posables. Le texte qui attend (celui de
    /// l'étape et des étapes sans photo avant elle) occupe la page de gauche et
    /// les premières photos remplissent la page de droite : le texte tient en
    /// tête de la page de gauche, photos dessous, quand il pèse au plus une
    /// demi-page ; sinon il prend la page et les photos sont à droite. Une page
    /// qui attendait déjà se joint à lui en planche de texte. Sans texte, les
    /// photos se composent seules.
    #[allow(clippy::too_many_arguments)]
    pub fn ouvrir(
        &mut self,
        composer: &mut Composer,
        mesure: &dyn Fn(&str, f64) -> f64,
        root: &Path,
        chapitre: &Chapter,
        legende: Option<String>,
        resumes: &mut [Resume],
        sp: &mut Vec<Spread>,
    ) {
        let g = *composer.geometrie();
        if let Some(finie) = self.finie.take() {
            if self.courante.is_empty() {
                self.courante = finie;
            } else {
                self.finie = Some(finie);
                self.fermer(&g, mesure, resumes, sp);
            }
        }
        // Le titre de l'étape ouvre déjà une planche de texte rendue ici : elle
        // porte la légende du chapitre, les photos n'en reçoivent pas une
        // seconde.
        let legende = legende.filter(|l| !sp.iter().any(|s| s.caption.as_deref() == Some(l)));
        if self.courante.is_empty() {
            sp.extend(composer.compose(chapitre, legende, root));
            return;
        }
        let textes = std::mem::take(&mut self.courante);
        let blocs = blocs_de(&textes);
        let (xg, _, w) = colonne(&g);
        let pb = pdf::page_box(false, &g);
        let mut photos = chapitre.photos.clone();
        composer.promouvoir_ouverture(&mut photos);
        let barre = Composer::barre_d_ouverture(&photos);
        let mut essais = Vec::new();
        if poids_page(&textes, mesure) <= lignes_pour_etre_court(&g) {
            let (_, h, _) = empiler(mesure, &blocs, xg, w, 0.0);
            essais.push(TexteOuvert::Tete((h + 8.0).ceil() as u32));
        }
        essais.push(TexteOuvert::Page);
        for texte in essais {
            let mut reste = photos.clone();
            let Some(mut ouverture) = composer.ouverture_avec_texte(&mut reste, barre, root, texte)
            else {
                continue;
            };
            let haut = match texte {
                TexteOuvert::Tete(_) => pb.y + pb.h,
                TexteOuvert::Page => pb.y + pb.h - HAUT_MM,
            };
            ouverture.objets = empiler(mesure, &blocs, xg, w, haut).0;
            ouverture.caption = legende;
            marquer(&[&textes], resumes, true);
            sp.push(ouverture);
            if !reste.is_empty() {
                let suite = Chapter { photos: reste, start: chapitre.start, end: chapitre.end };
                sp.extend(composer.compose(&suite, None, root));
            }
            return;
        }
        // Aucune grille ne tient ces photos à côté du texte (résolution,
        // orientation) : la page de texte seule, et la page de droite reste
        // blanche. Le bilan ne le cache pas : l'étape passe en « page de
        // récit faute de place ».
        for t in textes.iter().filter(|t| !t.sans_photo) {
            resumes[t.rang].placement = Placement::EnPageFauteDePlace;
        }
        marquer(&[&textes], resumes, true);
        let mut planche = planche_de_texte(&g, mesure, &textes, None);
        planche.caption = legende;
        sp.push(planche);
        sp.extend(composer.compose(chapitre, None, root));
    }
}

/// Le bilan d'une composition de voyage, tel que le rapport le porte et que
/// la CLI et l'écran le disent.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Bilan {
    pub titre: String,
    pub etapes: usize,
    /// Étapes qui n'ont rien donné au livre : ni photo gardée ni récit.
    pub etapes_vides: usize,
    pub pages_de_texte: usize,
    /// Étapes sans photo au texte court, regroupées avec d'autres ou en tête
    /// de l'ouverture de la suivante : chapitre à part dans le modèle, sans
    /// planche à elles.
    pub etapes_groupees: usize,
    pub recits_a_cote: usize,
    pub recits_en_page: usize,
    /// Planches dont la page de droite reste blanche : aucune grille ne tient
    /// les photos de l'étape à côté de son texte.
    pub pages_de_droite_blanches: usize,
    /// Noms des étapes dont le récit dépasse deux pages.
    pub recits_debordants: Vec<String>,
    pub emojis_non_imprimes: usize,
    /// Photos que `voyage.json` nomme et que le dossier n'a pas (ou que la
    /// curation a écartées avant l'étape).
    pub photos_absentes: usize,
    /// Photos du dossier qu'aucune étape ne nomme : à la réserve.
    pub photos_hors_etapes: usize,
    /// Une photo de couverture venait de la source.
    pub couverture_de_la_source: bool,
    /// `voyage.json` existait mais ne se lisait pas : le dossier s'est composé
    /// comme un dossier ordinaire, et c'est la phrase.
    pub refus: Option<String>,
}

impl Bilan {
    pub fn de(titre: &str, etapes: usize, resumes: &[Resume]) -> Self {
        let mut b = Bilan { titre: titre.to_string(), etapes, ..Default::default() };
        for r in resumes {
            b.emojis_non_imprimes += r.emojis;
            match r.placement {
                Placement::Rien => b.etapes_vides += 1,
                Placement::PhotosSeules => {}
                Placement::PageDeTexte => b.pages_de_texte += 1,
                Placement::ACote => b.recits_a_cote += 1,
                Placement::EnPage => b.recits_en_page += 1,
                Placement::EnPageFauteDePlace => {
                    b.recits_en_page += 1;
                    b.pages_de_droite_blanches += 1;
                }
                Placement::Groupee => b.etapes_groupees += 1,
            }
            if r.deborde {
                b.recits_debordants.push(r.nom.clone());
            }
        }
        b
    }

    /// Les lignes que la CLI et le journal de progression disent.
    pub fn phrases(&self) -> Vec<String> {
        let mut p = Vec::new();
        if let Some(r) = &self.refus {
            p.push(format!("voyage.json ignoré, dossier composé comme un dossier ordinaire : {r}"));
            return p;
        }
        p.push(format!(
            "voyage « {} » : {} étapes, une par chapitre ; {} récits à côté des photos, {} en page de récit, {} pages de texte",
            self.titre, self.etapes, self.recits_a_cote, self.recits_en_page, self.pages_de_texte
        ));
        if self.etapes_vides > 0 {
            p.push(format!("{} étapes sans photo ni récit n'ont rien donné au livre", self.etapes_vides));
        }
        if self.etapes_groupees > 0 {
            p.push(format!(
                "{} étapes sans photo au texte court sont regroupées avec une autre page au lieu d'avoir leur planche",
                self.etapes_groupees
            ));
        }
        if self.pages_de_droite_blanches > 0 {
            p.push(format!(
                "{} planches ont leur page de droite blanche : aucune photo de l'étape ne tient à côté de son texte",
                self.pages_de_droite_blanches
            ));
        }
        if self.emojis_non_imprimes > 0 {
            p.push(format!(
                "{} émojis non imprimés (gardés dans voyage.json)",
                self.emojis_non_imprimes
            ));
        }
        if self.photos_absentes > 0 {
            p.push(format!("{} photos du voyage ne sont pas dans le dossier", self.photos_absentes));
        }
        if self.photos_hors_etapes > 0 {
            p.push(format!(
                "{} photos du dossier ne sont dans aucune étape : elles sont dans la réserve",
                self.photos_hors_etapes
            ));
        }
        for nom in &self.recits_debordants {
            p.push(format!(
                "le récit de l'étape « {nom} » ne tient pas sur ses deux pages : il dépasse, à corriger dans l'éditeur"
            ));
        }
        p
    }
}

#[cfg(test)]
mod tests;
