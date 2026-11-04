//! Table de valeurs (man. p. 96-97, menu Calculs/Table de valeurs) :
//! dix valeurs d'une fonction ou d'une courbe paramétrée, en mode
//! automatique (première valeur + pas, défilement par les flèches) ou
//! manuel (valeurs de x données par l'utilisateur, expressions
//! acceptées — « 3/4 », « pi »...). Les valeurs s'affichent en
//! fractions si possible, et la table peut être dessinée sur la
//! feuille (fond opaque, comme les autres modules statistiques) aussi
//! bien que copiée dans le presse-papier par l'interface.

use crate::curve::{eval_fonction, Curve, CurveKind};
use crate::expr::{self, Node, VarSet};
use crate::stats::Piece;
use serde::{Deserialize, Serialize};

/// Nombre de lignes affichées (man. p. 97 : « la table ne montre que
/// 10 valeurs »).
pub const AFF: usize = 10;

/// Largeur de la colonne des valeurs de x (mm).
const L0: f64 = 24.0;
/// Largeur d'une colonne suivante (mm).
const LC: f64 = 30.0;
/// Hauteur d'une rangée (mm).
const RANGEE: f64 = 4.0;
/// Marge intérieure gauche/droite du cadre (mm).
const PADX: f64 = 1.0;

const NOIR: [u8; 3] = [0, 0, 0];

/// Une ligne de table : `u` = x (fonction) ou t (paramétrée), `v` =
/// x(t) (paramétrée seulement, vide sinon), `w` = y.
#[derive(Debug, Clone, PartialEq)]
pub struct Ligne {
    pub u: String,
    pub v: String,
    pub w: String,
}

/// Table de valeurs liée à une courbe (fonction ou paramétrée).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableValeurs {
    /// index de la courbe visée dans la liste
    pub courbe: usize,
    /// mode automatique (première valeur + pas) sinon manuel
    pub auto: bool,
    /// première valeur (mode automatique)
    pub x0: f64,
    /// pas entre les valeurs (mode automatique)
    pub pas: f64,
    /// valeurs de x en mode manuel, séparées par « ; » (« 3/4 »,
    /// « pi », « sqrt(2) »... acceptés)
    #[serde(default)]
    pub xs_txt: String,
    /// première ligne affichée (défilement par les flèches)
    pub depart: i64,
    /// afficher les valeurs en fractions si possible
    pub fractions: bool,
    /// dessiner la table sur la feuille
    pub dessiner: bool,
    pub titre: String,
    /// position du coin haut gauche sur la feuille (mm)
    pub x_mm: f64,
    pub y_mm: f64,
}

/// Valeur affichée : 6 décimales au plus (troncature), zéros
/// superflus retranchés.
pub fn nombre(v: f64) -> String {
    if v.is_nan() {
        return "?".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "+inf" } else { "-inf" }.into();
    }
    let mut tr = (v * 1e6).trunc() / 1e6;
    if tr == 0.0 {
        tr = 0.0; // absorbe le signe de −0
    }
    let mut s = format!("{tr:.6}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

/// Fraction à plus petit dénominateur ≤ maxden qui représente v
/// (écart < 1e-9) ; None si v est entier, non fini ou sans fraction
/// approchée (π...).
pub fn en_fraction(v: f64, maxden: u32) -> Option<String> {
    if !v.is_finite() || (v - v.round()).abs() < 1e-9 {
        return None;
    }
    for den in 2..=maxden.max(2) {
        let num = (v * den as f64).round();
        if (num / den as f64 - v).abs() < 1e-9 {
            let signe = if num < 0.0 { "-" } else { "" };
            return Some(format!("{signe}{}/{den}", num.abs() as u64));
        }
    }
    None
}

/// Valeur numérique d'une expression constante (« 3/4 », « pi »,
/// « sqrt(2) ») — aucune variable n'est tolérée.
fn eval_constant(e: &str) -> Result<f64, String> {
    let n: Node = expr::parse(e)?;
    n.validate_vars(&[])?;
    let mut unknown = Vec::new();
    let v = n.eval(&VarSet::default(), &mut unknown);
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("valeur non finie : {e}"))
    }
}

impl TableValeurs {
    /// La table vise une courbe paramétrée (3 colonnes t, x, y).
    pub fn est_parametree(&self, curves: &[Curve]) -> bool {
        matches!(
            curves.get(self.courbe).map(|c| &c.kind),
            Some(CurveKind::Parametree { .. })
        )
    }

