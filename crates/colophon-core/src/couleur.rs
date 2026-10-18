//! La couleur d'un objet libre ou du fond de la couverture, telle
//! qu'`album.json` l'écrit : `#rrggbb`, et le contraste d'un texte posé dessus.
//!
//! Une chaîne et pas trois nombres, parce que le fichier se répare à la main
//! et qu'une couleur s'y lit comme on la tape dans n'importe quel outil. La
//! lecture est ici, une fois : l'émetteur, le linter et la scène passent tous
//! par [`parse`], et rien d'autre ne découpe la chaîne.

/// Les trois composantes d'une couleur `#rrggbb`, entre 0 et 1, ou `None`.
///
/// `None` sur tout ce qui n'est pas exactement un dièse et six chiffres
/// hexadécimaux : `#abc`, `rouge`, `#12345g`, une chaîne vide. Jamais une
/// panique, parce que la chaîne vient d'un fichier qu'une main a pu toucher.
/// L'appelant retombe alors sur la couleur par défaut de l'objet, et l'album
/// s'exporte.
pub fn parse(s: &str) -> Option<[f64; 3]> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let composante = |i: usize| -> Option<f64> {
        u8::from_str_radix(hex.get(i..i + 2)?, 16).ok().map(|v| f64::from(v) / 255.0)
    };
    Some([composante(0)?, composante(2)?, composante(4)?])
}

/// La luminance relative de WCAG 2, de 0 (noir) à 1 (blanc), d'une couleur
/// sRGB aux composantes entre 0 et 1.
pub fn luminance(rgb: [f64; 3]) -> f64 {
    let lin = |c: f64| {
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2])
}

/// Le rapport de contraste de WCAG 2 entre deux couleurs, de 1 à 21.
pub fn contraste(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Le seuil de WCAG pour un texte courant.
pub const CONTRASTE_MIN: f64 = 4.5;

/// L'encre d'un texte posé sur `fond` : `encre` tant qu'elle y tient
/// [`CONTRASTE_MIN`], sinon celle des deux qui contraste le plus. Le texte de
/// couverture ne connaît que ces deux encres, l'encre du livre et le blanc
/// papier, et ne passe de l'une à l'autre que si la première se lit mal.
///
/// Les deux ne tiennent jamais le seuil ensemble — l'encre du livre est trop
/// sombre et le papier trop clair —, si bien que la règle revient à prendre la
/// plus contrastée ; le seuil dit pourquoi l'encre reste sur un fond clair.
pub fn encre_lisible(fond: [f64; 3], encre: [f64; 3], papier: [f64; 3]) -> [f64; 3] {
    if contraste(fond, encre) >= CONTRASTE_MIN || contraste(fond, encre) >= contraste(fond, papier) {
        encre
    } else {
        papier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_hexadecimaux_donnent_trois_composantes() {
        assert_eq!(parse("#000000"), Some([0.0, 0.0, 0.0]));
        assert_eq!(parse("#ffffff"), Some([1.0, 1.0, 1.0]));
        assert_eq!(parse("#FF0000"), Some([1.0, 0.0, 0.0]));
        let [r, g, b] = parse("#c0604a").unwrap();
        assert_eq!((r * 255.0).round(), 192.0);
        assert_eq!((g * 255.0).round(), 96.0);
        assert_eq!((b * 255.0).round(), 74.0);
    }

    /// Les bornes de WCAG : le noir sur le blanc vaut 21, une couleur contre
    /// elle-même vaut 1, et la luminance va de 0 à 1.
    #[test]
    fn le_contraste_wcag_aux_bornes() {
        let (noir, blanc) = ([0.0; 3], [1.0; 3]);
        assert_eq!(luminance(noir), 0.0);
        assert!((luminance(blanc) - 1.0).abs() < 1e-12);
        assert!((contraste(noir, blanc) - 21.0).abs() < 1e-9);
        assert!((contraste(blanc, noir) - 21.0).abs() < 1e-9, "symétrique");
        let terracotta = parse("#b04a1f").unwrap();
        assert!((contraste(terracotta, terracotta) - 1.0).abs() < 1e-12);
        // Le seuil de linéarisation : 0,04045 tombe sur la branche droite.
        assert!((luminance([0.04045; 3]) - 0.04045 / 12.92).abs() < 1e-12);
    }

    /// L'encre du texte de couverture, entre l'encre du livre et le blanc
    /// papier : l'encre tant qu'elle tient 4,5:1, sinon la plus contrastée.
    #[test]
    fn l_encre_lisible_aux_bornes() {
        let encre = [0.2, 0.19, 0.16];
        let papier = [0.99, 0.97, 0.94];
        let sur = |hex: &str| encre_lisible(parse(hex).unwrap(), encre, papier);
        assert_eq!(sur("#000000"), papier, "noir");
        assert_eq!(sur("#1a1a1a"), papier, "le noir de la palette");
        assert_eq!(sur("#ffffff"), encre, "blanc");
        assert_eq!(sur("#f3ead8"), encre, "crème");
        // Le gris moyen : aucune des deux n'atteint 4,5:1, la plus forte gagne.
        let gris = parse("#777777").unwrap();
        assert!(contraste(gris, encre) < CONTRASTE_MIN && contraste(gris, papier) < CONTRASTE_MIN);
        assert_eq!(sur("#777777"), papier, "gris moyen");
        assert_eq!(sur("#b04a1f"), papier, "terracotta");
        assert!(contraste(parse("#b04a1f").unwrap(), papier) >= CONTRASTE_MIN);
    }

    /// Tout le reste rend `None`, sans paniquer : la chaîne vient d'un
    /// fichier réparable à la main. Le multi-octet est là exprès, un découpage
    /// par indices d'octets y casserait une frontière de caractère.
    #[test]
    fn tout_le_reste_rend_none() {
        for s in ["", "#", "000000", "#abc", "#1234567", "#12345g", "rouge", "#ééé", "# 12345", "#-12345"] {
            assert_eq!(parse(s), None, "{s:?}");
        }
    }
}
