//! Schéma géométrique (man. p. 27-44). v1.3 : socle (points nommés,
//! courbes de Bézier). v1.5 : segments (avec médiane et hauteur de
//! triangle), vecteurs, droites (7 définitions), demi-droites, cercles,
//! ellipses, carré et polygones (prédéfinis, réguliers à n côtés,
//! quelconques), angles, zones coloriées et textes — les coniques
//! suivront en v1.6.
//!
//! Comme dans l'original, les objets citent des points par leur nom ;
//! les points peuvent être définis APRÈS les objets qui les utilisent
//! (résolution en deux passes). Une droite nommée peut être citée par
//! une autre droite (parallèle, perpendiculaire) si elle est définie
//! plus tôt dans la liste. Les points ne sont pas déplacés à la souris
//! ici (coordonnées saisies dans la fenêtre, comme les annotations
//! v0.2) ; le « Colorier une zone » se ramène à un polygone fermé
//! rempli uniformément (les hachures de l'original ne sont pas
//! reproduites, comme sa note OpenOffice).

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

fn def_vrai() -> bool {
    true
}

fn def_remplir() -> [u8; 3] {
    [140, 205, 250]
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

// ==================================================================
// v1.5 : objets géométriques
// ==================================================================

/// Forme d'une extrémité de segment (man. p. 30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Extremite {
    #[default]
    Nu,
    Fleche,
    Rond,
}

/// Style de trait : plein ou pointillé (les tirets rapprochés de
/// l'original sont rendus pointillés — approximation documentée).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TraitStyle {
    #[default]
    Plein,
    Pointille,
}

/// Un vecteur (direction et sens), jamais nul après normalisation.
type V2 = (f64, f64);

fn compo(a: V2, b: V2) -> V2 {
    (b.0 - a.0, b.1 - a.1)
}

/// Unité de `v` (None si norme nulle).
fn unit(v: V2) -> Option<V2> {
    let n = (v.0 * v.0 + v.1 * v.1).sqrt();
    if n < 1e-12 {
        None
    } else {
        Some((v.0 / n, v.1 / n))
    }
}

/// Perpendiculaire directe (rotation trigonométrique de 90°).
fn perp(v: V2) -> V2 {
    (-v.1, v.0)
}

fn rotation(v: V2, c: f64, s: f64) -> V2 {
    (v.0 * c - v.1 * s, v.0 * s + v.1 * c)
}

/// Trait adapté au style (plein ou pointillé).
fn piece_trait(p0: V2, p1: V2, c: [u8; 3], w: f64, st: TraitStyle) -> Piece {
    match st {
        TraitStyle::Plein => Piece::Ligne {
            x0: p0.0,
            y0: p0.1,
            x1: p1.0,
            y1: p1.1,
            c,
            w,
        },
        TraitStyle::Pointille => Piece::Pointille {
            x0: p0.0,
            y0: p0.1,
            x1: p1.0,
            y1: p1.1,
            c,
            w,
        },
    }
}

const LONGUEUR_FLECHE: f64 = 3.2;
/// demi-ouverture d'une pointe de flèche : 25°
const DEMI_OUVERTURE_FLECHE: f64 = 25.0 * std::f64::consts::PI / 180.0;

/// Pointe de flèche en mm centrée sur `bout`, la direction `d_mm`
/// désignant le sens de pointe (vers l'extérieur) : deux traits en
/// arrière à ±25°.
fn pointe_de_fleche(bout: V2, d_mm: V2, c: [u8; 3], w: f64) -> Vec<Piece> {
    let Some(u) = unit(d_mm) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (cs, sn) in [
        (DEMI_OUVERTURE_FLECHE.cos(), DEMI_OUVERTURE_FLECHE.sin()),
        (DEMI_OUVERTURE_FLECHE.cos(), -DEMI_OUVERTURE_FLECHE.sin()),
    ] {
        let d = rotation(u, cs, sn);
        out.push(Piece::Ligne {
            x0: bout.0,
            y0: bout.1,
            x1: bout.0 - d.0 * LONGUEUR_FLECHE,
            y1: bout.1 - d.1 * LONGUEUR_FLECHE,
            c,
            w,
        });
    }
    out
}

/// Point rond en mm (extrémité pleine).
fn point_rond(bout: V2, c: [u8; 3]) -> Piece {
    Piece::Disque {
        c: bout,
        r: 0.45,
        fill: c,
        stroke: None,
    }
}

/// Table des points nommés du schéma (les points peuvent être définis
/// après les objets qui les citent).
fn carte_des_points(schema: &[ShObj]) -> std::collections::BTreeMap<String, V2> {
    let mut m = std::collections::BTreeMap::new();
    for o in schema {
        if let ShObj::Pt(pt) = o {
            m.insert(pt.nom.clone(), (pt.x, pt.y));
        }
    }
    m
}

/// Trajet a→b en coordonnées utilisateur, ou None si un des points
/// manque.
fn segment_nomme(
    pts: &std::collections::BTreeMap<String, V2>,
    a: &str,
    b: &str,
) -> Option<(V2, V2)> {
    Some((*pts.get(a)?, *pts.get(b)?))
}

/// Débit de la droite / demi-droite sur les bords de la page
/// (Liang–Barsky en coordonnées utilisateur). `t0` démarre à 0 pour
/// une demi-droite, −∞ pour une droite entière.
fn clip_rayon(p: &Page, origine: V2, v: V2, demi: bool) -> Option<(V2, V2)> {
    let (xg, xd) = p.x_range();
    let (yb, yh) = p.y_range(); // (bas, haut) en valeurs graphiques
    if !(xg.is_finite() && xd.is_finite() && yb.is_finite() && yh.is_finite()) {
        return None;
    }
    let (xmin, xmax) = (xg.min(xd), xg.max(xd));
    let (ymin, ymax) = (yb.min(yh), yb.max(yh));
    let (mut t0, mut t1) = if demi {
        (0.0, f64::MAX)
    } else {
        (f64::MIN, f64::MAX)
    };
    let bord = |num: f64, den: f64, t0: &mut f64, t1: &mut f64| -> bool {
        if den.abs() < 1e-12 {
            return num > 0.0;
        }
        let t = num / den;
        if den < 0.0 {
            *t0 = t0.max(t);
        } else {
            *t1 = t1.min(t);
        }
        true
    };
    let (dx, dy) = v;
    if !bord(origine.0 - xmin, -dx, &mut t0, &mut t1) {
        return None;
    }
    if !bord(xmax - origine.0, dx, &mut t0, &mut t1) {
        return None;
    }
    if !bord(origine.1 - ymin, -dy, &mut t0, &mut t1) {
        return None;
    }
    if !bord(ymax - origine.1, dy, &mut t0, &mut t1) {
        return None;
    }
    if t0 > t1 {
        return None;
    }
    let sa = (origine.0 + t0 * dx, origine.1 + t0 * dy);
    let sb = (origine.0 + t1 * dx, origine.1 + t1 * dy);
    Some((sa, sb))
}

/// Droite entière ou demi-droite dessinée d'un bord à l'autre de la
/// page, avec le style de trait voulu.
fn ligne_infinie(
    p: &Page,
    origine: V2,
    v: V2,
    demi: bool,
    st: TraitStyle,
    c: [u8; 3],
    w: f64,
) -> Vec<Piece> {
    let Some((a, b)) = clip_rayon(p, origine, v, demi) else {
        return Vec::new();
    };
    let am = (p.mm_x(a.0), p.mm_y(a.1));
    let bm = (p.mm_x(b.0), p.mm_y(b.1));
    vec![piece_trait(am, bm, c, w, st)]
}

// ------------------------------------------------------------------
// Segment
// ------------------------------------------------------------------

/// Définition d'un segment (man. p. 30) : deux points, médiane ou
/// hauteur d'un triangle (le sommet de départ est donné par son rang
/// dans le triangle).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SegDef {
    DeuxPoints { a: String, b: String },
    Mediane { tri: [String; 3], de: usize },
    Hauteur { tri: [String; 3], de: usize },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub def: SegDef,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default)]
    pub debut: Extremite,
    #[serde(default)]
    pub fin: Extremite,
    #[serde(default)]
    pub style: TraitStyle,
}

impl Default for Segment {
    fn default() -> Self {
        Segment {
            def: SegDef::DeuxPoints {
                a: String::new(),
                b: String::new(),
            },
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            debut: Extremite::Nu,
            fin: Extremite::Nu,
            style: TraitStyle::Plein,
        }
    }
}

/// Projection orthogonale de `m` sur la droite (a, b).
fn projete(m: V2, a: V2, b: V2) -> V2 {
    let v = compo(a, b);
    let n2 = v.0 * v.0 + v.1 * v.1;
    if n2 < 1e-18 {
        return a;
    }
    let t = ((m.0 - a.0) * v.0 + (m.1 - a.1) * v.1) / n2;
    (a.0 + t * v.0, a.1 + t * v.1)
}

impl Segment {
    /// Les deux extrémités en coordonnées utilisateur.
    pub fn extremite(&self, pts: &std::collections::BTreeMap<String, V2>) -> Option<(V2, V2)> {
        match &self.def {
            SegDef::DeuxPoints { a, b } => segment_nomme(pts, a, b),
            SegDef::Mediane { tri, de } => {
                let i = (*de).min(2);
                let s = tri.get(i).and_then(|n| pts.get(n)).copied()?;
                let q = tri.get((i + 1) % 3).and_then(|n| pts.get(n)).copied()?;
                let r = tri.get((i + 2) % 3).and_then(|n| pts.get(n)).copied()?;
                Some((s, ((q.0 + r.0) / 2.0, (q.1 + r.1) / 2.0)))
            }
            SegDef::Hauteur { tri, de } => {
                let i = (*de).min(2);
                let s = tri.get(i).and_then(|n| pts.get(n)).copied()?;
                let q = tri.get((i + 1) % 3).and_then(|n| pts.get(n)).copied()?;
                let r = tri.get((i + 2) % 3).and_then(|n| pts.get(n)).copied()?;
                Some((s, projete(s, q, r)))
            }
        }
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let Some((a, b)) = self.extremite(&pts) else {
            return Vec::new();
        };
        let am = (p.mm_x(a.0), p.mm_y(a.1));
        let bm = (p.mm_x(b.0), p.mm_y(b.1));
        let mut out = vec![piece_trait(
            am,
            bm,
            self.couleur,
            self.epaisseur,
            self.style,
        )];
        for (bout, d_mm, e) in [
            (am, compo(bm, am), self.debut),
            (bm, compo(am, bm), self.fin),
        ] {
            match e {
                Extremite::Nu => {}
                Extremite::Fleche => {
                    out.extend(pointe_de_fleche(bout, d_mm, self.couleur, self.epaisseur))
                }
                Extremite::Rond => out.push(point_rond(bout, self.couleur)),
            }
        }
        out
    }
}

// ------------------------------------------------------------------
// Vecteur
// ------------------------------------------------------------------

/// Vecteur (man. p. 30-31) : nommé ou non, dessiné ou non. Départ =
/// point nommé ; arrivée = le point nommé `vers` (si non vide) sinon
/// le départ +(u, v).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vecteur {
    pub nom: String,
    #[serde(default)]
    pub afficher_nom: bool,
    pub depart: String,
    #[serde(default)]
    pub vers: String,
    #[serde(default)]
    pub u: f64,
    #[serde(default)]
    pub v: f64,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default = "def_vrai")]
    pub dessiner: bool,
}

impl Default for Vecteur {
    fn default() -> Self {
        Vecteur {
            nom: String::new(),
            afficher_nom: false,
            depart: String::new(),
            vers: String::new(),
            u: 1.0,
            v: 0.0,
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            dessiner: def_vrai(),
        }
    }
}

impl Vecteur {
    /// (départ, arrivée) en coordonnées utilisateur.
    pub fn extremite(&self, pts: &std::collections::BTreeMap<String, V2>) -> Option<(V2, V2)> {
        let a = *pts.get(&self.depart)?;
        let b = if self.vers.is_empty() {
            (a.0 + self.u, a.1 + self.v)
        } else {
            *pts.get(&self.vers)?
        };
        Some((a, b))
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        if !self.dessiner {
            return Vec::new();
        }
        let pts = carte_des_points(schema);
        let Some((a, b)) = self.extremite(&pts) else {
            return Vec::new();
        };
        let am = (p.mm_x(a.0), p.mm_y(a.1));
        let bm = (p.mm_x(b.0), p.mm_y(b.1));
        let mut out = vec![piece_trait(
            am,
            bm,
            self.couleur,
            self.epaisseur,
            TraitStyle::Plein,
        )];
        out.extend(pointe_de_fleche(
            bm,
            compo(am, bm),
            self.couleur,
            self.epaisseur,
        ));
        if self.afficher_nom && !self.nom.is_empty() {
            out.push(Piece::Texte {
                mm: ((am.0 + bm.0) / 2.0, (am.1 + bm.1) / 2.0 + 1.4),
                txt: self.nom.clone(),
                c: self.couleur,
                taille: 2.3,
                ancre: "middle",
                vertical: false,
                italique: true,
                gras: false,
            });
        }
        out
    }
}

// ------------------------------------------------------------------
// Droite
// ------------------------------------------------------------------

/// Direction d'une droite : deux points nommés, ou vecteur (u, v).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DirSpec {
    DeuxPoints { a: String, b: String },
    Vecteur { u: f64, v: f64 },
}

/// Définition d'une droite (man. p. 31-32) : par un point (direction
/// deux points ou vecteur), par un point et un vecteur normal,
/// parallèle ou perpendiculaire à une droite nommée, médiatrice,
/// bissectrice d'un angle (3 points, le 2e est le sommet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DroDef {
    PtDir { pt: String, dir: DirSpec },
    PtNorm { pt: String, dir: DirSpec },
    Parallele { pt: String, de: String },
    Perpendiculaire { pt: String, de: String },
    Mediatrice { a: String, b: String },
    Bissectrice { tri: [String; 3] },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Droite {
    pub nom: String,
    pub def: DroDef,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default)]
    pub style: TraitStyle,
}

impl Default for Droite {
    fn default() -> Self {
        Droite {
            nom: String::new(),
            def: DroDef::PtDir {
                pt: String::new(),
                dir: DirSpec::DeuxPoints {
                    a: String::new(),
                    b: String::new(),
                },
            },
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            style: TraitStyle::Plein,
        }
    }
}

/// Ligne résolue : nom + un point + une direction, en coordonnées
/// utilisateur.
#[derive(Debug, Clone)]
pub struct LigneRes {
    pub nom: String,
    pub p0: V2,
    pub v: V2,
}

