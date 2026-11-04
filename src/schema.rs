//! Schéma géométrique (man. p. 27-44 ; socle v1.3) : points nommés et
//! courbes de Bézier à 4 points de contrôle. Le reste du module Schéma
//! de l'original (segments, vecteurs, droites, cercles, ellipses,
//! polygones, angles, zones coloriées, coniques…) s'appuiera sur ce
//! socle des objets dans une version ultérieure.
//!
//! Comme dans l'original, une courbe de Bézier cite des points définis
//! avant elle par leur nom ; les tangentes aux extrémités sont les
//! droites (AB) et (CD). Les points ne sont pas déplacés à la souris
//! ici (coordonnées saisies dans la fenêtre, comme les annotations v0.2).

use crate::grid::Page;
use crate::stats::Piece;
use serde::{Deserialize, Serialize};

/// Nombre d'échantillons de la polyligne de Bézier.
const N_ECH: usize = 80;

const NOIR: [u8; 3] = [0, 0, 0];

/// Point du schéma : nom + coordonnées utilisateur (man. p. 28).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PointSchema {
    pub nom: String,
    pub x: f64,
    pub y: f64,
}

/// Courbe de Bézier cubique définie par 4 points nommés (man. p. 38) :
/// A départ, B et C points de contrôle (tangentes (AB) à A et (CD)
/// en D), D arrivée. Tierce version (courbe définie point par point,
/// man. p. 22) à venir sur ce même socle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bezier {
    pub a: String,
    pub b: String,
    pub c: String,
    pub d: String,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
}

fn def_couleur() -> [u8; 3] {
    [0, 0, 0]
}

fn def_epaisseur() -> f64 {
    2.0
}

impl Default for Bezier {
    fn default() -> Self {
        Bezier {
            a: String::new(),
            b: String::new(),
            c: String::new(),
            d: String::new(),
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
        }
    }
}

/// Objet du schéma (les points d'abord, comme l'original exige que les
/// points soient définis avant la courbe qui les utilise).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShObj {
    Pt(PointSchema),
    Bez(Bezier),
}

impl PointSchema {
    /// Point sur la feuille (croix + nom italique, comme le bouton
    /// « Définir un point » de l'original).
    pub fn draw_list(&self, p: &Page) -> Vec<Piece> {
        let mut out = Vec::new();
        let x = p.mm_x(self.x);
        let y = p.mm_y(self.y);
        if !x.is_finite() || !y.is_finite() {
            return out;
        }
        let r = 0.8;
        out.push(Piece::Ligne {
            x0: x - r,
            y0: y,
            x1: x + r,
            y1: y,
            c: NOIR,
            w: 0.35,
        });
        out.push(Piece::Ligne {
            x0: x,
            y0: y - r,
            x1: x,
            y1: y + r,
            c: NOIR,
            w: 0.35,
        });
        if !self.nom.is_empty() {
            out.push(Piece::Texte {
                mm: (x + 0.9, y + 0.6),
                txt: self.nom.clone(),
                c: NOIR,
                taille: 2.3,
                ancre: "",
                vertical: false,
                italique: true,
                gras: false,
            });
        }
        out
    }
}

impl Bezier {
    /// Point de la courbe au paramètre `t` (polynômes de Bernstein) ;
    /// `p` : position des 4 points dans l'ordre A, B, C, D.
    pub fn point_a_t(pts: &[(f64, f64); 4], t: f64) -> (f64, f64) {
        let u = 1.0 - t;
        let b0 = u * u * u;
        let b1 = 3.0 * t * u * u;
        let b2 = 3.0 * t * t * u;
        let b3 = t * t * t;
        (
            b0 * pts[0].0 + b1 * pts[1].0 + b2 * pts[2].0 + b3 * pts[3].0,
            b0 * pts[0].1 + b1 * pts[1].1 + b2 * pts[2].1 + b3 * pts[3].1,
        )
    }

    /// Résolution des 4 points nommés dans la liste du schéma (ordre
    /// A, B, C, D) ; None si un des noms manque.
    pub fn resolve(&self, pts: &[ShObj]) -> Option<[(f64, f64); 4]> {
        let cherche = |nom: &str| {
            pts.iter().find_map(|o| match o {
                ShObj::Pt(pt) if pt.nom == nom => Some((pt.x, pt.y)),
                _ => None,
            })
        };
        Some([
            cherche(&self.a)?,
            cherche(&self.b)?,
            cherche(&self.c)?,
            cherche(&self.d)?,
        ])
    }

    /// Courbe tracée de A à D (Polyligne Poly en mm).
    pub fn draw_list_with(&self, p: &Page, pts: &[(f64, f64); 4]) -> Vec<Piece> {
        let mut mm = Vec::with_capacity(N_ECH + 1);
        for i in 0..=N_ECH {
            let t = i as f64 / N_ECH as f64;
            let (x, y) = Self::point_a_t(pts, t);
            mm.push((p.mm_x(x), p.mm_y(y)));
        }
        if mm.iter().any(|q| !q.0.is_finite() || !q.1.is_finite()) {
            return Vec::new();
        }
        vec![Piece::Poly {
            pts: mm,
            fill: None,
            stroke: Some((self.couleur, self.epaisseur)),
        }]
    }

