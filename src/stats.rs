//! Statistiques à une variable — parité avec la boîte « Statistiques à
//! une variable » (4 volets) et « Boîtes à moustaches multiples » du
//! Sine qua non original 2.9.4 (mode d'emploi p. 43-51). Les formules de
//! quantile reproduisent celles de l'original : valeurs comparées aux
//! exemples « Nombre d'enfants » et « Répartition des salaires » dont le
//! panneau Calculs affiche les valeurs exactes.

use crate::grid::{Page, PAS_MM};
use serde::{Deserialize, Serialize};

/// Côté d'un carreau de la grille, en mm.
pub const CARREAU: f64 = 5.0;

/// Cycle de couleurs pastel des modalités (celui visible dans la grille
/// de saisie de l'original : cyan, rose, jaune, lavande, vert, saumon,
/// cyan pâle — en RGB).
pub const COULEURS_MODALITES: [[u8; 3]; 7] = [
    [112, 216, 240],
    [240, 175, 200],
    [220, 235, 81],
    [175, 175, 225],
    [130, 216, 130],
    [235, 145, 130],
    [188, 232, 240],
];

/// Couleur par défaut d'une série : le « Couleur : 3396608 » BGR de
/// l'original = lime (RGB 96-217-51) ; c'est la couleur des barres des
/// exemples « Répartition des salaires » du manuel.
pub const LIME: [u8; 3] = [96, 217, 51];

/// Loi de probabilité (module statistiques v0.5) : normale, binomiale,
/// poisson ou exponentielle. Chaque loi se trace sur son axe (densité
/// ou fonction de distribution) et se calcule.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Loi {
    Normal {
        /// moyenne μ
        mu: f64,
        /// écart-type σ (> 0)
        sigma: f64,
    },
    Binomial {
        /// taille de l'échantillon n
        n: f64,
        /// probabilité p (0 < p < 1)
        p: f64,
    },
    Poisson {
        /// paramètre λ (= moyenne)
        mu: f64,
    },
    Exponentielle {
        /// paramètre λ (> 0)
        lambda: f64,
    },
}

impl Loi {
    /// Nom court de la loi (pour les légendes et le panneau Calculs).
    pub fn nom(&self) -> &'static str {
        match self {
            Loi::Normal { .. } => "Loi normale",
            Loi::Binomial { .. } => "Loi binomiale",
            Loi::Poisson { .. } => "Loi de Poisson",
            Loi::Exponentielle { .. } => "Loi exponentielle",
        }
    }

    /// Description complète pour les légendes (ex. « Loi normale : μ = 100,5 ; σ = 12 »).
    pub fn description(&self) -> String {
        let (mu, sigma) = match self {
            Loi::Normal { mu, sigma } => (*mu, *sigma),
            Loi::Binomial { n, p } => (*n * *p, 1.0 - *p),
            Loi::Poisson { mu } => (*mu, *mu),
            Loi::Exponentielle { lambda } => (*lambda, *lambda),
        };
        format!("{} : μ = {}, σ = {}", self.nom(), num(mu), num(sigma))
    }

    /// Plage d'intérêt (abscisses, densité normalisée au pic) pour le tracé.
    fn plage(&self) -> (f64, f64, f64) {
        match self {
            Loi::Normal { mu, sigma } => {
                if *sigma <= 0.0 {
                    return (0.0, 1.0, 1.0);
                }
                let s = *sigma;
                (
                    *mu - 3.0 * s,
                    *mu + 3.0 * s,
                    1.0 / (s * (2.0 * std::f64::consts::PI).sqrt()),
                )
            }
            Loi::Binomial { n, p } => {
                if *n < 0.0 || !(0.0..=1.0).contains(p) {
                    return (0.0, 1.0, 1.0);
                }
                let n_i = *n as i64;
                let p = *p;
                if n_i <= 0 {
                    return (0.0, 1.0, 0.0);
                }
                // mode de la loi binomiale
                let k = ((n_i + 1) as f64 * p) as i64;
                let k = k.clamp(0, n_i);
                let pdf = |kk: i64| -> f64 {
                    let mut comb: f64 = 1.0;
                    for i in 1..=kk {
                        comb *= (n_i - kk + i) as f64 / i as f64;
                    }
                    comb * p.powi(kk as i32) * (1.0 - p).powi((n_i - kk) as i32)
                };
                let ymax = pdf(k);
                (0.0, n_i as f64, ymax)
            }
            Loi::Poisson { mu } => {
                let m = mu.max(0.0);
                let hi = (m * 5.0).max(10.0);
                let mode = m.round() as i64;
                let pdf = |kk: f64| -> f64 {
                    let k = kk.floor().max(0.0) as i64;
                    let mut fact = 1.0f64;
                    for i in 2..=k.max(1) {
                        fact *= i as f64;
                    }
                    m.powi(k as i32) * (-m).exp() / fact
                };
                (0.0, hi, pdf(mode as f64))
            }
            Loi::Exponentielle { lambda } => {
                if *lambda <= 0.0 {
                    return (0.0, 1.0, 1.0);
                }
                (0.0, 10.0 / lambda, *lambda)
            }
        }
    }

    /// Densité de probabilité (pdf).
    pub fn pdf(&self, x: f64) -> f64 {
        match self {
            Loi::Normal { mu, sigma } => {
                let z = (x - mu) / sigma;
                (-z * z / 2.0).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
            }
            Loi::Binomial { n, p } => {
                if *n < 0.0 || !(0.0..=1.0).contains(p) {
                    return 0.0;
                }
                let n_i = n.floor() as i64;
                let k_i = (x.floor().max(0.0)).min(n_i.max(0) as f64) as i64;
                let k_i = k_i.min(n_i);
                if k_i < 0 {
                    return 0.0;
                }
                // C(n, k) = Π_{i=1..k} (n−k+i) / i
                let mut comb: f64 = 1.0;
                for i in 1..=k_i {
                    comb *= (n_i - k_i + i) as f64 / i as f64;
                }
                comb * p.powi(k_i as i32) * (1.0 - p).powi((n_i - k_i) as i32)
            }
            Loi::Poisson { mu } => {
                let m = mu.max(0.0);
                let k = x.floor().max(0.0) as i64;
                let mut fact = 1.0f64;
                for i in 2..=k.max(1) {
                    fact *= i as f64;
                }
                m.powi(k as i32) * (-m).exp() / fact
            }
            Loi::Exponentielle { lambda } => {
                if *lambda > 0.0 {
                    lambda * (-lambda * x).exp()
                } else {
                    0.0
                }
            }
        }
    }

    /// Fonction de distribution (cdf). Loi de probabilité (module
    /// statistiques v0.5) : disponible pour les calculs de probabilité
    /// (P(X ≤ x)) avant l'arrivée du module complet.
    #[allow(dead_code)]
    pub fn cdf(&self, x: f64) -> f64 {
        match self {
            Loi::Normal { mu, sigma } => {
                if *sigma <= 0.0 {
                    return 0.5;
                }
                phi((x - mu) / sigma)
            }
            Loi::Binomial { n, p } => {
                if *n < 0.0 || !(0.0..=1.0).contains(p) {
                    return 0.0;
                }
                let n_i = n.max(0.0) as i64;
                let mut cum = 0.0;
                for k in 0..=n_i {
                    cum += self.pdf(k as f64);
                    if (k as f64) >= x {
                        return cum;
                    }
                }
                1.0
            }
            Loi::Poisson { mu } => {
                let m = mu.max(0.0);
                let mut cum = 0.0;
                let mut term = (-m).exp();
                let mut k = 0.0;
                loop {
                    cum += term;
                    if k >= x {
                        return cum;
                    }
                    term *= m / (k + 1.0);
                    k += 1.0;
                    if k > 1e7 {
                        return 1.0;
                    }
                }
            }
            Loi::Exponentielle { lambda } => {
                if *lambda <= 0.0 {
                    return 1.0;
                }
                if x < 0.0 {
                    return 0.0;
                }
                1.0 - (-lambda * x).exp()
            }
        }
    }

    /// Probabilité « point » : P(X = k) (lois discrètes) ; pour la loi
    /// exponentielle / normale on renvoie la densité (évaluation
    /// locale).
    pub fn probabilite(&self, k: f64) -> f64 {
        self.pdf(k)
    }
}

/// Style du graphique, correspondant aux choix de la boîte de saisie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    /// non numérique : camembert
    Camembert,
    /// non numérique ET classes : histogramme
    Histogramme,
    /// isolées : diagramme en bâtons
    Batons,
    /// isolées + classes : diagramme en boîte
    Boite,
    /// isolées + classes : boîte avec déciles
    BoiteAvecDeciles,
    /// classes : fréquences cumulées croissantes/décroissantes
    FreqCumul,
    /// classes : droite de Henry
    DroiteHenry,
    /// loi de probabilité (module statistiques v0.5) : densité tracée
    /// sur l'axe des valeurs
    LoiProba,
}

/// Une modalité du volet « variable non numérique ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modalite {
    pub nom: String,
    pub n: f64,
    pub couleur: [u8; 3],
}

/// Une ligne du volet « valeurs isolées ».
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ValEff {
    pub v: f64,
    pub n: f64,
}

