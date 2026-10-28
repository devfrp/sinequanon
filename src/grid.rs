//! Repère en millimètres : conversions coordonnées utilisateur ↔ page (mm).
//!
//! La page est une feuille de dimensions physiques (`width_mm` × `height_mm`)
//! dont le centre correspond au point utilisateur `(cx, cy)`. Une unité
//! utilisateur mesure `unit_mm` millimètres ; la grille mineure suit les
//! petits carreaux de 5 × 5 mm, comme dans Sine qua non.

use serde::{Deserialize, Serialize};

/// Pas de la grille mineure, en mm.
pub const PAS_MM: f64 = 5.0;

/// Page de dessin : dimensions physiques et repère utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// largeur de la page en mm
    pub width_mm: f64,
    /// hauteur de la page en mm
    pub height_mm: f64,
    /// abscisse du point situé au centre de la page
    pub cx: f64,
    /// ordonnée du point situé au centre de la page
    pub cy: f64,
    /// longueur d'une unité utilisateur, en mm
    pub unit_mm: f64,
    /// échelle logarithmique sur x (une unité = une décade)
    pub log_x: bool,
    /// échelle logarithmique sur y (une unité = une décade)
    pub log_y: bool,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            width_mm: 170.0,
            height_mm: 120.0,
            cx: 0.0,
            cy: 0.0,
            unit_mm: 20.0,
            log_x: false,
            log_y: false,
        }
    }
}

impl Page {
    // Un centre nul n'a pas de sens en échelle logarithmique : on retombe sur 1.
    fn log_center(c: f64) -> f64 {
        if c <= 0.0 {
            1.0
        } else {
            c
        }
    }

    fn forward(&self, u: f64, center: f64, log: bool) -> f64 {
        if log {
            let c = Self::log_center(center);
            if u <= 0.0 {
                return f64::NAN;
            }
            (u.log10() - c.log10()) * self.unit_mm
        } else {
            (u - center) * self.unit_mm
        }
    }

    fn backward(&self, m: f64, center: f64, log: bool) -> f64 {
        if log {
            let c = Self::log_center(center);
            10f64.powf(c.log10() + m / self.unit_mm)
        } else {
            center + m / self.unit_mm
        }
    }

    /// Position (mm depuis le bord gauche) de l'abscisse utilisateur `x`.
    pub fn mm_x(&self, x: f64) -> f64 {
        self.forward(x, self.cx, self.log_x) + self.width_mm / 2.0
    }

    /// Position (mm depuis le bord haut) de l'ordonnée utilisateur `y`.
    pub fn mm_y(&self, y: f64) -> f64 {
        self.height_mm / 2.0 - self.forward(y, self.cy, self.log_y)
    }

    /// Abscisse utilisateur du point situé à `mm` du bord gauche.
    pub fn x_at_mm(&self, mm: f64) -> f64 {
        self.backward(mm - self.width_mm / 2.0, self.cx, self.log_x)
    }

    /// Ordonnée utilisateur du point situé à `mm` du bord haut.
    pub fn y_at_mm(&self, mm: f64) -> f64 {
        self.backward(self.height_mm / 2.0 - mm, self.cy, self.log_y)
    }

    /// Plage d'abscisses visibles (bord gauche, bord droit).
    pub fn x_range(&self) -> (f64, f64) {
        (self.x_at_mm(0.0), self.x_at_mm(self.width_mm))
    }

    /// Plage d'ordonnées visibles (bord bas, bord haut).
    pub fn y_range(&self) -> (f64, f64) {
        (self.y_at_mm(self.height_mm), self.y_at_mm(0.0))
    }

    /// Valeurs « majeures » d'un axe : une par unité (ou une par décade).
    pub fn majors(&self, lo: f64, hi: f64, log: bool) -> Vec<f64> {
        if hi <= lo {
            return Vec::new();
        }
        if log {
            let lo = lo.max(1e-12);
            let hi = hi.max(1e-12);
            let k0 = lo.log10().floor() as i64;
            let k1 = hi.log10().ceil() as i64;
            let (k0, k1) = (k0.max(-30), k1.min(30));
            (k0..=k1)
                .filter(|k| {
                    let v = 10f64.powi(*k as i32);
                    v >= lo * (1.0 - 1e-9) && v <= hi * (1.0 + 1e-9)
                })
                .map(|k| 10f64.powi(k as i32))
                .collect()
        } else {
            let k0 = lo.floor() as i64;
            let k1 = hi.ceil() as i64;
            if k1 - k0 > 10_000 {
                return Vec::new();
            }
            (k0..=k1).map(|k| k as f64).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_lineaire() {
        let p = Page::default();
        for x in [-3.5, 0.0, 2.25, 10.0] {
            let back = p.x_at_mm(p.mm_x(x));
            assert!((back - x).abs() < 1e-9, "x={x} back={back}");
        }
        for y in [-2.0, 0.0, 1.75] {
            let back = p.y_at_mm(p.mm_y(y));
            assert!((back - y).abs() < 1e-9, "y={y} back={back}");
        }
    }

    #[test]
    fn origine_au_centre() {
        let p = Page::default();
        assert!((p.mm_x(0.0) - p.width_mm / 2.0).abs() < 1e-12);
        assert!((p.mm_y(0.0) - p.height_mm / 2.0).abs() < 1e-12);
        // y croît vers le haut
        assert!(p.mm_y(1.0) < p.mm_y(0.0));
    }

    #[test]
    fn majors_lineaires() {
        let p = Page::default();
        let m = p.majors(-3.2, 5.7, false);
        assert_eq!(m.first(), Some(&-4.0));
        assert_eq!(m.last(), Some(&6.0));
        assert_eq!(m.len(), 11);
    }

    #[test]
    fn majors_log() {
        let p = Page {
            log_x: true,
            ..Default::default()
        };
        let (lo, hi) = p.x_range();
        let m = p.majors(lo, hi, true);
        assert!(m.contains(&0.1) && m.contains(&1.0) && m.contains(&10.0));
        assert!(m.iter().all(|v| *v > 0.0));
    }

    #[test]
    fn aller_retour_log() {
        let p = Page {
            log_x: true,
            cx: 1.0,
            ..Default::default()
        };
        for x in [0.01, 1.0, 100.0] {
            let back = p.x_at_mm(p.mm_x(x));
            assert!((back - x).abs() / x < 1e-9, "x={x} back={back}");
        }
        // x ≤ 0 n'a pas de position en log
        assert!(p.mm_x(0.0).is_nan());
        assert!(p.mm_x(-1.0).is_nan());
    }

    #[test]
    fn plage_log_avec_centre_nul() {
        // cx = 0 est remplacé par 1 en échelle log : la plage reste positive
        let p = Page {
            log_x: true,
            ..Default::default()
        };
        let (lo, hi) = p.x_range();
        assert!(lo > 0.0 && hi > lo);
    }
}
