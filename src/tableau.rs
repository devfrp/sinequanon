//! Tableaux de signes et de variations — l'équivalent du module
//! « Définir / Tableaux de signes et de variations » de l'original.
//!
//! Syntaxe du code (une ligne = une rangée), identique à l'original :
//! - cases délimitées par `[` et `]`, trois zones de cadrage séparées par
//!   deux `:` : `[gauche : centre : droite]` ;
//! - **entre** les cases : séparateur ou valeur posée sur la frontière :
//!   `|` (trait pleine hauteur de la rangée), `||` (double barre, valeur
//!   interdite), `|0` ou `0|` (zéro traversé par la barre), `0` (zéro nu),
//!   tout autre texte (ex. `]5[`) étant écrit centré sur la séparation ;
//! - en **fin** de ligne, un séparateur ou une valeur s'applique au bord
//!   droit du tableau ;
//! - **dans** une zone : `|` trait court (tige), `|0` zéro barré, `||`
//!   double barre, `/>` flèche ascendante (ou `/:>`), `\>` descendante,
//!   `_>` horizontale ; une case contenant `//` est hachurée ;
//! - macros LaTeX : `-\infty`, `+\infty`, `\infty`, `\sqrt{...}`,
//!   `\frac{...}{...}`, `\text{...}`, `\array{...\\...}` (étiquette sur
//!   plusieurs lignes), `\pi`, `\theta`, `\ln`, `\le`, `\ge`,
//!   `\left|`/`\right|`.

use serde::{Deserialize, Serialize};

/// Séparateur vertical à gauche d'une case (ou en fin de rangée).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Sep {
    /// pas de séparateur
    #[default]
    Rien,
    /// trait simple pleine hauteur
    Trait,
    /// double barre (valeur interdite)
    Double,
    /// zéro traversé par la barre
    Zero,
    /// zéro nu (sans barre)
    ZeroNu,
}

/// Une case : séparateur gauche, trois zones de texte, état hachuré,
/// et éventuelle valeur posée sur la séparation gauche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub sep: Sep,
    pub zones: [String; 3],
    pub hachee: bool,
    #[serde(default)]
    pub valeur_frontiere: Option<String>,
}

impl Default for Case {
    fn default() -> Self {
        Self {
            sep: Sep::Rien,
            zones: Default::default(),
            hachee: false,
            valeur_frontiere: None,
        }
    }
}

/// Une rangée du tableau.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Rangee {
    pub cases: Vec<Case>,
    /// séparateur en fin de rangée (bord droit du tableau)
    #[serde(default)]
    pub sep_fin: Sep,
    /// texte en fin de rangée (bord droit du tableau)
    #[serde(default)]
    pub valeur_fin: Option<String>,
}

/// Tableau complet (le code source est conservé pour l'édition).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tableau {
    pub code: String,
    pub rows: Vec<Rangee>,
    pub couleur_traits: [u8; 3],
    pub couleur_textes: [u8; 3],
    /// position du coin haut-gauche sur la page (mm)
    pub x_mm: f64,
    pub y_mm: f64,
    /// dessiner le tableau sur la page
    pub dessiner: bool,
}

impl Default for Tableau {
    fn default() -> Self {
        Self {
            code: String::new(),
            rows: Vec::new(),
            couleur_traits: [26, 26, 26],
            couleur_textes: [26, 26, 26],
            x_mm: 20.0,
            y_mm: 8.0,
            dessiner: false,
        }
    }
}

/// Direction d'une flèche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirFleche {
    Montante,
    Descendante,
    Horizontale,
}

/// Jeton d'une zone.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// fragment de texte (macros comprises)
    Txt(String),
    Fleche(DirFleche),
    /// trait court (tige)
    Tige,
    /// zéro traversé par une barre
    ZeroB,
    /// double barre
    Double,
}

/// Analyse une zone : renvoie les jetons dans l'ordre.
/// Accepte `/>` et `/:>` (montante), `\>` et `\:>` (descendante),
/// `_>` et `_:>` (horizontale), `|0`/`0|` (zéro barré), `||`, `|`.
pub fn parse_zone(s: &str) -> Vec<Tok> {
    let mut toks: Vec<Tok> = Vec::new();
    let mut txt = String::new();
    let ch: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        let tri = |a: char, b: char, c: char| -> bool {
            ch[i] == a && i + 2 < ch.len() && ch[i + 1] == b && ch[i + 2] == c
        };
        let deux = |a: char, b: char| -> bool { ch[i] == a && i + 1 < ch.len() && ch[i + 1] == b };
        // `\left|` et `\right|` : la barre fait partie du texte (valeur
        // absolue), elle ne doit pas devenir une tige de colonne
        if ch[i] == '|' && (txt.ends_with("\\left") || txt.ends_with("\\right")) {
            txt.push('|');
            i += 1;
        } else if tri('/', ':', '>') {
            flush_txt(&mut toks, &mut txt);
            toks.push(Tok::Fleche(DirFleche::Montante));
            i += 3;
        } else if deux('/', '>') {
            flush_txt(&mut toks, &mut txt);
            toks.push(Tok::Fleche(DirFleche::Montante));
            i += 2;
        } else if tri('\\', ':', '>') || deux('\\', '>') {
            flush_txt(&mut toks, &mut txt);
            toks.push(Tok::Fleche(DirFleche::Descendante));
            i += if tri('\\', ':', '>') { 3 } else { 2 };
        } else if tri('_', ':', '>') || deux('_', '>') {
            flush_txt(&mut toks, &mut txt);
            toks.push(Tok::Fleche(DirFleche::Horizontale));
            i += if tri('_', ':', '>') { 3 } else { 2 };
        } else if deux('|', '0') || deux('0', '|') {
            flush_txt(&mut toks, &mut txt);
            toks.push(Tok::ZeroB);
            i += 2;
        } else if deux('|', '|') {
            flush_txt(&mut toks, &mut txt);
            toks.push(Tok::Double);
            i += 2;
        } else if ch[i] == '|' {
            flush_txt(&mut toks, &mut txt);
            toks.push(Tok::Tige);
            i += 1;
        } else {
            txt.push(ch[i]);
            i += 1;
        }
    }
    flush_txt(&mut toks, &mut txt);
    toks
}

fn flush_txt(toks: &mut Vec<Tok>, txt: &mut String) {
    let t = txt.trim().to_string();
    txt.clear();
    if !t.is_empty() {
        toks.push(Tok::Txt(t));
    }
}

