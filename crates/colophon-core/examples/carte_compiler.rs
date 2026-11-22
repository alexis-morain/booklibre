//! Compile Natural Earth en `assets/carte/carte.bin` (P-s4).
//!
//! `cargo run --release -p colophon-core --example carte_compiler -- <natural-earth> <sortie.bin> [tol_monde tol_detail]`
//!
//! `<natural-earth>` contient un dossier par couche, comme les zips dézippés
//! (`ne_10m_coastline/ne_10m_coastline.shp`…). Voir `scripts/carte.sh`.

use colophon_core::carte::compilation::{compiler, Reglages, Sources};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err(
            "usage : carte_compiler <dossier natural-earth> <sortie.bin> [tol_monde tol_detail]"
                .into(),
        );
    }
    let dossier = Path::new(&args[1]);
    let lire = |nom: &str| {
        std::fs::read(dossier.join(nom).join(format!("{nom}.shp")))
            .map_err(|e| format!("{nom} : {e}"))
    };
    let (land50, pays50, pays10, cote10, lacs10, rivieres10) = (
        lire("ne_50m_land")?,
        lire("ne_50m_admin_0_countries")?,
        lire("ne_10m_admin_0_countries")?,
        lire("ne_10m_coastline")?,
        lire("ne_10m_lakes")?,
        lire("ne_10m_rivers_lake_centerlines")?,
    );
    let mut reglages = Reglages::default();
    if let (Some(m), Some(d)) = (args.get(3), args.get(4)) {
        reglages = Reglages {
            tolerance_monde: m.parse()?,
            tolerance_detail: d.parse()?,
        };
    }
    let octets = compiler(
        &Sources {
            land50: &land50,
            countries50: &pays50,
            countries10: &pays10,
            coastline10: &cote10,
            lakes10: &lacs10,
            rivers10: &rivieres10,
        },
        &reglages,
    )?;
    std::fs::write(&args[2], &octets)?;
    let actif = colophon_core::carte::actif::Actif::lire(&octets)?;
    for c in colophon_core::carte::actif::Couche::TOUTES {
        let points: usize = actif
            .parties(c)
            .map(|p| p.and_then(|p| p.points()).map_or(0, |p| p.len()))
            .sum();
        println!(
            "{c:?} : {} parties, {points} points, {} octets",
            actif.nombre_de_parties(c),
            actif.octets(c)
        );
    }
    println!(
        "{} octets, empreinte {}",
        octets.len(),
        colophon_core::carte::compilation::empreinte(&octets)
    );
    Ok(())
}
