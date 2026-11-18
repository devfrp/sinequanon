//! Analyseur et évaluateur d'expressions mathématiques.
//!
//! Syntaxe proche des calculatrices : `2*x+1`, `sin(x)`, `x^2`,
//! `exp(-x/4)*cos(2*pi*x)`. La multiplication implicite est acceptée
//! (`2x`, `3sin(x)`, `2(x+1)`), ainsi que `pi`, `e`, `π` et `θ`.

use std::f64::consts::{E, PI};

/// Arbre syntaxique d'une expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Num(f64),
    Var(String),
    Neg(Box<Node>),
    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),
    Pow(Box<Node>, Box<Node>),
    /// factorielle postfixée x! (man. p. 18) : Γ(x+1)
    Fact(Box<Node>),
    /// comparaison < > ≤ ≥ (man. p. 17) : 1.0 si vraie, NaN si fausse
    /// (la courbe n'est pas dessinée : intervalle de définition, p. 19)
    Rel(char, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
}

/// Variables disponibles lors de l'évaluation.
#[derive(Debug, Clone, Copy, Default)]
pub struct VarSet {
    pub x: Option<f64>,
    pub t: Option<f64>,
    pub theta: Option<f64>,
    /// indice de la suite (u_n = f(n))
    pub n: Option<f64>,
    /// valeur courant de la suite (u_{n+1} = f(u_n))
    pub u: Option<f64>,
    /// paramètre d'une famille de fonctions fp(x) (man. p. 26)
    pub p: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
}