/// Classe le texte situé entre deux cases (ou en fin de ligne) :
/// séparateur connu, repli « barre simple » pour une suite de `|`/`0`,
/// ou valeur posée sur la frontière.
fn classify_sep(text: &str) -> (Sep, Option<String>) {
    let t = text.trim();
    match t {
        "" => (Sep::Rien, None),
        "|" => (Sep::Trait, None),
        "||" => (Sep::Double, None),
        "|0" | "0|" | "|0|" => (Sep::Zero, None),
        "0" => (Sep::ZeroNu, None),
        _ if t
            .chars()
            .all(|c| c == '|' || c == '0' || c.is_ascii_whitespace()) =>
        {
            (Sep::Trait, None)
        }
        _ => (Sep::Rien, Some(t.to_string())),
    }
}

/// Découpe le contenu d'une case en (au plus) 3 zones, en consommant
/// les jetons à deux-points `/:>`, `\:>`, `_:>` sans les couper.
fn split_fields(s: &str) -> Result<Vec<String>, String> {
    let ch: Vec<char> = s.chars().collect();
    let mut fields = vec![String::new()];
    let mut i = 0;
    while i < ch.len() {
        let tri = |a: char, b: char, c: char| {
            ch[i] == a && i + 2 < ch.len() && ch[i + 1] == b && ch[i + 2] == c
        };
        if tri('/', ':', '>') {
            fields.last_mut().unwrap().push_str("/:>");
            i += 3;
        } else if tri('\\', ':', '>') {
            fields.last_mut().unwrap().push_str("\\:>");
            i += 3;
        } else if tri('_', ':', '>') {
            fields.last_mut().unwrap().push_str("_:>");
            i += 3;
        } else if ch[i] == ':' {
            fields.push(String::new());
            i += 1;
        } else {
            fields.last_mut().unwrap().push(ch[i]);
            i += 1;
        }
    }
    // tolérance : deux-points superflus en tête/fin de case
    while fields.len() > 3 {
        if fields[0].trim().is_empty() {
            fields.remove(0);
        } else if fields[fields.len() - 1].trim().is_empty() {
            fields.pop();
        } else {
            return Err("plus de deux « : » utiles dans une case".into());
        }
    }
    Ok(fields)
}

/// Analyse le contenu d'une case, avec le texte la précédant.
fn parse_case(contenu: &str, entre: &str) -> Result<Case, String> {
    let (sep, vf) = classify_sep(entre);
    let mut case = Case {
        sep,
        valeur_frontiere: vf,
        ..Default::default()
    };
    if contenu.contains("//") {
        case.hachee = true;
        return Ok(case);
    }
    let mut fields = split_fields(contenu)?;
    while fields.len() < 3 {
        fields.push(String::new());
    }
    case.zones = [
        fields[0].trim().to_string(),
        fields[1].trim().to_string(),
        fields[2].trim().to_string(),
    ];
    Ok(case)
}

/// Analyse le code complet.
pub fn parse(code: &str) -> Result<Tableau, String> {
    let mut rows: Vec<Rangee> = Vec::new();
    for (il, ligne) in code.lines().enumerate() {
        let ligne = ligne.trim();
        if ligne.is_empty() {
            continue;
        }
        let r = parse_ligne(ligne).map_err(|e| format!("ligne {} : {e}", il + 1))?;
        rows.push(r);
    }
    // toutes les rangées doivent avoir le même nombre de cases
    if let Some(n) = rows.first().map(|r| r.cases.len()) {
        for (i, r) in rows.iter().enumerate() {
            if r.cases.len() != n {
                return Err(format!(
                    "ligne {} : {} cases au lieu de {n} (boutons +/− colonnes)",
                    i + 1,
                    r.cases.len()
                ));
            }
        }
    }
    Ok(Tableau {
        code: code.to_string(),
        rows,
        ..Default::default()
    })
}

/// Analyse une ligne : cases `[...]`, séparateurs et valeurs entre elles,
/// séparateur/valeur de fin de ligne (bord droit).
fn parse_ligne(ligne: &str) -> Result<Rangee, String> {
    let ch: Vec<char> = ligne.chars().collect();
    let mut i = 0;
    while i < ch.len() && ch[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= ch.len() {
        return Ok(Rangee::default());
    }
    if ch[i] != '[' {
        return Err("texte avant la première case".into());
    }
    let mut cases: Vec<Case> = Vec::new();
    let mut pending = String::new();
    let mut fin: Option<(Sep, Option<String>)> = None;
    while i < ch.len() {
        match ch[i] {
            '[' => {
                let fin_case = (i + 1..ch.len()).find(|&k| ch[k] == ']');
                let Some(fin_case) = fin_case else {
                    return Err("`[` sans `]`".into());
                };
                let contenu: String = ch[i + 1..fin_case].iter().collect();
                cases.push(parse_case(&contenu, &pending)?);
                pending.clear();
                i = fin_case + 1;
            }
            ']' => {
                // séparateur / valeur de bord droit (un `]` final est toléré)
                let mut reste: String =
                    pending.chars().chain(ch[i + 1..].iter().copied()).collect();
                pending.clear();
                if reste.trim_end().ends_with(']') {
                    let tr = reste.trim_end().to_string();
                    reste = tr[..tr.len() - 1].to_string();
                }
                if reste.contains('[') || reste.contains(']') {
                    return Err("crochets en fin de ligne".into());
                }
                fin = Some(classify_sep(&reste));
                i = ch.len();
            }
            c => {
                pending.push(c);
                i += 1;
            }
        }
    }
    let (sep_fin, valeur_fin) = fin.unwrap_or_else(|| classify_sep(&pending));
    Ok(Rangee {
        cases,
        sep_fin,
        valeur_fin,
    })
}

// --- resérialisation (boutons Égaliser / + colonne / − colonne) ---------------

/// Texte de code reproduisant un séparateur (se choisit pour se re-analyser
/// à l'identique).
fn sep_vers_txt(sep: Sep) -> &'static str {
    match sep {
        Sep::Rien => "",
        Sep::Trait => "|",
        Sep::Double => "||",
        Sep::Zero => "|0",
        Sep::ZeroNu => "0",
    }
}

/// Resérialise une rangée en ligne de code canonique.
pub fn rangee_vers_code(r: &Rangee) -> String {
    let mut s = String::new();
    for (i, c) in r.cases.iter().enumerate() {
        if i > 0 {
            s.push_str(sep_vers_txt(c.sep));
            if let Some(v) = &c.valeur_frontiere {
                s.push_str(v);
            }
        }
        s.push('[');
        if c.hachee {
            s.push_str("//://://");
        } else {
            s.push_str(&c.zones.join(":"));
        }
        s.push(']');
    }
    s.push_str(sep_vers_txt(r.sep_fin));
    if let Some(v) = &r.valeur_fin {
        s.push_str(v);
    }
    s
}

/// Réécrit le code en canonique avec le même nombre de cases sur toutes les
/// lignes : les lignes courtes reçoivent des cases vides à droite
/// (bouton « Égaliser » du module original).
pub fn egaliser(code: &str) -> Result<String, String> {
    ajuster_colonnes(code, 0)
}

