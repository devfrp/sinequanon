//! Expressions mathématiques en code LaTeX (man. p. 41-43, v1.6).
//!
//! Sous-ensemble du langage, comme l'original : préfixe de taille
//! `4$`, couleurs `\blue` `\red` `\green` `\black`, fraction `\frac{…}
//! {…}`, racine carrée `\sqrt{…}`, exposants `^…`, indices `_…`, et
//! les symboles grecs/courants disponibles dans la fonte bitmap
//! (α β γ δ Δ ε θ λ μ π ρ σ Σ τ φ Φ ω Ω ∞ √ × ≤ ≥ ′ é è …).
//!
//! L'assemblage décompose l'expression en une liste de pièces
//! (`Piece::Texte` par bloc, `Piece::Ligne` pour les barres de
//! fraction et les vinculum), positionnées en mm autour d'un point
//! d'ancrage donné par ses coordonnées utilisateur.

use crate::grid::Page;
use crate::stats::Piece;

/// Avance horizontale d'un caractère, en mm (fonte bitmap : avance
/// 6 px pour une casse de 7 px).
fn avance(taille: f64) -> f64 {
    6.0 / 7.0 * taille
}

/// Hauteur de casse au-dessus du ligne de base (mm).
fn casse(taille: f64) -> f64 {
    0.72 * taille
}

/// Couleurs nommées du sous-ensemble LaTeX.
fn couleur_nommee(nom: &str) -> Option<[u8; 3]> {
    match nom {
        "blue" => Some([0, 0, 255]),
        "red" => Some([255, 0, 0]),
        "green" => Some([0, 128, 0]),
        "black" => Some([0, 0, 0]),
        "white" => Some([255, 255, 255]),
        "darkgreen" => Some([0, 100, 0]),
        _ => None,
    }
}

/// Symboles du sous-ensemble LaTeX (fonte bitmap : √ π θ ∞ × ≤ ≥ ′ Ω)
/// ; les autres commandes inconnues sont recopiées telles quelles.
fn symbole(cmd: &str) -> Option<&'static str> {
    Some(match cmd {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "Delta" => "Δ",
        "epsilon" | "varepsilon" => "ε",
        "theta" => "θ",
        "lambda" => "λ",
        "mu" => "μ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "Sigma" => "Σ",
        "tau" => "τ",
        "phi" | "varphi" => "φ",
        "Phi" => "Φ",
        "omega" => "ω",
        "Omega" => "Ω",
        "infty" => "∞",
        "times" => "×",
        "leq" => "≤",
        "geq" => "≥",
        "prime" => "′",
        "sqrt" => "√",
        "euro" => "€",
        "Euro" => "€",
        _ => return None,
    })
}

/// Bloc rendu : pièces posées (une coupe au-dessus du ligne de base
/// locale y = 0), largeur, hauteur au-dessus et sous le ligne de base.
struct Boite {
    pieces: Vec<Piece>,
    largeur: f64,
    haut: f64,
    bas: f64,
}

impl Boite {
    fn vide() -> Self {
        Boite {
            pieces: Vec::new(),
            largeur: 0.0,
            haut: 0.0,
            bas: 0.0,
        }
    }
}

/// Découpage en blocs usuels : renvoie la sous-expression entre
/// accolades (ou le caractère simple si pas d'accolade), avec la
/// position juste après.
fn lit_groupe(code: &[char], i: &mut usize) -> Vec<char> {
    while *i < code.len() && code[*i] == ' ' {
        *i += 1;
    }
    if *i < code.len() && code[*i] == '{' {
        *i += 1;
        let debut = *i;
        let mut profondeur = 1;
        while *i < code.len() {
            match code[*i] {
                '{' => profondeur += 1,
                '}' => {
                    profondeur -= 1;
                    if profondeur == 0 {
                        break;
                    }
                }
                _ => {}
            }
            *i += 1;
        }
        let gr = code[debut..*i].to_vec();
        if *i < code.len() {
            *i += 1;
        }
        return gr;
    }
    if *i < code.len() {
        let ch = code[*i];
        *i += 1;
        return vec![ch];
    }
    Vec::new()
}