impl Droite {
    /// Résolution : les points viennent de la carte ; les droites
    /// nommées citées doivent précéder dans `lignes`, donc dans
    /// l'ordre de la liste du schéma.
    pub fn resoudre(
        &self,
        pts: &std::collections::BTreeMap<String, V2>,
        lignes: &[LigneRes],
    ) -> Option<LigneRes> {
        let dir_de = |ds: &DirSpec, pts: &_| match ds {
            DirSpec::DeuxPoints { a, b } => {
                let (x, y) = segment_nomme(pts, a, b)?;
                unit(compo(x, y))
            }
            DirSpec::Vecteur { u, v } => unit((*u, *v)),
        };
        let (p0, v) = match &self.def {
            DroDef::PtDir { pt, dir } => {
                let a = *pts.get(pt)?;
                (a, dir_de(dir, pts)?)
            }
            DroDef::PtNorm { pt, dir } => {
                let a = *pts.get(pt)?;
                (a, unit(perp(dir_de(dir, pts)?))?)
            }
            DroDef::Parallele { pt, de } => {
                let a = *pts.get(pt)?;
                let l = lignes.iter().find(|l| l.nom == *de)?;
                (a, l.v)
            }
            DroDef::Perpendiculaire { pt, de } => {
                let a = *pts.get(pt)?;
                let l = lignes.iter().find(|l| l.nom == *de)?;
                (a, unit(perp(l.v))?)
            }
            DroDef::Mediatrice { a, b } => {
                let (x, y) = segment_nomme(pts, a, b)?;
                let m = ((x.0 + y.0) / 2.0, (x.1 + y.1) / 2.0);
                (m, unit(perp(compo(x, y)))?)
            }
            DroDef::Bissectrice { tri } => {
                let s = tri.get(1).and_then(|n| pts.get(n)).copied()?;
                let a1 = tri.first().and_then(|n| pts.get(n)).copied()?;
                let a2 = tri.get(2).and_then(|n| pts.get(n)).copied()?;
                let u1 = unit(compo(s, a1))?;
                let u2 = unit(compo(s, a2))?;
                let v = unit((u1.0 + u2.0, u1.1 + u2.1)).or_else(|| unit(perp(u1)))?;
                (s, v)
            }
        };
        Some(LigneRes {
            nom: self.nom.clone(),
            p0,
            v,
        })
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let lignes = resoudre_lignes(schema, &pts);
        // nommée : ligne déjà résolue dans le schéma ; sans nom :
        // résolution directe (pas de cible)
        let r = if self.nom.is_empty() {
            self.resoudre(&pts, &lignes)
        } else {
            lignes.iter().rev().find(|l| l.nom == self.nom).cloned()
        };
        match r {
            Some(l) => ligne_infinie(
                p,
                l.p0,
                l.v,
                false,
                self.style,
                self.couleur,
                self.epaisseur,
            ),
            None => Vec::new(),
        }
    }
}

/// Résolution des droites du schéma, dans l'ordre de la liste (une
/// droite nommée peut être citée par une suivante) ; un nom déjà pris
/// n'est pas réinscrit.
pub fn resoudre_lignes(
    schema: &[ShObj],
    pts: &std::collections::BTreeMap<String, V2>,
) -> Vec<LigneRes> {
    let mut out: Vec<LigneRes> = Vec::new();
    for o in schema {
        if let ShObj::Dro(d) = o {
            if let Some(r) = d.resoudre(pts, &out) {
                if !r.nom.is_empty() && out.iter().any(|l| l.nom == r.nom) {
                    continue;
                }
                out.push(r);
            }
        }
    }
    out
}

// ------------------------------------------------------------------
// Demi-droite
// ------------------------------------------------------------------

/// Demi-droite (man. p. 32) : origine = point nommé ; direction =
/// autre point, vecteur (u, v) ou angle polaire en degrés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DemiDef {
    Point { dir: String },
    Vecteur { u: f64, v: f64 },
    Angle { deg: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DemiDroite {
    pub origine: String,
    pub def: DemiDef,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default)]
    pub style: TraitStyle,
}

impl Default for DemiDroite {
    fn default() -> Self {
        DemiDroite {
            origine: String::new(),
            def: DemiDef::Point { dir: String::new() },
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            style: TraitStyle::Plein,
        }
    }
}

impl DemiDroite {
    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let Some(o) = pts.get(&self.origine) else {
            return Vec::new();
        };
        let v = match &self.def {
            DemiDef::Point { dir } => {
                let Some(q) = pts.get(dir) else {
                    return Vec::new();
                };
                unit(compo(*o, *q))
            }
            DemiDef::Vecteur { u, v } => unit((*u, *v)),
            DemiDef::Angle { deg } => {
                let t = deg.to_radians();
                Some(rotation((1.0, 0.0), t.cos(), t.sin()))
            }
        };
        let Some(v) = v else {
            return Vec::new();
        };
        ligne_infinie(p, *o, v, true, self.style, self.couleur, self.epaisseur)
    }
}

// ------------------------------------------------------------------
// Cercle
// ------------------------------------------------------------------

/// Cercle (man. p. 33) : centre + rayon, centre + point du cercle, ou
/// deux points diamétralement opposés. Deux couleurs dans l'original :
/// bord (`couleur`) et intérieur (`remplir`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CerDef {
    CentreRayon { c: String, r: f64 },
    CentrePoint { c: String, p: String },
    Diametral { a: String, b: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cercle {
    pub def: CerDef,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remplir: Option<[u8; 3]>,
}

impl Default for Cercle {
    fn default() -> Self {
        Cercle {
            def: CerDef::CentreRayon {
                c: String::new(),
                r: 1.0,
            },
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            remplir: None,
        }
    }
}

impl Cercle {
    /// (centre, rayon) en unités utilisateur. Le rayon est dessiné en
    /// mm avec l'unité des abscisses (repère orthonormé ; les unités
    /// x ≠ y de l'original ne déforment pas le cercle).
    pub fn centre_rayon(&self, pts: &std::collections::BTreeMap<String, V2>) -> Option<(V2, f64)> {
        match &self.def {
            CerDef::CentreRayon { c, r } => Some((*pts.get(c)?, *r)),
            CerDef::CentrePoint { c, p } => {
                let (x, y) = segment_nomme(pts, c, p)?;
                Some((x, (y.0 - x.0).hypot(y.1 - x.1)))
            }
            CerDef::Diametral { a, b } => {
                let (x, y) = segment_nomme(pts, a, b)?;
                Some((
                    ((x.0 + y.0) / 2.0, (x.1 + y.1) / 2.0),
                    (y.0 - x.0).hypot(y.1 - x.1) / 2.0,
                ))
            }
        }
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let Some((c, r)) = self.centre_rayon(&pts) else {
            return Vec::new();
        };
        if !(r.is_finite() && r > 0.0) {
            return Vec::new();
        }
        vec![Piece::Disque {
            c: (p.mm_x(c.0), p.mm_y(c.1)),
            r: r * p.unit_mm,
            fill: self.remplir.unwrap_or([255, 255, 255]),
            stroke: Some((self.couleur, self.epaisseur)),
        }]
    }
}

// ------------------------------------------------------------------
// Ellipse
// ------------------------------------------------------------------

/// Échantillons du tracé d'ellipse.
const N_ELL: usize = 72;

/// Ellipse (man. p. 33) : centre + 2 demi-axes, ou deux sommets
/// opposés du rectangle qui la contient. Axes parallèles au repère
/// (comme l'original ; l'ellipse oblique passe par les polygones).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EllDef {
    CentreAxes { c: String, rx: f64, ry: f64 },
    Rectangle { a: String, b: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ellipse {
    pub def: EllDef,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remplir: Option<[u8; 3]>,
}

impl Default for Ellipse {
    fn default() -> Self {
        Ellipse {
            def: EllDef::CentreAxes {
                c: String::new(),
                rx: 1.0,
                ry: 0.5,
            },
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            remplir: None,
        }
    }
}

impl Ellipse {
    /// (centre, demi-axes) en unités utilisateur.
    pub fn centre_axes(
        &self,
        pts: &std::collections::BTreeMap<String, V2>,
    ) -> Option<(V2, f64, f64)> {
        match &self.def {
            EllDef::CentreAxes { c, rx, ry } => Some((*pts.get(c)?, *rx, *ry)),
            EllDef::Rectangle { a, b } => {
                let (x, y) = segment_nomme(pts, a, b)?;
                let c = ((x.0 + y.0) / 2.0, (x.1 + y.1) / 2.0);
                Some((c, (c.0 - x.0).abs(), (c.1 - x.1).abs()))
            }
        }
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let Some((c, rx, ry)) = self.centre_axes(&pts) else {
            return Vec::new();
        };
        if !(rx.is_finite() && ry.is_finite() && rx > 0.0 && ry > 0.0) {
            return Vec::new();
        }
        let mut mm = Vec::with_capacity(N_ELL + 1);
        for i in 0..=N_ELL {
            let t = 2.0 * std::f64::consts::PI * i as f64 / N_ELL as f64;
            let g = (c.0 + rx * t.cos(), c.1 + ry * t.sin());
            mm.push((p.mm_x(g.0), p.mm_y(g.1)));
        }
        if mm.iter().any(|q| !q.0.is_finite() || !q.1.is_finite()) {
            return Vec::new();
        }
        vec![Piece::Poly {
            pts: mm,
            fill: self.remplir,
            stroke: Some((self.couleur, self.epaisseur)),
        }]
    }
}

// ------------------------------------------------------------------
// Polygones
// ------------------------------------------------------------------

/// Polygones prédéfinis posés sur l'arête [A, B], construits dans le
/// demi-plan à gauche de A→B (sens trigonométrique, man. p. 34) ; h
/// est la seconde dimension (hauteur, diagonale...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PresetPoly {
    /// triangle isocèle de base [A,B] et de hauteur h
    #[default]
    Isocele,
    /// triangle équilatéral posé sur [A,B] (hauteur automatique)
    Equilateral,
    /// triangle rectangle en A, construit sur [A,B] (côté A vertical)
    RectEnA,
    /// carré direct construit sur [A,B]
    Carre,
    /// rectangle de base [A,B] et de hauteur h
    Rectangle,
    /// losange de diagonale [A,B], l'autre diagonale valant h
    Losange,
    /// pentagone régulier posé sur [A,B]
    Pentagone,
    /// hexagone régulier posé sur [A,B]
    Hexagone,
    /// octogone régulier posé sur [A,B]
    Octogone,
}

/// Nature d'un polygone (man. p. 34-35) : quelconque (sommets nommés),
/// régulier à n côtés posé sur [A,B], parallélogramme de 3 sommets
/// (le 4e vaut D = A − B + C), ou prédéfini sur un segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PgnDef {
    Noms {
        sommets: Vec<String>,
    },
    Regulier {
        a: String,
        b: String,
        n: usize,
    },
    Parallelogramme {
        a: String,
        b: String,
        c: String,
    },
    Predefini {
        nature: PresetPoly,
        a: String,
        b: String,
        h: f64,
    },
}

/// Sommets d'un polygone régulier posé sur [A, B] (sens trigonométrique,
/// angle extérieur 2π/n entre chaque arête).
fn polygone_regulier(a: V2, b: V2, n: usize) -> Vec<V2> {
    let ext = 2.0 * std::f64::consts::PI / n.max(3) as f64;
    let mut pts = vec![a, b];
    let d = compo(a, b);
    let (mut x, mut y) = b;

    for k in 1..n.max(3).saturating_sub(1) {
        let d = rotation(d, (k as f64 * ext).cos(), (k as f64 * ext).sin());
        x += d.0;
        y += d.1;
        pts.push((x, y));
    }
    pts
}

/// Sommets d'un polygone prédéfini sur [A, B].
fn sommets_preset(nature: PresetPoly, a: V2, b: V2, h: f64) -> Vec<V2> {
    let n = unit(compo(a, b)).map(perp).unwrap_or((0.0, 1.0));
    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let ab = (b.0 - a.0, b.1 - a.1);
    let lab = ab.0.hypot(ab.1);
    match nature {
        PresetPoly::Isocele => vec![a, b, (m.0 + n.0 * h, m.1 + n.1 * h)],
        PresetPoly::Equilateral => {
            let hh = lab * 3.0f64.sqrt() / 2.0;
            vec![a, b, (m.0 + n.0 * hh, m.1 + n.1 * hh)]
        }
        PresetPoly::RectEnA => vec![a, b, (a.0 + n.0 * h, a.1 + n.1 * h)],
        PresetPoly::Carre => vec![
            a,
            b,
            (b.0 + n.0 * lab, b.1 + n.1 * lab),
            (a.0 + n.0 * lab, a.1 + n.1 * lab),
        ],
        PresetPoly::Rectangle => vec![
            a,
            b,
            (b.0 + n.0 * h, b.1 + n.1 * h),
            (a.0 + n.0 * h, a.1 + n.1 * h),
        ],
        PresetPoly::Losange => {
            let n2 = unit(perp(unit(compo(a, b)).unwrap_or((1.0, 0.0)))).unwrap_or((0.0, 1.0));
            vec![
                a,
                (m.0 + n2.0 * h / 2.0, m.1 + n2.1 * h / 2.0),
                b,
                (m.0 - n2.0 * h / 2.0, m.1 - n2.1 * h / 2.0),
            ]
        }
        PresetPoly::Pentagone => polygone_regulier(a, b, 5),
        PresetPoly::Hexagone => polygone_regulier(a, b, 6),
        PresetPoly::Octogone => polygone_regulier(a, b, 8),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygone {
    pub def: PgnDef,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remplir: Option<[u8; 3]>,
    /// dessiner les sommets (croix + nom, comme l'original les crée)
    #[serde(default = "def_vrai")]
    pub sommets: bool,
}

impl Default for Polygone {
    fn default() -> Self {
        Polygone {
            def: PgnDef::Noms { sommets: vec![] },
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            remplir: None,
            sommets: def_vrai(),
        }
    }
}

impl Polygone {
    /// Sommets calculés en coordonnées utilisateur ; None si un point
    /// nommé manque.
    pub fn sommets_calcules(
        &self,
        pts: &std::collections::BTreeMap<String, V2>,
    ) -> Option<Vec<V2>> {
        Some(match &self.def {
            PgnDef::Noms { sommets } => sommets
                .iter()
                .map(|n| pts.get(n).copied())
                .collect::<Option<Vec<V2>>>()?,
            PgnDef::Regulier { a, b, n } => {
                let (x, y) = segment_nomme(pts, a, b)?;
                polygone_regulier(x, y, *n)
            }
            PgnDef::Parallelogramme { a, b, c } => {
                let x = *pts.get(a)?;
                let y = *pts.get(b)?;
                let z = *pts.get(c)?;
                vec![x, y, z, (x.0 - y.0 + z.0, x.1 - y.1 + z.1)]
            }
            PgnDef::Predefini { nature, a, b, h } => {
                let (x, y) = segment_nomme(pts, a, b)?;
                sommets_preset(*nature, x, y, *h)
            }
        })
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let Some(sg) = self.sommets_calcules(&pts) else {
            return Vec::new();
        };
        if sg.len() < 3 {
            return Vec::new();
        }
        let mut out = Vec::new();
        if self.sommets {
            let noms: Vec<String> = match &self.def {
                PgnDef::Noms { sommets } => sommets.clone(),
                _ => vec![String::new(); sg.len()],
            };
            for (i, (x, y)) in sg.iter().enumerate() {
                let xm = p.mm_x(*x);
                let ym = p.mm_y(*y);
                let r = 0.6;
                out.push(Piece::Ligne {
                    x0: xm - r,
                    y0: ym,
                    x1: xm + r,
                    y1: ym,
                    c: NOIR,
                    w: 0.35,
                });
                out.push(Piece::Ligne {
                    x0: xm,
                    y0: ym - r,
                    x1: xm,
                    y1: ym + r,
                    c: NOIR,
                    w: 0.35,
                });
                if let Some(n) = noms.get(i) {
                    if !n.is_empty() {
                        out.push(Piece::Texte {
                            mm: (xm + 0.9, ym + 0.6),
                            txt: n.clone(),
                            c: NOIR,
                            taille: 2.3,
                            ancre: "",
                            vertical: false,
                            italique: true,
                            gras: false,
                        });
                    }
                }
            }
        }
        let mm: Vec<(f64, f64)> = sg.iter().map(|g| (p.mm_x(g.0), p.mm_y(g.1))).collect();
        if mm.iter().any(|q| !q.0.is_finite() || !q.1.is_finite()) {
            return Vec::new();
        }
        out.push(Piece::Poly {
            pts: mm,
            fill: self.remplir,
            stroke: Some((self.couleur, self.epaisseur)),
        });
        out
    }
}

// ------------------------------------------------------------------
// Angle
// ------------------------------------------------------------------

/// Angle (man. p. 39) : sommet = 2e point (notation (A, S, F) : le
/// sommet S est le second). Arc tracé au sommet entre les deux
/// directions, du rayon `rayon` (0 = la moitié de la plus courte des
/// deux branches), balayage le plus court.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Angle {
    pub sommet: String,
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub rayon: f64,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
}

impl Default for Angle {
    fn default() -> Self {
        Angle {
            sommet: String::new(),
            a: String::new(),
            b: String::new(),
            rayon: 0.0,
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
        }
    }
}

impl Angle {
    /// (sommet, θ1, balayage signé, rayon).
    pub fn resoudre(
        &self,
        pts: &std::collections::BTreeMap<String, V2>,
    ) -> Option<(V2, f64, f64, f64)> {
        let s = *pts.get(&self.sommet)?;
        let p1 = *pts.get(&self.a)?;
        let p2 = *pts.get(&self.b)?;
        let d1 = compo(s, p1);
        let d2 = compo(s, p2);
        let l1 = d1.0.hypot(d1.1);
        let l2 = d2.0.hypot(d2.1);
        let r = if self.rayon > 0.0 {
            self.rayon
        } else {
            l1.min(l2) / 2.0
        };
        let t1 = d1.1.atan2(d1.0);
        let t2 = d2.1.atan2(d2.0);
        let deux_pi = 2.0 * std::f64::consts::PI;
        let mut delta = t2 - t1;
        delta = (delta % deux_pi + deux_pi) % deux_pi;
        if delta > std::f64::consts::PI {
            delta -= deux_pi;
        }
        Some((s, t1, delta, r))
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let Some((s, t1, delta, r)) = self.resoudre(&pts) else {
            return Vec::new();
        };
        if !r.is_finite() || r <= 0.0 {
            return Vec::new();
        }
        let n = 16;
        let mut out = Vec::new();
        let mut precedent = (p.mm_x(s.0 + r * t1.cos()), p.mm_y(s.1 + r * t1.sin()));
        for k in 1..=n {
            let t = t1 + delta * k as f64 / n as f64;
            let g = (p.mm_x(s.0 + r * t.cos()), p.mm_y(s.1 + r * t.sin()));
            out.push(piece_trait(
                precedent,
                g,
                self.couleur,
                self.epaisseur,
                TraitStyle::Plein,
            ));
            precedent = g;
        }
        out
    }
}

// ------------------------------------------------------------------
// Zone coloriée
// ------------------------------------------------------------------

/// Zone coloriée (man. p. 39) : polygone fermé à partir de sommets
/// nommés, remplissage uniforme. Dans l'original, le bouton colorie le
/// contour d'une figure par flood-fill ; ici, le polygone fermé prend
/// sa couleur d'autant tout en gardant le trait des arêtes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub sommets: Vec<String>,
    #[serde(default = "def_remplir")]
    pub couleur: [u8; 3],
}

