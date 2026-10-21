//! The cover, as the printer receives it: one wide sheet, laid flat.
//!
//! Left to right, that is the back cover, the spine, and the front. The
//! interior is a book of double pages; the cover is a single piece of card
//! that wraps around it, so it is its own file with its own geometry and its
//! own bleed.
//!
//! Two things here belong to the printer profile and never to this code. The
//! **spine** exists only when the supplier asks us for it
//! ([`Dos::Calcule`]); Prodigi builds its own and gets a sheet without one.
//! The **bleed** is the profile's, edge by edge, and it is not the interior's:
//! a cover is trimmed on all four sides, so the spine-side value that exists
//! for an interior page has no meaning here.
//!
//! Every measurement below starts at the trim, never at the sheet edge. A
//! title placed 9 mm from the card is not 9 mm from the finished book, and
//! that difference is exactly what the preflight caught on the captions.

#[cfg(test)]
use crate::font;

use crate::model::{Album, Cover};
use crate::pdf::{self, Boxes, PdfWriter, Rect};
use crate::printer::{Fichiers, PrinterProfile, GRAMMAGE_DEFAUT};
use crate::{couleur, meta, print};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Title size on a 210 mm wide cover, scaled with the format from there.
const TITLE_PT_AT_210: f64 = 26.0;
const SUBTITLE_PT_AT_210: f64 = 12.0;
pub const BACK_TEXT_PT_AT_210: f64 = 10.5;
const SPINE_PT_AT_210: f64 = 11.0;

/// The title block sits this far in from the trim, as a share of the panel.
/// Same 9 % the cover editor shows, so what is on screen is what prints.
const TITLE_INSET: f64 = 0.09;

/// Below this, a spine is a fold and not a surface: nothing is printed on it.
/// Type on a 6 mm spine misses the fold by more than its own height.
pub const SPINE_TEXT_MIN_MM: f64 = 9.0;

/// Line spacing of the back-cover text, in multiples of its size.
const BACK_LEADING: f64 = 1.45;

/// White, for type over a photo, and the hard shadow under it. A blurred
/// halo is a screen effect; on paper it prints as a smudge, so the shadow is
/// one hard offset instead, the same shape the editor now shows.
const PAPER: [f64; 3] = [0.99, 0.97, 0.94];
const SHADOW: [f64; 3] = [0.09, 0.08, 0.06];
const SHADOW_OFFSET: f64 = 0.45;

/// The flat sheet, in millimetres, origin bottom-left of the card.
///
/// `back`, `spine` and `front` are **trim** rectangles: the finished panels,
/// with the bleed already excluded. The sheet around them is what the knife
/// takes off.
#[derive(Debug, Clone)]
pub struct CoverGeometry {
    pub media_w: f64,
    pub media_h: f64,
    /// Bleed actually applied, edge by edge, from the profile.
    pub bleed_ext: f64,
    pub bleed_haut: f64,
    pub bleed_bas: f64,
    pub back: Rect,
    /// Absent when the supplier builds the spine itself.
    pub spine: Option<Rect>,
    pub front: Rect,
    /// Keep-clear distance from the trim, from the profile.
    pub safe: f64,
}

impl CoverGeometry {
    /// The trim rectangle of the whole sheet: `[x0, y0, x1, y1]`.
    pub fn trim(&self) -> [f64; 4] {
        [
            self.bleed_ext,
            self.bleed_bas,
            self.media_w - self.bleed_ext,
            self.media_h - self.bleed_haut,
        ]
    }

    /// Spine width in millimetres, zero when there is none.
    pub fn spine_mm(&self) -> f64 {
        self.spine.as_ref().map_or(0.0, |r| r.w)
    }
}

/// Work out the flat sheet for an album at a supplier.
///
/// The width is the whole point of this function, and the one number the
/// definition of done asks to measure: twice the page, plus the spine the
/// profile computes, plus the hinge groove on each side of it, plus what the
/// board takes on the outside.
///
/// The sheet **grows around its contents, it never moves them**. A supplier
/// who binds a soft cover carries the three case-wrap cotes at zero, and
/// then every number below is the one this function has always returned:
/// the boards are a case of the same arithmetic, not a branch beside it.
pub fn geometry(album: &Album, profil: &PrinterProfile) -> CoverGeometry {
    let pages = album.spreads.len() * 2;
    let spine_w = profil.dos_mm(pages, GRAMMAGE_DEFAUT).unwrap_or(0.0);
    let b = &profil.bleed_mm;
    let (ext, haut, bas) = (b.exterieur, b.haut, b.bas);
    let (rempli, debord, mors) = (profil.rempli_mm, profil.debord_mm, profil.mors_mm);

    // What lies between the edge of the card and the finished panel: the
    // bleed the knife takes, then the board's overhang, then the turn-in
    // that folds behind it. The same three on all four sides.
    let marge = ext + rempli + debord;
    let media_w = album.trim_mm.w * 2.0 + spine_w + mors * 2.0 + marge * 2.0;
    let media_h = album.trim_mm.h + (haut + rempli + debord) + (bas + rempli + debord);
    let panel = |x: f64, w: f64| Rect { x, y: bas + rempli + debord, w, h: album.trim_mm.h };

    let back = panel(marge, album.trim_mm.w);
    let spine = profil
        .dos_mm(pages, GRAMMAGE_DEFAUT)
        .map(|w| panel(marge + album.trim_mm.w + mors, w));
    let front = panel(marge + album.trim_mm.w + mors + spine_w + mors, album.trim_mm.w);

    CoverGeometry {
        media_w,
        media_h,
        bleed_ext: ext,
        bleed_haut: haut,
        bleed_bas: bas,
        back,
        spine,
        front,
        safe: profil.safe_mm,
    }
}

/// Render the flat cover sheet to `out`, at print resolution.
///
/// The name is the caller's, not ours: the delivery is `album-cover.pdf`
/// (`--cover`), the app's preview `album-cover.apercu.pdf`. Only the first is
/// judged by the preflight, so a function that named its own file would put
/// every preview under the delivery's rules.
///
/// Fails loudly on a missing original, like the interior render: a cover with
/// a hole in it costs a reprint of the whole book.
pub fn render_cover_pdf(
    dir: &Path,
    profil: &'static PrinterProfile,
    out: &Path,
) -> Result<PathBuf> {
    render_cover_pdf_at(dir, profil, out, crate::pdfx::stamp())
}

/// [`render_cover_pdf`] at a pinned instant: the one field of the file that
/// is not a function of the album, split out so byte identity is testable.
fn render_cover_pdf_at(
    dir: &Path,
    profil: &'static PrinterProfile,
    out: &Path,
    stamp: chrono::DateTime<chrono::Local>,
) -> Result<PathBuf> {
    let json = dir.join("album.json");
    let album: Album = serde_json::from_str(
        &fs::read_to_string(&json).with_context(|| format!("lecture de {}", json.display()))?,
    )
    .context("album.json illisible")?;

    let g = geometry(&album, profil);
    // An album composed before the cover editor has no cover: its title is
    // the one thing we know, and it is enough for a first sheet.
    let cover = album.cover.clone().unwrap_or(Cover {
        title: album.title.clone(),
        subtitle: String::new(),
        photo: None,
        back_text: String::new(),
        fond: None,
        ..Default::default()
    });

    let mut writer = PdfWriter::with_stamp(&album, dir, stamp);
    let mut content = String::new();
    let mut xobjects = lopdf::dictionary! {};
    let fond = cover.fond_rgb();
    if let Some(rgb) = fond {
        aplat(&mut content, &g, rgb);
    }

    // The photo bleeds off the top, the bottom and the outer edge, and stops
    // dead at the fold: an image running onto the spine is folded in half.
    if let Some(slot) = &cover.photo {
        let root = std::path::PathBuf::from(&album.root);
        anyhow::ensure!(
            root.is_dir(),
            "dossier de photos introuvable : {} (déplacé ou disque absent ?)",
            root.display()
        );
        let src = root.join(&slot.src);
        let rect = photo_rect(&g);
        let orientation = meta::read(&src).orientation;
        // The adjustment is keyed by source, so a photo on the cover and on
        // a spread is adjusted once and prints the same on both.
        let asset = print::print_asset(
            &src,
            orientation,
            &rect,
            slot.focal,
            slot.zoom,
            album.reglages.get(&slot.src),
        )
        .with_context(|| format!("photo de couverture : {}", slot.src))?;
        writer.draw_image(&mut content, &mut xobjects, 0, &asset, &rect);
    }

    draw_back_photo(&mut writer, &mut content, &mut xobjects, &album, &g, &cover)?;
    draw_text(&mut content, &mut writer.ecrivain, &g, &album, &cover, encre_sur(fond));

    writer.add_page(
        Boxes { media: [g.media_w, g.media_h], trim: g.trim() },
        content,
        xobjects,
    );

    crate::print::poser_pdf(writer, out)
}

/// Where the front-cover photo goes: everything to the right of the spine,
/// out to the three edges of the card.
///
/// Two numbers, and each is a decision. The **width** is what is left of the
/// sheet, not the panel plus a bleed: on a case-wrap cover the card runs on
/// past the finished page by the board's overhang and the turn-in, and a
/// photograph that stopped at the old width would leave the last centimetres
/// of the front white. On a sheet without boards the two are the same number,
/// which is what lets this stay one expression.
///
/// The **origin** is the right edge of the spine, so the image runs *through*
/// the hinge groove instead of starting at it. A case is assembled to a
/// millimetre or two, and a white line in the groove would be visible on
/// every copy; a photograph folded into the groove is not. There is nothing
/// to read there, so nothing is lost — the rule this obeys is that the mors
/// carries no text, not that it carries no ink.
pub fn photo_rect(g: &CoverGeometry) -> Rect {
    let x = g.spine.as_ref().map_or(g.front.x, |s| s.x + s.w);
    Rect {
        x,
        y: 0.0,
        w: g.media_w - x,
        h: g.media_h,
    }
}

