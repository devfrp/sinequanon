//! Rendus d'export : SVG vectoriel (mm, imprimable 1:1) et PNG raster.

use crate::curve::Curve;
use crate::grid::{Page, PAS_MM};
use crate::png::Raster;
use std::path::Path;

/// Polylignes d'une courbe, en coordonnées page (mm).
pub type CurvePolys = (Curve, Vec<Vec<(f64, f64)>>);

pub fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn fmt_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

const SUP: [&str; 10] = ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];

fn superscript(k: i64) -> String {
    let neg = k < 0;
    let digits: String = k
        .abs()
        .to_string()
        .chars()
        .map(|c| SUP[c as usize - '0' as usize])
        .collect();
    format!("{}{digits}", if neg { "⁻" } else { "" })
}

fn f(v: f64) -> String {
    format!("{v:.3}")
}

fn svg_line(s: &mut String, x0: f64, y0: f64, x1: f64, y1: f64, stroke: &str, width: f64) {
    s.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{stroke}\" stroke-width=\"{width}\"/>\n",
        f(x0),
        f(y0),
        f(x1),
        f(y1)
    ));
}

/// Construit le SVG complet de la page.
pub fn svg(page: &Page, curves: &[CurvePolys]) -> String {
    let p = page;
    let mut s = String::with_capacity(64 * 1024);
    s.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}mm\" height=\"{:.2}mm\" viewBox=\"0 0 {:.2} {:.2}\">\n",
        p.width_mm, p.height_mm, p.width_mm, p.height_mm
    ));
    s.push_str(&format!(
        "<defs><clipPath id=\"page\"><rect x=\"0\" y=\"0\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath></defs>\n",
        p.width_mm, p.height_mm
    ));
    s.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{:.2}\" height=\"{:.2}\" fill=\"#ffffff\"/>\n",
        p.width_mm, p.height_mm
    ));

    // grille mineure (5 × 5 mm)
    let nx = (p.width_mm / PAS_MM).floor() as i64;
    let ny = (p.height_mm / PAS_MM).floor() as i64;
    for i in 1..nx {
        let x = i as f64 * PAS_MM;
        svg_line(&mut s, x, 0.0, x, p.height_mm, "#dbeaf5", 0.2);
    }
    for i in 1..ny {
        let y = i as f64 * PAS_MM;
        svg_line(&mut s, 0.0, y, p.width_mm, y, "#dbeaf5", 0.2);
    }

    // lignes majeures (une unité / une décade)
    let (xlo, xhi) = p.x_range();
    let (ylo, yhi) = p.y_range();
    let majors_x = p.majors(xlo, xhi, p.log_x);
    let majors_y = p.majors(ylo, yhi, p.log_y);
    for &k in &majors_x {
        let x = p.mm_x(k);
        if x.is_finite() && x > 0.0 && x < p.width_mm {
            svg_line(&mut s, x, 0.0, x, p.height_mm, "#b3cfe3", 0.35);
        }
    }
    for &k in &majors_y {
        let y = p.mm_y(k);
        if y.is_finite() && y > 0.0 && y < p.height_mm {
            svg_line(&mut s, 0.0, y, p.width_mm, y, "#b3cfe3", 0.35);
        }
    }

    // axes et graduations
    let yax = p.mm_y(0.0);
    let xax = p.mm_x(0.0);
    let axe = "#1a1a1a";
    let has_x_axis = yax.is_finite() && yax >= -0.01 && yax <= p.height_mm + 0.01;
    let has_y_axis = xax.is_finite() && xax >= -0.01 && xax <= p.width_mm + 0.01;

    s.push_str("<g clip-path=\"url(#page)\">\n");
    if has_x_axis {
        svg_line(&mut s, 0.0, yax, p.width_mm, yax, axe, 0.5);
        // flèche à droite
        s.push_str(&format!(
            "<polygon points=\"{},{},{},{},{},{}\" fill=\"{axe}\"/>\n",
            f(p.width_mm),
            f(yax),
            f(p.width_mm - 2.4),
            f(yax - 0.9),
            f(p.width_mm - 2.4),
            f(yax + 0.9)
        ));
    }
    if has_y_axis {
        svg_line(&mut s, xax, 0.0, xax, p.height_mm, axe, 0.5);
        // flèche en haut
        s.push_str(&format!(
            "<polygon points=\"{},{},{},{},{},{}\" fill=\"{axe}\"/>\n",
            f(xax),
            f(0.0),
            f(xax - 0.9),
            f(2.4),
            f(xax + 0.9),
            f(2.4)
        ));
    }

    // labels des graduations, si la place le permet
    let label_ok_x = match (majors_x.first(), majors_x.last()) {
        (Some(&a), Some(&b)) => {
            let da = (p.mm_x(b) - p.mm_x(a)).abs() / majors_x.len().max(1) as f64;
            da > 7.0
        }
        _ => true,
    };
    let label_ok_y = match (majors_y.first(), majors_y.last()) {
        (Some(&a), Some(&b)) => {
            let da = (p.mm_y(b) - p.mm_y(a)).abs() / majors_y.len().max(1) as f64;
            da > 7.0
        }
        _ => true,
    };

    let mut origin_done = false;
    for &k in &majors_x {
        let x = p.mm_x(k);
        if !x.is_finite() || x <= 0.0 || x >= p.width_mm {
            continue;
        }
        if has_x_axis {
            svg_line(&mut s, x, yax - 0.8, x, yax + 0.8, axe, 0.3);
        }
        if label_ok_x && k.abs() > 1e-12 {
            let txt = if p.log_x {
                format!("10{}", superscript(k.log10().round() as i64))
            } else {
                fmt_num(k)
            };
            let ytext = if has_x_axis {
                yax + 4.2
            } else {
                p.height_mm - 1.5
            };
            s.push_str(&format!(
                "<text x=\"{}\" y=\"{ytext}\" font-size=\"3\" font-family=\"DejaVu Sans, sans-serif\" fill=\"#333333\" text-anchor=\"middle\">{txt}</text>\n",
                f(x)
            ));
        }
        if k.abs() <= 1e-12 && has_y_axis {
            origin_done = true;
        }
    }
    for &k in &majors_y {
        let y = p.mm_y(k);
        if !y.is_finite() || y <= 0.0 || y >= p.height_mm {
            continue;
        }
        if has_y_axis {
            svg_line(&mut s, xax - 0.8, y, xax + 0.8, y, axe, 0.3);
        }
        if label_ok_y && k.abs() > 1e-12 {
            let txt = if p.log_y {
                format!("10{}", superscript(k.log10().round() as i64))
            } else {
                fmt_num(k)
            };
            let xtext = if has_y_axis { xax - 1.2 } else { 1.5 };
            s.push_str(&format!(
                "<text x=\"{xtext}\" y=\"{}\" font-size=\"3\" font-family=\"DejaVu Sans, sans-serif\" fill=\"#333333\" text-anchor=\"end\">{txt}</text>\n",
                f(y + 1.0)
            ));
        }
    }
    // label de l'origine
    if has_x_axis && has_y_axis && !origin_done {
        s.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"3\" font-family=\"DejaVu Sans, sans-serif\" fill=\"#333333\" text-anchor=\"end\">0</text>\n",
            f(xax - 1.2),
            f(yax + 4.2)
        ));
    }

    // courbes
    for (curve, polys) in curves {
        if !curve.visible {
            continue;
        }
        let stroke = hex(curve.color);
        let width = (curve.thickness as f64 * 0.3).max(0.2);
        for poly in polys {
            if poly.len() < 2 {
                continue;
            }
            let mut d = String::with_capacity(poly.len() * 16);
            for (i, (x, y)) in poly.iter().enumerate() {
                d.push_str(if i == 0 { "M" } else { "L" });
                d.push_str(&format!("{} {} ", f(*x), f(*y)));
            }
            s.push_str(&format!(
                "<path d=\"{d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\n"
            ));
        }
    }
    s.push_str("</g>\n</svg>\n");
    s
}

