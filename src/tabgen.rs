//! Génération automatique d'un tableau de variations depuis une courbe
//! de type fonction f(x) : zéros de f′ (balayage + bissection), signes de
//! f′, valeurs de f aux points critiques et aux bornes, flèches — le tout
//! sérialisé dans le code du module tableau (éditable ensuite).

use crate::expr::{Node, VarSet};

/// Subdivisions du balayage des zéros de f′.
const N_BALAYAGE: usize = 600;

/// Évalue f en x (NaN, ±∞, hors domaine → None).
fn f_en(ast: &Node, x: f64) -> Option<f64> {
    let v = ast.eval(
        &VarSet {
            x: Some(x),
            ..Default::default()
        },
        &mut Vec::new(),
    );
    v.is_finite().then_some(v)
}

/// Dérivée numérique (différences centrées).
fn f_prime(ast: &Node, x: f64) -> Option<f64> {
    let h = 1e-6 * (1.0 + x.abs());
    let ap = f_en(ast, x + h)?;
    let am = f_en(ast, x - h)?;
    Some((ap - am) / (2.0 * h))
}

/// Zéros de f′ dans [a, b] : balayage des changements de signe puis
/// bissection (60 itérations), dédupliqués.
pub fn zeros_f_prime(ast: &Node, a: f64, b: f64) -> Vec<f64> {
    let n = N_BALAYAGE.max(32);
    let pas = (b - a) / n as f64;
    let mut racines: Vec<f64> = Vec::new();
    let mut x0 = a;
    let mut f0 = f_prime(ast, x0).unwrap_or(f64::NAN);
    for k in 1..=n {
        let x1 = a + pas * k as f64;
        let f1 = f_prime(ast, x1).unwrap_or(f64::NAN);
        if f0.is_finite() && f1.is_finite() && (f0 * f1 < 0.0 || f1 == 0.0) {
            let (mut lo, mut hi) = (x0, x1);
            let mut flo = f0;
            for _ in 0..60 {
                let m = 0.5 * (lo + hi);
                let Some(fm) = f_prime(ast, m) else {
                    break;
                };
                if fm == 0.0 {
                    lo = m;
                    hi = m;
                    break;
                }
                if hi - lo < 1e-12 * (1.0 + m.abs()) {
                    break;
                }
                if flo * fm > 0.0 {
                    lo = m;
                    flo = fm;
                } else {
                    hi = m;
                }
            }
            let r = 0.5 * (lo + hi);
            if !racines
                .iter()
                .any(|&q| (q - r).abs() < 1e-7 * (1.0 + (b - a)))
            {
                racines.push(r);
            }
        }
        x0 = x1;
        f0 = f1;
    }
    racines.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
    racines
}

/// Formate une valeur pour le tableau : entier simple, sinon décimal
/// (3 décimales max, zéros superflus supprimés) ; virgule décimale en
/// français.
pub fn fmt_val(v: f64, fr: bool) -> String {
    if v.abs() > 1e15 {
        return String::new();
    }
    if (v - v.round()).abs() < 1e-9 {
        return format!("{}", v.round() as i64);
    }
    let mut s = format!("{v:.3}");
    s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s == "-0" {
        s = "0".to_string();
    }
    if fr {
        s.replace('.', ",")
    } else {
        s
    }
}

/// Ligne d'espaces de la largeur d'un champ de case (lisibilité du code).
fn pad(s: &str) -> String {
    format!("{s:<8}")
}

