//! Impression (man. p. 338-340 : bouton « Imprimer » de la barre
//! Fichier) : le dessin est posé sur une feuille A4 à l'échelle 1:1
//! (1 mm = 2,834 6 pt), décalé de ses marges gauche et haute — comme
//! à l'imprimante.
//!
//! L'image rasterisée (la même que le PNG) est intégrée en flux
//! FlateDecode (zlib à blocs « stored »), comme dans l'IDAT d'un PNG.
//! Pas de texte vectoriel : la feuille reproduit le rendu écran.
//! Les marges et l'orientation viennent des préférences ; elles sont
//! passées en valeurs brutes (f64/bool) pour découpler l'écriture
//! du fichier de la struct Preferences.
use crate::{
    annot::AnnotDraw,
    grid::Page,
    png,
    render::{rasterize_page, CurvePolys},
    schema::ShObj,
    simu::Simulation,
    stats::{Arbre, BoitesMultiples, Serie1, Serie2, Suite},
    tableau::Tableau,
};
use std::path::Path;

/// pt par mm (72 pt = 1 pouce = 25,4 mm).
const PT_MM: f64 = 72.0 / 25.4;

/// Feuille A4 en mm (portrait : 210 × 297).
fn a4(portrait: bool) -> (f64, f64) {
    if portrait {
        (210.0, 297.0)
    } else {
        (297.0, 210.0)
    }
}