impl Default for Zone {
    fn default() -> Self {
        Zone {
            sommets: vec![],
            couleur: def_remplir(),
        }
    }
}

impl Zone {
    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let pts = carte_des_points(schema);
        let Some(sg) = self
            .sommets
            .iter()
            .map(|n| pts.get(n).copied())
            .collect::<Option<Vec<V2>>>()
        else {
            return Vec::new();
        };
        if sg.len() < 3 {
            return Vec::new();
        }
        let mm: Vec<(f64, f64)> = sg.iter().map(|g| (p.mm_x(g.0), p.mm_y(g.1))).collect();
        if mm.iter().any(|q| !q.0.is_finite() || !q.1.is_finite()) {
            return Vec::new();
        }
        vec![Piece::Poly {
            pts: mm,
            fill: Some(self.couleur),
            stroke: None,
        }]
    }
}

// ------------------------------------------------------------------
// Textes
// ------------------------------------------------------------------

/// Texte ordinaire du schéma (man. p. 40-41) : contenu, coordonnées
/// (en unités du repère), taille (mm), gras, italique, couleur ; y
/// compris texte vertical (incliné de 90°, modalité raster commune —
/// l'original dispose d'une inclinaison arbitraire, hors raster).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TexteSchema {
    pub x: f64,
    pub y: f64,
    pub texte: String,
    #[serde(default = "def_taille_txt")]
    pub taille: f64,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default)]
    pub gras: bool,
    #[serde(default)]
    pub italique: bool,
    #[serde(default)]
    pub vertical: bool,
    /// couleur de l'arrière-plan (man. p. 39, v1.6) ; rien par défaut
    #[serde(default)]
    pub fond: Option<[u8; 3]>,
    /// texte souligné (attribut cumulatif, man. p. 40, v1.6)
    #[serde(default)]
    pub souligne: bool,
}

fn def_taille_txt() -> f64 {
    3.2
}

impl Default for TexteSchema {
    fn default() -> Self {
        TexteSchema {
            x: 0.0,
            y: 0.0,
            texte: String::new(),
            taille: def_taille_txt(),
            couleur: def_couleur(),
            gras: false,
            italique: false,
            vertical: false,
            fond: None,
            souligne: false,
        }
    }
}

impl TexteSchema {
    /// Détection d'un second caractère en indice (man. p. 40) :
    /// texte de 2 caractères dont le second est un chiffre (a1, B0 —
    /// toujours romain), ou l'un des 3 cas spéciaux C f/g/h (indice
    /// italique).
    fn indice_auto(&self) -> Option<(char, usize)> {
        let ch: Vec<char> = self.texte.chars().collect();
        if ch.len() != 2 {
            return None;
        }
        if ch[1].is_ascii_digit() && !ch[0].is_ascii_digit() {
            return Some((ch[1], 0));
        }
        if ch[0] == 'C' && matches!(ch[1], 'f' | 'g' | 'h') {
            return Some((ch[1], 1));
        }
        None
    }

    /// Avance horizontale d'un caractère, mm (la fonte bitmap avance
    /// de 6 px par casse de 7 px) ; le SVG s'en tient à la même
    /// estimation.
    fn avance(&self) -> f64 {
        6.0 / 7.0 * self.taille
    }

    pub fn draw_list(&self, p: &Page) -> Vec<Piece> {
        if self.texte.is_empty() {
            return Vec::new();
        }
        let pos = (p.mm_x(self.x), p.mm_y(self.y));
        let nb = self.texte.chars().count();
        let (indice, italique_idx) = match self.indice_auto() {
            Some((ch, it)) => (Some(ch), it == 1),
            None => (None, false),
        };
        // fond (arrière-plan) : rectangle derrière le texte, posé AVANT
        let mut out = Vec::new();
        if let Some(c) = self.fond {
            let (mm0, w, h) = match self.vertical {
                // colonne verticale (rotation −90°, lecture bas→haut) :
                // corps des glyphes à gauche de la verticale, descentes
                // (et souligné) à droite
                true => (
                    (
                        pos.0 - 1.0 * self.taille,
                        pos.1 - nb as f64 * self.avance() - 0.1 * self.taille,
                    ),
                    1.4 * self.taille,
                    nb as f64 * self.avance() + 0.45 * self.taille,
                ),
                false => (
                    (pos.0, pos.1 - 0.2 * self.taille),
                    nb as f64 * self.avance() + 0.2 * self.taille,
                    1.15 * self.taille,
                ),
            };
            out.push(Piece::Rect {
                mm: mm0,
                w,
                h,
                fill: Some(c),
                stroke: ([0, 0, 0], 0.0),
            });
        }
        // texte (ou base + indice automatique)
        match (indice, self.vertical) {
            (Some(idx), false) => {
                out.push(Piece::Texte {
                    mm: pos,
                    txt: self.texte.chars().next().unwrap().to_string(),
                    c: self.couleur,
                    taille: self.taille,
                    ancre: "",
                    vertical: false,
                    gras: self.gras,
                    italique: self.italique,
                });
                // l'indice : toujours 70 % de la taille, sans gras ;
                // romain pour les chiffres, italique pour Cf/Cg/Ch
                out.push(Piece::Texte {
                    mm: (pos.0 + self.avance(), pos.1 + 0.32 * self.taille),
                    txt: idx.to_string(),
                    c: self.couleur,
                    taille: self.taille * 0.7,
                    ancre: "",
                    vertical: false,
                    gras: false,
                    italique: italique_idx,
                });
            }
            _ => {
                out.push(Piece::Texte {
                    mm: pos,
                    txt: self.texte.clone(),
                    c: self.couleur,
                    taille: self.taille,
                    ancre: "",
                    vertical: self.vertical,
                    gras: self.gras,
                    italique: self.italique,
                });
            }
        }
        // souligné
        if self.souligne {
            let long = nb as f64 * self.avance();
            let (x0, y0, x1, y1) = if self.vertical {
                // à droite de la colonne (côté descentes), le long du
                // texte qui monte depuis le point d'ancrage
                (
                    pos.0 + 0.28 * self.taille,
                    pos.1,
                    pos.0 + 0.28 * self.taille,
                    pos.1 - long,
                )
            } else {
                (
                    pos.0,
                    pos.1 + 0.18 * self.taille,
                    pos.0 + long,
                    pos.1 + 0.18 * self.taille,
                )
            };
            out.push(Piece::Ligne {
                x0,
                y0,
                x1,
                y1,
                c: self.couleur,
                w: (0.07 * self.taille).max(0.18),
            });
        }
        out
    }
}

// ------------------------------------------------------------------
// Coniques (man. p. 35-37)
// ------------------------------------------------------------------

/// Directrice d'une conique définie par foyer/excentricité : une
/// droite déjà définie dans le schéma (cité par son nom), ou deux
/// points nommés (man. p. 35).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Directrice {
    Droite { nom: String },
    DeuxPoints { a: String, b: String },
}

/// Conique (man. p. 35-37) : par ses caractéristiques géométriques
/// (foyer, directrice, excentricité strictement positive) ou par son
/// équation cartésienne ax² + bxy + cy² + dx + ey + f = 0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConDef {
    FoyerDirectrice {
        foyer: String,
        directrice: Directrice,
        e: f64,
    },
    Cartesienne {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },
}

/// Nombre de valeurs d'échantillonnage : de l'angle pour la définition
/// géométrique, de x pour la cartésienne.
const N_CON: usize = 480;
const N_CON_X: usize = 600;

/// Éléments caractéristiques calculés (unités utilisateur), pour les
/// options de traçage (man. p. 35) et l'excentricité affichable.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConicParts {
    pub branches: Vec<Vec<V2>>,
    pub foyers: Vec<V2>,
    pub sommets: Vec<V2>,
    pub centre: Option<V2>,
    /// (point, direction) pour les axes, directrices et asymptotes.
    pub axes: Vec<(V2, V2)>,
    pub directrices: Vec<(V2, V2)>,
    pub asymptotes: Vec<(V2, V2)>,
}

/// Normale unitaire de la directrice pointant vers le foyer, et
/// distance du foyer à la directrice (h > 0). La forme polaire
/// r(α) = e·h / (1 − e·cos α) mesurera α depuis cette normale.
fn directrice_normale(foyer: V2, p0: V2, v: V2) -> Option<(V2, f64)> {
    let n0 = unit(perp(v))?;
    let mut d = (foyer.0 - p0.0) * n0.0 + (foyer.1 - p0.1) * n0.1;
    let n = if d < 0.0 {
        d = -d;
        (-n0.0, -n0.1)
    } else {
        n0
    };
    if d <= 1e-12 {
        None // foyer sur la directrice : conique dégénérée
    } else {
        Some((n, d))
    }
}

/// Valeurs propres et vecteurs propres orthonormés de la matrice
/// [[a, b2], [b2, c]] (λ1 ≥ λ2).
fn eigen2(a: f64, b2: f64, c: f64) -> (f64, V2, f64, V2) {
    let tr = a + c;
    let disc = ((a - c) * (a - c) + 4.0 * b2 * b2).max(0.0).sqrt();
    let l1 = (tr + disc) / 2.0;
    let l2 = (tr - disc) / 2.0;
    if b2.abs() <= 1e-14 {
        let v1 = if l1 >= l2 { (1.0, 0.0) } else { (0.0, 1.0) };
        (l1, v1, l2, perp(v1))
    } else {
        let v = |l: f64| -> V2 { unit((b2, l - a)).unwrap_or((1.0, 0.0)) };
        let v1 = v(l1);
        // second vecteur : perpendiculaire (base orthonormée)
        let v2 = perp(v1);
        (l1, v1, l2, v2)
    }
}

/// Décomposition tensorielle d'une conique cartésienne : renvoie
/// (centre, λ1, v1, λ2, v2, γ = Q(centre)) pour une conique à centre
/// (det A2 ≠ 0).
fn conique_centre(
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
) -> Option<(V2, f64, V2, f64, V2, f64)> {
    let b2 = b / 2.0;
    let det = a * c - b2 * b2;
    if det.abs() <= 1e-12 {
        return None;
    }
    let ox = -(c * d - b2 * e) / (2.0 * det);
    let oy = -(-b2 * d + a * e) / (2.0 * det);
    let (l1, v1, l2, v2) = eigen2(a, b2, c);
    let g = a * ox * ox + b * ox * oy + c * oy * oy + d * ox + e * oy + f;
    Some(((ox, oy), l1, v1, l2, v2, g))
}

/// Définition géométrique : détermination du foyer, de la normale de
/// la directrice, de la distance foyer-directrice et de
/// l'excentricité.
fn conique_geo_donnees(
    fo: &str,
    dir: &Directrice,
    e: f64,
    schema: &[ShObj],
) -> Option<(V2, V2, f64, f64)> {
    if e <= 0.0 || !e.is_finite() {
        return None;
    }
    let pts = carte_des_points(schema);
    let f = *pts.get(fo)?;
    let (p0, v) = match dir {
        Directrice::Droite { nom } => {
            let lignes = resoudre_lignes(schema, &pts);
            let l = lignes.iter().find(|l| l.nom == *nom)?;
            (l.p0, l.v)
        }
        Directrice::DeuxPoints { a, b } => {
            let (x, y) = segment_nomme(&pts, a, b)?;
            (x, unit(compo(x, y))?)
        }
    };
    let (n, h) = directrice_normale(f, p0, v)?;
    Some((f, n, h, e))
}