/// Commande après une barre oblique : lettres, ou lettre simple.
fn lit_commande(code: &[char], i: &mut usize) -> String {
    let debut = *i;
    while *i < code.len() && code[*i].is_ascii_alphabetic() {
        *i += 1;
    }
    if *i > debut {
        code[debut..*i].iter().collect()
    } else {
        *i += 1;
        String::new()
    }
}

/// Assemble une sous-expression (un groupe de caractères) à partir de
/// l'ancrage (x, y) en mm : baseline locale à y. Renvoie la boîte avec
/// les pièces en placements absolus (mm).
fn assemble(
    code: &[char],
    taille: f64,
    couleur: [u8; 3],
    ancre: (f64, f64),
    profondeur: usize,
) -> Boite {
    if profondeur > 12 {
        return Boite::vide();
    }
    let mut i = 0;
    let mut x = ancre.0;
    let mut haut = 0.0f64;
    let mut bas = 0.0f64;
    let mut pieces: Vec<Piece> = Vec::new();
    // taille courante (modifiée par le préfixe `n$`) et couleur
    let mut t = taille;
    let mut c = couleur;
    while i < code.len() {
        match code[i] {
            ' ' => {
                x += avance(t) * 0.5;
                i += 1;
            }
            '{' | '}' => {
                i += 1; // groupes étrangers : traverse
            }
            '\\' => {
                i += 1;
                let cmd = lit_commande(code, &mut i);
                if let Some(cc) = couleur_nommee(&cmd) {
                    c = cc;
                    // une commande de style avale l'espace qui la suit
                    while i < code.len() && code[i] == ' ' {
                        i += 1;
                    }
                    continue;
                }
                if cmd == "frac" {
                    let num = lit_groupe(code, &mut i);
                    let den = lit_groupe(code, &mut i);
                    if !num.is_empty() || !den.is_empty() {
                        let b = frac(&num, &den, t, c, (x, ancre.1), profondeur);
                        x += b.largeur;
                        haut = haut.max(b.haut);
                        bas = bas.max(b.bas);
                        pieces.extend(b.pieces);
                        continue;
                    }
                } else if cmd == "sqrt" {
                    let gr = lit_groupe(code, &mut i);
                    if !gr.is_empty() {
                        let b = racine(&gr, t, c, (x, ancre.1), profondeur);
                        x += b.largeur;
                        haut = haut.max(b.haut);
                        bas = bas.max(b.bas);
                        pieces.extend(b.pieces);
                        continue;
                    }
                } else if let Some(sym) = symbole(&cmd) {
                    let piece = Piece::Texte {
                        mm: (x, ancre.1),
                        txt: sym.to_string(),
                        c,
                        taille: t,
                        ancre: "",
                        vertical: false,
                        gras: false,
                        italique: false,
                    };
                    pieces.push(piece);
                    x += avance(t) * sym.chars().count() as f64;
                    haut = haut.max(casse(t));
                    bas = bas.max(0.25 * t);
                    continue;
                }
                // commande inconnue : recopiée telle quelle
                let s = format!("\\{}", cmd);
                let piece = Piece::Texte {
                    mm: (x, ancre.1),
                    txt: s,
                    c,
                    taille: t,
                    ancre: "",
                    vertical: false,
                    gras: false,
                    italique: false,
                };
                pieces.push(piece);
                x += avance(t) * (cmd.chars().count() + 1) as f64;
            }
            '$' => {
                i += 1; // '$' isolé : ignoré
            }
            '0'..='9' if i + 1 < code.len() && code[i + 1] == '$' => {
                // préfixe de taille `4$` : la valeur passe en mm
                t = (code[i] as u8 - b'0') as f64;
                i += 2;
            }
            '^' | '_' => {
                let mode = code[i];
                i += 1;
                let gr = lit_groupe(code, &mut i);
                if gr.is_empty() {
                    continue;
                }
                let t0 = t * 0.7;
                let dy = if mode == '^' { -0.45 * t } else { 0.32 * t };
                let b = assemble(&gr, t0, c, (x, ancre.1 + dy), profondeur + 1);
                x += b.largeur;
                haut = haut.max(b.haut - dy);
                bas = bas.max(b.bas + dy);
                pieces.extend(b.pieces);
            }
            ch => {
                let piece = Piece::Texte {
                    mm: (x, ancre.1),
                    txt: ch.to_string(),
                    c,
                    taille: t,
                    ancre: "",
                    vertical: false,
                    gras: false,
                    italique: false,
                };
                pieces.push(piece);
                x += avance(t);
                haut = haut.max(casse(t));
                bas = bas.max(0.25 * t);
                i += 1;
            }
        }
    }
    Boite {
        pieces,
        largeur: x - ancre.0,
        haut,
        bas,
    }
}

