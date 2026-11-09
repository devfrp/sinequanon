//! Courbes définies point par point (man. p. 22-24) : jusqu'à 20
//! points joints dans l'ordre, reliés par des arcs de Bézier cubiques.
//! Deux méthodes (qui se mélangent point à point) :
//! • pente saisie en un point = nombre dérivé : la tangente y = p·(x−xᵢ)
//!   est respectée (schéma de Hermite, man. p. 23) ;
//! • pente vide : interpolation automatique (Catmull-Rom scalé par
//!   lissage/50, défaut 50, man. p. 24). Si le dernier point est le
//!   même que le premier, la courbe est fermée.

use serde::{Deserialize, Serialize};

/// Un point de la courbe : coordonnées et pente (vide = automatique).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PpkPt {
    pub x: f64,
    pub y: f64,
    /// nombre dérivé en ce point ; chaîne vide = interpolation auto
    #[serde(default)]
    pub pente: String,
}

impl PpkPt {
    pub fn nouveau(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            pente: String::new(),
        }
    }
}

/// Nombre maximal de points d'une courbe point par point (man. p. 23).
pub const MAX_PTS: usize = 20;

/// Degré 3 de de Casteljau : échantillonner un arc de Bézier cubique.
fn arc_bezier(
    p0: (f64, f64),
    c1: (f64, f64),
    c2: (f64, f64),
    p1: (f64, f64),
    n: usize,
) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(n + 1);
    for k in 0..=n {
        let t = k as f64 / n as f64;
        let s = 1.0 - t;
        let x =
            s * s * s * p0.0 + 3.0 * s * s * t * c1.0 + 3.0 * s * t * t * c2.0 + t * t * t * p1.0;
        let y =
            s * s * s * p0.1 + 3.0 * s * s * t * c1.1 + 3.0 * s * t * t * c2.1 + t * t * t * p1.1;
        out.push((x, y));
    }
    out
}

/// Pente saisie en un point, NaN si vide ou invalide.
fn pente_de(p: &PpkPt) -> f64 {
    p.pente.trim().parse::<f64>().unwrap_or(f64::NAN)
}

/// Liste des arcs (coordonnées utilisateur) de la courbe point par
/// point : témoins exacts aux points de la liste, tangente respectée
/// là où une pente est saisie. `lissage` : 0 = lignes brisées ;
/// 50 (défaut manuel p. 24) = Catmull-Rom ; plus grand = plus rond.
pub fn arcs(pts: &[PpkPt], lissage: f64) -> Vec<Vec<(f64, f64)>> {
    if pts.len() < 2 {
        return Vec::new();
    }
    // courbe fermée : le dernier point reprend le premier (man. p. 24)
    let fermee = (pts[0].x - pts[pts.len() - 1].x).abs() < 1e-9
        && (pts[0].y - pts[pts.len() - 1].y).abs() < 1e-9;
    // points uniques dans l'ordre cyclique
    let uniques: Vec<&PpkPt> = if fermee {
        pts[..pts.len() - 1].iter().collect()
    } else {
        pts.iter().collect()
    };
    let n = uniques.len();
    // voisin i−1 et i+1 (cyclique si fermée, arrêté sinon)
    let gauche = |i: usize| -> Option<usize> {
        if i == 0 {
            if fermee {
                Some(n - 1)
            } else {
                None
            }
        } else {
            Some(i - 1)
        }
    };
    let droite = |i: usize| -> Option<usize> {
        if i + 1 >= n {
            if fermee {
                Some(0)
            } else {
                None
            }
        } else {
            Some(i + 1)
        }
    };
    // tangente automatique en i (vecteur, man. p. 24) : Catmull-Rom
    // central scalé par lissage/50 (50 : coefficient (P[i+1]−P[i−1])/2)
    let l = lissage.clamp(0.0, 500.0) / 50.0;
    let mut tangente_auto: Vec<Option<(f64, f64)>> = Vec::with_capacity(n);
    for i in 0..n {
        let tg = match (gauche(i), droite(i)) {
            (Some(a), Some(b)) => Some((
                (uniques[b].x - uniques[a].x) * l / 2.0,
                (uniques[b].y - uniques[a].y) * l / 2.0,
            )),
            _ => None,
        };
        tangente_auto.push(tg);
    }
    // tangente en i : saisie (1, p)·Δ/3 si pente donnée, sinon auto.
    // Pour l'arc i→j, on a besoin de la direction au départ et à
    // l'arrivée ; la saisie donne (1, pente) direction pente·Δx.
    let mut arcs: Vec<Vec<(f64, f64)>> = Vec::new();
    for i in 0..n {
        let j = if i + 1 >= n { 0 } else { i + 1 };
        if !fermee && j == 0 {
            break;
        }
        let a = uniques[i];
        let b = uniques[j];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        // intervalle dégénéré (crochet de fermeture explicite)
        if dx.abs() < 1e-12 && dy.abs() < 1e-12 {
            continue;
        }
        let t_e = if let Some(m) = {
            let v = pente_de(a);
            v.is_finite().then_some(v)
        } {
            (dx / 3.0, m * dx / 3.0)
        } else {
            tangente_auto[i].unwrap_or((dx / 3.0, dy / 3.0))
        };
        let t_s = if let Some(m) = {
            let v = pente_de(b);
            v.is_finite().then_some(v)
        } {
            (dx / 3.0, m * dx / 3.0)
        } else {
            tangente_auto[j].unwrap_or((dx / 3.0, dy / 3.0))
        };
        let c1 = (a.x + t_e.0, a.y + t_e.1);
        let c2 = (b.x - t_s.0, b.y - t_s.1);
        // points non triés en x (courbe fermée) : la saisie pente·Δx
        // reste valable ; l'auto Catmull-Rom ne suppose pas trié.
        arcs.push(arc_bezier((a.x, a.y), c1, c2, (b.x, b.y), 16));
    }
    arcs
}

