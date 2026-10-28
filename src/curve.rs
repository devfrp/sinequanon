//! Courbes : fonctions y=f(x), paramétriques, polaires, puis échantillonnage.

use crate::expr::{Node, VarSet};
use crate::grid::Page;
use serde::{Deserialize, Serialize};

/// Style de trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    Continu,
    Tirets,
    Pointilles,
}

/// Nature d'une courbe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CurveKind {
    Fonction {
        expr: String,
    },
    Parametree {
        xt: String,
        yt: String,
        tmin: f64,
        tmax: f64,
    },
    Polaire {
        r: String,
        tmin: f64,
        tmax: f64,
    },
}

/// Identifiant de nature, pour l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindId {
    Fonction,
    Parametree,
    Polaire,
}

impl CurveKind {
    pub fn kind_id(&self) -> KindId {
        match self {
            CurveKind::Fonction { .. } => KindId::Fonction,
            CurveKind::Parametree { .. } => KindId::Parametree,
            CurveKind::Polaire { .. } => KindId::Polaire,
        }
    }

    /// Convertit vers une autre nature en conservant les expressions utiles.
    pub fn convert(&self, to: KindId) -> CurveKind {
        if self.kind_id() == to {
            return self.clone();
        }
        let t = (-2.0 * std::f64::consts::PI, 2.0 * std::f64::consts::PI);
        match (self, to) {
            (_, KindId::Fonction) => CurveKind::Fonction {
                expr: "sin(x)".into(),
            },
            (CurveKind::Fonction { expr }, KindId::Polaire) => CurveKind::Polaire {
                r: expr.clone(),
                tmin: t.0,
                tmax: t.1,
            },
            (_, KindId::Polaire) => CurveKind::Polaire {
                r: "1+cos(theta)".into(),
                tmin: t.0,
                tmax: t.1,
            },
            (CurveKind::Fonction { expr }, KindId::Parametree) => CurveKind::Parametree {
                xt: "t".into(),
                yt: expr.clone().replace('x', "t"),
                tmin: -10.0,
                tmax: 10.0,
            },
            (_, KindId::Parametree) => CurveKind::Parametree {
                xt: "cos(t)".into(),
                yt: "sin(t)".into(),
                tmin: t.0,
                tmax: t.1,
            },
        }
    }
}

/// Une courbe dessinable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub kind: CurveKind,
    pub color: [u8; 3],
    pub thickness: f32,
    pub style: LineStyle,
    pub visible: bool,
}

impl Curve {
    pub fn fonction(expr: impl Into<String>, color: [u8; 3]) -> Self {
        Self {
            kind: CurveKind::Fonction { expr: expr.into() },
            color,
            thickness: 2.0,
            style: LineStyle::Continu,
            visible: true,
        }
    }

    pub fn with_kind(kind: CurveKind, color: [u8; 3]) -> Self {
        Self {
            kind,
            color,
            thickness: 2.0,
            style: LineStyle::Continu,
            visible: true,
        }
    }

    /// Libellé affiché dans la liste.
    pub fn label(&self) -> String {
        match &self.kind {
            CurveKind::Fonction { expr } => format!("y = {expr}"),
            CurveKind::Parametree { xt, yt, .. } => format!("x(t)={xt} ; y(t)={yt}"),
            CurveKind::Polaire { r, .. } => format!("r(θ) = {r}"),
        }
    }
}

/// Palette par défaut (10 couleurs type Sine qua non).
pub const PALETTE: [[u8; 3]; 10] = [
    [0, 0, 204],
    [204, 0, 0],
    [0, 136, 0],
    [170, 0, 170],
    [0, 153, 204],
    [204, 102, 0],
    [102, 0, 204],
    [0, 153, 102],
    [204, 0, 102],
    [51, 51, 51],
];

/// Polyligne en coordonnées page (mm).
pub type Poly = Vec<(f64, f64)>;

const LIMITE_MM: f64 = 1e6;

