//! Annotations du tracé : points sur courbes, tangentes (dérivée
//! numérique) et intégrales hachurées (somme des trapèzes) — l'équivalent
//! des menus « Point » et « Intégrales » du Sine qua non original.

use crate::curve::{Curve, CurveKind, Poly};
use crate::expr::{Node, VarSet};
use crate::grid::Page;
use serde::{Deserialize, Serialize};

/// Point placé sur une courbe. Pour une fonction, `x` est l'abscisse ;
/// pour une courbe paramétrée ou polaire, c'est le paramètre t (ou θ).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointAnnot {
    pub curve: usize,
    pub x: f64,
}

/// Tangente à une courbe en un point (fonctions uniquement).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TangenteAnnot {
    pub curve: usize,
    pub x: f64,
}

/// Intégrale hachurée : aire entre la courbe et l'axe des abscisses
/// sur [a, b] (fonctions uniquement).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IntegraleAnnot {
    pub curve: usize,
    pub a: f64,
    pub b: f64,
}

/// Annotation prête à dessiner (coordonnées page en mm).
#[derive(Debug, Clone)]
pub enum AnnotDraw {
    Point {
        mm: (f64, f64),
        label: String,
        color: [u8; 3],
    },
    Tangente {
        /// segment (mm)
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        /// point de tangence
        mm: (f64, f64),
        label: String,
        color: [u8; 3],
    },
    Integrale {
        /// contour fermé de l'aire (courbe + fermeture sur l'axe)
        path: Poly,
        /// valeur approchée, formatée (aire algébrique)
        label: String,
        label_mm: (f64, f64),
        color: [u8; 3],
    },
}

/// Évalue une courbe de type fonction en x.
fn eval_fn(ast: &Node, x: f64) -> f64 {
    ast.eval(
        &VarSet {
            x: Some(x),
            ..Default::default()
        },
        &mut Vec::new(),
    )
}

/// Dérivée numérique (différences centrées).
fn derivative(ast: &Node, x: f64) -> f64 {
    let h = 1e-5 * (1.0 + x.abs());
    (eval_fn(ast, x + h) - eval_fn(ast, x - h)) / (2.0 * h)
}

/// Aire algébrique entre la courbe et l'axe x sur [a, b] (trapèzes).
fn integrale(ast: &Node, a: f64, b: f64) -> f64 {
    let n = 2000usize;
    let h = (b - a) / n as f64;
    let mut s = (eval_fn(ast, a) + eval_fn(ast, b)) / 2.0;
    for i in 1..n {
        s += eval_fn(ast, a + i as f64 * h);
    }
    s * h
}

/// Segments de la trame de hachures (droites à 45°, `s = x + y` constant)
/// intérieurs au polygone, espacés de `period` mm. Utilisé par le rendu
/// écran (l'export SVG utilise une `<pattern>`, le PNG un balayage).
pub fn hatch_segments(path: &Poly, period: f64) -> Vec<((f64, f64), (f64, f64))> {
    let mut out = Vec::new();
    if path.len() < 2 || period <= 0.0 {
        return out;
    }
    let mut smin = f64::MAX;
    let mut smax = f64::MIN;
    for &(x, y) in path {
        smin = smin.min(x + y);
        smax = smax.max(x + y);
    }
    let mut level = smin + period;
    while level < smax {
        // intersections du niveau avec chaque arête
        let mut hits: Vec<f64> = Vec::new();
        let mut pts: Vec<(f64, f64)> = Vec::new();
        for w in path.windows(2) {
            let (p, q) = (w[0], w[1]);
            let (s1, s2) = (p.0 + p.1, q.0 + q.1);
            if (s1 - level) * (s2 - level) < 0.0 {
                let t = (level - s1) / (s2 - s1);
                hits.push(p.0 + t * (q.0 - p.0));
                pts.push((p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)));
            }
        }
        // points triés par abscisse : les paires consécutives délimitent
        // les portions intérieures
        let mut order: Vec<usize> = (0..pts.len()).collect();
        order.sort_by(|&a, &b| hits[a].total_cmp(&hits[b]));
        for chunk in order.chunks(2) {
            if chunk.len() == 2 {
                out.push((pts[chunk[0]], pts[chunk[1]]));
            }
        }
        level += period;
    }
    out
}

