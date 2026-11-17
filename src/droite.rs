//! Droites définies par équation réduite (man. p. 25) : « y = ax + b »
//! ou « x = a », saisies complètes, dessinées puis étiquetées.

use crate::curve::LineStyle;
use crate::grid::Page;
use serde::{Deserialize, Serialize};

/// Une droite saisie par équation réduite (10 maximum, man. p. 25).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DroiteEq {
    /// équation complète : « y = 2x + 1 », « x = PI/2 »…
    pub eq: String,
    /// dessiner la droite
    pub visible: bool,
    /// afficher l'équation sur la droite
    pub afficher_eq: bool,
    pub color: [u8; 3],
    /// épaisseur (mm, 0,1 à 1 comme les courbes)
    pub thickness: f32,
    pub style: LineStyle,
}

impl Default for DroiteEq {
    fn default() -> Self {
        Self {
            eq: String::new(),
            visible: true,
            afficher_eq: false,
            color: [0, 0, 204],
            thickness: 1.0,
            style: LineStyle::Continu,
        }
    }
}

/// Forme dégagée de l'équation : oblique y = ax + b ou verticale x = a.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Forme {
    Oblique { a: f64, b: f64 },
    Verticale { x0: f64 },
}

/// Évalue une expression, erreur si variable autre que x.
fn eval(expr: &str, x: Option<f64>) -> Result<f64, String> {
    let ast = crate::expr::parse(expr)?;
    ast.validate_vars(if x.is_some() { &["x"] } else { &[] })?;
    let mut unknown = Vec::new();
    let v = ast.eval(
        &crate::expr::VarSet {
            x,
            ..Default::default()
        },
        &mut unknown,
    );
    if let Some(u) = unknown.first() {
        return Err(format!("identifiant inconnu : {u}"));
    }
    if !v.is_finite() {
        return Err("expression non finie".into());
    }
    Ok(v)
}

/// Analyse une équation réduite : commence obligatoirement par « y = »
/// ou « x = » (espaces ignorés, man. p. 25). L'expression peut utiliser
/// des constantes calculées (PI) et des fonctions : y=(ln(2))x+sin(pi/6).
pub fn analyse(eq: &str) -> Result<Forme, String> {
    let s: String = eq
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    let (axe, reste) = if let Some(r) = s.strip_prefix("y=") {
        ("y", r)
    } else if let Some(r) = s.strip_prefix("x=") {
        ("x", r)
    } else {
        return Err("droite : l'équation doit commencer par « y = » ou « x = »".into());
    };
    if reste.is_empty() {
        return Err("droite : expression vide après le signe =".into());
    }
    if axe == "x" {
        // verticale : la valeur doit être une constante
        let x0 = eval(reste, None)?;
        return Ok(Forme::Verticale { x0 });
    }
    // oblique : a = f(1) − f(0), b = f(0) ; expression non affine refusée
    let b = eval(reste, Some(0.0))?;
    let f1 = eval(reste, Some(1.0))?;
    let a = f1 - b;
    let f2 = eval(reste, Some(2.0))?;
    if ((f2 - f1) - a).abs() > 1e-6 * (1.0 + a.abs()) {
        return Err(format!("droite : {eq} n'est pas de la forme y = ax + b"));
    }
    if !a.is_finite() || !b.is_finite() {
        return Err(format!("droite : {eq} a des coefficients non finis"));
    }
    Ok(Forme::Oblique { a, b })
}

