//! Simulations statistiques (v1.0, manuel p. 65–74) : tirages aléatoires
//! suivant 7 modèles probabilistes, organisés en échantillons, avec
//! graphiques points / histogrammes / boîtes et courbe théorique.
//!
//! Pas de dépendance : le générateur aléatoire est un xorshift64*
//! (Marsaglia), semé par l'horloge à chaque lancement dans l'interface.
//! La graine est enregistrée dans la session : l'export en ligne de
//! commande rejoue donc exactement les tirages affichés.
//!
//! Comme l'original (case « échelle automatique »), le lancement
//! reconfigure le repère de la page pour que les axes épousent les
//! données ; le dessin reste dans le repère de la page.

use crate::grid::Page;
use crate::stats::{
    pieces_axes, titre_piece, BoiteMultiple, BoitesMultiples, Classe, Donnees, Params, Piece,
    Serie1, ValEff,
};
use serde::{Deserialize, Serialize};

/// Modèle probabiliste d'une simulation (manuel p. 65–66 : 7 choix).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Modele {
    /// entiers de `min` à `max` équiprobables (dé à « max − min + 1 » faces)
    UniformeDiscrete { min: i64, max: i64 },
    /// valeur réelle uniformément répartie sur [`min`, `max`]
    UniformeContinue { min: f64, max: f64 },
    /// épreuve succès/échec : 1 avec la probabilité `p`, 0 sinon
    Bernoulli { p: f64 },
    /// nombre de succès de `n` épreuves de Bernoulli indépendantes
    Binomiale { n: u32, p: f64 },
    /// nombre d'épreuves avant le premier succès (à partir de 1)
    Geometrique { p: f64 },
    /// loi de Poisson de paramètre `lambda` (> 0)
    Poisson { lambda: f64 },
    /// loi normale : moyenne `mu`, écart type `sigma`
    Normale { mu: f64, sigma: f64 },
}

/// Graphique d'une simulation (manuel p. 65 : « points, histogrammes,
/// diagrammes en boîtes, pour chacun des échantillons ou pour
/// l'ensemble des tirages »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Graphique {
    /// 1 point par échantillon : la moyenne de chaque échantillon
    #[default]
    Points,
    /// un histogramme par échantillon, empilé en bandes verticales
    Histogrammes,
    /// un histogramme unique de l'ensemble des tirages
    HistogrammeEnsemble,
    /// boîtes à moustaches : une par échantillon
    Boites,
}

/// Une simulation statistique : modèle, taille et nombre d'échantillons,
/// réglages du graphique. Les tirages eux-mêmes ne sont pas enregistrés
/// dans la session : la graine suffit à les rejouer à l'identique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Simulation {
    pub titre: String,
    pub modele: Modele,
    /// taille de chaque échantillon (man : 50)
    #[serde(default = "def_taille")]
    pub taille: usize,
    /// nombre d'échantillons
    #[serde(default = "def_taille")]
    pub nombre: usize,
    /// nombre de classes de l'histogramme de l'ensemble (man : 20)
    #[serde(default = "def_classes")]
    pub classes: usize,
    pub graphique: Graphique,
    pub dessiner: bool,
    /// ajouter la courbe / valeur théorique au graphique
    #[serde(default)]
    pub courbe_theorique: bool,
    /// Bernoulli uniquement : afficher l'intervalle de fluctuation de la
    /// moyenne au taux de confiance 95 % (manuel p. 71)
    #[serde(default)]
    pub aff_fluctuation: bool,
    /// graine du générateur xorshift64* ; 0 = non semée (dessin sur la
    /// graine par défaut), sinon les tirages sont rejoués à l'identique
    #[serde(default)]
    pub graine: u64,
}

fn def_taille() -> usize {
    50
}
fn def_classes() -> usize {
    20
}

/// Graine par défaut (distincte de 0, valeur quelconque).
const GRAINE_NEUTRE: u64 = 881_726_454_633_252;

impl Default for Simulation {
    fn default() -> Self {
        Simulation {
            titre: String::new(),
            modele: Modele::UniformeDiscrete { min: 1, max: 6 },
            taille: 50,
            nombre: 5,
            classes: 20,
            graphique: Graphique::Histogrammes,
            dessiner: true,
            courbe_theorique: false,
            aff_fluctuation: false,
            graine: 0,
        }
    }
}