/// Ajoute (`delta` > 0) ou retire (`delta` < 0) une colonne à toutes les
/// rangées, puis réécrit le code en canonique (boutons « +/− colonne »).
pub fn ajuster_colonnes(code: &str, delta: i64) -> Result<String, String> {
    let mut rows: Vec<Rangee> = Vec::new();
    for ligne in code.lines() {
        if ligne.trim().is_empty() {
            continue;
        }
        rows.push(parse_ligne(ligne)?);
    }
    let mut n = rows.iter().map(|r| r.cases.len()).max().unwrap_or(0);
    n = (n as i64 + delta).max(1) as usize;
    for r in &mut rows {
        while r.cases.len() < n {
            r.cases.push(Case::default());
        }
        r.cases.truncate(n);
    }
    Ok(rows
        .iter()
        .map(rangee_vers_code)
        .collect::<Vec<_>>()
        .join("\n"))
}

// --- mise en page ------------------------------------------------------------

/// Hauteurs de rangée (mm) : haute si la rangée porte des flèches,
/// une fraction ou un bloc `\array`.
const H_RANGEE: f64 = 11.0;
const H_HAUT: f64 = 16.0;
/// Largeur minimale de la colonne des étiquettes (mm).
const LARGEUR_ETIQ: f64 = 22.0;
/// Marges horizontales dans une case (mm).
const MARGE: f64 = 3.0;
/// Corps du texte (mm).
const TAILLE: f64 = 4.2;
/// Écart centre-à-centre des deux traits d'une double barre (mm).
const GAP_DOUBLE: f64 = 0.75;
/// Largeur d'un trait (mm).
pub const W_TRAIT: f64 = 0.35;

/// Remplace les macros à glyphes simples (infinis, lettres grecques…).
fn remplace_glyphes(s: &str) -> String {
    // du plus long au plus court : `\le` ne doit pas manger `\left|`
    s.replace("-\\infty", "−∞")
        .replace("+\\infty", "+∞")
        .replace("\\infty", "∞")
        .replace("\\left|", "|")
        .replace("\\right|", "|")
        .replace("\\left(", "(")
        .replace("\\right)", ")")
        .replace("\\pi", "π")
        .replace("\\theta", "θ")
        .replace("\\ln", "ln")
        .replace("\\le", "≤")
        .replace("\\ge", "≥")
}

/// Largeur d'un fragment sans macro structurante (mm), après remplacement
/// des glyphes (cohérent avec `rend_simple`).
fn largeur_simple(s: &str, size: f64) -> f64 {
    let s = remplace_glyphes(s);
    size * 0.62 * s.chars().count() as f64
}

/// Largeur approximative d'un fragment de texte rendu (mm).
pub fn largeur_txt(s: &str, size: f64) -> f64 {
    let mut w = 0.0f64;
    let mut rest = s;
    while let Some(p) = trouve_macro(rest) {
        let (avant, mac, apres) = p;
        w += largeur_simple(avant, size);
        w += largeur_macro(&mac, size);
        rest = apres;
    }
    w += largeur_simple(rest, size);
    w.max(size * 0.6)
}

