//! Intégration numérique (man. p. 97-99, menu Calculs/Intégration) :
//! valeur approchée d'une intégrale par rectangles (à gauche, à
//! droite, point-milieu) ou trapèzes, avec encadrement.

/// Méthodes de l'original : « rectangles à gauche », « à droite »,
/// « point-milieu » et « trapèzes ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Methode {
    Gauche,
    Droite,
    Milieu,
    Trapezes,
}

/// Somme approchée de ∫ₐᵇ f(x) dx par `n` subdivisions (`n ≥ 1`).
pub fn somme(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize, m: Methode) -> f64 {
    let n = n.max(1);
    let dx = (b - a) / n as f64;
    let mut total = 0.0;
    for i in 0..n {
        let x0 = a + i as f64 * dx;
        let val = match m {
            Methode::Gauche => f(x0),
            Methode::Droite => f(x0 + dx),
            Methode::Milieu => f(x0 + dx / 2.0),
            Methode::Trapezes => (f(x0) + f(x0 + dx)) / 2.0,
        };
        total += val;
    }
    total * dx
}

/// Encadrement par rectangles inscrits / exinscrits : sur chaque
/// sous-intervalle on échantillonne à gauche, au milieu et à droite,
/// la somme des minima majore la partie inférieure et la somme des
/// maxima la partie supérieure (strict pour f continue monotone par
/// morceaux avec 3 points d'échantillonnage).
pub fn encadrement(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> (f64, f64) {
    let n = n.max(1);
    let dx = (b - a) / n as f64;
    let mut inf = 0.0;
    let mut sup = 0.0;
    for i in 0..n {
        let x0 = a + i as f64 * dx;
        let vals = [f(x0), f(x0 + dx / 2.0), f(x0 + dx)];
        inf += vals[0].min(vals[1]).min(vals[2]);
        sup += vals[0].max(vals[1]).max(vals[2]);
    }
    (inf * dx, sup * dx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    type F = fn(f64) -> f64;

    #[test]
    fn constantes_et_affines() {
        // constante : exact quelle que soit la méthode
        for m in [
            Methode::Gauche,
            Methode::Droite,
            Methode::Milieu,
            Methode::Trapezes,
        ] {
            for n in [1usize, 5, 37] {
                let s = somme(|_| 3.0, 0.0, 2.0, n, m);
                assert!((s - 6.0).abs() < 1e-9, "{m:?} n={n} : {s}");
            }
        }
        // affine f = 2x sur [0;2] (∫ = 4) : milieu et trapèzes exacts ;
        // les rectangles s'écartent de (f(b)−f(a))·dx/2 = ±4/n
        for n in [1usize, 5, 37] {
            let e = 4.0 / n as f64;
            assert!((somme(|x| 2.0 * x, 0.0, 2.0, n, Methode::Milieu) - 4.0).abs() < 1e-9);
            assert!((somme(|x| 2.0 * x, 0.0, 2.0, n, Methode::Trapezes) - 4.0).abs() < 1e-9);
            assert!((somme(|x| 2.0 * x, 0.0, 2.0, n, Methode::Gauche) - (4.0 - e)).abs() < 1e-9);
            assert!((somme(|x| 2.0 * x, 0.0, 2.0, n, Methode::Droite) - (4.0 + e)).abs() < 1e-9);
        }
    }

    #[test]
    fn ordre_des_methodes_fonction_croissante() {
        // x² sur [0;1] : gauche sous-estime, droite surestime, le
        // point-milieu est entre les deux, les trapèzes aussi
        let g = somme(|x| x * x, 0.0, 1.0, 4, Methode::Gauche);
        let d = somme(|x| x * x, 0.0, 1.0, 4, Methode::Droite);
        let m = somme(|x| x * x, 0.0, 1.0, 4, Methode::Milieu);
        let t = somme(|x| x * x, 0.0, 1.0, 4, Methode::Trapezes);
        assert!(g < 1.0 / 3.0, "{g}");
        assert!(d > 1.0 / 3.0, "{d}");
        assert!(g < m && m < d && t < d, "{g} {m} {t} {d}");
        // le point-milieu est aussi précis que les trapèzes ou plus
        assert!((m - 1.0 / 3.0).abs() <= (t - 1.0 / 3.0).abs());
        // convergence (écart théorique ≈ 1/(2n) pour le rectangle
        // gauche sur x²)
        let exact = somme(|x| x * x, 0.0, 1.0, 10_000, Methode::Gauche);
        assert!((exact - 1.0 / 3.0).abs() < 1e-4, "{exact}");
    }

    #[test]
    fn trapezes_exact_sur_affines_et_sinus() {
        // ∫[-2;5] (3x−1) dx = [3x²/2 − x] = 32.5 − 8 = 24.5 : les
        // trapèzes sont exacts sur une affine, n importe
        for n in [1usize, 12, 40] {
            let s = somme(|x| 3.0 * x - 1.0, -2.0, 5.0, n, Methode::Trapezes);
            assert!((s - 24.5).abs() < 1e-9, "n={n} : {s}");
        }
        // n = 3000 : l'erreur des trapèzes est O(h²) et vaut ici
        // (π/n)²·2·π/12 ; il faut n ≳ 2000 pour passer sous 1e-6
        let sinus = somme(|x| x.sin(), 0.0, PI, 3000, Methode::Trapezes);
        assert!((sinus - 2.0).abs() < 1e-6, "{sinus}");
    }

    #[test]
    fn encadrement_contient_la_valeur() {
        let cas: [(F, f64); 3] = [
            (|x| x * x, 1.0 / 3.0),
            // ∫₀¹ sin(x) dx = 1 − cos(1)
            (|x| x.sin(), 1.0 - 1.0f64.cos()),
            // gaussienne tronquée, décroissante sur [0;1] : le
            // minorant vient des rectangles à droite
            (|x| (-(x * x)).exp(), 0.74682413),
        ];
        for (f, exact) in cas {
            let n = 8;
            let (inf, sup) = encadrement(f, 0.0, 1.0, n);
            assert!(inf <= exact + 1e-6, "inf {inf} > {exact}");
            assert!(sup >= exact - 1e-6, "sup {sup} < {exact}");
            // toute méthode reste dans l'encadrement
            for m in [
                Methode::Gauche,
                Methode::Droite,
                Methode::Milieu,
                Methode::Trapezes,
            ] {
                let v = somme(f, 0.0, 1.0, n, m);
                assert!(inf - 1e-6 <= v && v <= sup + 1e-6, "{m:?} {v} {inf} {sup}");
            }
        }
    }

    #[test]
    fn encadrement_se_serre_quand_n_augmente() {
        let (i1, s1) = encadrement(|x| x.sin(), 0.0, PI, 4);
        let (i2, s2) = encadrement(|x| x.sin(), 0.0, PI, 40);
        assert!(s1 - i1 > s2 - i2, "{s1} {i1} {s2} {i2}");
        // le sommet π/2 est un point de la grille (n multiple de 4) :
        // l'amplitude vaut dx·TV(sin) = (π/40)·2 = π/20
        assert!((s2 - i2 - PI / 20.0).abs() < 1e-9, "{}", s2 - i2);
    }
}