/// Valeur (x, y) d'un point sur la courbe visée, ou None (indice hors
/// bornes, expression invalide, valeur non finie).
pub fn point_value(curves: &[Curve], p: &PointAnnot) -> Option<(f64, f64)> {
    let c = curves.get(p.curve)?;
    let (ux, uy) = match &c.kind {
        CurveKind::Fonction { expr } => {
            let ast = crate::expr::parse(expr).ok()?;
            (p.x, eval_fn(&ast, p.x))
        }
        CurveKind::Parametree { xt, yt, .. } => {
            let (ax, ay) = (crate::expr::parse(xt).ok()?, crate::expr::parse(yt).ok()?);
            let vars = VarSet {
                t: Some(p.x),
                ..Default::default()
            };
            let mut unk = Vec::new();
            (ax.eval(&vars, &mut unk), ay.eval(&vars, &mut unk))
        }
        CurveKind::Polaire { r, .. } => {
            let ar = crate::expr::parse(r).ok()?;
            let vars = VarSet {
                theta: Some(p.x),
                ..Default::default()
            };
            let mut unk = Vec::new();
            let rad = ar.eval(&vars, &mut unk);
            (rad * p.x.cos(), rad * p.x.sin())
        }
    };
    (ux.is_finite() && uy.is_finite()).then_some((ux, uy))
}

/// (pente, ordonnée à l'origine) de la tangente à une fonction, ou None.
pub fn tangente_value(curves: &[Curve], t: &TangenteAnnot) -> Option<(f64, f64)> {
    let c = curves.get(t.curve)?;
    let CurveKind::Fonction { expr } = &c.kind else {
        return None;
    };
    let ast = crate::expr::parse(expr).ok()?;
    let fa = eval_fn(&ast, t.x);
    if !fa.is_finite() {
        return None;
    }
    let d = derivative(&ast, t.x);
    (d.is_finite()).then_some((d, fa - d * t.x))
}

/// Valeur approchée de l'intégrale sur [a, b], ou None.
pub fn integrale_value(curves: &[Curve], it: &IntegraleAnnot) -> Option<f64> {
    let c = curves.get(it.curve)?;
    let CurveKind::Fonction { expr } = &c.kind else {
        return None;
    };
    if !it.a.is_finite() || !it.b.is_finite() || (it.a - it.b).abs() < 1e-12 {
        return None;
    }
    let ast = crate::expr::parse(expr).ok()?;
    let (lo, hi) = if it.a < it.b {
        (it.a, it.b)
    } else {
        (it.b, it.a)
    };
    let v = integrale(&ast, lo, hi);
    v.is_finite().then_some(v)
}