/// Une classe du volet « valeurs regroupées en classes » : n = effectif
/// des valeurs x avec a ⩽ x < b.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Classe {
    pub a: f64,
    pub b: f64,
    pub n: f64,
}

/// Données de la série (le volet choisi dans la boîte de saisie).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Donnees {
    Quali(Vec<Modalite>),
    Isolees(Vec<ValEff>),
    Classes(Vec<Classe>),
}

impl Donnees {
    /// Vrai si les données correspondent au volet donné (0 quali,
    /// 1 isolées, 2 classes).
    pub fn est_volet(&self, volet: usize) -> bool {
        match self {
            Donnees::Quali(_) => volet == 0,
            Donnees::Isolees(_) => volet == 1,
            Donnees::Classes(_) => volet == 2,
        }
    }
}

/// Coche « Visualiser les paramètres » (Médiane seulement / Médiane et
/// quartiles / Tous les paramètres).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visualiser {
    MedianeSeulement,
    MedianeEtQuartiles,
    Tous,
}

/// Une série statistique à une variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Serie1 {
    pub titre: String,
    pub donnees: Donnees,
    pub style: Style,
    pub dessiner: bool,
    pub couleur: [u8; 3],
    /// classes : dessin avec les pourcentages (sinon les effectifs)
    #[serde(default = "oui")]
    pub avec_pct: bool,
    /// classes : le carreau représente `canaux` unités (1.0 % par défaut)
    #[serde(default = "un")]
    pub canaux: f64,
    /// histogramme : afficher les effectifs au-dessus des barres
    #[serde(default = "oui")]
    pub aff_effectifs: bool,
    /// cumulées : croissantes / décroissantes
    #[serde(default = "oui")]
    pub croissantes: bool,
    #[serde(default)]
    pub decroissantes: bool,
    #[serde(default = "def_visualiser")]
    pub visualiser: Visualiser,
    /// loi de probabilité à tracer (module statistiques v0.5) ; si
    /// `None`, la loi normale est ajustée sur les données.
    #[serde(default)]
    pub loi: Option<Loi>,
}

fn oui() -> bool {
    true
}
fn un() -> f64 {
    1.0
}
fn def_visualiser() -> Visualiser {
    Visualiser::MedianeEtQuartiles
}

impl Default for Serie1 {
    fn default() -> Self {
        Serie1 {
            titre: String::new(),
            donnees: Donnees::Isolees(Vec::new()),
            style: Style::Batons,
            dessiner: true,
            couleur: LIME,
            avec_pct: true,
            canaux: 1.0,
            aff_effectifs: true,
            croissantes: true,
            decroissantes: false,
            visualiser: Visualiser::MedianeEtQuartiles,
            loi: None,
        }
    }
}

/// Paramètres calculés (panneau « Calculs » de la boîte de saisie).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub n: f64,
    pub somme: f64,
    pub moyenne: f64,
    pub sigma: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub d1: f64,
    pub q1: f64,
    pub mediane: f64,
    pub q3: f64,
    pub d9: f64,
}

/// Quantile d'une série à valeurs isolées : première valeur dont
/// l'effectif cumulé atteint k·n — celle de l'original : enfants →
/// D1 0, Q1 1, Méd 2, Q3 3, D9 4.
fn quantile_isolees(rows: &[ValEff], k: f64, n: f64) -> f64 {
    let mut zones: Vec<ValEff> = rows.to_vec();
    zones.sort_by(|a, b| a.v.partial_cmp(&b.v).unwrap());
    let mut cum = 0.0;
    for r in &zones {
        cum += r.n;
        if cum >= k * n - 1e-9 {
            return r.v;
        }
    }
    zones.last().map(|r| r.v).unwrap_or(0.0)
}

/// Quantile d'une série en classes : interpolation linéaire dans la
/// classe qui contient k·n — salaires → Q1 917,857 ; Méd 1003,85 ;
/// Q3 1058,33 ; D1 850 ; D9 1192,86.
fn quantile_classes(rows: &[Classe], k: f64, n: f64) -> f64 {
    let cible = k * n;
    let mut cum = 0.0;
    for c in rows {
        if cum + c.n >= cible - 1e-9 {
            if c.n <= 0.0 {
                return c.a;
            }
            return c.a + (cible - cum) / c.n * (c.b - c.a);
        }
        cum += c.n;
    }
    rows.last().map(|c| c.b).unwrap_or(0.0)
}

impl Serie1 {
    /// Paramètres de position et de dispersion. `None` pour un caractère
    /// qualitatif (l'original n'y autorise aucun calcul).
    pub fn parametres(&self) -> Option<Params> {
        match &self.donnees {
            Donnees::Quali(_) => None,
            Donnees::Isolees(vals) => {
                let mut rows: Vec<ValEff> = vals.clone();
                if rows.iter().any(|r| !r.v.is_finite()) {
                    return None;
                }
                let n: f64 = rows.iter().map(|r| r.n).sum();
                if n <= 0.0 {
                    return None;
                }
                let somme: f64 = rows.iter().map(|r| r.v * r.n).sum();
                let somme2: f64 = rows.iter().map(|r| r.v * r.v * r.n).sum();
                let moyenne = somme / n;
                let var = somme2 / n - moyenne * moyenne;
                rows.sort_by(|a, b| a.v.partial_cmp(&b.v).unwrap());
                let q = |k: f64| quantile_isolees(&rows, k, n);
                Some(Params {
                    n,
                    somme,
                    moyenne,
                    sigma: var.max(0.0).sqrt(),
                    minimum: rows.first().map(|r| r.v).unwrap_or(0.0),
                    maximum: rows.last().map(|r| r.v).unwrap_or(0.0),
                    d1: q(0.1),
                    q1: q(0.25),
                    mediane: q(0.5),
                    q3: q(0.75),
                    d9: q(0.9),
                })
            }
            Donnees::Classes(c) => {
                let n: f64 = c.iter().map(|cl| cl.n).sum();
                if n <= 0.0 {
                    return None;
                }
                // chaque classe ramenée à son point moyen
                let moyenne: f64 = c.iter().map(|cl| (cl.a + cl.b) / 2.0 * cl.n).sum::<f64>() / n;
                let m2: f64 = c
                    .iter()
                    .map(|cl| ((cl.a + cl.b) / 2.0).powi(2) * cl.n)
                    .sum();
                let var = m2 / n - moyenne * moyenne;
                let q = |k: f64| quantile_classes(c, k, n);
                Some(Params {
                    n,
                    somme: moyenne * n,
                    moyenne,
                    sigma: var.max(0.0).sqrt(),
                    minimum: c.first().map(|cl| cl.a).unwrap_or(0.0),
                    maximum: c.last().map(|cl| cl.b).unwrap_or(0.0),
                    d1: q(0.1),
                    q1: q(0.25),
                    mediane: q(0.5),
                    q3: q(0.75),
                    d9: q(0.9),
                })
            }
        }
    }

    /// Effectif total (somme des effectifs).
    pub fn n_total(&self) -> f64 {
        match &self.donnees {
            Donnees::Quali(m) => m.iter().map(|x| x.n).sum(),
            Donnees::Isolees(v) => v.iter().map(|x| x.n).sum(),
            Donnees::Classes(c) => c.iter().map(|x| x.n).sum(),
        }
    }

    /// Cumuls pour le tracé des fréquences cumulées : (borne supérieure,
    /// effectif cumulé croissant).
    fn cumuls(&self) -> Vec<(f64, f64)> {
        match &self.donnees {
            Donnees::Classes(c) => {
                let mut out = Vec::new();
                let mut cum = 0.0;
                for cl in c {
                    cum += cl.n;
                    out.push((cl.b, cum));
                }
                out
            }
            _ => Vec::new(),
        }
    }
}

// ---------------------------------------------------------------- dessin

/// Pièce de dessin en coordonnées page (mm), Y vers le bas.
#[derive(Debug, Clone)]
pub enum Piece {
    /// rectangle plein + contour (barres, boîte)
    Rect {
        mm: (f64, f64),
        w: f64,
        h: f64,
        fill: Option<[u8; 3]>,
        stroke: ([u8; 3], f64),
    },
    /// polygone plein (part de camembert, aire de cumulées)
    Poly {
        pts: Vec<(f64, f64)>,
        fill: Option<[u8; 3]>,
        stroke: Option<([u8; 3], f64)>,
    },
    /// trait plein
    Ligne {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        c: [u8; 3],
        w: f64,
    },
    /// trait pointillé (équipotentielles, prolongements de moustaches)
    Pointille {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        c: [u8; 3],
        w: f64,
    },
    /// petit disque (valeurs individuelles au-delà des déciles ; points
    /// de Henry)
    Disque {
        c: (f64, f64),
        r: f64,
        fill: [u8; 3],
        stroke: Option<([u8; 3], f64)>,
    },
    Texte {
        mm: (f64, f64),
        txt: String,
        c: [u8; 3],
        /// hauteur de casse en mm
        taille: f64,
        ancre: &'static str,
        /// texte bas-vers-haut (noms de modalités sous l'axe)
        vertical: bool,
        gras: bool,
        /// italique (x et y des axes, légendes des boîtes multiples)
        italique: bool,
    },
    /// fond blanc opaque sous la zone dessinée (camembert)
    Fond { mm: (f64, f64), w: f64, h: f64 },
}