/// Where the front photograph goes on a single leaf: the whole card. A leaf
/// has no fold to stop it at, so it bleeds on all four sides.
pub fn leaf_photo_rect(g: &CoverGeometry) -> Rect {
    Rect { x: 0.0, y: 0.0, w: g.media_w, h: g.media_h }
}

/// The rectangle the front photograph is printed into **at this supplier**,
/// whichever shape the cover takes there.
///
/// One function rather than a `match` at every caller, because the two shapes
/// are not the same surface at all: on a 210 mm book the flat case-wrap sheet
/// prints it into 239 × 258 mm and a single leaf into 216 × 216. Anything
/// that measures the cover — the preflight's resolution rule first of all —
/// reads it here, or it measures a rectangle nobody prints.
pub fn photo_rect_du_profil(album: &Album, profil: &PrinterProfile) -> Rect {
    match profil.fichiers {
        Fichiers::Deux => photo_rect(&geometry(album, profil)),
        Fichiers::Un => {
            leaf_photo_rect(&page_geometry(album, profil, Face::Premiere))
        }
    }
}

/// Which face of the cover a single leaf carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Première de couverture: the photo, the title, the subtitle.
    Premiere,
    /// Quatrième de couverture: the dedication, and nothing else.
    Quatrieme,
}

/// The cover as a single leaf, for a supplier that binds one file and reads
/// its first and last page as the cover.
///
/// Not a flat sheet cut in two. A leaf is trimmed on all four sides and has
/// no fold, so there is no spine here and no spine-side edge to protect: the
/// supplier wraps the boards themselves and the width they need is theirs to
/// know, which is exactly why they ask for a single file.
pub fn page_geometry(album: &Album, profil: &PrinterProfile, face: Face) -> CoverGeometry {
    let b = &profil.bleed_mm;
    let (ext, haut, bas) = (b.exterieur, b.haut, b.bas);
    let media_w = album.trim_mm.w + ext * 2.0;
    let media_h = album.trim_mm.h + haut + bas;
    let panel = Rect { x: ext, y: bas, w: album.trim_mm.w, h: album.trim_mm.h };
    // The face this leaf does not carry is given no surface at all, so a
    // caller that draws the wrong one draws nothing rather than something in
    // the wrong corner.
    let absent = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
    let (back, front) = match face {
        Face::Premiere => (absent, panel),
        Face::Quatrieme => (panel, absent),
    };

    CoverGeometry {
        media_w,
        media_h,
        bleed_ext: ext,
        bleed_haut: haut,
        bleed_bas: bas,
        back,
        spine: None,
        front,
        safe: profil.safe_mm,
    }
}

/// Draw one cover face as the next page of `writer`.
///
/// The same title, photo and dedication the flat sheet carries, on a leaf
/// instead of a panel. Called by the interior render for the suppliers that
/// take a single file, so that the first page of that file is the front
/// cover and the last is the back.
pub(crate) fn add_cover_page(
    writer: &mut PdfWriter,
    album: &Album,
    profil: &PrinterProfile,
    face: Face,
) -> Result<()> {
    let g = page_geometry(album, profil, face);
    let cover = album.cover.clone().unwrap_or(Cover {
        title: album.title.clone(),
        subtitle: String::new(),
        photo: None,
        back_text: String::new(),
        fond: None,
        ..Default::default()
    });

    let mut content = String::new();
    let mut xobjects = lopdf::dictionary! {};
    let fond = cover.fond_rgb();
    if let Some(rgb) = fond {
        aplat(&mut content, &g, rgb);
    }
    let encre = encre_sur(fond);

    match face {
        Face::Premiere => {
            // On a leaf the photo bleeds on all four sides: there is no fold
            // to stop it at.
            if let Some(slot) = &cover.photo {
                let root = std::path::PathBuf::from(&album.root);
                anyhow::ensure!(
                    root.is_dir(),
                    "dossier de photos introuvable : {} (déplacé ou disque absent ?)",
                    root.display()
                );
                let src = root.join(&slot.src);
                let rect = leaf_photo_rect(&g);
                let orientation = meta::read(&src).orientation;
                let asset = print::print_asset(
                    &src,
                    orientation,
                    &rect,
                    slot.focal,
                    slot.zoom,
                    album.reglages.get(&slot.src),
                )
                .with_context(|| format!("photo de couverture : {}", slot.src))?;
                writer.draw_image(&mut content, &mut xobjects, 0, &asset, &rect);
            }
            draw_front(&mut content, &mut writer.ecrivain, &g, album, &cover, encre);
        }
        Face::Quatrieme => {
            draw_back_photo(writer, &mut content, &mut xobjects, album, &g, &cover)?;
            draw_back(&mut content, &mut writer.ecrivain, &g, album, &cover, encre)
        }
    }

    writer.add_page(
        Boxes { media: [g.media_w, g.media_h], trim: g.trim() },
        content,
        xobjects,
    );
    Ok(())
}

/// Type on the three panels. Sizes scale with the page so a 30 × 30 album
/// does not wear a 21 × 21 album's title.
///
/// The three faces draw separately because they do not always share a sheet:
/// a supplier that binds a single file gets the front and the back as two
/// leaves of the interior, with no spine between them.
fn draw_text(
    content: &mut String,
    ecrivain: &mut pdf::Ecrivain,
    g: &CoverGeometry,
    album: &Album,
    cover: &Cover,
    encre: [f64; 3],
) {
    draw_front(content, ecrivain, g, album, cover, encre);
    draw_spine(content, ecrivain, g, album, cover, encre);
    draw_back(content, ecrivain, g, album, cover, encre);
}

/// The fond: one flat fill of the whole card, bleed included, laid before
/// anything else so the photograph and the type print over it. The front,
/// the spine and the back are one colour because they are one piece of card.
///
/// The rectangle is rounded **up** to the hundredth of a point: rounded to
/// nearest it could stop a few thousandths short of the card's edge, and a
/// hairline of white at the knife is exactly what the bleed is there to
/// prevent.
fn aplat(content: &mut String, g: &CoverGeometry, rgb: [f64; 3]) {
    let pt = |mm: f64| (mm * 72.0 / 25.4 * 100.0).ceil() / 100.0;
    let (r, v, b) = (rgb[0], rgb[1], rgb[2]);
    content.push_str(&format!(
        "q {r} {v} {b} rg 0 0 {:.2} {:.2} re f Q\n",
        pt(g.media_w),
        pt(g.media_h)
    ));
}

/// The ink of the type that sits on the card rather than on the photograph:
/// the book's own ink on a bare sheet, so a cover without a fond is the one
/// this project has always printed, and otherwise whichever of that ink and
/// the paper white reads on the fond ([`couleur::encre_lisible`]).
fn encre_sur(fond: Option<[f64; 3]>) -> [f64; 3] {
    fond.map_or(pdf::TEXT_INK, |f| couleur::encre_lisible(f, pdf::TEXT_INK, PAPER))
}

/// The title the book wears, from the album alone: the cover's when it was
/// given one of its own, the album's name otherwise. The flat sheet reads it
/// through [`cover_title`] with the cover it is rendering; everything else
/// that has to print the book's name (the half-title, [`crate::garde`]) asks
/// here, so a book called « Un été » on its cover is not called something
/// else three pages later.
pub fn titre_du_livre(album: &Album) -> &str {
    match album.cover.as_ref() {
        Some(c) if !c.title.trim().is_empty() => c.title.as_str(),
        _ => album.title.as_str(),
    }
}

/// The title of the album, whether or not the cover editor has been opened.
fn cover_title<'a>(album: &'a Album, cover: &'a Cover) -> &'a str {
    if cover.title.is_empty() { album.title.as_str() } else { cover.title.as_str() }
}

/// Front: title block, bottom left, inside the trim by the same share the
/// editor shows. Baselines stack upward from the subtitle.
fn draw_front(
    content: &mut String,
    ecrivain: &mut pdf::Ecrivain,
    g: &CoverGeometry,
    album: &Album,
    cover: &Cover,
    encre: [f64; 3],
) {
    let scale = album.trim_mm.w / 210.0;
    let title_pt = TITLE_PT_AT_210 * scale;
    let subtitle_pt = SUBTITLE_PT_AT_210 * scale;
    let over_photo = cover.photo.is_some();

    let x = g.front.x + g.front.w * TITLE_INSET;
    let y = g.front.y + g.front.h * TITLE_INSET;
    let title = cover_title(album, cover);
    let (subtitle_y, title_y) = if cover.subtitle.is_empty() {
        (y, y)
    } else {
        (y, y + title_pt * PT_TO_MM * 1.25)
    };
    plate(content, ecrivain, x, subtitle_y, subtitle_pt, over_photo, encre, &cover.subtitle);
    plate(content, ecrivain, x, title_y, title_pt, over_photo, encre, title);
}

/// Spine: the title along the fold, running bottom to top, centred. Only when
/// there is a surface to print on, and never on a single leaf.
fn draw_spine(
    content: &mut String,
    ecrivain: &mut pdf::Ecrivain,
    g: &CoverGeometry,
    album: &Album,
    cover: &Cover,
    encre: [f64; 3],
) {
    let scale = album.trim_mm.w / 210.0;
    if let Some(spine) = &g.spine {
        if spine.w >= SPINE_TEXT_MIN_MM {
            let title = cover_title(album, cover);
            let size = SPINE_PT_AT_210 * scale;
            let width = ecrivain.largeur_mm(title, size);

            let cx = spine.x + spine.w / 2.0 - size * PT_TO_MM * 0.35;
            let cy = spine.y + (spine.h - width) / 2.0;
            rotated(content, ecrivain, cx, cy, size, encre, title);
        }
    }
}