/// Écrit la page en SVG. Renvoie la taille du fichier.
pub fn write_svg_file(page: &Page, curves: &[CurvePolys], path: &Path) -> std::io::Result<usize> {
    let s = svg(page, curves);
    std::fs::write(path, s.as_bytes())?;
    Ok(s.len())
}

/// Dessine grille et axes dans un raster (sans texte).
fn draw_page_raster(r: &mut Raster, p: &Page, ppm: f64) {
    let w = r.w as f64;
    let h = r.h as f64;
    let minor = [219, 234, 245, 255];
    let major = [179, 207, 227, 255];
    let axis = [26, 26, 26, 255];

    let nx = (p.width_mm / PAS_MM).floor() as i64;
    let ny = (p.height_mm / PAS_MM).floor() as i64;
    for i in 1..nx {
        let x = i as f64 * PAS_MM * ppm;
        r.line(x, 0.0, x, h - 1.0, minor, 1.0);
    }
    for i in 1..ny {
        let y = i as f64 * PAS_MM * ppm;
        r.line(0.0, y, w - 1.0, y, minor, 1.0);
    }

    let (xlo, xhi) = p.x_range();
    let (ylo, yhi) = p.y_range();
    for k in p.majors(xlo, xhi, p.log_x) {
        let x = p.mm_x(k) * ppm;
        if x.is_finite() && x > 0.0 && x < w {
            r.line(x, 0.0, x, h - 1.0, major, 1.0);
        }
    }
    for k in p.majors(ylo, yhi, p.log_y) {
        let y = p.mm_y(k) * ppm;
        if y.is_finite() && y > 0.0 && y < h {
            r.line(0.0, y, w - 1.0, y, major, 1.0);
        }
    }

    let yax = p.mm_y(0.0);
    let xax = p.mm_x(0.0);
    let lw = (0.5 * ppm).clamp(1.0, 3.0);
    if yax.is_finite() && yax >= 0.0 && yax <= p.height_mm {
        let y = yax * ppm;
        r.line(0.0, y, w - 1.0, y, axis, lw);
        // flèche à droite
        let tip = w - 1.0;
        let a = (lw * 2.0).max(4.0);
        r.line(tip - a, y - a / 2.0, tip, y, axis, lw);
        r.line(tip - a, y + a / 2.0, tip, y, axis, lw);
    }
    if xax.is_finite() && xax >= 0.0 && xax <= p.width_mm {
        let x = xax * ppm;
        r.line(x, 0.0, x, h - 1.0, axis, lw);
        let tip = h - 1.0;
        let a = (lw * 2.0).max(4.0);
        r.line(x - a / 2.0, tip - a, x, tip, axis, lw);
        r.line(x + a / 2.0, tip - a, x, tip, axis, lw);
    }
}