/// Polylignes (mm) de la courbe, hors page et limites écartées.
pub fn polys_mm(
    pts: &[PpkPt],
    lissage: f64,
    mm_x: impl Fn(f64) -> f64 + Copy,
    mm_y: impl Fn(f64) -> f64 + Copy,
) -> Vec<Vec<(f64, f64)>> {
    arcs(pts, lissage)
        .into_iter()
        .map(|arc| {
            arc.into_iter()
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .map(|(x, y)| (clamp_big(mm_x(x)), clamp_big(mm_y(y))))
                .collect()
        })
        .filter(|p: &Vec<(f64, f64)>| p.len() >= 2)
        .collect()
}

fn clamp_big(v: f64) -> f64 {
    v.clamp(-1e6, 1e6)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64, pente: &str) -> PpkPt {
        PpkPt {
            x,
            y,
            pente: pente.into(),
        }
    }

    #[test]
    fn pentes_1_droite_identite() {
        // tous les points de pente 1 sur la diagonale : la courbe est
        // la droite y = x (man. p. 23, exemple carré de pente)
        let pts = [pt(0.0, 0.0, "1"), pt(1.0, 1.0, "1"), pt(2.0, 2.0, "1")];
        let arcs = arcs(&pts, 50.0);
        assert_eq!(arcs.len(), 2);
        for arc in &arcs {
            for &(x, y) in arc {
                assert!((y - x).abs() < 1e-9, "x={x} y={y}");
            }
        }
    }

    #[test]
    fn pentes_c_moyennes_egales_aux_temoins() {
        // les milieux des arcs coïncident avec la spline de Hermite :
        // au milieu de [0,1] de y=x² la valeur attendue est 0.25
        let pts = [pt(0.0, 0.0, "0"), pt(1.0, 1.0, "2"), pt(2.0, 4.0, "4")];
        let arcs = arcs(&pts, 50.0);
        // milieu du premier arc : contrôle c1 = (1/3, 0), c2 = (2/3, 1−2/3)
        // t=0.5 : x = 0.5 ; y = (0·0.125 + 1·0.375 + 1·0.375? recompute:
        let arc = &arcs[0];
        let (_, y) = arc[8]; // t = 0.5
                             // Hermite exact : y(0.5) = 0.125·0 + 0.375·0·0.5 + 0.375·2·0.5 + 0.125·1
                             // = 0 + 0.375 + 0.125 = 0.5? On recalcule : h(x)= m0·x + (3 −
                             // 2m0 − m1)x² + (m0 + m1 − 2)x³ pour y(0)=0 y(1)=1
                             // m0=0, m1=2 : y = (3−2)·x²? = x² + 0·x³ → y(0.5) = 0.25
        assert!((y - 0.25).abs() < 1e-9, "y={y}");
        // pente au milieu : passe par 0.5·? dérivée en 0.5 vaut (3−2)x² …
    }

    #[test]
    fn auto_passe_par_les_temoins() {
        // interpolation automatique : la courbe passe exactement par
        // les points de la liste (extrémités des arcs de Bézier)
        let pts = [
            pt(0.0, 0.0, ""),
            pt(1.0, 2.0, ""),
            pt(3.0, -1.0, ""),
            pt(2.0, 1.0, ""),
        ];
        let arcs = arcs(&pts, 50.0);
        assert_eq!(arcs.len(), 3, "3 intervalles, ordre de la liste");
        assert_eq!(arcs[0][0], (0.0, 0.0));
        assert_eq!(*arcs[0].last().unwrap(), (1.0, 2.0));
        assert_eq!(*arcs[2].last().unwrap(), (2.0, 1.0));
        // lissage 0 : arcs rectilignes (témoins alignés sur les cordes)
        let l0 = super::arcs(&pts, 0.0);
        for &(x, y) in &l0[0] {
            assert!((y - 2.0 * x).abs() < 1e-9, "x={x} y={y}");
        }
        assert_eq!(l0[0].len(), 17, "16 subdivisions + 1");
    }

    #[test]
    fn fermee_retrouve_le_premier_point() {
        // courbe fermée : le dernier point reprend le premier,
        // l'arc de fermeture retombe exactement sur lui
        let pts = [
            pt(0.0, 0.0, ""),
            pt(1.0, 0.0, ""),
            pt(1.0, 1.0, ""),
            pt(0.0, 0.0, ""),
        ];
        let arcs = arcs(&pts, 50.0);
        assert_eq!(arcs.len(), 3, "3 uniques → 3 arcs (fermeture incluse)");
        assert_eq!(*arcs[2].last().unwrap(), (0.0, 0.0));
    }

    #[test]
    fn mm_via_la_page() {
        let page = crate::grid::Page::default();
        let pts = [pt(0.0, 0.0, "1"), pt(2.0, 2.0, "1")];
        let polys = polys_mm(&pts, 50.0, |x| page.mm_x(x), |y| page.mm_y(y));
        assert_eq!(polys.len(), 1);
        // (0 ; 0) → centre de page (85 ; 60) puis (2 ; 2) → (125 ; 60−40=20)
        assert_eq!(polys[0][0], (85.0, 60.0));
        assert_eq!(*polys[0].last().unwrap(), (125.0, 20.0));
    }
}