/// Génère le code du tableau de variations de `expr` sur [a, b].
/// `fr` = virgule décimale (convention française).
pub fn genere(expr: &str, a: f64, b: f64, fr: bool) -> Result<String, String> {
    if b <= a {
        return Err("intervalle vide (b ≤ a)".to_string());
    }
    let ast = crate::expr::parse(expr)?;
    let racines = zeros_f_prime(&ast, a, b);

    // signe de f′ et flèche sur chaque intervalle entre frontières
    let mut bornes: Vec<f64> = vec![a];
    bornes.extend(racines.iter().copied());
    bornes.push(b);
    let mut signes: Vec<Option<bool>> = Vec::new(); // true = +
    for w in bornes.windows(2) {
        let m = 0.5 * (w[0] + w[1]);
        signes.push(f_prime(&ast, m).map(|d| d > 0.0));
    }
    let fleches: Vec<&str> = signes
        .iter()
        .map(|s| match s {
            Some(true) => "/:>",
            Some(false) => "\\:>",
            None => "",
        })
        .collect();

    // valeurs de f aux frontières et aux bornes
    let v0 = f_en(&ast, a).map(|v| fmt_val(v, fr)).unwrap_or_default();
    let vfin = f_en(&ast, b).map(|v| fmt_val(v, fr)).unwrap_or_default();
    let vcrit: Vec<String> = racines
        .iter()
        .map(|&r| f_en(&ast, r).map(|v| fmt_val(v, fr)).unwrap_or_default())
        .collect();

    // --- ligne x : 1 case par intervalle, n frontières internes ---
    let n = racines.len();
    let mut lx = String::from("[:x       :]||[");
    lx.push_str(&pad(&fmt_val(a, fr)));
    if n == 0 {
        lx.push_str(":        :");
        lx.push_str(&pad(&fmt_val(b, fr)));
        lx.push(']');
    } else {
        lx.push_str(":        :]");
        for (k, r) in racines.iter().enumerate() {
            lx.push_str(&fmt_val(*r, fr));
            if k + 1 < n {
                lx.push_str("[:        :]");
            }
        }
        lx.push_str("[:        :");
        lx.push_str(&pad(&fmt_val(b, fr)));
        lx.push(']');
    }

    // --- ligne f' : signes, zéros de f' sur les barres ---
    let sg = |s: &Option<bool>| match s {
        Some(true) => "+",
        Some(false) => "-",
        None => "",
    };
    let mut ld = String::from("[:f '(x) :]||[         :");
    ld.push_str(sg(&signes[0]));
    ld.push_str("  :]");
    for s in &signes[1..] {
        ld.push_str("|0[:      :");
        ld.push_str(sg(s));
        ld.push_str("  :]");
    }

    // --- ligne f(x) : flèches, valeurs aux frontières ---
    let mut lf = String::from("[:f(x)    :]||[");
    lf.push_str(&pad(&v0));
    lf.push(':');
    lf.push_str(fleches[0]);
    if n == 0 {
        lf.push(':');
        lf.push_str(&vfin);
    }
    lf.push(']');
    for k in 1..n {
        lf.push_str(&vcrit[k - 1]);
        lf.push_str("[        :");
        lf.push_str(fleches[k]);
        lf.push_str(":]");
    }
    if n >= 1 {
        lf.push_str(&vcrit[n - 1]);
        lf.push_str("[        :");
        lf.push_str(fleches[n]);
        lf.push(':');
        lf.push_str(&vfin);
        lf.push(']');
    }

    Ok(format!("{lx}\n{ld}\n{lf}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tableau;

    #[test]
    fn parabole_sur_moins3_3() {
        // f(x) = x^2-2 : f' = 2x, zéro en 0, − puis +, f(0) = -2
        let code = genere("x^2-2", -3.0, 3.0, true).unwrap();
        let t = tableau::parse(&code).unwrap();
        assert_eq!(t.rows.len(), 3);
        // 1 zéro de f' → étiquette + 2 cases
        assert!(t.rows.iter().all(|r| r.cases.len() == 3));
        // la ligne x porte le zéro : « 0 » nu = séparateur ZeroNu (rendu
        // identique à la frontière), ou valeur frontière
        assert!(
            t.rows[0].cases[2].valeur_frontiere.as_deref() == Some("0")
                || t.rows[0].cases[2].sep == tableau::Sep::ZeroNu
        );
        // f(0) = -2 posé sur la frontière de la ligne f(x)
        assert!(t.rows[2].cases[2].valeur_frontiere.as_deref() == Some("-2"));
        // le code doit se re-analyser et contenir les signes
        assert!(code.contains("-") && code.contains("+"));
        assert!(code.contains("/:>") && code.contains("\\:>"));
    }

    #[test]
    fn cube_sans_zero_de_prime() {
        // f(x) = x^3 : f' = 3x² ≥ 0, aucun changement de signe
        let code = genere("x^3", -1.0, 2.0, false).unwrap();
        let t = tableau::parse(&code).unwrap();
        assert_eq!(t.rows.len(), 3);
        // étiquette + 1 case (aucune frontière interne)
        assert!(t.rows.iter().all(|r| r.cases.len() == 2));
        // montante partout (une flèche par cellule : étiquette + 1 case)
        assert!(code.matches("/:>").count() == 1 && !code.contains("\\:>"));
    }

    #[test]
    fn sinus_zeros_multiples() {
        // f' = cos(x) : zéros en -π/2, π/2 sur [-4, 4] → étiquette + 3 cases
        let code = genere("sin(x)", -4.0, 4.0, true).unwrap();
        let t = tableau::parse(&code).unwrap();
        assert!(t.rows.iter().all(|r| r.cases.len() == 4));
        // valeurs de f aux critiques : ±1
        let vals: Vec<String> = t.rows[2]
            .cases
            .iter()
            .filter_map(|c| c.valeur_frontiere.clone())
            .collect();
        assert!(vals.contains(&"-1".to_string()) && vals.contains(&"1".to_string()));
    }

    #[test]
    fn virgule_et_formatage() {
        assert_eq!(fmt_val(2.5, true), "2,5");
        assert_eq!(fmt_val(2.5, false), "2.5");
        assert_eq!(fmt_val(-0.0001, true), "0");
        assert_eq!(fmt_val(1.0 / 3.0, true), "0,333");
        assert_eq!(fmt_val(7.0, false), "7");
    }
}