/// Segment (mm, coordonnées page) de la droite, clippé au rectangle de
/// la page ; None si la droite est entièrement hors page.
pub fn segment_mm(page: &Page, forme: &Forme) -> Option<((f64, f64), (f64, f64))> {
    let (w, h) = (page.width_mm, page.height_mm);
    let mut p0;
    let mut p1;
    match forme {
        Forme::Verticale { x0 } => {
            let x = page.mm_x(*x0);
            if !(0.0..=w).contains(&x) {
                return None;
            }
            p0 = (x, 0.0);
            p1 = (x, h);
        }
        Forme::Oblique { a, b } => {
            let (u0, u1) = page.x_range();
            p0 = (page.mm_x(u0), page.mm_y(a * u0 + b));
            p1 = (page.mm_x(u1), page.mm_y(a * u1 + b));
        }
    }
    // Liang–Barsky sur le rectangle de la page
    let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
    let mut t0: f64 = 0.0;
    let mut t1: f64 = 1.0;
    let conds: [(f64, f64); 4] = [(-dx, p0.0), (dx, w - p0.0), (-dy, p0.1), (dy, h - p0.1)];
    for &(p, q) in conds.iter() {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
    }
    if t0 > t1 {
        return None;
    }
    p0 = (p0.0 + t0 * dx, p0.1 + t0 * dy);
    p1 = (p0.0 + (t1 - t0) * dx, p0.1 + (t1 - t0) * dy);
    Some((p0, p1))
}

/// Taille (mm) de l'étiquette de l'équation sur la droite.
pub const TAILLE_EQ: f64 = 2.2;

/// Polylignes (mm) dessinées : segment clippé, découpé suivant le style
/// de trait (tirets 2,6/1,6 mm, pointillés — comme les courbes).
pub fn polys(page: &Page, forme: &Forme, style: LineStyle) -> Vec<Vec<(f64, f64)>> {
    let Some((p0, p1)) = segment_mm(page, forme) else {
        return Vec::new();
    };
    crate::curve::apply_style(crate::curve::Trace::simple(vec![vec![p0, p1]]), style).polys
}

/// Étiquette affichée sur la droite (man. p. 25) : l'équation, au bout
/// droit de la droite juste au-dessus ; pour une verticale, le long de
/// la droite près du bas (lecture bas → haut, comme le texte vertical).
pub struct Libelle {
    /// point d'ancrage (mm) : la même convention que le texte du
    /// schéma (ligne de base du haut de casse pour l'horizontal,
    /// ancre ci-dessous pour le vertical)
    pub mm: (f64, f64),
    pub txt: String,
    pub verticale: bool,
    /// « end » (bout droit, horizontal) ou « start »
    pub ancre: &'static str,
}

