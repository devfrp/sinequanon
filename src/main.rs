//! Sine qua non (Rust) — traceur de courbes.
//!
//! Sans argument : interface graphique. `sinequanon export ...` : tracé
//! en ligne de commande (SVG/PNG).

#![forbid(unsafe_code)]

mod annot;
mod app;
mod curve;
mod expr;
mod grid;
mod i18n;
mod pdf;
mod png;
mod preferences;
mod render;
mod schema;
mod session;
mod simu;
mod solve;
mod stats;
mod tabgen;
mod tableau;

use crate::render::CurvePolys;
use annot::{IntegraleAnnot, PointAnnot, TangenteAnnot};
use curve::{apply_style, Curve, CurveKind, PALETTE};
use eframe::egui;
use grid::Page;
use i18n::Lang;
use std::path::PathBuf;

const AIDE: &str = "\
Sine qua non (Rust) — traceur de courbes

UTILISATION
  sinequanon                  Ouvre l'interface graphique
  sinequanon export ...       Génère un tracé en SVG et/ou PNG
  sinequanon --aide           Affiche cette aide
  sinequanon --version        Affiche le numéro de version

OPTIONS D'EXPORT
  -f, --fonction EXPR         Ajoute y = EXPR (répétable)
  -p, --polaire EXPR          Ajoute r(θ) = EXPR (répétable)
  -c, --param X;Y             Ajoute x = X(t), y = Y(t) (répétable)
      --t-min N --t-max N     Domaine de t/θ (défaut : -2π .. 2π)
      --point C:x             Point sur la courbe C à l'abscisse x
      --tangente C:x          Tangente à la courbe C en x (fonctions)
      --aire C:a:b            Intégrale hachurée de C entre a et b
      --svg FICHIER           Sortie SVG (défaut : sinequanon.svg)
      --png FICHIER           Sortie PNG
      --px-par-mm N           Résolution du PNG (défaut : 4)
      --largeur-mm N          Largeur de la page (défaut : 170)
      --hauteur-mm N          Hauteur de la page (défaut : 120)
      --origine-x N           Abscisse du centre de la page (défaut : 0)
      --origine-y N           Ordonnée du centre de la page (défaut : 0)
      --unite-mm N            Longueur d'une unité en mm (défaut : 20)
      --log-x --log-y         Échelles logarithmiques

OPTIONS GÉNÉRALES
      --lang fr|en            Langue de l'interface (défaut : fr)

EXEMPLES
  sinequanon export -f \"sin(x)\" -f \"x^2/4-2\" --svg courbe.svg
  sinequanon export -p \"2+cos(theta)\" --png courbe.png
  sinequanon export -c \"cos(t);sin(t)/2\" --t-min -6.28 --t-max 6.28
  sinequanon export -f \"x^2\" --aire 0:1:2 --tangente 0:2 --point 0:1
";

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => gui(lang_de(&args)).map_err(|e| e.to_string()),
        Some("--version" | "-V") => {
            println!("sinequanon {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("--aide" | "--help" | "-h" | "aide") => {
            println!("{AIDE}");
            Ok(())
        }
        Some("export") => exporter(&args[1..]),
        Some("--lang") => gui(lang_de(&args)).map_err(|e| e.to_string()),
        Some(other) => Err(format!(
            "commande inconnue « {other} ». Sans argument, l'interface graphique s'ouvre ; voir --aide."
        )),
    }
}

/// Langue demandée par `--lang fr|en` (None sinon).
fn lang_de(args: &[String]) -> Option<Lang> {
    for (i, a) in args.iter().enumerate() {
        if a == "--lang" {
            return match args.get(i + 1).map(String::as_str) {
                Some("fr") => Some(Lang::Fr),
                Some("en") => Some(Lang::En),
                _ => {
                    eprintln!("avertissement : --lang attend fr ou en");
                    None
                }
            };
        }
    }
    None
}

fn gui(lang: Option<Lang>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 780.0])
            .with_min_inner_size([760.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sine qua non",
        options,
        Box::new(|_cc| Ok(Box::new(app::SqApp::new(lang)) as Box<dyn eframe::App>)),
    )
}

