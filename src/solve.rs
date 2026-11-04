//! Solveur numérique du menu « Calculs » (v1.1 ; man. p. 95-96) :
//! résolution approchée de f(x) = g(x) par balayage + dichotomie, et
//! extrema locaux par annulation de la dérivée numérique.
//!
//! Le résultat est une liste de valeurs approchées que l'on peut
//! marquer sur la feuille (points placés sur la courbe, comme
//! l'« interprétation graphique » de l'original).

/// Recherche approchée des zéros de `f` sur `[a ; b]` : balayage en
/// `pas` segments, puis dichotomie sur chaque changement de signe.
/// Les valeurs hors domaine (non finies) sont ignorées.
pub fn zeros(f: &dyn Fn(f64) -> f64, a: f64, b: f64, pas: usize) -> Vec<f64> {
    let pas = pas.max(8);
    if !a.is_finite() || !b.is_finite() || b <= a {
        return Vec::new();
    }
    let h = (b - a) / pas as f64;
    let mut prev = f(a);
    let mut prevx = a;
    let mut out = Vec::new();
    for i in 1..=pas {
        let x = a + h * i as f64;
        let v = f(x);
        if prev.is_finite() && v.is_finite() {
            if prev == 0.0 {
                out.push(prevx);
            } else if v * prev < 0.0 {
                out.push(dichotomie(f, prevx, x, prev, v));
            }
        }
        if i == pas && v.is_finite() && v.abs() < h * 1e-6 {
            // zéro porté par la borne droite (sin 2π, etc.)
            out.push(x);
        }
        prev = v;
        prevx = x;
    }
    // dédoublonnage (zéro d'extrémité repéré deux fois, etc.)
    out.dedup_by(|u, v| (*v - *u).abs() < (b - a).abs() * 1e-9);
    out
}

/// Résolution de f(x) = g(x) sur `[a ; b]` : zéros de f − g.
pub fn resolue(
    f1: &dyn Fn(f64) -> f64,
    f2: &dyn Fn(f64) -> f64,
    a: f64,
    b: f64,
    pas: usize,
) -> Vec<f64> {
    let g = |x: f64| f1(x) - f2(x);
    zeros(&g, a, b, pas)
}

/// Extrema locaux de `f` sur `[a ; b]` : annulations de la dérivée
/// numérique (différences centrées), nature déterminée par la dérivée
/// seconde. Renvoie `(x, f(x), est_un_minimum)` trié par x.
pub fn extrema(f: &dyn Fn(f64) -> f64, a: f64, b: f64, pas: usize) -> Vec<(f64, f64, bool)> {
    let pas = pas.max(8);
    if !a.is_finite() || !b.is_finite() || b <= a {
        return Vec::new();
    }
    let h = (b - a) / pas as f64;
    let d = |x: f64| (f(x + h) - f(x - h)) / (2.0 * h);
    let mut out = Vec::new();
    for x in zeros(&d, a + h, b - h, pas) {
        let y = f(x);
        if !y.is_finite() {
            continue;
        }
        // dérivée seconde numérique : > 0 → minimum
        let dd = f(x + h) - 2.0 * y + f(x - h);
        out.push((x, y, dd > 0.0));
    }
    out
}

/// Dichotomie classique entre deux abscisses de signes opposés.
fn dichotomie(f: &dyn Fn(f64) -> f64, mut lo: f64, mut hi: f64, mut flo: f64, mut fhi: f64) -> f64 {
    for _ in 0..80 {
        let m = 0.5 * (lo + hi);
        if m <= lo || m >= hi {
            break;
        }
        let fm = f(m);
        if !fm.is_finite() {
            break;
        }
        if fm == 0.0 {
            return m;
        }
        if flo * fm < 0.0 {
            hi = m;
            fhi = fm;
        } else {
            lo = m;
            flo = fm;
        }
        if (hi - lo).abs() <= (lo.abs() + hi.abs()).max(1.0) * 1e-15 {
            break;
        }
    }
    // flo/fhi font croire que l'intervalle reste encadrant
    let _ = fhi;
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f2(x: f64) -> f64 {
        x * x - 2.0
    }

    #[test]
    fn zero_de_x2_moins_2() {
        let z = zeros(&f2, 0.0, 2.0, 4000);
        assert_eq!(z.len(), 1);
        assert!((z[0] - 2.0f64.sqrt()).abs() < 1e-9, "√2 ≈ {}", z[0]);
    }

    #[test]
    fn zeros_du_sinus() {
        let s = zeros(&f64::sin, 0.0, 2.0 * std::f64::consts::PI, 4000);
        assert_eq!(s.len(), 3, "{s:?}");
        let pi = std::f64::consts::PI;
        assert!((s[0]).abs() < 1e-4);
        assert!((s[1] - pi).abs() < 1e-9);
        assert!((s[2] - 2.0 * pi).abs() < 1e-9);
    }

    #[test]
    fn extrema_de_x2_moins_2() {
        let e = extrema(&f2, -2.0, 2.0, 4000);
        assert_eq!(e.len(), 1);
        let (x, y, min) = e[0];
        assert!(x.abs() < 1e-9, "x = {x}");
        assert!((y + 2.0).abs() < 1e-9, "y = {y}");
        assert!(min);
    }

    #[test]
    fn extrema_de_x3_moins_3x() {
        let f = |x: f64| x.powi(3) - 3.0 * x;
        let e = extrema(&f, -3.0, 3.0, 4000);
        assert_eq!(e.len(), 2, "{e:?}");
        let (x0, y0, min0) = e[0];
        let (x1, y1, min1) = e[1];
        assert!(
            (x0 + 1.0).abs() < 1e-5 && (y0 - 2.0).abs() < 1e-3 && !min0,
            "{e:?}"
        );
        assert!(
            (x1 - 1.0).abs() < 1e-5 && (y1 + 2.0).abs() < 1e-3 && min1,
            "{e:?}"
        );
    }

    #[test]
    fn intersection_exemple_du_manuel() {
        // f1(x) = 4x / (x² + 1), f2(x) = 0,5x + 1 (man. p. 95)
        let f1 = |x: f64| 4.0 * x / (x * x + 1.0);
        let f2 = |x: f64| 0.5 * x + 1.0;
        let s = resolue(&f1, &f2, -10.0, 10.0, 4000);
        assert!(!s.is_empty(), "aucune intersection trouvée");
        // chaque solution vérifie bien f1(x) = f2(x)
        for &x in &s {
            let d = (f1(x) - f2(x)).abs();
            assert!(d < 1e-6, "x = {x} : |f1 − f2| = {d}");
        }
    }

    #[test]
    fn domaines_non_definis_ignores() {
        // √x : hors [0 ; ∞) la fonction n'est pas finie
        let r = |x: f64| x.sqrt();
        let z = zeros(&r, -1.0, 4.0, 4000);
        assert_eq!(z.len(), 1);
        assert!(z[0].abs() < 1e-9);
    }
}
