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
    /// famille de fonctions fp(x) (man. p. 26-27) : expression avec le
    /// paramètre p (multiplication implicite acceptée), variation de
    /// p par [pmin ; pmax] de pas « pas » (défaut [−5 ; 5] pas 1).
    /// Options : couleur différente par valeur de p (« choisie par
    /// l'ordinateur »), étiquettes des valeurs de p (abscisse
    /// d'affichage en p, position dessus/sur/dessous) ; les deux
    /// « progressions » ne concernent que l'aperçu GUI.
    #[serde(rename_all = "camelCase")]
    Famille {
        expr: String,
        #[serde(default = "def_pmin")]
        pmin: f64,
        #[serde(default = "def_pmax")]
        pmax: f64,
        #[serde(default = "def_pas")]
        pas: f64,
        #[serde(default)]
        palette: bool,
        #[serde(default)]
        valeurs: bool,
        #[serde(default = "def_x_etiq")]
        x_etiq: String,
        #[serde(default)]
        pos: PosP,
        #[serde(default)]
        progression: bool,
        #[serde(default)]
        progression_fine: bool,
    },
}

/// Position de l'étiquette « valeur de p » par rapport à la courbe
/// (man. p. 26) : au-dessus, sur la courbe ou en dessous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PosP {
    #[default]
    Dessus,
    Sur,
    Dessous,
}

/// Sortie d'échantillonnage d'une courbe : polylignes en mm page,
/// couleur par polyligne (vide = couleur unique de la courbe) et
/// étiquettes posées par une famille (valeurs de p).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trace {
    pub polys: Vec<Poly>,
    /// couleur par polyligne ; vide = couleur de la courbe partout
    pub couleurs: Vec<[u8; 3]>,
    pub etiquettes: Vec<EtiquetteC>,
}

/// Étiquette « valeur de p » d'une famille : position en mm page
/// (haut-gauche du texte, comme les textes statistiques), texte,
/// couleur.
#[derive(Debug, Clone, PartialEq)]
pub struct EtiquetteC {
    pub mm: (f64, f64),
    pub txt: String,
    pub couleur: [u8; 3],
}

fn def_pmin() -> f64 {
    -5.0
}

fn def_pmax() -> f64 {
    5.0
}

fn def_pas() -> f64 {
    1.0
}

fn def_x_etiq() -> String {
    "1".into()
}