/// Écrit la page en PNG à `px_per_mm` pixels par millimètre.
pub fn write_png_file(
    page: &Page,
    curves: &[CurvePolys],
    px_per_mm: f64,
    path: &Path,
) -> std::io::Result<()> {
    let w = ((page.width_mm * px_per_mm).round() as usize).clamp(16, 20000);
    let h = ((page.height_mm * px_per_mm).round() as usize).clamp(16, 20000);
    let ppm = w as f64 / page.width_mm;
    let mut r = Raster::new(w, h, [255, 255, 255, 255]);
    draw_page_raster(&mut r, page, ppm);
    for (curve, polys) in curves {
        if !curve.visible {
            continue;
        }
        let c4 = [curve.color[0], curve.color[1], curve.color[2], 255];
        let th = (curve.thickness as f64 * ppm * 0.3).clamp(1.0, 20.0);
        for poly in polys {
            for seg in poly.windows(2) {
                r.line(
                    seg[0].0 * ppm,
                    seg[0].1 * ppm,
                    seg[1].0 * ppm,
                    seg[1].1 * ppm,
                    c4,
                    th,
                );
            }
        }
    }
    r.write_png(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{sample_curve, Curve, CurveKind};

    #[test]
    fn svg_contient_les_elements() {
        let p = Page::default();
        let c = Curve::fonction("sin(x)", [0, 0, 204]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let s = svg(&p, &[(c, polys)]);
        assert!(s.contains("viewBox=\"0 0 170.00 120.00\""), "{s}");
        assert!(s.contains("clipPath"));
        assert!(s.contains("#0000cc"));
        assert!(s.contains("<path"));
        assert!(s.contains("<text"));
    }

    #[test]
    fn log_labels_en_puissances_de_dix() {
        let p = Page {
            log_x: true,
            ..Default::default()
        };
        let c = Curve::fonction("log(x)", [204, 0, 0]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let s = svg(&p, &[(c, polys)]);
        assert!(
            s.contains("10³") || s.contains("10²") || s.contains("10¹"),
            "{s}"
        );
    }

    #[test]
    fn png_ecrit_un_fichier_valide() {
        let p = Page::default();
        let c = Curve::fonction("sin(x)", [0, 0, 204]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let dir = std::env::temp_dir().join("sqn-test-svg");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");
        write_png_file(&p, &[(c, polys)], 2.0, &path).unwrap();
        let data = std::fs::read(&path).unwrap();
        assert_eq!(
            &data[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn courbe_invisible_ignoree() {
        let p = Page::default();
        let mut c = Curve::fonction("sin(x)", [0, 0, 204]);
        c.visible = false;
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let s = svg(&p, &[(c.clone(), polys)]);
        assert!(!s.contains("#0000cc"));
        let _ = CurveKind::Fonction {
            expr: String::new(),
        }; // silence clippy
    }
}