    /// Évalue la table : AFF lignes au plus, avec les valeurs telles
    /// qu'affichées (fractions si demandé et possible). Erreur si la
    /// courbe est absente, polaire (comme dans l'original) ou si le
    /// mode manuel n'a aucune valeur lisible.
    pub fn lignes(&self, curves: &[Curve]) -> Result<Vec<Ligne>, String> {
        let c = curves
            .get(self.courbe)
            .ok_or_else(|| "table de valeurs : courbe introuvable".to_string())?;
        // valeurs de la 1re colonne (x pour une fonction, t sinon)
        let xs: Vec<f64> = if self.auto {
            (0..AFF)
                .map(|i| self.x0 + (self.depart + i as i64) as f64 * self.pas)
                .collect()
        } else {
            let vals = self
                .xs_txt
                .split([';', '\n'])
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .map(eval_constant)
                .collect::<Result<Vec<f64>, String>>()?;
            let d = (self.depart.max(0) as usize).min(vals.len());
            vals.into_iter().skip(d).take(AFF).collect()
        };
        if xs.is_empty() {
            return Err("table de valeurs : aucune valeur (mode manuel vide ?)".into());
        }
        let fmt = |v: f64| -> String {
            if self.fractions {
                en_fraction(v, 24).unwrap_or_else(|| nombre(v))
            } else {
                nombre(v)
            }
        };
        let mut out = Vec::with_capacity(xs.len());
        match &c.kind {
            CurveKind::Fonction { expr, .. } => {
                for x in xs {
                    let y = eval_fonction(expr, x)?;
                    out.push(Ligne {
                        u: fmt(x),
                        v: String::new(),
                        w: fmt(y),
                    });
                }
            }
            CurveKind::Parametree { xt, yt, .. } => {
                let ax: Node = expr::parse(xt)?;
                ax.validate_vars(&["t"])?;
                let ay: Node = expr::parse(yt)?;
                ay.validate_vars(&["t"])?;
                let mut unknown = Vec::new();
                for t in xs {
                    let vars = VarSet {
                        t: Some(t),
                        ..Default::default()
                    };
                    let xv = ax.eval(&vars, &mut unknown);
                    let yv = ay.eval(&vars, &mut unknown);
                    out.push(Ligne {
                        u: fmt(t),
                        v: fmt(xv),
                        w: fmt(yv),
                    });
                }
            }
            CurveKind::Polaire { .. } => {
                return Err(
                    "table de valeurs : courbes polaires non gérées (comme dans l'original)".into(),
                );
            }
        }
        Ok(out)
    }