impl Trace {
    /// Trace sans couleurs ni étiquettes (toutes les natures sauf la
    /// famille).
    pub fn simple(polys: Vec<Poly>) -> Self {
        Trace {
            polys,
            couleurs: Vec::new(),
            etiquettes: Vec::new(),
        }
    }
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
    Famille,
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
            CurveKind::Famille { .. } => KindId::Famille,
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
            // famille : une fonction y=f(x) devient p·f(x) (si p absent
            // de l'expression), sinon l'exemple du manuel
            (CurveKind::Fonction { expr }, KindId::Famille) => CurveKind::Famille {
                expr: if expr.contains('p') {
                    expr.clone()
                } else {
                    format!("p*({expr})")
                },
                pmin: def_pmin(),
                pmax: def_pmax(),
                pas: def_pas(),
                palette: false,
                valeurs: false,
                x_etiq: def_x_etiq(),
                pos: PosP::default(),
                progression: false,
                progression_fine: false,
            },
            (_, KindId::Famille) => CurveKind::Famille {
                expr: "p*sin(x)".into(),
                pmin: def_pmin(),
                pmax: def_pmax(),
                pas: def_pas(),
                palette: false,
                valeurs: false,
                x_etiq: def_x_etiq(),
                pos: PosP::default(),
                progression: false,
                progression_fine: false,
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
            CurveKind::Famille { expr, .. } => format!("fp(x) = {expr}"),
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
pub fn sample_trace(
    curve: &Curve,
    page: &Page,
    n_fn: usize,
    n_param: usize,
) -> Result<Trace, String> {
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
            Ok(Trace::simple(polys))
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
            Ok(Trace::simple(polys))
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
            Ok(Trace::simple(polys))
        }
        CurveKind::Ppk { pts, lissage } => {
            if pts.len() < 2 {
                return Err("courbe point par point : au moins 2 points requis".into());
            }
            Ok(Trace::simple(crate::ppk::polys_mm(
                pts,
                *lissage as f64,
                |x| page.mm_x(x),
                |y| page.mm_y(y),
            )))
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
            Ok(Trace::simple(polys))
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
            Ok(Trace::simple(polys))
        }
        CurveKind::Famille {
            expr,
            pmin,
            pmax,
            pas,
            palette,
            valeurs,
            x_etiq,
            pos,
            ..
        } => {
            // famille de fonctions fp(x) (man. p. 26-27) : une courbe
            // par valeur de p (coupures d'asymptotes par courbe), le
            // paramètre collé aux facteurs accepté (px² - 3px + 5)
            let ps = fam_p_values(*pmin, *pmax, *pas)?;
            let ast: Node = crate::expr::parse(expr)?;
            ast.validate_vars(&["x", "p"])?;
            // abscisse d'affichage des valeurs de p : expression en p
            // seule (man. p. 26), posée dessus/sur/dessous la courbe
            let axe: Option<Node> = if *valeurs {
                let a = crate::expr::parse(x_etiq)?;
                a.validate_vars(&["p"])?;
                Some(a)
            } else {
                None
            };
            let mut tr = Trace::default();
            for (k, &pv) in ps.iter().enumerate() {
                let auto = palette.then(|| couleur_palette(k, ps.len()));
                let mut unknown = Vec::new();
                {
                    // évaluation fp(x) pour ce p ; le double fermoir
                    // |x| v(x) garde v utilisable pour l'étiquette
                    let mut v = |x: f64| -> f64 {
                        eval_node(
                            &ast,
                            &VarSet {
                                x: Some(x),
                                p: Some(pv),
                                ..Default::default()
                            },
                            &mut unknown,
                        )
                    };
                    for poly in courbe_fonction(page, n_fn, &mut v) {
                        if let Some(c) = auto {
                            tr.couleurs.push(c);
                        }
                        tr.polys.push(poly);
                    }
                    // étiquette « valeur de p » sur la courbe de ce p
                    if *valeurs {
                        let mut unk2 = Vec::new();
                        let xe = eval_node(
                            axe.as_ref().expect("axe posé avec valeurs"),
                            &VarSet {
                                p: Some(pv),
                                ..Default::default()
                            },
                            &mut unk2,
                        );
                        let ye = v(xe);
                        if xe.is_finite() && ye.is_finite() {
                            let (mmx, mmy) = (page.mm_x(xe), page.mm_y(ye));
                            if mmx.is_finite() && mmy.is_finite() {
                                // texte de hauteur 2,6 mm : dessus = au
                                //-dessus du point de la courbe, sur =
                                // posé dessus, dessous = en dessous
                                let top = match pos {
                                    PosP::Dessus => mmy - 3.2,
                                    PosP::Sur => mmy - 1.3,
                                    PosP::Dessous => mmy + 0.9,
                                };
                                tr.etiquettes.push(EtiquetteC {
                                    mm: (mmx, top),
                                    txt: fmt_p(pv),
                                    couleur: auto.unwrap_or(curve.color),
                                });
                            }
                        }
                    }
                }
                if !unknown.is_empty() {
                    return Err(format!("identifiant inconnu : {}", unknown[0]));
                }
            }
            Ok(tr)
        }
    }
}

/// Polylignes seules, équivalent à `sample_trace(...).polys` (gardé
/// pour les tests, qui n'ont pas besoin des couleurs ni étiquettes).
#[cfg(test)]
pub fn sample_curve(
    curve: &Curve,
    page: &Page,
    n_fn: usize,
    n_param: usize,
) -> Result<Vec<Poly>, String> {
    sample_trace(curve, page, n_fn, n_param).map(|t| t.polys)
}

