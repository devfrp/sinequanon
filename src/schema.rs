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
        }
    }
}

impl TexteSchema {
    pub fn draw_list(&self, p: &Page) -> Vec<Piece> {
        if self.texte.is_empty() {
            return Vec::new();
        }
        vec![Piece::Texte {
            mm: (p.mm_x(self.x), p.mm_y(self.y)),
            txt: self.texte.clone(),
            c: self.couleur,
            taille: self.taille,
            ancre: "",
            vertical: self.vertical,
            gras: self.gras,
            italique: self.italique,
        }]
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
            if !matches!(o, ShObj::Zon(_) | ShObj::Txt(_)) {
                out.extend(o.draw_list(p, schema));
            }
        }
        for o in schema {
            if let ShObj::Txt(_) = o {
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
}