fn clamp_mm(v: f64) -> f64 {
    v.clamp(-LIMITE_MM, LIMITE_MM)
}

fn eval_node(node: &Node, vars: &VarSet, unknown: &mut Vec<String>) -> f64 {
    node.eval(vars, unknown)
}

/// Échantillonne la courbe en polylignes (mm). Coupe les segments aux
/// asymptotes et hors domaine. `n_fn` : colonnes pour une fonction,
/// `n_param` : points pour paramétrique/polaire.
pub fn sample_curve(
    curve: &Curve,
    page: &Page,
    n_fn: usize,
    n_param: usize,
) -> Result<Vec<Poly>, String> {
    let n_fn = n_fn.max(32);
    let n_param = n_param.max(64);
    let mut unknown = Vec::new();
    match &curve.kind {
        CurveKind::Fonction { expr } => {
            let ast: Node = crate::expr::parse(expr)?;
            ast.validate_vars(&["x"])?;
            let (x0, x1) = page.x_range();
            let l0 = if page.log_x {
                x0.max(1e-300).log10()
            } else {
                0.0
            };
            let l1 = if page.log_x {
                x1.max(1e-300).log10()
            } else {
                0.0
            };
            let mut polys: Vec<Poly> = Vec::new();
            let mut cur: Poly = Vec::new();
            let mut prev_mm: Option<(f64, f64)> = None;
            for i in 0..=n_fn {
                let t = i as f64 / n_fn as f64;
                let x = if page.log_x {
                    10f64.powf(l0 + (l1 - l0) * t)
                } else {
                    x0 + (x1 - x0) * t
                };
                let y = eval_node(
                    &ast,
                    &VarSet {
                        x: Some(x),
                        ..Default::default()
                    },
                    &mut unknown,
                );
                let mx = page.mm_x(x);
                let my = page.mm_y(y);
                let ok = mx.is_finite() && my.is_finite() && my.abs() <= LIMITE_MM;
                match (ok, prev_mm) {
                    (true, Some(pm)) => {
                        // coupure d'asymptote : saut vertical brutal
                        if (my - pm.1).abs() > 1.5 * page.height_mm {
                            if cur.len() >= 2 {
                                polys.push(std::mem::take(&mut cur));
                            } else {
                                cur.clear();
                            }
                        }
                        cur.push((clamp_mm(mx), clamp_mm(my)));
                    }
                    (true, None) => cur.push((clamp_mm(mx), clamp_mm(my))),
                    (false, _) => {
                        if cur.len() >= 2 {
                            polys.push(std::mem::take(&mut cur));
                        } else {
                            cur.clear();
                        }
                    }
                }
                prev_mm = ok.then_some((mx, my));
            }
            if cur.len() >= 2 {
                polys.push(cur);
            }
            if !unknown.is_empty() {
                return Err(format!("identifiant inconnu : {}", unknown[0]));
            }
            Ok(polys)
        }
        CurveKind::Parametree { xt, yt, tmin, tmax } => {
            let ax: Node = crate::expr::parse(xt)?;
            ax.validate_vars(&["t"])?;
            let ay: Node = crate::expr::parse(yt)?;
            ay.validate_vars(&["t"])?;
            let (a, b) = if tmin <= tmax {
                (*tmin, *tmax)
            } else {
                (*tmax, *tmin)
            };
            let mut polys: Vec<Poly> = Vec::new();
            let mut cur: Poly = Vec::new();
            for i in 0..=n_param {
                let t = a + (b - a) * i as f64 / n_param as f64;
                let vars = VarSet {
                    t: Some(t),
                    ..Default::default()
                };
                let x = eval_node(&ax, &vars, &mut unknown);
                let y = eval_node(&ay, &vars, &mut unknown);
                let mx = page.mm_x(x);
                let my = page.mm_y(y);
                if mx.is_finite()
                    && my.is_finite()
                    && my.abs() <= LIMITE_MM
                    && mx.abs() <= LIMITE_MM
                {
                    // coupure si saut brutal (point isolé / discontinuité)
                    if let Some(last) = cur.last() {
                        if ((my - last.1).abs() > 1.5 * page.height_mm)
                            || ((mx - last.0).abs() > 1.5 * page.width_mm)
                        {
                            if cur.len() >= 2 {
                                polys.push(std::mem::take(&mut cur));
                            } else {
                                cur.clear();
                            }
                        }
                    }
                    cur.push((clamp_mm(mx), clamp_mm(my)));
                } else if cur.len() >= 2 {
                    polys.push(std::mem::take(&mut cur));
                } else {
                    cur.clear();
                }
            }
            if cur.len() >= 2 {
                polys.push(cur);
            }
            if !unknown.is_empty() {
                return Err(format!("identifiant inconnu : {}", unknown[0]));
            }
            Ok(polys)
        }
        CurveKind::Polaire { r, tmin, tmax } => {
            let ast: Node = crate::expr::parse(r)?;
            ast.validate_vars(&["theta", "t"])?;
            let (a, b) = if tmin <= tmax {
                (*tmin, *tmax)
            } else {
                (*tmax, *tmin)
            };
            let mut polys: Vec<Poly> = Vec::new();
            let mut cur: Poly = Vec::new();
            for i in 0..=n_param {
                let th = a + (b - a) * i as f64 / n_param as f64;
                let vars = VarSet {
                    theta: Some(th),
                    t: Some(th),
                    ..Default::default()
                };
                let rad = eval_node(&ast, &vars, &mut unknown);
                let (x, y) = (rad * th.cos(), rad * th.sin());
                let mx = page.mm_x(x);
                let my = page.mm_y(y);
                if mx.is_finite()
                    && my.is_finite()
                    && my.abs() <= LIMITE_MM
                    && mx.abs() <= LIMITE_MM
                {
                    cur.push((clamp_mm(mx), clamp_mm(my)));
                } else if cur.len() >= 2 {
                    polys.push(std::mem::take(&mut cur));
                } else {
                    cur.clear();
                }
            }
            if cur.len() >= 2 {
                polys.push(cur);
            }
            if !unknown.is_empty() {
                return Err(format!("identifiant inconnu : {}", unknown[0]));
            }
            Ok(polys)
        }
    }
}