/// Cherche la première macro `nom{...}` ; renvoie (avant, (nom, contenu), après).
fn trouve_macro(s: &str) -> Option<(&str, (&'static str, String), &str)> {
    // la macro la plus à GAUCHE dans la chaîne (et non la première de la
    // liste) : `\frac{\sqrt{2}}{2}` doit rendre le \frac, pas le \sqrt
    let mut best: Option<(usize, &'static str)> = None;
    for nom in ["\\array{", "\\sqrt{", "\\frac{", "\\text{"] {
        if let Some(i) = s.find(nom) {
            best = match best {
                Some((bi, bn)) if bi <= i => Some((bi, bn)),
                _ => Some((i, nom)),
            };
        }
    }
    let (i, nom) = best?;
    // deb = position de l'accolade ouvrante de la macro
    let (contenu, apres) = groupe_equilibre(s, i + nom.len() - 1)?;
    // \frac{a}{b} : contenu = les deux groupes accoladés (avec accolades),
    // comme le attend `split_frac`
    let (nom_ok, contenu_ok, apres_ok) = if nom == "\\frac{" {
        match groupe_equilibre(apres, 0) {
            Some((g2, ap2)) => ("\\frac", format!("{{{contenu}}}{{{g2}}}"), ap2),
            None => ("\\frac", format!("{{{contenu}}}"), apres),
        }
    } else {
        (nom.trim_end_matches('{'), contenu.to_string(), apres)
    };
    Some((&s[..i], (nom_ok, contenu_ok), apres_ok))
}

/// Contenu du groupe `{...}` commençant à `deb` (accolades équilibrées),
/// avec le texte qui suit l'accolade fermante. Le contenu EXCLUT les
/// accolades.
fn groupe_equilibre(s: &str, deb: usize) -> Option<(String, &str)> {
    let bytes = s.as_bytes();
    let mut prof = 0usize;
    for (k, &b) in bytes[deb..].iter().enumerate() {
        match b {
            b'{' => prof += 1,
            b'}' => {
                prof = prof.checked_sub(1)?;
                if prof == 0 {
                    return Some((s[deb + 1..deb + k].to_string(), &s[deb + k + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// Coupe le contenu d'un `\array{...}` au niveau haut sur `\\`.
fn lignes_array(s: &str) -> Vec<String> {
    let ch: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prof = 0usize;
    let mut i = 0;
    while i < ch.len() {
        if ch[i] == '{' {
            prof += 1;
            cur.push(ch[i]);
        } else if ch[i] == '}' {
            prof = prof.saturating_sub(1);
            cur.push(ch[i]);
        } else if prof == 0 && ch[i] == '\\' && i + 1 < ch.len() && ch[i + 1] == '\\' {
            out.push(cur.trim().to_string());
            cur.clear();
            i += 2;
            continue;
        } else {
            cur.push(ch[i]);
        }
        i += 1;
    }
    out.push(cur.trim().to_string());
    out
}

fn largeur_macro(mac: &(&'static str, String), size: f64) -> f64 {
    match mac.0 {
        "\\sqrt" => largeur_txt(&mac.1, size * 0.8) + size * 0.8,
        "\\frac" => {
            let (a, b) = split_frac(&mac.1);
            largeur_txt(&a, size * 0.75).max(largeur_txt(&b, size * 0.75))
        }
        "\\array" => lignes_array(&mac.1)
            .iter()
            .map(|l| largeur_txt(l, size))
            .fold(0.0f64, f64::max),
        _ => largeur_simple(&mac.1, size),
    }
}

/// Sépare numérateur et dénominateur d'un `\frac{a}{b}`.
fn split_frac(s: &str) -> (String, String) {
    if let Some((num, reste)) = groupe_equilibre(s, 0) {
        if let Some(i) = reste.find('{') {
            if let Some((den, _)) = groupe_equilibre(reste, i) {
                return (num, den);
            }
        }
        return (num, String::new());
    }
    (s.to_string(), String::new())
}

/// Largeur naturelle d'une zone (mm).
fn largeur_zone(zone: &str, size: f64) -> f64 {
    parse_zone(zone)
        .iter()
        .map(|t| match t {
            Tok::Txt(s) => largeur_txt(s, size),
            Tok::Fleche(_) => size * 3.2,
            Tok::Tige | Tok::ZeroB | Tok::Double => 0.9,
        })
        .sum()
}

/// Rangée « haute » ? (flèches, fraction ou étiquette multiligne)
fn rangee_haute(r: &Rangee) -> bool {
    r.cases.iter().any(|c| {
        c.zones.iter().any(|z| {
            z.contains("\\frac")
                || z.contains("\\array")
                || parse_zone(z).iter().any(|t| matches!(t, Tok::Fleche(_)))
        })
    })
}

/// Largeurs de colonnes (mm).
pub fn colonnes(t: &Tableau) -> Vec<f64> {
    if t.rows.is_empty() {
        return Vec::new();
    }
    let ncols = t.rows[0].cases.len();
    let mut widths = vec![LARGEUR_ETIQ; ncols];
    // étiquettes
    let mut letiq = LARGEUR_ETIQ;
    for r in &t.rows {
        if let Some(c) = r.cases.first() {
            if !c.hachee {
                let w: f64 = c
                    .zones
                    .iter()
                    .map(|z| largeur_zone(z, TAILLE))
                    .fold(0.0f64, f64::max)
                    + 2.0 * MARGE;
                letiq = letiq.max(w);
            }
        }
    }
    widths[0] = letiq;
    // colonnes de contenu
    for r in &t.rows {
        for (i, c) in r.cases.iter().enumerate().skip(1) {
            if c.hachee {
                widths[i] = widths[i].max(14.0);
                continue;
            }
            // une flèche seule dans la case s'étire : sa largeur propre
            // ne fixe pas la colonne
            let seule = fleche_seule_case(c).is_some();
            let lz = if seule {
                0.0
            } else {
                largeur_zone(&c.zones[0], TAILLE)
            };
            let rz = if seule {
                0.0
            } else {
                largeur_zone(&c.zones[2], TAILLE)
            };
            let centre_fleche = est_fleche_seule(&c.zones[1]);
            let cz = if centre_fleche {
                8.0
            } else {
                largeur_zone(&c.zones[1], TAILLE)
            };
            let w = lz + cz + rz + 2.0 * MARGE + 1.0;
            widths[i] = widths[i].max(w);
        }
    }
    // valeurs posées sur les frontières : moitié de chaque côté
    for r in &t.rows {
        for (i, c) in r.cases.iter().enumerate() {
            if let Some(v) = &c.valeur_frontiere {
                let w = largeur_txt(v, TAILLE) / 2.0 + 2.0;
                if i > 0 {
                    widths[i - 1] = widths[i - 1].max(w);
                }
                if i < ncols {
                    widths[i] = widths[i].max(w);
                }
            }
        }
        if let Some(v) = &r.valeur_fin {
            let w = largeur_txt(v, TAILLE) / 2.0 + 2.0;
            let n = widths.len();
            widths[n - 1] = widths[n - 1].max(w);
        }
    }
    widths
}

/// La zone ne contient-elle qu'une flèche (qui peut s'étirer) ?
fn est_fleche_seule(zone: &str) -> bool {
    matches!(parse_zone(zone).as_slice(), [Tok::Fleche(_)])
}

/// Dimensions du tableau (mm).
pub fn dimensions(t: &Tableau) -> (f64, f64) {
    if t.rows.is_empty() {
        return (0.0, 0.0);
    }
    let w: f64 = colonnes(t).iter().sum();
    let h: f64 = t
        .rows
        .iter()
        .map(|r| if rangee_haute(r) { H_HAUT } else { H_RANGEE })
        .sum();
    (w.max(30.0), h)
}

/// Primitive de dessin (mm, origine = coin haut-gauche du tableau).
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    /// texte : (x, y) = coin bas-gauche (ligne de base), taille en mm
    Texte {
        x: f64,
        y: f64,
        size: f64,
        txt: String,
        italique: bool,
    },
    /// segment
    Ligne {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        w: f64,
    },
}

/// Récolte les primitives par catégorie (les textes sont dessinés en
/// dernier pour passer au-dessus des barres, comme dans l'original).
struct Sink {
    lignes: Vec<Piece>,
    textes: Vec<Piece>,
}

impl Sink {
    fn ligne(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, w: f64) {
        self.lignes.push(Piece::Ligne { x0, y0, x1, y1, w });
    }
    fn texte(&mut self, x: f64, y: f64, size: f64, txt: String, italique: bool) {
        self.textes.push(Piece::Texte {
            x,
            y,
            size,
            txt,
            italique,
        });
    }
    fn fleche(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        // segment + pointe
        self.ligne(x0, y0, x1, y1, 0.5);
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (px, py) = (-uy, ux);
        let (bx, by) = (x1 - 2.2 * ux, y1 - 2.2 * uy);
        self.ligne(x1, y1, bx + 0.85 * px, by + 0.85 * py, 0.4);
        self.ligne(x1, y1, bx - 0.85 * px, by - 0.85 * py, 0.4);
    }
    fn hachures(&mut self, x: f64, y: f64, w: f64, h: f64) {
        // diagonales à 45°, pas 2.2 mm
        let pas = 2.2;
        let mut d = pas;
        while d < w + h {
            // segment x+y = d dans le rectangle, découpé
            let (mut sx, mut sy) = (0.0f64, d);
            if sy > h {
                sy = h;
                sx = d - h;
            }
            let (mut ex, mut ey) = (d, 0.0f64);
            if ex > w {
                ex = w;
                ey = d - w;
            }
            if sx <= w && ex >= 0.0 {
                self.ligne(x + sx, y + sy, x + ex, y + ey, 0.3);
            }
            d += pas;
        }
    }
}

/// Génère toutes les primitives de dessin du tableau.
pub fn pieces(t: &Tableau) -> Vec<Piece> {
    let mut s = Sink {
        lignes: Vec::new(),
        textes: Vec::new(),
    };
    if t.rows.is_empty() {
        return Vec::new();
    }
    let widths = colonnes(t);
    let tw: f64 = widths.iter().sum();
    let mut ys = vec![0.0f64];
    for r in &t.rows {
        let last = *ys.last().unwrap();
        ys.push(last + if rangee_haute(r) { H_HAUT } else { H_RANGEE });
    }
    let h_tot = *ys.last().unwrap();

    // lignes horizontales + bords gauche/droit
    for &y in &ys {
        s.ligne(0.0, y, tw, y, W_TRAIT);
    }
    s.ligne(0.0, 0.0, 0.0, h_tot, W_TRAIT);
    s.ligne(tw, 0.0, tw, h_tot, W_TRAIT);

    for (j, r) in t.rows.iter().enumerate() {
        let (yt, yb) = (ys[j], ys[j + 1]);
        let h = yb - yt;
        let mut x = 0.0;
        for (i, c) in r.cases.iter().enumerate() {
            // barre à gauche de la case
            match c.sep {
                Sep::Trait => s.ligne(x, yt, x, yb, W_TRAIT),
                Sep::Double => {
                    s.ligne(x - GAP_DOUBLE / 2.0, yt, x - GAP_DOUBLE / 2.0, yb, W_TRAIT);
                    s.ligne(x + GAP_DOUBLE / 2.0, yt, x + GAP_DOUBLE / 2.0, yb, W_TRAIT);
                }
                Sep::Zero => {
                    s.ligne(x, yt, x, yb, W_TRAIT);
                    let w0 = largeur_txt("0", TAILLE);
                    s.texte(
                        x - w0 / 2.0,
                        (yt + yb) / 2.0 + 0.35 * TAILLE,
                        TAILLE,
                        "0".into(),
                        false,
                    );
                }
                Sep::ZeroNu => {
                    let w0 = largeur_txt("0", TAILLE);
                    s.texte(
                        x - w0 / 2.0,
                        (yt + yb) / 2.0 + 0.35 * TAILLE,
                        TAILLE,
                        "0".into(),
                        false,
                    );
                }
                Sep::Rien => {}
            }
            // valeur posée sur la frontière (macros comprises)
            if let Some(v) = &c.valeur_frontiere {
                let wv = largeur_txt(v, TAILLE);
                rend_txt(
                    &mut s,
                    v,
                    x - wv / 2.0,
                    (yt + yb) / 2.0 + 0.35 * TAILLE,
                    TAILLE,
                );
            }
            // contenu
            let cw = widths[i];
            if c.hachee {
                s.hachures(x + 0.4, yt + 0.4, cw - 0.8, h - 0.8);
            } else {
                // demi-largeurs des valeurs bordant la case (gauche/droite)
                let vg = demi_frontiere(c.sep, &c.valeur_frontiere);
                let vd = if i + 1 < r.cases.len() {
                    let n = &r.cases[i + 1];
                    demi_frontiere(n.sep, &n.valeur_frontiere)
                } else {
                    match (r.sep_fin, &r.valeur_fin) {
                        (_, Some(v)) => largeur_txt(v, TAILLE) / 2.0 + 1.5,
                        (Sep::Zero | Sep::ZeroNu, None) => largeur_txt("0", TAILLE) / 2.0 + 1.5,
                        _ => 0.0,
                    }
                };
                dessine_case(&mut s, c, x, cw, yt, yb, vg, vd);
            }
            x += cw;
        }
        // fin de rangée (bord droit)
        match r.sep_fin {
            Sep::Trait => s.ligne(tw, yt, tw, yb, W_TRAIT),
            Sep::Double => s.ligne(tw - GAP_DOUBLE, yt, tw - GAP_DOUBLE, yb, W_TRAIT),
            Sep::Zero => {
                let w0 = largeur_txt("0", TAILLE);
                s.texte(
                    tw - 1.8 - w0 / 2.0,
                    (yt + yb) / 2.0 + 0.35 * TAILLE,
                    TAILLE,
                    "0".into(),
                    false,
                );
            }
            Sep::ZeroNu => {
                let w0 = largeur_txt("0", TAILLE);
                s.texte(
                    tw - 1.8 - w0 / 2.0,
                    (yt + yb) / 2.0 + 0.35 * TAILLE,
                    TAILLE,
                    "0".into(),
                    false,
                );
            }
            Sep::Rien => {}
        }
        if let Some(v) = &r.valeur_fin {
            let wv = largeur_txt(v, TAILLE);
            rend_txt(
                &mut s,
                v,
                tw - 1.5 - wv / 2.0,
                (yt + yb) / 2.0 + 0.35 * TAILLE,
                TAILLE,
            );
        }
    }

    let mut out = s.lignes;
    out.extend(s.textes);
    out
}

/// Dessine le contenu d'une case (3 zones).
/// La case ne contient-elle qu'une seule flèche (toutes zones confondues) ?
/// Dans ce cas elle s'étire sur toute la largeur de la case, comme dans
/// l'original (cas `[/:>:]` de Var4).
fn fleche_seule_case(c: &Case) -> Option<DirFleche> {
    let mut dir = None;
    let mut n = 0;
    for z in &c.zones {
        for t in parse_zone(z) {
            match t {
                Tok::Fleche(d) => {
                    n += 1;
                    dir = Some(d);
                }
                _ => return None,
            }
        }
    }
    if n == 1 {
        dir
    } else {
        None
    }
}

/// Demi-largeur de ce qui est posé sur une frontière (valeur ou zéro),
/// pour raccourcir les flèches étirées qui la bordent.
fn demi_frontiere(sep: Sep, vf: &Option<String>) -> f64 {
    if let Some(v) = vf {
        largeur_txt(v, TAILLE) / 2.0
    } else {
        match sep {
            Sep::Zero | Sep::ZeroNu => largeur_txt("0", TAILLE) / 2.0,
            _ => 0.0,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn dessine_case(s: &mut Sink, c: &Case, x: f64, cw: f64, yt: f64, yb: f64, vg: f64, vd: f64) {
    // flèche seule dans la case : étirée, mais sans chevaucher les valeurs
    // posées sur les frontières gauche et droite
    if let Some(dir) = fleche_seule_case(c) {
        // garde de 3 mm devant une valeur de frontière
        let xa = if vg > 0.0 { x + vg + 3.0 } else { x + 1.0 };
        let xb = if vd > 0.0 {
            (x + cw - vd - 3.0).max(xa + 2.0)
        } else {
            (x + cw - 1.0).max(xa + 2.0)
        };
        dessine_fleche(s, Tok::Fleche(dir), xa, xb, yt, yb);
        return;
    }
    let lz = largeur_zone(&c.zones[0], TAILLE);
    let rz = largeur_zone(&c.zones[2], TAILLE);
    let xg = x + MARGE; // début du contenu gauche
    let xfin = x + cw - MARGE; // fin du contenu droit
    let xd = xfin - rz; // début du contenu droit
    let (cx0, cx1) = (xg + lz, xd); // domaine central
    dessine_tokens(s, &c.zones[0], xg, xg + lz, Align::Gauche, yt, yb);
    dessine_tokens(
        s,
        &c.zones[1],
        cx0 + 0.5,
        (cx1 - 0.5).max(cx0 + 1.0),
        Align::Centre,
        yt,
        yb,
    );
    dessine_tokens(s, &c.zones[2], xd, xfin, Align::Droite, yt, yb);
}

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Gauche,
    Centre,
    Droite,
}

/// Dessine les jetons d'une zone dans le domaine horizontal [xa, xb].
fn dessine_tokens(s: &mut Sink, zone: &str, xa: f64, xb: f64, mode: Align, yt: f64, yb: f64) {
    let toks = parse_zone(zone);
    if toks.is_empty() || xb <= xa {
        return;
    }
    let ws: Vec<f64> = toks
        .iter()
        .map(|t| match t {
            Tok::Txt(txt) => largeur_txt(txt, TAILLE),
            Tok::Fleche(_) => TAILLE * 3.2,
            Tok::Tige | Tok::ZeroB | Tok::Double => 0.9,
        })
        .collect();
    // une flèche seule au centre s'étire sur tout le domaine
    if mode == Align::Centre && toks.len() == 1 && matches!(toks[0], Tok::Fleche(_)) {
        dessine_fleche(s, toks[0].clone(), xa, xb, yt, yb);
        return;
    }
    let tot: f64 = ws.iter().sum();
    let mut cx = match mode {
        Align::Gauche => xa,
        Align::Droite => xa - tot + (xb - xa).max(tot),
        Align::Centre => xa + ((xb - xa) - tot).max(0.0) / 2.0,
    };
    let yc = (yt + yb) / 2.0;
    for (t, w) in toks.iter().zip(&ws) {
        match t {
            Tok::Txt(txt) => {
                rend_txt(s, txt, cx, yc + 0.35 * TAILLE, TAILLE);
                cx += w;
            }
            Tok::Fleche(_) => {
                dessine_fleche(s, t.clone(), cx, cx + w, yt, yb);
                cx += w;
            }
            Tok::Tige => {
                s.ligne(
                    cx + w / 2.0,
                    yt + 0.22 * (yb - yt),
                    cx + w / 2.0,
                    yb - 0.22 * (yb - yt),
                    W_TRAIT,
                );
                cx += w;
            }
            Tok::ZeroB => {
                s.ligne(cx + w / 2.0, yt, cx + w / 2.0, yb, W_TRAIT);
                let w0 = largeur_txt("0", TAILLE);
                s.texte(
                    cx + w / 2.0 - w0 / 2.0,
                    yc + 0.35 * TAILLE,
                    TAILLE,
                    "0".into(),
                    false,
                );
                cx += w;
            }
            Tok::Double => {
                s.ligne(
                    cx + w / 2.0 - GAP_DOUBLE / 2.0,
                    yt,
                    cx + w / 2.0 - GAP_DOUBLE / 2.0,
                    yb,
                    W_TRAIT,
                );
                s.ligne(
                    cx + w / 2.0 + GAP_DOUBLE / 2.0,
                    yt,
                    cx + w / 2.0 + GAP_DOUBLE / 2.0,
                    yb,
                    W_TRAIT,
                );
                cx += w;
            }
        }
    }
}

/// Trace une flèche dans le domaine horizontal [xa, xb].
fn dessine_fleche(s: &mut Sink, dir: Tok, xa: f64, xb: f64, yt: f64, yb: f64) {
    let yc = (yt + yb) / 2.0;
    let dy = (yb - yt) * 0.28;
    match dir {
        Tok::Fleche(DirFleche::Montante) => s.fleche(xa, yc + dy, xb, yc - dy),
        Tok::Fleche(DirFleche::Descendante) => s.fleche(xa, yc - dy, xb, yc + dy),
        Tok::Fleche(DirFleche::Horizontale) => s.fleche(xa, yc, xb, yc),
        _ => {}
    }
}

/// Rend un fragment de texte (avec macros) à (x, y) = ligne de base ;
/// renvoie l'abscisse de fin.
fn rend_txt(s: &mut Sink, txt: &str, x: f64, y: f64, size: f64) -> f64 {
    let mut cx = x;
    let mut rest = txt;
    while let Some((avant, mac, apres)) = trouve_macro(rest) {
        cx = rend_simple(s, avant, cx, y, size, true);
        match mac.0 {
            "\\sqrt" => {
                let w = largeur_txt(&mac.1, size * 0.8);
                s.texte(cx, y, size * 0.9, "√".into(), false);
                s.ligne(
                    cx + size * 0.55,
                    y - size * 0.72,
                    cx + size * 0.55 + w + size * 0.25,
                    y - size * 0.72,
                    0.3,
                );
                rend_txt(s, &mac.1, cx + size * 0.6, y, size * 0.8);
                cx += w + size * 0.8;
            }
            "\\frac" => {
                let (a, b) = split_frac(&mac.1);
                let wa = largeur_txt(&a, size * 0.75);
                let wb = largeur_txt(&b, size * 0.75);
                let wm = wa.max(wb);
                rend_txt(s, &a, cx + (wm - wa) / 2.0, y - size * 0.45, size * 0.75);
                s.ligne(cx, y - size * 0.22, cx + wm, y - size * 0.22, 0.3);
                rend_txt(s, &b, cx + (wm - wb) / 2.0, y + size * 0.62, size * 0.75);
                cx += wm + size * 0.25;
            }
            "\\array" => {
                let lignes = lignes_array(&mac.1);
                let n = lignes.len() as f64;
                let wmax = lignes
                    .iter()
                    .map(|l| largeur_txt(l, size))
                    .fold(0.0f64, f64::max);
                for (k, l) in lignes.iter().enumerate() {
                    let wl = largeur_txt(l, size);
                    // bloc centré sur y (baseline mono-ligne) : la 1re ligne
                    // (k = 0) au-dessus ; l'axe y pointe vers le bas
                    let yk = y + (k as f64 - (n - 1.0) / 2.0) * 1.1 * size;
                    rend_txt(s, l, cx + (wmax - wl) / 2.0, yk, size);
                }
                cx += wmax;
            }
            "\\text" => {
                cx = rend_simple(s, &mac.1, cx, y, size, false);
            }
            _ => {
                cx = rend_simple(s, &mac.1, cx, y, size, true);
            }
        }
        rest = apres;
    }
    rend_simple(s, rest, cx, y, size, true)
}

/// Rend du texte simple (sans macro structurante) ; renvoie la nouvelle abscisse.
fn rend_simple(s: &mut Sink, txt: &str, x: f64, y: f64, size: f64, italique: bool) -> f64 {
    let t = remplace_glyphes(txt);
    let t = t.trim();
    if t.is_empty() {
        return x;
    }
    s.texte(x, y, size, t.to_string(), italique);
    x + largeur_simple(t, size)
}

// --- exemples de base (les 10 de l'original) ---------------------------------

/// Codes des 10 exemples : Var1..Var8, Sgn, Abs.
pub fn exemples() -> [(&'static str, &'static str); 10] {
    [
        ("Var1", EX_VAR1),
        ("Var2", EX_VAR2),
        ("Var3", EX_VAR3),
        ("Var4", EX_VAR4),
        ("Var5", EX_VAR5),
        ("Var6", EX_VAR6),
        ("Var7", EX_VAR7),
        ("Var8", EX_VAR8),
        ("Sgn", EX_SGN),
        ("Abs", EX_ABS),
    ]
}

/// Var1 — recopié de l'original (Manuel p. 85).
pub const EX_VAR1: &str = r#"
[:x     :]||[-\infty:    :]5[:    :+\infty]
[:f(x) :]||[          :/>:]2[:\>:          ]
"#;

/// Var2 — variations sur plusieurs valeurs, étiquette multiligne.
pub const EX_VAR2: &str = r#"
[:x                                       :]||[-1:    :]1[:    :]2[:    :4]
[:\array{\text{Variations}\\\text{de }f(x)}:]||[0  :/:>:  ][  :/:>:  ]4[\>  :  :-3]
"#;

/// Var3 — recopié de l'original.
pub const EX_VAR3: &str = r#"
[:x      :]||[-\infty:    :]-3[:    :+\infty]
[:f '(x) :]||[          :-  :]|0[:+  :          ]
[:f(x)   :]||[+\infty:\>:]2  [/:>:+\infty]
"#;

/// Var4 — recopié de l'original (double barre au bord droit).
pub const EX_VAR4: &str = r#"
[:x      :]||[-\infty:    :]-1[:    :]0                    [:    :1          ]
[:f '(x) :]||[          :-  :]|0[:+  :]|0                  [:+  :]||        ]
[:f(x)   :]||[1          :\>:]0  [/:>:]\frac{1}{2}[/:>:+\infty||]
"#;

/// Var5 — recopié de l'original (double barre d'étiquette, valeur \frac).
pub const EX_VAR5: &str = r#"
[:x      :]|[0        :   :]\frac{\sqrt{2}}{2}       [:   :+\infty]
[:g '(x):]||[          :-  :]|0                       [:+ :         ]
[:g(x)  :]||[+\infty:\>:]\frac{3+\text{ln2}}{2}[:/:>:+\infty]
"#;

/// Var6 — trigonométrique, valeurs en fractions de π.
pub const EX_VAR6: &str = r#"
[:x      :]||[0 :    :  ]\frac{\pi}{6}[:    :  ]\frac{5\pi}{6}[:    :  ]\frac{3\pi}{2}[:    :  ]2\pi[:    :    ]
[:f '(x) :]||[  :+  :  ]|0[:-  :  ]|0[:+  :  ]|0[:-  :  ]|0[:+  :     ]
[:f(x)   :]||[2 :/:>:  ]\frac{3\sqrt{3}}{2}[\>  :  :]0[/:>:  ]\frac{3\sqrt{3}}{2}[\>  :  :]2[:    :    ]
"#;

/// Var7 — valeur interdite (case hachurée).
pub const EX_VAR7: &str = r#"
[:x      :]||[-2  :    :  ]-1[//://:][:    :2]
[:f '(x) :]||[      :-  :]|0[//://:][  :+  :  ]
[:f(x)   :]||[\sqrt{3}  :\>:]0[//://:][-\infty:/:>:\sqrt{3}]
"#;

/// Var8 — courbe paramétrique.
pub const EX_VAR8: &str = r#"
[:t      :]||[-\infty:    :]-2[:    :]0[:    :]2[:    :+\infty]
[:x(t)   :]||[    :+    :]|0[:-  :  ]|0[:+  :  ]|0[:-  :      ]
[:y(t)   :]||[0  :/:>:  ]\frac{1}{2}[\>  :  :]0[/:>:  ]\frac{1}{2}[\>  :  :0]
"#;

/// Sgn — tableau de signes, recopié de l'original.
pub const EX_SGN: &str = r#"
[:x             :]|[-\infty:  :]-2[:  :]1  [:  :+\infty]
[:x-1           :]|[          :-:]|   [:-:]0|[:+:          ]
[:x+2           :]|[          :-:]0|[:+:]|   [:+:          ]
[:\frac{x-1}{x+2}:]|[          :+:]|| [:-:]0|[:+:          ]
"#;

/// Abs — valeurs absolues, recopié de l'original.
pub const EX_ABS: &str = r#"
[:x              :]|[-\infty:       :]-4[:       :]\frac{5}{2}[:       :+\infty]
[:2x-5           :]|[         :-   :]|  |[:-   :]0|              [:+   :          ]
[:\left|2x-5\right|:]|[         :5-2x:]|  |[:5-2x:]|              [:2x-5:          ]
[:x+4            :]|[         :-   :]|0|[:+    :]||                |[:+    :          ]
[:\left|x+4\right| :]|[         :-x-4:]|  |[:x+4  :]||                |[:x+4  :          ]
[:\left|2x-5\right|+\left|x+4\right|:]|[         :1-3x:]|  |[:9-x  :]||                |[:3x-1:          ]
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_var3() {
        let t = parse(EX_VAR3).unwrap();
        assert_eq!(t.rows.len(), 3);
        assert_eq!(t.rows[0].cases.len(), 3);
        // étiquette sans séparateur, « x » au centre
        assert_eq!(t.rows[0].cases[0].sep, Sep::Rien);
        assert_eq!(t.rows[0].cases[0].zones[1], "x");
        // étiquette des contenus : double barre
        assert_eq!(t.rows[1].cases[1].sep, Sep::Double);
        // -\infty cadré à gauche, x row
        assert_eq!(t.rows[0].cases[1].zones[0], "-\\infty");
        // valeur −3 posée sur la frontière
        assert_eq!(t.rows[0].cases[2].valeur_frontiere.as_deref(), Some("-3"));
        // f '(x) : moins au centre, zéro sur barre en frontière
        assert_eq!(t.rows[1].cases[1].zones[1], "-");
        assert_eq!(t.rows[1].cases[2].sep, Sep::Zero);
        // f(x) : flèches et valeur 2 sur la frontière
        assert!(est_fleche_seule("\\:>"));
        assert_eq!(t.rows[2].cases[2].valeur_frontiere.as_deref(), Some("2"));
    }

    #[test]
    fn jetons_basiques() {
        assert_eq!(
            parse_zone("/:> \\:> _> | |0 ||"),
            vec![
                Tok::Fleche(DirFleche::Montante),
                Tok::Fleche(DirFleche::Descendante),
                Tok::Fleche(DirFleche::Horizontale),
                Tok::Tige,
                Tok::ZeroB,
                Tok::Double,
            ]
        );
        // les deux écritures de la flèche montante
        assert_eq!(parse_zone("/:>"), parse_zone("/:>"));
        assert_eq!(parse_zone("/:>"), vec![Tok::Fleche(DirFleche::Montante)]);
        assert_eq!(parse_zone("/>"), vec![Tok::Fleche(DirFleche::Montante)]);
    }

    #[test]
    fn separateurs_et_valeurs() {
        // valeur entre cases
        let (sep, v) = classify_sep("5");
        assert_eq!(sep, Sep::Rien);
        assert_eq!(v.as_deref(), Some("5"));
        // séparateurs avec espaces tolérés
        assert_eq!(classify_sep("|  ").0, Sep::Trait);
        assert_eq!(classify_sep("0|").0, Sep::Zero);
        assert_eq!(classify_sep("|| ").0, Sep::Double);
        // repli : suite de barres (comme dans Abs) => trait simple
        assert_eq!(classify_sep("|  |").0, Sep::Trait);
        assert_eq!(classify_sep("||            |").0, Sep::Trait);
        // zéro nu
        assert_eq!(classify_sep("0").0, Sep::ZeroNu);
        // Var4 : double barre en bord droit
        let t = parse(EX_VAR4).unwrap();
        assert_eq!(t.rows[1].sep_fin, Sep::Double);
        // Abs : zéros sur barre
        let a = parse(EX_ABS).unwrap();
        assert_eq!(a.rows[1].cases[3].sep, Sep::Zero);
        assert_eq!(a.rows[3].cases[2].sep, Sep::Zero);
    }

    #[test]
    fn hachures_et_erreurs() {
        let t = parse("[:a:://:][:  :b  :]").unwrap();
        assert!(t.rows[0].cases[0].hachee);
        assert!(!t.rows[0].cases[1].hachee);
        // texte avant la première case
        assert!(parse("x[:b:]").is_err());
        // crochets non appariés
        assert!(parse("[:a:[").is_err());
        // nombres de cases différents
        assert!(parse("[:a:][:b:][:c:]\n[:d:]").is_err());
    }

    #[test]
    fn tous_les_exemples_se_parsent() {
        for (nom, code) in exemples() {
            let t = parse(code).unwrap_or_else(|e| panic!("{nom} : {e}"));
            let n = t.rows[0].cases.len();
            for r in &t.rows {
                assert_eq!(r.cases.len(), n, "{nom} : rangées inégales");
            }
            let (w, h) = dimensions(&t);
            assert!(w > 30.0 && h > 20.0, "{nom} : {w}×{h}");
            let p = pieces(&t);
            assert!(!p.is_empty(), "{nom} : aucune primitive");
        }
    }

    #[test]
    fn largeur_frac() {
        assert!(largeur_txt("\\frac{\\sqrt{2}}{2}", 4.2) > largeur_txt("2", 4.2));
        // \array : largeur = plus grande ligne
        assert!(
            largeur_txt("\\array{\\text{Variations}\\\\\\text{de }f(x)}", 4.2)
                > largeur_txt("\\text{de }f(x)", 4.2)
        );
    }

    #[test]
    fn pieces_contiennent_bordure_et_texte() {
        let t = parse(EX_VAR3).unwrap();
        let p = pieces(&t);
        let lignes = p
            .iter()
            .filter(|x| matches!(x, Piece::Ligne { .. }))
            .count();
        let textes = p
            .iter()
            .filter(|x| matches!(x, Piece::Texte { .. }))
            .count();
        let diagonales = p
            .iter()
            .filter(|x| matches!(x, Piece::Ligne { x0, y0, x1, y1, .. } if (x0 - x1).abs() > 1.0 && (y0 - y1).abs() > 1.0))
            .count();
        assert!(lignes >= 12, "lignes = {lignes}");
        assert!(textes >= 5, "textes = {textes}");
        assert!(
            diagonales >= 4,
            "diagonales (flèches+pointes) = {diagonales}"
        );
        // hachures : Var7 contient de nombreuses diagonales courtes
        let t7 = parse(EX_VAR7).unwrap();
        let p7 = pieces(&t7);
        let petites_diag = p7
            .iter()
            .filter(|x| {
                matches!(x, Piece::Ligne { x0, y0, x1, y1, w } if *w < 0.35
                    && (x1 - x0).abs() > 0.5 && (y1 - y0).abs() > 0.5)
            })
            .count();
        assert!(petites_diag >= 10, "hachures = {petites_diag}");
    }

    #[test]
    fn valeurs_sur_frontieres() {
        // Var1 : le « 5 » est entre la 2e et la 3e case de la ligne x
        let t = parse(EX_VAR1).unwrap();
        assert_eq!(t.rows[0].cases[2].valeur_frontiere.as_deref(), Some("5"));
        assert_eq!(t.rows[1].cases[2].valeur_frontiere.as_deref(), Some("2"));
        // Var2 : étiquette multiligne
        let v2 = parse(EX_VAR2).unwrap();
        assert!(v2.rows[1].cases[0].zones[1].contains("\\array"));
        let p2 = pieces(&v2);
        let textes = p2
            .iter()
            .filter(|x| matches!(x, Piece::Texte { .. }))
            .count();
        assert!(textes >= 6);
    }

    #[test]
    fn aller_retour_code() {
        for (nom, code) in exemples() {
            let t = parse(code).unwrap_or_else(|e| panic!("{nom} : {e}"));
            for r in &t.rows {
                let ligne = rangee_vers_code(r);
                let back = parse_ligne(&ligne).unwrap_or_else(|e| panic!("{nom} : {e}"));
                assert_eq!(back.cases.len(), r.cases.len(), "{nom}");
                for (c0, c1) in r.cases.iter().zip(&back.cases) {
                    assert_eq!(c0.sep, c1.sep, "{nom}");
                    assert_eq!(c0.hachee, c1.hachee, "{nom}");
                    if !c0.hachee {
                        assert_eq!(c0.zones, c1.zones, "{nom}");
                    }
                    assert_eq!(c0.valeur_frontiere, c1.valeur_frontiere, "{nom}");
                }
                assert_eq!(back.sep_fin, r.sep_fin, "{nom}");
                assert_eq!(back.valeur_fin, r.valeur_fin, "{nom}");
            }
        }
    }

    #[test]
    fn egaliser_pad_les_lignes_courtes() {
        let mauvais = "[:x:]||[a:b:c]\n[:f(x):]||[1:2:3]\n[:g:][:h:]||[u:v:w]\n";
        assert!(parse(mauvais).is_err());
        let fixe = egaliser(mauvais).unwrap();
        let t = parse(&fixe).unwrap();
        assert!(t.rows.iter().all(|r| r.cases.len() == 3));
        // les cases existantes sont conservées
        assert_eq!(t.rows[2].cases[2].zones[0], "u");
        assert_eq!(t.rows[2].cases[2].zones[1], "v");
    }
}