/// Conique par foyer/directrice/excentricité : branches échantillonnées
/// en polaire (foyer pôle, α mesuré depuis la normale) + éléments
/// caractéristiques (man. p. 35-36).
fn conique_geo(f: V2, n: V2, h: f64, e: f64, r_max: f64) -> ConicParts {
    let mut parts = ConicParts::default();
    let w = perp(n);
    // forme polaire ; les α où 1 − e·cos α ≈ 0 coupent les branches
    let mut cur: Vec<V2> = Vec::new();
    for k in 0..=N_CON {
        let t = 2.0 * std::f64::consts::PI * k as f64 / N_CON as f64;
        let den = 1.0 - e * t.cos();
        let ok = den.abs() > 1e-12;
        let r = if ok { e * h / den } else { f64::INFINITY };
        if !ok || !r.is_finite() || r.abs() > r_max {
            if cur.len() > 1 {
                parts.branches.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
            continue;
        }
        // direction α : rotation de n d'un angle t
        let d = rotation(n, t.cos(), t.sin());
        cur.push((f.0 + r * d.0, f.1 + r * d.1));
    }
    if cur.len() > 1 {
        parts.branches.push(cur);
    }
    // éléments caractéristiques
    if (e - 1.0).abs() < 1e-12 {
        // parabole : foyer F, paramètre p = h, sommet au milieu du
        // segment [foyer, pied de la perpendiculaire], ouverture vers n
        let s = (f.0 - h / 2.0 * n.0, f.1 - h / 2.0 * n.1);
        parts.foyers = vec![f];
        parts.sommets = vec![s];
        parts.axes = vec![(s, n)];
        parts.directrices = vec![((f.0 - h * n.0, f.1 - h * n.1), w)];
    } else {
        let p = e * h;
        let a = if e < 1.0 {
            p / (1.0 - e * e)
        } else {
            p / (e * e - 1.0)
        };
        let cc = e * a;
        // b : ellipse a² − c² ; hyperbole c² − a²
        let b = if e < 1.0 {
            (a * a - cc * cc).sqrt()
        } else {
            (cc * cc - a * a).sqrt()
        };
        let o = if e < 1.0 {
            (f.0 + cc * n.0, f.1 + cc * n.1)
        } else {
            (f.0 - cc * n.0, f.1 - cc * n.1)
        };
        parts.centre = Some(o);
        parts.foyers = vec![f, (2.0 * o.0 - f.0, 2.0 * o.1 - f.1)];
        parts.sommets = vec![
            (o.0 + a * n.0, o.1 + a * n.1),
            (o.0 - a * n.0, o.1 - a * n.1),
        ];
        if e < 1.0 {
            // ellipse : petit axe supplémentaire (4 sommets)
            parts.sommets.push((o.0 + b * w.0, o.1 + b * w.1));
            parts.sommets.push((o.0 - b * w.0, o.1 - b * w.1));
        } else {
            // hyperbole : asymptotes de directions (±a ; ±b) dans la
            // base (n, w)
            let c_loc = cc;
            let dd = 1.0 / c_loc;
            let v = (a * n.0 + b * w.0, a * n.1 + b * w.1);
            parts.asymptotes = vec![(o, (v.0 * dd, v.1 * dd))];
            let v2 = (a * n.0 - b * w.0, a * n.1 - b * w.1);
            parts.asymptotes.push((o, (v2.0 * dd, v2.1 * dd)));
        }
        parts.axes = vec![(o, n), (o, w)];
        // directrices à la distance a/e du centre, normales à l'axe
        let dired = a / e;
        parts.directrices = vec![
            ((o.0 - dired * n.0, o.1 - dired * n.1), w),
            ((o.0 + dired * n.0, o.1 + dired * n.1), w),
        ];
    }
    parts
}

/// Conique cartésienne : branches échantillonnées par x
/// (cy² + (bx+e)y + (ax²+dx+f) = 0), et éléments caractéristiques par
/// réduction tensorielle (man. p. 37).
fn conique_cart(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, p: &Page) -> ConicParts {
    let mut parts = ConicParts::default();
    if [a, b, c, d, e, f].iter().all(|q| q.abs() <= 1e-15) {
        return parts; // équation identiquement nulle : tout le plan
    }
    let s = a
        .abs()
        .max(b.abs())
        .max(c.abs())
        .max(d.abs())
        .max(e.abs())
        .max(f.abs());
    let (xg, xd) = p.x_range();
    let (yb, yh) = p.y_range();
    let large = xd - xg;
    let marge = large / 2.0;
    let x0 = xg - marge / 2.0;
    let x1 = xd + marge / 2.0;
    let y_lim = (yh - yb) * 0.6 + 1.0;
    // cas des droites verticales dégénérées (pas de terme en y) :
    // ax² + dx + f = 0 → x = x1, x2…
    if c.abs() <= 1e-14 && b.abs() <= 1e-14 && e.abs() <= 1e-14 {
        let racines = solve_quad(a, d, f);
        for r in racines {
            if r.is_finite() && r >= x0 && r <= x1 {
                parts
                    .branches
                    .push(vec![(r, yb - marge / 2.0), (r, yh + marge / 2.0)]);
            }
        }
        return parts;
    }
    let mut slots: [Vec<V2>; 2] = [Vec::new(), Vec::new()];
    for k in 0..=N_CON_X {
        let x = x0 + (x1 - x0) * k as f64 / N_CON_X as f64;
        let by = b * x + e;
        let cy = a * x * x + d * x + f;
        // racines de c·y² + by·y + cy = 0
        let ys: [Option<f64>; 2] = if c.abs() <= 1e-14 {
            if by.abs() <= 1e-14 {
                [None, None]
            } else {
                [Some(-cy / by), None]
            }
        } else {
            let disc = by * by - 4.0 * c * cy;
            if disc < 0.0 {
                [None, None]
            } else {
                let r = disc.sqrt();
                [Some((-by - r) / (2.0 * c)), Some((-by + r) / (2.0 * c))]
            }
        };
        for (i, y) in ys.into_iter().enumerate() {
            let Some(y) = y else {
                if slots[i].len() > 1 {
                    parts.branches.push(std::mem::take(&mut slots[i]));
                } else {
                    slots[i].clear();
                }
                continue;
            };
            if !y.is_finite() || y.abs() > y_lim {
                if slots[i].len() > 1 {
                    parts.branches.push(std::mem::take(&mut slots[i]));
                } else {
                    slots[i].clear();
                }
                continue;
            }
            slots[i].push((x, y));
        }
    }
    for slot in slots {
        if slot.len() > 1 {
            parts.branches.push(slot);
        }
    }
    // éléments caractéristiques (man. p. 37) : réduction tensorielle
    let _ = s;
    let b2 = b / 2.0;
    let det = a * c - b2 * b2;
    let det3 = a * c * f - a * e * e / 4.0 - b2 * b2 * f - c * d * d / 4.0 + b * d * e / 4.0;
    let eps2 = 1e-9 * s * s;
    let eps3 = 1e-9 * s * s * s;
    if det.abs() <= eps2 {
        // parabole (si non dégénérée) : axe = direction propre nulle
        if det3.abs() <= eps3 {
            return parts; // dégénérée (2 droites, ou vide) : pas
                          // d'élément caractéristique
        }
        let tr = a + c;
        // vecteur propre de la valeur propre nulle
        let v0 = if a.abs() > eps2 {
            unit((b2, -a)).unwrap_or((1.0, 0.0))
        } else {
            (1.0, 0.0)
        };
        let v1 = perp(v0);
        let q = (d / 2.0, e / 2.0);
        let q0 = q.0 * v0.0 + q.1 * v0.1;
        let q1 = q.0 * v1.0 + q.1 * v1.1;
        if tr.abs() <= 1e-14 || q0.abs() <= 1e-14 * s {
            return parts; // cas dégénéré résiduel
        }
        // forme réduite : λ t² + 2q1 t + 2q0 s + f = 0, axe ∥ v0
        let tv = -q1 / tr;
        let sv = (q1 * q1 - f * tr) / (2.0 * tr * q0);
        let v = (sv * v0.0 + tv * v1.0, sv * v0.1 + tv * v1.1);
        // ouverture : s − sv = C·(t − tv)², C = −λ/(2q0)
        let ccst = -tr / (2.0 * q0);
        let g = if ccst > 0.0 { v0 } else { (-v0.0, -v0.1) };
        let fl = (q0.abs() / (2.0 * tr.abs())).max(0.0); // longueur focale
        parts.sommets = vec![v];
        parts.foyers = vec![(v.0 + fl * g.0, v.1 + fl * g.1)];
        parts.axes = vec![(v, g)];
        parts.directrices = vec![((v.0 - fl * g.0, v.1 - fl * g.1), v1)];
        return parts;
    }
    if det3.abs() <= eps3 {
        return parts; // dégénérée : pas d'élément caractéristique
    }
    let Some((o, l1, v1, l2, v2, g)) = conique_centre(a, b, c, d, e, f) else {
        return parts;
    };
    parts.centre = Some(o);
    parts.axes = vec![(o, v1), (o, v2)];
    if det > 0.0 {
        // ellipse (réelle si γ et λ de signes opposés)
        if g * l1 < 0.0 {
            let a1 = (-g / l1).sqrt();
            let a2 = (-g / l2).sqrt();
            let (vm, am, vi, ai) = if a1 >= a2 {
                (v1, a1, v2, a2)
            } else {
                (v2, a2, v1, a1)
            };
            let c_loc = (am * am - ai * ai).max(0.0).sqrt();
            parts.foyers = vec![
                (o.0 + c_loc * vm.0, o.1 + c_loc * vm.1),
                (o.0 - c_loc * vm.0, o.1 - c_loc * vm.1),
            ];
            parts.sommets = vec![
                (o.0 + am * vm.0, o.1 + am * vm.1),
                (o.0 - am * vm.0, o.1 - am * vm.1),
                (o.0 + ai * vi.0, o.1 + ai * vi.1),
                (o.0 - ai * vi.0, o.1 - ai * vi.1),
            ];
            if c_loc > 1e-12 {
                let dd = am * am / c_loc;
                parts.directrices = vec![
                    ((o.0 + dd * vm.0, o.1 + dd * vm.1), vi),
                    ((o.0 - dd * vm.0, o.1 - dd * vm.1), vi),
                ];
            }
        }
    } else {
        // hyperbole : l'axe réel porté par le vecteur propre de signe
        // opposé à γ ; a² = −γ/λ (axé), b² = γ/λ' (conjugué)
        let (vr, a_carre) = if g * l1 < 0.0 {
            (v1, -g / l1)
        } else {
            (v2, -g / l2)
        };
        let ar = a_carre.sqrt();
        let vi = perp(vr);
        let br = g / (if g * l1 < 0.0 { l2 } else { l1 });
        let br = br.max(0.0).sqrt();
        let c_loc = (ar * ar + br * br).sqrt();
        parts.sommets = vec![
            (o.0 + ar * vr.0, o.1 + ar * vr.1),
            (o.0 - ar * vr.0, o.1 - ar * vr.1),
        ];
        parts.foyers = vec![
            (o.0 + c_loc * vr.0, o.1 + c_loc * vr.1),
            (o.0 - c_loc * vr.0, o.1 - c_loc * vr.1),
        ];
        let dd = ar * ar / c_loc;
        parts.directrices = vec![
            ((o.0 + dd * vr.0, o.1 + dd * vr.1), vi),
            ((o.0 - dd * vr.0, o.1 - dd * vr.1), vi),
        ];
        // asymptotes : directions (±a ; ±b) dans la base (vr, vi)
        let v = (ar * vr.0 + br * vi.0, ar * vr.1 + br * vi.1);
        let v2 = (ar * vr.0 - br * vi.0, ar * vr.1 - br * vi.1);
        parts.asymptotes = vec![(o, v), (o, v2)];
    }
    parts
}

/// Racines réelles de a x² + d x + f (a ≠ 0 : deux racines ; a = 0 :
/// une racine ou aucune).
fn solve_quad(a: f64, d: f64, f: f64) -> Vec<f64> {
    if a.abs() <= 1e-15 {
        if d.abs() <= 1e-15 {
            return Vec::new();
        }
        let r = -f / d;
        return if r.is_finite() { vec![r] } else { Vec::new() };
    }
    let disc = d * d - 4.0 * a * f;
    if disc < 0.0 {
        return Vec::new();
    }
    let r = disc.sqrt();
    vec![(-d - r) / (2.0 * a), (-d + r) / (2.0 * a)]
}

/// Petite croix (marqueur de point calculé, comme les points du
/// schéma).
fn croix_con(p: &Page, q: V2, c: [u8; 3]) -> Vec<Piece> {
    let x = p.mm_x(q.0);
    let y = p.mm_y(q.1);
    if !x.is_finite() || !y.is_finite() {
        return Vec::new();
    }
    let r = 0.8;
    vec![
        Piece::Ligne {
            x0: x - r,
            y0: y,
            x1: x + r,
            y1: y,
            c,
            w: 0.35,
        },
        Piece::Ligne {
            x0: x,
            y0: y - r,
            x1: x,
            y1: y + r,
            c,
            w: 0.35,
        },
    ]
}

/// Options de traçage d'une conique (man. p. 35) : éléments dessinés
/// en plus de la courbe. Les textes calculés (excentricité,
/// équations…) ne sont pas restitués.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conique {
    pub def: ConDef,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "def_epaisseur")]
    pub epaisseur: f64,
    #[serde(default)]
    pub centre: bool,
    #[serde(default)]
    pub foyers: bool,
    #[serde(default)]
    pub directrices: bool,
    #[serde(default)]
    pub axes: bool,
    #[serde(default)]
    pub sommets: bool,
    #[serde(default)]
    pub asymptotes: bool,
}

impl Default for Conique {
    fn default() -> Self {
        Conique {
            def: ConDef::FoyerDirectrice {
                foyer: String::new(),
                directrice: Directrice::DeuxPoints {
                    a: String::new(),
                    b: String::new(),
                },
                e: 1.0,
            },
            couleur: def_couleur(),
            epaisseur: def_epaisseur(),
            centre: false,
            foyers: true,
            directrices: false,
            axes: false,
            sommets: true,
            asymptotes: false,
        }
    }
}

impl Conique {
    /// Parties calculées de la conique (branches en unités
    /// utilisateur + éléments caractéristiques).
    pub fn parties(&self, p: &Page, schema: &[ShObj]) -> ConicParts {
        match &self.def {
            ConDef::FoyerDirectrice {
                foyer,
                directrice,
                e,
            } => match conique_geo_donnees(foyer, directrice, *e, schema) {
                Some((f, n, h, e)) => {
                    let diag_norm = p.unit_mm.max(0.01);
                    let diag = p.width_mm.hypot(p.height_mm);
                    let r_max = diag * 1.4 / diag_norm;
                    conique_geo(f, n, h, e, r_max)
                }
                None => ConicParts::default(),
            },
            ConDef::Cartesienne { a, b, c, d, e, f } => conique_cart(*a, *b, *c, *d, *e, *f, p),
        }
    }

    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let parts = self.parties(p, schema);
        let mut out = Vec::new();
        for br in &parts.branches {
            let mut mm = Vec::with_capacity(br.len());
            for (x, y) in br {
                mm.push((p.mm_x(*x), p.mm_y(*y)));
            }
            if mm.iter().any(|q| !q.0.is_finite() || !q.1.is_finite()) {
                continue;
            }
            out.push(Piece::Poly {
                pts: mm,
                fill: None,
                stroke: Some((self.couleur, self.epaisseur)),
            });
        }
        // options de traçage (man. p. 35)
        for q in &parts.foyers {
            if self.foyers {
                out.extend(croix_con(p, *q, self.couleur));
            }
        }
        for q in &parts.sommets {
            if self.sommets {
                out.extend(croix_con(p, *q, self.couleur));
            }
        }
        if let (true, Some(o)) = (self.centre, parts.centre) {
            out.extend(croix_con(p, o, self.couleur));
        }
        if self.axes {
            for (o, v) in &parts.axes {
                out.extend(ligne_infinie(
                    p,
                    *o,
                    *v,
                    false,
                    TraitStyle::Plein,
                    self.couleur,
                    self.epaisseur,
                ));
            }
        }
        if self.directrices {
            for (o, v) in &parts.directrices {
                out.extend(ligne_infinie(
                    p,
                    *o,
                    *v,
                    false,
                    TraitStyle::Pointille,
                    self.couleur,
                    self.epaisseur,
                ));
            }
        }
        if self.asymptotes {
            for (o, v) in &parts.asymptotes {
                out.extend(ligne_infinie(
                    p,
                    *o,
                    *v,
                    false,
                    TraitStyle::Pointille,
                    self.couleur,
                    self.epaisseur,
                ));
            }
        }
        out
    }
}