/// Construit la liste des annotations à dessiner. Les erreurs (indice de
/// courbe invalide, tangente/intégrale sur une courbe non fonction, hors
/// domaine, courbe masquée) sont ignorées : l'annotation est simplement
/// absente.
pub fn draw_list(
    page: &Page,
    curves: &[Curve],
    points: &[PointAnnot],
    tangentes: &[TangenteAnnot],
    integrales: &[IntegraleAnnot],
) -> Vec<AnnotDraw> {
    let mut out = Vec::new();

    for p in points {
        let Some(c) = curves.get(p.curve) else {
            continue;
        };
        if !c.visible {
            continue;
        }
        let Some((ux, uy)) = point_value(curves, p) else {
            continue;
        };
        let mm = (page.mm_x(ux), page.mm_y(uy));
        if !mm.0.is_finite() || !mm.1.is_finite() {
            continue;
        }
        out.push(AnnotDraw::Point {
            mm,
            label: format!("({:.3} ; {:.3})", ux, uy),
            color: c.color,
        });
    }

    for t in tangentes {
        let Some(c) = curves.get(t.curve) else {
            continue;
        };
        if !c.visible {
            continue;
        }
        let Some((d, b0)) = tangente_value(curves, t) else {
            continue;
        };
        let Some(fa) = point_value(
            curves,
            &PointAnnot {
                curve: t.curve,
                x: t.x,
            },
        )
        .map(|(_, y)| y) else {
            continue;
        };
        // demi-fenêtre : un quart de largeur visible
        let w = (page.x_range().1 - page.x_range().0).abs() / 8.0;
        let (x0, x1) = (t.x - w, t.x + w);
        let (y0, y1) = (fa + d * (x0 - t.x), fa + d * (x1 - t.x));
        let (m0, m1) = (
            (page.mm_x(x0), page.mm_y(y0)),
            (page.mm_x(x1), page.mm_y(y1)),
        );
        if !m0.0.is_finite() || !m0.1.is_finite() || !m1.0.is_finite() || !m1.1.is_finite() {
            continue;
        }
        let signe = if b0 >= 0.0 { "+" } else { "-" };
        let label = format!("y = {:.3}x {} {:.3}", d.abs(), signe, b0.abs());
        out.push(AnnotDraw::Tangente {
            x0: m0.0,
            y0: m0.1,
            x1: m1.0,
            y1: m1.1,
            mm: (page.mm_x(t.x), page.mm_y(fa)),
            label,
            color: c.color,
        });
    }

    for it in integrales {
        let Some(c) = curves.get(it.curve) else {
            continue;
        };
        if !c.visible {
            continue;
        }
        let Some(value) = integrale_value(curves, it) else {
            continue;
        };
        let (lo, hi) = if it.a < it.b {
            (it.a, it.b)
        } else {
            (it.b, it.a)
        };
        let n = 400usize;
        let y_axis = page.mm_y(0.0);
        if !y_axis.is_finite() {
            continue;
        }
        let mut path: Poly = Vec::with_capacity(n + 2);
        path.push((page.mm_x(lo), y_axis));
        // label placé au point le plus éloigné de l'axe (plus lisible)
        let mut label_mm = (page.mm_x((lo + hi) / 2.0), y_axis);
        let mut best_depth = -1.0f64;
        let mut ok = true;
        for i in 0..=n {
            let x = lo + (hi - lo) * i as f64 / n as f64;
            let Some((_, y)) = point_value(curves, &PointAnnot { curve: it.curve, x }) else {
                ok = false;
                break;
            };
            let (mx, my) = (page.mm_x(x), page.mm_y(y));
            if !mx.is_finite() || !my.is_finite() {
                ok = false;
                break;
            }
            let depth = (my - y_axis).abs();
            if depth > best_depth {
                best_depth = depth;
                label_mm = (mx, my);
            }
            path.push((mx, my));
        }
        if !ok {
            continue;
        }
        let last = *path.last().unwrap();
        path.push((last.0, y_axis));
        out.push(AnnotDraw::Integrale {
            path,
            label: format!("{:.4}", value),
            label_mm,
            color: c.color,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;

    #[test]
    fn point_sur_fonction() {
        let page = Page::default();
        let curves = vec![Curve::fonction("x^2", [0, 0, 204])];
        let out = draw_list(&page, &curves, &[PointAnnot { curve: 0, x: 2.0 }], &[], &[]);
        match &out[..] {
            [AnnotDraw::Point { mm, label, .. }] => {
                assert!((mm.0 - page.mm_x(2.0)).abs() < 1e-9);
                assert!((mm.1 - page.mm_y(4.0)).abs() < 1e-9);
                assert_eq!(label, "(2.000 ; 4.000)");
            }
            other => panic!("annotation inattendue : {other:?}"),
        }
    }

    #[test]
    fn tangente_en_x() {
        let page = Page::default();
        let curves = vec![Curve::fonction("x^2", [0, 0, 204])];
        let out = draw_list(
            &page,
            &curves,
            &[],
            &[TangenteAnnot { curve: 0, x: 2.0 }],
            &[],
        );
        match &out[..] {
            [AnnotDraw::Tangente { label, mm, .. }] => {
                // y = 4x - 4 en x = 2
                assert!(label.contains("4.000"), "label = {label}");
                assert!((mm.1 - page.mm_y(4.0)).abs() < 1e-6);
            }
            other => panic!("annotation inattendue : {other:?}"),
        }
    }

    #[test]
    fn integrale_de_x2() {
        let page = Page::default();
        let curves = vec![Curve::fonction("x^2", [0, 0, 204])];
        let out = draw_list(
            &page,
            &curves,
            &[],
            &[],
            &[IntegraleAnnot {
                curve: 0,
                a: 0.0,
                b: 2.0,
            }],
        );
        match &out[..] {
            [AnnotDraw::Integrale { label, path, .. }] => {
                // ∫₀² x² dx = 8/3 (label arrondi à 4 décimales)
                let v: f64 = label.parse().unwrap();
                assert!((v - 8.0 / 3.0).abs() < 5e-5, "label = {label}");
                // contour fermé : premier et dernier points sur l'axe
                assert!((path.first().unwrap().1 - page.mm_y(0.0)).abs() < 1e-9);
                assert!((path.last().unwrap().1 - page.mm_y(0.0)).abs() < 1e-9);
            }
            other => panic!("annotation inattendue : {other:?}"),
        }
    }

    #[test]
    fn valeurs_des_helpers() {
        let curves = vec![Curve::fonction("x^2", [0, 0, 204])];
        let y = point_value(&curves, &PointAnnot { curve: 0, x: 3.0 })
            .unwrap()
            .1;
        assert!((y - 9.0).abs() < 1e-9);
        let (d, b) = tangente_value(&curves, &TangenteAnnot { curve: 0, x: 2.0 }).unwrap();
        assert!((d - 4.0).abs() < 1e-4);
        assert!((b + 4.0).abs() < 1e-3);
        let v = integrale_value(
            &curves,
            &IntegraleAnnot {
                curve: 0,
                a: -1.0,
                b: 1.0,
            },
        )
        .unwrap();
        assert!((v - 2.0 / 3.0).abs() < 1e-6);
        // bornes inversées : même valeur
        let v2 = integrale_value(
            &curves,
            &IntegraleAnnot {
                curve: 0,
                a: 1.0,
                b: -1.0,
            },
        )
        .unwrap();
        assert!((v2 - v).abs() < 1e-12);
    }

    #[test]
    fn hachures_dans_le_polygone() {
        // carré unité 10 × 10 : chaque droite à 45° qui le traverse donne un
        // segment intérieur, d'extrémités sur le contour
        let carre = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ];
        let segs = hatch_segments(&carre, 2.0);
        assert!(!segs.is_empty());
        for ((x0, y0), (x1, y1)) in &segs {
            for &(x, y) in &[(*x0, *y0), (*x1, *y1)] {
                assert!(
                    (0.0..=10.0).contains(&x) && (0.0..=10.0).contains(&y),
                    "extrémité hors contour : ({x}, {y})"
                );
            }
            assert!((x0 + y0 - x1 - y1).abs() < 1e-9, "segment non diagonal");
        }
        // polygone dégénéré : rien
        assert!(hatch_segments(&Vec::new(), 2.0).is_empty());
    }

    #[test]
    fn erreurs_ignorees() {
        let page = Page::default();
        let curves = vec![Curve::fonction("x^2", [0, 0, 204])];
        // indice hors bornes, tangente sur courbe inexistante, domaine vide
        let out = draw_list(
            &page,
            &curves,
            &[PointAnnot { curve: 5, x: 1.0 }],
            &[TangenteAnnot { curve: 3, x: 1.0 }],
            &[IntegraleAnnot {
                curve: 0,
                a: f64::NAN,
                b: 2.0,
            }],
        );
        assert!(out.is_empty());
    }

    #[test]
    fn courbe_masquee_sans_annotation() {
        let page = Page::default();
        let mut c = Curve::fonction("x^2", [0, 0, 204]);
        c.visible = false;
        let curves = vec![c];
        let out = draw_list(
            &page,
            &curves,
            &[PointAnnot { curve: 0, x: 1.0 }],
            &[TangenteAnnot { curve: 0, x: 1.0 }],
            &[IntegraleAnnot {
                curve: 0,
                a: 0.0,
                b: 1.0,
            }],
        );
        assert!(out.is_empty());
        // mais les helpers de valeur restent utilisables (panneau)
        assert!(point_value(&curves, &PointAnnot { curve: 0, x: 1.0 }).is_some());
    }

    #[test]
    fn point_sur_polaire() {
        let page = Page::default();
        let curves = vec![Curve::with_kind(
            CurveKind::Polaire {
                r: "1".into(),
                tmin: 0.0,
                tmax: 2.0 * std::f64::consts::PI,
            },
            [0, 0, 204],
        )];
        let out = draw_list(
            &page,
            &curves,
            &[PointAnnot {
                curve: 0,
                x: std::f64::consts::FRAC_PI_2,
            }],
            &[],
            &[],
        );
        match &out[..] {
            [AnnotDraw::Point { mm, .. }] => {
                assert!((mm.0 - page.mm_x(0.0)).abs() < 1e-9);
                assert!((mm.1 - page.mm_y(1.0)).abs() < 1e-9);
            }
            other => panic!("annotation inattendue : {other:?}"),
        }
    }
}