/// Valeurs du paramètre p : pmin + k·pas jusqu'à pmax inclus (man.
/// p. 26 : défaut [−5 ; 5] pas 1, soit 11 courbes). Erreur si le pas
/// n'est pas strictement positif ou si la famille dépasse 256 courbes.
pub fn fam_p_values(pmin: f64, pmax: f64, pas: f64) -> Result<Vec<f64>, String> {
    if !(pas.is_finite() && pas > 0.0) {
        return Err("famille : le pas de p doit être strictement positif".into());
    }
    if !(pmin.is_finite() && pmax.is_finite()) {
        return Err("famille : bornes de p non finies".into());
    }
    let (lo, hi) = if pmin <= pmax {
        (pmin, pmax)
    } else {
        (pmax, pmin)
    };
    let n = ((hi - lo) / pas + 1e-9).floor();
    if n + 1.0 > 256.0 {
        return Err(format!(
            "famille : {} valeurs de p (256 max) — augmentez le pas",
            n as i64 + 1
        ));
    }
    Ok((0..=n as usize).map(|k| lo + k as f64 * pas).collect())
}

/// Couleur automatique d'une courbe de famille (man. p. 26 : « choisie
/// par l'ordinateur ») : teintes réparties par angle d'or, saturation
/// 75 %, valeur 85 % (approche simplifiée de l'original).
pub fn couleur_palette(i: usize, _n: usize) -> [u8; 3] {
    hsv_rgb((i as f64 * 137.508) % 360.0, 0.75, 0.85)
}

/// HSV (h en degrés, s et v en 0..1) vers RGB 0..255.
fn hsv_rgb(h: f64, s: f64, v: f64) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h as u32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    ]
}

