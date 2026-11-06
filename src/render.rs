//! Rendus d'export : SVG vectoriel (mm, imprimable 1:1) et PNG raster.

use crate::annot::AnnotDraw;
use crate::curve::Curve;
use crate::grid::{Page, PAS_MM};
use crate::png::{self, Raster};
use crate::schema::ShObj;
use crate::simu::Simulation;
use crate::stats::{Arbre, BoitesMultiples, Piece as PieceStats, Serie1, Serie2, Suite};
use crate::tableau::{pieces, Piece, Tableau};
use crate::tabval::TableValeurs;
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

/// Ligne pointillée (petits carreaux de la grille de statistiques) :
/// tirets de 0,55 mm alternés, comme les captures du manuel.
fn svg_dashed_line(s: &mut String, x0: f64, y0: f64, x1: f64, y1: f64, stroke: &str, width: f64) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    let pas = 1.1; // tiret + trou
    let n = (len / pas).ceil() as i64 + 1;
    for i in 0..n {
        let t0 = (i as f64 * pas / len).min(1.0);
        let t1 = ((i as f64 * pas + 0.55) / len).min(1.0);
        if t1 <= t0 {
            continue;
        }
        svg_line(
            s,
            x0 + t0 * dx,
            y0 + t0 * dy,
            x0 + t1 * dx,
            y0 + t1 * dy,
            stroke,
            width,
        );
    }
}

fn svg_text(s: &mut String, x: f64, y: f64, txt: &str, color: &str, anchor: &str) {
    s.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-size=\"3\" font-family=\"DejaVu Sans, sans-serif\" fill=\"{color}\" text-anchor=\"{anchor}\">{txt}</text>\n",
        f(x),
        f(y)
    ));
}

/// Échappe les caractères spéciaux XML.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Ajoute le SVG d'un tableau (textes mathématiques en serif italique).
fn svg_tableau(s: &mut String, t: &Tableau) {
    if !t.dessiner || t.rows.is_empty() {
        return;
    }
    let traits = hex(t.couleur_traits);
    let textes = hex(t.couleur_textes);
    s.push_str(&format!(
        "<g transform=\"translate({},{})\" stroke=\"{}\">\n",
        f(t.x_mm),
        f(t.y_mm),
        traits
    ));
    for piece in pieces(t) {
        match piece {
            Piece::Ligne { x0, y0, x1, y1, w } => svg_line(s, x0, y0, x1, y1, &traits, w),
            Piece::Texte {
                x,
                y,
                size,
                txt,
                italique,
            } => {
                let (fam, style) = if italique {
                    ("DejaVu Serif, serif", " font-style=\"italic\"")
                } else {
                    ("DejaVu Sans, sans-serif", "")
                };
                s.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-family=\"{fam}\"{style} fill=\"{textes}\" stroke=\"none\">{}</text>\n",
                    f(x),
                    f(y),
                    fmt_num(size),
                    xml_escape(&txt)
                ));
            }
        }
    }
    s.push_str("</g>\n");
}