    pub fn draw_list(&self, p: &Page, pts: &[ShObj]) -> Vec<Piece> {
        match self.resolve(pts) {
            Some(q) => self.draw_list_with(p, &q),
            None => Vec::new(),
        }
    }
}

impl ShObj {
    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        match self {
            ShObj::Pt(pt) => pt.draw_list(p),
            ShObj::Bez(b) => b.draw_list(p, schema),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Piece as PieceS;

    fn pts() -> Vec<ShObj> {
        let mk = |nom: &str, x: f64, y: f64| {
            ShObj::Pt(PointSchema {
                nom: nom.into(),
                x,
                y,
            })
        };
        vec![
            mk("A", 0.0, 0.0),
            mk("B", 1.0, 1.0),
            mk("C", 2.0, -1.0),
            mk("D", 3.0, 0.0),
        ]
    }

    #[test]
    fn ponto_a_t_bordures_et_milieu() {
        let q = [(0.0, 0.0), (1.0, 1.0), (2.0, -1.0), (3.0, 0.0)];
        let (x, y) = Bezier::point_a_t(&q, 0.0);
        assert_eq!((x, y), q[0]);
        let (x, y) = Bezier::point_a_t(&q, 1.0);
        assert_eq!((x, y), q[3]);
        // t = 1/2 : B(1/2) = (A + 3·B + 3·C + D)/8, y = 0 car les
        // contributions 3/8·1 (point B) et 3/8·(−1) (point C) s'annulent
        let (x, y) = Bezier::point_a_t(&q, 0.5);
        assert!((x - 1.5).abs() < 1e-9);
        assert!(y.abs() < 1e-9, "{y}");
    }

    #[test]
    fn tangente_a_departs() {
        // dérivée en t=0 colinéaire à (B−A), en t=1 à (D−C) : on compare
        // par le produit vectoriel planaire (nul ⇔ colinéarité)
        let q = [(0.0, 0.0), (1.0, 2.0), (2.0, 0.0), (1.0, -1.0)];
        let h = 1e-4;
        let (x0, y0) = Bezier::point_a_t(&q, h);
        let d0 = (x0 - q[0].0, y0 - q[0].1);
        let ab = (q[1].0 - q[0].0, q[1].1 - q[0].1);
        assert!((d0.0 * ab.1 - d0.1 * ab.0).abs() < 1e-3, "{d0:?} vs {ab:?}");
        let (x1, y1) = Bezier::point_a_t(&q, 1.0 - h);
        let d1 = (q[3].0 - x1, q[3].1 - y1);
        let dc = (q[3].0 - q[2].0, q[3].1 - q[2].1);
        assert!((d1.0 * dc.1 - d1.1 * dc.0).abs() < 1e-3, "{d1:?} vs {dc:?}");
    }

    #[test]
    fn resolution_par_noms() {
        let schema = pts();
        let b = Bezier {
            a: "A".into(),
            b: "B".into(),
            c: "C".into(),
            d: "Z".into(),
            ..Default::default()
        };
        assert!(b.resolve(&schema).is_none());
        let b = Bezier {
            a: "A".into(),
            b: "B".into(),
            c: "C".into(),
            d: "D".into(),
            ..Default::default()
        };
        let q = b.resolve(&schema).unwrap();
        assert_eq!(q[3], (3.0, 0.0));
    }

    #[test]
    fn croix_et_courbe_pieces() {
        let p = Page::default();
        // Page::default : repère centré sur A4 ? Les coordonnées de test
        // utilisent l'unité du repère : on vérifie seulement la nature
        let mut a = PointSchema {
            nom: "A".into(),
            x: 1.0,
            y: 2.0,
        };
        let pieces = a.draw_list(&p);
        assert_eq!(pieces.len(), 3); // 2 segments de la croix + nom
        assert!(matches!(pieces[0], PieceS::Ligne { .. }));
        let PieceS::Texte { txt, italique, .. } = &pieces[2] else {
            unreachable!("le 3e morceau est le nom")
        };
        assert_eq!(txt, "A");
        assert!(italique);
        let schema = pts();
        let b = Bezier {
            a: "A".into(),
            b: "B".into(),
            c: "C".into(),
            d: "D".into(),
            ..Default::default()
        };
        let pieces = b.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 1);
        let PieceS::Poly { pts, .. } = &pieces[0] else {
            unreachable!()
        };
        assert_eq!(pts.len(), N_ECH + 1);
        // point manquant → rien n'est dessiné
        let b = Bezier {
            a: "A".into(),
            b: "B".into(),
            c: "C".into(),
            d: "X".into(),
            ..Default::default()
        };
        assert!(b.draw_list(&p, &schema).is_empty());
        a.nom = String::new();
        // point sans nom : croix seule
        assert_eq!(a.draw_list(&p).len(), 2);
    }
}