/// Fraction : numérateur au-dessus, barre, dénominateur au-dessous,
/// chacun à 62 % de la taille, centrés l'un sur l'autre (man. p. 42).
fn frac(
    num: &[char],
    den: &[char],
    t: f64,
    c: [u8; 3],
    ancre: (f64, f64),
    profondeur: usize,
) -> Boite {
    let t0 = t * 0.62;
    let bn = assemble(num, t0, c, (0.0, 0.0), profondeur + 1);
    let bd = assemble(den, t0, c, (0.0, 0.0), profondeur + 1);
    let largeur = bn.largeur.max(bd.largeur) + 0.3 * t;
    let cx = ancre.0 + (largeur - bn.largeur.max(bd.largeur)) / 2.0;
    // placements : numéraire baseline à −0.62·t, dénominateur à +0.62·t
    let mut pieces = Vec::new();
    for pc in &bn.pieces {
        match pc {
            Piece::Texte {
                mm,
                txt,
                taille,
                gras,
                italique,
                ..
            } => {
                pieces.push(Piece::Texte {
                    mm: (cx + mm.0, ancre.1 - 0.62 * t + mm.1),
                    txt: txt.clone(),
                    c,
                    taille: *taille,
                    ancre: "",
                    vertical: false,
                    gras: *gras,
                    italique: *italique,
                });
            }
            Piece::Ligne {
                x0, y0, x1, y1, w, ..
            } => {
                pieces.push(Piece::Ligne {
                    x0: cx + x0,
                    y0: ancre.1 - 0.62 * t + y0,
                    x1: cx + x1,
                    y1: ancre.1 - 0.62 * t + y1,
                    c,
                    w: *w,
                });
            }
            _ => {}
        }
    }
    for pc in &bd.pieces {
        match pc {
            Piece::Texte {
                mm,
                txt,
                taille,
                gras,
                italique,
                ..
            } => {
                pieces.push(Piece::Texte {
                    mm: (cx + mm.0, ancre.1 + 0.62 * t + mm.1),
                    txt: txt.clone(),
                    c,
                    taille: *taille,
                    ancre: "",
                    vertical: false,
                    gras: *gras,
                    italique: *italique,
                });
            }
            Piece::Ligne {
                x0, y0, x1, y1, w, ..
            } => {
                pieces.push(Piece::Ligne {
                    x0: cx + x0,
                    y0: ancre.1 + 0.62 * t + y0,
                    x1: cx + x1,
                    y1: ancre.1 + 0.62 * t + y1,
                    c,
                    w: *w,
                });
            }
            _ => {}
        }
    }
    // barre de fraction
    pieces.push(Piece::Ligne {
        x0: ancre.0,
        y0: ancre.1 - 0.06 * t,
        x1: ancre.0 + largeur,
        y1: ancre.1 - 0.06 * t,
        c,
        w: (0.16 * t).max(0.18),
    });
    Boite {
        largeur,
        haut: 0.62 * t + bn.haut,
        bas: 0.62 * t + bd.bas,
        pieces,
    }
}