const NOIR: [u8; 3] = [0, 0, 0];
const ROUGE: [u8; 3] = [204, 0, 0];
const BLEU: [u8; 3] = [0, 0, 204];
/// Nombres des axes : gris pâle, plus petits (captures du manuel).
const GRIS_AXE: [u8; 3] = [135, 135, 140];
/// Taille des nombres d'axes.
const NUM_TXT: f64 = 2.3;
/// Baselines des deux rangées sous l'axe (nombres, paramètres) et
/// profondeur des chutes pointillées.
const TX_NOMBRES: f64 = 4.0;
const TX_PARAM: f64 = 8.65;
const VERT_DECILE: [u8; 3] = [0, 128, 0];
const CYAN_BOITE: [u8; 3] = [104, 216, 232];
const ROSE_STICK: [u8; 3] = [214, 108, 214];

/// Épsilon décimal pour comparer les cumuls.
const EPS: f64 = 1e-9;

/// Nombre formaté à la française (⩽ 3 décimales, virgule).
pub fn num(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_string()
    } else {
        s.replace('.', ",")
    }
}

/// « 25,80 % » : 2 décimales, virgule (format des barres de l'original).
fn num2(v: f64) -> String {
    format!("{v:.2} %").replace('.', ",")
}

/// « 0,05 » de la grille Henry.
fn num_pct(p: f64) -> String {
    let s = format!("{:.3}", p);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}