/// Expression mathématique en code LaTeX (man. p. 41-43, v1.6) :
/// préfixe de taille `4$`, couleurs \blue \red …, \frac, \sqrt,
/// exposants et indices, ancrée par ses coordonnées (baseline).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TexSchema {
    pub x: f64,
    pub y: f64,
    pub code: String,
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
}

impl Default for TexSchema {
    fn default() -> Self {
        TexSchema {
            x: 0.0,
            y: 0.0,
            code: String::new(),
            couleur: def_couleur(),
        }
    }
}

impl TexSchema {
    pub fn draw_list(&self, p: &Page) -> Vec<Piece> {
        if self.code.trim().is_empty() {
            return Vec::new();
        }
        crate::tex::rendu(p, self.x, self.y, &self.code, self.couleur)
    }
}

// ==================================================================
// Rapporteur trigonométrique ou géométrique (man. p. 87-88)

/// Genre de rapporteur (man. p. 88) : lycée dessine un rapporteur
/// trigonométrique (graduations dans le sens trigonométrique,
/// degrés et radians) ; collège un rapporteur géométrique
/// (graduation extérieure dans le sens horaire, comme une règle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenreRap {
    Lycee,
    College,
}

/// Rapporteur dessiné en haute résolution (imprimable sur calque).
/// Le centre est donné en mm de page et le rayon en millimètres
/// vrais (man. p. 88 : 40 à 100 mm), indépendants du repère.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rapporteur {
    /// centre (mm page, Y vers le bas)
    pub c: [f64; 2],
    /// rayon en mm vrais, borné à 40..=100 au dessin
    pub rayon: f64,
    /// lycée (trigonométrique) ou collège (géométrique)
    pub genre: GenreRap,
    /// graduations en degrés
    #[serde(default = "def_couleur")]
    pub couleur: [u8; 3],
    #[serde(default = "vrai")]
    pub degres: bool,
    /// graduations en radians (lycée)
    #[serde(default = "vrai")]
    pub radians: bool,
    /// quadrillage intérieur (lycée)
    #[serde(default)]
    pub quadrillage: bool,
    /// occulter le cercle extérieur pour obtenir un cercle
    /// trigonométrique simple (lycée, man. p. 88)
    #[serde(default)]
    pub cercle_trigo: bool,
    /// valeurs remarquables : rayons pointillés aux angles
    /// principales (30°, 45°, 60° et symétriques ; man. p. 88)
    #[serde(default)]
    pub valeurs: bool,
    /// collège : demi-cercle (dôme vers le haut, corde à plat)
    /// ou cercle entier
    #[serde(default = "vrai")]
    pub demi: bool,
}

fn vrai() -> bool {
    true
}

fn deg(i: f64) -> f64 {
    i.to_radians()
}

/// Multiple de 15° retenus pour les étiquettes radians : les
/// angles remarquables (multiples de 30° et de 45°).
const RADIANS_TENUS: [usize; 16] = [0, 2, 3, 4, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20, 21, 22];

/// Textes d'angle en radians pour les multiples de 15°.
const RADIANS_TXT: [&str; 24] = [
    "0", "π/12", "π/6", "π/4", "π/3", "5π/12", "π/2", "7π/12", "2π/3", "3π/4", "5π/6", "11π/12",
    "π", "13π/12", "7π/6", "5π/4", "4π/3", "17π/12", "3π/2", "19π/12", "5π/3", "7π/4", "11π/6",
    "23π/12",
];

impl Default for Rapporteur {
    fn default() -> Self {
        Rapporteur {
            c: [85.0, 60.0],
            rayon: 50.0,
            genre: GenreRap::Lycee,
            degres: true,
            radians: true,
            quadrillage: false,
            cercle_trigo: false,
            valeurs: false,
            demi: false,
            couleur: def_couleur(),
        }
    }
}

impl Rapporteur {
    /// Point (mm page) de l'angle d en degrés sur le cercle de rayon
    /// r, selon le genre : lycée = sens trigonométrique depuis
    /// l'est ; collège = sens horaire, 0 à l'ouest (comme la lecture
    /// d'une règle de gauche à droite).
    fn point(&self, r: f64, d: f64) -> (f64, f64) {
        let e = match self.genre {
            GenreRap::Lycee => d,
            GenreRap::College => 180.0 - d, // sens horaire depuis l'ouest
        };
        let (cs, si) = (deg(e).cos(), deg(e).sin());
        (self.c[0] + r * cs, self.c[1] - r * si)
    }

    /// Nombre de degrés dessinés (entier : demi-cercle collège 180).
    fn amplitude(&self) -> f64 {
        match self.genre {
            GenreRap::Lycee => 360.0,
            GenreRap::College if self.demi => 180.0,
            GenreRap::College => 360.0,
        }
    }

    pub fn draw_list(&self, _p: &Page) -> Vec<Piece> {
        let r = self.rayon.clamp(40.0, 100.0);
        let [cx, cy] = self.c;
        if !(cx.is_finite() && cy.is_finite()) {
            return Vec::new();
        }
        let col = self.couleur;
        let lycee = self.genre == GenreRap::Lycee;
        let mut v: Vec<Piece> = Vec::new();
        let arc = |r: f64, d0: f64, d1: f64, w: f64| -> Piece {
            // arc interpolé (d0 < d1, degrés du genre) en polyline mm
            let n = ((d1 - d0) / 2.0).ceil().max(2.0) as usize;
            let pts: Vec<(f64, f64)> = (0..=n)
                .map(|k| self.point(r, d0 + (d1 - d0) * k as f64 / n as f64))
                .collect();
            Piece::Poly {
                pts,
                fill: None,
                stroke: Some((col, w)),
            }
        };
        let amp = self.amplitude();
        let plein = !matches!((lycee, self.demi), (false, true));
        // ---- interne : quadrillage et axes ; lycée, hors cercle trigo simple
        if lycee {
            if self.quadrillage && !self.cercle_trigo {
                // cercles concentriques aux quarts de rayon
                for k in [0.25, 0.5, 0.75] {
                    v.push(arc(r * k, 0.0, 360.0, 0.15));
                }
                // rayards tous les 15° (fins)
                for d in (0..24).map(|a| a as f64 * 15.0) {
                    let (x, y) = self.point(r * (15.0 / 16.0), d);
                    v.push(Piece::Ligne {
                        x0: cx,
                        y0: cy,
                        x1: x,
                        y1: y,
                        c: col,
                        w: 0.15,
                    });
                }
            }
            // axes horizontaux et verticaux traversants
            v.push(Piece::Ligne {
                x0: cx - r,
                y0: cy,
                x1: cx + r,
                y1: cy,
                c: col,
                w: 0.25,
            });
            v.push(Piece::Ligne {
                x0: cx,
                y0: cy - r,
                x1: cx,
                y1: cy + r,
                c: col,
                w: 0.25,
            });
            // valeurs remarquables : rayons pointillés 30°/45°/60°…
            if self.valeurs {
                for d in [
                    30.0, 45.0, 60.0, 120.0, 135.0, 150.0, 210.0, 225.0, 240.0, 300.0, 315.0, 330.0,
                ] {
                    let (x, y) = self.point(r, d);
                    v.push(Piece::Pointille {
                        x0: cx,
                        y0: cy,
                        x1: x,
                        y1: y,
                        c: col,
                        w: 0.2,
                    });
                }
            }
        } else if self.demi {
            // corde du demi-cercle, à plat
            v.push(Piece::Ligne {
                x0: cx - r,
                y0: cy,
                x1: cx + r,
                y1: cy,
                c: col,
                w: 0.3,
            });
            // croix du centre + crans pour le repérage
            v.push(Piece::Ligne {
                x0: cx - 1.5,
                y0: cy,
                x1: cx + 1.5,
                y1: cy,
                c: col,
                w: 0.3,
            });
        }
        // ---- graduations, sauf cercle trigonométrique simple
        if !(lycee && self.cercle_trigo) {
            let tourne = amp;
            for k in 0..=tourne as i64 {
                let d = k as f64;
                // longueur du cran : 1° court, 5° moyen, 10° long
                let long = if k % 10 == 0 {
                    3.5
                } else if k % 5 == 0 {
                    2.5
                } else {
                    1.5
                };
                let (x0, y0) = self.point(r, d);
                let (x1, y1) = self.point(r - long, d);
                v.push(Piece::Ligne {
                    x0,
                    y0,
                    x1,
                    y1,
                    c: col,
                    w: 0.2,
                });
            }
        }
        // ---- cercle extérieur (ou simple cercle en mode trigo)
        v.push(arc(r, 0.0, if plein { 360.0 } else { 180.0 }, 0.4));
        // ---- étiquettes de degrés
        if self.degres && !(lycee && self.cercle_trigo) {
            for k in (0..=amp as i64).step_by(10) {
                if plein && k == amp as i64 {
                    continue; // 360 superposé à 0
                }
                let lb = if k == 0 { "0" } else { &format!("{k}")[..] };
                let (x, y) = self.point(r - 6.5, k as f64);
                v.push(Piece::Texte {
                    mm: (x, y + 1.1),
                    txt: lb.to_string(),
                    c: col,
                    taille: 2.2,
                    ancre: "middle",
                    vertical: false,
                    gras: false,
                    italique: false,
                });
            }
        }
        // ---- étiquettes de radians (lycée)
        if lycee && self.radians && !self.cercle_trigo {
            for &k in RADIANS_TENUS.iter() {
                let d = k as f64 * 15.0;
                if d >= 360.0 {
                    continue;
                }
                let txt = if k == 0 {
                    if self.degres {
                        continue; // 0° déjà étiqueté
                    } else {
                        "0"
                    }
                } else {
                    RADIANS_TXT[k]
                };
                let (x, y) = self.point(r - 13.0, d);
                v.push(Piece::Texte {
                    mm: (x, y + 1.1),
                    txt: txt.to_string(),
                    c: col,
                    taille: 2.2,
                    ancre: "middle",
                    vertical: false,
                    gras: false,
                    italique: false,
                });
            }
        }
        v
    }
}

// ==================================================================
// Routage (enum, dessin de la liste)
// ==================================================================

/// Objet du schéma (les points d'abord, comme l'original exige que les
/// points soient définis avant la courbe qui les utilise).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShObj {
    Pt(PointSchema),
    Bez(Bezier),
    Seg(Segment),
    Vec(Vecteur),
    Dro(Droite),
    Demi(DemiDroite),
    Cer(Cercle),
    Ell(Ellipse),
    Pgn(Polygone),
    Ang(Angle),
    Zon(Zone),
    Txt(TexteSchema),
    Con(Conique),
    Tex(TexSchema),
    Rap(Rapporteur),
}

impl ShObj {
    pub fn draw_list(&self, p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        match self {
            ShObj::Pt(pt) => pt.draw_list(p),
            ShObj::Bez(b) => b.draw_list(p, schema),
            ShObj::Seg(s) => s.draw_list(p, schema),
            ShObj::Vec(v) => v.draw_list(p, schema),
            ShObj::Dro(d) => d.draw_list(p, schema),
            ShObj::Demi(d) => d.draw_list(p, schema),
            ShObj::Cer(c) => c.draw_list(p, schema),
            ShObj::Ell(e) => e.draw_list(p, schema),
            ShObj::Pgn(g) => g.draw_list(p, schema),
            ShObj::Ang(a) => a.draw_list(p, schema),
            ShObj::Zon(z) => z.draw_list(p, schema),
            ShObj::Txt(t) => t.draw_list(p),
            ShObj::Con(c) => c.draw_list(p, schema),
            ShObj::Tex(x) => x.draw_list(p),
            ShObj::Rap(r) => r.draw_list(p),
        }
    }

    /// Dessin de tout le schéma, dans l'ordre voulu par l'original :
    /// les zones coloriées d'abord (fond), puis les objets dans l'ordre
    /// de la liste, puis les textes au premier plan (man. p. 40).
    pub fn draw_schema(p: &Page, schema: &[ShObj]) -> Vec<Piece> {
        let mut out = Vec::new();
        for o in schema {
            if let ShObj::Zon(_) = o {
                out.extend(o.draw_list(p, schema));
            }
        }
        for o in schema {
            if !matches!(o, ShObj::Zon(_) | ShObj::Txt(_) | ShObj::Tex(_)) {
                out.extend(o.draw_list(p, schema));
            }
        }
        for o in schema {
            if let ShObj::Txt(_) | ShObj::Tex(_) = o {
                out.extend(o.draw_list(p, schema));
            }
        }
        out
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

    // --------------------------------------------------------------
    // v1.5 : segment, vecteur, droite, demi-droite, cercle, ellipse,
    // polygone, angle, zone, texte, ordre de dessin
    // --------------------------------------------------------------

    fn schema15() -> Vec<ShObj> {
        let mk = |nom: &str, x: f64, y: f64| {
            ShObj::Pt(PointSchema {
                nom: nom.into(),
                x,
                y,
            })
        };
        vec![
            mk("A", 0.0, 0.0),
            mk("B", 1.0, 0.0),
            mk("C", 0.0, 1.0),
            mk("D", 2.0, 1.0),
        ]
    }

    #[test]
    fn segment_mediane_hauteur_manquant() {
        let p = Page::default();
        let schema = schema15();
        // médiane issue de A dans le triangle ABC : A → milieu de [B,C]
        let m = Segment {
            def: SegDef::Mediane {
                tri: ["A".into(), "B".into(), "C".into()],
                de: 0,
            },
            ..Default::default()
        };
        let pieces = m.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 1);
        let PieceS::Ligne { x0, y0, x1, y1, .. } = &pieces[0] else {
            unreachable!()
        };
        assert!((*x0 - p.mm_x(0.0)).abs() < 1e-9);
        assert!((*y0 - p.mm_y(0.0)).abs() < 1e-9);
        let milieu = (p.mm_x(0.5), p.mm_y(0.5));
        assert!((*x1 - milieu.0).abs() < 1e-9, "{x1}");
        assert!((*y1 - milieu.1).abs() < 1e-9, "{y1}");
        // hauteur issue de C sur [AB] : pied en A (triangle rectangle en
        // A), le segment est vertical
        let h = Segment {
            def: SegDef::Hauteur {
                tri: ["A".into(), "B".into(), "C".into()],
                de: 2,
            },
            ..Default::default()
        };
        let pieces = h.draw_list(&p, &schema);
        let PieceS::Ligne { x0, x1, .. } = &pieces[0] else {
            unreachable!()
        };
        assert!((*x0 - *x1).abs() < 1e-9, "verticale : {x0} {x1}");
        // point absent → rien
        let s = Segment {
            def: SegDef::DeuxPoints {
                a: "A".into(),
                b: "Z".into(),
            },
            ..Default::default()
        };
        assert!(s.draw_list(&p, &schema).is_empty());
    }