/// Texte d'une valeur de p : entier sans décimales, sinon décimales
/// épurées (le séparateur reste le point, comme les autres nombres
/// des exports).
fn fmt_p(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
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

/// Découpe les polylignes selon le style de trait (tirets/pointillés) :
/// les couleurs par polyligne suivent la découpe, les étiquettes
/// restent posées sur la courbe.
pub fn apply_style(tr: Trace, style: LineStyle) -> Trace {
    let Trace {
        polys,
        couleurs,
        etiquettes,
    } = tr;
    if style == LineStyle::Continu {
        return Trace {
            polys,
            couleurs,
            etiquettes,
        };
    }
    let (on, off) = match style {
        LineStyle::Tirets => (2.6, 1.6),
        _ => (0.45, 1.4),
    };
    let mut out: Vec<Poly> = Vec::new();
    let mut cols: Vec<[u8; 3]> = Vec::new();
    for (i, poly) in polys.iter().enumerate() {
        let avant = out.len();
        split_dashes(poly, on, off, &mut out);
        if let Some(&c) = couleurs.get(i) {
            for _ in avant..out.len() {
                cols.push(c);
            }
        }
    }
    Trace {
        polys: out,
        couleurs: cols,
        etiquettes,
    }
}

/// Découpe UNE polyligne en tronçons alternés (longueur on, off) :
/// pousse les morceaux pleins (≥ 2 points) dans `out`.
fn split_dashes(poly: &Poly, on: f64, off: f64, out: &mut Vec<Poly>) {
    let mut cur: Poly = Vec::new();
    let mut drawing = true;
    let mut remaining = on;
    let mut prev: Option<(f64, f64)> = None;
    for &(x, y) in poly {
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
        let dashed = apply_style(Trace::simple(polys), LineStyle::Tirets);
        assert!(dashed.polys.len() > 3, "segments = {}", dashed.polys.len());
        let dotted = apply_style(
            Trace::simple(vec![vec![(0.0, 0.0), (100.0, 0.0)]]),
            LineStyle::Pointilles,
        );
        assert!(dotted.polys.len() > 10);
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

    // ----- famille de fonctions fp(x) (man. p. 26-27, v1.7) -----

    fn famille(expr: &str, palette: bool, valeurs: bool, x_etiq: &str, pos: PosP) -> Curve {
        Curve::with_kind(
            CurveKind::Famille {
                expr: expr.into(),
                pmin: -5.0,
                pmax: 5.0,
                pas: 1.0,
                palette,
                valeurs,
                x_etiq: x_etiq.into(),
                pos,
                progression: false,
                progression_fine: false,
            },
            PALETTE[0],
        )
    }

    #[test]
    fn famille_onze_courbes_par_defaut() {
        // intervalle et pas par défaut : [−5 ; 5] pas 1 (man. p. 26)
        assert_eq!(
            fam_p_values(-5.0, 5.0, 1.0).unwrap(),
            (-5..=5).map(f64::from).collect::<Vec<_>>()
        );
        // bornes permutées : mêmes valeurs
        assert_eq!(
            fam_p_values(5.0, -5.0, 1.0).unwrap(),
            fam_p_values(-5.0, 5.0, 1.0).unwrap()
        );
        // y = p/5 reste dans la page : une polyligne par valeur de p
        let tr = sample_trace(
            &famille("p/5", false, false, "1", PosP::Dessus),
            &page(),
            400,
            100,
        )
        .unwrap();
        assert_eq!(tr.polys.len(), 11);
        assert!(tr.couleurs.is_empty());
        assert!(tr.etiquettes.is_empty());
        for poly in &tr.polys {
            for &(mx, my) in poly {
                assert!(mx.is_finite() && my.is_finite());
            }
        }
    }

    #[test]
    fn famille_pas_invalide() {
        assert!(fam_p_values(-5.0, 5.0, 0.0).is_err());
        assert!(fam_p_values(-5.0, 5.0, -1.0).is_err());
        assert!(fam_p_values(-5.0, 5.0, f64::NAN).is_err());
        assert!(fam_p_values(f64::NAN, 5.0, 1.0).is_err());
    }

    #[test]
    fn famille_plafond_256() {
        assert_eq!(fam_p_values(0.0, 255.0, 1.0).unwrap().len(), 256);
        assert!(fam_p_values(0.0, 300.0, 1.0).is_err());
    }

    #[test]
    fn famille_valeur_a_p_et_x() {
        // fp(x) = p·sin(x) en x = π/2, p = 2 → 2 (man. p. 26)
        let ast = crate::expr::parse("p*sin(x)").unwrap();
        ast.validate_vars(&["x", "p"]).unwrap();
        let mut unk = Vec::new();
        let v = eval_node(
            &ast,
            &crate::expr::VarSet {
                x: Some(std::f64::consts::FRAC_PI_2),
                p: Some(2.0),
                ..Default::default()
            },
            &mut unk,
        );
        assert!((v - 2.0).abs() < 1e-9);
        // paramètre collé aux facteurs (man. p. 26 : px² − 3px + 5) et
        // multiplication implicite après parenthèse ((p/2)x² + log p)
        for (src, pv, xv, attendu) in [
            ("px^2-3px+5", 1.0, 1.0, 3.0),
            ("(p/2)x^2+log(p)", 2.0, 3.0, 9.0 + 2.0f64.log10()),
            ("p sin(x)", 2.0, std::f64::consts::FRAC_PI_2, 2.0),
        ] {
            let ast = crate::expr::parse(src).unwrap();
            ast.validate_vars(&["x", "p"]).unwrap();
            let mut unk = Vec::new();
            let v = eval_node(
                &ast,
                &crate::expr::VarSet {
                    x: Some(xv),
                    p: Some(pv),
                    ..Default::default()
                },
                &mut unk,
            );
            assert!((v - attendu).abs() < 1e-9, "{src} → {v} ≠ {attendu}");
        }
    }

    #[test]
    fn famille_identifiant_inconnu() {
        let c = famille("q*sin(x)", false, false, "1", PosP::Dessus);
        let err = sample_trace(&c, &page(), 400, 100).unwrap_err();
        assert!(err.contains("identifiant inconnu"), "err = {err}");
        // abscisse d'affichage : expression en p seule
        let c = famille("p/5", false, true, "x", PosP::Dessus);
        assert!(sample_trace(&c, &page(), 400, 100).is_err());
    }

    #[test]
    fn famille_palette_distincte() {
        assert_ne!(couleur_palette(0, 11), couleur_palette(1, 11));
        let tr = sample_trace(
            &famille("p/5", true, false, "1", PosP::Dessus),
            &page(),
            400,
            100,
        )
        .unwrap();
        assert_eq!(tr.couleurs.len(), tr.polys.len());
        // sans palette : pas de couleur par polyligne
        let tr = sample_trace(
            &famille("p/5", false, false, "1", PosP::Dessus),
            &page(),
            400,
            100,
        )
        .unwrap();
        assert!(tr.couleurs.is_empty());
    }

    #[test]
    fn famille_etiquettes_positions() {
        // positions dessus < sur la courbe < dessous (mêmes p)
        let mut hauts = Vec::new();
        for p in [PosP::Dessus, PosP::Sur, PosP::Dessous] {
            let tr = sample_trace(&famille("p/5", false, true, "0", p), &page(), 400, 100).unwrap();
            assert_eq!(tr.etiquettes.len(), 11);
            // index 5 = p = 0 → texte « 0 », couleur de la courbe
            let e = &tr.etiquettes[5];
            assert_eq!(e.txt, "0");
            assert_eq!(e.couleur, PALETTE[0]);
            assert!((e.mm.0 - page().mm_x(0.0)).abs() < 1e-9);
            hauts.push(e.mm.1);
        }
        assert!(hauts[0] < hauts[1] && hauts[1] < hauts[2]);
        // étiquette hors domaine (1/p en p = 0) : simplement omise
        let tr = sample_trace(
            &famille("p/5", false, true, "1/p", PosP::Dessus),
            &page(),
            400,
            100,
        )
        .unwrap();
        assert_eq!(tr.etiquettes.len(), 10);
        // fmt_p : entier sans décimales, décimales épurées
        assert_eq!(fmt_p(-5.0), "-5");
        assert_eq!(fmt_p(0.5), "0.5");
        assert_eq!(fmt_p(-2.25), "-2.25");
    }

    #[test]
    fn famille_palette_couleur_etiquettes() {
        let tr = sample_trace(
            &famille("p/5", true, true, "0", PosP::Sur),
            &page(),
            400,
            100,
        )
        .unwrap();
        // l'étiquette de chaque p porte la couleur de palette de sa courbe
        assert_eq!(tr.etiquettes[0].couleur, tr.couleurs[0]);
        assert_eq!(tr.etiquettes[10].couleur, tr.couleurs[10]);
    }

    #[test]
    fn famille_conversion_depuis_fonction() {
        // y = f(x) sans p → enveloppée p·(f(x))
        let f = CurveKind::Fonction {
            expr: "sin(x)".into(),
        }
        .convert(KindId::Famille);
        assert!(
            matches!(&f, CurveKind::Famille { expr, .. } if expr == "p*(sin(x))"),
            "expr = {f:?}"
        );
        // expression déjà paramétrée : conservée telle quelle
        let f = CurveKind::Fonction { expr: "p*x".into() }.convert(KindId::Famille);
        assert!(matches!(&f, CurveKind::Famille { expr, .. } if expr == "p*x"));
        // autres natures : exemple du manuel, défauts [−5 ; 5] pas 1
        let f = CurveKind::Polaire {
            r: "x".into(),
            tmin: 0.0,
            tmax: 1.0,
        }
        .convert(KindId::Famille);
        assert!(
            matches!(&f, CurveKind::Famille { expr, pmin, pmax, pas, .. }
                if expr == "p*sin(x)" && *pmin == -5.0 && *pmax == 5.0 && *pas == 1.0),
            "expr = {f:?}"
        );
        // libellé de liste
        assert_eq!(
            famille("p*sin(x)", false, false, "1", PosP::Dessus).label(),
            "fp(x) = p*sin(x)"
        );
    }

    #[test]
    fn famille_session_defauts_serde() {
        // session écrite sans les champs optionnels : défauts du manuel
        let k: CurveKind = serde_json::from_str(r#"{"type":"famille","expr":"p*x"}"#).unwrap();
        assert!(
            matches!(&k, CurveKind::Famille { pmin, pmax, pas, x_etiq, .. }
                if *pmin == -5.0 && *pmax == 5.0 && *pas == 1.0 && x_etiq == "1"),
            "kind = {k:?}"
        );
        // sérialisation : clés camelCase et position « dessus »
        let js =
            serde_json::to_string(&famille("p*x", false, false, "1", PosP::Dessus).kind).unwrap();
        assert!(js.contains("\"xEtiq\":\"1\""), "json = {js}");
        assert!(js.contains("\"pos\":\"dessus\""), "json = {js}");
    }
}
