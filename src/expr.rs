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
    Call(String, Vec<Node>),
}

/// Variables disponibles lors de l'évaluation.
#[derive(Debug, Clone, Copy, Default)]
pub struct VarSet {
    pub x: Option<f64>,
    pub t: Option<f64>,
    pub theta: Option<f64>,
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
    let node = p.expr()?;
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
        } else if "+-*/^".contains(c) {
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
            // « xsin(x) » → « x » puis « sin(x) » (x n'initie aucune fonction)
            if lower.len() > 1 && lower.starts_with('x') && !lower.starts_with("x_") {
                toks.push(Tok::Ident("x".into()));
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
        let base = self.atom()?;
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
                    let mut args = vec![self.expr()?];
                    while matches!(self.peek(), Some(Tok::Comma)) {
                        self.bump();
                        args.push(self.expr()?);
                    }
                    if !matches!(self.peek(), Some(Tok::RParen)) {
                        return Err(format!("« ) » attendu après les arguments de {name}"));
                    }
                    self.bump();
                    Ok(Node::Call(name, args))
                } else {
                    Ok(Node::Var(name))
                }
            }
            Tok::LParen => {
                self.bump();
                let n = self.expr()?;
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

fn lookup(name: &str, vars: &VarSet) -> Option<f64> {
    match name {
        "x" => vars.x,
        "t" => vars.t,
        "theta" => vars.theta,
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
        ("asin", 1) => a.asin(),
        ("acos", 1) => a.acos(),
        ("atan", 1) => a.atan(),
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
        ("pow", 2) => a.powf(b),
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
            Node::Pow(a, b) => a.eval(vars, unknown).powf(b.eval(vars, unknown)),
            Node::Call(name, args) => call(name, args, vars, unknown),
        }
    }

    /// Fonctions reconnues par l'évaluateur.
    pub const FONCTIONS: &[&str] = &[
        "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "ln",
        "log", "log10", "log2", "sqrt", "abs", "floor", "ceil", "round", "sign", "min", "max",
        "mod", "pow",
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
            | Node::Pow(a, b) => {
                a.validate_vars(allowed)?;
                b.validate_vars(allowed)
            }
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
}