/// Découpe les polylignes selon le style de trait (tirets/pointillés).
pub fn apply_style(polys: Vec<Poly>, style: LineStyle) -> Vec<Poly> {
    match style {
        LineStyle::Continu => polys,
        LineStyle::Tirets => split_dashes(polys, 2.6, 1.6),
        LineStyle::Pointilles => split_dashes(polys, 0.45, 1.4),
    }
}

fn split_dashes(polys: Vec<Poly>, on: f64, off: f64) -> Vec<Poly> {
    let mut out: Vec<Poly> = Vec::new();
    for poly in polys {
        let mut cur: Poly = Vec::new();
        let mut drawing = true;
        let mut remaining = on;
        let mut prev: Option<(f64, f64)> = None;
        for &(x, y) in &poly {
            let Some((px, py)) = prev else {
                prev = Some((x, y));
                continue;
            };
            let seg = ((x - px).powi(2) + (y - py).powi(2)).sqrt();
            if seg < 1e-12 {
                prev = Some((x, y));
                continue;
            }
            let (mut sx, mut sy) = (px, py);
            let mut d = seg;
            while d > 1e-9 {
                let step = d.min(remaining);
                let t = step / d;
                let ex = sx + (x - sx) * t;
                let ey = sy + (y - sy) * t;
                if drawing {
                    if cur.is_empty() {
                        cur.push((sx, sy));
                    }
                    cur.push((ex, ey));
                }
                d -= step;
                remaining -= step;
                sx = ex;
                sy = ey;
                if remaining <= 1e-9 {
                    if drawing && cur.len() >= 2 {
                        out.push(std::mem::take(&mut cur));
                    }
                    drawing = !drawing;
                    remaining = if drawing { on } else { off };
                }
            }
            prev = Some((sx, sy));
        }
        if drawing && cur.len() >= 2 {
            out.push(cur);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        Page::default()
    }

    #[test]
    fn sinus_complet() {
        let c = Curve::fonction("sin(x)", PALETTE[0]);
        let polys = sample_curve(&c, &page(), 400, 400).unwrap();
        assert!(!polys.is_empty());
        for poly in &polys {
            for &(mx, my) in poly {
                assert!(mx.is_finite() && my.is_finite());
                assert!(my.abs() <= 1e6);
            }
        }
    }

    #[test]
    fn tangente_coupee_a_l_asymptote() {
        let c = Curve::fonction("tan(x)", PALETTE[0]);
        let polys = sample_curve(&c, &page(), 800, 400).unwrap();
        // au moins deux branches séparées autour de pi/2
        assert!(polys.len() >= 2, "branches = {}", polys.len());
    }

    #[test]
    fn expression_invalide() {
        let c = Curve::fonction("foo(x)", PALETTE[0]);
        assert!(sample_curve(&c, &page(), 100, 100).is_err());
        let c = Curve::fonction("t+1", PALETTE[0]);
        assert!(sample_curve(&c, &page(), 100, 100).is_err());
    }

    #[test]
    fn parametree_cercle() {
        let c = Curve::with_kind(
            CurveKind::Parametree {
                xt: "cos(t)".into(),
                yt: "sin(t)".into(),
                tmin: 0.0,
                tmax: 2.0 * std::f64::consts::PI,
            },
            PALETTE[1],
        );
        let polys = sample_curve(&c, &page(), 100, 400).unwrap();
        assert_eq!(polys.len(), 1);
        // le cercle de rayon 1 fait 40 mm de large (unité 20 mm) ; la page
        // 170×120 mm le contient entièrement
        let xs: Vec<f64> = polys[0].iter().map(|p| p.0).collect();
        let w = xs.iter().cloned().fold(f64::NAN, f64::max)
            - xs.iter().cloned().fold(f64::NAN, f64::min);
        assert!((w - 40.0).abs() < 2.0, "largeur = {w}");
    }

    #[test]
    fn polaire_cardioide() {
        let c = Curve::with_kind(
            CurveKind::Polaire {
                r: "1+cos(theta)".into(),
                tmin: 0.0,
                tmax: 2.0 * std::f64::consts::PI,
            },
            PALETTE[2],
        );
        let polys = sample_curve(&c, &page(), 100, 1000).unwrap();
        assert_eq!(polys.len(), 1);
    }

    #[test]
    fn hors_domaine_coupe() {
        let c = Curve::fonction("sqrt(x)", PALETTE[0]);
        // page centrée en x = -2 : la moitié gauche (x < 0) est hors domaine
        let p = Page { cx: -2.0, ..page() };
        let polys = sample_curve(&c, &p, 400, 100).unwrap();
        assert!(!polys.is_empty(), "au moins une branche");
        // tout point tracé est à droite du domaine valide
        let x_min_mm = p.mm_x(0.0);
        for poly in &polys {
            for &(mx, _) in poly {
                assert!(mx >= x_min_mm - 1e-6, "mx={mx} seuil={x_min_mm}");
            }
        }
    }

    #[test]
    fn tirets_produisent_des_segments() {
        let c = Curve::fonction("x", PALETTE[0]);
        let polys = sample_curve(&c, &page(), 400, 100).unwrap();
        let dashed = apply_style(polys, LineStyle::Tirets);
        assert!(dashed.len() > 3, "segments = {}", dashed.len());
        let dotted = apply_style(vec![vec![(0.0, 0.0), (100.0, 0.0)]], LineStyle::Pointilles);
        assert!(dotted.len() > 10);
    }
}