    #[test]
    fn segment_extremites_fleche_et_rond() {
        let p = Page::default();
        let schema = schema15();
        let s = Segment {
            def: SegDef::DeuxPoints {
                a: "A".into(),
                b: "B".into(),
            },
            debut: Extremite::Fleche,
            fin: Extremite::Rond,
            ..Default::default()
        };
        let pieces = s.draw_list(&p, &schema);
        // trait + 2 traits de la pointe + 1 rond plein
        assert_eq!(pieces.len(), 4);
        assert!(matches!(pieces[0], PieceS::Ligne { .. }));
        assert!(matches!(pieces[1], PieceS::Ligne { .. }));
        assert!(matches!(pieces[2], PieceS::Ligne { .. }));
        let PieceS::Disque { r, .. } = &pieces[3] else {
            unreachable!()
        };
        assert!((*r - 0.45).abs() < 1e-9);
    }

    #[test]
    fn vecteur_coordonnees_point_et_nom() {
        let p = Page::default();
        let schema = schema15();
        // vecteur libre depuis A, nom affiché : trait + 2 traits de
        // pointe + nom
        let v = Vecteur {
            nom: "u".into(),
            afficher_nom: true,
            depart: "A".into(),
            u: 1.0,
            v: 1.0,
            ..Default::default()
        };
        let pieces = v.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 4);
        let PieceS::Texte { txt, italique, .. } = &pieces[3] else {
            unreachable!()
        };
        assert_eq!(txt, "u");
        assert!(*italique);
        // arrivée sur un point nommé : le trait va de A à C
        let v2 = Vecteur {
            depart: "A".into(),
            vers: "C".into(),
            ..Default::default()
        };
        let pieces = v2.draw_list(&p, &schema);
        let PieceS::Ligne { y1, .. } = &pieces[0] else {
            unreachable!()
        };
        assert!((*y1 - p.mm_y(1.0)).abs() < 1e-9);
        // dessiner = false → rien du tout
        let mut v3 = v.clone();
        v3.dessiner = false;
        assert!(v3.draw_list(&p, &schema).is_empty());
    }

    #[test]
    fn droite_sept_definitions() {
        let p = Page::default();
        let mk_pt = |nom: &str, x: f64, y: f64| {
            ShObj::Pt(PointSchema {
                nom: nom.into(),
                x,
                y,
            })
        };
        let d1 = Droite {
            nom: "(D1)".into(),
            def: DroDef::PtDir {
                pt: "A".into(),
                dir: DirSpec::DeuxPoints {
                    a: "A".into(),
                    b: "B".into(),
                },
            },
            ..Default::default()
        };
        let schema = vec![
            mk_pt("A", 0.0, 0.0),
            mk_pt("B", 1.0, 0.0),
            mk_pt("C", 0.0, 1.0),
            ShObj::Dro(d1.clone()),
        ];
        // 1. point + direction : horizontale clippée aux bords de la page
        let pieces = d1.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 1);
        let PieceS::Ligne { x0, y0, x1, y1, .. } = &pieces[0] else {
            unreachable!()
        };
        assert!((*y0 - *y1).abs() < 1e-9, "horizontale");
        assert!(*x0 < p.mm_x(0.0) && *x1 > p.mm_x(1.0));
        // 1b. idem sans nom (résolution directe)
        let anon = Droite {
            nom: String::new(),
            ..d1.clone()
        };
        assert_eq!(anon.draw_list(&p, &schema).len(), 1);
        // 2. point + vecteur normal : verticale en A
        let d2 = Droite {
            def: DroDef::PtNorm {
                pt: "A".into(),
                dir: DirSpec::Vecteur { u: 1.0, v: 0.0 },
            },
            ..Default::default()
        };
        let PieceS::Ligne { x0, x1, .. } = &d2.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*x0 - p.mm_x(0.0)).abs() < 1e-9 && (*x1 - p.mm_x(0.0)).abs() < 1e-9);
        // 3. parallèle à (D1) passant par C
        let d3 = Droite {
            def: DroDef::Parallele {
                pt: "C".into(),
                de: "(D1)".into(),
            },
            ..Default::default()
        };
        let PieceS::Ligne { y0, y1, .. } = &d3.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*y0 - p.mm_y(1.0)).abs() < 1e-9 && (*y1 - p.mm_y(1.0)).abs() < 1e-9);
        // 4. perpendiculaire à (D1) passant par A
        let d4 = Droite {
            def: DroDef::Perpendiculaire {
                pt: "A".into(),
                de: "(D1)".into(),
            },
            ..Default::default()
        };
        let PieceS::Ligne { x0, x1, .. } = &d4.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*x0 - p.mm_x(0.0)).abs() < 1e-9 && (*x1 - p.mm_x(0.0)).abs() < 1e-9);
        // 5. médiatrice de [A,B] : x = 0.5
        let d5 = Droite {
            def: DroDef::Mediatrice {
                a: "A".into(),
                b: "B".into(),
            },
            ..Default::default()
        };
        let PieceS::Ligne { x0, .. } = &d5.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*x0 - p.mm_x(0.5)).abs() < 1e-9);
        // 6. bissectrice de l'angle en A ([AB) et [AC)) : direction
        //    (1/√2, 1/√2) — le sommet est le 2ᵉ point du triplet
        let d6 = Droite {
            def: DroDef::Bissectrice {
                tri: ["B".into(), "A".into(), "C".into()],
            },
            ..Default::default()
        };
        let r = d6.resoudre(&carte_des_points(&schema), &[]).unwrap();
        assert!(r.p0.0.abs() < 1e-9 && r.p0.1.abs() < 1e-9);
        assert!((r.v.0 - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        assert!((r.v.1 - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        // 6b. dessin : clippée, une seule pièce
        assert_eq!(d6.draw_list(&p, &schema).len(), 1);
        // 7. droite nommée manquante → rien
        let d7 = Droite {
            def: DroDef::Parallele {
                pt: "A".into(),
                de: "(ZZZ)".into(),
            },
            ..Default::default()
        };
        assert!(d7.draw_list(&p, &schema).is_empty());
    }

    #[test]
    fn demi_droite_point_vecteur_angle() {
        let p = Page::default();
        let schema = schema15();
        // angle polaire 45° depuis A : part de A vers le haut à droite
        // (mm_y décroît quand y augmente)
        let d = DemiDroite {
            origine: "A".into(),
            def: DemiDef::Angle { deg: 45.0 },
            ..Default::default()
        };
        let pieces = d.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 1);
        let PieceS::Ligne { x0, y0, x1, y1, .. } = &pieces[0] else {
            unreachable!()
        };
        assert!((*x0 - p.mm_x(0.0)).abs() < 1e-9 && (*y0 - p.mm_y(0.0)).abs() < 1e-9);
        assert!(*x1 > *x0 && *y1 < *y0, "{x0} {y0} -> {x1} {y1}");
        // par point nommé : [AC) verticale montante
        let d2 = DemiDroite {
            origine: "A".into(),
            def: DemiDef::Point { dir: "C".into() },
            ..Default::default()
        };
        let PieceS::Ligne { x1, .. } = &d2.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*x1 - p.mm_x(0.0)).abs() < 1e-9);
        // par vecteur : [AB) horizontale
        let d3 = DemiDroite {
            origine: "A".into(),
            def: DemiDef::Vecteur { u: 1.0, v: 0.0 },
            ..Default::default()
        };
        let PieceS::Ligne { y1, .. } = &d3.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*y1 - p.mm_y(0.0)).abs() < 1e-9);
        // origine manquante → rien
        let mut d4 = d.clone();
        d4.origine = "Z".into();
        assert!(d4.draw_list(&p, &schema).is_empty());
    }

    #[test]
    fn cercle_trois_definitions() {
        let p = Page::default();
        let schema = schema15();
        // centre + rayon 2 : disque r = 2·unité, bord + remplissage
        let c = Cercle {
            def: CerDef::CentreRayon {
                c: "A".into(),
                r: 2.0,
            },
            remplir: Some([255, 240, 200]),
            ..Default::default()
        };
        let pieces = c.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 1);
        let PieceS::Disque {
            r, fill, stroke, ..
        } = &pieces[0]
        else {
            unreachable!()
        };
        assert!((*r - 2.0 * p.unit_mm).abs() < 1e-9, "{}", *r);
        assert_eq!(*fill, [255, 240, 200]);
        assert!(stroke.is_some());
        // centre + point du cercle : [AB] de longueur 1
        let c2 = Cercle {
            def: CerDef::CentrePoint {
                c: "A".into(),
                p: "B".into(),
            },
            ..Default::default()
        };
        let PieceS::Disque { r, .. } = &c2.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*r - p.unit_mm).abs() < 1e-9);
        // deux points diamétraux [A,B] : rayon 0.5
        let c3 = Cercle {
            def: CerDef::Diametral {
                a: "A".into(),
                b: "B".into(),
            },
            ..Default::default()
        };
        let PieceS::Disque { r, .. } = &c3.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert!((*r - 0.5 * p.unit_mm).abs() < 1e-9);
        // rayon nul, point absent → rien
        let c4 = Cercle {
            def: CerDef::CentreRayon {
                c: "A".into(),
                r: 0.0,
            },
            ..Default::default()
        };
        assert!(c4.draw_list(&p, &schema).is_empty());
        let c5 = Cercle {
            def: CerDef::CentreRayon {
                c: "Z".into(),
                r: 1.0,
            },
            ..Default::default()
        };
        assert!(c5.draw_list(&p, &schema).is_empty());
    }

    #[test]
    fn ellipse_axes_et_rectangle() {
        let p = Page::default();
        let schema = schema15();
        let e = Ellipse {
            def: EllDef::CentreAxes {
                c: "A".into(),
                rx: 2.0,
                ry: 1.0,
            },
            remplir: Some([220, 250, 210]),
            ..Default::default()
        };
        let pieces = e.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 1);
        let PieceS::Poly { pts, fill, stroke } = &pieces[0] else {
            unreachable!()
        };
        assert_eq!(pts.len(), N_ELL + 1);
        assert_eq!(*fill, Some([220, 250, 210]));
        assert!(stroke.is_some());
        // t = 0 → point le plus à droite : (A.x + rx, A.y)
        assert!((pts[0].0 - p.mm_x(2.0)).abs() < 1e-9);
        assert!((pts[0].1 - p.mm_y(0.0)).abs() < 1e-9);
        // rectangle A→C (coins opposés (0,0) et (0,1)) : rx = 0 → rien
        let e2 = Ellipse {
            def: EllDef::Rectangle {
                a: "A".into(),
                b: "C".into(),
            },
            ..Default::default()
        };
        assert!(e2.draw_list(&p, &schema).is_empty());
        // rectangle A→D ((0,0) et (2,1)) : rx = 1, ry = 0.5
        let e3 = Ellipse {
            def: EllDef::Rectangle {
                a: "A".into(),
                b: "D".into(),
            },
            ..Default::default()
        };
        let PieceS::Poly { pts, .. } = &e3.draw_list(&p, &schema)[0] else {
            unreachable!()
        };
        assert_eq!(pts.len(), N_ELL + 1);
        // t = 0 → sommet droit (info : centre (1 ; 0.5), rx = 1)
        assert!((pts[0].0 - p.mm_x(2.0)).abs() < 1e-9);
        assert!((pts[0].1 - p.mm_y(0.5)).abs() < 1e-9);
        // point bas de l'ellipse : (centre ; ry = 0.5)
        let bas = p.mm_y(0.0);
        assert!(pts.iter().any(|q| q.0 == p.mm_x(1.0) && q.1 == bas));
    }

    #[test]
    fn polygones_preset_regulier_parallelogramme() {
        let schema = schema15();
        let carte = carte_des_points(&schema);
        // carré direct sur [A,B] : croix des 4 sommets (8 traits) +
        // contour ; sommets (0,0) (1,0) (1,1) (0,1)
        let car = Polygone {
            def: PgnDef::Predefini {
                nature: PresetPoly::Carre,
                a: "A".into(),
                b: "B".into(),
                h: 0.0,
            },
            ..Default::default()
        };
        let pieces = car.draw_list(&Page::default(), &schema);
        assert_eq!(pieces.len(), 9);
        let PieceS::Poly { pts, .. } = &pieces[8] else {
            unreachable!()
        };
        let p = Page::default();
        assert!((pts[2].0 - p.mm_x(1.0)).abs() < 1e-9);
        assert!((pts[2].1 - p.mm_y(1.0)).abs() < 1e-9);
        // pentagone régulier posé sur [A,B] : 5 sommets, 5 côtés de
        // longueur |AB| = 1
        let pent = Polygone {
            def: PgnDef::Regulier {
                a: "A".into(),
                b: "B".into(),
                n: 5,
            },
            sommets: false,
            ..Default::default()
        };
        let sg = pent.sommets_calcules(&carte).unwrap();
        assert_eq!(sg.len(), 5);
        for k in 0..5 {
            let d = compo(sg[k], sg[(k + 1) % 5]);
            assert!((d.0.hypot(d.1) - 1.0).abs() < 1e-9);
        }
        // équilatéral posé sur [A,B] : 3ᵉ sommet à hauteur √3/2
        let equi = Polygone {
            def: PgnDef::Predefini {
                nature: PresetPoly::Equilateral,
                a: "A".into(),
                b: "B".into(),
                h: 0.0,
            },
            sommets: false,
            ..Default::default()
        };
        let sg = equi.sommets_calcules(&carte).unwrap();
        assert!((sg[2].1 - 3.0f64.sqrt() / 2.0).abs() < 1e-9);
        // parallélogramme ABC : 4ᵉ sommet = A − B + C = (−1, 1)
        let par = Polygone {
            def: PgnDef::Parallelogramme {
                a: "A".into(),
                b: "B".into(),
                c: "C".into(),
            },
            sommets: false,
            ..Default::default()
        };
        let sg = par.sommets_calcules(&carte).unwrap();
        assert_eq!(sg[3], (-1.0, 1.0));
        // polygone quelconque par noms, puis point manquant
        let quel = Polygone {
            def: PgnDef::Noms {
                sommets: vec!["A".into(), "B".into(), "D".into(), "C".into()],
            },
            sommets: false,
            ..Default::default()
        };
        assert_eq!(quel.sommets_calcules(&carte).unwrap().len(), 4);
        let rien = Polygone {
            def: PgnDef::Noms {
                sommets: vec!["A".into(), "Z".into(), "C".into()],
            },
            sommets: false,
            ..Default::default()
        };
        assert!(rien.sommets_calcules(&carte).is_none());
    }

    #[test]
    fn angle_sommet_rayon_et_balayage() {
        let schema = schema15();
        let carte = carte_des_points(&schema);
        // angle droit en A entre [AB) et [AC) : rayon auto = moitié de
        // la plus courte des branches = 0.5 ; balayage +π/2
        let a = Angle {
            sommet: "A".into(),
            a: "B".into(),
            b: "C".into(),
            ..Default::default()
        };
        let (s, t1, delta, r) = a.resoudre(&carte).unwrap();
        assert_eq!(s, (0.0, 0.0));
        assert!(t1.abs() < 1e-12);
        assert!((delta - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((r - 0.5).abs() < 1e-12);
        // arc : 16 petits traits
        assert_eq!(a.draw_list(&Page::default(), &schema).len(), 16);
        // rayon imposé
        let a2 = Angle {
            rayon: 0.3,
            ..a.clone()
        };
        let (_, _, _, r) = a2.resoudre(&carte).unwrap();
        assert!((r - 0.3).abs() < 1e-12);
        // balayage le plus court : de [AC) vers [AB) → delta = −π/2
        let a3 = Angle {
            sommet: "A".into(),
            a: "C".into(),
            b: "B".into(),
            ..Default::default()
        };
        let (_, _, delta, _) = a3.resoudre(&carte).unwrap();
        assert!((delta + std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        // sommet absent → rien
        let mut a4 = a.clone();
        a4.sommet = "Z".into();
        assert!(a4.draw_list(&Page::default(), &schema).is_empty());
    }

    #[test]
    fn zone_et_texte() {
        let p = Page::default();
        let schema = schema15();
        let z = Zone {
            sommets: vec!["A".into(), "B".into(), "D".into(), "C".into()],
            couleur: [250, 220, 160],
        };
        let pieces = z.draw_list(&p, &schema);
        assert_eq!(pieces.len(), 1);
        let PieceS::Poly { pts, fill, stroke } = &pieces[0] else {
            unreachable!()
        };
        assert_eq!(pts.len(), 4);
        assert_eq!(*fill, Some([250, 220, 160]));
        assert!(stroke.is_none());
        // point manquant → rien ; moins de 3 sommets → rien
        let z2 = Zone {
            sommets: vec!["A".into(), "Z".into(), "C".into()],
            ..z.clone()
        };
        assert!(z2.draw_list(&p, &schema).is_empty());
        let z3 = Zone {
            sommets: vec!["A".into(), "B".into()],
            ..z.clone()
        };
        assert!(z3.draw_list(&p, &schema).is_empty());
        // texte ordinaire : posé à (x, y), gras demandé
        let t = TexteSchema {
            x: 1.0,
            y: 2.0,
            texte: "titre du schema".into(),
            gras: true,
            ..Default::default()
        };
        let pieces = t.draw_list(&p);
        assert_eq!(pieces.len(), 1);
        let PieceS::Texte { mm, txt, gras, .. } = &pieces[0] else {
            unreachable!()
        };
        assert!((mm.0 - p.mm_x(1.0)).abs() < 1e-9);
        assert!((mm.1 - p.mm_y(2.0)).abs() < 1e-9);
        assert_eq!(txt, "titre du schema");
        assert!(*gras);
        // texte vide → rien
        let t2 = TexteSchema {
            texte: String::new(),
            x: 1.0,
            y: 2.0,
            ..Default::default()
        };
        assert!(t2.draw_list(&p).is_empty());
        // indices automatiques (man. p. 40) : a1 → base + chiffre
        // romain en indice à 70 %
        let t = TexteSchema {
            x: 0.0,
            y: 0.0,
            texte: "a1".into(),
            italique: true,
            ..Default::default()
        };
        let pieces = t.draw_list(&p);
        assert_eq!(pieces.len(), 2);
        let PieceS::Texte {
            txt,
            taille,
            italique,
            ..
        } = &pieces[0]
        else {
            unreachable!()
        };
        assert_eq!(txt, "a");
        assert_eq!(*taille, 3.2);
        assert!(*italique);
        let PieceS::Texte {
            txt,
            taille,
            italique,
            gras,
            ..
        } = &pieces[1]
        else {
            unreachable!()
        };
        assert_eq!(txt, "1");
        assert!((*taille - 3.2 * 0.7).abs() < 1e-9);
        assert!(!*italique, "le chiffre en indice reste romain");
        assert!(!*gras);
        // Cf → indice italique
        let t = TexteSchema {
            x: 0.0,
            y: 0.0,
            texte: "Cf".into(),
            ..Default::default()
        };
        let pieces = t.draw_list(&p);
        let PieceS::Texte { italique, .. } = &pieces[1] else {
            unreachable!()
        };
        assert!(*italique, "l'indice de Cf est italique");
        // fond d'arrière-plan : rect posé avant le texte
        let t = TexteSchema {
            x: 1.0,
            y: 2.0,
            texte: "salut".into(),
            fond: Some([250, 220, 160]),
            // texte de 5 caractères : pas d'indice auto
            souligne: true,
            ..Default::default()
        };
        let pieces = t.draw_list(&p);
        // fond + texte + souligné
        assert_eq!(pieces.len(), 3);
        assert!(matches!(pieces[0], PieceS::Rect { .. }));
        assert!(matches!(pieces[1], PieceS::Texte { .. }));
        assert!(matches!(pieces[2], PieceS::Ligne { .. }));
    }

    #[test]
    fn ordre_dessin_zones_puis_objets_puis_textes() {
        let p = Page::default();
        let mut schema = vec![
            ShObj::Txt(TexteSchema {
                x: 1.0,
                y: 2.0,
                texte: "t".into(),
                ..Default::default()
            }),
            ShObj::Ang(Angle {
                sommet: "A".into(),
                a: "B".into(),
                b: "C".into(),
                ..Default::default()
            }),
            ShObj::Zon(Zone {
                sommets: vec!["A".into(), "B".into(), "D".into()],
                ..Default::default()
            }),
        ];
        schema.extend(schema15());
        let pieces = ShObj::draw_schema(&p, &schema);
        // contenu : zone (1), angle (16), 4 points (2 traits + nom),
        // texte ordinaire en dernier mais aussi en tête de liste
        assert_eq!(pieces.len(), 30);
        // zone d'abord (fond)
        assert!(matches!(pieces[0], PieceS::Poly { .. }));
        // puis les objets dans l'ordre de la liste : l'angle (16 traits)
        for piece in &pieces[1..17] {
            assert!(matches!(piece, PieceS::Ligne { .. }));
        }
        // … suivis des points (2 traits + nom chacun)
        assert!(matches!(pieces[19], PieceS::Texte { .. }));
        assert!(matches!(pieces[22], PieceS::Texte { .. }));
        // les textes ordinaires sont au premier plan, toujours derniers
        let PieceS::Texte { txt, .. } = &pieces[29] else {
            unreachable!("la dernière pièce est le texte ordinaire")
        };
        assert_eq!(txt, "t");
    }

    // -----------------------------------------------------------------
    // Coniques (v1.6, man. p. 35-37)
    // -----------------------------------------------------------------

    fn page6() -> Page {
        Page::default()
    }

    /// Parabole géométrique : foyer F(0,0), directrice (y = −1),
    /// excentricité 1 → sommet (0 ; −0,5), ouverture vers +y.
    fn parabole_geo_schema() -> Vec<ShObj> {
        let mk = |nom: &str, x: f64, y: f64| {
            ShObj::Pt(PointSchema {
                nom: nom.into(),
                x,
                y,
            })
        };
        vec![mk("F", 0.0, 0.0), mk("P", 0.0, -1.0), mk("Q", 1.0, -1.0)]
    }

    fn con_parabole() -> Conique {
        Conique {
            def: ConDef::FoyerDirectrice {
                foyer: "F".into(),
                directrice: Directrice::DeuxPoints {
                    a: "P".into(),
                    b: "Q".into(),
                },
                e: 1.0,
            },
            couleur: [0, 0, 0],
            epaisseur: 0.4,
            centre: false,
            foyers: true,
            directrices: false,
            axes: false,
            sommets: true,
            asymptotes: false,
        }
    }

    #[test]
    fn conique_parabole_geometrique() {
        let p = page6();
        let schema = parabole_geo_schema();
        let c = con_parabole();
        let parts = c.parties(&p, &schema);
        // une seule branche (r décroît vers le sommet puis ré-augmente)
        assert_eq!(parts.branches.len(), 1);
        // sommet à mi-distance entre foyer et directrice
        assert_eq!(parts.sommets.len(), 1);
        let s = parts.sommets[0];
        assert!((s.0 - 0.0).abs() < 1e-9);
        assert!((s.1 + 0.5).abs() < 1e-9);
        assert_eq!(parts.foyers.len(), 1);
        assert!((parts.foyers[0].0 - 0.0).abs() < 1e-12);
        assert!((parts.foyers[0].1 - 0.0).abs() < 1e-12);
        // axe vertical montant, directrice y = −1
        assert_eq!(parts.axes.len(), 1);
        let (o, v) = parts.axes[0];
        assert!((o.0 - s.0).abs() < 1e-12 && (o.1 - s.1).abs() < 1e-12);
        assert!((v.0 - 0.0).abs() < 1e-12 && (v.1 - 1.0).abs() < 1e-12);
        assert_eq!(parts.directrices.len(), 1);
        let (d0, dv) = parts.directrices[0];
        assert!((d0.1 + 1.0).abs() < 1e-12);
        assert!((dv.1 - 0.0).abs() < 1e-12);
        assert!(parts.asymptotes.is_empty());
        // chaque point de branche vérifie dist(foyer) = dist(directrice)
        let sq = |q: V2| q.0 * q.0 + q.1 * q.1;
        for br in &parts.branches {
            for (i, q) in br.iter().enumerate().take(N_CON).skip(1) {
                let prev = br[i - 1];
                let _ = prev;
                let d_directrice = (q.1 + 1.0).abs();
                assert!((sq(*q).sqrt() - d_directrice).abs() < 1e-6, "{q:?}");
            }
        }
    }

    #[test]
    fn conique_parabole_geometrique_dessin() {
        let p = page6();
        let schema = parabole_geo_schema();
        let c = con_parabole();
        let pieces = c.draw_list(&p, &schema);
        // 1 polygone (branche) + 1 croix focus (2 lignes) + 1 croix
        // sommet (2 lignes)
        assert_eq!(pieces.len(), 5);
        assert!(matches!(pieces[0], PieceS::Poly { .. }));
        // foyer absent → rien du tout
        let mut c2 = c.clone();
        if let ConDef::FoyerDirectrice { foyer, .. } = &mut c2.def {
            *foyer = "Z".into();
        }
        assert!(c2.draw_list(&p, &schema).is_empty());
        // foyer sur la directrice (conique dégénérée P(1;0) Q(2;0)) →
        // rien
        let schema_deg = vec![
            ShObj::Pt(PointSchema {
                nom: "F".into(),
                x: 0.0,
                y: 1.0,
            }),
            ShObj::Pt(PointSchema {
                nom: "P".into(),
                x: 0.0,
                y: 1.0,
            }),
            ShObj::Pt(PointSchema {
                nom: "Q".into(),
                x: 2.0,
                y: 1.0,
            }),
        ];
        assert!(c.draw_list(&p, &schema_deg).is_empty());
        // excentricité nulle ou négative → rien
        let c3 = foy_direct_e(&c, 0.0);
        assert!(c3.draw_list(&p, &schema).is_empty());
        let c4 = foy_direct_e(&c, -0.5);
        assert!(c4.draw_list(&p, &schema).is_empty());
    }

    fn foy_direct_e(c: &Conique, e: f64) -> Conique {
        let mut c2 = c.clone();
        if let ConDef::FoyerDirectrice { e: ex, .. } = &mut c2.def {
            *ex = e;
        }
        c2
    }

    /// Ellipse géométrique : foyer F(0,0), directrice y = −1, e = 0,5.
    #[test]
    fn conique_ellipse_geometrique() {
        let p = page6();
        let schema = parabole_geo_schema();
        let c = foy_direct_e(&con_parabole(), 0.5);
        let parts = c.parties(&p, &schema);
        // branche fermée unique
        assert_eq!(parts.branches.len(), 1);
        // p = e·h = 0,5 ; a = p/(1−e²) = 2/3 ; c = e·a = 1/3 ;
        // b² = a²−c² = 1/3
        let a = 2.0 / 3.0;
        let cc = 1.0 / 3.0;
        let b = (1.0f64 / 3.0).sqrt();
        // centre sur la normale (vers le foyer) à c du foyer
        let Some(o) = parts.centre else {
            unreachable!()
        };
        assert!(o.0.abs() < 1e-9);
        assert!((o.1 - cc).abs() < 1e-9);
        // deux foyers symétriques par rapport au centre
        assert_eq!(parts.foyers.len(), 2);
        let f2 = parts.foyers[1];
        assert!((f2.0 - 0.0).abs() < 1e-9 && (f2.1 - 2.0 * cc).abs() < 1e-9);
        // 4 sommets (grand axe a, petit axe b) ; w = perp(n) = (−1;0)
        assert_eq!(parts.sommets.len(), 4);
        assert!((parts.sommets[0].1 - (cc + a)).abs() < 1e-9);
        assert!((parts.sommets[1].1 - (cc - a)).abs() < 1e-9);
        assert!((parts.sommets[2].0.abs() - b).abs() < 1e-9);
        assert!((parts.sommets[2].1 - cc).abs() < 1e-9);
        assert!((parts.sommets[3].0.abs() - b).abs() < 1e-9);
        assert!((parts.sommets[3].1 - cc).abs() < 1e-9);
        // directrices à a/e = 4/3 du centre
        let dd = a / 0.5;
        assert_eq!(parts.directrices.len(), 2);
        assert!((parts.directrices[0].0 .1 - (cc - dd)).abs() < 1e-9);
        assert!((parts.directrices[1].0 .1 - (cc + dd)).abs() < 1e-9);
        assert!(parts.asymptotes.is_empty());
        // points de la branche : dist(F) = e · dist(directrice)
        let (fc, n, h, _) = directrice_normale((0.0, 0.0), (0.0, -1.0), (1.0, 0.0))
            .map(|(n, h)| ((0.0, 0.0), n, h, 0.5))
            .unwrap();
        let _ = fc;
        for q in &parts.branches[0] {
            let depl_direc = (q.1 + 1.0).abs();
            let dist_foyer = (q.0 * q.0 + q.1 * q.1).sqrt();
            assert!((dist_foyer - 0.5 * depl_direc).abs() < 1e-6, "{q:?}");
        }
        let _ = (n, h);
    }

    /// Hyperbole géométrique : foyer (0;0), directrice y = −1, e = 2.
    #[test]
    fn conique_hyperbole_geometrique() {
        let p = page6();
        let schema = parabole_geo_schema();
        let c = foy_direct_e(&con_parabole(), 2.0);
        let parts = c.parties(&p, &schema);
        // les deux ailes : l'aile proche d'une pièce continue, l'aile
        // éloignée en 2 fragments raccordés au bouclage α=0/2π
        assert_eq!(parts.branches.len(), 3);
        // p = 2 ; a = p/(e²−1) = 2/3 ; c = e·a = 4/3 ; b² = c²−a² = 4/3
        let a = 2.0 / 3.0;
        let cc = 4.0 / 3.0;
        let b = (4.0f64 / 3.0).sqrt();
        let Some(o) = parts.centre else {
            unreachable!()
        };
        assert!((o.1 + cc).abs() < 1e-9);
        assert_eq!(parts.sommets.len(), 2);
        assert!((parts.sommets[0].1 - (-cc + a)).abs() < 1e-9);
        assert!((parts.sommets[1].1 - (-cc - a)).abs() < 1e-9);
        // foyers : (0;0) et son symétrique
        assert_eq!(parts.foyers.len(), 2);
        assert!(parts.foyers[0].0.abs() < 1e-12 && parts.foyers[0].1.abs() < 1e-12);
        assert!((parts.foyers[1].1 + 2.0 * cc).abs() < 1e-9);
        // asymptotes passant par le centre, pentes ±a/b (axe réel
        // porté par la normale)
        assert_eq!(parts.asymptotes.len(), 2);
        for (o2, v) in &parts.asymptotes {
            assert!((o2.0 - o.0).abs() < 1e-12 && (o2.1 - o.1).abs() < 1e-12);
            let pente = (v.1 / v.0).abs();
            assert!((pente - a / b).abs() < 1e-9, "{v:?}");
        }
    }

    /// Parabole cartésienne y² = 4x : a=0 b=0 c=1 d=−4 e=0 f=0.
    #[test]
    fn conique_parabole_cartesienne() {
        let p = page6();
        let c = Conique {
            def: ConDef::Cartesienne {
                a: 0.0,
                b: 0.0,
                c: 1.0,
                d: -4.0,
                e: 0.0,
                f: 0.0,
            },
            ..Default::default()
        };
        let parts = c.parties(&p, &[]);
        // sommet à l'origine, axe vers +x, foyer (1;0), directrice x=−1
        assert_eq!(parts.sommets.len(), 1);
        let s = parts.sommets[0];
        assert!(s.0.abs() < 1e-9 && s.1.abs() < 1e-9);
        assert_eq!(parts.axes.len(), 1);
        let (_, v) = parts.axes[0];
        assert!((v.0 - 1.0).abs() < 1e-9 && v.1.abs() < 1e-9);
        assert_eq!(parts.foyers.len(), 1);
        assert!((parts.foyers[0].0 - 1.0).abs() < 1e-9);
        assert_eq!(parts.directrices.len(), 1);
        let (d0, dv) = parts.directrices[0];
        assert!((d0.0 + 1.0).abs() < 1e-9);
        assert!(dv.0.abs() < 1e-9 && (dv.1 - 1.0).abs() < 1e-9);
        // deux branches (y = ±2√x pour x ≥ 0), points exacts
        assert_eq!(parts.branches.len(), 2);
        for br in &parts.branches {
            for q in br {
                assert!((q.1 * q.1 - 4.0 * q.0).abs() < 1e-6, "{q:?}");
            }
        }
    }

    /// Ellipse cartésienne x²/4 + y²/9 = 1 : 9x² + 4y² − 36 = 0.
    #[test]
    fn conique_ellipse_cartesienne() {
        let p = page6();
        let c = Conique {
            def: ConDef::Cartesienne {
                a: 9.0,
                b: 0.0,
                c: 4.0,
                d: 0.0,
                e: 0.0,
                f: -36.0,
            },
            ..Default::default()
        };
        let parts = c.parties(&p, &[]);
        let Some(o) = parts.centre else {
            unreachable!()
        };
        assert!(o.0.abs() < 1e-9 && o.1.abs() < 1e-9);
        // grand axe vertical (λ2 sur y), a = 3 ; petit axe b = 2
        assert_eq!(parts.sommets.len(), 4);
        let rac5 = 5.0f64.sqrt();
        assert_eq!(parts.foyers.len(), 2);
        assert!((parts.foyers[0].1 - rac5).abs() < 1e-9 || (parts.foyers[0].1 + rac5).abs() < 1e-9);
        assert!((parts.foyers[0].0).abs() < 1e-9);
        assert!((parts.foyers[1].1 + parts.foyers[0].1).abs() < 1e-9);
        // directrices y = ± a²/c = ± 9/√5
        let dd = 9.0 / rac5;
        assert_eq!(parts.directrices.len(), 2);
        assert!((parts.directrices[0].0 .1.abs() - dd).abs() < 1e-9);
        assert!(parts.asymptotes.is_empty());
        // points de branche : sur l'ellipse x²/4 + y²/9 = 1
        for br in &parts.branches {
            for q in br {
                let s = q.0 * q.0 / 4.0 + q.1 * q.1 / 9.0;
                assert!((s - 1.0).abs().sqrt() < 1e-6, "{q:?}");
            }
        }
    }

    /// Hyperbole cartésienne x²/4 − y²/9 = 1 : 9x² − 4y² − 36 = 0.
    #[test]
    fn conique_hyperbole_cartesienne() {
        let p = page6();
        let c = Conique {
            def: ConDef::Cartesienne {
                a: 9.0,
                b: 0.0,
                c: -4.0,
                d: 0.0,
                e: 0.0,
                f: -36.0,
            },
            ..Default::default()
        };
        let parts = c.parties(&p, &[]);
        let Some(o) = parts.centre else {
            unreachable!()
        };
        assert!(o.0.abs() < 1e-9 && o.1.abs() < 1e-9);
        // sommets (±2 ; 0), foyers (±5 ; 0)
        assert_eq!(parts.sommets.len(), 2);
        assert!((parts.sommets[0].0 - 2.0).abs() < 1e-9);
        assert!((parts.sommets[1].0 + 2.0).abs() < 1e-9);
        assert_eq!(parts.foyers.len(), 2);
        let rac13 = 13.0f64.sqrt();
        assert!((parts.foyers[0].0 - rac13).abs() < 1e-9);
        assert!((parts.foyers[1].0 + rac13).abs() < 1e-9);
        // directrices x = ± a²/c = ± 4/√13
        let dd = 4.0 / rac13;
        assert_eq!(parts.directrices.len(), 2);
        assert!((parts.directrices[0].0 .0.abs() - dd).abs() < 1e-9);
        // asymptotes de pentes ±b/a = ±3/2 : y = ±(3/2)x
        assert_eq!(parts.asymptotes.len(), 2);
        for (_, v) in &parts.asymptotes {
            assert!((v.1.abs() / v.0.abs() - 1.5).abs() < 1e-9, "{v:?}");
        }
        // branches : points sur l'hyperbole (4 segments : 2 ailes ×
        // partie supérieure/inférieure)
        let mut npt = 0;
        for br in &parts.branches {
            for q in br {
                let s = q.0 * q.0 / 4.0 - q.1 * q.1 / 9.0;
                assert!((s - 1.0).abs() < 1e-6, "{q:?}");
                npt += 1;
            }
        }
        assert!(npt > 100);
    }

    /// Équation dégénérée x² − 1 = 0 : deux droites verticales.
    #[test]
    fn conique_cartesienne_degeneree_verticale() {
        let p = page6();
        let c = Conique {
            def: ConDef::Cartesienne {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: -1.0,
            },
            ..Default::default()
        };
        let parts = c.parties(&p, &[]);
        assert_eq!(parts.branches.len(), 2);
        for br in &parts.branches {
            assert_eq!(br.len(), 2);
            let x = br[0].0;
            assert!((x * x - 1.0).abs() < 1e-12);
            assert!((br[1].0 - x).abs() < 1e-12);
        }
        assert!(parts.sommets.is_empty());
        // équation vide → rien
        let c2 = Conique {
            def: ConDef::Cartesienne {
                a: 0.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
            },
            ..Default::default()
        };
        assert!(c2.parties(&p, &[]).branches.is_empty());
        // droite horizontale y = 0 (e=1) : branche le long de l'axe
        let c3 = Conique {
            def: ConDef::Cartesienne {
                a: 0.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                e: 1.0,
                f: 0.0,
            },
            ..Default::default()
        };
        let parts3 = c3.parties(&p, &[]);
        assert_eq!(parts3.branches.len(), 1);
        for q in &parts3.branches[0] {
            assert!(q.1.abs() < 1e-9, "{q:?}");
        }
    }

    /// Conique à centre avec terme croisé (b ≠ 0) : 2x² + 2xy + 2y² − 1
    /// = 0 : valeurs propres 3 et 1, premier axe selon (1;1)/√2.
    #[test]
    fn conique_ellipse_terme_croise() {
        let p = page6();
        let c = Conique {
            def: ConDef::Cartesienne {
                a: 2.0,
                b: 2.0,
                c: 2.0,
                d: 0.0,
                e: 0.0,
                f: -1.0,
            },
            ..Default::default()
        };
        let parts = c.parties(&p, &[]);
        // centre à l'origine ; axes tournés de 45°
        let Some(o) = parts.centre else {
            unreachable!()
        };
        assert!(o.0.abs() < 1e-9 && o.1.abs() < 1e-9);
        // λ1 = 3 (direction (1;1)/√2), λ2 = 1 (perp = (−1;1)/√2) ;
        // demi-axes sqrt(1/3) et 1
        let a1 = (1.0f64 / 3.0).sqrt();
        assert_eq!(parts.sommets.len(), 4);
        for q in &parts.sommets {
            let sur_v1 = (q.1 - q.0).abs() < 1e-9; // direction (1;1)
            let sur_v2 = (q.1 + q.0).abs() < 1e-9; // direction (−1;1)
            assert!(sur_v1 || sur_v2, "{q:?}");
            let norm = q.0.hypot(q.1);
            if sur_v1 {
                assert!((norm - a1).abs() < 1e-9, "{q:?}");
            } else {
                assert!((norm - 1.0).abs() < 1e-9, "{q:?}");
            }
        }
        // points de branches : 2·x²+2xy+2y² = 1
        for br in &parts.branches {
            for q in br {
                let s = 2.0 * q.0 * q.0 + 2.0 * q.0 * q.1 + 2.0 * q.1 * q.1;
                assert!((s - 1.0).abs() < 1e-6, "{q:?}");
            }
        }
    }

    /// Tous les éléments demandés (axes, centre, asymptotes) font
    /// apparaître des pièces supplémentaires dans draw_list, dans
    /// l'ordre voulu (man. p. 40 : la conique est un objet ordinaire).
    #[test]
    fn conique_options_tracage_draw_schema() {
        let p = page6();
        let schema = parabole_geo_schema();
        // hyperbole géométrique avec toutes les options
        let c = Conique {
            foyers: true,
            sommets: true,
            axes: true,
            centre: true,
            directrices: true,
            asymptotes: true,
            ..foy_direct_e(&con_parabole(), 2.0)
        };
        let pieces = c.draw_list(&p, &schema);
        // 3 Poly (branches : en polaire, l'aile éloignée est découpée par
        // le bouclage α=0/2π, 2 fragments) + 2 foyers (2 lignes chacun)
        // + 2 sommets (2 lignes) + centre (2 lignes) + 2 axes (1 trait
        // chacun) + 2 directrices + 2 asymptotes = 3+4+4+2+2+2+2 = 19
        assert_eq!(pieces.len(), 19, "{}", pieces.len());
        // aucune option → juste les branches
        let c2 = Conique {
            foyers: false,
            sommets: false,
            ..foy_direct_e(&con_parabole(), 2.0)
        };
        let pieces2 = c2.draw_list(&p, &schema);
        assert_eq!(pieces2.len(), 3);
        assert!(pieces2.iter().all(|q| matches!(q, PieceS::Poly { .. })));
        // intégrée au schéma : le dessin passe bien par draw_schema
        let mut schema2 = parabole_geo_schema();
        schema2.push(ShObj::Con(c2));
        let all = ShObj::draw_schema(&p, &schema2);
        assert!(all.len() >= 2);
        assert!(all.iter().any(|q| matches!(q, PieceS::Poly { .. })));
    }

    /// Crochet de graduation du rapports : chant au sens du genre
    /// (lycée trigonométrique CCW depuis l'est ; collège horaire).
    #[test]
    fn rapporteur_sens_de_graduation() {
        let p = Page::default();
        // lycée : la graduation 180° est à l'ouest (x = 85 − 50)
        let rap = Rapporteur {
            c: [85.0, 60.0],
            rayon: 50.0,
            radians: false, // pour n'avoir que les crans aux degrés
            ..Rapporteur::default()
        };
        let v = ShObj::Rap(rap.clone()).draw_list(&p, &[]);
        // cran long de 180° : segment horizontal au bord gauche
        let ok180 = v.iter().any(|q| {
            matches!(q, PieceS::Ligne { x0, y0, x1, y1, .. }
            if (*x1 - 38.5).abs() < 1e-6 && (*y1 - 60.0).abs() < 1e-6 && (*x0 - 35.0).abs() < 1e-6)
        });
        assert!(ok180, "cran 180° à l'ouest (lycée) absent");
        // collège entier : 180° à l'est
        let rap = Rapporteur {
            genre: GenreRap::College,
            rayon: 50.0,
            c: [85.0, 60.0],
            ..Rapporteur::default()
        };
        let v = ShObj::Rap(rap).draw_list(&p, &[]);
        let ok180 = v.iter().any(|q| matches!(q, PieceS::Ligne { x0, y0, x1, y1, .. }
            if (*x1 - 131.5).abs() < 1e-6 && (*y1 - 60.0).abs() < 1e-6 && (*x0 - 135.0).abs() < 1e-6));
        assert!(ok180, "cran 180° à l'est (collège horaire) absent");
        // ... et 270° au sud (horaire : north 90, east 180, south 270) :
        // cran vertical sous le centre, y de 110 (bord) à 106.5
        let ok270 = v.iter().any(|q| {
            matches!(q, PieceS::Ligne { x0, y0, x1, y1, .. }
            if (*x0 - 85.0).abs() < 1e-6 && (*y0 - 110.0).abs() < 1e-6
                && (*y1 - 106.5).abs() < 1e-6)
        });
        assert!(ok270, "cran 270° au sud (collège) absent");
    }

    #[test]
    fn rapporteur_demi_cercle_college() {
        let p = Page::default();
        let rap = Rapporteur {
            genre: GenreRap::College,
            rayon: 50.0,
            demi: true,
            radians: false,
            ..Rapporteur::default()
        };
        let v = ShObj::Rap(rap).draw_list(&p, &[]);
        // corde au sud... non : le dôme au-dessus, corde horizontale
        // à plat : segment (35,60)-(135,60)
        let corde = v.iter().any(|q| {
            matches!(q, PieceS::Ligne { x0, y0, x1, y1, .. }
            if (*x0 - 35.0).abs() < 1e-6 && (*x1 - 135.0).abs() < 1e-6)
        });
        assert!(corde, "corde du demi-cercle absente");
        // amplitude 180 : aucun cran au-delà de 180° (au sud)
        for q in &v {
            if let PieceS::Ligne { y0, y1, .. } = q {
                assert!(*y0 < 61.0 || *y1 < 61.0, "cran hors demi-cercle {q:?}");
            }
        }
        // arc au-dessus : l'arc extérieur part de l'ouest (35, 60)
        let arcs: Vec<&Vec<(f64, f64)>> = v
            .iter()
            .filter_map(|q| match q {
                PieceS::Poly {
                    pts, fill: None, ..
                } => Some(pts),
                _ => None,
            })
            .collect();
        let ok = arcs
            .iter()
            .any(|pts| pts[0].0 < 40.0 && pts.iter().all(|q| q.1 <= 60.0 + 1e-9));
        assert!(ok, "arc du dôme au-dessus de la corde");
    }

    #[test]
    fn rapporteur_rayon_borne_et_cercle_trigo() {
        let p = Page::default();
        // rayon hors bornes 40..100 : borné au dessin (man. p. 88)
        let rap = Rapporteur {
            rayon: 200.0,
            genre: GenreRap::Lycee,
            radians: false,
            ..Rapporteur::default()
        };
        let v = ShObj::Rap(rap).draw_list(&p, &[]);
        // cran 90 (vertical au-dessus du centre) ne dépasse pas cy−100
        for q in &v {
            if let PieceS::Ligne { y0, .. } = q {
                assert!(*y0 >= 60.0 - 100.0 - 1e-9, "hors page bas {q:?}");
            }
        }
        // cercle trigo simple : aucune étiquette de graduations
        let rap = Rapporteur {
            cercle_trigo: true,
            radians: true,
            ..Rapporteur::default()
        };
        let v = ShObj::Rap(rap).draw_list(&p, &[]);
        assert!(
            !v.iter().any(|q| matches!(q, PieceS::Texte { .. })),
            "cercle trigo simple : pas d'étiquettes"
        );
    }
}