/// Nombre d'axe : 4 chiffres significatifs au plus, zéros tronqués,
/// virgule décimale (« 700 », « 0,5 », « 1050 ») comme l'original.
fn num_axe(v: f64) -> String {
    if !v.is_finite() {
        return String::new();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    let a = v.abs();
    let d = ((3.0 - a.log10().floor()).round() as i32).clamp(0, 6);
    let s = format!("{:.*}", d as usize, v);
    // les zéros en trop ne se rognent que s'il y a une partie décimale :
    // « 1000 » est un entier, « 0,500 » devient « 0,5 »
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    s.replace('.', ",")
}

const AX_EPAIS: f64 = 0.4;
const AX_TXT: f64 = 2.7;
const PARAM_TXT: f64 = 3.0;

/// Axes du repère avec flèches, lettres « x »/« y » italiques et nombres
/// aux multiples de l'unité de graduation (grad_x/grad_y de la page),
/// comme les graphiques statistiques de l'original. Les axes croisent à
/// l'origine des axes de la page (origin_x/origin_y) ; en sens unique
/// (`x_cote_positif`, graphes statistiques) les nombres d'abscisses ne
/// sont imprimés qu'au-delà du croisement, comme z-008/017. Les axes
/// (et leurs nombres) ne sont dessinés que si le croisement est sur la
/// page ; les nombres qui se chevauchent sont sautés.
pub fn pieces_axes(p: &Page, avec_y: bool, avec_x: bool, x_cote_positif: bool) -> Vec<Piece> {
    let mut out = Vec::new();
    let ax_y = p.mm_y(p.origin_y);
    let ay_x = p.mm_x(p.origin_x);
    let horizontale = ax_y.is_finite() && (0.5..=p.height_mm - 0.5).contains(&ax_y);
    let verticale = ay_x.is_finite() && (0.5..=p.width_mm - 0.5).contains(&ay_x);

    // axe des abscisses : trait + flèche + lettre x
    if avec_x && horizontale {
        out.push(Piece::Ligne {
            x0: 0.0,
            y0: ax_y,
            x1: p.width_mm,
            y1: ax_y,
            c: NOIR,
            w: AX_EPAIS,
        });
        out.push(Piece::Poly {
            pts: vec![
                (p.width_mm, ax_y),
                (p.width_mm - 2.2, ax_y - 0.65),
                (p.width_mm - 2.2, ax_y + 0.65),
            ],
            fill: Some(NOIR),
            stroke: None,
        });
        out.push(Piece::Texte {
            mm: (p.width_mm - 0.6, ax_y + TX_NOMBRES),
            txt: "x".into(),
            c: NOIR,
            taille: AX_TXT,
            ancre: "end",
            vertical: false,
            gras: false,
            italique: true,
        });
        // nombres : multiples de l'unité de graduation, sautés si trop
        // serrés (règle de collision comme l'original)
        let g = p.grad_x;
        if g > 0.0 {
            let (lo, hi) = p.x_range();
            let mut i0 = (lo / g).ceil() as i64;
            let i1 = (hi / g).floor() as i64;
            if x_cote_positif {
                // nombres d'abscisses imprimés seulement au-delà du
                // croisement (convention des graphes de l'original)
                i0 = i0.max((p.origin_x / g).ceil() as i64);
            }
            if i1 - i0 <= 2000 && i1 >= i0 {
                let mut prev_r: Option<f64> = None;
                for i in i0..=i1 {
                    let v = i as f64 * g;
                    let x = p.mm_x(v);
                    if !(1.0..=p.width_mm - 1.0).contains(&x) {
                        continue;
                    }
                    let s = num_axe(v);
                    if s.is_empty() {
                        continue;
                    }
                    let w = s.chars().count() as f64 * 0.62 * NUM_TXT;
                    if let Some(r) = prev_r {
                        if x - w / 2.0 < r + 0.7 {
                            continue;
                        }
                    }
                    // l'étiquette qui coïncide avec le croisement se
                    // colle à gauche de l'axe (ecr-001 : « 700 »)
                    let croise = x_cote_positif && (x - p.mm_x(p.origin_x)).abs() < 0.5;
                    out.push(Piece::Texte {
                        mm: (if croise { x - 1.0 } else { x }, ax_y + TX_NOMBRES),
                        txt: s,
                        c: GRIS_AXE,
                        taille: NUM_TXT,
                        ancre: if croise { "end" } else { "middle" },
                        vertical: false,
                        gras: false,
                        italique: false,
                    });
                    prev_r = Some(if croise { x - 1.0 } else { x + w / 2.0 });
                }
            }
        }
    }
    // axe des ordonnées : trait + flèche + lettre y + nombres à gauche
    if avec_y && verticale && horizontale {
        out.push(Piece::Ligne {
            x0: ay_x,
            y0: 0.0,
            x1: ay_x,
            y1: p.height_mm,
            c: NOIR,
            w: AX_EPAIS,
        });
        out.push(Piece::Poly {
            pts: vec![(ay_x, 0.0), (ay_x - 0.65, 2.2), (ay_x + 0.65, 2.2)],
            fill: Some(NOIR),
            stroke: None,
        });
        out.push(Piece::Texte {
            mm: (ay_x - 2.4, 4.4),
            txt: "y".into(),
            c: NOIR,
            taille: AX_TXT,
            ancre: "end",
            vertical: false,
            gras: false,
            italique: true,
        });
        let g = p.grad_y;
        if g > 0.0 {
            let (lo, hi) = p.y_range();
            let i0 = (lo / g).ceil() as i64;
            let i1 = (hi / g).floor() as i64;
            if i1 - i0 <= 2000 {
                let mut prev_y: Option<f64> = None;
                for i in i0..=i1 {
                    let v = i as f64 * g;
                    let y = p.mm_y(v);
                    if !(1.0..=p.height_mm - 1.0).contains(&y) {
                        continue;
                    }
                    let s = num_axe(v);
                    if s.is_empty() {
                        continue;
                    }
                    // largeur disponible à gauche de l'axe
                    let w = s.chars().count() as f64 * 0.62 * NUM_TXT;
                    if ay_x - 1.0 - w < 0.3 {
                        continue;
                    }
                    if let Some(py) = prev_y {
                        if (y - py).abs() < 3.4 {
                            continue;
                        }
                    }
                    out.push(Piece::Texte {
                        mm: (ay_x - 1.0, y + 1.0),
                        txt: s,
                        c: GRIS_AXE,
                        taille: NUM_TXT,
                        ancre: "end",
                        vertical: false,
                        gras: false,
                        italique: false,
                    });
                    prev_y = Some(y);
                }
            }
        }
    }
    out
}

impl Serie1 {
    /// Liste des pièces (mm) pour le style choisi, dans le repère de la
    /// page (grille et axes fournie par la couche commune).
    pub fn draw_list(&self, page: &Page) -> Vec<Piece> {
        if !self.dessiner || !page.mm_x(0.0).is_finite() || !page.mm_y(0.0).is_finite() {
            return Vec::new();
        }
        match self.style {
            Style::Camembert => self.dessin_camembert(page),
            Style::Histogramme => match &self.donnees {
                Donnees::Quali(_) => self.dessin_histo_quali(page),
                Donnees::Classes(_) => self.dessin_histo_classes(page),
                Donnees::Isolees(_) => Vec::new(),
            },
            Style::Batons => self.dessin_batons(page),
            Style::Boite => self.dessin_boite(page, false),
            Style::BoiteAvecDeciles => self.dessin_boite(page, true),
            Style::FreqCumul => self.dessin_cumulees(page),
            Style::DroiteHenry => self.dessin_henry(page),
            Style::LoiProba => self.dessin_loi(page),
        }
    }

    fn texte_titre(&self, p: &Page) -> Option<Piece> {
        titre_piece(p, &self.titre)
    }

    /// Camembert : fond opaque, part en couleurs pastel, étiquettes
    /// « nom : % », titre en haut. Départ à 12 h, sens horaire.
    fn dessin_camembert(&self, p: &Page) -> Vec<Piece> {
        let Donnees::Quali(mods) = &self.donnees else {
            return Vec::new();
        };
        let total = self.n_total();
        if total <= 0.0 {
            return Vec::new();
        }
        let cx = p.width_mm / 2.0;
        let cy = (p.height_mm + 6.0) / 2.0 + 2.0;
        let ray = (p.width_mm / 2.0 - 26.0)
            .min(p.height_mm / 2.0 - 8.0)
            .max(12.0);
        let mut out = vec![Piece::Fond {
            mm: (1.0, 1.0),
            w: p.width_mm - 2.0,
            h: p.height_mm - 2.0,
        }];
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        let mut angle = std::f64::consts::FRAC_PI_2;
        for m in mods {
            if m.n <= 0.0 {
                continue;
            }
            let balaye = std::f64::consts::TAU * m.n / total;
            let a0 = angle;
            let a1 = angle - balaye;
            let mut pts = vec![(cx, cy)];
            let nsteps = ((balaye.abs() / 0.035).ceil() as usize).max(2);
            for i in 0..=nsteps {
                let a = a0 + (a1 - a0) * i as f64 / nsteps as f64;
                pts.push((cx + ray * a.cos(), cy - ray * a.sin()));
            }
            out.push(Piece::Poly {
                pts,
                fill: Some(m.couleur),
                stroke: Some((NOIR, 0.25)),
            });
            let am = (a0 + a1) / 2.0;
            let lx = cx + (ray + 5.0) * am.cos();
            let ly = cy - (ray + 5.0) * am.sin();
            let ancre = if am.cos().abs() < 0.35 {
                "middle"
            } else if am.cos() > 0.0 {
                "end"
            } else {
                "start"
            };
            let pct = 100.0 * m.n / total;
            out.push(Piece::Texte {
                mm: (lx, ly),
                txt: format!("{} : {:.2} %", m.nom, pct),
                c: NOIR,
                taille: 2.2,
                ancre,
                vertical: false,
                gras: false,
                italique: false,
            });
            angle = a1;
        }
        out
    }

    /// Histogramme qualitatif : barre par modalité, hauteur = % ;
    /// « 25,80 % » au-dessus (gras) et modalité verticale sous l'axe.
    fn dessin_histo_quali(&self, p: &Page) -> Vec<Piece> {
        let Donnees::Quali(mods) = &self.donnees else {
            return Vec::new();
        };
        let total = self.n_total();
        if total <= 0.0 {
            return Vec::new();
        }
        let yax = p.mm_y(0.0);
        let mut out = pieces_axes(p, true, true, true);
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        for (i, m) in mods.iter().enumerate() {
            if m.n <= 0.0 {
                continue;
            }
            let pct = 100.0 * m.n / total;
            let xc = i as f64 + 1.0;
            let x0 = p.mm_x(xc - 0.375);
            let x1 = p.mm_x(xc + 0.375);
            let ytop = p.mm_y(pct);
            if !ytop.is_finite() {
                continue;
            }
            out.push(Piece::Rect {
                mm: (x0.min(x1), ytop.min(yax)),
                w: (x1 - x0).abs(),
                h: (yax - ytop).abs(),
                fill: Some(m.couleur),
                stroke: (NOIR, 0.25),
            });
            out.push(Piece::Texte {
                mm: ((x0 + x1) / 2.0, ytop.min(yax) - 1.0),
                txt: num2(pct),
                c: NOIR,
                taille: 2.3,
                ancre: "middle",
                vertical: false,
                gras: true,
                italique: false,
            });
            out.push(Piece::Texte {
                mm: ((x0 + x1) / 2.0, yax + 6.0),
                txt: m.nom.clone(),
                c: NOIR,
                taille: 2.6,
                ancre: "end",
                vertical: true,
                gras: false,
                italique: false,
            });
        }
        out
    }

    /// Histogramme en classes : aire des rectangles ∝ effectif ; le
    /// carreau header = `canaux` unités (« = 1.0 % » ou « = 5 ») ; base
    /// = bornes de la classe ; hauteur (mm²) = CARREAU² × n/canaux.
    fn dessin_histo_classes(&self, p: &Page) -> Vec<Piece> {
        let Donnees::Classes(c) = &self.donnees else {
            return Vec::new();
        };
        let total = self.n_total();
        if total <= 0.0 || self.canaux <= 0.0 {
            return Vec::new();
        }
        let yax = p.mm_y(0.0);
        let mut out = pieces_axes(p, true, true, true);
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        for (i, cl) in c.iter().enumerate() {
            if cl.n <= 0.0 || cl.b - cl.a <= 0.0 {
                continue;
            }
            let (x0, x1) = (p.mm_x(cl.a), p.mm_x(cl.b));
            let mut unite = if self.avec_pct {
                100.0 * cl.n / total
            } else {
                cl.n
            };
            unite /= self.canaux;
            let h = CARREAU * CARREAU * unite / (x1 - x0).abs();
            if !h.is_finite() || h <= 0.0 {
                continue;
            }
            out.push(Piece::Rect {
                mm: (x0.min(x1), yax - h),
                w: (x1 - x0).abs(),
                h,
                fill: Some(self.couleur),
                stroke: (NOIR, 0.25),
            });
            if self.aff_effectifs {
                out.push(Piece::Texte {
                    mm: ((x0 + x1) / 2.0, yax - h - 1.0),
                    txt: num(cl.n),
                    c: NOIR,
                    taille: 2.3,
                    ancre: "middle",
                    vertical: false,
                    gras: true,
                    italique: false,
                });
            }
            let _ = i;
        }
        // légende « 🟩 = 1,0 % » : un carreau posé sur la première colonne
        // de grille à droite de l'axe, texte gras à la rangée des
        // paramètres (mesures ecr-001 : carreau à +5,0 mm, texte à +8,65)
        let legende = if self.avec_pct {
            format!("= {:.1} %", self.canaux).replace('.', ",")
        } else {
            format!("= {}", num(self.canaux))
        };
        let ox = p.mm_x(p.origin_x);
        if ox.is_finite() {
            // première colonne de grille (5 mm) strictement à droite de l'axe
            let lx = ((ox / PAS_MM).floor() + 1.0) * PAS_MM;
            if lx < p.width_mm {
                out.push(Piece::Rect {
                    mm: (lx, yax + 5.0),
                    w: CARREAU,
                    h: CARREAU,
                    fill: Some(self.couleur),
                    stroke: (NOIR, 0.3),
                });
                out.push(Piece::Texte {
                    mm: (lx + CARREAU + 2.6, yax + TX_PARAM),
                    txt: legende,
                    c: NOIR,
                    taille: PARAM_TXT,
                    ancre: "start",
                    vertical: false,
                    gras: true,
                    italique: false,
                });
            }
        }
        if let Some(pr) = self.parametres() {
            dessin_parametres(&mut out, p, &pr, &self.visualiser);
        }
        out
    }

    /// Diagramme en bâtons (isolées) : bâton fin à chaque valeur ; le
    /// bâton de la médiane teinté quand les paramètres sont affichés.
    fn dessin_batons(&self, p: &Page) -> Vec<Piece> {
        let Donnees::Isolees(vals) = &self.donnees else {
            return Vec::new();
        };
        if vals.is_empty() {
            return Vec::new();
        }
        let yax = p.mm_y(0.0);
        let mut out = pieces_axes(p, true, true, true);
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        let pr = self.parametres();
        for r in vals {
            if r.n <= 0.0 || !r.v.is_finite() {
                continue;
            }
            let xc = p.mm_x(r.v);
            if !xc.is_finite() {
                continue;
            }
            let ytop = p.mm_y(r.n);
            if !ytop.is_finite() {
                continue;
            }
            let med = pr.as_ref().map(|q| q.mediane);
            let c = if med == Some(r.v) && self.visualiser != Visualiser::MedianeSeulement {
                ROSE_STICK
            } else {
                CYAN_BOITE
            };
            out.push(Piece::Rect {
                mm: (xc - 0.55, ytop.min(yax)),
                w: 1.1,
                h: (yax - ytop).abs(),
                fill: Some(c),
                stroke: (NOIR, 0.2),
            });
        }
        if let Some(pr) = pr {
            dessin_parametres(&mut out, p, &pr, &self.visualiser);
        }
        out
    }

    /// Boîte à moustaches (± déciles) posée au-dessus de l'axe des
    /// abscisses (axe seul) : boîte pleine de la couleur de la série,
    /// médiane rouge, moustaches noires épaisses, prolongements
    /// pointillés min→D1/D9→max, barres vertes aux déciles et cercles
    /// des valeurs isolées strictement au-delà des déciles.
    fn dessin_boite(&self, p: &Page, deciles: bool) -> Vec<Piece> {
        let Some(pr) = self.parametres() else {
            return Vec::new();
        };
        let ax_y = p.mm_y(0.0);
        if !(0.5..=p.height_mm - 0.5).contains(&ax_y) {
            return Vec::new();
        }
        // boîte centrée 22 mm au-dessus de l'axe (mesures img-008)
        let ym = ax_y - 22.0;
        let x = |v: f64| p.mm_x(v);
        let h = 25.0;
        let mut out = pieces_axes(p, false, true, true);
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        let (q1x, q3x) = (x(pr.q1), x(pr.q3));
        if deciles {
            out.push(Piece::Pointille {
                x0: x(pr.minimum),
                y0: ym,
                x1: x(pr.d1),
                y1: ym,
                c: NOIR,
                w: 0.35,
            });
            out.push(Piece::Pointille {
                x0: x(pr.d9),
                y0: ym,
                x1: x(pr.maximum),
                y1: ym,
                c: NOIR,
                w: 0.35,
            });
            // barres verticales des déciles, hauteur ≈ boîte
            for &d in [&pr.d1, &pr.d9] {
                out.push(Piece::Ligne {
                    x0: x(d),
                    y0: ym - h / 2.0,
                    x1: x(d),
                    y1: ym + h / 2.0,
                    c: VERT_DECILE,
                    w: 1.1,
                });
            }
            // valeurs isolées strictement au-delà des déciles : cercles
            if let Donnees::Isolees(vals) = &self.donnees {
                for r in vals {
                    if r.n <= 0.0 {
                        continue;
                    }
                    if r.v < pr.d1 - EPS || r.v > pr.d9 + EPS {
                        out.push(Piece::Disque {
                            c: (x(r.v), ym),
                            r: 1.05,
                            fill: [233, 100, 190],
                            stroke: None,
                        });
                    }
                }
            }
        }
        // boîte Q1…Q3 (couleur de la série, bord noir)
        out.push(Piece::Rect {
            mm: (q1x.min(q3x), ym - h / 2.0),
            w: (q3x - q1x).abs(),
            h,
            fill: Some(self.couleur),
            stroke: (NOIR, 0.35),
        });
        // moustaches noires épaisses : min→Q1 et Q3→max, ou D1→Q1 et
        // Q3→D9 quand les déciles sont montrés
        let gauche = if deciles { x(pr.d1) } else { x(pr.minimum) };
        let droite = if deciles { x(pr.d9) } else { x(pr.maximum) };
        out.push(Piece::Ligne {
            x0: gauche,
            y0: ym,
            x1: q1x.min(q3x),
            y1: ym,
            c: NOIR,
            w: 1.3,
        });
        out.push(Piece::Ligne {
            x0: q1x.max(q3x),
            y0: ym,
            x1: droite,
            y1: ym,
            c: NOIR,
            w: 1.3,
        });
        // médiane : trait rouge vertical dans la boîte
        out.push(Piece::Ligne {
            x0: x(pr.mediane),
            y0: ym - h / 2.0,
            x1: x(pr.mediane),
            y1: ym + h / 2.0,
            c: ROUGE,
            w: 0.65,
        });
        dessin_parametres(&mut out, p, &pr, &self.visualiser);
        out
    }

    /// Fréquences cumulées croissantes et/ou décroissantes polygones
    /// remplis + equipotentielles pointillées.
    fn dessin_cumulees(&self, p: &Page) -> Vec<Piece> {
        let Donnees::Classes(c) = &self.donnees else {
            return Vec::new();
        };
        if c.is_empty() {
            return Vec::new();
        }
        let yax = p.mm_y(0.0);
        let mut out = pieces_axes(p, true, true, true);
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        let n = self.cumuls();
        if self.croissantes {
            let mut pts = vec![(p.mm_x(c[0].a), yax)];
            for (b, cum) in &n {
                pts.push((p.mm_x(*b), p.mm_y(*cum)));
            }
            out.push(Piece::Poly {
                pts,
                fill: Some(self.couleur),
                stroke: Some((NOIR, 0.3)),
            });
        }
        if self.decroissantes {
            let total = self.n_total();
            let mut pts = vec![(p.mm_x(c[0].a), p.mm_y(total))];
            let mut cum = total;
            for cl in c {
                cum -= cl.n;
                pts.push((p.mm_x(cl.b), p.mm_y(cum)));
            }
            out.push(Piece::Poly {
                pts,
                fill: Some(self.couleur),
                stroke: Some((NOIR, 0.3)),
            });
        }
        // équipotentielles pointillées : horizontales aux seuils exacts
        // (k·N sur l'échelle des cumuls, y compris à mi-segment) depuis
        // l'axe Oy, verticales tombant aux valeurs interpolées des
        // paramètres — l'original montre les positions calculées, pas
        // les bornes de classes (img-021), puis fleurs et étiquettes
        // colorées sous l'axe
        let total = self.n_total();
        let vis = &self.visualiser;
        let tous = matches!(vis, Visualiser::Tous);
        let quartiles = matches!(vis, Visualiser::MedianeEtQuartiles | Visualiser::Tous);
        let Some(pr) = self.parametres() else {
            return out;
        };
        let mut items: Vec<(f64, &'static str, [u8; 3], f64)> = Vec::new();
        if tous {
            items.push((0.1, "D1", BLEU, pr.d1));
        }
        if quartiles {
            items.push((0.25, "Q1", BLEU, pr.q1));
        }
        items.push((0.5, "Med", ROUGE, pr.mediane));
        if quartiles {
            items.push((0.75, "Q3", BLEU, pr.q3));
        }
        if tous {
            items.push((0.9, "D9", BLEU, pr.d9));
        }
        let bas_y = p.mm_y(0.0);
        // les tirets horizontaux partent de l'axe Oy s'il est sur la page
        let ay_x = p.mm_x(p.origin_x);
        let gauche_x = if ay_x.is_finite() && (0.5..p.width_mm).contains(&ay_x) {
            ay_x
        } else {
            0.5
        };
        for (k, nom, coul, xv) in items {
            let x = p.mm_x(xv);
            if !x.is_finite() || !(0.0..=p.width_mm).contains(&x) {
                continue;
            }
            // niveau exact du point lu sur chaque courbe affichée :
            // k·N pour la croissante, N − k·N pour la décroissante
            let mut niveaux = Vec::new();
            if self.croissantes {
                niveaux.push(k * total);
            }
            if self.decroissantes {
                niveaux.push(total - k * total);
            }
            let mut vu = false;
            for t in niveaux {
                let y = p.mm_y(t);
                if !y.is_finite() || !(0.0..=p.height_mm).contains(&y) {
                    continue;
                }
                vu = true;
                out.push(Piece::Pointille {
                    x0: gauche_x,
                    y0: y,
                    x1: x,
                    y1: y,
                    c: coul,
                    w: 0.25,
                });
                out.push(Piece::Pointille {
                    x0: x,
                    y0: y,
                    x1: x,
                    y1: bas_y,
                    c: coul,
                    w: 0.25,
                });
            }
            if !vu {
                continue;
            }
            // étiquette seule (la capture cumulée n'a ni trait ni chute)
            out.push(Piece::Texte {
                mm: (x, bas_y + TX_PARAM),
                txt: nom.to_string(),
                c: coul,
                taille: PARAM_TXT,
                ancre: "middle",
                vertical: false,
                gras: true,
                italique: false,
            });
        }
        out
    }

    /// Droite de Henry : papier gausso-arithmétique (lignes bleues
    /// horizontales denses, % à gauche, nombres z à droite), droite de
    /// régression noire (fit des points) et points noirs pleins, plus
    /// deux lignes de caption sous le graphique.
    fn dessin_henry(&self, p: &Page) -> Vec<Piece> {
        let Donnees::Classes(c) = &self.donnees else {
            return Vec::new();
        };
        let Some(pr) = self.parametres() else {
            return Vec::new();
        };
        let n = pr.n;
        let mut out = pieces_axes(p, true, true, true);
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        // points (borne supérieure, z = Φ⁻¹(cumul/n)) du papier
        let mut pts: Vec<(f64, f64)> = Vec::new();
        let mut cum = 0.0;
        for cl in c {
            if n <= 0.0 {
                break;
            }
            cum += cl.n;
            let pc = cum / n;
            if pc >= 1.0 - EPS {
                continue;
            }
            let pc = pc.clamp(0.001, 0.999);
            pts.push((cl.b, probit(pc)));
        }
        // fit par moindres carrés z = a·x + b
        let fit = |pts: &[(f64, f64)]| -> (f64, f64) {
            let m = pts.len() as f64;
            let (sx, sz, sxx, sxz) = pts.iter().fold((0.0, 0.0, 0.0, 0.0), |acc, (x, z)| {
                (acc.0 + x, acc.1 + z, acc.2 + x * x, acc.3 + x * z)
            });
            let den = sxx * m - sx * sx;
            if den.abs() < 1e-12 {
                (0.0, sz / m)
            } else {
                let a = (sxz * m - sx * sz) / den;
                (a, (sz - a * sx) / m)
            }
        };
        let (a, b) = fit(&pts);
        // papier : lignes bleues fines aux probabilités classiques, % à
        // gauche ; lignes légèrement plus marquées aux z entiers, avec
        // les nombres z à droite
        let probs = [
            0.001, 0.005, 0.01, 0.02, 0.025, 0.05, 0.1, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.75,
            0.8, 0.9, 0.95, 0.98, 0.99, 0.995, 0.999,
        ];
        for &pc in &probs {
            let ym = mm_de_probit(p, pc);
            out.push(Piece::Ligne {
                x0: 1.0,
                y0: ym,
                x1: p.width_mm - 1.0,
                y1: ym,
                c: BLEU,
                w: 0.15,
            });
            out.push(Piece::Texte {
                mm: (1.4, ym + 0.9),
                txt: num_pct(pc),
                c: NOIR,
                taille: 2.0,
                ancre: "start",
                vertical: false,
                gras: false,
                italique: false,
            });
        }
        for z in [-3, -2, -1, 0, 1, 2, 3].map(|z| z as f64) {
            let ym = mm_de_z(p, z);
            out.push(Piece::Ligne {
                x0: 1.0,
                y0: ym,
                x1: p.width_mm - 1.0,
                y1: ym,
                c: BLEU,
                w: 0.3,
            });
            let s = num_axe(z);
            out.push(Piece::Texte {
                mm: (p.width_mm - 1.4, ym + 0.9),
                txt: s,
                c: NOIR,
                taille: 2.0,
                ancre: "end",
                vertical: false,
                gras: false,
                italique: false,
            });
        }
        // droite de Henry noire, étendue à la plage visible
        let (xlo, xhi) = p.x_range();
        out.push(Piece::Ligne {
            x0: p.mm_x(xlo),
            y0: mm_de_z(p, a * xlo + b),
            x1: p.mm_x(xhi),
            y1: mm_de_z(p, a * xhi + b),
            c: NOIR,
            w: 0.7,
        });
        // points noirs pleins
        for (v, z) in &pts {
            let ym = mm_de_z(p, *z);
            if ym.is_finite() {
                out.push(Piece::Disque {
                    c: (p.mm_x(*v), ym),
                    r: 1.05,
                    fill: NOIR,
                    stroke: None,
                });
            }
        }
        // captions sous le graphique
        let l1 = format!(
            "Droite de Henry obtenue : y = {} x {}{}",
            num_axe(a),
            if b < 0.0 { "−" } else { "+" },
            num_axe(b.abs())
        );
        let mu = -b / a;
        let sigma = 1.0 / a;
        let l2 = format!(
            "Estimation de la série statistique par la loi normale N({} ; {})",
            format!("{mu:.1}").replace('.', ","),
            format!("{sigma:.2}").replace('.', ",")
        );
        for (k, txt) in [l1, l2].iter().enumerate() {
            out.push(Piece::Texte {
                mm: (p.width_mm / 2.0, p.height_mm - 4.6 - 2.6 * k as f64),
                txt: txt.clone(),
                c: NOIR,
                taille: 2.4,
                ancre: "middle",
                vertical: false,
                gras: false,
                italique: false,
            });
        }
        out
    }

    /// Loi de probabilité (module statistiques v0.5) : la densité
    /// `f(x)` de la loi (par défaut la normale ajustée sur les données,
    /// comme la droite de Henry) est tracée sur l'axe des valeurs, avec
    /// une échelle de densité normalisée au pic.
    fn dessin_loi(&self, p: &Page) -> Vec<Piece> {
        let Some(pr) = self.parametres() else {
            return Vec::new();
        };
        let loi = self.loi.unwrap_or(Loi::Normal {
            mu: pr.moyenne,
            sigma: pr.sigma.max(0.5),
        });
        let (xlo, xhi, ymax) = loi.plage();
        if !xlo.is_finite() || !xhi.is_finite() || xlo > xhi || ymax <= 0.0 {
            return Vec::new();
        }
        // on borne la plage à l'axe visible de la page
        let (xlo, xhi) = (xlo.max(0.0).min(p.width_mm), xhi.max(0.0).min(p.width_mm));
        if xhi <= xlo {
            return Vec::new();
        }
        let mut out = pieces_axes(p, true, true, true);
        if let Some(t) = self.texte_titre(p) {
            out.push(t);
        }
        // courbe : échantillonnage de la densité, normalisée au pic
        let n = 240;
        let mut pts: Vec<(f64, f64)> = Vec::new();
        for i in 0..=n {
            let x = xlo + (xhi - xlo) * i as f64 / n as f64;
            let f = loi.pdf(x);
            if !f.is_finite() || f <= 0.0 {
                continue;
            }
            pts.push((p.mm_x(x), p.mm_y(f / ymax)));
        }
        if pts.len() >= 2 {
            out.push(Piece::Poly {
                pts,
                fill: None,
                stroke: Some((NOIR, 0.7)),
            });
        }
        let caption = loi.description();
        out.push(Piece::Texte {
            mm: (p.width_mm / 2.0, p.height_mm - 4.6),
            txt: caption,
            c: NOIR,
            taille: 2.4,
            ancre: "middle",
            vertical: false,
            gras: false,
            italique: false,
        });
        out
    }
}

/// La série « Boîtes à moustaches multiples » (4e volet de la boîte de
/// saisie de l'original) : plusieurs boîtes empilées à la même échelle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoitesMultiples {
    pub titre: String,
    pub boites: Vec<BoiteMultiple>,
    pub dessiner: bool,
}

impl BoitesMultiples {
    /// Dessin : boîtes verticales posées le long du repère, la première
    /// à 30 mm du croisement, suivantes espacées de 30 mm ; bord et
    /// moustaches bleus, losanges aux valeurs hors déciles, légende
    /// italique centrée dans la boîte (img-027 de l'original).
    pub fn draw_list(&self, p: &Page) -> Vec<Piece> {
        if !self.dessiner || self.boites.is_empty() {
            return Vec::new();
        }
        let mut out = pieces_axes(p, true, true, false);
        if let Some(t) = titre_piece(p, &self.titre) {
            out.push(t);
        }
        let x0 = p.mm_x(0.0);
        let base = if x0.is_finite() && (0.0..=p.width_mm - 30.0).contains(&x0) {
            x0
        } else {
            0.0
        };
        let y = |v: f64| p.mm_y(v);
        for (i, b) in self.boites.iter().enumerate() {
            let cxi = base + 30.0 * (i as f64 + 1.0);
            let (yq1, yq3) = (y(b.q1), y(b.q3));
            let top = yq1.min(yq3);
            let bot = yq1.max(yq3);
            // moustaches bleues : Q1→D1 et Q3→D9, avec min/max en losange
            // quand ils dépassent les déciles ; sinon min→Q1 / Q3→max
            let yd1 = b.d1.map(y);
            let yd9 = b.d9.map(y);
            let bout_g = yd1.unwrap_or_else(|| y(b.min));
            let bout_d = yd9.unwrap_or_else(|| y(b.max));
            out.push(Piece::Ligne {
                x0: cxi,
                y0: bot,
                x1: cxi,
                y1: bout_g,
                c: BLEU,
                w: 0.8,
            });
            out.push(Piece::Ligne {
                x0: cxi,
                y0: top,
                x1: cxi,
                y1: bout_d,
                c: BLEU,
                w: 0.8,
            });
            if yd1.is_some() {
                out.push(Piece::Ligne {
                    x0: cxi - 2.2,
                    y0: bout_g,
                    x1: cxi + 2.2,
                    y1: bout_g,
                    c: BLEU,
                    w: 0.8,
                });
            }
            if yd9.is_some() {
                out.push(Piece::Ligne {
                    x0: cxi - 2.2,
                    y0: bout_d,
                    x1: cxi + 2.2,
                    y1: bout_d,
                    c: BLEU,
                    w: 0.8,
                });
            }
            // losanges min/max hors déciles
            for yv in [
                if b.min < b.d1.unwrap_or(b.min) {
                    Some(y(b.min))
                } else {
                    None
                },
                if b.max > b.d9.unwrap_or(b.max) {
                    Some(y(b.max))
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            {
                out.push(Piece::Poly {
                    pts: vec![
                        (cxi, yv - 1.9),
                        (cxi + 1.1, yv),
                        (cxi, yv + 1.9),
                        (cxi - 1.1, yv),
                    ],
                    fill: Some(BLEU),
                    stroke: None,
                });
            }
            // boîte verticale
            out.push(Piece::Rect {
                mm: (cxi - 5.0, top),
                w: 10.0,
                h: (bot - top).abs(),
                fill: Some(b.couleur),
                stroke: (BLEU, 0.8),
            });
            if let Some(med) = b.med {
                out.push(Piece::Ligne {
                    x0: cxi - 5.0,
                    y0: y(med),
                    x1: cxi + 5.0,
                    y1: y(med),
                    c: BLEU,
                    w: 0.4,
                });
            }
            // légende italique centrée dans la boîte
            out.push(Piece::Texte {
                mm: (cxi, (top + bot) / 2.0 + 0.9),
                txt: b.legende.clone(),
                c: BLEU,
                taille: 2.4,
                ancre: "middle",
                vertical: false,
                gras: false,
                italique: true,
            });
        }
        out
    }
}

/// Titre centré en haut de la page (partagé par tous les dessins).
fn titre_piece(p: &Page, titre: &str) -> Option<Piece> {
    if titre.is_empty() {
        return None;
    }
    Some(Piece::Texte {
        mm: (p.width_mm / 2.0, 8.2),
        txt: titre.to_string(),
        c: NOIR,
        taille: 4.4,
        ancre: "middle",
        vertical: false,
        gras: true,
        italique: false,
    })
}

/// Étiquettes des paramètres sous l'axe (petits traits colorés + noms
/// D1/Q1/Med/Q3/D9 en gras, bleus/rouge comme l'original).
fn dessin_parametres(out: &mut Vec<Piece>, p: &Page, pr: &Params, vis: &Visualiser) {
    let yax = p.mm_y(0.0);
    if !yax.is_finite() {
        return;
    }
    let tous = matches!(vis, Visualiser::Tous);
    let quartiles = matches!(vis, Visualiser::MedianeEtQuartiles | Visualiser::Tous);
    let mut items: Vec<(f64, &'static str, [u8; 3])> = Vec::new();
    if tous {
        items.push((pr.d1, "D1", BLEU));
    }
    if quartiles {
        items.push((pr.q1, "Q1", BLEU));
    }
    items.push((pr.mediane, "Med", ROUGE));
    if quartiles {
        items.push((pr.q3, "Q3", BLEU));
    }
    if tous {
        items.push((pr.d9, "D9", BLEU));
    }
    for (v, nom, c) in items {
        let x = p.mm_x(v);
        if !x.is_finite() || x < -2.0 || x > p.width_mm + 2.0 {
            continue;
        }
        // trait coloré qui traverse l'axe (captures du manuel : ~2 mm de
        // chaque côté)
        out.push(Piece::Ligne {
            x0: x,
            y0: yax - 2.0,
            x1: x,
            y1: yax + 1.9,
            c,
            w: 0.8,
        });
        // nom coloré gras sous les nombres d'axe
        out.push(Piece::Texte {
            mm: (x, yax + TX_PARAM),
            txt: nom.to_string(),
            c,
            taille: PARAM_TXT,
            ancre: "middle",
            vertical: false,
            gras: true,
            italique: false,
        });
    }
}

/// Y (mm) correspondant à un z (échelle « papier gausso-arithmétique ») :
/// les probabilités décimales occupent uniformément la hauteur. mm_de_z
/// appelle mm_de_probit(Φ(z)).
fn mm_de_z(p: &Page, z: f64) -> f64 {
    mm_de_probit(p, phi(z))
}

/// Y (mm) du papier gausso-arithmétique pour une probabilité `pc` :
/// pc = 1 en haut de la zone utile (9 mm), pc = 0 en bas.
fn mm_de_probit(p: &Page, pc: f64) -> f64 {
    let lo = 9.0;
    let hi = p.height_mm - 9.0;
    hi - pc.clamp(0.0, 1.0) * (hi - lo)
}

/// Φ⁻¹ (Abramowitz-Stegun 26.2.23) ; p = probabilité cumulée.
pub fn probit(p: f64) -> f64 {
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    let q = if p < 0.5 { p } else { 1.0 - p };
    let t = (-2.0 * q.ln()).sqrt();
    let a = [2.515517, 0.802853, 0.010328];
    let b = [1.431788, 0.189269, 0.001308];
    let z = t - (a[0] + t * (a[1] + t * a[2])) / (1.0 + t * (b[0] + t * (b[1] + t * b[2])));
    if p < 0.5 {
        -z
    } else {
        z
    }
}

/// Φ(x), par erf.
fn phi(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// erf (série de Taylor : erf(x) = (2/√π) Σ(−1)^n x^(2n+1) / (n! (2n+1))).
fn erf(x: f64) -> f64 {
    let s = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    let mut sum = 0.0;
    let mut term_pow = ax;
    let mut n_fact = 1.0;
    let mut n: i32 = 0;
    loop {
        let term = (2.0 / std::f64::consts::PI.sqrt()) * term_pow / (n_fact * (2 * n + 1) as f64);
        sum += if n % 2 == 0 { term } else { -term };
        if n > 0 && term.abs() < 1e-17 {
            break;
        }
        n += 1;
        term_pow *= ax * ax;
        n_fact *= n as f64;
    }
    s * sum
}

impl Default for BoitesMultiples {
    fn default() -> Self {
        BoitesMultiples {
            titre: String::new(),
            boites: Vec::new(),
            dessiner: true,
        }
    }
}

/// Boîte du volet multiple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoiteMultiple {
    pub legende: String,
    pub min: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d1: Option<f64>,
    pub q1: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub med: Option<f64>,
    pub q3: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d9: Option<f64>,
    pub max: f64,
    pub couleur: [u8; 3],
}

// ---------------------------------------------------------------- suites

/// Nombre maximal de termes d'une suite tracée.
pub const NB_TERME: usize = 24;

/// Définition d'une suite numérique (module statistiques v0.6) :
/// `u_n = f(n)` (suite explicite) ou `u_{n+1} = f(u_n)` (suite par
/// récurrence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuiteDef {
    /// suite explicite : u_n = f(n)
    Lineaire { f: String },
    /// suite par récurrence : u_{n+1} = f(u_n)
    Recurrence { f: String },
}

impl SuiteDef {
    /// Évalue `f` (variable `var`) en `v`.
    fn eval_f(f: &str, var: &str, v: f64) -> Option<f64> {
        let ast = crate::expr::parse(f).ok()?;
        ast.validate_vars(&[var]).ok()?;
        let mut vars = crate::expr::VarSet::default();
        if var == "n" {
            vars.n = Some(v);
        } else {
            vars.u = Some(v);
        }
        Some(ast.eval(&vars, &mut Vec::new()))
    }

    /// u_n d'une suite explicite (`u_n = f(n)`).
    pub fn valeur(&self, n: f64) -> Option<f64> {
        match self {
            SuiteDef::Lineaire { f } => Self::eval_f(f, "n", n),
            SuiteDef::Recurrence { .. } => None,
        }
    }

    /// Termes de la suite : pour `Lineaire` les u_n (n = 0..) ; pour
    /// `Recurrence` les itérations successives depuis u_0 = 0.
    pub fn termes(&self, n: usize) -> Vec<f64> {
        let mut out = Vec::with_capacity(n);
        match self {
            SuiteDef::Lineaire { .. } => {
                for k in 0..n {
                    out.push(self.valeur(k as f64).unwrap_or(f64::NAN));
                }
            }
            SuiteDef::Recurrence { f } => {
                let mut u = 0.0f64;
                for _ in 0..n {
                    out.push(u);
                    u = Self::eval_f(f, "u", u).unwrap_or(f64::NAN);
                }
            }
        }
        out
    }
}

/// Une suite numérique (module statistiques v0.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suite {
    pub titre: String,
    pub def: SuiteDef,
    pub dessiner: bool,
}

impl Suite {
    /// Dessin : escalier (u_n = f(n)) ou toile d'araignée (itérations de
    /// la récurrence), avec légende.
    pub fn draw_list(&self, p: &Page) -> Vec<Piece> {
        if !self.dessiner || !p.mm_x(0.0).is_finite() || !p.mm_y(0.0).is_finite() {
            return Vec::new();
        }
        let termes = self.def.termes(NB_TERME);
        if termes.iter().any(|v| v.is_nan()) {
            // expression invalide : rien à tracer
            return Vec::new();
        }
        let (f, recurrence) = match &self.def {
            SuiteDef::Lineaire { f } => (f.clone(), false),
            SuiteDef::Recurrence { f } => (f.clone(), true),
        };
        let mut out = Vec::new();
        for i in 0..termes.len().saturating_sub(1) {
            let (x0, y0) = (p.mm_x(i as f64), p.mm_y(termes[i]));
            let (x1, y1) = (p.mm_x((i + 1) as f64), p.mm_y(termes[i + 1]));
            out.push(Piece::Ligne {
                x0,
                y0,
                x1,
                y1,
                c: NOIR,
                w: 0.7,
            });
            out.push(Piece::Ligne {
                x0: x1,
                y0,
                x1,
                y1,
                c: NOIR,
                w: 0.7,
            });
        }
        let caption = if recurrence {
            format!("u_{{n+1}} = {f}")
        } else {
            format!("u_n = {f}")
        };
        out.push(Piece::Texte {
            mm: (p.width_mm / 2.0, p.height_mm - 4.6),
            txt: caption,
            c: NOIR,
            taille: 2.4,
            ancre: "middle",
            vertical: false,
            gras: false,
            italique: false,
        });
        out
    }
}

// ---------------------------------------------------------------- exemples

/// Exemple de l'original : valeurs isolées (les nombres du panneau
/// Calculs de p.45 : moyenne 1,958 ; écart type 1,69595…).
pub fn exemple_enfants() -> Serie1 {
    Serie1 {
        titre: "Nombre d'enfants dans la famille".into(),
        donnees: Donnees::Isolees(vec![
            ValEff { v: 0.0, n: 105.0 },
            ValEff { v: 1.0, n: 139.0 },
            ValEff { v: 2.0, n: 97.0 },
            ValEff { v: 3.0, n: 63.0 },
            ValEff { v: 4.0, n: 47.0 },
            ValEff { v: 5.0, n: 33.0 },
            ValEff { v: 6.0, n: 10.0 },
            ValEff { v: 7.0, n: 4.0 },
            ValEff { v: 8.0, n: 2.0 },
        ]),
        couleur: [0, 200, 200],
        ..Default::default()
    }
}

/// Exemple de l'original : classes (les valeurs de p.46 : moy 1006,
/// écart type 116,679 …).
pub fn exemple_salaires() -> Serie1 {
    Serie1 {
        titre: "Répartition des salaires".into(),
        donnees: Donnees::Classes(vec![
            Classe {
                a: 800.0,
                b: 900.0,
                n: 30.0,
            },
            Classe {
                a: 900.0,
                b: 1000.0,
                n: 42.0,
            },
            Classe {
                a: 1000.0,
                b: 1050.0,
                n: 39.0,
            },
            Classe {
                a: 1050.0,
                b: 1150.0,
                n: 18.0,
            },
            Classe {
                a: 1150.0,
                b: 1300.0,
                n: 21.0,
            },
        ]),
        couleur: LIME,
        ..Default::default()
    }
}

/// Exemple qualitatif (p.43).
pub fn exemple_quali() -> Serie1 {
    let rows = [
        "Aucun diplôme",
        "Brevet des collèges",
        "CAP-BEP",
        "Bac ou équivalent",
        "BTS-DUT",
        "Licence et +",
        "En cours d'étude",
    ];
    let n = [15480.0, 3960.0, 12180.0, 8640.0, 5940.0, 7740.0, 6060.0];
    Serie1 {
        titre: "Niveau d'études des + de 15 ans en France en 2012 (x 1000 hab.)".into(),
        donnees: Donnees::Quali(
            rows.iter()
                .zip(n)
                .enumerate()
                .map(|(i, (nom, nn))| Modalite {
                    nom: nom.to_string(),
                    n: nn,
                    couleur: COULEURS_MODALITES[i % COULEURS_MODALITES.len()],
                })
                .collect(),
        ),
        style: Style::Histogramme,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enfants_parametres_egalaux_original() {
        let q = exemple_enfants().parametres().unwrap();
        assert_eq!(q.n, 500.0);
        assert_eq!(q.somme, 979.0);
        assert_eq!(q.moyenne, 1.958);
        assert!((q.sigma - 1.69595).abs() < 5e-5, "{}", q.sigma);
        assert_eq!(q.minimum, 0.0);
        assert_eq!(q.maximum, 8.0);
        assert_eq!(q.d1, 0.0);
        assert_eq!(q.q1, 1.0);
        assert_eq!(q.mediane, 2.0);
        assert_eq!(q.q3, 3.0);
        assert_eq!(q.d9, 4.0);
    }

    #[test]
    fn salaires_parametres_egalaux_original() {
        let q = exemple_salaires().parametres().unwrap();
        assert_eq!(q.n, 150.0);
        assert_eq!(q.moyenne, 1006.0);
        assert!((q.sigma - 116.679).abs() < 5e-3, "{}", q.sigma);
        assert_eq!(q.minimum, 800.0);
        assert_eq!(q.maximum, 1300.0);
        assert_eq!(q.d1, 850.0);
        assert!((q.q1 - 917.857).abs() < 1e-2, "{}", q.q1);
        assert!((q.mediane - 1003.85).abs() < 1e-2, "{}", q.mediane);
        assert!((q.q3 - 1058.33).abs() < 1e-2, "{}", q.q3);
        assert!((q.d9 - 1192.86).abs() < 1e-2, "{}", q.d9);
    }

    #[test]
    fn quali_pas_de_parametres() {
        assert!(exemple_quali().parametres().is_none());
        assert_eq!(exemple_quali().n_total(), 60000.0);
    }

    #[test]
    fn serie_vide_dessine_rien() {
        let s = Serie1::default();
        assert!(s.parametres().is_none());
        let p = Page::default();
        assert!(s.draw_list(&p).is_empty());
    }

    #[test]
    fn json_aller_retour_serie() {
        let s = exemple_salaires();
        let txt = serde_json::to_string(&s).unwrap();
        let back: Serie1 = serde_json::from_str(&txt).unwrap();
        assert_eq!(back, s);
        // valeurs par défaut des champs v0.4+
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["canaux"], 1.0);
        assert_eq!(v["avec_pct"], true);
    }

    #[test]
    fn loi_normale_cdf_sensible() {
        let loi = Loi::Normal {
            mu: 0.0,
            sigma: 1.0,
        };
        assert!((loi.cdf(0.0) - 0.5).abs() < 1e-9);
        assert!((loi.cdf(1.96) - 0.975).abs() < 5e-3, "{}", loi.cdf(1.96));
        assert!((loi.cdf(-1.96) - 0.025).abs() < 5e-3);
        assert!((loi.cdf(6.0) - 0.9999999998).abs() < 1e-6);
        // densité : pic = 1/(σ√(2π))
        assert!((loi.pdf(0.0) - 1.0 / (2.0 * std::f64::consts::PI).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn loi_binomiale_somme_une() {
        let loi = Loi::Binomial { n: 10.0, p: 0.3 };
        let mut s = 0.0;
        for k in 0..=10 {
            s += loi.pdf(k as f64);
        }
        assert!((s - 1.0).abs() < 1e-9, "{s}");
        assert!((loi.cdf(10.0) - 1.0).abs() < 1e-9);
        assert!((loi.cdf(2.0) - loi.cdf(1.0) - loi.pdf(2.0)).abs() < 1e-9);
        // P(X = 2) = C(10,2) 0,3² 0,7⁸
        let mut comb: f64 = 1.0;
        for i in 1..=2 {
            comb *= (10.0 - 2.0 + i as f64) / i as f64;
        }
        let attendu = comb * 0.09 * 0.7f64.powi(8);
        assert!((loi.pdf(2.0) - attendu).abs() < 1e-12);
    }

    #[test]
    fn loi_poisson_pdf_sensible() {
        let loi = Loi::Poisson { mu: 3.4 };
        let mut s = 0.0;
        for k in 0..=30 {
            s += loi.pdf(k as f64);
        }
        assert!((s - 1.0).abs() < 5e-3, "{s}");
        assert!((loi.cdf(30.0) - 1.0).abs() < 5e-3);
        assert!(loi.cdf(0.0) < 0.05);
    }

    #[test]
    fn loi_exponentielle_cdf() {
        let loi = Loi::Exponentielle { lambda: 1.5 };
        assert!(loi.cdf(0.0) < 1e-12);
        assert!((loi.cdf(10.0) - (1.0 - (-15.0f64).exp())).abs() < 1e-9);
        assert!((loi.probabilite(0.0) - 1.5).abs() < 1e-12);
    }

    #[test]
    fn serie_loi_proba_dessine_une_courbe() {
        let s = Serie1 {
            titre: "loi".into(),
            donnees: Donnees::Isolees(vec![ValEff { v: 1.0, n: 1.0 }]),
            style: Style::LoiProba,
            ..Default::default()
        };
        let p = Page::default();
        let pieces = s.draw_list(&p);
        assert!(!pieces.is_empty());
        assert!(
            matches!(pieces[0], Piece::Ligne { .. }) || matches!(pieces[0], Piece::Poly { .. })
        );
    }

    #[test]
    fn suite_lineaire_termes() {
        let d = SuiteDef::Lineaire { f: "2n+1".into() };
        assert_eq!(d.valeur(3.0), Some(7.0));
        let termes = d.termes(5);
        assert_eq!(termes.len(), 5);
        assert_eq!(termes[0], 1.0);
        assert_eq!(termes[4], 9.0);
    }

    #[test]
    fn suite_recurrente_termes() {
        let d = SuiteDef::Recurrence { f: "u+1".into() };
        let termes = d.termes(5);
        assert_eq!(termes.len(), 5);
        assert_eq!(termes[0], 0.0);
        assert_eq!(termes[4], 4.0);
    }

    #[test]
    fn suite_invalide_dessine_vide() {
        let s = Suite {
            titre: "S".into(),
            def: SuiteDef::Lineaire { f: "2++".into() },
            dessiner: true,
        };
        assert!(s.draw_list(&Page::default()).is_empty());
    }

    #[test]
    fn suite_dessine_escalier() {
        let s = Suite {
            titre: "Suite 1".into(),
            def: SuiteDef::Lineaire { f: "2n+1".into() },
            dessiner: true,
        };
        let pieces = s.draw_list(&Page::default());
        assert!(!pieces.is_empty());
        assert!(pieces.iter().any(|p| matches!(p, Piece::Ligne { .. })));
    }

    #[test]
    fn suite_dessiner_fausse_vide() {
        let s = Suite {
            titre: "S".into(),
            def: SuiteDef::Lineaire { f: "2n+1".into() },
            dessiner: false,
        };
        assert!(s.draw_list(&Page::default()).is_empty());
    }

    #[test]
    fn probit_sensiblement_correct() {
        // Abramowitz-Stegun 26.2.23 : erreur max ~4,5e-4
        assert!((probit(0.5)).abs() < 5e-4, "{}", probit(0.5));
        assert!((probit(0.975) - 1.96).abs() < 0.01, "{}", probit(0.975));
        assert!((probit(0.025) + 1.96).abs() < 0.01);
    }
}