/// The bounds a type size read from `album.json` is held to: the file is
/// repaired by hand, and a 0 pt or a 400 pt quatrième is a typo, not a book.
pub const BACK_TAILLE_MIN_PT: f64 = 6.0;
pub const BACK_TAILLE_MAX_PT: f64 = 48.0;

/// Where a line's ink sits around its baseline, as a share of the type size.
/// A proxy, the same on both sides and for every face: what it decides is
/// whether a line climbs onto the photo or into the safe band, and the
/// margins it is compared with are centimetres, not hairlines.
pub const ENCRE_HAUT: f64 = 0.75;
pub const ENCRE_BAS: f64 = 0.25;

/// The size the quatrième prints at: the one the cover was given, held to
/// its bounds, or the format's own when it was given none — which is the
/// size every quatrième printed at before the field existed.
pub fn taille_quatrieme(cover: &Cover, trim_w: f64) -> f64 {
    match cover.back_taille_pt.filter(|t| t.is_finite()) {
        Some(t) => t.clamp(BACK_TAILLE_MIN_PT, BACK_TAILLE_MAX_PT),
        None => BACK_TEXT_PT_AT_210 * (trim_w / 210.0),
    }
}

/// The square frame of the back photograph, in the panel's frame.
///
/// The side is a share of the shorter side of the inner box — the panel less
/// [`TITLE_INSET`] on every edge, the inset the front's title keeps — so the
/// frame sits in the page whatever its format, never at the bleed. Centred
/// across; against the top or the bottom of the inner box, or centred.
pub fn cadre_quatrieme(panneau: &Rect, p: &crate::model::PhotoQuatrieme) -> Rect {
    use crate::model::PositionQuatrieme as P;
    let (iw, ih) = (panneau.w * (1.0 - 2.0 * TITLE_INSET), panneau.h * (1.0 - 2.0 * TITLE_INSET));
    let cote = p.taille.part() * iw.min(ih);
    let bas = panneau.y + panneau.h * TITLE_INSET;
    let y = match p.position {
        P::Haut => bas + ih - cote,
        P::Centre => panneau.y + (panneau.h - cote) / 2.0,
        P::Bas => bas,
    };
    Rect { x: panneau.x + (panneau.w - cote) / 2.0, y, w: cote, h: cote }
}

/// One line of the quatrième: its text, its baseline, its measured width.
#[derive(Debug, Clone, PartialEq)]
pub struct LigneQuatrieme {
    pub texte: String,
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
}

/// The back cover laid out: the photo frame, the band the text is centred
/// in, the lines that print, and what did not fit.
#[derive(Debug, Clone)]
pub struct Quatrieme {
    pub cadre: Option<Rect>,
    pub bande: Rect,
    pub lignes: Vec<LigneQuatrieme>,
    pub taille_pt: f64,
    /// Lines with words in them that do not print: they would have fallen
    /// under the floor. Never dropped in silence any more — the preflight
    /// counts them ([`crate::prevol`], `quatrieme_deborde`).
    pub coupees: usize,
    /// A single word wider than the measure, left whole.
    pub trop_large: bool,
}

impl Quatrieme {
    /// How many printed lines leave the panel (the knife or the fold takes
    /// them) or climb out of their band (onto the photo), and how many of the
    /// others, or the frame, enter the safe band of the profile.
    pub fn ecarts(&self, panneau: &Rect, safe: f64) -> (usize, usize) {
        let mm = self.taille_pt * PT_TO_MM;
        let haut_bande = self.bande.y + self.bande.h;
        let (mut dehors, mut zone) = (0, 0);
        for l in &self.lignes {
            if l.texte.is_empty() {
                continue;
            }
            let (x0, x1) = (l.x, l.x + l.largeur);
            let (y0, y1) = (l.y - mm * ENCRE_BAS, l.y + mm * ENCRE_HAUT);
            if x0 < panneau.x || x1 > panneau.x + panneau.w || y1 > haut_bande {
                dehors += 1;
            } else if x0 < panneau.x + safe
                || x1 > panneau.x + panneau.w - safe
                || y0 < panneau.y + safe
                || y1 > panneau.y + panneau.h - safe
            {
                zone += 1;
            }
        }
        if let Some(c) = &self.cadre {
            if c.x < panneau.x + safe
                || c.x + c.w > panneau.x + panneau.w - safe
                || c.y < panneau.y + safe
                || c.y + c.h > panneau.y + panneau.h - safe
            {
                zone += 1;
            }
        }
        (dehors + self.coupees, zone)
    }
}

/// The quatrième, laid out on its panel. **The one layout**: the PDF draws
/// it, the preflight measures it, the dump carries it, and the editor's port
/// (`album.ts::quatrieme`) is checked against it.
///
/// Without a photo this is, expression for expression, what the back has
/// always printed: the text wrapped to the inner width, centred on the
/// panel, the block centred in its height, and nothing under the safe band.
/// A photo takes the top, the middle or the bottom; the text then takes the
/// band under it (or above it, when the photo sits at the bottom), one
/// leading clear of the frame.
pub fn quatrieme(
    panneau: &Rect,
    safe: f64,
    cover: &Cover,
    taille_pt: f64,
    mesure: &dyn Fn(&str, f64) -> f64,
) -> Quatrieme {
    use crate::model::PositionQuatrieme as P;
    let size = taille_pt;
    let leading = size * PT_TO_MM * BACK_LEADING;
    let cadre = cover.back_photo.as_ref().map(|p| cadre_quatrieme(panneau, p));
    let bande = match (&cadre, cover.back_photo.as_ref().map(|p| p.position)) {
        (Some(c), Some(P::Bas)) => {
            let y = c.y + c.h + leading;
            Rect { x: panneau.x, y, w: panneau.w, h: panneau.y + panneau.h - y }
        }
        (Some(c), _) => {
            Rect { x: panneau.x, y: panneau.y, w: panneau.w, h: c.y - leading - panneau.y }
        }
        (None, _) => panneau.clone(),
    };
    let mut q = Quatrieme {
        cadre,
        bande: bande.clone(),
        lignes: Vec::new(),
        taille_pt: size,
        coupees: 0,
        trop_large: false,
    };
    if cover.back_text.is_empty() {
        return q;
    }
    let box_w = panneau.w - 2.0 * (panneau.w * TITLE_INSET);
    let (lines, trop_large) = crate::scene::replier(&cover.back_text, box_w, size, mesure);
    q.trop_large = trop_large;
    // Under the band no line prints: on the bare panel that is the safe band
    // of the profile, under a photo it is the band's own foot.
    let plancher = (panneau.y + safe).max(bande.y);

    let block = lines.len() as f64 * leading;
    let mut y = bande.y + (bande.h + block) / 2.0 - leading;
    for (i, line) in lines.iter().enumerate() {
        if i > 0 && y < plancher {
            q.coupees = lines[i..].iter().filter(|l| !l.is_empty()).count();
            break;
        }
        // Centred on the panel, measured on the real advance widths.
        let largeur = mesure(line, size);
        let x = panneau.x + (panneau.w - largeur) / 2.0;
        q.lignes.push(LigneQuatrieme { texte: line.clone(), x, y, largeur });
        y -= leading;
    }
    q
}

/// Back: the quatrième, as [`quatrieme`] lays it out, in its own face when
/// it was given one. The photograph is drawn by the caller, which holds the
/// writer; this only sets the type.
fn draw_back(
    content: &mut String,
    ecrivain: &mut pdf::Ecrivain,
    g: &CoverGeometry,
    album: &Album,
    cover: &Cover,
    encre: [f64; 3],
) {
    if cover.back_text.is_empty() {
        return;
    }
    let face = ecrivain.ressource(cover.back_police.as_ref());
    let size = taille_quatrieme(cover, album.trim_mm.w);
    let q = {
        let e = &*ecrivain;
        quatrieme(&g.back, g.safe, cover, size, &|s, pt| e.largeur_mm_dans(face, s, pt))
    };
    for l in &q.lignes {
        pdf::text_op(content, ecrivain, face, l.x, l.y, size, encre, &l.texte);
    }
}

/// The back photograph, framed in the panel, cropped like any case. Index 1:
/// the front's image is `Im0` on the same page of a flat sheet.
fn draw_back_photo(
    writer: &mut PdfWriter,
    content: &mut String,
    xobjects: &mut lopdf::Dictionary,
    album: &Album,
    g: &CoverGeometry,
    cover: &Cover,
) -> Result<()> {
    let Some(p) = &cover.back_photo else { return Ok(()) };
    let root = std::path::PathBuf::from(&album.root);
    anyhow::ensure!(
        root.is_dir(),
        "dossier de photos introuvable : {} (déplacé ou disque absent ?)",
        root.display()
    );
    let src = root.join(&p.photo.src);
    let rect = cadre_quatrieme(&g.back, p);
    let asset = print::print_asset(
        &src,
        meta::read(&src).orientation,
        &rect,
        p.photo.focal,
        p.photo.zoom,
        album.reglages.get(&p.photo.src),
    )
    .with_context(|| format!("photo de quatrième : {}", p.photo.src))?;
    writer.draw_image(content, xobjects, 1, &asset, &rect);
    Ok(())
}

const PT_TO_MM: f64 = 25.4 / 72.0;

/// A line of cover type: white over the photo with one hard shadow under it,
/// plain `encre` on a bare panel. Two draws rather than a blur, because a blur
/// needs transparency and transparency is what a print file argues about.
fn plate(
    content: &mut String,
    ecrivain: &mut pdf::Ecrivain,
    x: f64,
    y: f64,
    size: f64,
    over_photo: bool,
    encre: [f64; 3],
    s: &str,
) {
    if s.is_empty() {
        return;
    }
    if over_photo {
        pdf::text_op(content, ecrivain, 1, x + SHADOW_OFFSET, y - SHADOW_OFFSET, size, SHADOW, s);
        pdf::text_op(content, ecrivain, 1, x, y, size, PAPER, s);
    } else {
        pdf::text_op(content, ecrivain, 1, x, y, size, encre, s);
    }
}