/// Construit le SVG complet de la page.
#[allow(clippy::too_many_arguments)]
pub fn svg(
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
    courbes: &[Curve],
    tables: &[TableValeurs],
    schema: &[ShObj],
) -> String {
    let p = page;
    // mode stats : les axes viennent des pièces statistiques (grad_x/grad_y)
    let stats_mode = !stats.is_empty()
        || !boites.is_empty()
        || !suites.is_empty()
        || !serie2.is_empty()
        || !arbres.is_empty()
        || !simus.is_empty();
    let mut s = String::with_capacity(64 * 1024);
    s.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}mm\" height=\"{:.2}mm\" viewBox=\"0 0 {:.2} {:.2}\">\n",
        p.width_mm, p.height_mm, p.width_mm, p.height_mm
    ));
    // defs : clip de page + une trame de hachures par couleur d'aire
    s.push_str("<defs>");
    s.push_str(&format!(
        "<clipPath id=\"page\"><rect x=\"0\" y=\"0\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath>",
        p.width_mm, p.height_mm
    ));
    let mut hatch_colors: Vec<[u8; 3]> = Vec::new();
    for a in annots {
        if let AnnotDraw::Integrale { color, .. } = a {
            if !hatch_colors.contains(color) {
                hatch_colors.push(*color);
                s.push_str(&format!(
                    "<pattern id=\"hatch-{:02x}{:02x}{:02x}\" width=\"1.8\" height=\"1.8\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate(45)\"><line x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1.8\" stroke=\"{}\" stroke-width=\"0.4\"/></pattern>",
                    color[0], color[1], color[2],
                    hex(*color)
                ));
            }
        }
    }
    s.push_str("</defs>\n");
    s.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{:.2}\" height=\"{:.2}\" fill=\"#ffffff\"/>\n",
        p.width_mm, p.height_mm
    ));

    // grille mineure (5 × 5 mm). Mode statistiques : petits carreaux
    // pointillés bleu-gris comme l'original (captures du manuel), sans
    // lignes majeures par unité ; sinon grille continuelle + majeures,
    // tels que publiés en v0.2/0.3 pour les courbes.
    let nx = (p.width_mm / PAS_MM).floor() as i64;
    let ny = (p.height_mm / PAS_MM).floor() as i64;
    if stats_mode {
        let coul = "#4a5464";
        for i in 1..nx {
            svg_dashed_line(
                &mut s,
                i as f64 * PAS_MM,
                0.0,
                i as f64 * PAS_MM,
                p.height_mm,
                coul,
                0.2,
            );
        }
        for i in 1..ny {
            svg_dashed_line(
                &mut s,
                0.0,
                i as f64 * PAS_MM,
                p.width_mm,
                i as f64 * PAS_MM,
                coul,
                0.2,
            );
        }
    } else {
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
    }

    if !stats_mode {
        // axes et graduations
        let yax = p.mm_y(0.0);
        let xax = p.mm_x(0.0);
        let axe = "#1a1a1a";
        let has_x_axis = yax.is_finite() && yax >= -0.01 && yax <= p.height_mm + 0.01;
        let has_y_axis = xax.is_finite() && xax >= -0.01 && xax <= p.width_mm + 0.01;
        let (xlo, xhi) = p.x_range();
        let (ylo, yhi) = p.y_range();
        let majors_x = p.majors(xlo, xhi, p.log_x);
        let majors_y = p.majors(ylo, yhi, p.log_y);

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
    // annotations : aires hachurées d'abord (sous les labels), puis
    // tangentes et points par-dessus
    for a in annots {
        if let AnnotDraw::Integrale {
            path,
            label,
            label_mm,
            color,
            ..
        } = a
        {
            let stroke = hex(*color);
            let mut d = String::with_capacity(path.len() * 16);
            for (i, (x, y)) in path.iter().enumerate() {
                d.push_str(if i == 0 { "M" } else { "L" });
                d.push_str(&format!("{} {} ", f(*x), f(*y)));
            }
            d.push('Z');
            s.push_str(&format!(
                "<path d=\"{d}\" fill=\"url(#hatch-{:02x}{:02x}{:02x})\" stroke=\"{stroke}\" stroke-width=\"0.4\"/>\n",
                color[0], color[1], color[2]
            ));
            svg_text(
                &mut s,
                label_mm.0,
                label_mm.1 - 1.2,
                &format!("∫ ≈ {label}"),
                &stroke,
                "middle",
            );
        }
    }
    for a in annots {
        match a {
            AnnotDraw::Point {
                mm, label, color, ..
            } => {
                let stroke = hex(*color);
                s.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"0.9\" fill=\"#ffffff\" stroke=\"{stroke}\" stroke-width=\"0.5\"/>\n",
                    f(mm.0),
                    f(mm.1)
                ));
                svg_text(&mut s, mm.0 + 1.4, mm.1 - 1.4, label, &stroke, "start");
            }
            AnnotDraw::Tangente {
                x0,
                y0,
                x1,
                y1,
                mm,
                label,
                color,
                ..
            } => {
                let stroke = hex(*color);
                svg_line(&mut s, *x0, *y0, *x1, *y1, &stroke, 0.5);
                s.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"0.9\" fill=\"#ffffff\" stroke=\"{stroke}\" stroke-width=\"0.5\"/>\n",
                    f(mm.0),
                    f(mm.1)
                ));
                svg_text(&mut s, mm.0 + 1.4, mm.1 - 1.4, label, &stroke, "start");
            }
            AnnotDraw::Integrale { .. } => {}
        }
    }
    // tableaux de signes et de variations, par-dessus la grille
    for t in tableaux {
        svg_tableau(&mut s, t);
    }
    // séries statistiques à une variable, puis boîtes à moustaches
    // multiples : le fond opaque du camembert masque la grille comme
    // dans l'original
    for sr in stats {
        svg_stats_pieces(&mut s, &sr.draw_list(p));
    }
    for b in boites {
        svg_stats_pieces(&mut s, &b.draw_list(p));
    }
    for su in suites {
        svg_stats_pieces(&mut s, &su.draw_list(p));
    }
    for s2 in serie2 {
        svg_stats_pieces(&mut s, &s2.draw_list(p));
    }
    for a in arbres {
        svg_stats_pieces(&mut s, &a.draw_list(p));
    }
    for si in simus {
        svg_stats_pieces(&mut s, &si.draw_list(p));
    }
    // tables de valeurs (v1.4) : le même ordre que les modules stats
    for tv in tables {
        svg_stats_pieces(&mut s, &tv.draw_list(courbes));
    }
    // schéma (v1.3 + v1.5) : zones d'abord, objets, puis textes au
    // premier plan ; sans déclencher le mode stats (repère normal)
    svg_stats_pieces(&mut s, &ShObj::draw_schema(p, schema));
    s.push_str("</g>\n</svg>\n");
    s
}