/// Écrit une page complète en PDF 1.4 : objet 1 (catalogue), 2
/// (arbre de pages), 3 (page : MediaBox + contenu 4 + image 5).
#[allow(clippy::too_many_arguments)]
pub fn write_pdf_file(
    page: &Page,
    curves: &[CurvePolys],
    annots: &[AnnotDraw],
    tableaux: &[Tableau],
    stats: &[Serie1],
    boites: &[BoitesMultiples],
    suites: &[Suite],
    serie2: &[Serie2],
    arbres: &[Arbre],
    simus: &[Simulation],
    schema: &[ShObj],
    px_per_mm: f64,
    marge_gauche_mm: f64,
    marge_haute_mm: f64,
    portrait: bool,
    path: &Path,
) -> std::io::Result<()> {
    use std::fmt::Write as _;
    let r = rasterize_page(
        page, curves, annots, tableaux, stats, boites, suites, serie2, arbres, simus, schema,
        px_per_mm,
    );
    let (w, h) = (r.w, r.h);
    // pixels RGBA → RGB (rangées de haut en bas, comme un objet image)
    let mut raw = vec![0u8; w * h * 3];
    for (i, px) in r.px.chunks_exact(4).enumerate() {
        raw[i * 3..i * 3 + 3].copy_from_slice(&px[..3]);
    }
    let img = png::zlib_store(&raw);

    let (sheet_w_mm, sheet_h_mm) = a4(portrait);
    let sheet_h = sheet_h_mm * PT_MM;
    let draw_w = page.width_mm * PT_MM;
    let draw_h = page.height_mm * PT_MM;
    // position depuis le bas de la feuille : marge haute + hauteur du
    // dessin (l'origine PDF est en bas à gauche, l'image se pose avec
    // sa première rangée en haut)
    let ty = sheet_h - marge_haute_mm * PT_MM - draw_h;
    let tx = marge_gauche_mm * PT_MM;
    // l'objet image occupe un carré unitaire : cm le met à l'échelle
    // sur les dimensions du dessin en pt (1 mm = 2,8346 pt)
    let sx = draw_w;
    let sy = draw_h;
    let cnt = format!("q {sx:.6} 0 0 {sy:.6} {tx:.6} {ty:.6} cm /Im0 Do Q\n");
    let mut media = String::new();
    let _ = write!(media, "[0 0 {:.3} {:.3}]", sheet_w_mm * PT_MM, sheet_h);

    let mut f: Vec<u8> = Vec::with_capacity(img.len() + 2048);
    let mut offs = [0usize; 6];
    f.extend_from_slice(b"%PDF-1.4\n");
    offs[1] = f.len();
    f.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offs[2] = f.len();
    f.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offs[3] = f.len();
    f.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox {media} /Resources \
             << /XObject << /Im0 5 0 R >> /ProcSet [/PDF /ImageC] >> /Contents 4 0 R >>\nendobj\n"
        )
        .as_bytes(),
    );
    offs[4] = f.len();
    f.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", cnt.len()).as_bytes());
    f.extend_from_slice(cnt.as_bytes());
    f.extend_from_slice(b"endstream\nendobj\n");
    offs[5] = f.len();
    f.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {l} >>\nstream\n",
            w = w,
            h = h,
            l = img.len()
        )
        .as_bytes(),
    );
    f.extend_from_slice(&img);
    f.extend_from_slice(b"\nendstream\nendobj\n");

    // table de référence : les 6 décalages calculés au fil de l'eau
    let xref = f.len();
    f.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in offs.iter().skip(1) {
        f.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    let taille = 6usize;
    f.extend_from_slice(
        format!("trailer\n<< /Size {taille} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    std::fs::write(path, &f)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::sample_curve;
    use crate::curve::Curve;
    use std::path::PathBuf;

    fn page() -> Page {
        Page::default()
    }

    #[test]
    fn entete_eof_et_xref_exacte() {
        let p = page();
        let tmp = PathBuf::from("/tmp/opencode/sqn-pdf-test.pdf");
        write_pdf_file(
            &p,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            3.0,
            15.0,
            15.0,
            true,
            &tmp,
        )
        .unwrap();
        let b = std::fs::read(&tmp).unwrap();
        // entête et fin
        assert_eq!(&b[..8], b"%PDF-1.4");
        let eof = b"%%EOF\n";
        assert_eq!(&b[b.len() - eof.len()..], eof);
        // MediaBox A4 portrait : 595 × 842 pt (fichier binaire : on
        // cherche la sous-séquence dans les octets)
        let cherche = |pat: &str| b.windows(pat.len()).any(|w| w == pat.as_bytes());
        assert!(cherche("[0 0 595.276 841.890]"), "MediaBox A4 portrait");
        // décalages xref : chaque entrée pointe exactement sur "N 0 obj"
        let sous = |hay: &[u8], needle: &str| -> Option<usize> {
            hay.windows(needle.len())
                .position(|w| w == needle.as_bytes())
        };
        let mut xi = sous(&b, "xref\n0 6\n").unwrap() + "xref\n0 6\n0000000000 65535 f \n".len();
        for k in 1..6 {
            let ligne = &b[xi..xi + 20]; // "0000000000 00000 n \n"
            let o: usize = std::str::from_utf8(&ligne[..10]).unwrap().parse().unwrap();
            let attendu = format!("{k} 0 obj");
            assert_eq!(&b[o..o + attendu.len()], attendu.as_bytes(), "entrée {k}");
            xi += 20;
        }
        let sx = sous(&b, "startxref\n").unwrap() + "startxref\n".len();
        let off: usize = std::str::from_utf8(&b[sx..sx + 6])
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(&b[off..off + 4], b"xref", "startxref pointé");
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn paysage_et_contenu_cm() {
        let p = page();
        let c = Curve::fonction("x^2", [204, 0, 0]);
        let polys = sample_curve(&c, &p, 40, 40).unwrap();
        let tmp = PathBuf::from("/tmp/opencode/sqn-pdf-test2.pdf");
        write_pdf_file(
            &p,
            &[(c, polys)],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            4.0,
            12.5,
            10.0,
            false,
            &tmp,
        )
        .unwrap();
        let b2 = std::fs::read(&tmp).unwrap();
        let txt = String::from_utf8_lossy(&b2[..b2.len() / 2]);
        // A4 paysage : 842 × 595 pt (début ASCII du fichier)
        assert!(txt.contains("[0 0 841.890 595.276]"), "{txt}");
        // stream de contenu : tokens = q sx 0 0 sy tx ty cm /Im0 Do Q
        let s = txt
            .lines()
            .find(|l| l.starts_with("q ") && l.ends_with("cm /Im0 Do Q"))
            .unwrap();
        let toks: Vec<&str> = s.split_whitespace().collect();
        assert_eq!(toks.len(), 11, "q sx 0 0 sy tx ty cm /Im0 Do Q");
        let v: Vec<f64> = toks[1..7].iter().map(|t| t.parse().unwrap()).collect();
        assert_eq!(v[1], 0.0);
        assert_eq!(v[2], 0.0);
        // la feuille contient le dessin (170 × 120 mm) sous 10 mm de
        // marge haute : y = 595.276 − 28.346 − 340.157 = 226.773
        let ty_v = 595.275590 - 10.0 * PT_MM - page().height_mm * PT_MM;
        assert!((v[5] - ty_v).abs() < 0.01, "{v:?}");
        assert!((v[4] - (12.5 * PT_MM)).abs() < 0.01, "{v:?}");
        std::fs::remove_file(&tmp).ok();
    }
}