/// Générateur xorshift64* : état 64 bits, renvoie un entier 64 bits.
fn xorshift64(st: &mut u64) -> u64 {
    let mut x = *st;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *st = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

/// Réel uniforme dans [0, 1[ à partir de l'état du générateur.
fn alea(st: &mut u64) -> f64 {
    (xorshift64(st) >> 11) as f64 / (1u64 << 53) as f64
}

/// Graine effective : 0 veut dire « jamais semée ».
fn graine_eff(s: u64) -> u64 {
    if s == 0 {
        GRAINE_NEUTRE
    } else {
        s
    }
}

fn log_factorielle(n: f64) -> f64 {
    // Stirling sur f64 ; précision suffisante pour les pmf
    if n < 2.0 {
        return 0.0;
    }
    0.5 * ((n + 0.5) * (2.0 * std::f64::consts::PI).ln()) + (n + 0.5) * n.ln() - n
}

fn log_combinaison(n: u32, k: u32) -> f64 {
    log_factorielle(n as f64) - log_factorielle(k as f64) - log_factorielle((n - k) as f64)
}

const BLEU: [u8; 3] = [0, 0, 204];
const ROUGE: [u8; 3] = [204, 0, 0];
const LIME: [u8; 3] = [96, 217, 51];

/// Cap de sécurité de la loi géométrique (au-delà, pmf négligeable).
const GEO_CAP: u32 = 100_000;

impl Modele {
    /// Un tirage du modèle (met à jour l'état du générateur). La valeur
    /// renvoyée est un entier pour les lois discrètes.
    pub fn tirage(&self, st: &mut u64) -> f64 {
        match *self {
            Modele::UniformeDiscrete { min, max } => {
                let palier = (max - min + 1).max(1) as u64;
                let r = ((xorshift64(st) >> 33) % palier) as i64;
                (min + r) as f64
            }
            Modele::UniformeContinue { min, max } => min + alea(st) * (max - min),
            Modele::Bernoulli { p } => {
                if alea(st) < p {
                    1.0
                } else {
                    0.0
                }
            }
            Modele::Binomiale { n, p } => {
                let mut k = 0u32;
                for _ in 0..n {
                    if alea(st) < p {
                        k += 1;
                    }
                }
                k as f64
            }
            Modele::Geometrique { p } => {
                // nombre d'épreuves avant le premier succès ; au-delà du
                // cap, renvoyer le cap (cas p quasi nul)
                let mut t = 1u32;
                while t < GEO_CAP && alea(st) >= p {
                    t += 1;
                }
                t as f64
            }
            Modele::Poisson { lambda } => {
                // algorithme de Knuth
                let l = (-lambda).exp();
                let (mut k, mut u) = (0u32, 1.0f64);
                loop {
                    u *= alea(st);
                    if u <= l {
                        break;
                    }
                    k += 1;
                }
                k as f64
            }
            Modele::Normale { mu, sigma } => {
                // Box-Muller : deux uniformes → une gaussienne
                let u1 = alea(st).max(1e-300);
                let u2 = alea(st);
                let r = (-2.0 * u1.ln()).sqrt();
                let th = 2.0 * std::f64::consts::PI * u2;
                mu + sigma * r * th.cos()
            }
        }
    }

    /// Moyenne théorique du modèle (courbe théorique du graphique
    /// « points »).
    pub fn moyenne_theorique(&self) -> f64 {
        match *self {
            Modele::UniformeDiscrete { min, max } => (min as f64 + max as f64) / 2.0,
            Modele::UniformeContinue { min, max } => (min + max) / 2.0,
            Modele::Bernoulli { p } => p,
            Modele::Binomiale { n, p } => n as f64 * p,
            Modele::Geometrique { p } => 1.0 / p,
            Modele::Poisson { lambda } => lambda,
            Modele::Normale { mu, .. } => mu,
        }
    }

    /// Densité (lois continues) ou probabilité ponctuelle k (lois
    /// discrètes) — pour la courbe théorique des histogrammes.
    pub fn densite(&self, x: f64) -> f64 {
        match *self {
            Modele::UniformeDiscrete { min, max } => {
                let k = x.round() as i64;
                if k >= min && k <= max {
                    1.0 / (max - min + 1) as f64
                } else {
                    0.0
                }
            }
            Modele::UniformeContinue { min, max } => {
                if (min..=max).contains(&x) && max > min {
                    1.0 / (max - min)
                } else {
                    0.0
                }
            }
            Modele::Bernoulli { p } => match x.round() as i64 {
                0 => 1.0 - p,
                1 => p,
                _ => 0.0,
            },
            Modele::Binomiale { n, p } => {
                let k = x.round() as i64;
                if k < 0 || k > n as i64 {
                    return 0.0;
                }
                let k = k as u32;
                (log_combinaison(n, k) + k as f64 * p.ln() + (n - k) as f64 * (1.0 - p).ln()).exp()
            }
            Modele::Geometrique { p } => {
                let k = x.round() as i64;
                if k < 1 {
                    return 0.0;
                }
                (1.0 - p).powf((k - 1) as f64) * p
            }
            Modele::Poisson { lambda } => {
                let k = x.round() as i64;
                if k < 0 {
                    return 0.0;
                }
                (-lambda + k as f64 * lambda.ln() - log_factorielle(k as f64)).exp()
            }
            Modele::Normale { mu, sigma } => {
                if sigma <= 0.0 {
                    return 0.0;
                }
                let z = (x - mu) / sigma;
                (-0.5 * z * z).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
            }
        }
    }
}

impl Modele {
    /// Le modèle est « à valeurs entières » : l'original dessine alors un
    /// histogramme à une barre par valeur possible (dé : 6 barres de
    /// 1 à 6, Bernoulli : 2 barres 0 / 1), la répartition en classes ne
    /// s'appliquant qu'aux lois continues (man. p. 67-69).
    pub fn discret(&self) -> bool {
        !matches!(
            *self,
            Modele::UniformeContinue { .. } | Modele::Normale { .. }
        )
    }
}

impl Simulation {
    /// Tous les échantillons : `nombre` vecteurs de `taille` tirages.
    /// Déterministe pour une graine donnée (session rejouée à l'identique).
    pub fn tirages(&self) -> Vec<Vec<f64>> {
        let mut st = graine_eff(self.graine);
        let nmin = self.taille.min(1_000_000);
        (0..self.nombre.min(10_000))
            .map(|_| (0..nmin).map(|_| self.modele.tirage(&mut st)).collect())
            .collect()
    }

    /// Ensemble de tous les tirages (grille générale de l'original).
    pub fn ensemble(&self, tirages: &[Vec<f64>]) -> Vec<f64> {
        tirages.iter().flatten().copied().collect()
    }

    /// Série statistique à une variable construite sur un échantillon
    /// (réutilise le module v0.4 pour les calculs).
    pub fn serie_de(&self, tirage: &[f64]) -> Serie1 {
        Serie1 {
            titre: String::new(),
            donnees: Donnees::Isolees(
                tirage
                    .iter()
                    .filter(|v| v.is_finite())
                    .map(|&v| ValEff { v, n: 1.0 })
                    .collect(),
            ),
            ..Default::default()
        }
    }

    /// Paramètres calculés sur l'ensemble des tirages (grille générale).
    pub fn params_ensemble(&self, tirages: &[Vec<f64>]) -> Option<Params> {
        let s = self.serie_de(&self.ensemble(tirages));
        s.parametres()
    }

    /// Moyennes des échantillons (graphique « points »).
    pub fn moyennes(&self, tirages: &[Vec<f64>]) -> Vec<f64> {
        tirages
            .iter()
            .map(|t| {
                let s = self.serie_de(t);
                s.parametres().map(|pr| pr.moyenne).unwrap_or(f64::NAN)
            })
            .collect()
    }

    /// Boîtes à moustaches : une par échantillon (module v0.4).
    pub fn boites_multiples(&self, tirages: &[Vec<f64>]) -> BoitesMultiples {
        let boites = tirages
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let pr = self.serie_de(t).parametres();
                let d = pr.unwrap_or(Params {
                    n: 0.0,
                    somme: 0.0,
                    moyenne: 0.0,
                    sigma: 0.0,
                    minimum: 0.0,
                    maximum: 0.0,
                    d1: 0.0,
                    q1: 0.0,
                    mediane: 0.0,
                    q3: 0.0,
                    d9: 0.0,
                });
                BoiteMultiple {
                    legende: format!("Serie {}", i + 1),
                    min: d.minimum,
                    d1: Some(d.d1),
                    q1: d.q1,
                    med: Some(d.mediane),
                    q3: d.q3,
                    d9: Some(d.d9),
                    max: d.maximum,
                    couleur: BLEU,
                }
            })
            .collect();
        BoitesMultiples {
            titre: String::new(),
            boites,
            dessiner: true,
        }
    }

    /// Classes d'égale largeur sur l'étendue de l'ensemble des tirages.
    pub fn classes_ensemble(&self, tirages: &[Vec<f64>]) -> Vec<Classe> {
        let vmin = self
            .ensemble(tirages)
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .fold(f64::INFINITY, f64::min);
        let vmax = self
            .ensemble(tirages)
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .fold(f64::NEG_INFINITY, f64::max);
        if !vmin.is_finite() || !vmax.is_finite() {
            return Vec::new();
        }
        // modèle discret : une classe par valeur entière (barre [k−0,5 ; k+0,5])
        if self.modele.discret() {
            let k0 = vmin.floor() as i64;
            let k1 = vmax.ceil() as i64;
            return (k0..=k1)
                .map(|k| Classe {
                    a: k as f64 - 0.5,
                    b: k as f64 + 0.5,
                    n: 0.0,
                })
                .collect();
        }
        if vmax <= vmin {
            return Vec::new();
        }
        let nb = self.classes.clamp(1, 400);
        let l = (vmax - vmin) / nb as f64;
        (0..nb)
            .map(|i| {
                let a = vmin + i as f64 * l;
                Classe {
                    a,
                    b: a + l,
                    n: 0.0,
                }
            })
            .collect()
    }

    /// Histogramme de l'ensemble des tirages (module v0.4 réutilisé).
    pub fn serie_ensemble(&self, tirages: &[Vec<f64>]) -> Option<Serie1> {
        let mut classes = self.classes_ensemble(tirages);
        if classes.is_empty() {
            return None;
        }
        let vmin = classes[0].a;
        let l = classes[0].b - classes[0].a;
        let nb = classes.len();
        for v in self.ensemble(tirages) {
            if !v.is_finite() {
                continue;
            }
            // classe [a, b) ; dernière classe fermée
            let mut i = ((v - vmin) / l) as usize;
            if i >= nb {
                i = nb - 1;
            }
            classes[i].n += 1.0;
        }
        Some(Serie1 {
            titre: String::new(),
            donnees: Donnees::Classes(classes),
            style: crate::stats::Style::Histogramme,
            ..Default::default()
        })
    }

    /// Plages (x, y) utiles du graphique, pour l'échelle automatique du
    /// repère (renvoyées en coordonnées utilisateur ; appelées par la
    /// GUI au lancement, comme l'original).
    pub fn plages(&self, tirages: &[Vec<f64>]) -> ((f64, f64), (f64, f64)) {
        let ens = self.ensemble(tirages);
        match self.graphique {
            Graphique::Points => {
                let moy = self.moyennes(tirages);
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for &m in &moy {
                    if m.is_finite() {
                        lo = lo.min(m);
                        hi = hi.max(m);
                    }
                }
                if !lo.is_finite() {
                    return ((0.0, 10.0), (0.0, 10.0));
                }
                let (lo, hi) = (lo - 0.5, hi + 0.5);
                ((0.5, self.nombre.max(1) as f64 + 0.5), (lo, hi))
            }
            Graphique::Histogrammes => {
                // modèle discret : les barres couvrent [k−0,5 ; k+0,5]
                if self.modele.discret() {
                    let cl = self.classes_ensemble(tirages);
                    if let (Some(f), Some(l)) = (cl.first(), cl.last()) {
                        return ((f.a - 0.5, l.b + 0.5), (0.0, self.nombre.max(1) as f64));
                    }
                }
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for &v in &ens {
                    if v.is_finite() {
                        lo = lo.min(v);
                        hi = hi.max(v);
                    }
                }
                if !lo.is_finite() || lo >= hi {
                    return ((0.0, 10.0), (0.0, 10.0));
                }
                let marge = (hi - lo) * 0.05;
                ((lo - marge, hi + marge), (0.0, self.nombre.max(1) as f64))
            }
            Graphique::HistogrammeEnsemble => {
                let Some(s) = self.serie_ensemble(tirages) else {
                    return ((0.0, 10.0), (0.0, 10.0));
                };
                let Donnees::Classes(cl) = &s.donnees else {
                    return ((0.0, 10.0), (0.0, 10.0));
                };
                let (x0, x1) = (
                    cl.first().map_or(0.0, |c| c.a),
                    cl.last().map_or(10.0, |c| c.b),
                );
                let ymax = cl.iter().map(|c| c.n).fold(1.0f64, f64::max);
                let marge = (x1 - x0) * 0.05;
                ((x0 - marge, x1 + marge), (0.0, ymax * 1.1 + 1.0))
            }
            Graphique::Boites => {
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for &v in &ens {
                    if v.is_finite() {
                        lo = lo.min(v);
                        hi = hi.max(v);
                    }
                }
                let marge = (hi - lo).max(1.0) * 0.08;
                ((lo - marge, hi + marge), (f64::NEG_INFINITY, f64::INFINITY))
            }
        }
    }

    /// Dessin (repère de la page, déterministe via la graine).
    pub fn draw_list(&self, p: &Page) -> Vec<Piece> {
        if !self.dessiner {
            return Vec::new();
        }
        let tirages = self.tirages();
        match self.graphique {
            Graphique::Points => self.dessine_points(p, &tirages),
            Graphique::Histogrammes => self.dessine_histos(p, &tirages),
            Graphique::HistogrammeEnsemble => self.dessine_ensemble(p, &tirages),
            Graphique::Boites => self.dessine_boites(p, &tirages),
        }
    }

    /// Graphique « points » : une moyenne par échantillon ; valeur
    /// théorique en trait plein, intervalle de fluctuation (Bernoulli)
    /// en pointillé (man. p. 69-71).
    fn dessine_points(&self, p: &Page, tirages: &[Vec<f64>]) -> Vec<Piece> {
        let moy = self.moyennes(tirages);
        let mut out = pieces_axes(p, true, true, false);
        if let Some(t) = titre_piece(p, &self.titre) {
            out.push(t);
        }
        for (i, m) in moy.iter().enumerate() {
            if !m.is_finite() {
                continue;
            }
            out.push(Piece::Disque {
                c: (p.mm_x(i as f64 + 1.0), p.mm_y(*m)),
                r: 0.9,
                fill: ROUGE,
                stroke: None,
            });
        }
        if self.courbe_theorique {
            let mu = self.modele.moyenne_theorique();
            let (ux0, ux1) = p.x_range();
            let (x0, x1) = (p.mm_x(ux0), p.mm_x(ux1));
            out.push(Piece::Ligne {
                x0,
                y0: p.mm_y(mu),
                x1,
                y1: p.mm_y(mu),
                c: BLEU,
                w: 0.7,
            });
        }
        // intervalle de fluctuation de la moyenne à 95 % (Bernoulli)
        if self.aff_fluctuation {
            if let Modele::Bernoulli { p: pb } = self.modele {
                let tail = self.taille.max(1) as f64;
                let e = 1.96 * (pb * (1.0 - pb) / tail).sqrt();
                let (ux0, ux1) = p.x_range();
                let (x0, x1) = (p.mm_x(ux0), p.mm_x(ux1));
                for yy in [pb - e, pb + e] {
                    out.push(Piece::Pointille {
                        x0,
                        y0: p.mm_y(yy),
                        x1,
                        y1: p.mm_y(yy),
                        c: BLEU,
                        w: 0.5,
                    });
                }
            }
        }
        out
    }

    /// Graphique « histogrammes » : une bande par échantillon, barres
    /// mises à l'échelle locale (hauteur 1 = effectif de la classe
    /// dominante de l'échantillon).
    fn dessine_histos(&self, p: &Page, tirages: &[Vec<f64>]) -> Vec<Piece> {
        let mut out = pieces_axes(p, true, true, false);
        if let Some(t) = titre_piece(p, &self.titre) {
            out.push(t);
        }
        for (j, tir) in tirages.iter().enumerate() {
            let y0 = j as f64; // bande [j, j+1)
                               // classes locales de la bande
            let local = Simulation {
                titre: String::new(),
                taille: self.taille,
                nombre: 1,
                classes: self.classes.min(60),
                ..self.clone()
            };
            let Some(s) = local.serie_ensemble(std::slice::from_ref(tir)) else {
                continue;
            };
            let Donnees::Classes(cl) = &s.donnees else {
                continue;
            };
            let ymax = cl.iter().map(|c| c.n).fold(1.0f64, f64::max);
            let ech = 0.9 / ymax;
            for c in cl {
                if c.n <= 0.0 {
                    continue;
                }
                let h = c.n * ech;
                out.push(Piece::Rect {
                    mm: (p.mm_x(c.a), p.mm_y(y0 + h)),
                    w: p.mm_x(c.b) - p.mm_x(c.a),
                    h: p.mm_y(y0) - p.mm_y(y0 + h),
                    fill: Some(LIME),
                    stroke: (BLEU, 0.3),
                });
            }
            // courbe théorique : effectif théorique = taille × densité
            // × largeur de la classe (man. p. 68 : 50/6 ≈ 8,33 par face)
            if self.courbe_theorique {
                for c in cl {
                    let ct = (c.a + c.b) / 2.0;
                    let y = y0 + self.taille as f64 * self.modele.densite(ct) * (c.b - c.a) * ech;
                    if y <= y0 || !y.is_finite() {
                        continue;
                    }
                    out.push(Piece::Ligne {
                        x0: p.mm_x(c.a),
                        y0: p.mm_y(y),
                        x1: p.mm_x(c.b),
                        y1: p.mm_y(y),
                        c: BLEU,
                        w: 0.7,
                    });
                }
            }
        }
        out
    }

    /// Graphique « histogramme de l'ensemble » : module v0.4 réutilisé
    /// + densité théorique (effectif théorique = n × densité × largeur).
    fn dessine_ensemble(&self, p: &Page, tirages: &[Vec<f64>]) -> Vec<Piece> {
        let Some(s) = self.serie_ensemble(tirages) else {
            return Vec::new();
        };
        let mut out = s.draw_list(p);
        if self.courbe_theorique {
            let Donnees::Classes(cl) = &s.donnees else {
                return out;
            };
            if cl.is_empty() {
                return out;
            }
            let total: f64 = cl.iter().map(|c| c.n).sum();
            let l = cl[0].b - cl[0].a;
            let x0 = cl[0].a;
            let x1 = cl.last().map_or(x0, |c| c.b);
            if l <= 0.0 || total <= 0.0 || x1 <= x0 {
                return out;
            }
            let n = 240;
            let mut pts = Vec::new();
            for i in 0..=n {
                let x = x0 + (x1 - x0) * i as f64 / n as f64;
                let y = total * self.modele.densite(x) * l;
                if y.is_finite() {
                    pts.push((p.mm_x(x), p.mm_y(y.max(0.0))));
                }
            }
            if pts.len() >= 2 {
                out.push(Piece::Poly {
                    pts,
                    fill: None,
                    stroke: Some((BLEU, 0.7)),
                });
            }
        }
        out
    }

    /// Graphique « boîtes » : module v0.4 (boîtes multiples) réutilisé.
    fn dessine_boites(&self, p: &Page, tirages: &[Vec<f64>]) -> Vec<Piece> {
        let bm = self.boites_multiples(tirages);
        let mut out = bm.draw_list(p);
        if out.is_empty() {
            return Vec::new();
        }
        if let Some(t) = titre_piece(p, &self.titre) {
            out.push(t);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Piece as PieceS;

    fn sim(modele: Modele) -> Simulation {
        Simulation {
            titre: "Sim".into(),
            modele,
            ..Default::default()
        }
    }

    #[test]
    fn graine_rejoue_les_tirages() {
        // même graine → mêmes tirages ; graine différente → différence
        let s = sim(Modele::UniformeContinue {
            min: 0.0,
            max: 10.0,
        });
        let a = s.clone().tirages();
        let b = s.tirages();
        assert_eq!(a, b);
        let s = Simulation {
            graine: 42,
            ..sim(Modele::UniformeContinue {
                min: 0.0,
                max: 10.0,
            })
        };
        let c = s.tirages();
        assert_ne!(a[0], c[0]);
        // taille et nombre respectés
        assert_eq!(a.len(), 5);
        assert_eq!(a[0].len(), 50);
    }

    #[test]
    fn uniforme_discrete_de() {
        // dé équilibré : moyenne proche de 3,5 ; bornes respectées
        let s = sim(Modele::UniformeDiscrete { min: 1, max: 6 });
        let t = s.tirages();
        let ens: Vec<f64> = t.iter().flatten().copied().collect();
        assert!(ens.iter().all(|v| (1.0..=6.0).contains(v)));
        assert!(ens.iter().all(|v| v.fract() == 0.0));
        let m: f64 = ens.iter().sum::<f64>() / ens.len() as f64;
        assert!((m - 3.5).abs() < 0.3, "moyenne du dé : {m}");
    }

    #[test]
    fn uniforme_continue_bornes() {
        let s = sim(Modele::UniformeContinue { min: 1.0, max: 6.0 });
        let t = s.tirages();
        let ens: Vec<f64> = t.iter().flatten().copied().collect();
        assert!(ens.iter().all(|v| (1.0..=6.0).contains(v)));
        let m = ens.iter().sum::<f64>() / ens.len() as f64;
        assert!((m - 3.5).abs() < 0.25, "{m}");
    }

    #[test]
    fn bernoulli_proportion() {
        let s = Simulation {
            taille: 2000,
            ..sim(Modele::Bernoulli { p: 0.6 })
        };
        let t = s.tirages();
        let ens: Vec<f64> = t.iter().flatten().copied().collect();
        assert!(ens.iter().all(|v| *v == 0.0 || *v == 1.0));
        let q = ens.iter().sum::<f64>() / ens.len() as f64;
        assert!((q - 0.6).abs() < 0.05, "{q}");
    }

    #[test]
    fn binomiale_moyenne() {
        let s = Simulation {
            taille: 500,
            classes: 11,
            graphique: Graphique::HistogrammeEnsemble,
            ..sim(Modele::Binomiale { n: 10, p: 0.5 })
        };
        let t = s.tirages();
        let ens: Vec<f64> = t.iter().flatten().copied().collect();
        assert!(ens.iter().all(|v| (0.0..=10.0).contains(v)));
        let m = ens.iter().sum::<f64>() / ens.len() as f64;
        assert!((m - 5.0).abs() < 0.2, "{m}");
    }

    #[test]
    fn geometrique_premier_succes() {
        let s = Simulation {
            taille: 1000,
            ..sim(Modele::Geometrique { p: 0.25 })
        };
        let t = s.tirages();
        let ens: Vec<f64> = t.iter().flatten().copied().collect();
        assert!(ens.iter().all(|v| *v >= 1.0));
        assert!(ens.iter().all(|v| v.fract() == 0.0));
        let m = ens.iter().sum::<f64>() / ens.len() as f64;
        assert!((m - 4.0).abs() < 0.8, "{m}");
    }

    #[test]
    fn poisson_parametre_lambda() {
        let s = Simulation {
            taille: 2000,
            graphique: Graphique::HistogrammeEnsemble,
            ..sim(Modele::Poisson { lambda: 6.0 })
        };
        let t = s.tirages();
        let ens: Vec<f64> = t.iter().flatten().copied().collect();
        assert!(ens.iter().all(|v| *v >= 0.0));
        let m = ens.iter().sum::<f64>() / ens.len() as f64;
        assert!((m - 6.0).abs() < 0.3, "{m}");
    }

    #[test]
    fn normale_cloche() {
        let s = Simulation {
            taille: 2000,
            graphique: Graphique::HistogrammeEnsemble,
            ..sim(Modele::Normale {
                mu: 175.0,
                sigma: 10.0,
            })
        };
        let t = s.tirages();
        let ens: Vec<f64> = t.iter().flatten().copied().collect();
        let m = ens.iter().sum::<f64>() / ens.len() as f64;
        assert!((m - 175.0).abs() < 2.5, "{m}");
        assert!(ens.iter().all(|v| (125.0..=225.0).contains(v)));
        // densité de la normale centrée réduite en 0 : 0,3989…
        assert!(
            (Modele::Normale {
                mu: 0.0,
                sigma: 1.0
            }
            .densite(0.0)
                - 0.3989422804014327)
                .abs()
                < 1e-10
        );
    }

    #[test]
    fn ensemble_et_classes() {
        let s = sim(Modele::UniformeDiscrete { min: 1, max: 6 });
        let t = s.tirages();
        let ens = s.ensemble(&t);
        assert_eq!(ens.len(), 250);
        let sc = s.serie_ensemble(&t).unwrap();
        let total = sc.n_total();
        assert!((total - 250.0).abs() < 1e-9, "{total}");
        // toutes les valeurs tombent dans une classe
        if let Donnees::Classes(cl) = &sc.donnees {
            assert!(!cl.is_empty());
            assert!((cl.iter().map(|c| c.n).sum::<f64>() - 250.0).abs() < 1e-9);
        }
        // boîtes : une par échantillon, paramètres ordonnés
        let bm = s.boites_multiples(&t);
        assert_eq!(bm.boites.len(), 5);
        for b in &bm.boites {
            assert!(b.min <= b.d1.unwrap_or(b.min));
            assert!(b.q1 <= b.med.unwrap_or(b.q1));
            assert!(b.med.unwrap_or(b.q1) <= b.q3);
            assert!(b.q3 <= b.d9.unwrap_or(b.q3));
            assert!(b.d9.unwrap_or(b.max) <= b.max);
        }
    }

    #[test]
    fn moyennes_et_plages() {
        let s = sim(Modele::UniformeDiscrete { min: 1, max: 6 });
        let t = s.tirages();
        // graphique points : plage x de 0,5 à nombre+0,5
        let ((x0, x1), _) = {
            let s = Simulation {
                graphique: Graphique::Points,
                ..s.clone()
            };
            s.plages(&t)
        };
        assert_eq!(x0, 0.5);
        assert!(x1 - 5.5 < 1e-12);
        // moyennes : 5 valeurs finies entre 1 et 6
        let moy = s.moyennes(&t);
        assert_eq!(moy.len(), 5);
        assert!(moy.iter().all(|m| m.is_finite() && (1.0..=6.0).contains(m)));
    }

    #[test]
    fn draw_list_graphiques() {
        let base = sim(Modele::UniformeDiscrete { min: 1, max: 6 });
        // points : un disque par échantillon + axes
        let s = Simulation {
            graphique: Graphique::Points,
            ..base.clone()
        };
        let pieces = s.draw_list(&Page::default());
        assert_eq!(
            pieces
                .iter()
                .filter(|p| matches!(p, PieceS::Disque { .. }))
                .count(),
            5
        );
        // courbe théorique : trait horizontal
        let s = Simulation {
            graphique: Graphique::Points,
            courbe_theorique: true,
            ..base.clone()
        };
        assert!(s
            .draw_list(&Page::default())
            .iter()
            .any(|p| matches!(p, PieceS::Ligne { .. })));
        // histogrammes : barres
        let s = Simulation {
            graphique: Graphique::Histogrammes,
            ..base.clone()
        };
        assert!(s
            .draw_list(&Page::default())
            .iter()
            .any(|p| matches!(p, PieceS::Rect { .. })));
        // boîtes : boîte par échantillon
        let s = Simulation {
            graphique: Graphique::Boites,
            ..base.clone()
        };
        assert!(!s.draw_list(&Page::default()).is_empty());
        // ensemble : classes + somme = effectif
        let s = Simulation {
            graphique: Graphique::HistogrammeEnsemble,
            ..base.clone()
        };
        assert!(s
            .draw_list(&Page::default())
            .iter()
            .any(|p| matches!(p, PieceS::Rect { .. })));
        // masqué : rien
        let s = Simulation {
            dessiner: false,
            ..base
        };
        assert!(s.draw_list(&Page::default()).is_empty());
    }

    #[test]
    fn histogramme_par_valeur_pour_les_discretes() {
        // man. p. 67-68 : dé → 6 barres ; Bernoulli → 2 barres (0 / 1) ;
        // la répartition en classes ne s'applique qu'aux lois continues
        let de = sim(Modele::UniformeDiscrete { min: 1, max: 6 });
        let t = de.tirages();
        let cl = de.classes_ensemble(&t);
        assert_eq!(cl.len(), 6);
        assert!((cl[0].a - 0.5).abs() < 1e-9);
        assert!((cl[5].b - 6.5).abs() < 1e-9);
        let sc = de.serie_ensemble(&t).unwrap();
        let Donnees::Classes(clf) = &sc.donnees else {
            unreachable!()
        };
        assert_eq!(clf.len(), 6);
        assert_eq!(clf.iter().map(|c| c.n).sum::<f64>().round() as usize, 250);
        let piece = Simulation {
            modele: Modele::Bernoulli { p: 0.6 },
            ..sim(Modele::Bernoulli { p: 0.6 })
        };
        let cl = piece.classes_ensemble(&piece.tirages());
        assert_eq!(cl.len(), 2);
        // loi continue : classes réparties sur l'étendue
        let u = sim(Modele::UniformeContinue { min: 1.0, max: 6.0 });
        assert_eq!(u.classes_ensemble(&u.tirages()).len(), 20);
    }

    #[test]
    fn profil_theorique_histos() {
        // man. p. 68 : la théorique traverse l'histogramme à 50/6 ≈ 8,33
        let base = sim(Modele::UniformeDiscrete { min: 1, max: 6 });
        let s = Simulation {
            graphique: Graphique::Histogrammes,
            courbe_theorique: true,
            ..base.clone()
        };
        let p = Page::default();
        let lignes = s.draw_list(&p).len();
        let s = Simulation {
            graphique: Graphique::Histogrammes,
            ..base
        };
        assert!(lignes > s.draw_list(&p).len(), "profil théorique absent");
    }

    #[test]
    fn json_defauts_v09_compat() {
        // session antérieure (sans simulation) → champs à défaut
        let txt = r#"{"titre":"S","modele":{"UniformeDiscrete":{"min":1,"max":6}},"graphique":"Points","dessiner":true}"#;
        let s: Simulation = serde_json::from_str(txt).unwrap();
        assert_eq!(s.taille, 50);
        assert_eq!(s.classes, 20);
        assert_eq!(s.graine, 0);
        assert!(!s.courbe_theorique);
    }

    #[test]
    fn fluctuation_bernoulli() {
        // intervalle de fluctuation au seuil 0,95 : p ± 1,96√(p(1−p)/n)
        let s = Simulation {
            modele: Modele::Bernoulli { p: 0.38 },
            taille: 2000,
            aff_fluctuation: true,
            graphique: Graphique::Points,
            ..Default::default()
        };
        let e = 1.96 * (0.38_f64 * 0.62 / 2000.0).sqrt();
        assert!(e > 0.01 && e < 0.03, "{e}");
        // le graphique contient deux lignes pointillées
        let pieces = s.draw_list(&Page::default());
        assert_eq!(
            pieces
                .iter()
                .filter(|p| matches!(p, PieceS::Pointille { .. }))
                .count(),
            2
        );
    }
}