/// SVG d'une liste de pièces statistiques (coordonnées mm de page).
fn svg_stats_pieces(s: &mut String, pieces: &[PieceStats]) {
    for piece in pieces {
        match piece {
            PieceStats::Fond { mm, w, h } => {
                // fond opaque : masque grille et axes sous le camembert
                s.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>\n",
                    f(mm.0),
                    f(mm.1),
                    f(*w),
                    f(*h)
                ));
            }
            PieceStats::Rect {
                mm,
                w,
                h,
                fill,
                stroke,
            } => {
                let (stc, stw) = stroke;
                let fill_attr = match fill {
                    Some(c) => format!("fill=\"{}\"", hex(*c)),
                    None => "fill=\"none\"".to_string(),
                };
                s.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {fill_attr} stroke=\"{}\" stroke-width=\"{}\"/>\n",
                    f(mm.0),
                    f(mm.1),
                    f(*w),
                    f(*h),
                    hex(*stc),
                    stw
                ));
            }
            PieceStats::Poly { pts, fill, stroke } => {
                let mut d = String::with_capacity(pts.len() * 14);
                for (i, (x, y)) in pts.iter().enumerate() {
                    d.push_str(if i == 0 { "M" } else { "L" });
                    d.push_str(&format!("{} {} ", f(*x), f(*y)));
                }
                d.push('Z');
                let fill_attr = match fill {
                    Some(c) => format!("fill=\"{}\"", hex(*c)),
                    None => "fill=\"none\"".to_string(),
                };
                let stroke_attr = match stroke {
                    Some((c, w)) => format!("stroke=\"{}\" stroke-width=\"{}\"", hex(*c), w),
                    None => "stroke=\"none\"".to_string(),
                };
                s.push_str(&format!(
                    "<path d=\"{d}\" {fill_attr} {stroke_attr} stroke-linejoin=\"round\"/>\n"
                ));
            }
            PieceStats::Ligne {
                x0,
                y0,
                x1,
                y1,
                c,
                w,
            } => {
                svg_line(s, *x0, *y0, *x1, *y1, &hex(*c), *w);
            }
            PieceStats::Pointille {
                x0,
                y0,
                x1,
                y1,
                c,
                w,
            } => {
                s.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"1.6 1.3\"/>\n",
                    f(*x0),
                    f(*y0),
                    f(*x1),
                    f(*y1),
                    hex(*c),
                    w
                ));
            }
            PieceStats::Disque {
                c: centre,
                r,
                fill,
                stroke,
            } => {
                let stroke_attr = match stroke {
                    Some((c, w)) => format!("stroke=\"{}\" stroke-width=\"{}\"", hex(*c), w),
                    None => "stroke=\"none\"".to_string(),
                };
                s.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" {stroke_attr}/>\n",
                    f(centre.0),
                    f(centre.1),
                    f(*r),
                    hex(*fill)
                ));
            }
            PieceStats::Texte {
                mm,
                txt,
                c,
                taille,
                ancre,
                vertical,
                gras,
                italique,
            } => {
                let mut style = if *gras { " font-weight=\"bold\"" } else { "" }.to_string();
                if *italique {
                    style += " font-style=\"italic\"";
                }
                let fam = if *italique {
                    "DejaVu Serif, serif"
                } else {
                    "DejaVu Sans, sans-serif"
                };
                let rot = if *vertical {
                    format!(" transform=\"rotate(-90 {} {})\"", f(mm.0), f(mm.1))
                } else {
                    String::new()
                };
                s.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\"{style} font-family=\"{fam}\" fill=\"{}\" text-anchor=\"{}\"{rot}>{}</text>\n",
                    f(mm.0),
                    f(mm.1),
                    fmt_num(*taille),
                    hex(*c),
                    ancre,
                    xml_escape(txt)
                ));
            }
        }
    }
}

/// Écrit la page en SVG. Renvoie la taille du fichier.
#[allow(clippy::too_many_arguments)]
pub fn write_svg_file(
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
    courbes: &[Curve],
    tables: &[TableValeurs],
    schema: &[ShObj],
    path: &Path,
) -> std::io::Result<usize> {
    let s = svg(
        page, curves, annots, tableaux, stats, boites, suites, serie2, arbres, simus, courbes,
        tables, schema,
    );
    std::fs::write(path, s.as_bytes())?;
    Ok(s.len())
}