    /// Pièces de dessin sur la feuille : fond opaque, cadre, titre,
    /// en-têtes et lignes. Rien si `dessiner` est faux ou si la table
    /// est erronée (l'interface affiche alors l'erreur).
    pub fn draw_list(&self, curves: &[Curve]) -> Vec<Piece> {
        if !self.dessiner {
            return Vec::new();
        }
        let lgs = match self.lignes(curves) {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };
        let param = self.est_parametree(curves);
        // colonnes : (en-tête, largeur)
        let cols: Vec<(&'static str, f64)> = if param {
            vec![("t", L0), ("x", LC), ("y", LC)]
        } else {
            vec![("x", L0), ("y", LC)]
        };
        let w_tot: f64 = cols.iter().map(|(_, w)| *w).sum::<f64>() + 2.0 * PADX;
        let tit_h = if self.titre.is_empty() { 0.0 } else { 5.0 };
        let h_tot = tit_h + RANGEE + lgs.len() as f64 * RANGEE;
        let (x, y) = (self.x_mm, self.y_mm);
        let mut out = vec![Piece::Rect {
            mm: (x, y),
            w: w_tot,
            h: h_tot,
            fill: Some([255, 255, 255]),
            stroke: (NOIR, 0.3),
        }];
        if tit_h > 0.0 {
            out.push(Piece::Texte {
                mm: (x + PADX, y + 3.2),
                txt: self.titre.clone(),
                c: NOIR,
                taille: 2.6,
                ancre: "",
                vertical: false,
                gras: true,
                italique: false,
            });
        }
        // séparateurs de colonnes (traits fins)
        let mut colx = x + PADX;
        let bornes = cols
            .iter()
            .map(|(_, w)| {
                let b = colx;
                colx += *w;
                b
            })
            .collect::<Vec<f64>>();
        for &b in bornes.iter().skip(1) {
            out.push(Piece::Ligne {
                x0: b,
                y0: y,
                x1: b,
                y1: y + h_tot,
                c: NOIR,
                w: 0.3,
            });
        }
        // en-têtes centrés dans leur colonne, séparés des valeurs
        let y_ent = y + tit_h + 3.0;
        for ((h, w), &b) in cols.iter().zip(&bornes) {
            out.push(Piece::Texte {
                mm: (b + w / 2.0, y_ent),
                txt: (*h).to_string(),
                c: NOIR,
                taille: 2.3,
                ancre: "middle",
                vertical: false,
                gras: false,
                italique: true,
            });
        }
        out.push(Piece::Ligne {
            x0: x + PADX,
            y0: y + tit_h + RANGEE,
            x1: x + w_tot - PADX,
            y1: y + tit_h + RANGEE,
            c: NOIR,
            w: 0.3,
        });
        // valeurs de chaque colonne, centrées (en mode fonction, la
        // 2e colonne est y ; en paramétrée : x(t), y(t))
        for (i, lg) in lgs.iter().enumerate() {
            let vals: Vec<&str> = if param {
                vec![&lg.u, &lg.v, &lg.w]
            } else {
                vec![&lg.u, &lg.w]
            };
            for (k, val) in vals.iter().enumerate() {
                out.push(Piece::Texte {
                    mm: (
                        bornes[k] + cols[k].1 / 2.0,
                        y + tit_h + RANGEE + i as f64 * RANGEE + 3.0,
                    ),
                    txt: (*val).to_string(),
                    c: NOIR,
                    taille: 2.2,
                    ancre: "middle",
                    vertical: false,
                    gras: false,
                    italique: false,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;

    /// nombre round-trip rapide sur les affichages.
    #[test]
    fn nombre_tronque_et_recadre() {
        assert_eq!(nombre(2.0), "2");
        assert_eq!(nombre(0.6816387), "0.681638");
        assert_eq!(nombre(-1.5), "-1.5");
        assert_eq!(nombre(-0.0000001), "0");
        assert_eq!(nombre(f64::NAN), "?");
        assert_eq!(nombre(1.0 / 0.0), "+inf");
    }

    #[test]
    fn fractions_denominateur_24() {
        assert_eq!(en_fraction(0.25, 24).as_deref(), Some("1/4"));
        assert_eq!(en_fraction(2.0 / 3.0, 24).as_deref(), Some("2/3"));
        assert_eq!(en_fraction(0.7, 24).as_deref(), Some("7/10"));
        assert_eq!(en_fraction(-0.25, 24).as_deref(), Some("-1/4"));
        // entier, π : pas de fraction
        assert_eq!(en_fraction(3.0, 24), None);
        assert_eq!(en_fraction(-4.0, 24), None);
        assert_eq!(en_fraction(std::f64::consts::PI, 24), None);
        // 1/24 : accepté ; 1/25 : refuse
        assert_eq!(en_fraction(1.0 / 24.0, 24).as_deref(), Some("1/24"));
        assert_eq!(en_fraction(1.0 / 25.0, 24), None);
    }

    #[test]
    fn mode_auto_x0_pas_et_defilement() {
        let c = Curve::fonction("x^2", [0, 0, 204]);
        let t = TableValeurs {
            courbe: 0,
            auto: true,
            x0: -1.0,
            pas: 0.5,
            depart: 0,
            ..Default::default()
        };
        let lgs = t.lignes(std::slice::from_ref(&c)).unwrap();
        assert_eq!(lgs.len(), AFF);
        assert_eq!(lgs[0].u, "-1");
        assert_eq!(lgs[0].w, "1");
        assert_eq!(lgs[1].u, "-0.5");
        assert_eq!(lgs[9].u, "3.5");
        // défilement : depart = -5 remonte de 5 lignes
        let mut t2 = t.clone();
        t2.depart = -5;
        let lgs = t2.lignes(std::slice::from_ref(&c)).unwrap();
        assert_eq!(lgs[0].u, "-3.5");
    }

    #[test]
    fn mode_manuel_avec_expressions_et_fractions() {
        let c = Curve::fonction("x/2", [0, 0, 204]);
        let mut t = TableValeurs {
            courbe: 0,
            auto: false,
            xs_txt: "0 ; 1/4 ; 3/4 ; -0.5 ; 0.125".into(),
            fractions: false,
            ..Default::default()
        };
        let lgs = t.lignes(std::slice::from_ref(&c)).unwrap();
        assert_eq!(lgs.len(), 5);
        assert_eq!(lgs[0].u, "0");
        assert_eq!(lgs[0].w, "0");
        assert_eq!(lgs[1].u, "0.25");
        assert_eq!(lgs[2].u, "0.75");
        assert_eq!(lgs[3].u, "-0.5");
        // les expressions constantes sont acceptées (« pi », « sqrt(2)
        // »), les variables refusées
        t.xs_txt = "pi/2 ; sqrt(2)".into();
        let lgs = t.lignes(std::slice::from_ref(&c)).unwrap();
        assert_eq!(lgs.len(), 2);
        assert_eq!(lgs[0].u, "1.570796"); // pi/2, tronqué
        assert_eq!(lgs[1].u, "1.414213"); // sqrt(2), tronqué
                                          // fractions : les demi- et quart-entiers s'affichent en
                                          // fractions
        t.xs_txt = "1/4 ; -0.5".into();
        t.fractions = true;
        let lgs = t.lignes(std::slice::from_ref(&c)).unwrap();
        assert_eq!(lgs[0].u, "1/4");
        assert_eq!(lgs[0].w, "1/8");
        assert_eq!(lgs[1].u, "-1/2");
        assert_eq!(lgs[1].w, "-1/4");
    }

    #[test]
    fn courbe_parametree_trois_colonnes() {
        let c = Curve::with_kind(
            CurveKind::Parametree {
                xt: "t".into(),
                yt: "2*t".into(),
                tmin: 0.0,
                tmax: 3.0,
            },
            [0, 0, 204],
        );
        let t = TableValeurs {
            courbe: 0,
            auto: true,
            x0: 0.0,
            pas: 1.0,
            ..Default::default()
        };
        assert!(t.est_parametree(std::slice::from_ref(&c)));
        let lgs = t.lignes(std::slice::from_ref(&c)).unwrap();
        assert_eq!(lgs[0].u, "0");
        assert_eq!(lgs[0].v, "0");
        assert_eq!(lgs[0].w, "0");
        assert_eq!(lgs[2].v, "2");
        assert_eq!(lgs[2].w, "4");
    }

    #[test]
    fn erreurs_proprietes() {
        // courbe absente
        let t = TableValeurs {
            courbe: 3,
            ..Default::default()
        };
        assert!(t.lignes(&[Curve::fonction("x", [0, 0, 0])]).is_err());
        // polaire : non gérée, comme dans l'original
        let p = Curve::with_kind(
            CurveKind::Polaire {
                r: "1".into(),
                tmin: 0.0,
                tmax: 1.0,
            },
            [0, 0, 0],
        );
        let t = TableValeurs {
            courbe: 0,
            auto: true,
            ..Default::default()
        };
        assert!(t.lignes(std::slice::from_ref(&p)).is_err());
        // mode manuel vide
        let t = TableValeurs {
            courbe: 0,
            auto: false,
            xs_txt: "  ;  ".into(),
            ..Default::default()
        };
        assert!(t.lignes(&[Curve::fonction("x", [0, 0, 0])]).is_err());
        // expression avec variable : refuse (valeurs constantes)
        let t = TableValeurs {
            courbe: 0,
            auto: false,
            xs_txt: "x/2".into(),
            ..Default::default()
        };
        assert!(t.lignes(&[Curve::fonction("x", [0, 0, 0])]).is_err());
    }

    #[test]
    fn dessin_fond_et_textes() {
        let c = Curve::fonction("x^2", [0, 0, 204]);
        let t = TableValeurs {
            titre: "x^2".into(),
            auto: true,
            x0: 0.0,
            pas: 0.25,
            fractions: true,
            dessiner: true,
            x_mm: 80.0,
            y_mm: 8.0,
            ..Default::default()
        };
        let pieces = t.draw_list(std::slice::from_ref(&c));
        assert!(!pieces.is_empty());
        // fond opaque plein + titre + en-têtes + 10 rangées × 2 colonnes
        assert!(pieces.len() >= 2 + 2 + 10 * 2);
        assert!(matches!(pieces[0], Piece::Rect { fill: Some(..), .. }));
        let txts: Vec<&str> = pieces
            .iter()
            .filter_map(|pi| match pi {
                Piece::Texte { txt, .. } => Some(txt.as_str()),
                _ => None,
            })
            .collect();
        assert!(txts.contains(&"x^2"));
        assert!(txts.contains(&"x"));
        assert!(txts.contains(&"y"));
        assert!(txts.contains(&"1/4")); // valeur fractionnaire dessinée
                                        // dessin désactivé ou table erronée → rien
        let mut t2 = t.clone();
        t2.dessiner = false;
        assert!(t2.draw_list(std::slice::from_ref(&c)).is_empty());
        t2.dessiner = true;
        t2.courbe = 5;
        assert!(t2.draw_list(std::slice::from_ref(&c)).is_empty());
    }
}