struct Opts {
    fonctions: Vec<String>,
    polaires: Vec<String>,
    params: Vec<(String, String)>,
    tmin: f64,
    tmax: f64,
    points: Vec<(usize, f64)>,
    tangentes: Vec<(usize, f64)>,
    aires: Vec<(usize, f64, f64)>,
    svg: Option<String>,
    png: Option<String>,
    bmp: Option<String>,
    gif: Option<String>,
    eps: Option<String>,
    px_par_mm: f64,
    page: Page,
    /// séries statistiques à une variable (v0.4), chargées depuis le
    /// fichier de session
    serie1: Vec<stats::Serie1>,
    /// boîtes à moustaches multiples (v0.4)
    boites: Vec<stats::BoitesMultiples>,
    /// suites numériques (v0.6+)
    suites: Vec<stats::Suite>,
    /// séries statistiques à 2 variables (v0.8+)
    serie2: Vec<stats::Serie2>,
    /// arbres de probabilités (v0.9+)
    arbres: Vec<stats::Arbre>,
    /// simulations statistiques (v1.0+)
    simus: Vec<crate::simu::Simulation>,
    /// objets du schéma (v1.3) : points nommés et Bézier
    schema: Vec<schema::ShObj>,
    /// impression (v1.3) : chemin du PDF à écrire
    pdf: Option<String>,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            fonctions: Vec::new(),
            polaires: Vec::new(),
            params: Vec::new(),
            tmin: -2.0 * std::f64::consts::PI,
            tmax: 2.0 * std::f64::consts::PI,
            points: Vec::new(),
            tangentes: Vec::new(),
            aires: Vec::new(),
            svg: None,
            png: None,
            bmp: None,
            gif: None,
            eps: None,
            px_par_mm: 4.0,
            page: Page::default(),
            serie1: Vec::new(),
            boites: Vec::new(),
            suites: Vec::new(),
            serie2: Vec::new(),
            arbres: Vec::new(),
            simus: Vec::new(),
            schema: Vec::new(),
            pdf: None,
        }
    }
}

/// Analyse « C:x » (courbe, abscisse) utilisé par --point/--tangente.
fn courbe_x(v: &str, option: &str) -> Result<(usize, f64), String> {
    let (c, x) = v
        .split_once(':')
        .ok_or_else(|| format!("{option} attend « C:x », reçu « {v} »"))?;
    let c = c
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("{option} : « {c} » n'est pas un indice de courbe"))?;
    let x = nb(x.trim(), option)?;
    Ok((c, x))
}

/// Analyse « C:a:b » utilisé par --aire.
fn courbe_ab(v: &str, option: &str) -> Result<(usize, f64, f64), String> {
    let (c, reste) = v
        .split_once(':')
        .ok_or_else(|| format!("{option} attend « C:a:b », reçu « {v} »"))?;
    let (a, b) = reste
        .split_once(':')
        .ok_or_else(|| format!("{option} attend « C:a:b », reçu « {v} »"))?;
    let c = c
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("{option} : « {c} » n'est pas un indice de courbe"))?;
    Ok((c, nb(a.trim(), option)?, nb(b.trim(), option)?))
}

fn valeur(args: &[String], i: &mut usize, option: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("valeur manquante après {option}"))
}

fn nb(v: &str, option: &str) -> Result<f64, String> {
    v.parse::<f64>()
        .map_err(|_| format!("« {v} » n'est pas un nombre (option {option})"))
}