/// Analyse une expression et renvoie son arbre syntaxique.
pub fn parse(src: &str) -> Result<Node, String> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks, pos: 0 };
    let node = p.relation()?;
    if let Some(t) = p.peek() {
        return Err(format!("symbole inattendu « {} »", p.describe_tok(t)));
    }
    Ok(node)
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '(' {
            toks.push(Tok::LParen);
            i += 1;
        } else if c == ')' {
            toks.push(Tok::RParen);
            i += 1;
        } else if c == ',' {
            toks.push(Tok::Comma);
            i += 1;
        } else if (c == '<' || c == '>') && chars.get(i + 1) == Some(&'=') {
            // opérateurs relationnels du manuel (p. 17) : <= et >=
            toks.push(Tok::Op(if c == '<' { '≤' } else { '≥' }));
            i += 2;
        } else if "+-*/^!<>".contains(c) {
            toks.push(Tok::Op(c));
            i += 1;
        } else if c == '×' {
            toks.push(Tok::Op('*'));
            i += 1;
        } else if c == '÷' {
            toks.push(Tok::Op('/'));
            i += 1;
        } else if c == '−' {
            toks.push(Tok::Op('-'));
            i += 1;
        } else if c == 'π' {
            toks.push(Tok::Ident("pi".into()));
            i += 1;
        } else if c == 'θ' {
            toks.push(Tok::Ident("theta".into()));
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // exposant : 1e-3, 2E5
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            let text: String = chars[start..i].iter().collect();
            let v: f64 = text
                .parse()
                .map_err(|_| format!("nombre invalide « {text} »"))?;
            toks.push(Tok::Num(v));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let lower = text.to_lowercase();
            // « xsin(x) » → « x » puis « sin(x) » (x n'initie aucune fonction) ;
            // « px^2 » → « p », « x »… sauf « pi » et « pow » (man. p. 26 :
            // le paramètre d'une famille peut être collé aux facteurs)
            if lower.len() > 1 && lower.starts_with('x') && !lower.starts_with("x_") {
                toks.push(Tok::Ident("x".into()));
                i = start + 1;
            } else if lower.len() > 1
                && lower.starts_with('p')
                && !lower.starts_with("pi")
                && !lower.starts_with("pow")
                && !lower.starts_with("p_")
            {
                toks.push(Tok::Ident("p".into()));
                i = start + 1;
            } else {
                toks.push(Tok::Ident(lower));
            }
        } else {
            return Err(format!("caractère inattendu « {c} »"));
        }
    }
    Ok(toks)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn describe_tok(&self, t: &Tok) -> String {
        match t {
            Tok::Num(v) => format!("{v}"),
            Tok::Ident(s) => s.clone(),
            Tok::Op(c) => c.to_string(),
            Tok::LParen => "(".into(),
            Tok::RParen => ")".into(),
            Tok::Comma => ",".into(),
        }
    }

    /// Niveau le plus bas : comparaisons < > <= >= (man. p. 17 et 19,
    /// intervalle de définition à la calculatrice).
    fn relation(&mut self) -> Result<Node, String> {
        let mut lhs = self.expr()?;
        while let Some(Tok::Op(c)) = self.peek().cloned() {
            if !matches!(c, '<' | '>' | '≤' | '≥') {
                break;
            }
            self.bump();
            let rhs = self.expr()?;
            lhs = Node::Rel(c, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn expr(&mut self) -> Result<Node, String> {
        let mut lhs = self.term()?;
        loop {
            match self.peek() {
                Some(Tok::Op('+')) => {
                    self.bump();
                    let rhs = self.term()?;
                    lhs = Node::Add(Box::new(lhs), Box::new(rhs));
                }
                Some(Tok::Op('-')) => {
                    self.bump();
                    let rhs = self.term()?;
                    lhs = Node::Sub(Box::new(lhs), Box::new(rhs));
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn term(&mut self) -> Result<Node, String> {
        let mut lhs = self.unary()?;
        loop {
            match self.peek() {
                Some(Tok::Op('*')) => {
                    self.bump();
                    let rhs = self.unary()?;
                    lhs = Node::Mul(Box::new(lhs), Box::new(rhs));
                }
                Some(Tok::Op('/')) => {
                    self.bump();
                    let rhs = self.unary()?;
                    lhs = Node::Div(Box::new(lhs), Box::new(rhs));
                }
                // multiplication implicite : 2x, 3sin(x), (x+1)(x-1)
                Some(Tok::Num(_) | Tok::Ident(_) | Tok::LParen) => {
                    let rhs = self.unary()?;
                    lhs = Node::Mul(Box::new(lhs), Box::new(rhs));
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Node, String> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.bump();
                let inner = self.unary()?;
                Ok(Node::Neg(Box::new(inner)))
            }
            Some(Tok::Op('+')) => {
                self.bump();
                self.unary()
            }
            _ => self.pow(),
        }
    }

    fn pow(&mut self) -> Result<Node, String> {
        let mut base = self.atom()?;
        // factorielle postfixée x! (man. p. 18) : aussi forte que
        // l'application de fonction (3!^2 = (3!)^2, -3! = -(3!))
        while matches!(self.peek(), Some(Tok::Op('!'))) {
            self.bump();
            base = Node::Fact(Box::new(base));
        }
        if matches!(self.peek(), Some(Tok::Op('^'))) {
            self.bump();
            // exponentiation droite-associative : 2^3^2 = 2^(3^2)
            let exp = self.unary()?;
            return Ok(Node::Pow(Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<Node, String> {
        let tok = match self.peek() {
            Some(t) => t.clone(),
            None => return Err("expression incomplète".into()),
        };
        match tok {
            Tok::Num(v) => {
                self.bump();
                Ok(Node::Num(v))
            }
            Tok::Ident(name) => {
                self.bump();
                if matches!(self.peek(), Some(Tok::LParen)) {
                    self.bump();
                    let mut args = vec![self.relation()?];
                    while matches!(self.peek(), Some(Tok::Comma)) {
                        self.bump();
                        args.push(self.expr()?);
                    }
                    if !matches!(self.peek(), Some(Tok::RParen)) {
                        return Err(format!("« ) » attendu après les arguments de {name}"));
                    }
                    self.bump();
                    Ok(Node::Call(name, args))
                } else if Node::prefixable(&name) {
                    // application sans parenthèses (man. p. 16 : « 2 sin x »,
                    // p. 26 : « p sin x », « log p ») : l'argument est le
                    // facteur primaire suivant (les fonctions sont
                    // prioritaires sur « ^ », p. 17)
                    let arg = self.atom()?;
                    Ok(Node::Call(name, vec![arg]))
                } else {
                    Ok(Node::Var(name))
                }
            }
            Tok::LParen => {
                self.bump();
                let n = self.relation()?;
                if !matches!(self.peek(), Some(Tok::RParen)) {
                    return Err("« ) » attendu".into());
                }
                self.bump();
                Ok(n)
            }
            other => Err(format!(
                "symbole inattendu « {} »",
                self.describe_tok(&other)
            )),
        }
    }
}

/// Exponentiation avec l'extension du manuel (p. 18) : pour une base
/// négative, x^y est réel si y = p/q avec q impair (x^(1/3), x^(2/3)),
/// signé selon la parité de p ; sinon NaN (x^(1/2) hors domaine).
fn puissance(x: f64, y: f64) -> f64 {
    if x >= 0.0 || !y.is_finite() {
        return x.powf(y);
    }
    if let Some((p, q)) = fraction(y) {
        if q % 2 == 1 {
            let r = (-x).powf(y);
            return if p.rem_euclid(2) == 1 { -r } else { r };
        }
    }
    f64::NAN
}

/// Plus petite fraction p/q (q ≤ 96) représentant y à 1e-9 près ; None
/// si y n'est pas une fraction à petit dénominateur.
fn fraction(y: f64) -> Option<(i64, u32)> {
    for q in 1u32..=96 {
        let p = (y * q as f64).round() as i64;
        if (y * q as f64 - p as f64).abs() < 1e-9 {
            return Some((p, q));
        }
    }
    None
}

/// Factorielle étendue aux réels (man. p. 18) : x! = Γ(x+1) ; entiers
/// non négatifs exacts, pôles aux entiers négatifs → NaN.
fn factorielle(v: f64) -> f64 {
    if v >= 0.0 && (v - v.round()).abs() < 1e-9 {
        let n = v.round() as u32;
        let mut r = 1.0;
        for k in 2..=n {
            r *= k as f64;
        }
        r
    } else {
        gamma(v + 1.0)
    }
}

/// Fonction gamma d'Euler (approximation de Lanczos, g = 7).
fn gamma(z: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if z <= 0.0 && (z - z.round()).abs() < 1e-9 {
        // pôles en 0, -1, -2… : Γ n'y est pas définie
        return f64::NAN;
    }
    if z < 0.5 {
        // réflexion : Γ(z) = π / (sin(πz)·Γ(1−z))
        return std::f64::consts::PI / ((std::f64::consts::PI * z).sin() * gamma(1.0 - z));
    }
    let z = z - 1.0;
    let mut x = C[0];
    for (i, &c) in C.iter().enumerate().skip(1) {
        x += c / (z + i as f64);
    }
    let t = z + 7.0 + 0.5;
    (2.0 * std::f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * x
}

fn lookup(name: &str, vars: &VarSet) -> Option<f64> {
    match name {
        "x" => vars.x,
        "t" => vars.t,
        "theta" => vars.theta,
        "n" => vars.n,
        "u" => vars.u,
        "p" => vars.p,
        "pi" => Some(PI),
        "e" => Some(E),
        _ => None,
    }
}

fn call(name: &str, args: &[Node], vars: &VarSet, unknown: &mut Vec<String>) -> f64 {
    let mut v = [0.0f64; 2];
    for (i, a) in args.iter().take(2).enumerate() {
        v[i] = a.eval(vars, unknown);
    }
    let (a, b) = (v[0], v[1]);
    match (name, args.len()) {
        ("sin", 1) => a.sin(),
        ("cos", 1) => a.cos(),
        ("tan", 1) => a.tan(),
        ("asin", 1) | ("arcsin", 1) => a.asin(),
        ("acos", 1) | ("arccos", 1) => a.acos(),
        ("atan", 1) | ("arctan", 1) => a.atan(),
        ("atan2", 2) => a.atan2(b),
        ("sinh", 1) => a.sinh(),
        ("cosh", 1) => a.cosh(),
        ("tanh", 1) => a.tanh(),
        ("exp", 1) => a.exp(),
        ("ln", 1) => a.ln(),
        ("log", 1) | ("log10", 1) => a.log10(),
        ("log2", 1) => a.log2(),
        ("sqrt", 1) => a.sqrt(),
        ("abs", 1) => a.abs(),
        ("floor", 1) => a.floor(),
        ("ceil", 1) => a.ceil(),
        ("round", 1) => a.round(),
        ("sign", 1) => {
            if a > 0.0 {
                1.0
            } else if a < 0.0 {
                -1.0
            } else if a == 0.0 {
                0.0
            } else {
                f64::NAN
            }
        }
        ("min", 2) => a.min(b),
        ("max", 2) => a.max(b),
        // modulo mathématique (resultat du signe du diviseur)
        ("mod", 2) => a - b * (a / b).floor(),
        ("pow", 2) => puissance(a, b),
        // noms français du manuel (p. 17) : sh ch th, argsh argch argth,
        // racine, int, frac, carré (ou carre)
        ("sh", 1) => a.sinh(),
        ("ch", 1) => a.cosh(),
        ("th", 1) => a.tanh(),
        ("argsh", 1) => a.asinh(),
        ("argch", 1) => a.acosh(),
        ("argth", 1) => a.atanh(),
        ("racine", 1) => a.sqrt(),
        ("int", 1) => a.floor(),
        ("frac", 1) => a - a.floor(),
        ("carré", 1) | ("carre", 1) => a * a,
        _ => {
            if unknown.len() < 16 {
                unknown.push(format!("{name}({} argument(s))", args.len()));
            }
            f64::NAN
        }
    }
}

impl Node {
    /// Évalue l'arbre ; les identifiants inconnus poussent un message dans
    /// `unknown` (plafonné) et produisent NaN.
    pub fn eval(&self, vars: &VarSet, unknown: &mut Vec<String>) -> f64 {
        match self {
            Node::Num(v) => *v,
            Node::Var(name) => lookup(name, vars).unwrap_or_else(|| {
                if unknown.len() < 16 {
                    unknown.push(name.clone());
                }
                f64::NAN
            }),
            Node::Neg(a) => -a.eval(vars, unknown),
            Node::Add(a, b) => a.eval(vars, unknown) + b.eval(vars, unknown),
            Node::Sub(a, b) => a.eval(vars, unknown) - b.eval(vars, unknown),
            Node::Mul(a, b) => a.eval(vars, unknown) * b.eval(vars, unknown),
            Node::Div(a, b) => a.eval(vars, unknown) / b.eval(vars, unknown),
            Node::Pow(a, b) => puissance(a.eval(vars, unknown), b.eval(vars, unknown)),
            Node::Fact(a) => factorielle(a.eval(vars, unknown)),
            Node::Rel(op, a, b) => {
                let (l, r) = (a.eval(vars, unknown), b.eval(vars, unknown));
                // vraie → 1.0 (neutre en produit), fausse → NaN (la
                // courbe n'est pas dessinée : man. p. 19)
                let vrai = match op {
                    '<' => l < r,
                    '>' => l > r,
                    '≤' => l <= r,
                    _ => l >= r,
                };
                if vrai {
                    1.0
                } else {
                    f64::NAN
                }
            }
            Node::Call(name, args) => call(name, args, vars, unknown),
        }
    }

    /// Fonctions reconnues par l'évaluateur.
    /// Fonctions à un argument : seules celles-ci admettent
    /// l'application sans parenthèses (man. p. 16).
    fn prefixable(name: &str) -> bool {
        matches!(
            name,
            "sin"
                | "cos"
                | "tan"
                | "asin"
                | "acos"
                | "atan"
                | "arcsin"
                | "arccos"
                | "arctan"
                | "sinh"
                | "cosh"
                | "tanh"
                | "sh"
                | "ch"
                | "th"
                | "argsh"
                | "argch"
                | "argth"
                | "exp"
                | "ln"
                | "log"
                | "log10"
                | "log2"
                | "sqrt"
                | "racine"
                | "abs"
                | "floor"
                | "ceil"
                | "round"
                | "sign"
                | "int"
                | "frac"
                | "carré"
                | "carre"
        )
    }

    pub const FONCTIONS: &[&str] = &[
        "sin", "cos", "tan", "asin", "acos", "atan", "arcsin", "arccos", "arctan", "atan2", "sinh",
        "cosh", "tanh", "exp", "ln", "log", "log10", "log2", "sqrt", "abs", "floor", "ceil",
        "round", "sign", "min", "max", "mod", "pow", "sh", "ch", "th", "argsh", "argch", "argth",
        "racine", "int", "frac", "carré", "carre",
    ];

    /// Vérifie que seuls des identifiants connus apparaissent : les
    /// fonctions doivent exister, les variables être dans `allowed`.
    pub fn validate_vars(&self, allowed: &[&str]) -> Result<(), String> {
        match self {
            Node::Num(_) => Ok(()),
            Node::Var(n) => {
                if matches!(n.as_str(), "pi" | "e") || allowed.contains(&n.as_str()) {
                    Ok(())
                } else {
                    Err(format!("identifiant inconnu « {n} »"))
                }
            }
            Node::Neg(a) => a.validate_vars(allowed),
            Node::Add(a, b)
            | Node::Sub(a, b)
            | Node::Mul(a, b)
            | Node::Div(a, b)
            | Node::Pow(a, b)
            | Node::Rel(_, a, b) => {
                a.validate_vars(allowed)?;
                b.validate_vars(allowed)
            }
            Node::Fact(a) => a.validate_vars(allowed),
            Node::Call(name, args) => {
                if !Self::FONCTIONS.contains(&name.as_str()) {
                    return Err(format!("fonction inconnue « {name} »"));
                }
                for a in args {
                    a.validate_vars(allowed)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(src: &str, x: f64) -> f64 {
        let n = parse(src).unwrap();
        n.eval(
            &VarSet {
                x: Some(x),
                ..Default::default()
            },
            &mut Vec::new(),
        )
    }

    #[test]
    fn basique() {
        assert!((ev("2*x+1", 3.0) - 7.0).abs() < 1e-12);
        assert!((ev("x/2 - 0.5", 3.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn implicite() {
        assert!((ev("2x", 5.0) - 10.0).abs() < 1e-12);
        assert!(ev("3sin(x)", 0.0).abs() < 1e-12);
        assert!((ev("2(x+1)", 2.0) - 6.0).abs() < 1e-12);
        assert!((ev("xsin(x)", 1.0) - 1.0f64.sin()).abs() < 1e-12);
    }

    #[test]
    fn priorites() {
        assert!((ev("-x^2", 3.0) + 9.0).abs() < 1e-12);
        assert!((ev("2^3^2", 0.0) - 512.0).abs() < 1e-9);
        assert!((ev("1+2*3", 0.0) - 7.0).abs() < 1e-12);
        assert!((ev("2^-3", 0.0) - 0.125).abs() < 1e-12);
    }

    #[test]
    fn fonctions_et_constantes() {
        assert!((ev("sin(pi/2)", 0.0) - 1.0).abs() < 1e-12);
        assert!((ev("sin(π/2)", 0.0) - 1.0).abs() < 1e-12);
        assert!((ev("log(100)", 0.0) - 2.0).abs() < 1e-12);
        assert!((ev("ln(e)", 0.0) - 1.0).abs() < 1e-12);
        assert!((ev("sqrt(2)*sqrt(2)", 0.0) - 2.0).abs() < 1e-12);
        assert!((ev("exp(1)", 0.0) - E).abs() < 1e-12);
        assert!((ev("abs(-3)", 0.0) - 3.0).abs() < 1e-12);
        assert!((ev("max(2,5)", 0.0) - 5.0).abs() < 1e-12);
        assert!((ev("mod(7,3)", 0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn variables_n_et_u() {
        let ev_n = |src: &str, n: f64| -> f64 {
            parse(src).unwrap().eval(
                &VarSet {
                    n: Some(n),
                    ..Default::default()
                },
                &mut Vec::new(),
            )
        };
        assert!((ev_n("2*n+1", 3.0) - 7.0).abs() < 1e-12);
        assert!((ev_n("n^2", 5.0) - 25.0).abs() < 1e-12);
        assert!((ev_n("3+n*2", 4.0) - 11.0).abs() < 1e-12);
        // « x » n'est pas une variable de suite
        assert!(ev_n("x*2", 3.0).is_nan());
        let ev_u = |src: &str, u: f64| -> f64 {
            parse(src).unwrap().eval(
                &VarSet {
                    u: Some(u),
                    ..Default::default()
                },
                &mut Vec::new(),
            )
        };
        assert!((ev_u("2*u+1", 2.0) - 5.0).abs() < 1e-12);
        assert!((ev_u("u^2-u+1", 3.0) - 7.0).abs() < 1e-12);
        assert!(parse("foo(n)").unwrap().validate_vars(&["n"]).is_err());
    }

    #[test]
    fn erreurs() {
        assert!(parse("2+").is_err());
        assert!(parse("(x").is_err());
        assert!(parse("2 $ 3").is_err());
        // « foo(x) » est syntaxiquement valide mais sémantiquement refusé :
        assert!(parse("foo(x)").is_ok());
        assert!(parse("foo(x)").unwrap().validate_vars(&["x"]).is_err());
        assert!(parse("sin(x").is_err());
        let n = parse("t").unwrap();
        assert!(n.validate_vars(&["x"]).is_err());
        assert!(n.validate_vars(&["x", "t"]).is_ok());
    }

    #[test]
    fn nan_sur_domaine_vide() {
        assert!(ev("sqrt(x)", -1.0).is_nan());
        assert!(ev("ln(x)", -1.0).is_nan());
        assert!(ev("1/0", 0.0).is_infinite());
    }

    // ----- v1.8 : liste des fonctions du manuel (p. 17-19) -----

    #[test]
    fn alias_fonctions_manuel() {
        // sh ch th, argsh argch argth, racine, int, frac, carré/carre
        assert!(ev("sh(0)", 0.0).abs() < 1e-12);
        assert!((ev("ch(0)", 0.0) - 1.0).abs() < 1e-12);
        assert!(ev("th(0)", 0.0).abs() < 1e-12);
        assert!((ev("argsh(sh(2))", 0.0) - 2.0).abs() < 1e-9);
        assert!((ev("argch(ch(2))", 0.0) - 2.0).abs() < 1e-9);
        assert!((ev("argth(th(0.5))", 0.0) - 0.5).abs() < 1e-9);
        assert!((ev("racine(4)", 0.0) - 2.0).abs() < 1e-12);
        assert!((ev("int(2.7)", 0.0) - 2.0).abs() < 1e-12);
        assert!((ev("frac(2.7)", 0.0) - 0.7).abs() < 1e-12);
        assert!((ev("carré(3)", 0.0) - 9.0).abs() < 1e-12);
        assert!((ev("carre(3)", 0.0) - 9.0).abs() < 1e-12);
        // noms officiels du manuel (p. 17) : arccos, arcsin, arctan
        assert!((ev("arccos(0.5)", 0.0) - std::f64::consts::FRAC_PI_3).abs() < 1e-9);
        assert!((ev("arcsin(0.5)", 0.0) - std::f64::consts::FRAC_PI_6).abs() < 1e-9);
        assert!((ev("arctan(1)", 0.0) - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
        // majuscules acceptées (man. p. 16)
        assert!((ev("RACINE(4)", 0.0) - 2.0).abs() < 1e-12);
        // log = logarithme décimal (man. p. 17)
        assert!((ev("log(100)", 0.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn application_sans_parentheses() {
        // man. p. 16 : « 2 sin x », p. 26 : « p sin x », « log p »
        let pi2 = std::f64::consts::FRAC_PI_2;
        assert!((ev("sin x", pi2) - 1.0).abs() < 1e-12);
        assert!((ev("2 sin x", pi2) - 2.0).abs() < 1e-12);
        assert!((ev("log x", 100.0) - 2.0).abs() < 1e-12);
        assert!(ev("ln sin x", pi2).abs() < 1e-12);
        assert!((ev("racine x", 9.0) - 3.0).abs() < 1e-12);
        assert!((ev("int x", 2.7) - 2.0).abs() < 1e-12);
        assert!((ev("sin x + 1", pi2) - 2.0).abs() < 1e-12);
        // priorité du manuel (p. 17) : fonctions avant « ^ » →
        // sin x^2 = (sin x)^2
        assert!((ev("sin x^2", 2.0) - 2.0f64.sin().powi(2)).abs() < 1e-12);
        // une fonction à deux arguments ne prend pas d'argument sans
        // parenthèses : « min x » reste un identifiant inconnu
        assert!(parse("min x").unwrap().validate_vars(&["x"]).is_err());
        assert!(parse("arccos x").unwrap().validate_vars(&["x"]).is_ok());
    }

    #[test]
    fn factorielle() {
        // entiers exacts, réels par Γ(x+1) (man. p. 18)
        assert!((ev("0!", 0.0) - 1.0).abs() < 1e-12);
        assert!((ev("5!", 0.0) - 120.0).abs() < 1e-9);
        assert!((ev("1.5!", 0.0) - 1.329_340_388_179_137).abs() < 1e-9);
        // Γ(0.5) = √π → (-0.5)! = √π
        assert!((ev("(-0.5)!", 0.0) - std::f64::consts::PI.sqrt()).abs() < 1e-9);
        // pôles aux entiers négatifs
        assert!(ev("(-1)!", 0.0).is_nan());
        assert!(ev("(-2)!", 0.0).is_nan());
        // exemple du manuel : coefficient binomial interpolé
        assert!((ev("6/(x!(3-x)!)", 1.0) - 3.0).abs() < 1e-9);
        // aussi forte que l'application de fonction
        assert!((ev("-3!", 0.0) + 6.0).abs() < 1e-12);
        assert!((ev("3!^2", 0.0) - 36.0).abs() < 1e-12);
        assert!((ev("2^3!", 0.0) - 64.0).abs() < 1e-12);
        assert!((ev("3x!", 2.0) - 6.0).abs() < 1e-12);
    }

    #[test]
    fn puissances_non_entieres() {
        // dénominateur impair : défini sur tout R (man. p. 18)
        assert!((ev("(-8)^(1/3)", 0.0) + 2.0).abs() < 1e-9);
        assert!((ev("(-8)^(2/3)", 0.0) - 4.0).abs() < 1e-9);
        assert!((ev("(-8)^(4/3)", 0.0) - 16.0).abs() < 1e-9);
        assert!((ev("(-27)^(1/3)", 0.0) + 3.0).abs() < 1e-9);
        assert!((ev("(-8)^(-1/3)", 0.0) + 0.5).abs() < 1e-9);
        // dénominateur pair : hors domaine pour x < 0
        assert!(ev("(-8)^(1/2)", 0.0).is_nan());
        assert!(ev("(-8)^(0.5)", 0.0).is_nan());
        assert!(ev("(-8)^(1/6)", 0.0).is_nan());
        // entiers et base positive inchangés
        assert!((ev("(-2)^2", 0.0) - 4.0).abs() < 1e-12);
        assert!((ev("8^(1/3)", 0.0) - 2.0).abs() < 1e-9);
        assert!((ev("pow(-8, 1/3)", 0.0) + 2.0).abs() < 1e-9);
        // priorité : 3x^4 = 3*(x^4) (man. p. 17)
        assert!((ev("3x^4", 2.0) - 48.0).abs() < 1e-12);
    }

    #[test]
    fn intervalle_de_definition() {
        // opérateurs relationnels : 1.0 si vraie, NaN si fausse (p. 19)
        assert_eq!(ev("x>=2", 3.0), 1.0);
        assert!(ev("x>=2", 1.0).is_nan());
        assert!(ev("x>2", 2.0).is_nan());
        assert_eq!(ev("x<=5", 5.0), 1.0);
        assert!(ev("x<=5", 6.0).is_nan());
        assert_eq!(ev("x<2", 1.0), 1.0);
        // f(x) = (3/(x-1))(x>=2) : dessinée sur [2, +∞[ seulement
        assert!((ev("(3/(x-1))(x>=2)", 3.0) - 1.5).abs() < 1e-12);
        assert!(ev("(3/(x-1))(x>=2)", 0.0).is_nan());
        // restreinte à ]1, 5] : les deux conditions
        let src = "(3/(x-1))(x>1)(x<=5)";
        assert!((ev(src, 3.0) - 1.5).abs() < 1e-12);
        assert!(ev(src, 0.5).is_nan());
        assert!(ev(src, 6.0).is_nan());
        assert!(ev(src, 1.0).is_nan());
    }

    #[test]
    fn validation_nouveautes() {
        // les nouvelles fonctions passent la validation
        parse("sh(x)+argth(x)+racine(x)+int(x)+frac(x)+carré(x)")
            .unwrap()
            .validate_vars(&["x"])
            .unwrap();
        // une fonction inconnue reste refusée
        assert!(parse("qsh(x)").is_ok());
        assert!(parse("qsh(x)").unwrap().validate_vars(&["x"]).is_err());
        // factorielle et comparaisons recursent
        parse("x!^2*(x>=0)").unwrap().validate_vars(&["x"]).unwrap();
        assert!(parse("3!q").is_ok());
        assert!(parse("3!q").unwrap().validate_vars(&["x"]).is_err());
    }
}
