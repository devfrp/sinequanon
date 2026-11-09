//! Systèmes d'inéquations linéaires à deux inconnues (man. p. 80-81) :
//! jusqu'à 10 inéquations « ax+by+c <0 / >0 / ≤0 / ≥0 » par système,
//! 10 systèmes. La solution (intersection des demi-plans) est hachurée
//! en diagonales 45° — ou à l'inverse la zone non solution, choix du
//! manuel pour lire un système (« Zone non hachurée »). Les frontières
//! ax+by+c = 0 sont tracées.

use crate::grid::Page;
use serde::{Deserialize, Serialize};

/// Sens de l'inégalité (forme ax+by+c < 0, > 0, ≤ 0 ou ≥ 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nature {
    Inf,
    Sup,
    InfEq,
    SupEq,
}

/// Une inééquation linéaire ax+by+c ⋛ 0 (10 maximum par système).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inequation {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub nature: Nature,
}

/// Un système d'inéquations (10 maximum, man. p. 80).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemeIneq {
    /// inéquations du système (10 max)
    pub ineqs: Vec<Inequation>,
    /// true : hachurer la zone solution ; false : hachurer le
    /// complément (la solution reste blanche — man. p. 80)
    pub hachure_solution: bool,
    /// espacement des hachures (mm, défaut 3)
    pub pas: f32,
    pub color: [u8; 3],
    /// dessiner le système
    pub visible: bool,
}

impl Default for SystemeIneq {
    fn default() -> Self {
        Self {
            ineqs: Vec::new(),
            hachure_solution: false,
            pas: 3.0,
            color: [102, 0, 204],
            visible: true,
        }
    }
}

impl Inequation {
    /// Valeur ax+by+c, côté positif = côté >= 0.
    fn valeur(&self, x: f64, y: f64) -> f64 {
        self.a * x + self.b * y + self.c
    }

    /// Forme à demi-plan positif : g(x, y) >= 0 est la solution.
    fn g(&self, x: f64, y: f64) -> f64 {
        match self.nature {
            Nature::Sup | Nature::SupEq => self.valeur(x, y),
            Nature::Inf | Nature::InfEq => -self.valeur(x, y),
        }
    }
}

/// Découpe un polygone convexe par le demi-plan f(x, y) >= 0
/// (Sutherland–Hodgman ; à tolérance de bord incluse).
fn clip_demi_plan(poly: &[(f64, f64)], f: impl Fn(f64, f64) -> f64) -> Vec<(f64, f64)> {
    let n = poly.len();
    if n == 0 {
        return Vec::new();
    }
    let interior = |x: f64, y: f64| f(x, y) >= -1e-12;
    let mut out = Vec::with_capacity(n + 1);
    for k in 0..n {
        let (u, v) = (poly[k], poly[(k + 1) % n]);
        let (fu, fv) = (f(u.0, u.1), f(v.0, v.1));
        let r = if (fu - fv).abs() < 1e-15 {
            f64::NAN
        } else {
            fu / (fu - fv)
        };
        let Some(t) = (0.0..=1.0)
            .contains(&r)
            .then_some((u.0 + r * (v.0 - u.0), u.1 + r * (v.1 - u.1)))
        else {
            if interior(v.0, v.1) {
                out.push(v);
            }
            continue;
        };
        if interior(v.0, v.1) {
            if !interior(u.0, u.1) {
                out.push(t);
            }
            out.push(v);
        } else if interior(u.0, u.1) {
            out.push(t);
        }
    }
    out
}

const EPS_PTS: f64 = 1e-9;

fn dedup(poly: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for p in poly {
        let ok = match out.last() {
            None => true,
            Some(q) => (q.0 - p.0).abs() > EPS_PTS || (q.1 - p.1).abs() > EPS_PTS,
        };
        if ok {
            out.push(p);
        }
    }
    if out.len() > 1 {
        let (a, b) = (*out.last().unwrap(), out[0]);
        if (a.0 - b.0).abs() <= EPS_PTS && (a.1 - b.1).abs() <= EPS_PTS {
            out.pop();
        }
    }
    out
}