/// Text turned a quarter turn anticlockwise, so the spine reads bottom to
/// top. One direction had to be picked and this is the one the cover editor
/// shows; a spine printed the other way up is a reprint, so the two sides
/// agree here and the choice is written down rather than left to a default.
fn rotated(
    content: &mut String,
    ecrivain: &mut pdf::Ecrivain,
    x: f64,
    y: f64,
    size: f64,
    rgb: [f64; 3],
    s: &str,
) {
    let mut run = String::new();
    pdf::text_op(&mut run, ecrivain, 1, 0.0, 0.0, size, rgb, s);
    let mm_to_pt = 72.0 / 25.4;
    content.push_str(&format!(
        "q 0 1 -1 0 {:.2} {:.2} cm\n{run}Q\n",
        x * mm_to_pt,
        y * mm_to_pt
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Size;

    fn album_de(spreads: usize) -> Album {
        let mut a = Album::new("Corse", std::path::Path::new("/photos"), Size { w: 210.0, h: 210.0 });
        for _ in 0..spreads {
            a.spreads.push(crate::model::Spread {
                template: "solo".into(),
                slots: vec![],
                caption: None,
                text: None,
                edited: false,
                locked: false,
                objets: Vec::new(),
            });
        }
        a
    }

    /// The sheet is twice the page, plus the spine, plus the two hinge
    /// grooves, plus what the board takes on each outer edge. Measured
    /// against the profile's own numbers rather than against a constant, so a
    /// profile change moves the cover with it.
    #[test]
    fn the_sheet_is_two_pages_plus_the_spine_plus_the_case() {
        let a = album_de(48); // 96 pages
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let g = geometry(&a, cp);
        let spine = cp.dos_mm(96, GRAMMAGE_DEFAUT).unwrap();
        let marge = cp.bleed_mm.exterieur + cp.rempli_mm + cp.debord_mm;

        assert!(
            (g.media_w - (210.0 * 2.0 + spine + cp.mors_mm * 2.0 + marge * 2.0)).abs() < 1e-9,
            "{}",
            g.media_w
        );
        assert!((g.media_h - (210.0 + marge * 2.0)).abs() < 1e-9, "{}", g.media_h);
        // The panels no longer touch the bleed: on a case-wrap cover they sit
        // inland by the board and the turn-in, and a groove separates each of
        // them from the spine.
        assert!((g.back.x - marge).abs() < 1e-9, "{}", g.back.x);
        assert!(
            (g.spine.as_ref().unwrap().x - (g.back.x + g.back.w + cp.mors_mm)).abs() < 1e-9
        );
        assert!(
            (g.front.x - (g.spine.as_ref().unwrap().x + spine + cp.mors_mm)).abs() < 1e-9
        );
        assert!((g.front.x + g.front.w - (g.media_w - marge)).abs() < 1e-9);
        // And the numbers their own template is drawn with: 492 × 258 at a
        // 14 mm spine is what `photobook_cw_s210_s_fc_cover.pdf` measures.
        assert!((g.media_h - 258.0).abs() < 1e-9, "{}", g.media_h);
        assert!(
            (210.0 * 2.0 + 14.0 + cp.mors_mm * 2.0 + marge * 2.0 - 492.0).abs() < 1e-9
        );
    }

    /// A supplier who does not wrap boards gets the sheet this project has
    /// always produced — the panels back against the bleed, the spine
    /// between them, nothing inland. The case-wrap arithmetic has to fold
    /// back to exactly that when the three cotes are zero, or every soft
    /// cover ever exported has quietly changed size.
    #[test]
    fn the_cotes_at_zero_give_back_the_flat_sheet() {
        let a = album_de(48);
        for id in ["prodigi", "lulu", "generique"] {
            let p = PrinterProfile::par_id(id).unwrap();
            let g = geometry(&a, p);
            let spine = p.dos_mm(96, GRAMMAGE_DEFAUT).unwrap_or(0.0);
            let ext = p.bleed_mm.exterieur;
            assert!((g.media_w - (210.0 * 2.0 + spine + ext * 2.0)).abs() < 1e-9, "{id}");
            assert!(
                (g.media_h - (210.0 + p.bleed_mm.haut + p.bleed_mm.bas)).abs() < 1e-9,
                "{id}"
            );
            assert!((g.back.x - ext).abs() < 1e-9, "{id}");
            assert!((g.back.y - p.bleed_mm.bas).abs() < 1e-9, "{id}");
            assert!((g.front.x + g.front.w - (g.media_w - ext)).abs() < 1e-9, "{id}");
            // And the photograph covers what it has always covered.
            let r = photo_rect(&g);
            assert!((r.x - g.front.x).abs() < 1e-9, "{id}");
            assert!((r.w - (g.front.w + ext)).abs() < 1e-9, "{id}");
            assert!((r.h - g.media_h).abs() < 1e-9, "{id}");
        }
    }

    /// A supplier that builds its own spine gets a sheet without one, and the
    /// two panels then meet at the middle of the trim.
    #[test]
    fn a_supplier_that_makes_its_own_spine_gets_none() {
        let a = album_de(40);
        let g = geometry(&a, PrinterProfile::par_id("prodigi").unwrap());
        assert!(g.spine.is_none());
        assert_eq!(g.spine_mm(), 0.0);
        assert!((g.media_w - 210.0 * 2.0).abs() < 1e-9, "Prodigi ne demande aucun fond perdu");
        assert!((g.front.x - (g.back.x + g.back.w)).abs() < 1e-9);
    }

    /// The spine grows with the book. A thin album and a fat one do not get
    /// the same sheet, which is the whole reason the width is computed.
    #[test]
    fn the_sheet_grows_with_the_book() {
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let thin = geometry(&album_de(20), cp);
        let fat = geometry(&album_de(90), cp);
        // The extra width is the extra spine, no more and no less: pinning a
        // millimetre count here would only pin the coefficient of the day.
        let attendu = cp.dos_mm(180, GRAMMAGE_DEFAUT).unwrap() - cp.dos_mm(40, GRAMMAGE_DEFAUT).unwrap();
        assert!(attendu > 0.0, "un dos qui ne grossit pas ne se calcule pas");
        assert!(
            (fat.media_w - thin.media_w - attendu).abs() < 1e-9,
            "{} vs {}, écart attendu {attendu}",
            fat.media_w,
            thin.media_w
        );
        assert!((fat.media_h - thin.media_h).abs() < 1e-9, "la hauteur ne bouge pas");
    }

    /// The photo reaches the four edges of the card and stops at the spine.
    /// It crosses the hinge groove on the way, deliberately: a case is
    /// assembled to a millimetre or two, and a white line down the groove
    /// would show on every copy. What it must never do is reach the spine,
    /// which would fold a face in half.
    #[test]
    fn the_front_photo_crosses_the_groove_and_stops_at_the_spine() {
        let a = album_de(48);
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let g = geometry(&a, cp);
        let spine = g.spine.as_ref().unwrap();
        let r = photo_rect(&g);
        assert!((r.x - (spine.x + spine.w)).abs() < 1e-9, "la photo mord sur le dos");
        assert!(r.x < g.front.x, "elle doit entrer dans le mors, pas s'y arrêter");
        assert!((g.front.x - r.x - cp.mors_mm).abs() < 1e-9, "d'exactement un mors");
        assert_eq!(r.y, 0.0);
        assert!((r.x + r.w - g.media_w).abs() < 1e-9, "elle atteint le bord extérieur");
        assert!((r.h - g.media_h).abs() < 1e-9, "et le haut comme le bas");
    }

    /// La photographie de couverture n'a pas la même surface chez tout le
    /// monde, et c'est une seule fonction qui le sait. Le carton habillé court
    /// au-delà du livre sur les quatre bords ; le feuillet volant s'arrête au
    /// fond perdu. Un prévol qui reconstruirait le rectangle lui-même
    /// mesurerait une couverture que personne n'imprime.
    #[test]
    fn la_photo_de_couverture_a_la_surface_de_son_fournisseur() {
        let a = album_de(24);
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let pr = PrinterProfile::par_id("prodigi").unwrap();

        // Feuille à plat : de la droite du dos au bord du carton, sur toute
        // sa hauteur. C'est bien le rectangle que l'émetteur y pose.
        let boitier = photo_rect_du_profil(&a, cp);
        let attendu = photo_rect(&geometry(&a, cp));
        assert!(
            (boitier.x - attendu.x).abs() < 1e-9
                && (boitier.w - attendu.w).abs() < 1e-9
                && (boitier.h - attendu.h).abs() < 1e-9,
            "{boitier:?} ≠ {attendu:?}"
        );
        assert!((boitier.h - (210.0 + 24.0 * 2.0)).abs() < 1e-9, "{boitier:?}");

        // Feuillet volant : la page plus son fond perdu, et rien de plus.
        // Prodigi n'en demande aucun, donc c'est la page nue.
        let feuillet = photo_rect_du_profil(&a, pr);
        assert!((feuillet.w - 210.0).abs() < 1e-9, "{feuillet:?}");
        assert!((feuillet.h - 210.0).abs() < 1e-9, "{feuillet:?}");
        assert!(boitier.w * boitier.h > feuillet.w * feuillet.h * 1.3);
    }

    /// A thin spine carries no type: the rule is a measurement, not a taste.
    /// The book's name comes from the cover when the cover was given one:
    /// the flat sheet, the leaf and the half-title all print the same string.
    #[test]
    fn the_book_wears_the_covers_title_when_it_has_one() {
        let mut a = album_de(24);
        a.title = "corse-2013".into();
        assert_eq!(titre_du_livre(&a), "corse-2013");
        a.cover = Some(Cover {
            title: "Un été".into(),
            subtitle: String::new(),
            photo: None,
            back_text: String::new(),
            fond: None,
            ..Default::default()
        });
        assert_eq!(titre_du_livre(&a), "Un été");
        // A cover left blank falls back to the album, it never prints nothing.
        a.cover.as_mut().unwrap().title = "   ".into();
        assert_eq!(titre_du_livre(&a), "corse-2013");
    }

    #[test]
    fn a_thin_spine_carries_no_title() {
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        // 24 pages: 12 sheets at 0.135 plus the two boards, 7.62 mm, under the floor.
        let g = geometry(&album_de(12), cp);
        assert!(g.spine_mm() < SPINE_TEXT_MIN_MM, "{}", g.spine_mm());
        let mut content = String::new();
        let cover = Cover {
            title: "Corse".into(),
            subtitle: String::new(),
            photo: None,
            back_text: String::new(),
            fond: None,
            ..Default::default()
        };
        draw_text(&mut content, &mut pdf::Ecrivain::incorporee(), &g, &album_de(12), &cover, pdf::TEXT_INK);
        // The title is on the front and nowhere else: one occurrence. Under
        // Identity-H a string in the stream is glyph ids, so the title is
        // looked for the way it is written.
        let face = font::Embarquee::incorporee().expect("face ouverte");
        let pose: String =
            face.glyphes("Corse").iter().map(|(gid, _)| format!("{gid:04X}")).collect();
        assert_eq!(content.matches(&format!("<{pose}>")).count(), 1, "{content}");
    }

    /// The quatrième's wrap, through the one [`crate::scene::replier`].
    fn wrap(e: &pdf::Ecrivain, text: &str, width: f64, size: f64) -> Vec<String> {
        crate::scene::replier(text, width, size, &|s, pt| e.largeur_mm(s, pt)).0
    }

    fn photo_q(
        position: crate::model::PositionQuatrieme,
        taille: crate::model::TailleQuatrieme,
    ) -> crate::model::PhotoQuatrieme {
        crate::model::PhotoQuatrieme {
            photo: crate::model::Slot::new("a.jpg".into(), [0.5, 0.5]),
            position,
            taille,
        }
    }

    const PANNEAU: Rect = Rect { x: 27.0, y: 24.0, w: 210.0, h: 210.0 };
    /// Six points to the character: wide enough to wrap, simple enough to count.
    fn mesure_simple(s: &str, pt: f64) -> f64 {
        s.chars().count() as f64 * pt * 0.2
    }

    /// The nine frames sit inside the inner box, square, centred across, and
    /// each size is its share of the box.
    #[test]
    fn les_neuf_cadres_tiennent_dans_la_boite_interieure() {
        use crate::model::{PositionQuatrieme as P, TailleQuatrieme as T};
        let p = &PANNEAU;
        let marge = p.w * TITLE_INSET;
        for pos in [P::Haut, P::Centre, P::Bas] {
            for t in [T::Petite, T::Moyenne, T::Grande] {
                let c = cadre_quatrieme(p, &photo_q(pos, t));
                assert!((c.w - c.h).abs() < 1e-9, "carré");
                assert!((c.w - t.part() * p.w * (1.0 - 2.0 * TITLE_INSET)).abs() < 1e-9);
                assert!((c.x + c.w / 2.0 - (p.x + p.w / 2.0)).abs() < 1e-9, "centré");
                assert!(c.y >= p.y + marge - 1e-9 && c.y + c.h <= p.y + p.h - marge + 1e-9);
                match pos {
                    P::Haut => assert!((c.y + c.h - (p.y + p.h - marge)).abs() < 1e-9),
                    P::Bas => assert!((c.y - (p.y + marge)).abs() < 1e-9),
                    P::Centre => assert!((c.y + c.h / 2.0 - (p.y + p.h / 2.0)).abs() < 1e-9),
                }
            }
        }
        // Un format en largeur : le côté suit la hauteur, la plus courte.
        let large = Rect { x: 0.0, y: 0.0, w: 280.0, h: 210.0 };
        let c = cadre_quatrieme(&large, &photo_q(P::Centre, T::Grande));
        assert!((c.w - 2.0 / 3.0 * 210.0 * (1.0 - 2.0 * TITLE_INSET)).abs() < 1e-9);
    }

    /// Le texte prend la bande libre : sous la photo quand elle est en haut ou
    /// au centre, au-dessus quand elle est en bas, une interligne à l'écart.
    #[test]
    fn le_texte_prend_la_bande_que_la_photo_laisse() {
        use crate::model::{PositionQuatrieme as P, TailleQuatrieme as T};
        let mut c = Cover { back_text: "Pour Marie.\nEt la suite.".into(), ..Default::default() };
        for pos in [P::Haut, P::Centre, P::Bas] {
            c.back_photo = Some(photo_q(pos, T::Moyenne));
            let q = quatrieme(&PANNEAU, 7.0, &c, 10.5, &mesure_simple);
            let cadre = q.cadre.clone().unwrap();
            assert_eq!(q.lignes.len(), 2, "{pos:?}");
            assert_eq!(q.coupees, 0);
            let mm = 10.5 * PT_TO_MM;
            for l in &q.lignes {
                if pos == P::Bas {
                    assert!(l.y - mm * ENCRE_BAS > cadre.y + cadre.h, "{pos:?} : au-dessus");
                } else {
                    assert!(l.y + mm * ENCRE_HAUT < cadre.y, "{pos:?} : dessous");
                }
            }
            assert_eq!(q.ecarts(&PANNEAU, 7.0), (0, 0), "{pos:?}");
        }
    }

    /// Ce qui ne tient pas se compte au lieu de disparaître : un texte trop
    /// long sous une grande photo laisse des lignes non imprimées, et un mot
    /// plus large que la page sort du panneau.
    #[test]
    fn ce_qui_ne_tient_pas_se_compte() {
        use crate::model::{PositionQuatrieme as P, TailleQuatrieme as T};
        let long = vec!["ligne"; 60].join("\n");
        let c = Cover {
            back_text: long,
            back_photo: Some(photo_q(P::Haut, T::Grande)),
            ..Default::default()
        };
        let q = quatrieme(&PANNEAU, 7.0, &c, 10.5, &mesure_simple);
        assert!(q.coupees > 0, "{}", q.lignes.len());
        assert_eq!(q.lignes.len() + q.coupees, 60);
        assert!(q.ecarts(&PANNEAU, 7.0).0 > 0);

        let mot = Cover { back_text: "x".repeat(110), ..Default::default() };
        let q = quatrieme(&PANNEAU, 7.0, &mot, 10.5, &mesure_simple);
        assert!(q.trop_large);
        assert_eq!(q.ecarts(&PANNEAU, 7.0), (1, 0), "plus large que la page");
        // Plus large que la boîte, pas que la page : la marge avertit.
        let mot = Cover { back_text: "x".repeat(94), ..Default::default() };
        let q = quatrieme(&PANNEAU, 7.0, &mot, 10.5, &mesure_simple);
        assert_eq!(q.ecarts(&PANNEAU, 7.0), (0, 1), "dans la bande sûre");
    }

    /// La feuille porte la photo de quatrième dans son cadre, et le texte
    /// dans sa face propre, à sa taille ; le feuillet volant aussi.
    #[test]
    fn la_quatrieme_porte_sa_photo_et_sa_face() {
        use crate::model::{PositionQuatrieme as P, TailleQuatrieme as T};
        let dir = std::env::temp_dir().join(format!("colophon-quatrieme-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let img = image::RgbImage::from_pixel(1600, 1600, image::Rgb([70, 110, 160]));
        image::DynamicImage::ImageRgb8(img)
            .save_with_format(dir.join("a.jpg"), image::ImageFormat::Jpeg)
            .unwrap();
        fs::write(dir.join("objet-Essai.ttf"), font::FONT_DATA).unwrap();
        let mut album = album_de(24);
        album.root = dir.to_string_lossy().to_string();
        album.cover = Some(Cover {
            title: "Corse".into(),
            photo: Some(crate::model::Slot::new("a.jpg".into(), [0.5, 0.5])),
            back_text: "Pour Marie.".into(),
            back_photo: Some(photo_q(P::Bas, T::Petite)),
            back_police: Some(crate::model::Police {
                fichier: "objet-Essai.ttf".into(),
                postscript: "Essai".into(),
                nom: "Essai".into(),
            }),
            back_taille_pt: Some(14.0),
            ..Default::default()
        });
        fs::write(dir.join("album.json"), serde_json::to_string(&album).unwrap()).unwrap();
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let out = dir.join("album-cover.pdf");
        render_cover_pdf_at(&dir, cp, &out, instant()).expect("rendu");
        let flux = flux_de(&fs::read(&out).unwrap());
        let g = geometry(&album, cp);
        let c = cadre_quatrieme(&g.back, album.cover.as_ref().unwrap().back_photo.as_ref().unwrap());
        let pt = |mm: f64| format!("{:.2}", mm * 72.0 / 25.4);
        let attendu = format!("q {} {} {} {} re W n", pt(c.x), pt(c.y), pt(c.w), pt(c.h));
        assert!(flux.contains(&attendu), "cadre {attendu} absent : {flux}");
        assert!(flux.contains("/Im1 Do") && flux.contains("/Im0 Do"), "{flux}");
        let quatrieme = flux.lines().find(|l| l.contains("/F2 ")).expect("texte en face 2");
        assert!(quatrieme.contains("/F2 14"), "à sa taille : {quatrieme}");

        let mut w = PdfWriter::new(&album, &dir);
        add_cover_page(&mut w, &album, PrinterProfile::par_id("prodigi").unwrap(), Face::Quatrieme)
            .unwrap();
        let feuillet = dir.join("feuillet.pdf");
        w.save(&feuillet).unwrap();
        let flux = flux_de(&fs::read(&feuillet).unwrap());
        assert!(flux.contains("/Im1 Do") && flux.contains("/F2 "), "{flux}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// La taille : celle du format sans champ, la sienne bornée sinon.
    #[test]
    fn la_taille_de_la_quatrieme() {
        let mut c = Cover::default();
        assert!((taille_quatrieme(&c, 210.0) - BACK_TEXT_PT_AT_210).abs() < 1e-12);
        assert!((taille_quatrieme(&c, 300.0) - BACK_TEXT_PT_AT_210 * 300.0 / 210.0).abs() < 1e-12);
        c.back_taille_pt = Some(14.0);
        assert_eq!(taille_quatrieme(&c, 210.0), 14.0);
        c.back_taille_pt = Some(0.0);
        assert_eq!(taille_quatrieme(&c, 210.0), BACK_TAILLE_MIN_PT);
        c.back_taille_pt = Some(400.0);
        assert_eq!(taille_quatrieme(&c, 210.0), BACK_TAILLE_MAX_PT);
        c.back_taille_pt = Some(f64::NAN);
        assert!((taille_quatrieme(&c, 210.0) - BACK_TEXT_PT_AT_210).abs() < 1e-12);
    }

    /// The quatrième wraps on the real widths of the face, inside the panel.
    #[test]
    fn the_back_text_wraps_to_the_panel() {
        let long = "Trois semaines de septembre sur la côte est, entre les calanques et \
                    les villages de l'intérieur, avec un appareil et pas de programme.";
        let lines = wrap(&pdf::Ecrivain::incorporee(), long, 100.0, 10.5);
        assert!(lines.len() > 1, "rien n'a été coupé : {lines:?}");
        for l in &lines {
            assert!(font::text_width_mm(l, 10.5) <= 100.0, "ligne trop large : {l}");
        }
        // Words survive whole.
        assert_eq!(lines.join(" ").split_whitespace().count(), long.split_whitespace().count());
    }

    /// Blank lines in the quatrième are kept: a dedication has paragraphs.
    #[test]
    fn the_back_text_keeps_its_paragraphs() {
        let lines =
            wrap(&pdf::Ecrivain::incorporee(), "Pour Marie.\n\nEt pour la suite.", 100.0, 10.5);
        assert_eq!(lines, vec!["Pour Marie.", "", "Et pour la suite."]);
    }

    /// The sheet the printer receives is measured in the file it receives,
    /// not in the arithmetic above. Renders a real cover and reads its boxes
    /// back out: the definition of done for this part of the session.
    #[test]
    fn the_rendered_sheet_measures_what_the_geometry_promised() {
        let dir = std::env::temp_dir().join(format!("colophon-cover-{}", std::process::id()));
        let photos = dir.join("photos");
        fs::create_dir_all(&photos).unwrap();
        let img = image::RgbImage::from_pixel(2400, 1600, image::Rgb([70, 110, 160]));
        image::DynamicImage::ImageRgb8(img)
            .save_with_format(photos.join("a.jpg"), image::ImageFormat::Jpeg)
            .unwrap();

        let mut album = album_de(48);
        album.root = photos.to_string_lossy().to_string();
        album.cover = Some(Cover {
            title: "Corse".into(),
            subtitle: "septembre 2013".into(),
            photo: Some(crate::model::Slot::new("a.jpg".into(), [0.5, 0.42])),
            back_text: "Trois semaines sur la côte est.".into(),
            fond: None,
            ..Default::default()
        });
        fs::write(dir.join("album.json"), serde_json::to_string(&album).unwrap()).unwrap();

        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let out = dir.join("album-cover.pdf");
        render_cover_pdf(&dir, cp, &out).expect("rendu de la couverture");

        let g = geometry(&album, cp);
        let doc = lopdf::Document::load(&out).expect("relecture");
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 1, "la couverture est une seule feuille");
        let page = doc.get_object(*pages.values().next().unwrap()).unwrap().as_dict().unwrap();
        let mm = |k: &[u8]| -> Vec<f64> {
            page.get(k)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|o| f64::from(o.as_float().unwrap()) * 25.4 / 72.0)
                .collect()
        };
        let media = mm(b"MediaBox");
        assert!((media[2] - g.media_w).abs() < 0.01, "{media:?} contre {}", g.media_w);
        assert!((media[3] - g.media_h).abs() < 0.01, "{media:?} contre {}", g.media_h);
        // Two pages, a spine, two grooves and the case on each outer edge:
        // the number a supplier checks against their own template.
        let marge = cp.bleed_mm.exterieur + cp.rempli_mm + cp.debord_mm;
        assert!(
            (media[2]
                - (210.0 * 2.0
                    + cp.dos_mm(96, GRAMMAGE_DEFAUT).unwrap()
                    + cp.mors_mm * 2.0
                    + marge * 2.0))
                .abs()
                < 0.01,
            "{media:?}"
        );
        let trim = mm(b"TrimBox");
        assert!((trim[0] - 3.0).abs() < 0.01, "{trim:?}");
        assert!((media[2] - trim[2] - 3.0).abs() < 0.01, "{trim:?}");

        let _ = fs::remove_dir_all(&dir);
    }

    /// L'empreinte de [`feuille_figee`] sans fond, ses dates neutralisées
    /// ([`neutre`]). Relevée en deux temps, parce que les dates suivent le
    /// fuseau de la machine : l'empreinte brute du fichier a d'abord été
    /// relevée sur `main` (481fd68) en UTC+2 et retrouvée à l'identique par
    /// le code de A4, puis ce sont ces mêmes octets, neutralisés, qui ont
    /// donné le chiffre ci-dessous, valable sous tous les fuseaux.
    ///
    /// Reprise au renommage (R-s1), où `/Producer` et `x:xmptk` disent
    /// Booklibre : avec les deux revenus à « Colophon », le code de R-s1
    /// rendait encore l'ancienne empreinte, `[193, 145, 59, …]` — le nom est
    /// donc le seul octet qui ait bougé.
    const EMPREINTE_SANS_FOND: [u8; 16] =
        [145, 244, 10, 161, 159, 104, 200, 192, 242, 183, 82, 253, 173, 16, 196, 243];

    /// L'instant figé de [`feuille_figee`].
    fn instant() -> chrono::DateTime<chrono::Local> {
        use chrono::TimeZone;
        chrono::Local.with_ymd_and_hms(2026, 10, 18, 11, 0, 0).unwrap()
    }

    /// Les octets d'une feuille dont on a masqué les trois écritures de
    /// l'instant — `/CreationDate` et `/ModDate`, les dates du paquet XMP,
    /// l'`/ID` qui en dérive. Elles s'écrivent en heure locale, donc le même
    /// instant donne d'autres chiffres en UTC sur la CI qu'en UTC+2 sur le
    /// Mac ; leur longueur, elle, ne bouge pas, ni rien d'autre du fichier.
    /// Chacune doit être trouvée : un masque qui ne masque rien laisserait
    /// le test redevenir dépendant du fuseau sans le dire.
    fn neutre(octets: &[u8]) -> Vec<u8> {
        let t = instant();
        let id: String = crate::pdfx::fingerprint("Corse", t)
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect();
        let mut out = octets.to_vec();
        for (motif, attendu) in [
            (crate::pdfx::pdf_date(t), 2),
            (crate::pdfx::xmp_date(t), 3),
            (id.clone(), 2),
            (id.to_lowercase(), 0),
        ] {
            let m = motif.as_bytes();
            let mut n = 0;
            let mut i = 0;
            while i + m.len() <= out.len() {
                if &out[i..i + m.len()] == m {
                    out[i..i + m.len()].fill(b'0');
                    n += 1;
                    i += m.len();
                } else {
                    i += 1;
                }
            }
            if attendu > 0 {
                assert_eq!(n, attendu, "{motif} trouvé {n} fois");
            }
        }
        out
    }

    /// Un album de test sans photo, à couverture complète, posé dans un
    /// dossier à lui, et sa feuille rendue à un instant figé. Sans photo, le
    /// fichier ne dépend que de la face du projet et de l'arithmétique : il
    /// rend les mêmes octets sur les trois OS du gate.
    fn feuille_figee(nom: &str, fond: Option<&str>) -> Vec<u8> {
        let dir = std::env::temp_dir()
            .join(format!("colophon-fond-{nom}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut album = album_de(48);
        album.root = dir.to_string_lossy().to_string();
        album.cover = Some(Cover {
            title: "Corse".into(),
            subtitle: "septembre 2013".into(),
            photo: None,
            back_text: "Trois semaines sur la côte est.".into(),
            fond: fond.map(str::to_string),
            ..Default::default()
        });
        fs::write(dir.join("album.json"), serde_json::to_string(&album).unwrap()).unwrap();
        let out = dir.join("album-cover.pdf");
        render_cover_pdf_at(&dir, PrinterProfile::par_id("cloudprinter").unwrap(), &out, instant())
            .expect("rendu");
        let octets = fs::read(&out).unwrap();
        let _ = fs::remove_dir_all(&dir);
        octets
    }

    /// Sans fond, la feuille est celle d'avant le fond, à l'octet. Le chiffre
    /// a été relevé sur `main` (481fd68), avant la première ligne de A4 : un
    /// album qui n'a pas choisi de couleur ne doit rien voir changer.
    #[test]
    fn sans_fond_la_feuille_est_celle_d_avant_a_l_octet() {
        let octets = feuille_figee("aucun", None);
        assert_eq!(
            crate::export::empreinte(&neutre(&octets)),
            EMPREINTE_SANS_FOND,
            "{} octets",
            octets.len()
        );
    }

    /// Le flux de contenu de la première page d'un PDF, décompressé.
    fn flux_de(octets: &[u8]) -> String {
        let doc = lopdf::Document::load_mem(octets).expect("relecture");
        let page = *doc.get_pages().values().next().unwrap();
        String::from_utf8_lossy(&doc.get_page_content(page).unwrap()).into_owned()
    }

    /// Les cotes de la MediaBox, en points.
    fn media_pt(octets: &[u8]) -> [f64; 2] {
        let doc = lopdf::Document::load_mem(octets).expect("relecture");
        let page = *doc.get_pages().values().next().unwrap();
        let d = doc.get_object(page).unwrap().as_dict().unwrap();
        let m = d.get(b"MediaBox").unwrap().as_array().unwrap();
        [f64::from(m[2].as_float().unwrap()), f64::from(m[3].as_float().unwrap())]
    }

    /// Le fond est un aplat de la feuille entière, fond perdu compris, posé
    /// avant tout le reste : la photo et le texte passent dessus. La règle
    /// est mesurée dans le fichier, pas dans l'arithmétique.
    #[test]
    fn un_fond_couvre_toute_la_feuille_fond_perdu_compris() {
        let octets = feuille_figee("noir", Some("#1a1a1a"));
        let flux = flux_de(&octets);
        let premiere = flux.lines().next().unwrap_or_default();
        let ops: Vec<&str> = premiere.split_whitespace().collect();
        let c = 26.0 / 255.0;
        assert_eq!(&ops[..2], ["q", &c.to_string()], "{premiere}");
        assert_eq!(ops[4], "rg", "{premiere}");
        assert_eq!(&ops[5..7], ["0", "0"], "l'aplat part du coin de la feuille : {premiere}");
        assert_eq!(&ops[9..], ["re", "f", "Q"], "{premiere}");
        let [w, h] = media_pt(&octets);
        let (aw, ah): (f64, f64) = (ops[7].parse().unwrap(), ops[8].parse().unwrap());
        assert!(aw >= w && aw - w < 0.02, "{aw} contre {w}");
        assert!(ah >= h && ah - h < 0.02, "{ah} contre {h}");
        assert_ne!(crate::export::empreinte(&neutre(&octets)), EMPREINTE_SANS_FOND);
    }

    /// Une valeur hors `#rrggbb` est ignorée : la feuille est celle d'un album
    /// sans fond, à l'octet, et l'album s'exporte.
    #[test]
    fn un_fond_illisible_rend_la_feuille_sans_fond() {
        for gate in ["terracotta", "#abc", "#12345g"] {
            let octets = feuille_figee("gate", Some(gate));
            assert_eq!(crate::export::empreinte(&neutre(&octets)), EMPREINTE_SANS_FOND, "{gate}");
        }
    }

    /// Le texte sans photo dessous suit le fond : sur du noir il passe au
    /// blanc papier, sur du crème il garde l'encre. Dos et quatrième compris.
    #[test]
    fn le_texte_prend_l_encre_lisible_sur_le_fond() {
        let encre = |fond: &str| {
            let flux = flux_de(&feuille_figee(&format!("encre-{}", &fond[1..]), Some(fond)));
            let papier = format!("{} {} {} rg", PAPER[0], PAPER[1], PAPER[2]);
            let ink = format!("{} {} {} rg", pdf::TEXT_INK[0], pdf::TEXT_INK[1], pdf::TEXT_INK[2]);
            // Une ligne de texte par couleur rencontrée : titre, sous-titre,
            // dos et quatrième, aucune photo dessous.
            let textes: Vec<&str> = flux.lines().filter(|l| l.contains("Tj")).collect();
            assert_eq!(textes.len(), 4, "{flux}");
            (
                textes.iter().filter(|l| l.contains(&papier)).count(),
                textes.iter().filter(|l| l.contains(&ink)).count(),
            )
        };
        assert_eq!(encre("#1a1a1a"), (4, 0), "sur du noir, tout passe au papier");
        assert_eq!(encre("#f3ead8"), (0, 4), "sur du crème, l'encre reste");
    }

    /// Le feuillet volant d'un fournisseur qui relie un seul fichier porte le
    /// même aplat, sur toute la feuille fond perdu compris, sous la photo de
    /// la première comme sous la quatrième.
    #[test]
    fn le_feuillet_porte_le_fond_aussi() {
        let dir = std::env::temp_dir()
            .join(format!("colophon-feuillet-fond-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let img = image::RgbImage::from_pixel(2400, 1600, image::Rgb([70, 110, 160]));
        image::DynamicImage::ImageRgb8(img)
            .save_with_format(dir.join("a.jpg"), image::ImageFormat::Jpeg)
            .unwrap();
        let mut album = album_de(12);
        album.root = dir.to_string_lossy().to_string();
        album.cover = Some(Cover {
            title: "Corse".into(),
            subtitle: String::new(),
            photo: Some(crate::model::Slot::new("a.jpg".into(), [0.5, 0.42])),
            back_text: "Pour Marie.".into(),
            fond: Some("#f3ead8".into()),
            ..Default::default()
        });
        let gen = PrinterProfile::par_id("generique").unwrap();
        for (face, nom) in [(Face::Premiere, "premiere"), (Face::Quatrieme, "quatrieme")] {
            let mut w = PdfWriter::new(&album, &dir);
            add_cover_page(&mut w, &album, gen, face).unwrap();
            let path = dir.join(format!("{nom}.pdf"));
            w.save(&path).unwrap();
            let octets = fs::read(&path).unwrap();
            let flux = flux_de(&octets);
            let premiere = flux.lines().next().unwrap_or_default();
            let ops: Vec<&str> = premiere.split_whitespace().collect();
            assert_eq!(ops.len(), 12, "{nom} : {flux}");
            assert_eq!(ops[0], "q", "{nom} : {premiere}");
            assert_eq!(ops[4], "rg", "{nom} : {premiere}");
            assert_eq!(&ops[5..7], ["0", "0"], "{nom} : {premiere}");
            assert_eq!(&ops[9..], ["re", "f", "Q"], "{nom} : {premiere}");
            let [w, h] = media_pt(&octets);
            let (aw, ah): (f64, f64) = (ops[7].parse().unwrap(), ops[8].parse().unwrap());
            assert!(aw >= w && aw - w < 0.02, "{nom} : {aw} contre {w}");
            assert!(ah >= h && ah - h < 0.02, "{nom} : {ah} contre {h}");
            assert!(w > 210.0 * 72.0 / 25.4, "{nom} : fond perdu compris");
            if face == Face::Premiere {
                assert!(flux.contains(" Do"), "la photo passe sur l'aplat : {flux}");
            }
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// A leaf is one panel, trimmed on four sides, with no spine and no fold.
    /// The face it does not carry gets no surface, so nothing of the back
    /// cover can land on the front.
    #[test]
    fn a_cover_leaf_is_one_panel_and_no_spine() {
        let a = album_de(24);
        let pr = PrinterProfile::par_id("prodigi").unwrap();

        let devant = page_geometry(&a, pr, Face::Premiere);
        assert!(devant.spine.is_none(), "une feuille volante n'a pas de dos");
        // Prodigi builds the bleed itself: the leaf is the finished page.
        assert!((devant.media_w - 210.0).abs() < 1e-9, "{}", devant.media_w);
        assert!((devant.media_h - 210.0).abs() < 1e-9, "{}", devant.media_h);
        assert!((devant.front.w - 210.0).abs() < 1e-9);
        assert_eq!(devant.back.w, 0.0, "la quatrième n'est pas sur cette feuille");

        let derriere = page_geometry(&a, pr, Face::Quatrieme);
        assert!((derriere.back.w - 210.0).abs() < 1e-9);
        assert_eq!(derriere.front.w, 0.0, "la première n'est pas sur cette feuille");

        // A profile that asks for bleed gets it on all four edges, because a
        // leaf is cut on all four: this is not half a flat sheet.
        let gen = PrinterProfile::par_id("generique").unwrap();
        let g = page_geometry(&a, gen, Face::Premiere);
        assert!((g.media_w - (210.0 + 6.0)).abs() < 1e-9, "{}", g.media_w);
        assert!((g.media_h - (210.0 + 6.0)).abs() < 1e-9, "{}", g.media_h);
    }

    /// The one that matters: a supplier who binds a single file must find the
    /// front cover on page one and the back cover on the last page. Sending
    /// the interior alone binds the whole book one page out of place, and the
    /// file is valid, so nothing but this test catches it.
    #[test]
    fn a_single_file_supplier_gets_the_cover_inside_the_interior() {
        // Its own name: `colophon-print-` is already taken by a print.rs test
        // that removes the folder while this one is still writing into it.
        let dir = std::env::temp_dir().join(format!("colophon-couv-int-{}", std::process::id()));
        let photos = dir.join("photos");
        fs::create_dir_all(&photos).unwrap();
        let img = image::RgbImage::from_pixel(2400, 1600, image::Rgb([70, 110, 160]));
        image::DynamicImage::ImageRgb8(img)
            .save_with_format(photos.join("a.jpg"), image::ImageFormat::Jpeg)
            .unwrap();

        let mut album = album_de(4);
        album.root = photos.to_string_lossy().to_string();
        for spread in &mut album.spreads {
            spread.slots = vec![crate::model::Slot::new("a.jpg".into(), [0.5, 0.5])];
        }
        album.cover = Some(Cover {
            title: "Corse".into(),
            subtitle: "septembre 2013".into(),
            photo: Some(crate::model::Slot::new("a.jpg".into(), [0.5, 0.42])),
            back_text: "Trois semaines sur la côte est.".into(),
            fond: None,
            ..Default::default()
        });
        fs::write(dir.join("album.json"), serde_json::to_string(&album).unwrap()).unwrap();

        let largeurs = |profil: &'static PrinterProfile, nom: &str| -> Vec<f64> {
            let out = dir.join(format!("album-print-{nom}.pdf"));
            print::render_print_pdf(&dir, profil, &out, &|_| {}, &|| false).expect("rendu");
            let doc = lopdf::Document::load(&out).expect("relecture");
            let pages = doc.get_pages();
            let mut ids: Vec<_> = pages.iter().collect();
            ids.sort_by_key(|(n, _)| **n);
            ids.iter()
                .map(|(_, id)| {
                    let page = doc.get_object(**id).unwrap().as_dict().unwrap();
                    let media = page.get(b"MediaBox").unwrap().as_array().unwrap();
                    f64::from(media[2].as_float().unwrap()) * 25.4 / 72.0
                })
                .collect()
        };

        // Prodigi: the book block between two cover leaves. It binds page by
        // page, so four spreads make eight interior pages — the first spread's
        // recto, then both pages of the other three, then the blank verso —
        // and the file is ten pages long.
        let pr = largeurs(PrinterProfile::par_id("prodigi").unwrap(), "prodigi");
        assert_eq!(pr.len(), 10, "8 pages de livre plus les deux couvertures : {pr:?}");
        assert!((pr[0] - 210.0).abs() < 0.01, "première de couverture : {pr:?}");
        assert!((pr[9] - 210.0).abs() < 0.01, "quatrième de couverture : {pr:?}");
        // A leaf is the trimmed page, an interior page is that page plus the
        // bleed the album carries: they are close in width and the file has
        // to keep them apart, which is what the count above says.
        for (i, w) in pr[1..9].iter().enumerate() {
            assert!((w - 213.0).abs() < 0.01, "page {} : {pr:?}", i + 1);
        }

        // Cloudprinter binds two files, so its interior stays an interior.
        let cp = largeurs(PrinterProfile::par_id("cloudprinter").unwrap(), "cloudprinter");
        assert_eq!(cp.len(), 8, "l'intérieur seul, la couverture est son fichier : {cp:?}");
        // Their own inside-page template: 216 × 216, bleed on all four edges.
        assert!(cp.iter().all(|w| (w - 216.0).abs() < 0.01), "{cp:?}");

        // Lulu imposes our spreads itself, so its interior is still an
        // interior of spreads — the shape the emitter has always written, and
        // the one the whole file must keep producing unchanged.
        let lu = largeurs(PrinterProfile::par_id("lulu").unwrap(), "lulu");
        assert_eq!(lu.len(), 4, "quatre planches doubles : {lu:?}");
        assert!(lu.iter().all(|w| (w - 426.0).abs() < 0.01), "{lu:?}");

        let _ = fs::remove_dir_all(&dir);
    }

    /// Le banc de l'œil : la feuille du boîtier, ses panneaux et ses cotes
    /// tracés par-dessus, à côté du gabarit de Cloudprinter rasterisé au même
    /// nombre de pixels par millimètre.
    ///
    /// C'est le seul contrôle qui attrape une cote posée du mauvais côté.
    /// Aucun test ne le fait : ils vérifient tous que la somme tombe juste, et
    /// une somme tombe juste aussi quand le rempli est à gauche et le débord à
    /// droite. Deux images superposables, elles, ne mentent pas.
    ///
    /// La seconde image est la vraie épreuve : nos lignes sont dessinées sur
    /// **leur** gabarit. Le dos y est forcé aux 14 mm pour lesquels leur
    /// fichier est dessiné — il annonce lui-même « 100 pages of 200 gsm
    /// machine coated gloss », dont la main 0,80 n'est plus celle du papier
    /// commandé — de sorte que tout le reste doit tomber au pixel : la marge
    /// de 24, les panneaux de 210, les deux mors de 5.
    ///
    /// ```text
    /// COLOPHON_ALBUM=.albums/corse-2013 \
    /// COLOPHON_GABARIT="$HOME/Downloads/photobook_cw_s210_s_fc_product (1)/photobook_cw_s210_s_fc_cover.pdf" \
    ///   cargo test -p colophon-core --release banc_la_feuille_du_boitier -- --ignored --nocapture
    /// ```
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn banc_la_feuille_du_boitier() {
        let dir = PathBuf::from(
            std::env::var("COLOPHON_ALBUM").expect("COLOPHON_ALBUM = dossier d'un album composé"),
        );
        let cp = PrinterProfile::par_id("cloudprinter").unwrap();
        let album: Album =
            serde_json::from_str(&fs::read_to_string(dir.join("album.json")).unwrap()).unwrap();
        let g = geometry(&album, cp);
        let out = dir.join("banc-boitier");
        fs::create_dir_all(&out).unwrap();

        // 1. Notre feuille, telle que la presse la reçoit, annotée.
        let pdf = out.join("feuille.pdf");
        render_cover_pdf(&dir, cp, &pdf).expect("rendu de la couverture");
        let mut feuille = rasterise(&pdf, &out.join("feuille-brute.png"));
        let ppmm = f64::from(feuille.width()) / g.media_w;
        guides(&mut feuille, &g, cp, ppmm, g.spine_mm());
        feuille.save(out.join("feuille.png")).unwrap();

        // 2. Leur gabarit, sous nos lignes. Le dos forcé au sien.
        let gabarit = std::env::var("COLOPHON_GABARIT").ok().map(PathBuf::from);
        if let Some(src) = gabarit.filter(|p| p.is_file()) {
            let mut leur = rasterise(&src, &out.join("gabarit-brut.png"));
            const DOS_DU_GABARIT: f64 = 14.0;
            let large = album.trim_mm.w * 2.0
                + DOS_DU_GABARIT
                + cp.mors_mm * 2.0
                + (cp.bleed_mm.exterieur + cp.rempli_mm + cp.debord_mm) * 2.0;
            let ppmm = f64::from(leur.width()) / large;
            let mut gg = g.clone();
            gg.media_w = large;
            gg.spine = gg.spine.map(|r| Rect { w: DOS_DU_GABARIT, ..r });
            gg.front.x = gg.back.x + gg.back.w + cp.mors_mm + DOS_DU_GABARIT + cp.mors_mm;
            guides(&mut leur, &gg, cp, ppmm, DOS_DU_GABARIT);
            leur.save(out.join("gabarit.png")).unwrap();
            println!("gabarit  : {}", out.join("gabarit.png").display());
        } else {
            println!("gabarit  : absent (COLOPHON_GABARIT), la superposition n'est pas faite");
        }

        println!("feuille  : {}", out.join("feuille.png").display());
        println!(
            "cotes    : feuille {:.2} × {:.2} mm, dos {:.2}, rempli {}, débord {}, mors {}",
            g.media_w, g.media_h, g.spine_mm(), cp.rempli_mm, cp.debord_mm, cp.mors_mm
        );
        println!("légende  : rouge = coupe de la feuille (fond perdu)");
        println!("           vert  = panneaux finis (dos, quatrième, première)");
        println!("           bleu  = mors, la gorge où la couverture plie");
        println!("           jaune = pli du rempli, ce qui se replie derrière le carton");
    }

    /// Un PDF en pixels, par `sips`, la même porte que `scripts/pdf-png.py`.
    #[cfg(target_os = "macos")]
    fn rasterise(pdf: &Path, png: &Path) -> image::RgbImage {
        let ok = std::process::Command::new("sips")
            .args(["-s", "format", "png"])
            .arg(pdf)
            .arg("--out")
            .arg(png)
            .output()
            .expect("sips");
        assert!(ok.status.success(), "sips a refusé {}", pdf.display());
        image::open(png).expect("relecture du PNG").to_rgb8()
    }

    /// Les lignes de notre géométrie, posées sur une image déjà rasterisée.
    #[cfg(target_os = "macos")]
    fn guides(
        im: &mut image::RgbImage,
        g: &CoverGeometry,
        profil: &PrinterProfile,
        ppmm: f64,
        dos: f64,
    ) {
        const ROUGE: [u8; 3] = [220, 40, 40];
        const VERT: [u8; 3] = [40, 190, 80];
        const BLEU: [u8; 3] = [60, 120, 240];
        const JAUNE: [u8; 3] = [240, 200, 40];
        let (w, h) = (im.width(), im.height());
        fn v(im: &mut image::RgbImage, x_mm: f64, ppmm: f64, c: [u8; 3]) {
            let x = (x_mm * ppmm).round() as i64;
            if x < 0 || x >= i64::from(im.width()) {
                return;
            }
            for y in 0..im.height() {
                im.put_pixel(x as u32, y, image::Rgb(c));
            }
        }
        // L'image est en coordonnées écran, la géométrie en coordonnées PDF.
        fn hh(im: &mut image::RgbImage, y_mm: f64, media_h: f64, ppmm: f64, c: [u8; 3]) {
            let y = ((media_h - y_mm) * ppmm).round() as i64;
            if y < 0 || y >= i64::from(im.height()) {
                return;
            }
            for x in 0..im.width() {
                im.put_pixel(x, y as u32, image::Rgb(c));
            }
        }
        let _ = (w, h);
        // La coupe : ce que le massicot laisse de la feuille.
        for x in [g.bleed_ext, g.media_w - g.bleed_ext] {
            v(im, x, ppmm, ROUGE);
        }
        for y in [g.bleed_bas, g.media_h - g.bleed_haut] {
            hh(im, y, g.media_h, ppmm, ROUGE);
        }
        // Le pli du rempli : le carton commence là.
        let pli = g.bleed_ext + profil.rempli_mm;
        for x in [pli, g.media_w - pli] {
            v(im, x, ppmm, JAUNE);
        }
        for y in [g.bleed_bas + profil.rempli_mm, g.media_h - g.bleed_haut - profil.rempli_mm] {
            hh(im, y, g.media_h, ppmm, JAUNE);
        }
        // Les panneaux finis, et les deux mors entre eux.
        for x in [g.back.x, g.back.x + g.back.w, g.front.x, g.front.x + g.front.w] {
            v(im, x, ppmm, VERT);
        }
        for y in [g.back.y, g.back.y + g.back.h] {
            hh(im, y, g.media_h, ppmm, VERT);
        }
        if dos > 0.0 {
            let dos_x = g.back.x + g.back.w + profil.mors_mm;
            for x in [dos_x, dos_x + dos] {
                v(im, x, ppmm, BLEU);
            }
        }
    }
}
