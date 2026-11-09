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
    /// courbe de la dérivée f′ (man. p. 19) ; f saisie in extenso
    Derivee {
        expr: String,
    },
    /// primitive F passant par (x0 ; y0) (man. p. 19)
    Primitive {
        expr: String,
        x0: f64,
        y0: f64,
    },
    /// courbe définie point par point (man. p. 22-24) : jusqu'à 20
    /// points, pente saisie (vide = auto), lissage Catmull-Rom
    Ppk {
        pts: Vec<crate::ppk::PpkPt>,
        lissage: f32,
    },
}

/// Identifiant de nature, pour l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindId {
    Fonction,
    Parametree,
    Polaire,
    Derivee,
    Primitive,
    Ppk,
}

impl CurveKind {
    pub fn kind_id(&self) -> KindId {
        match self {
            CurveKind::Fonction { .. } => KindId::Fonction,
            CurveKind::Parametree { .. } => KindId::Parametree,
            CurveKind::Polaire { .. } => KindId::Polaire,
            CurveKind::Derivee { .. } => KindId::Derivee,
            CurveKind::Primitive { .. } => KindId::Primitive,
            CurveKind::Ppk { .. } => KindId::Ppk,
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
            (_, KindId::Derivee) => CurveKind::Derivee { expr: "x^2".into() },
            (_, KindId::Primitive) => CurveKind::Primitive {
                expr: "2*x".into(),
                x0: 0.0,
                y0: 0.0,
            },
            (_, KindId::Ppk) => CurveKind::Ppk {
                pts: vec![
                    crate::ppk::PpkPt::nouveau(-2.0, 1.0),
                    crate::ppk::PpkPt::nouveau(1.0, -1.0),
                    crate::ppk::PpkPt::nouveau(3.0, 2.0),
                ],
                lissage: 50.0,
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
            CurveKind::Derivee { expr } => format!("f′ : f(x)={expr}"),
            CurveKind::Primitive { expr, x0, y0 } => {
                format!("F({x0:.2})={y0:.2} : f(x)={expr}")
            }
            CurveKind::Ppk { pts, .. } => {
                format!("point par point : {} pts", pts.len())
            }
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

/// Évalue une expression de fonction en un x donné (solveur « Calculs »).
pub fn eval_fonction(expr: &str, x: f64) -> Result<f64, String> {
    let ast = crate::expr::parse(expr)?;
    ast.validate_vars(&["x"])?;
    let mut inconnues = Vec::new();
    Ok(eval_node(
        &ast,
        &VarSet {
            x: Some(x),
            ..Default::default()
        },
        &mut inconnues,
    ))
}

/// Construit les polylignes y=f(x) sur le domaine de la page, en
/// coupant aux asymptotes et hors domaine (mutualisé : Fonction,
/// Dérivée, Primitive).
fn courbe_fonction(page: &Page, n: usize, mut f: impl FnMut(f64) -> f64) -> Vec<Poly> {
    let n = n.max(32);
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
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let x = if page.log_x {
            10f64.powf(l0 + (l1 - l0) * t)
        } else {
            x0 + (x1 - x0) * t
        };
        let y = f(x);
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
    polys
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
            let mut unknown = Vec::new();
            let v = |x: f64| -> f64 {
                eval_node(
                    &ast,
                    &VarSet {
                        x: Some(x),
                        ..Default::default()
                    },
                    &mut unknown,
                )
            };
            let polys = courbe_fonction(page, n_fn, v);
            if !unknown.is_empty() {
                return Err(format!("identifiant inconnu : {}", unknown[0]));
            }
            Ok(polys)
        }
        CurveKind::Derivee { expr } => {
            // nombre dérivé par différence symétrique (man. p. 19)
            let ast: Node = crate::expr::parse(expr)?;
            ast.validate_vars(&["x"])?;
            let (x0, x1) = page.x_range();
            let pas = ((x1 - x0) / (n_fn.max(64) as f64 * 8.0)).abs().max(1e-9);
            let mut unknown = Vec::new();
            let mut v = |x: f64| -> f64 {
                eval_node(
                    &ast,
                    &VarSet {
                        x: Some(x),
                        ..Default::default()
                    },
                    &mut unknown,
                )
            };
            let polys = courbe_fonction(page, n_fn, |x| {
                let a = v(x - pas);
                let b = v(x + pas);
                if a.is_finite() && b.is_finite() {
                    (b - a) / (2.0 * pas)
                } else {
                    f64::NAN
                }
            });
            if !unknown.is_empty() {
                return Err(format!("identifiant inconnu : {}", unknown[0]));
            }
            Ok(polys)
        }
        CurveKind::Primitive { expr, x0, y0 } => {
            // F(x) = y0 + S(x) − S(x0), S = ∫ depuis le bord gauche :
            // Simpson composé sur une grille affine double (man. p. 19)
            let ast: Node = crate::expr::parse(expr)?;
            ast.validate_vars(&["x"])?;
            let (xl, xr) = page.x_range();
            let n = n_fn.max(256);
            let dx = (xr - xl) / n as f64;
            let mut unknown = Vec::new();
            let mut v = |x: f64| -> f64 {
                eval_node(
                    &ast,
                    &VarSet {
                        x: Some(x),
                        ..Default::default()
                    },
                    &mut unknown,
                )
            };
            // grille fine : 2n+1 points avec valeurs de f
            let mut fs = Vec::with_capacity(2 * n + 1);
            for i in 0..=(2 * n) {
                let x = xl + (xr - xl) * i as f64 / (2 * n) as f64;
                let y = v(x);
                fs.push(if y.is_finite() && y.abs() <= 1e12 {
                    y
                } else {
                    f64::NAN
                });
            }
            // cumuls Simpson (Simpson complet sur les cellules doubles,
            // trapèze sur les cellules simples impaires)
            let dx2 = dx / 2.0;
            let mut ss = vec![f64::NAN; 2 * n + 1];
            ss[0] = 0.0;
            for k in 1..=n {
                let (a, m, b) = (fs[2 * k - 2], fs[2 * k - 1], fs[2 * k]);
                if a.is_finite() && m.is_finite() && b.is_finite() {
                    ss[2 * k] = ss[2 * k - 2] + dx2 * (a + 4.0 * m + b) / 3.0;
                } else {
                    ss[2 * k] = ss[2 * k - 2]; // trou : pas d'accumulation
                }
                let s_mid = if a.is_finite() && m.is_finite() {
                    ss[2 * k - 2] + (a + m) * 0.5 * dx2
                } else if m.is_finite() && b.is_finite() {
                    ss[2 * k] - (m + b) * 0.5 * dx2
                } else {
                    f64::NAN
                };
                ss[2 * k - 1] = s_mid;
            }
            // S interpolé linéairement sur la grille fine
            let s_at = |x: f64| -> f64 {
                let t = ((x - xl) / (xr - xl)).clamp(0.0, 1.0);
                let h = t * (2.0 * n as f64);
                let i = (h as usize).min(2 * n - 1);
                let k = h - i as f64;
                let (a, b) = (ss[i], ss[i + 1]);
                if a.is_nan() || b.is_nan() {
                    // cellule contiguë à un trou : reprend le cumul sûr
                    let ok_gauche = |j: usize| -> f64 {
                        (0..=j)
                            .rev()
                            .find(|j| !ss[*j].is_nan())
                            .map(|j| ss[j])
                            .unwrap_or(0.0)
                    };
                    let ok_droite = |j: usize| -> f64 {
                        (j..=2 * n)
                            .find(|j| !ss[*j].is_nan())
                            .map(|j| ss[j])
                            .unwrap_or(0.0)
                    };
                    let (g, dfl) = (ok_gauche(i), ok_droite(i + 1));
                    g + (dfl - g) * k
                } else {
                    a + (b - a) * k
                }
            };
            let s0 = s_at(*x0);
            let polys = courbe_fonction(page, n_fn, |x| {
                if v(x).is_finite() {
                    *y0 + s_at(x) - s0
                } else {
                    f64::NAN
                }
            });
            if !unknown.is_empty() {
                return Err(format!("identifiant inconnu : {}", unknown[0]));
            }
            Ok(polys)
        }
        CurveKind::Ppk { pts, lissage } => {
            if pts.len() < 2 {
                return Err("courbe point par point : au moins 2 points requis".into());
            }
            Ok(crate::ppk::polys_mm(
                pts,
                *lissage as f64,
                |x| page.mm_x(x),
                |y| page.mm_y(y),
            ))
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

/// Valeur de y dessinée par les natures à abscisse x (fonction,
/// dérivée par différence symétrique, primitive par Simpson composé,
/// man. p. 19). None si indéfinie ou non finie.
pub fn y_nature(kind: &CurveKind, x: f64) -> Option<f64> {
    match kind {
        CurveKind::Fonction { expr } => eval_fonction(expr, x).ok(),
        CurveKind::Derivee { expr } => {
            let ast = crate::expr::parse(expr).ok()?;
            ast.validate_vars(&["x"]).ok()?;
            let h = 1e-6;
            let a = eval_node(
                &ast,
                &VarSet {
                    x: Some(x - h),
                    ..Default::default()
                },
                &mut Vec::new(),
            );
            let b = eval_node(
                &ast,
                &VarSet {
                    x: Some(x + h),
                    ..Default::default()
                },
                &mut Vec::new(),
            );
            let d = (b - a) / (2.0 * h);
            d.is_finite().then_some(d)
        }
        CurveKind::Primitive { expr, x0, y0 } => {
            let ast = crate::expr::parse(expr).ok()?;
            ast.validate_vars(&["x"]).ok()?;
            let v = |t: f64| {
                eval_node(
                    &ast,
                    &VarSet {
                        x: Some(t),
                        ..Default::default()
                    },
                    &mut Vec::new(),
                )
            };
            let (a, b) = if *x0 <= x { (*x0, x) } else { (x, *x0) };
            if !(a.is_finite() && b.is_finite()) {
                return None;
            }
            let n = 256;
            let h = (b - a) / n as f64;
            let mut s = 0.0;
            for i in 0..n {
                let fl = v(a + i as f64 * h);
                let fm = v(a + (i as f64 + 0.5) * h);
                let fr = v(a + (i as f64 + 1.0) * h);
                if !(fl.is_finite() && fm.is_finite() && fr.is_finite()) {
                    return None;
                }
                s += (fl + 4.0 * fm + fr) * h / 6.0;
            }
            let f = if *x0 <= x { *y0 + s } else { *y0 - s };
            f.is_finite().then_some(f)
        }
        CurveKind::Ppk { pts, lissage } => {
            // y au paramètre « x » : le long des arcs, premier croisement
            if pts.len() < 2 {
                return None;
            }
            for arc in crate::ppk::arcs(pts, *lissage as f64) {
                for seg in arc.windows(2) {
                    let ((x0, y0), (x1, y1)) = (seg[0], seg[1]);
                    if (x1 - x0).abs() < 1e-12 {
                        continue;
                    }
                    if (x >= x0.min(x1) - 1e-12) && (x <= x0.max(x1) + 1e-12) {
                        let t = (x - x0) / (x1 - x0);
                        let y = y0 + (y1 - y0) * t;
                        if y.is_finite() {
                            return Some(y);
                        }
                    }
                }
            }
            None
        }
        _ => None,
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

    // ----- natures v1.6 : dérivée et primitive (man. p. 19) -----

    #[test]
    fn derivee_de_carree() {
        let c = Curve::with_kind(CurveKind::Derivee { expr: "x^2".into() }, PALETTE[1]);
        let p = page();
        let polys = sample_curve(&c, &p, 400, 100).unwrap();
        assert!(!polys.is_empty());
        // chaque point de la polyligne est bien sur la droite y = 2x
        for poly in &polys {
            for &(mx, my) in poly {
                let (x, y) = (p.x_at_mm(mx), p.y_at_mm(my));
                assert!(
                    (y - 2.0 * x).abs() < 0.02,
                    "x={x} y={y} attendu {}",
                    2.0 * x
                );
            }
        }
    }

    #[test]
    fn primitive_de_2x_par_0_3() {
        // F(x) = x² + 3 si F(0) = 3
        let c = Curve::with_kind(
            CurveKind::Primitive {
                expr: "2*x".into(),
                x0: 0.0,
                y0: 3.0,
            },
            PALETTE[1],
        );
        let p = page();
        let polys = sample_curve(&c, &p, 400, 100).unwrap();
        assert!(!polys.is_empty());
        for poly in &polys {
            for &(mx, my) in poly {
                let (x, y) = (p.x_at_mm(mx), p.y_at_mm(my));
                let attendu = x * x + 3.0;
                assert!((y - attendu).abs() < 0.02, "x={x} y={y} attendu {attendu}");
            }
        }
    }

    #[test]
    fn primitive_racine_coupee() {
        // √x est indéfinie pour x < 0 : F(x) = x^(3/2)/1.5 pour x ≥ 0
        let c = Curve::with_kind(
            CurveKind::Primitive {
                expr: "sqrt(x)".into(),
                x0: 0.0,
                y0: 0.0,
            },
            PALETTE[1],
        );
        let p = page();
        let polys = sample_curve(&c, &p, 400, 100).unwrap();
        assert!(!polys.is_empty());
        let x0_mm = p.mm_x(0.0);
        for poly in &polys {
            for &(mx, my) in poly {
                assert!(mx >= x0_mm - 1e-6, "mx={mx} seuil={x0_mm}");
                let (x, y) = (p.x_at_mm(mx), p.y_at_mm(my));
                if x > 0.05 {
                    let attendu = x.powf(1.5) / 1.5;
                    assert!((y - attendu).abs() < 0.05, "x={x} y={y} attendu {attendu}");
                }
            }
        }
    }

    #[test]
    fn y_nature_derivee_et_primitive() {
        let der = CurveKind::Derivee {
            expr: "sin(x)".into(),
        };
        let y0 = y_nature(&der, std::f64::consts::PI / 2.0).unwrap();
        // f′ = cos, cos(π/2) = 0
        assert!((y0 - 0.0).abs() < 1e-3);
        let pri = CurveKind::Primitive {
            expr: "2*x".into(),
            x0: 0.0,
            y0: 3.0,
        };
        let y1 = y_nature(&pri, 2.0).unwrap();
        assert!((y1 - 7.0).abs() < 1e-6, "y1={y1}");
        // x0 ≠ 0 : intégrale de x0 à x
        let pri2 = CurveKind::Primitive {
            expr: "2*x".into(),
            x0: 1.0,
            y0: 0.0,
        };
        let y2 = y_nature(&pri2, 3.0).unwrap();
        assert!((y2 - 8.0).abs() < 1e-6, "y2={y2}");
        // hors domaine
        let pri3 = CurveKind::Primitive {
            expr: "sqrt(x)".into(),
            x0: 0.0,
            y0: 0.0,
        };
        assert!(y_nature(&pri3, -1.0).is_none());
    }
}