/// Liste des polygones à hacher, en mm (page) : la zone solution
/// (un polygone convexe) ou chaque complément de demi-plan (conseil
/// du manuel p. 80 : « zone non hachurée »).
pub fn zones(page: &Page, s: &SystemeIneq) -> Vec<Vec<(f64, f64)>> {
    // découpe en coordonnées utilisateur (les inéquations sont écrites
    // en user), cartographie mm à la sortie
    let (u0, u1) = page.x_range();
    let (v1, v0) = page.y_range();
    let rect = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)];
    let mm = |(x, y): (f64, f64)| (page.mm_x(x), page.mm_y(y));
    if s.hachure_solution {
        let mut cur = rect.to_vec();
        for ineq in &s.ineqs {
            cur = clip_demi_plan(&cur, |x, y| ineq.g(x, y));
            if cur.is_empty() {
                return Vec::new();
            }
        }
        let cur: Vec<(f64, f64)> = dedup(cur).into_iter().map(mm).collect();
        (cur.len() >= 3).then_some(cur).into_iter().collect()
    } else {
        s.ineqs
            .iter()
            .map(|ineq| {
                // hachurer le NON-solution : opposé du demi-plan
                let cur = clip_demi_plan(&rect, |x, y| -ineq.g(x, y));
                dedup(cur).into_iter().map(mm).collect()
            })
            .filter(|p: &Vec<(f64, f64)>| p.len() >= 3)
            .collect()
    }
}

/// Épaisseur (mm) des traits de hachure.
pub const TRAIT_MM: f64 = 0.35;

/// Segments de hachures 45° (mm) clippés à un polygone convexe :
/// la famille x+y = c, espacement perpendiculaire = pas.
pub fn hachures_poly(poly: &[(f64, f64)], pas: f64) -> Vec<((f64, f64), (f64, f64))> {
    if poly.len() < 3 || pas <= 0.0 {
        return Vec::new();
    }
    // pas = distance perpendiculaire → c perd √2·pas entre lignes
    let g = pas * std::f64::consts::SQRT_2;
    let mut smin = f64::MAX;
    let mut smax = f64::MIN;
    for &(x, y) in poly {
        smin = smin.min(x + y);
        smax = smax.max(x + y);
    }
    let mut out = Vec::new();
    let mut c = (smin / g).floor() * g;
    while c <= smax {
        // intersections de x+y = c avec les arêtes (polygone convexe :
        // 0 ou 2 points d'entrée/sortie)
        let mut ts: Vec<f64> = Vec::new();
        for k in 0..poly.len() {
            let (u, v) = (poly[k], poly[(k + 1) % poly.len()]);
            let su = u.0 + u.1;
            let sv = v.0 + v.1;
            let d = sv - su;
            if d.abs() < 1e-12 {
                continue;
            }
            let t = (c - su) / d;
            if (0.0..=1.0).contains(&t) {
                ts.push(u.0 + t * (v.0 - u.0));
            }
        }
        if ts.len() >= 2 {
            ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let t0 = ts[0];
            let t1 = *ts.last().unwrap();
            if t1 - t0 > 1e-6 {
                let cut = |t: f64| {
                    (
                        t.clamp(smin - g * 0.5, smax + g * 0.5),
                        c - t.clamp(smin - g * 0.5, smax + g * 0.5),
                    )
                };
                out.push((cut(t0), cut(t1)));
            }
        }
        c += g;
    }
    out
}

/// Tous les segments de hachures du système (mm de page).
pub fn hachures(page: &Page, s: &SystemeIneq) -> Vec<((f64, f64), (f64, f64))> {
    if !s.visible {
        return Vec::new();
    }
    zones(page, s)
        .iter()
        .flat_map(|poly| hachures_poly(poly, s.pas as f64))
        .collect()
}