pub fn libelle(page: &Page, forme: &Forme, eq: &str) -> Option<Libelle> {
    if eq.trim().is_empty() {
        return None;
    }
    let (p0, p1) = segment_mm(page, forme)?;
    match forme {
        Forme::Verticale { .. } => Some(Libelle {
            mm: (p0.0 - 0.45 * TAILLE_EQ, page.height_mm - 1.0),
            txt: eq.trim().to_string(),
            verticale: true,
            ancre: "start",
        }),
        Forme::Oblique { .. } => {
            let fin = if p0.0 > p1.0 { p0 } else { p1 };
            // le bout droit peut toucher le bord haut ou bas de la page :
            // on ramène l'ancrage (haut de casse) à l'intérieur
            let mut mm1 = fin.1 - 0.25;
            // ligne de base (ancre + 0,72·t) au-dessus du bord bas
            mm1 = mm1.min(page.height_mm - 0.3 - 0.72 * TAILLE_EQ);
            // haut des glyphes sous le bord haut
            mm1 = mm1.max(0.25 + 0.3 * TAILLE_EQ);
            Some(Libelle {
                mm: (fin.0 - 0.3, mm1),
                txt: eq.trim().to_string(),
                verticale: false,
                ancre: "end",
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        Page::default()
    }

    #[test]
    fn analyse_oblique() {
        let f = analyse("y = 3/2*x + 1").unwrap();
        assert_eq!(f, Forme::Oblique { a: 1.5, b: 1.0 });
        // forme y = b + a·x (man. p. 25)
        let f = analyse("y=1+3/2x").unwrap();
        assert_eq!(f, Forme::Oblique { a: 1.5, b: 1.0 });
        // constantes calculées et fonctions (man. p. 25)
        let f = analyse("y=(ln(2))x+sin(pi/6)").unwrap();
        let Forme::Oblique { a, b } = f else {
            panic!("forme oblique attendue");
        };
        assert!((a - std::f64::consts::LN_2).abs() < 1e-12);
        assert!((b - 0.5).abs() < 1e-9);
        // négative
        let f = analyse("y=-2x-3").unwrap();
        assert_eq!(f, Forme::Oblique { a: -2.0, b: -3.0 });
    }

    #[test]
    fn analyse_verticale() {
        let f = analyse("x = -3").unwrap();
        assert_eq!(f, Forme::Verticale { x0: -3.0 });
        let f = analyse("x=PI/2").unwrap();
        assert_eq!(
            f,
            Forme::Verticale {
                x0: 0.5 * std::f64::consts::PI
            }
        );
    }

    #[test]
    fn analyse_erreurs() {
        // pas de préfixe
        assert!(analyse("2x+1").is_err());
        // expression non affine
        assert!(analyse("y=x^2").is_err());
        // vide
        assert!(analyse("y=").is_err());
        // variable interdite dans x =
        assert!(analyse("x=y").is_err());
    }

    #[test]
    fn segment_clipped() {
        let p = page(); // 170 × 120 mm, unité 20 mm
                        // y = 0 → segment horizontal sur toute la largeur
        let ((x0, y0), (x1, y1)) = segment_mm(&p, &analyse("y=0").unwrap()).unwrap();
        assert_eq!((x0, y0), (0.0, 60.0));
        assert_eq!((x1, y1), (170.0, 60.0));
        // x = -3 : mm_x(-3) = 85 + (-3)·20 = 25
        let ((x0, y0), (x1, y1)) = segment_mm(&p, &analyse("x=-3").unwrap()).unwrap();
        assert_eq!((x0, y0), (25.0, 0.0));
        assert_eq!((x1, y1), (25.0, 120.0));
        // x = -10 : complètement hors page
        assert!(segment_mm(&p, &analyse("x=-10").unwrap()).is_none());
        // y = x hors page en haut à droite ?
        // mm : de (0 ; 2050 mm) à (170 ; −150 mm) → coupé
        let ((x0, y0), (_, y1)) = segment_mm(&p, &analyse("y=x").unwrap()).unwrap();
        assert!(x0 >= 0.0 - 1e-9);
        assert!(y1 <= 60.0 + 1e-9);
        assert!(y0 <= 120.0 + 1e-9);
    }

    #[test]
    fn etiquette_sur_la_droite() {
        let p = Page::default();
        let f = analyse("y=2x+1").unwrap();
        let l = libelle(&p, &f, "y = 2x + 1").unwrap();
        assert!(!l.verticale && l.ancre == "end");
        assert_eq!(l.txt, "y = 2x + 1");
        // ancre au-dessus du bout droit
        assert!(l.mm.1 < 60.0);
        // verticale : le long de la droite
        let f = analyse("x=3").unwrap();
        let l = libelle(&p, &f, "x=3").unwrap();
        assert!(l.verticale);
        assert!(l.mm.0 < p.mm_x(3.0));
        // oblique qui sort par le bord bas : l'étiquette reste dans la page
        let f = analyse("y=1-2x").unwrap();
        let l = libelle(&p, &f, "y=1-2x").unwrap();
        let case = l.mm.1 + 0.72 * TAILLE_EQ;
        assert!(
            case <= p.height_mm - 0.2,
            "base = {case} hors page haute {}",
            p.height_mm
        );
        assert!(l.mm.1 >= 0.9);
        // et bord haut : y = 2x+1 finit en haut à droite
        let f = analyse("y=2x+1").unwrap();
        let l = libelle(&p, &f, "y=2x+1").unwrap();
        assert!(l.mm.1 >= 0.9, "ancre = {} hors page", l.mm.1);
    }
}