/// Dessine grille et axes dans un raster (sans texte).
fn draw_page_raster(r: &mut Raster, p: &Page, ppm: f64, stats_mode: bool) {
    let w = r.w as f64;
    let h = r.h as f64;
    let minor = [219, 234, 245, 255];
    let major = [179, 207, 227, 255];
    let axis = [26, 26, 26, 255];

    let nx = (p.width_mm / PAS_MM).floor() as i64;
    let ny = (p.height_mm / PAS_MM).floor() as i64;
    if stats_mode {
        // petits carreaux pointillés bleu-gris (captures du manuel), pas
        // de mineures continuelles ni de majeures par unité
        for i in 1..nx {
            let x = i as f64 * PAS_MM;
            r.dash_line(x, 0.0, x, p.height_mm, [62, 68, 84, 255], 1.0, 1.1, ppm);
        }
        for i in 1..ny {
            let y = i as f64 * PAS_MM;
            r.dash_line(0.0, y, p.width_mm, y, [62, 68, 84, 255], 1.0, 1.1, ppm);
        }
        return;
    }
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

    if stats_mode {
        return;
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

/// Point dans un polygone (test pair-impair, coordonnées quelconques).
fn point_in_poly(path: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut j = path.len() - 1;
    for (i, &(xi, yi)) in path.iter().enumerate() {
        let (xj, yj) = path[j];
        if (yi > y) != (yj > y) {
            let x_int = (xj - xi) * (y - yi) / (yj - yi) + xi;
            if x < x_int {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Disque rempli (marqueur de point/tangence).
fn raster_disc(r: &mut Raster, cx: f64, cy: f64, radius: f64, c: [u8; 4]) {
    let r2 = radius * radius;
    for py in ((cy - radius).floor() as i64)..=((cy + radius).ceil() as i64) {
        for px in ((cx - radius).floor() as i64)..=((cx + radius).ceil() as i64) {
            let dx = px as f64 + 0.5 - cx;
            let dy = py as f64 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                r.set(px, py, c);
            }
        }
    }
}

/// Dessine les annotations dans un raster. Les libellés texte ne sont
/// restitués qu'en SVG (pas de police dans le raster) ; marqueurs et
/// hachures y sont fidèles.
fn draw_annots_raster(r: &mut Raster, annots: &[AnnotDraw], ppm: f64) {
    // aires hachurées d'abord
    for a in annots {
        if let AnnotDraw::Integrale { path, color, .. } = a {
            let c4 = [color[0], color[1], color[2], 255];
            // trame diagonale : période 1,8 mm, trait 0,35 mm (comme le SVG)
            let period = (1.8 * ppm).max(3.0);
            let width = (0.35 * ppm).max(1.0);
            let mut minx = f64::MAX;
            let mut maxx = f64::MIN;
            let mut miny = f64::MAX;
            let mut maxy = f64::MIN;
            for &(x, y) in path {
                minx = minx.min(x);
                maxx = maxx.max(x);
                miny = miny.min(y);
                maxy = maxy.max(y);
            }
            // balayage en pixels, test d'intérieur en millimètres
            for py in (miny * ppm).floor() as i64..=((maxy * ppm).ceil() as i64) {
                for px in (minx * ppm).floor() as i64..=((maxx * ppm).ceil() as i64) {
                    if !point_in_poly(path, px as f64 / ppm, py as f64 / ppm) {
                        continue;
                    }
                    if ((px + py) as f64).rem_euclid(period) < width {
                        r.set(px, py, c4);
                    }
                }
            }
            for seg in path.windows(2) {
                r.line(
                    seg[0].0 * ppm,
                    seg[0].1 * ppm,
                    seg[1].0 * ppm,
                    seg[1].1 * ppm,
                    c4,
                    1.0,
                );
            }
        }
    }
    // tangentes puis points
    for a in annots {
        match a {
            AnnotDraw::Tangente {
                x0,
                y0,
                x1,
                y1,
                mm,
                color,
                ..
            } => {
                let c4 = [color[0], color[1], color[2], 255];
                r.line(
                    x0 * ppm,
                    y0 * ppm,
                    x1 * ppm,
                    y1 * ppm,
                    c4,
                    (0.5 * ppm).clamp(1.0, 3.0),
                );
                let rad = (0.9 * ppm).max(2.0);
                raster_disc(r, mm.0 * ppm, mm.1 * ppm, rad, [255, 255, 255, 255]);
                raster_disc(
                    r,
                    mm.0 * ppm,
                    mm.1 * ppm,
                    rad * 0.55,
                    [color[0], color[1], color[2], 255],
                );
            }
            AnnotDraw::Point { mm, color, .. } => {
                let rad = (0.9 * ppm).max(2.0);
                raster_disc(r, mm.0 * ppm, mm.1 * ppm, rad, [255, 255, 255, 255]);
                raster_disc(
                    r,
                    mm.0 * ppm,
                    mm.1 * ppm,
                    rad * 0.55,
                    [color[0], color[1], color[2], 255],
                );
            }
            AnnotDraw::Integrale { .. } => {}
        }
    }
}

/// Dessine un tableau dans un raster (police bitmap 5×7 pour les textes).
fn draw_tableau_raster(r: &mut Raster, t: &Tableau, ppm: f64) {
    if !t.dessiner || t.rows.is_empty() {
        return;
    }
    let c_traits = [
        t.couleur_traits[0],
        t.couleur_traits[1],
        t.couleur_traits[2],
        255,
    ];
    let c_textes = [
        t.couleur_textes[0],
        t.couleur_textes[1],
        t.couleur_textes[2],
        255,
    ];
    for piece in pieces(t) {
        match piece {
            Piece::Ligne { x0, y0, x1, y1, w } => {
                r.line(
                    (t.x_mm + x0) * ppm,
                    (t.y_mm + y0) * ppm,
                    (t.x_mm + x1) * ppm,
                    (t.y_mm + y1) * ppm,
                    c_traits,
                    (w * ppm).clamp(1.0, 4.0),
                );
            }
            Piece::Texte {
                x,
                y,
                size,
                txt,
                italique,
            } => {
                r.draw_text(
                    (t.x_mm + x) * ppm,
                    (t.y_mm + y) * ppm,
                    &txt,
                    size * ppm,
                    c_textes,
                    italique,
                );
            }
        }
    }
}

/// Écrit la page en PNG à `px_per_mm` pixels par millimètre (les
/// arguments décrivent tout le contenu d'une page, d'où leur nombre).
#[allow(clippy::too_many_arguments)]
/// Rasterise la page entière (grille, courbes, annotations, tableaux,
/// modules statistiques, schéma) : rangées RGBA cm-haut → haut-bas.
pub fn rasterize_page(
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
    courbes: &[Curve],
    tables: &[TableValeurs],
    schema: &[ShObj],
    px_per_mm: f64,
) -> Raster {
    let w = ((page.width_mm * px_per_mm).round() as usize).clamp(16, 20000);
    let h = ((page.height_mm * px_per_mm).round() as usize).clamp(16, 20000);
    let ppm = w as f64 / page.width_mm;
    let mut r = Raster::new(w, h, [255, 255, 255, 255]);
    // mode stats : les axes viennent des pièces statistiques
    let stats_mode = !stats.is_empty()
        || !boites.is_empty()
        || !suites.is_empty()
        || !serie2.is_empty()
        || !arbres.is_empty()
        || !simus.is_empty();
    draw_page_raster(&mut r, page, ppm, stats_mode);
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
    draw_annots_raster(&mut r, annots, ppm);
    for t in tableaux {
        draw_tableau_raster(&mut r, t, ppm);
    }
    for sr in stats {
        draw_stats_raster(&mut r, ppm, &sr.draw_list(page));
    }
    for b in boites {
        draw_stats_raster(&mut r, ppm, &b.draw_list(page));
    }
    for su in suites {
        draw_stats_raster(&mut r, ppm, &su.draw_list(page));
    }
    for s2 in serie2 {
        draw_stats_raster(&mut r, ppm, &s2.draw_list(page));
    }
    for a in arbres {
        draw_stats_raster(&mut r, ppm, &a.draw_list(page));
    }
    for si in simus {
        draw_stats_raster(&mut r, ppm, &si.draw_list(page));
    }
    // tables de valeurs (v1.4)
    for tv in tables {
        draw_stats_raster(&mut r, ppm, &tv.draw_list(courbes));
    }
    // schéma (v1.3 + v1.5) : même place qu'en SVG, sans mode stats
    draw_stats_raster(&mut r, ppm, &ShObj::draw_schema(page, schema));
    r
}

/// Rasterise puis écrit un PNG.
#[allow(clippy::too_many_arguments)]
pub fn write_png_file(
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
    courbes: &[Curve],
    tables: &[TableValeurs],
    schema: &[ShObj],
    px_per_mm: f64,
    path: &Path,
) -> std::io::Result<()> {
    rasterize_page(
        page, curves, annots, tableaux, stats, boites, suites, serie2, arbres, simus, courbes,
        tables, schema, px_per_mm,
    )
    .write_png(path)
}

/// Rasterise puis écrit un BMP 24 bits (man. p. 100).
#[allow(clippy::too_many_arguments)]
pub fn write_bmp_file(
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
    courbes: &[Curve],
    tables: &[TableValeurs],
    schema: &[ShObj],
    px_per_mm: f64,
    path: &Path,
) -> std::io::Result<()> {
    rasterize_page(
        page, curves, annots, tableaux, stats, boites, suites, serie2, arbres, simus, courbes,
        tables, schema, px_per_mm,
    )
    .write_bmp(path)
}

/// Rasterise puis écrit un GIF 256 couleurs (man. p. 100).
#[allow(clippy::too_many_arguments)]
pub fn write_gif_file(
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
    courbes: &[Curve],
    tables: &[TableValeurs],
    schema: &[ShObj],
    px_per_mm: f64,
    path: &Path,
) -> std::io::Result<()> {
    rasterize_page(
        page, curves, annots, tableaux, stats, boites, suites, serie2, arbres, simus, courbes,
        tables, schema, px_per_mm,
    )
    .write_gif(path)
}

/// EPS niveau 2 avec l'image rasterisée embarquée (hexadécimal) —
/// directement importable dans un document LaTeX via \includegraphics.
#[allow(clippy::too_many_arguments)]
pub fn write_eps_file(
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
    courbes: &[Curve],
    tables: &[TableValeurs],
    schema: &[ShObj],
    px_per_mm: f64,
    path: &Path,
) -> std::io::Result<()> {
    use std::fmt::Write as _;
    let r = rasterize_page(
        page, curves, annots, tableaux, stats, boites, suites, serie2, arbres, simus, courbes,
        tables, schema, px_per_mm,
    );
    let (w, h) = (r.w, r.h);
    // la zone englobante est en points PostScript (72 pt = 1 pouce) ;
    // forme canonique : le carré unitaire est mis à l'échelle sur la
    // page, et l'ImageMatrix [w 0 0 -h 0 h] remplit ce carré (image
    // lisible de haut en bas : la ligne 1 des données est le haut)
    let wpt_e = page.width_mm * 72.0 / 25.4;
    let hpt_e = page.height_mm * 72.0 / 25.4;
    let mut s = String::with_capacity(w * h * 2 / 3 + 2048);
    writeln!(s, "%!PS-Adobe-2.0 EPSF-2.0").unwrap();
    writeln!(s, "%%BoundingBox: 0 0 {wpt_e:.3} {hpt_e:.3}").unwrap();
    writeln!(s, "%%Title: sinequanon").unwrap();
    writeln!(
        s,
        "%%Creator: sinequanon {} (Rust) — image rasterisée {}×{} px",
        env!("CARGO_PKG_VERSION"),
        w,
        h
    )
    .unwrap();
    writeln!(s, "%%EndComments").unwrap();
    writeln!(s, "%%Page: 1 1").unwrap();
    writeln!(s, "save").unwrap();
    writeln!(s, "{wpt_e:.3} {hpt_e:.3} scale").unwrap();
    writeln!(
        s,
        "/DeviceRGB setcolorspace << /ImageType 1 /Width {w} /Height {h} /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /ImageMatrix [{w} 0 0 -{h} 0 {h}] /DataSource currentfile /ASCIIHexDecode filter >> image"
    )
    .unwrap();
    // données hexadécimales (80 caractères par ligne)
    let mut line = String::new();
    for px in r.px.chunks_exact(4) {
        for b in &px[..3] {
            write!(line, "{:02x}", b).unwrap();
        }
        if line.len() >= 78 {
            s.push_str(&line);
            s.push('\n');
            line.clear();
        }
    }
    if !line.is_empty() {
        s.push_str(&line);
        s.push('\n');
    }
    writeln!(s, ">").unwrap(); // marqueur de fin ASCIIHexDecode
    writeln!(s, "restore").unwrap();
    writeln!(s, "%%Trailer").unwrap();
    writeln!(s, "%%EOF").unwrap();
    std::fs::write(path, s)
}

/// Rendu raster des pièces statistiques (coordonnées mm → px).
fn draw_stats_raster(r: &mut Raster, ppm: f64, pieces: &[PieceStats]) {
    for piece in pieces {
        match piece {
            PieceStats::Fond { mm, w, h } => {
                // fond opaque, par-dessus la grille
                r.rect(
                    mm.0 * ppm,
                    mm.1 * ppm,
                    w * ppm,
                    h * ppm,
                    [255, 255, 255, 255],
                );
            }
            PieceStats::Rect {
                mm,
                w,
                h,
                fill,
                stroke,
            } => {
                if let Some(c) = fill {
                    r.rect(
                        mm.0 * ppm,
                        mm.1 * ppm,
                        w * ppm,
                        h * ppm,
                        [c[0], c[1], c[2], 255],
                    );
                }
                let (stc, stw) = stroke;
                let th = (stw * ppm).clamp(0.8, 6.0);
                let (x0, y0) = (mm.0 * ppm, mm.1 * ppm);
                let (x1, y1) = ((mm.0 + w) * ppm, (mm.1 + h) * ppm);
                let c4 = [stc[0], stc[1], stc[2], 255];
                // contour antihoraire pour couvrir proprement les coins
                for (xa, ya, xb, yb) in [
                    (x0, y0, x1, y0),
                    (x1, y0, x1, y1),
                    (x1, y1, x0, y1),
                    (x0, y1, x0, y0),
                ] {
                    r.line(xa, ya, xb, yb, c4, th);
                }
            }
            PieceStats::Poly { pts, fill, stroke } => {
                if let Some(c) = fill {
                    let pl: Vec<(f64, f64)> = pts.iter().map(|(x, y)| (x * ppm, y * ppm)).collect();
                    png::poly(r, &pl, [c[0], c[1], c[2], 255]);
                }
                if let Some((c, w)) = stroke {
                    let th = (w * ppm).clamp(0.8, 6.0);
                    let c4 = [c[0], c[1], c[2], 255];
                    for seg in pts.windows(2) {
                        r.line(
                            seg[0].0 * ppm,
                            seg[0].1 * ppm,
                            seg[1].0 * ppm,
                            seg[1].1 * ppm,
                            c4,
                            th,
                        );
                    }
                    // referme le polygone
                    if pts.len() > 2 {
                        let (xa, ya) = pts[pts.len() - 1];
                        let (xb, yb) = pts[0];
                        r.line(xa * ppm, ya * ppm, xb * ppm, yb * ppm, c4, th);
                    }
                }
            }
            PieceStats::Ligne {
                x0,
                y0,
                x1,
                y1,
                c,
                w,
            } => {
                let th = (w * ppm).clamp(0.8, 14.0);
                r.line(
                    x0 * ppm,
                    y0 * ppm,
                    x1 * ppm,
                    y1 * ppm,
                    [c[0], c[1], c[2], 255],
                    th,
                );
            }
            PieceStats::Pointille {
                x0,
                y0,
                x1,
                y1,
                c,
                w,
            } => {
                let th = (w * ppm).clamp(0.8, 6.0);
                png::dashed(
                    r,
                    x0 * ppm,
                    y0 * ppm,
                    x1 * ppm,
                    y1 * ppm,
                    [c[0], c[1], c[2], 255],
                    th,
                );
            }
            PieceStats::Disque {
                c: centre,
                r: ray,
                fill,
                stroke,
            } => {
                png::disque(
                    r,
                    centre.0 * ppm,
                    centre.1 * ppm,
                    ray * ppm,
                    [fill[0], fill[1], fill[2], 255],
                );
                if let Some((c, _w)) = stroke {
                    // anneau : petit cercle en segments épais
                    let th = 1.0f64.max(0.25 * ppm);
                    let n = 16;
                    let c4 = [c[0], c[1], c[2], 255];
                    let ray = ray * ppm;
                    for i in 0..n {
                        let a0 = std::f64::consts::TAU * i as f64 / n as f64;
                        let a1 = std::f64::consts::TAU * (i + 1) as f64 / n as f64;
                        r.line(
                            centre.0 * ppm + ray * a0.cos(),
                            centre.1 * ppm + ray * a0.sin(),
                            centre.0 * ppm + ray * a1.cos(),
                            centre.1 * ppm + ray * a1.sin(),
                            c4,
                            th,
                        );
                    }
                }
            }
            PieceStats::Texte {
                mm,
                txt,
                c,
                taille,
                ancre,
                vertical,
                gras,
                italique,
            } => {
                // ancrage : largeur mesurée à la taille bitmap (comme le
                // reste de l'export raster)
                let size_px = taille * ppm;
                let c4 = [c[0], c[1], c[2], 255];
                if *vertical {
                    let (x, y) = (mm.0 * ppm, mm.1 * ppm);
                    // ancre : la fin (end) ou le centre (middle) au point
                    let len = png::texte_largeur_px(txt, size_px);
                    let (x0, y0) = match *ancre {
                        "end" => (x, y + len),
                        "middle" => (x, y + len / 2.0),
                        _ => (x, y),
                    };
                    if *gras {
                        png::draw_text_vertical(r, x0, y0, txt, size_px, c4);
                        png::draw_text_vertical(r, x0 + 1.0, y0, txt, size_px, c4);
                    } else {
                        png::draw_text_vertical(r, x0, y0, txt, size_px, c4);
                    }
                } else {
                    let w = png::texte_largeur_px(txt, size_px);
                    let x = match *ancre {
                        "end" => mm.0 * ppm - w,
                        "middle" => mm.0 * ppm - w / 2.0,
                        _ => mm.0 * ppm,
                    };
                    // la ligne de base bitmap : la casse monte au-dessus
                    let y = mm.1 * ppm + (taille * ppm) * 0.72;
                    if *gras {
                        r.draw_text(x, y, txt, size_px, c4, *italique);
                        r.draw_text(x + 1.0, y, txt, size_px, c4, *italique);
                    } else {
                        r.draw_text(x, y, txt, size_px, c4, *italique);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annot::{draw_list, IntegraleAnnot, PointAnnot, TangenteAnnot};
    use crate::curve::{sample_curve, Curve, CurveKind};

    #[test]
    fn svg_contient_les_elements() {
        let p = Page::default();
        let c = Curve::fonction("sin(x)", [0, 0, 204]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let s = svg(
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
            &[],
            &[],
        );
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
        let s = svg(
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
            &[],
            &[],
        );
        assert!(
            s.contains("10³") || s.contains("10²") || s.contains("10¹"),
            "{s}"
        );
    }

    #[test]
    fn svg_integrale_hachures_et_valeur() {
        let p = Page::default();
        let c = Curve::fonction("x^2", [0, 0, 204]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let annots = draw_list(
            &p,
            std::slice::from_ref(&c),
            &[],
            &[],
            &[IntegraleAnnot {
                curve: 0,
                a: 0.0,
                b: 2.0,
            }],
        );
        let s = svg(
            &p,
            &[(c, polys)],
            &annots,
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
        );
        assert!(s.contains("<pattern"), "trame manquante : {s}");
        assert!(s.contains("url(#hatch-0000cc)"), "{s}");
        assert!(s.contains("rotate(45)"), "{s}");
        assert!(s.contains("∫ ≈ 2.6667"), "{s}");
    }

    #[test]
    fn svg_point_et_tangente() {
        let p = Page::default();
        let c = Curve::fonction("x^2", [0, 0, 204]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let annots = draw_list(
            &p,
            std::slice::from_ref(&c),
            &[PointAnnot { curve: 0, x: 1.0 }],
            &[TangenteAnnot { curve: 0, x: 2.0 }],
            &[],
        );
        let s = svg(
            &p,
            &[(c, polys)],
            &annots,
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
        );
        assert!(s.matches("<circle").count() == 2, "{s}");
        assert!(s.contains("(1.000 ; 1.000)"), "{s}");
        assert!(s.contains("y = 4.000x"), "{s}");
    }

    #[test]
    fn png_ecrit_un_fichier_valide() {
        let p = Page::default();
        let c = Curve::fonction("sin(x)", [0, 0, 204]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let dir = std::env::temp_dir().join("sqn-test-svg");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");
        write_png_file(
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
            &[],
            &[],
            2.0,
            &path,
        )
        .unwrap();
        let data = std::fs::read(&path).unwrap();
        assert_eq!(
            &data[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn png_avec_annots_ecrit_un_fichier_valide() {
        let p = Page::default();
        let c = Curve::fonction("x^2", [0, 0, 204]);
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let annots = draw_list(
            &p,
            std::slice::from_ref(&c),
            &[PointAnnot { curve: 0, x: 1.0 }],
            &[TangenteAnnot { curve: 0, x: 1.5 }],
            &[IntegraleAnnot {
                curve: 0,
                a: 0.0,
                b: 2.0,
            }],
        );
        assert_eq!(annots.len(), 3);
        let dir = std::env::temp_dir().join("sqn-test-svg");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test-annots.png");
        write_png_file(
            &p,
            &[(c, polys)],
            &annots,
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
            2.0,
            &path,
        )
        .unwrap();
        let data = std::fs::read(&path).unwrap();
        assert_eq!(
            &data[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn schema_point_et_bezier_dans_svg() {
        // bout en bout : objets du schéma → pièces → SVG (après les
        // simulations, sans mode stats)
        use crate::schema::{Bezier, PointSchema, ShObj};
        let p = Page::default();
        let schema = vec![
            ShObj::Pt(PointSchema {
                nom: "A".into(),
                x: 0.0,
                y: 0.0,
            }),
            ShObj::Pt(PointSchema {
                nom: "B".into(),
                x: 1.0,
                y: 2.0,
            }),
            ShObj::Pt(PointSchema {
                nom: "C".into(),
                x: 4.0,
                y: 2.0,
            }),
            ShObj::Pt(PointSchema {
                nom: "D".into(),
                x: 5.0,
                y: 0.0,
            }),
            ShObj::Bez(Bezier {
                a: "A".into(),
                b: "B".into(),
                c: "C".into(),
                d: "D".into(),
                couleur: [0, 0, 204],
                epaisseur: 1.0,
            }),
        ];
        let s = svg(
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
            &[],
            &schema,
        );
        // croix des 4 points + nom italique
        assert!(s.matches("<line").count() >= 8, "{s}");
        assert!(s.contains("font-style=\"italic\""), "{s}");
        assert!(s.contains(">A</text>"), "{s}");
        // courbe : chemin à 80 segments bleus (polyligne de 81 points)
        let pl = s
            .lines()
            .find(|l| l.contains("stroke=\"#0000cc\""))
            .unwrap();
        assert!(pl.contains("<path"), "{pl}");
        assert_eq!(pl.matches('L').count(), 80, "{pl}");
        // les points manquants ne dessinent rien
        let b = ShObj::Bez(Bezier {
            a: "A".into(),
            b: "B".into(),
            c: "C".into(),
            d: "Z".into(),
            ..Default::default()
        });
        assert!(b.draw_list(&p, &schema).is_empty());
    }

    #[test]
    fn courbe_invisible_ignoree() {
        let p = Page::default();
        let mut c = Curve::fonction("sin(x)", [0, 0, 204]);
        c.visible = false;
        let polys = sample_curve(&c, &p, 200, 200).unwrap();
        let s = svg(
            &p,
            &[(c.clone(), polys)],
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
            &[],
        );
        assert!(!s.contains("#0000cc"));
        let _ = CurveKind::Fonction {
            expr: String::new(),
        }; // silence clippy
    }

    #[test]
    fn tableau_dans_svg_et_png() {
        let p = Page::default();
        let mut t = crate::tableau::parse(crate::tableau::EX_VAR4).unwrap();
        t.dessiner = true;
        t.x_mm = 10.0;
        t.y_mm = 12.0;
        let s = svg(
            &p,
            &[],
            &[],
            &[t.clone()],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
        );
        let g = s
            .split('\n')
            .find(|l| l.contains("<g transform="))
            .expect("groupe tableau absent");
        assert!(
            g.contains("translate(10.000,12.000)"),
            "groupe tableau mal placé : {g}"
        );
        // valeurs sur les frontières et textes des cases
        assert!(s.contains(">-1<") && s.contains(">0<"));
        assert!(s.contains("font-style=\"italic\""));
        // PNG : la police bitmap doit marquer des pixels de texte
        let dir = std::env::var("SQN_TMP")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let path = dir.join("sqn_test_tableau.png");
        write_png_file(
            &p,
            &[],
            &[],
            &[t],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            6.0,
            &path,
        )
        .unwrap();
        let txt = std::fs::read(&path).unwrap();
        assert_eq!(&txt[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        if std::env::var("SQN_TMP").is_err() {
            let _ = std::fs::remove_file(&path);
        }
    }

    /// Aperçus PNG pour contrôle visuel : les 10 exemples du module
    /// tableau et deux tableaux générés depuis une courbe.
    /// Sans `SQN_TMP` défini, le test ne fait rien (au départ de cargo).
    #[test]
    fn dump_visuel() {
        let Ok(dir) = std::env::var("SQN_TMP") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        for (nom, code) in crate::tableau::exemples() {
            let mut t = crate::tableau::parse(code).unwrap();
            t.dessiner = true;
            let (tw, th) = crate::tableau::dimensions(&t);
            // page ajustée au tableau : aperçu zoomé
            let p = Page {
                width_mm: tw + 10.0,
                height_mm: th + 10.0,
                ..Page::default()
            };
            t.x_mm = 5.0;
            t.y_mm = 5.0;
            let path = dir.join(format!("ex-{nom}.png"));
            write_png_file(
                &p,
                &[],
                &[],
                &[t],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                10.0,
                &path,
            )
            .unwrap();
        }
        // tableaux générés depuis une courbe
        for (nom, (e, a, b)) in [
            ("gen-para", ("x^2-2", -3.0, 3.0)),
            ("gen-sin", ("sin(x)", -4.0, 4.0)),
        ] {
            let code = crate::tabgen::genere(e, a, b, true).unwrap();
            let mut t = crate::tableau::parse(&code).unwrap();
            t.dessiner = true;
            let (tw, th) = crate::tableau::dimensions(&t);
            let p2 = Page {
                width_mm: tw + 10.0,
                height_mm: th + 10.0,
                ..Page::default()
            };
            t.x_mm = 5.0;
            t.y_mm = 5.0;
            write_png_file(
                &p2,
                &[],
                &[],
                &[t],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                10.0,
                &dir.join(format!("{nom}.png")),
            )
            .unwrap();
        }
    }

    #[test]
    fn point_dans_polygone() {
        let tri = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_poly(&tri, 5.0, 5.0));
        assert!(!point_in_poly(&tri, 15.0, 5.0));
        assert!(!point_in_poly(&tri, -1.0, -1.0));
    }
}