/// Frontières du système : segments (mm) des droites ax+by+c = 0,
/// clippés au rectangle de la page (réutilise Liang–Barsky des droites).
pub fn frontieres(page: &Page, s: &SystemeIneq) -> Vec<((f64, f64), (f64, f64))> {
    if !s.visible {
        return Vec::new();
    }
    let mut out = Vec::new();
    for ineq in &s.ineqs {
        let (a, b, c) = (ineq.a, ineq.b, ineq.c);
        // inutilisée dans le cas b = 0 exact : verticale x = −c/a
        let forme = if b.abs() > 1e-12 {
            crate::droite::Forme::Oblique {
                a: -a / b,
                b: -c / b,
            }
        } else if a.abs() > 1e-12 {
            crate::droite::Forme::Verticale { x0: -c / a }
        } else {
            continue;
        };
        if let Some(seg) = crate::droite::segment_mm(page, &forme) {
            out.push(seg);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        Page::default() // x ∈ [−4.25 ; 4.25], y ∈ [−3 ; 3]
    }

    /// ramène un polygone sortie (mm) aux coordonnées utilisateur
    fn user(p: &Page, poly: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        poly.into_iter()
            .map(|(x, y)| (p.x_at_mm(x), p.y_at_mm(y)))
            .collect()
    }

    fn ineq(a: f64, b: f64, c: f64, nature: Nature) -> Inequation {
        Inequation { a, b, c, nature }
    }

    #[test]
    fn demi_plan_unique() {
        // x >= 0 : solution moitié droite de la page
        let s = SystemeIneq {
            hachure_solution: true,
            ineqs: vec![ineq(1.0, 0.0, 0.0, Nature::SupEq)],
            ..Default::default()
        };
        let p = page();
        let z = zones(&p, &s);
        assert_eq!(z.len(), 1);
        // la moitié droite : tous les sommets avec x >= 0 et l'un sur
        // l'axe vertical
        let zu = user(&p, z[0].clone());
        for &(x, _) in &zu {
            assert!(x >= -1e-9, "x = {x}");
        }
        assert!(zu.iter().any(|&(x, _)| x.abs() < 1e-9));
        assert!(zu.iter().any(|&(x, _)| (x - 4.25).abs() < 1e-9));
    }

    #[test]
    fn complement_de_demi_plan() {
        // conseil du manuel : hachurer le complément de x >= 0
        let mut s = SystemeIneq::default();
        s.ineqs.push(ineq(1.0, 0.0, 0.0, Nature::SupEq));
        let p = page();
        let z = zones(&p, &s);
        assert_eq!(z.len(), 1);
        let zu = user(&p, z[0].clone());
        for &(x, _) in &zu {
            assert!(x <= 1e-9, "x = {x}");
        }
        assert!(zu.iter().any(|&(x, _)| (x + 4.25).abs() < 1e-9));
    }

    #[test]
    fn intersection_systeme() {
        // { x >= -1 ; y >= 0 ; x+y >= 1 } : sommets (−1 ; 3), (4,25 ; 3),
        // (4,25 ; 0), (1 ; 0), (−1 ; 2)
        let s = SystemeIneq {
            hachure_solution: true,
            ineqs: vec![
                ineq(1.0, 0.0, 1.0, Nature::SupEq),
                ineq(0.0, 1.0, 0.0, Nature::SupEq),
                ineq(1.0, 1.0, -1.0, Nature::SupEq),
            ],
            ..Default::default()
        };
        let p = page();
        let z = zones(&p, &s);
        assert_eq!(z.len(), 1, "un polygone");
        let poly = &user(&p, z[0].clone());
        // tous les sommets vérifient les 3 inéquations
        for &(x, y) in poly {
            assert!(x >= -1.0 - 1e-9, "x = {x}");
            assert!(y >= -1e-9, "y = {y}");
            assert!(x + y >= 1.0 - 1e-9, "x+y = {}", x + y);
        }
        // les contraintes actives sont bien présentes
        assert!(poly.iter().any(|&(x, _)| (x + 1.0).abs() < 1e-9));
        assert!(poly.iter().any(|&(_, y)| y.abs() < 1e-9));
        assert!(poly.iter().any(
            |&(x, y)| ((x + y - 1.0).round() - (x + y - 1.0)).abs() < 1e-9 && x + y < 1.0 + 1e-6
        ));
    }

    #[test]
    fn stricts_et_inverses() {
        // x < 0 hachuré par défaut ? non : on hachure le complément,
        // soit x >= 0 (les sommets sont à droite)
        let mut s = SystemeIneq::default();
        s.ineqs.push(ineq(1.0, 0.0, 0.0, Nature::Inf));
        let z = zones(&page(), &s);
        assert_eq!(z.len(), 1);
        let zu = user(&page(), z[0].clone());
        for &(x, _) in &zu {
            assert!(x >= -1e-9, "x = {x}");
        }
        assert!(zu.iter().any(|&(x, _)| (x - 4.25).abs() < 1e-9));
        // solution stricte vide ? x >= 100 : impossible sur la page
        let s = SystemeIneq {
            hachure_solution: true,
            ineqs: vec![ineq(1.0, 0.0, -100.0, Nature::SupEq)],
            ..Default::default()
        };
        assert!(zones(&page(), &s).is_empty());
    }

    #[test]
    fn hachures_diagonales() {
        // moitié droite de page, pas 3 mm : chaque segment est une
        // diagonale x+y = c, à l'intérieur du polygone
        let s = SystemeIneq {
            hachure_solution: true,
            ineqs: vec![ineq(1.0, 0.0, 0.0, Nature::SupEq)],
            ..Default::default()
        };
        let p = page();
        let z = &zones(&p, &s)[0];
        let hs = hachures_poly(z, 3.0);
        assert!(!hs.is_empty());
        // diagonales exactes : chaque segment porte x+y = c
        for (a, b) in &hs {
            let ca = a.0 + a.1;
            let cb = b.0 + b.1;
            assert!((ca - cb).abs() < 1e-9, "non diagonale {ca} {cb}");
            // à l'intérieur du polygone : x >= 0
            assert!(a.0 >= -1e-9 && b.0 >= -1e-9);
            assert!(a.1 >= -3.0 - 1e-9 && b.1 >= -3.0 - 1e-9);
        }
        // espacement : les c sont en progression arithmétique avec un
        // pas de 3·√2 (multiple flottant toléré sur le premier)
        let mut cs: Vec<f64> = hs.iter().map(|(a, _)| a.0 + a.1).collect();
        cs.sort_by(|x, y| x.partial_cmp(y).unwrap());
        for w in cs.windows(2) {
            let ecart = w[1] - w[0];
            let n = (ecart / (3.0 * std::f64::consts::SQRT_2)).round();
            assert!(
                (ecart - n * 3.0 * std::f64::consts::SQRT_2).abs() < 1e-6,
                "écart de {ecart}"
            );
        }
    }

    #[test]
    fn frontieres_droites() {
        let s = SystemeIneq {
            ineqs: vec![ineq(1.0, 1.0, -1.0, Nature::SupEq)],
            ..Default::default()
        };
        let p = page();
        let f = frontieres(&p, &s);
        assert_eq!(f.len(), 1);
        // les deux bouts, ramenés en user : x+y = 1
        for (a, b) in &f {
            let ua = p.x_at_mm(a.0);
            let va = p.y_at_mm(a.1);
            assert!((ua + va - 1.0).abs() < 1e-6);
            let ub = p.x_at_mm(b.0);
            let vb = p.y_at_mm(b.1);
            assert!((ub + vb - 1.0).abs() < 1e-6);
        }
    }
}