fn exporter(args: &[String]) -> Result<(), String> {
    let mut o = Opts::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "-f" | "--fonction" => o.fonctions.push(valeur(args, &mut i, a)?),
            "-p" | "--polaire" => o.polaires.push(valeur(args, &mut i, a)?),
            "-c" | "--param" => {
                let v = valeur(args, &mut i, a)?;
                let (xt, yt) = v
                    .split_once(';')
                    .ok_or_else(|| format!("--param attend « X;Y », reçu « {v} »"))?;
                o.params.push((xt.trim().into(), yt.trim().into()));
            }
            "--t-min" => o.tmin = nb(&valeur(args, &mut i, a)?, a)?,
            "--t-max" => o.tmax = nb(&valeur(args, &mut i, a)?, a)?,
            "--point" => o.points.push(courbe_x(&valeur(args, &mut i, a)?, a)?),
            "--tangente" => o.tangentes.push(courbe_x(&valeur(args, &mut i, a)?, a)?),
            "--aire" => o.aires.push(courbe_ab(&valeur(args, &mut i, a)?, a)?),
            "--lang" => {
                // toléré sur la ligne export (sans effet sur la sortie)
                valeur(args, &mut i, a)?;
            }
            "--session" => {
                // v0.4 : charge page, repère et séries statistiques d'une
                // session JSON
                let chemin = valeur(args, &mut i, a)?;
                let txt = std::fs::read_to_string(&chemin)
                    .map_err(|e| format!("lecture de {chemin} : {e}"))?;
                let s: session::Session = serde_json::from_str(&txt)
                    .map_err(|e| format!("session {chemin} invalide : {e}"))?;
                o.page = s.page;
                o.serie1 = s.stats;
                o.boites = s.boites;
                o.suites = s.suites;
                o.serie2 = s.serie2;
                o.arbres = s.arbres;
                o.simus = s.simus;
                o.schema = s.schema;
            }
            "--svg" => o.svg = Some(valeur(args, &mut i, a)?),
            "--png" => o.png = Some(valeur(args, &mut i, a)?),
            "--bmp" => o.bmp = Some(valeur(args, &mut i, a)?),
            "--gif" => o.gif = Some(valeur(args, &mut i, a)?),
            "--eps" => o.eps = Some(valeur(args, &mut i, a)?),
            "--pdf" => o.pdf = Some(valeur(args, &mut i, a)?),
            "--px-par-mm" => o.px_par_mm = nb(&valeur(args, &mut i, a)?, a)?,
            "--largeur-mm" => o.page.width_mm = nb(&valeur(args, &mut i, a)?, a)?,
            "--hauteur-mm" => o.page.height_mm = nb(&valeur(args, &mut i, a)?, a)?,
            "--origine-x" => o.page.cx = nb(&valeur(args, &mut i, a)?, a)?,
            "--origine-y" => o.page.cy = nb(&valeur(args, &mut i, a)?, a)?,
            "--unite-mm" => o.page.unit_mm = nb(&valeur(args, &mut i, a)?, a)?,
            "--log-x" => o.page.log_x = true,
            "--log-y" => o.page.log_y = true,
            other => return Err(format!("option inconnue « {other} » (voir --aide)")),
        }
        i += 1;
    }

    let mut curves = Vec::new();
    for (k, e) in o.fonctions.iter().enumerate() {
        curves.push(Curve::fonction(e.clone(), PALETTE[k % PALETTE.len()]));
    }
    for (k, e) in o.polaires.iter().enumerate() {
        curves.push(Curve::with_kind(
            CurveKind::Polaire {
                r: e.clone(),
                tmin: o.tmin,
                tmax: o.tmax,
            },
            PALETTE[(k + 1) % PALETTE.len()],
        ));
    }
    for (k, (xt, yt)) in o.params.iter().enumerate() {
        curves.push(Curve::with_kind(
            CurveKind::Parametree {
                xt: xt.clone(),
                yt: yt.clone(),
                tmin: o.tmin,
                tmax: o.tmax,
            },
            PALETTE[(k + 2) % PALETTE.len()],
        ));
    }
    if curves.is_empty()
        && o.serie1.is_empty()
        && o.boites.is_empty()
        && o.suites.is_empty()
        && o.serie2.is_empty()
        && o.arbres.is_empty()
        && o.simus.is_empty()
        && o.schema.is_empty()
    {
        return Err(
            "aucune courbe ni série statistique : utilisez -f, -p, -c ou --session (voir --aide)"
                .into(),
        );
    }
    // polylignes des courbes (vide pour un export statistique seul)
    let data: Vec<CurvePolys> = if curves.is_empty() {
        Vec::new()
    } else {
        // nombre d'échantillons : assez fin pour le PNG demandé
        let n_fn = match &o.png {
            Some(_) => ((o.page.width_mm * o.px_par_mm).round() as usize).clamp(256, 20000),
            None => 1600,
        };
        let mut data = Vec::with_capacity(curves.len());
        for c in &curves {
            let polys = curve::sample_curve(c, &o.page, n_fn, 6000)?;
            data.push((c.clone(), apply_style(polys, c.style)));
        }
        data
    };

    let points = o
        .points
        .iter()
        .map(|&(curve, x)| PointAnnot { curve, x })
        .collect::<Vec<_>>();
    let tangentes = o
        .tangentes
        .iter()
        .map(|&(curve, x)| TangenteAnnot { curve, x })
        .collect::<Vec<_>>();
    let integrales = o
        .aires
        .iter()
        .map(|&(curve, a, b)| IntegraleAnnot { curve, a, b })
        .collect::<Vec<_>>();
    let annots = annot::draw_list(&o.page, &curves, &points, &tangentes, &integrales);

    if let Some(svg) = &o.svg {
        let n = render::write_svg_file(
            &o.page,
            &data,
            &annots,
            &[],
            &o.serie1,
            &o.boites,
            &o.suites,
            &o.serie2,
            &o.arbres,
            &o.simus,
            &o.schema,
            &PathBuf::from(svg),
        )
        .map_err(|e| format!("écriture de {svg} : {e}"))?;
        println!("✔ {svg} écrit ({} octets)", n);
    } else if o.png.is_none() {
        let n = render::write_svg_file(
            &o.page,
            &data,
            &annots,
            &[],
            &o.serie1,
            &o.boites,
            &o.suites,
            &o.serie2,
            &o.arbres,
            &o.simus,
            &o.schema,
            &PathBuf::from("sinequanon.svg"),
        )
        .map_err(|e| format!("écriture de sinequanon.svg : {e}"))?;
        println!("✔ sinequanon.svg écrit ({} octets)", n);
    }
    if let Some(png) = &o.png {
        render::write_png_file(
            &o.page,
            &data,
            &annots,
            &[],
            &o.serie1,
            &o.boites,
            &o.suites,
            &o.serie2,
            &o.arbres,
            &o.simus,
            &o.schema,
            o.px_par_mm,
            &PathBuf::from(png),
        )
        .map_err(|e| format!("écriture de {png} : {e}"))?;
        println!("✔ {png} écrit");
    }
    if let Some(bmp) = &o.bmp {
        render::write_bmp_file(
            &o.page,
            &data,
            &annots,
            &[],
            &o.serie1,
            &o.boites,
            &o.suites,
            &o.serie2,
            &o.arbres,
            &o.simus,
            &o.schema,
            o.px_par_mm,
            &PathBuf::from(bmp),
        )
        .map_err(|e| format!("écriture de {bmp} : {e}"))?;
        println!("✔ {bmp} écrit");
    }
    if let Some(gif) = &o.gif {
        render::write_gif_file(
            &o.page,
            &data,
            &annots,
            &[],
            &o.serie1,
            &o.boites,
            &o.suites,
            &o.serie2,
            &o.arbres,
            &o.simus,
            &o.schema,
            o.px_par_mm,
            &PathBuf::from(gif),
        )
        .map_err(|e| format!("écriture de {gif} : {e}"))?;
        println!("✔ {gif} écrit");
    }
    if let Some(eps) = &o.eps {
        render::write_eps_file(
            &o.page,
            &data,
            &annots,
            &[],
            &o.serie1,
            &o.boites,
            &o.suites,
            &o.serie2,
            &o.arbres,
            &o.simus,
            &o.schema,
            o.px_par_mm,
            &PathBuf::from(eps),
        )
        .map_err(|e| format!("écriture de {eps} : {e}"))?;
        println!("✔ {eps} écrit");
    }
    // impression (v1.3) : PDF A4 posé sous les marges des préférences
    if let Some(pdf) = &o.pdf {
        let prefs =
            preferences::Preferences::charger().unwrap_or_else(preferences::Preferences::standard);
        let (mg, mh) = prefs.marges();
        pdf::write_pdf_file(
            &o.page,
            &data,
            &annots,
            &[],
            &o.serie1,
            &o.boites,
            &o.suites,
            &o.serie2,
            &o.arbres,
            &o.simus,
            &o.schema,
            o.px_par_mm,
            mg,
            mh,
            prefs.portrait(),
            &PathBuf::from(pdf),
        )
        .map_err(|e| format!("écriture de {pdf} : {e}"))?;
        println!("✔ {pdf} écrit");
    }
    Ok(())
}