/// Racine carrée : le glyphe √ suivi du contenu, recouvert d'un
/// vinculum (trait horizontal) qui le surlonge légèrement.
fn racine(gr: &[char], t: f64, c: [u8; 3], ancre: (f64, f64), profondeur: usize) -> Boite {
    let mut pieces = Vec::new();
    let x0 = ancre.0;
    // le glyphe √
    pieces.push(Piece::Texte {
        mm: (x0, ancre.1),
        txt: "√".to_string(),
        c,
        taille: t,
        ancre: "",
        vertical: false,
        gras: false,
        italique: false,
    });
    let depart = x0 + avance(t);
    let b = assemble(gr, t, c, (depart, ancre.1), profondeur + 1);
    pieces.extend(b.pieces);
    // vinculum : barre au sommet du contenu (hors √)
    let sommet = ancre.1 - casse(t) + 0.04 * t;
    pieces.push(Piece::Ligne {
        x0: depart - 0.08 * t,
        y0: sommet,
        x1: depart + b.largeur + 0.15 * t,
        y1: sommet,
        c,
        w: (0.16 * t).max(0.18),
    });
    Boite {
        largeur: avance(t) + b.largeur + 0.2 * t,
        haut: casse(t) + 0.06 * t,
        bas: b.bas,
        pieces,
    }
}

/// Assemble l'expression LaTeX entière sur la position utilisateur
/// (x, y) (baseline au point d'ancrage) et renvoie la liste des pièces.
pub fn rendu(p: &Page, x: f64, y: f64, code: &str, couleur: [u8; 3]) -> Vec<Piece> {
    let chars: Vec<char> = code.chars().collect();
    let b = assemble(&chars, 3.2, couleur, (p.mm_x(x), p.mm_y(y)), 0);
    b.pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        Page::default()
    }

    #[test]
    fn texte_simple_position() {
        let p = page();
        let pieces = rendu(&p, 0.0, 0.0, "f(x)=3x", [0, 0, 0]);
        // un caractère = une pièce
        assert_eq!(pieces.len(), 7);
        let Piece::Texte { mm, taille, .. } = &pieces[0] else {
            unreachable!()
        };
        assert_eq!(*taille, 3.2);
        assert!((mm.0 - p.mm_x(0.0)).abs() < 1e-9);
        assert!((mm.1 - p.mm_y(0.0)).abs() < 1e-9);
    }

    #[test]
    fn prerefixe_taille_et_couleur() {
        let p = page();
        let pieces = rendu(&p, 0.0, 0.0, "4$\\blue12", [0, 0, 0]);
        assert_eq!(pieces.len(), 2);
        let infos: Vec<(f64, [u8; 3])> = pieces
            .iter()
            .map(|q| {
                let Piece::Texte { mm, taille, c, .. } = q else {
                    unreachable!()
                };
                (taille + mm.0, *c)
            })
            .collect();
        assert_eq!(infos[0].1, [0, 0, 255]);
        assert_eq!(infos[1].1, [0, 0, 255]);
        // taille 4 demandée
        let taille = pieces
            .iter()
            .map(|q| {
                let Piece::Texte { taille, .. } = q else {
                    unreachable!()
                };
                *taille
            })
            .collect::<Vec<_>>();
        assert_eq!(taille, [4.0, 4.0]);
        // décalage horizontal de 2 caractères
        let largeur_char = 6.0 / 7.0 * 4.0;
        assert!((infos[1].0 - infos[0].0).abs() > largeur_char - 0.01);
    }

    #[test]
    fn fraction_deux_niveaux() {
        let p = page();
        let pieces = rendu(&p, 0.0, 0.0, "\\frac{3x+1}{x^2+1}", [0, 0, 0]);
        // numérateur (4 glyphes) + exposant (1) + dénominateur (4) +
        // exposant + barre
        let nbarre = pieces
            .iter()
            .filter(|q| matches!(q, Piece::Ligne { .. }))
            .count();
        assert_eq!(nbarre, 1);
        // les 2 tailles fines 0.62·3.2 = 1.984
        let petites = pieces
            .iter()
            .filter_map(|q| {
                let Piece::Texte { taille, .. } = q else {
                    return None;
                };
                if (*taille - 3.2 * 0.62).abs() < 1e-9 {
                    Some(1)
                } else {
                    None
                }
            })
            .count();
        // numérateur 4 blocs à 0.62·t + dénominateur 3 à 0.62·t (le ²
        // exposant est encore plus petit, 0.7·0.62·t) + barre
        assert_eq!(petites, 7);
        assert_eq!(pieces.len(), 9);
        // la barre de fraction : le seul trait, centré y = baseline
        let barre = pieces
            .iter()
            .find_map(|q| match q {
                Piece::Ligne { y0, y1, .. } => Some((*y0, *y1)),
                _ => None,
            })
            .expect("barre absente");
        assert_eq!(barre.0, barre.1);
        // les numérateur et dénominateur sont des deux côtés de la barre
        let Piece::Texte { mm, .. } = &pieces[0] else {
            unreachable!()
        };
        assert!(mm.1 < barre.1);
        let Piece::Texte { mm, .. } = &pieces[4] else {
            unreachable!()
        };
        assert!(mm.1 > barre.1);
    }

    #[test]
    fn racine_recouverte() {
        let p = page();
        let pieces = rendu(&p, 0.0, 0.0, "\\sqrt{x^2+1}", [0, 0, 0]);
        // √ + contenu + vinculum
        assert!(pieces.len() >= 6);
        let Piece::Texte { txt, .. } = &pieces[0] else {
            unreachable!()
        };
        assert_eq!(txt, "√");
        let dernier = pieces.last().unwrap();
        assert!(matches!(dernier, Piece::Ligne { .. }));
    }

    #[test]
    fn symboles_et_exposant() {
        let p = page();
        let pieces = rendu(&p, 0.0, 0.0, "\\pi\\leq4", [0, 0, 0]);
        assert!(pieces.len() >= 3);
        let Piece::Texte { txt, .. } = &pieces[0] else {
            unreachable!()
        };
        assert_eq!(txt, "π");
        let Piece::Texte { txt, .. } = &pieces[1] else {
            unreachable!()
        };
        assert_eq!(txt, "≤");
        // exposant : x^1 → 2 pièces horizonales et dernières tailles
        let pieces2 = rendu(&p, 0.0, 0.0, "x^2", [0, 0, 0]);
        assert_eq!(pieces2.len(), 2);
        let Piece::Texte { taille, mm, .. } = &pieces2[1] else {
            unreachable!()
        };
        assert!((*taille - 3.2 * 0.7).abs() < 1e-9);
        assert!((mm.1 + 0.45 * 3.2 - p.mm_y(0.0)).abs() < 1e-9);
    }

    #[test]
    fn expression_du_manuel() {
        // man. p. 42 : 4$ \blue f(x)=\frac{3x+1}{\sqrt{x^2+1}}
        let p = page();
        let code = "4$\\blue f(x)=\\frac{3x+1}{\\sqrt{x^2+1}}";
        let pieces = rendu(&p, 1.0, 2.0, code, [0, 0, 0]);
        // 16 blocs : f ( ) x ) = (5) + numérateur (4) + racine (√ + 4)
        // + vinculum + barre de fraction
        assert_eq!(pieces.len(), 16);
        // ancrage utilisateur (1 ; 2) et taille 4$, couleur \blue
        let Piece::Texte {
            txt, mm, taille, c, ..
        } = &pieces[0]
        else {
            unreachable!()
        };
        assert_eq!(txt, "f");
        assert!((mm.0 - p.mm_x(1.0)).abs() < 1e-9);
        assert!((mm.1 - p.mm_y(2.0)).abs() < 1e-9);
        assert_eq!(*taille, 4.0);
        assert_eq!(*c, [0, 0, 255]);
        // taille du √ = taille du texte standard (pas petit)
        let rac = pieces
            .iter()
            .filter(|q| {
                let Piece::Texte { txt, .. } = q else {
                    return false;
                };
                txt == "√"
            })
            .count();
        assert_eq!(rac, 1);
        // au moins 2 traits (barre frac + vinculum)
        let nlignes = pieces
            .iter()
            .filter(|q| matches!(q, Piece::Ligne { .. }))
            .count();
        assert_eq!(nlignes, 2);
    }
}
