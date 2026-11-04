//! Interface graphique (egui/eframe) — feuille quadrillée 5 × 5 mm,
//! liste de courbes, repère paramétrable, annotations (points, tangentes,
//! intégrales hachurées) et export SVG/PNG.

use crate::annot::{self, draw_list, AnnotDraw, IntegraleAnnot, PointAnnot, TangenteAnnot};
use crate::curve::{apply_style, eval_fonction, sample_curve, Curve, CurveKind, KindId, PALETTE};
use crate::grid::Page;
use crate::i18n::{self, Lang};
use crate::render::{self, CurvePolys};
use crate::session::{self, Session};
use crate::simu::{Graphique, Modele, Simulation};
use crate::solve;
use crate::stats::{
    self, Arbre, BoitesMultiples, BrancheArbre, Donnees, Loi, Piece as PieceStats, Reg, Role,
    Style, Visualiser,
};
use crate::tableau::{self as tab, Piece, Tableau};
use eframe::egui::{
    self, Align2, Color32, ComboBox, DragValue, FontId, Pos2, Rect, RichText, Sense, Stroke,
    TextEdit, Ui, Vec2,
};
use std::path::PathBuf;
use std::time::Instant;

/// Résultat d'échantillonnage d'une courbe : polylignes (mm) ou erreur.
pub type CurveResult = Result<Vec<Vec<(f64, f64)>>, String>;

/// Application Sine qua non (Rust).
pub struct SqApp {
    pub page: Page,
    pub curves: Vec<Curve>,
    pub selected: Option<usize>,
    /// langue de l'interface
    pub lang: Lang,
    /// points placés sur les courbes
    pub points: Vec<PointAnnot>,
    /// tangentes (fonctions)
    pub tangentes: Vec<TangenteAnnot>,
    /// intégrales hachurées (fonctions)
    pub integrales: Vec<IntegraleAnnot>,
    /// zoom d'affichage (1 = page entière visible)
    pub zoom: f32,
    /// décalage de la page dans le panneau central, en px
    pub view_offset: Vec2,
    /// résolution d'export PNG (px par mm)
    pub px_per_mm: f32,
    pub status: Option<(String, Instant)>,
    pub dirty: bool,
    /// polylignes calculées (mm), alignées sur `curves`
    pub poly: Vec<CurveResult>,
    /// annotations prêtes à dessiner (recalculées à chaque frame)
    pub annots: Vec<AnnotDraw>,
    /// tableaux de signes et de variations (v0.3+)
    pub tableaux: Vec<Tableau>,
    /// séries statistiques à une variable (v0.4+)
    pub stats: Vec<crate::stats::Serie1>,
    /// boîtes à moustaches multiples (v0.4+)
    pub boites: Vec<crate::stats::BoitesMultiples>,
    /// suites numériques (v0.6+)
    pub suites: Vec<crate::stats::Suite>,
    /// fenêtre d'édition ouverte : index de la suite
    pub suite_sel: Option<usize>,
    /// séries statistiques à 2 variables (v0.8+)
    pub serie2: Vec<crate::stats::Serie2>,
    /// fenêtre d'édition ouverte : index de la série à 2 variables
    pub serie2_sel: Option<usize>,
    /// arbres de probabilités (v0.9+)
    pub arbres: Vec<Arbre>,
    /// fenêtre d'édition ouverte : index de l'arbre
    pub arbre_sel: Option<usize>,
    /// simulations statistiques (v1.0+)
    pub simus: Vec<Simulation>,
    /// fenêtre d'édition ouverte : index de la simulation
    pub simu_sel: Option<usize>,
    /// fenêtre Calculs (v1.1) ouverte : solveur + extrema
    pub calculs_ouvert: bool,
    /// courbe de gauche du solveur (index d'une Fonction)
    pub calc_f1: Option<usize>,
    /// courbe de droite (Some = Fonction, None = Ox)
    pub calc_f2: Option<usize>,
    /// intervalle d'étude du solveur
    pub calc_a: f64,
    pub calc_b: f64,
    /// marquer les solutions / extrema sur la feuille
    pub calc_marquer: bool,
    /// solutions affichées dans la fenêtre : (x, y)
    pub calc_solutions: Vec<(f64, f64)>,
    /// extrema locaux affichés : (x, y, minimum ?)
    pub calc_extrema: Vec<(f64, f64, bool)>,
    /// marques posées par le solveur sur la feuille (retirées à la
    /// prochaine recherche, comme un recalcul de l'original)
    pub solve_marques: Vec<PointAnnot>,
    /// fenêtre d'édition ouverte : index de la série statistique
    pub stat_sel: Option<usize>,
    /// volet actif de la fenêtre statistique (0..=3)
    pub stat_volet: usize,
    /// fenêtre d'édition ouverte : index du tableau
    pub tab_sel: Option<usize>,
    /// fenêtre de génération depuis une courbe (état)
    pub gen: Option<GenEtat>,
}

/// État de la fenêtre « générer un tableau depuis une courbe ».
#[derive(Debug, Clone, Copy)]
pub struct GenEtat {
    /// courbe (type fonction) choisie
    pub curve: usize,
    /// bornes de l'intervalle d'étude
    pub a: f64,
    pub b: f64,
}

impl SqApp {
    pub fn new(lang_flag: Option<Lang>) -> Self {
        let (
            page,
            curves,
            saved_lang,
            points,
            tangentes,
            integrales,
            tableaux,
            stats,
            boites,
            suites,
        ) = match session::load() {
            Some(s) => (
                s.page,
                s.curves,
                Some(s.lang),
                s.points,
                s.tangentes,
                s.integrales,
                s.tableaux,
                s.stats,
                s.boites,
                s.suites,
            ),
            None => (
                Page::default(),
                vec![
                    Curve::fonction("sin(x)", PALETTE[0]),
                    Curve::fonction("x^2/4 - 2", PALETTE[1]),
                ],
                None,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        };
        // le drapeau --lang prime sur la session ; sinon français
        let lang = lang_flag.or(saved_lang).unwrap_or(Lang::Fr);
        let selected = (!curves.is_empty()).then_some(0usize);
        Self {
            page,
            curves,
            selected,
            lang,
            points,
            tangentes,
            integrales,
            zoom: 1.0,
            view_offset: Vec2::ZERO,
            px_per_mm: 4.0,
            status: None,
            dirty: false,
            poly: Vec::new(),
            annots: Vec::new(),
            tableaux,
            stats,
            boites,
            suites,
            suite_sel: None,
            serie2: Vec::new(),
            serie2_sel: None,
            arbres: Vec::new(),
            arbre_sel: None,
            simus: Vec::new(),
            simu_sel: None,
            calculs_ouvert: false,
            calc_f1: None,
            calc_f2: None,
            calc_a: 0.0,
            calc_b: 0.0,
            calc_marquer: true,
            calc_solutions: Vec::new(),
            calc_extrema: Vec::new(),
            solve_marques: Vec::new(),
            stat_sel: None,
            stat_volet: 0,
            tab_sel: None,
            gen: None,
        }
    }

    fn set_status(&mut self, msg: String) {
        self.status = Some((msg, Instant::now()));
    }

    /// Recalcule toutes les polylignes et les annotations (à chaque frame).
    fn refresh(&mut self, cols: usize) {
        let mut poly = Vec::with_capacity(self.curves.len());
        for c in &self.curves {
            poly.push(sample_curve(c, &self.page, cols, 4000).map(|ps| apply_style(ps, c.style)));
        }
        self.poly = poly;
        self.annots = draw_list(
            &self.page,
            &self.curves,
            &self.points,
            &self.tangentes,
            &self.integrales,
        );
    }

    /// Données prêtes pour l'export (courbes visibles + polylignes valides).
    fn export_data(&self) -> Vec<CurvePolys> {
        self.curves
            .iter()
            .zip(&self.poly)
            .filter_map(|(c, r)| {
                r.as_ref()
                    .ok()
                    .filter(|_| c.visible)
                    .map(|polys| (c.clone(), polys.clone()))
            })
            .collect()
    }

    fn panneau_lateral(&mut self, ctx: &egui::Context) {
        let t = i18n::table(self.lang);
        egui::SidePanel::left("outils")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading("Sine qua non");
                            ui.small(format!("v{}", env!("CARGO_PKG_VERSION")));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(t.sous_titre).small().weak());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ComboBox::from_label(t.langue)
                                        .selected_text(self.lang.label())
                                        .width(72.0)
                                        .show_ui(ui, |ui| {
                                            for l in Lang::ALL {
                                                if ui
                                                    .selectable_label(self.lang == l, l.label())
                                                    .clicked()
                                                    && self.lang != l
                                                {
                                                    self.lang = l;
                                                    self.dirty = true;
                                                }
                                            }
                                        });
                                },
                            );
                        });
                        ui.separator();
                        self.section_liste(ui);
                        ui.separator();
                        self.section_proprietes(ui);
                        ui.separator();
                        self.section_annotations(ui);
                        ui.separator();
                        self.section_tableaux(ui);
                        ui.separator();
                        self.section_statistiques(ui);
                        ui.separator();
                        self.section_suites(ui);
                        ui.separator();
                        self.section_serie2(ui);
                        ui.separator();
                        self.section_arbres(ui);
                        ui.separator();
                        self.section_simulations(ui);
                        ui.separator();
                        self.section_calculs(ui);
                        ui.separator();
                        self.section_repere(ui);
                        ui.separator();
                        self.section_export(ui);
                    });
            });
    }

    fn section_liste(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.horizontal(|ui| {
            ui.strong(t.courbes);
            ui.menu_button(t.ajouter, |ui| {
                let n = self.curves.len();
                let color = PALETTE[n % PALETTE.len()];
                if ui.button(t.m_fonction).clicked() {
                    self.curves.push(Curve::fonction("sin(x)", color));
                    self.selected = Some(self.curves.len() - 1);
                    self.dirty = true;
                    ui.close_menu();
                }
                if ui.button(t.m_param).clicked() {
                    self.curves.push(Curve::with_kind(
                        CurveKind::Parametree {
                            xt: "cos(t)".into(),
                            yt: "sin(t)".into(),
                            tmin: -2.0 * std::f64::consts::PI,
                            tmax: 2.0 * std::f64::consts::PI,
                        },
                        color,
                    ));
                    self.selected = Some(self.curves.len() - 1);
                    self.dirty = true;
                    ui.close_menu();
                }
                if ui.button(t.m_polaire).clicked() {
                    self.curves.push(Curve::with_kind(
                        CurveKind::Polaire {
                            r: "1+cos(theta)".into(),
                            tmin: -2.0 * std::f64::consts::PI,
                            tmax: 2.0 * std::f64::consts::PI,
                        },
                        color,
                    ));
                    self.selected = Some(self.curves.len() - 1);
                    self.dirty = true;
                    ui.close_menu();
                }
            });
            if ui.button(t.vider).clicked() {
                self.curves.clear();
                self.selected = None;
                self.dirty = true;
            }
        });
        let mut to_delete: Option<usize> = None;
        let mut to_toggle: Option<usize> = None;
        for i in 0..self.curves.len() {
            let curve = &self.curves[i];
            let err = matches!(self.poly.get(i), Some(Err(_)));
            ui.horizontal(|ui| {
                let [r, g, b] = curve.color;
                let swatch = Color32::from_rgb(r, g, b);
                let vis_text = if curve.visible { "◉" } else { "○" };
                if ui
                    .add(
                        egui::Label::new(RichText::new(vis_text).color(swatch).size(16.0))
                            .sense(Sense::click()),
                    )
                    .clicked()
                {
                    to_toggle = Some(i);
                }
                let label = RichText::new(curve.label()).small();
                let label = if curve.visible { label } else { label.weak() };
                let label = if err {
                    label.color(Color32::RED)
                } else {
                    label
                };
                if ui
                    .add(egui::Label::new(label).sense(Sense::click()))
                    .clicked()
                {
                    self.selected = Some(i);
                }
                if ui.small_button("✕").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_toggle {
            if let Some(c) = self.curves.get_mut(i) {
                c.visible = !c.visible;
                self.dirty = true;
            }
        }
        if let Some(i) = to_delete {
            self.curves.remove(i);
            let _ = self.poly.remove(i);
            self.selected = self.selected.and_then(|s| match s.cmp(&i) {
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => Some(s - 1),
                std::cmp::Ordering::Less => Some(s),
            });
            self.dirty = true;
        }
    }

    fn section_proprietes(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.strong(t.proprietes);
        let Some(sel) = self.selected else {
            ui.label(RichText::new(t.aucune).weak().small());
            return;
        };
        let Some(curve) = self.curves.get_mut(sel) else {
            self.selected = None;
            return;
        };

        // nature
        let kind_id = curve.kind.kind_id();
        ComboBox::from_label(t.nature)
            .selected_text(match kind_id {
                KindId::Fonction => t.l_fonction,
                KindId::Parametree => t.l_param,
                KindId::Polaire => t.l_polaire,
            })
            .show_ui(ui, |ui| {
                for (id, name) in [
                    (KindId::Fonction, t.l_fonction),
                    (KindId::Parametree, t.l_param),
                    (KindId::Polaire, t.l_polaire),
                ] {
                    if ui.selectable_label(kind_id == id, name).clicked() && kind_id != id {
                        curve.kind = curve.kind.convert(id);
                        self.dirty = true;
                    }
                }
            });

        // expressions
        match &mut curve.kind {
            CurveKind::Fonction { expr } => {
                ui.horizontal(|ui| {
                    ui.label("y =");
                    if ui
                        .add(TextEdit::singleline(expr).desired_width(f32::INFINITY))
                        .changed()
                    {
                        self.dirty = true;
                    }
                });
            }
            CurveKind::Parametree { xt, yt, tmin, tmax } => {
                ui.horizontal(|ui| {
                    ui.label("x(t) =");
                    if ui
                        .add(TextEdit::singleline(xt).desired_width(f32::INFINITY))
                        .changed()
                    {
                        self.dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("y(t) =");
                    if ui
                        .add(TextEdit::singleline(yt).desired_width(f32::INFINITY))
                        .changed()
                    {
                        self.dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t.t_de);
                    if ui.add(DragValue::new(tmin).speed(0.1)).changed() {
                        self.dirty = true;
                    }
                    ui.label(t.a);
                    if ui.add(DragValue::new(tmax).speed(0.1)).changed() {
                        self.dirty = true;
                    }
                });
            }
            CurveKind::Polaire { r, tmin, tmax } => {
                ui.horizontal(|ui| {
                    ui.label("r(θ) =");
                    if ui
                        .add(TextEdit::singleline(r).desired_width(f32::INFINITY))
                        .changed()
                    {
                        self.dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t.theta_de);
                    if ui.add(DragValue::new(tmin).speed(0.1)).changed() {
                        self.dirty = true;
                    }
                    ui.label(t.a);
                    if ui.add(DragValue::new(tmax).speed(0.1)).changed() {
                        self.dirty = true;
                    }
                });
            }
        }

        // style
        ui.horizontal(|ui| {
            ui.label(t.couleur);
            let old = Color32::from_rgb(curve.color[0], curve.color[1], curve.color[2]);
            let mut c = old;
            ui.color_edit_button_srgba(&mut c);
            if c != old {
                curve.color = [c.r(), c.g(), c.b()];
                self.dirty = true;
            }
            ui.label(t.epaisseur);
            if ui
                .add(egui::Slider::new(&mut curve.thickness, 0.5..=6.0))
                .changed()
            {
                self.dirty = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(t.style);
            for (st, name) in [
                (crate::curve::LineStyle::Continu, t.continu),
                (crate::curve::LineStyle::Tirets, t.tirets),
                (crate::curve::LineStyle::Pointilles, t.pointilles),
            ] {
                if ui.selectable_label(curve.style == st, name).clicked() && curve.style != st {
                    curve.style = st;
                    self.dirty = true;
                }
            }
        });

        // erreur d'évaluation
        if let Some(Err(msg)) = self.poly.get(sel) {
            ui.label(
                RichText::new(format!("⚠ {msg}"))
                    .small()
                    .color(Color32::RED),
            );
        }
    }

    /// Panneau des annotations : points, tangentes, intégrales hachurées.
    fn section_annotations(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.strong(t.annotations);
        let nc = self.curves.len().saturating_sub(1) as i64;

        // --- points ---
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.points).strong().small());
            if !self.curves.is_empty() && ui.small_button(t.aj_point).clicked() {
                self.points.push(PointAnnot {
                    curve: self.selected.unwrap_or(0),
                    x: 1.0,
                });
                self.dirty = true;
            }
        });
        let mut del_point: Option<usize> = None;
        for i in 0..self.points.len() {
            let p = &mut self.points[i];
            ui.horizontal(|ui| {
                ui.label(t.courbe);
                ui.add(DragValue::new(&mut p.curve).range(0..=nc.max(0)));
                ui.label("x");
                if ui.add(DragValue::new(&mut p.x).speed(0.1)).changed() {
                    self.dirty = true;
                }
                if let Some((ux, uy)) = annot::point_value(&self.curves, p) {
                    ui.small(format!("({ux:.3} ; {uy:.3})"));
                }
                if ui.small_button("✕").clicked() {
                    del_point = Some(i);
                }
            });
        }
        if let Some(i) = del_point {
            self.points.remove(i);
            self.dirty = true;
        }

        // --- tangentes ---
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.tangentes).strong().small());
            if !self.curves.is_empty() && ui.small_button(t.aj_tangente).clicked() {
                self.tangentes.push(TangenteAnnot {
                    curve: self.selected.unwrap_or(0),
                    x: 1.0,
                });
                self.dirty = true;
            }
        });
        let mut del_tan: Option<usize> = None;
        for i in 0..self.tangentes.len() {
            let tg = &mut self.tangentes[i];
            ui.horizontal(|ui| {
                ui.label(t.courbe);
                ui.add(DragValue::new(&mut tg.curve).range(0..=nc.max(0)));
                ui.label("x");
                if ui.add(DragValue::new(&mut tg.x).speed(0.1)).changed() {
                    self.dirty = true;
                }
                if let Some((d, b)) = annot::tangente_value(&self.curves, tg) {
                    let signe = if b >= 0.0 { "+" } else { "-" };
                    ui.small(format!("y = {d:.3}x {signe} {}", b.abs()));
                }
                if ui.small_button("✕").clicked() {
                    del_tan = Some(i);
                }
            });
        }
        if let Some(i) = del_tan {
            self.tangentes.remove(i);
            self.dirty = true;
        }

        // --- intégrales ---
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.integrales).strong().small());
            if !self.curves.is_empty() && ui.small_button(t.aj_integrale).clicked() {
                self.integrales.push(IntegraleAnnot {
                    curve: self.selected.unwrap_or(0),
                    a: 0.0,
                    b: 2.0,
                });
                self.dirty = true;
            }
        });
        let mut del_int: Option<usize> = None;
        for i in 0..self.integrales.len() {
            let it = &mut self.integrales[i];
            ui.horizontal(|ui| {
                ui.label(t.courbe);
                ui.add(DragValue::new(&mut it.curve).range(0..=nc.max(0)));
                ui.label(t.a);
                if ui.add(DragValue::new(&mut it.a).speed(0.1)).changed() {
                    self.dirty = true;
                }
                ui.label("→");
                if ui.add(DragValue::new(&mut it.b).speed(0.1)).changed() {
                    self.dirty = true;
                }
                if let Some(v) = annot::integrale_value(&self.curves, it) {
                    ui.small(format!("∫ ≈ {v:.4}"));
                }
                if ui.small_button("✕").clicked() {
                    del_int = Some(i);
                }
            });
        }
        if let Some(i) = del_int {
            self.integrales.remove(i);
            self.dirty = true;
        }
    }

    /// Section « Tableaux de signes et de variations » du panneau latéral.
    fn section_tableaux(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.horizontal(|ui| {
            ui.strong(t.tableaux);
            if ui.button(t.m_tableau).clicked() {
                // modèle : étiquette + 3 colonnes vides, sur 2 rangées
                let row = tab::rangee_vers_code(&tab::Rangee {
                    cases: vec![tab::Case::default(); 3],
                    ..Default::default()
                });
                let code = format!("{row}\n{row}");
                let rows = tab::parse(&code).map(|t| t.rows).unwrap_or_default();
                self.tableaux.push(Tableau {
                    code,
                    rows,
                    ..Default::default()
                });
                self.tab_sel = Some(self.tableaux.len() - 1);
                self.dirty = true;
            }
            if ui.button(t.depuis_courbe).clicked() {
                let fns = self.courbes_fonctions();
                if let Some(first) = fns.first() {
                    let (x0, x1) = self.page.x_range();
                    self.gen = Some(GenEtat {
                        curve: *first,
                        a: x0,
                        b: x1,
                    });
                } else {
                    self.set_status(format!("✖ {}", t.aucune));
                }
            }
        });
        let mut to_delete: Option<usize> = None;
        let mut to_toggle: Option<usize> = None;
        for (i, tabl) in self.tableaux.iter().enumerate() {
            let dess = tabl.dessiner;
            ui.horizontal(|ui| {
                let vis = if dess { "◉" } else { "○" };
                if ui
                    .add(egui::Label::new(RichText::new(vis).size(16.0)).sense(Sense::click()))
                    .clicked()
                {
                    to_toggle = Some(i);
                }
                if ui
                    .add(
                        egui::Label::new(RichText::new(format!("Tableau {}", i + 1)).small())
                            .sense(Sense::click()),
                    )
                    .clicked()
                {
                    self.tab_sel = Some(i);
                }
                if ui.small_button("✕").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_toggle {
            if let Some(x) = self.tableaux.get_mut(i) {
                x.dessiner = !x.dessiner;
                self.dirty = true;
            }
        }
        if let Some(i) = to_delete {
            self.tableaux.remove(i);
            if self.tab_sel == Some(i) {
                self.tab_sel = None;
            }
            self.dirty = true;
        }
    }

    /// Fenêtre d'édition d'un tableau (code, boutons, exemples, couleurs).
    fn fenetre_tableau(&mut self, ctx: &egui::Context) {
        let t = i18n::table(self.lang);
        let Some(idx) = self.tab_sel else { return };
        if self.tableaux.get(idx).is_none() {
            self.tab_sel = None;
            return;
        }
        let mut ouverte = true;
        egui::Window::new(format!("{} — {}", t.tableaux, idx + 1))
            .open(&mut ouverte)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                let tabl = &mut self.tableaux[idx];
                ui.label(t.code_tableau);
                let re = ui.add(
                    TextEdit::multiline(&mut tabl.code)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(6)
                        .desired_width(560.0),
                );
                if re.changed() {
                    if let Ok(nt) = tab::parse(&tabl.code) {
                        tabl.rows = nt.rows;
                    }
                    self.dirty = true;
                }
                match tab::parse(&tabl.code) {
                    Ok(ta) => {
                        let (w, h) = tab::dimensions(&ta);
                        ui.label(RichText::new(format!("✔ {:.0} × {:.0} mm", w, h)).small());
                    }
                    Err(e) => {
                        ui.label(RichText::new(format!("✖ {e}")).color(Color32::RED).small());
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(t.plus_lig).clicked() {
                        let n = tabl.rows.first().map(|r| r.cases.len()).unwrap_or(3).max(2);
                        let row = tab::rangee_vers_code(&tab::Rangee {
                            cases: vec![tab::Case::default(); n],
                            ..Default::default()
                        });
                        let mut c = tabl.code.trim_end().to_string();
                        if !c.is_empty() {
                            c.push('\n');
                        }
                        c.push_str(&row);
                        Self::maj_code(tabl, c);
                        self.dirty = true;
                    }
                    if ui.button(t.moins_lig).clicked() {
                        let c = tabl.code.trim_end().to_string();
                        let c = match c.rfind('\n') {
                            Some(pos) => c[..pos].to_string(),
                            None => String::new(),
                        };
                        Self::maj_code(tabl, c);
                        self.dirty = true;
                    }
                    if ui.button(t.plus_col).clicked() {
                        if let Ok(c) = tab::ajuster_colonnes(&tabl.code, 1) {
                            Self::maj_code(tabl, c);
                            self.dirty = true;
                        }
                    }
                    if ui.button(t.moins_col).clicked() {
                        if let Ok(c) = tab::ajuster_colonnes(&tabl.code, -1) {
                            Self::maj_code(tabl, c);
                            self.dirty = true;
                        }
                    }
                    if ui.button(t.egaliser).clicked() {
                        if let Ok(c) = tab::egaliser(&tabl.code) {
                            Self::maj_code(tabl, c);
                            self.dirty = true;
                        }
                    }
                    if ui.button(t.effacer).clicked() {
                        Self::maj_code(tabl, String::new());
                        self.dirty = true;
                    }
                });
                // insertion de fragments à la fin de la dernière ligne
                ui.horizontal_wrapped(|ui| {
                    ui.label(t.inserer);
                    for (lbl, tok) in [
                        ("+∞", "+\\infty"),
                        ("−∞", "-\\infty"),
                        ("√", "\\sqrt{}"),
                        ("a/b", "\\frac{}{}"),
                        ("‖", "||"),
                        ("|", "|"),
                        ("|0", "|0"),
                        ("↗", "/:>"),
                        ("↘", "\\:>"),
                        ("→", "_:>"),
                        ("//", "//"),
                    ] {
                        if ui.small_button(lbl).clicked() {
                            let mut c = tabl.code.trim_end().to_string();
                            c.push_str(tok);
                            Self::maj_code(tabl, c);
                            self.dirty = true;
                        }
                    }
                });
                // exemples de l'original (Var1..Var8, Sgn, Abs)
                ui.horizontal_wrapped(|ui| {
                    ui.label(t.exemples);
                    for (nom, code) in tab::exemples() {
                        if ui.small_button(nom).clicked() {
                            Self::maj_code(tabl, code.trim().to_string());
                            self.dirty = true;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t.couleur_traits);
                    ui.color_edit_button_srgb(&mut tabl.couleur_traits);
                    ui.label(t.couleur_textes);
                    ui.color_edit_button_srgb(&mut tabl.couleur_textes);
                });
                ui.horizontal(|ui| {
                    ui.label(t.position_t);
                    ui.add(
                        DragValue::new(&mut tabl.x_mm)
                            .range(-50.0..=400.0)
                            .suffix(" x"),
                    );
                    ui.add(
                        DragValue::new(&mut tabl.y_mm)
                            .range(-50.0..=400.0)
                            .suffix(" y"),
                    );
                    ui.checkbox(&mut tabl.dessiner, t.dessiner_t)
                        .changed()
                        .then(|| self.dirty = true);
                });
            });
        if !ouverte {
            self.tab_sel = None;
            self.dirty = true;
        }
    }

    /// Remplace le code d'un tableau et ré-analyse ses rangées.
    fn maj_code(tabl: &mut Tableau, code: String) {
        tabl.code = code;
        if let Ok(nt) = tab::parse(&tabl.code) {
            tabl.rows = nt.rows;
        }
    }

    /// Indices des courbes de type fonction (évaluables en x).
    fn courbes_fonctions(&self) -> Vec<usize> {
        self.curves
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.kind, CurveKind::Fonction { .. }))
            .map(|(i, _)| i)
            .collect()
    }

    /// Fenêtre « générer un tableau depuis une courbe » : zéros de f′
    /// (bissection), signes, valeurs ; crée un Tableau éditable.
    fn fenetre_genere(&mut self, ctx: &egui::Context) {
        if self.gen.is_none() {
            return;
        }
        let t = i18n::table(self.lang);
        let mut ouverte = true;
        let mut faire: Option<(usize, f64, f64)> = None;
        egui::Window::new(t.depuis_courbe)
            .open(&mut ouverte)
            .resizable(false)
            .show(ctx, |ui| {
                let fns = self.courbes_fonctions();
                if fns.is_empty() {
                    ui.label(RichText::new(t.aucune).weak());
                    return;
                }
                let Some(g) = self.gen.as_mut() else { return };
                if !fns.contains(&g.curve) {
                    g.curve = fns[0];
                }
                ComboBox::from_label(t.courbe_l)
                    .selected_text(self.curves[g.curve].label())
                    .show_ui(ui, |ui| {
                        for i in fns {
                            ui.selectable_value(&mut g.curve, i, self.curves[i].label());
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("a :");
                    ui.add(DragValue::new(&mut g.a).speed(0.25));
                    ui.label("b :");
                    ui.add(DragValue::new(&mut g.b).speed(0.25));
                });
                if ui.button(t.generer).clicked() {
                    faire = Some((g.curve, g.a, g.b));
                }
            });
        if let Some((ci, a, b)) = faire {
            let expr = match &self.curves[ci].kind {
                CurveKind::Fonction { expr } => expr.clone(),
                _ => String::new(),
            };
            match crate::tabgen::genere(&expr, a, b, self.lang == Lang::Fr) {
                Ok(code) => {
                    let rows = tab::parse(&code).map(|x| x.rows).unwrap_or_default();
                    self.tableaux.push(Tableau {
                        code,
                        rows,
                        ..Default::default()
                    });
                    self.tab_sel = Some(self.tableaux.len() - 1);
                    self.dirty = true;
                    self.gen = None;
                }
                Err(e) => self.set_status(format!("✖ {e}")),
            }
        }
        if !ouverte {
            self.gen = None;
        }
    }

    /// Section « Suites » du panneau latéral (v0.7) : ajout, liste
    /// cliquant-ouvrant l'éditeur, masquage et suppression.
    fn section_suites(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.horizontal(|ui| {
            ui.strong(t.suites);
            if ui.button(t.st_aj_suite).clicked() {
                self.suites.push(stats::Suite {
                    titre: format!("Suite {}", self.suites.len() + 1),
                    def: stats::SuiteDef::Lineaire { f: "2x+1".into() },
                    dessiner: true,
                    n0: 0,
                    u0: 0.0,
                });
                self.suite_sel = Some(self.suites.len() - 1);
                self.dirty = true;
            }
        });
        let mut to_delete: Option<usize> = None;
        let mut to_toggle: Option<usize> = None;
        for (i, s) in self.suites.iter().enumerate() {
            let label = if s.titre.is_empty() {
                format!("{} {}", t.suites, i + 1)
            } else {
                s.titre.lines().next().unwrap_or("").to_string()
            };
            ui.horizontal(|ui| {
                let vis = if s.dessiner { "\u{25c9}" } else { "\u{25cb}" };
                if ui
                    .add(egui::Label::new(RichText::new(vis).size(16.0)).sense(Sense::click()))
                    .clicked()
                {
                    to_toggle = Some(i);
                }
                let caption = match &s.def {
                    stats::SuiteDef::Lineaire { f } => format!("{label} — u_n = {f}"),
                    stats::SuiteDef::Recurrence { f } => format!("{label} — u_(n+1) = {f}"),
                };
                if ui
                    .add(egui::Label::new(RichText::new(caption).small()).sense(Sense::click()))
                    .clicked()
                {
                    self.suite_sel = Some(i);
                }
                if ui.small_button("\u{2715}").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_toggle {
            if let Some(s) = self.suites.get_mut(i) {
                s.dessiner = !s.dessiner;
                self.dirty = true;
            }
        }
        if let Some(i) = to_delete {
            self.suites.remove(i);
            self.suite_sel = None;
            self.dirty = true;
        }
        if self.suites.is_empty() {
            ui.label(RichText::new(t.aucune).weak().small());
        }
    }

    /// Fenêtre d'édition d'une suite (v0.7) : titre, type, formule
    /// (variable x comme l'original, n/u restent acceptés), premier
    /// indice et premier terme u₀.
    fn fenetre_suite(&mut self, ctx: &egui::Context) {
        let t = i18n::table(self.lang);
        let Some(idx) = self.suite_sel else { return };
        if self.suites.get(idx).is_none() {
            self.suite_sel = None;
            return;
        }
        let mut ouverte = true;
        egui::Window::new(format!("{} — {}", t.suites, idx + 1))
            .open(&mut ouverte)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                let dirty = &mut self.dirty;
                let s = &mut self.suites[idx];
                ui.horizontal(|ui| {
                    ui.label(t.stat_titre);
                    if ui
                        .add(TextEdit::singleline(&mut s.titre).desired_width(200.0))
                        .changed()
                    {
                        *dirty = true;
                    }
                    ui.checkbox(&mut s.dessiner, t.stat_dessiner);
                });
                ui.horizontal(|ui| {
                    ui.label(t.suite_type);
                    if ui
                        .selectable_label(
                            matches!(s.def, stats::SuiteDef::Lineaire { .. }),
                            t.suite_expl,
                        )
                        .clicked()
                        && !matches!(s.def, stats::SuiteDef::Lineaire { .. })
                    {
                        s.def = stats::SuiteDef::Lineaire { f: "2x+1".into() };
                        *dirty = true;
                    }
                    if ui
                        .selectable_label(
                            matches!(s.def, stats::SuiteDef::Recurrence { .. }),
                            t.suite_rec,
                        )
                        .clicked()
                        && !matches!(s.def, stats::SuiteDef::Recurrence { .. })
                    {
                        s.def = stats::SuiteDef::Recurrence { f: "2x+1".into() };
                        *dirty = true;
                    }
                });
                match &mut s.def {
                    stats::SuiteDef::Lineaire { f } | stats::SuiteDef::Recurrence { f } => {
                        ui.horizontal(|ui| {
                            ui.label(t.suite_f);
                            if ui
                                .add(
                                    TextEdit::singleline(f)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(180.0),
                                )
                                .changed()
                            {
                                *dirty = true;
                            }
                        });
                    }
                }
                ui.horizontal(|ui| {
                    ui.label(t.suite_n0);
                    if ui
                        .add(DragValue::new(&mut s.n0).range(0..=1e9 as usize))
                        .changed()
                    {
                        *dirty = true;
                    }
                    if matches!(s.def, stats::SuiteDef::Recurrence { .. }) {
                        ui.label(t.suite_u0);
                        if ui
                            .add(DragValue::new(&mut s.u0).speed(0.1).range(-1e12..=1e12))
                            .changed()
                        {
                            *dirty = true;
                        }
                    }
                });
            });
        if !ouverte {
            self.suite_sel = None;
        }
    }

    /// Section « Séries 2 variables » (v0.8, module statistiques) :
    /// ajout, liste cliquant-ouvrant l'éditeur, masquage, suppression.
    fn section_serie2(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.horizontal(|ui| {
            ui.strong(t.serie2);
            if ui.button(t.st2_aj).clicked() {
                self.serie2.push(stats::Serie2 {
                    titre: format!("Serie2 {}", self.serie2.len() + 1),
                    ..Default::default()
                });
                self.serie2_sel = Some(self.serie2.len() - 1);
                self.dirty = true;
            }
        });
        let mut to_delete: Option<usize> = None;
        let mut to_toggle: Option<usize> = None;
        for (i, s) in self.serie2.iter().enumerate() {
            let label = if s.titre.is_empty() {
                format!("{} {}", t.serie2, i + 1)
            } else {
                s.titre.lines().next().unwrap_or("").to_string()
            };
            ui.horizontal(|ui| {
                let vis = if s.dessiner { "\u{25c9}" } else { "\u{25cb}" };
                if ui
                    .add(egui::Label::new(RichText::new(vis).size(16.0)).sense(Sense::click()))
                    .clicked()
                {
                    to_toggle = Some(i);
                }
                let caption = format!("{label} ({} pt)", s.points.len());
                if ui
                    .add(egui::Label::new(RichText::new(caption).small()).sense(Sense::click()))
                    .clicked()
                {
                    self.serie2_sel = Some(i);
                }
                if ui.small_button("\u{2715}").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_toggle {
            if let Some(s) = self.serie2.get_mut(i) {
                s.dessiner = !s.dessiner;
                self.dirty = true;
            }
        }
        if let Some(i) = to_delete {
            self.serie2.remove(i);
            self.serie2_sel = None;
            self.dirty = true;
        }
        if self.serie2.is_empty() {
            ui.label(RichText::new(t.aucune).weak().small());
        }
    }

    /// Fenêtre d'édition d'une série à 2 variables (v0.8) : tableau des
    /// couples (xᵢ, yᵢ, nᵢ), nature de régression, rôle des variables,
    /// sous-nuage de Mayer, équation affichée, panneau Calculs
    /// (moments pondérés, r corrigé).
    fn fenetre_serie2(&mut self, ctx: &egui::Context) {
        let t = i18n::table(self.lang);
        let Some(idx) = self.serie2_sel else { return };
        if self.serie2.get(idx).is_none() {
            self.serie2_sel = None;
            return;
        }
        let mut ouverte = true;
        egui::Window::new(format!("{} — {}", t.serie2, idx + 1))
            .open(&mut ouverte)
            .resizable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                let dirty = &mut self.dirty;
                let s = &mut self.serie2[idx];
                ui.horizontal(|ui| {
                    ui.label(t.stat_titre);
                    let re = ui.add(TextEdit::singleline(&mut s.titre).desired_width(200.0));
                    ui.checkbox(&mut s.dessiner, t.stat_dessiner);
                    if ui.checkbox(&mut s.aff_equation, t.st2_equation).changed() || re.changed() {
                        *dirty = true;
                    }
                });
                // nature de la régression
                let natures = [
                    (Reg::Aucune, t.st2_aucune),
                    (Reg::Lineaire, t.st2_lineaire),
                    (Reg::Logarithmique, t.st2_log),
                    (Reg::Exponentielle, t.st2_expo),
                    (Reg::Puissance, t.st2_puissance),
                    (Reg::Mayer, t.st2_mayer),
                    (Reg::Poly2, t.st2_poly2),
                    (Reg::Poly3, t.st2_poly3),
                    (Reg::Poly4, t.st2_poly4),
                ];
                ui.horizontal(|ui| {
                    ui.label(t.st2_nature);
                    ComboBox::from_id_salt("nature-reg2")
                        .selected_text(natures[s.reg as usize].1)
                        .show_ui(ui, |ui| {
                            for (r, label) in natures {
                                if ui.selectable_label(s.reg == r, label).clicked() {
                                    s.reg = r;
                                    *dirty = true;
                                }
                            }
                        });
                    ui.label(t.st2_role);
                    if ui
                        .selectable_label(s.role == Role::YenX, t.st2_yenx)
                        .clicked()
                        && s.role != Role::YenX
                    {
                        s.role = Role::YenX;
                        *dirty = true;
                    }
                    if ui
                        .selectable_label(s.role == Role::XenY, t.st2_xeny)
                        .clicked()
                        && s.role != Role::XenY
                    {
                        s.role = Role::XenY;
                        *dirty = true;
                    }
                });
                // sous-nuage de Mayer
                if s.reg == Reg::Mayer {
                    let m = s.points.len().max(2);
                    ui.horizontal(|ui| {
                        ui.label(t.st2_mayer_k);
                        if ui
                            .add(DragValue::new(&mut s.mayer_k).range(1..=m.saturating_sub(1)))
                            .changed()
                        {
                            *dirty = true;
                        }
                    });
                }
                // tableau des couples (xᵢ, yᵢ, nᵢ)
                egui::Grid::new("serie2-pts").striped(true).show(ui, |ui| {
                    ui.strong(t.st2_x);
                    ui.strong(t.st2_y);
                    ui.strong(t.st2_n);
                    ui.label("");
                    ui.end_row();
                    let mut to_remove: Option<usize> = None;
                    for (i, pt) in s.points.iter_mut().enumerate() {
                        if ui
                            .add(DragValue::new(&mut pt.x).speed(1.0).range(-1e12..=1e12))
                            .changed()
                        {
                            *dirty = true;
                        }
                        if ui
                            .add(DragValue::new(&mut pt.y).speed(1.0).range(-1e12..=1e12))
                            .changed()
                        {
                            *dirty = true;
                        }
                        if ui
                            .add(DragValue::new(&mut pt.n).speed(1.0).range(0.0..=1e12))
                            .changed()
                        {
                            *dirty = true;
                        }
                        if ui.small_button("\u{2715}").clicked() {
                            to_remove = Some(i);
                        }
                        ui.end_row();
                    }
                    if let Some(i) = to_remove {
                        s.points.remove(i);
                        *dirty = true;
                    }
                });
                if ui.button(t.st_aj_ligne).clicked() {
                    s.points.push(stats::Couple {
                        x: 0.0,
                        y: 0.0,
                        n: 1.0,
                    });
                    *dirty = true;
                }
                ui.separator();
                // panneau Calculs (moments pondérés, r, équation)
                ui.strong(t.stat_calculs);
                match s.moments() {
                    Some(mom) => {
                        egui::Grid::new("calculs-s2").num_columns(2).show(ui, |ui| {
                            let mut rows = vec![
                                (t.st_n, mom.n),
                                ("x\u{203e}", mom.mx),
                                ("y\u{203e}", mom.my),
                                ("\u{3c3}x", mom.sx),
                                ("\u{3c3}y", mom.sy),
                                (t.st2_cov, mom.cov),
                            ];
                            // r linéaire, sur les variables x et y (le
                            // rôle permuté n'intervient pas, manuel p. 54)
                            if let Some(r) = s.correlation() {
                                rows.push((t.st2_r, r));
                            }
                            for (lbl, val) in rows {
                                ui.label(lbl);
                                ui.label(RichText::new(stats::num(val)).strong());
                                ui.end_row();
                            }
                        });
                        if let Some(aj) = s.ajuste() {
                            let eq = aj.equation();
                            let eq = if s.role == Role::XenY {
                                eq.replacen('y', "x", 1)
                            } else {
                                eq
                            };
                            ui.label(RichText::new(eq).strong());
                            // corrélation corrigée des couples transformés
                            // (sans signification pour les polynômes > 1)
                            if let Some(r) = s.r_corrige() {
                                ui.label(format!("{} = {}", t.st2_r_corr, stats::num(r)));
                            }
                        }
                    }
                    None => {
                        ui.weak(t.st_aucun_calcul);
                    }
                }
            });
        if !ouverte {
            self.serie2_sel = None;
        }
    }

    /// Section « Arbres de probabilités » (v0.9, module statistiques) :
    /// ajout (exemple du manuel si vide), liste cliquant-ouvrant
    /// l'éditeur, masquage, suppression.
    fn section_arbres(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.horizontal(|ui| {
            ui.strong(t.arbres);
            if ui.button(t.m_arbre).clicked() {
                if self.arbres.is_empty() {
                    // exemple du manuel comme contenu par défaut
                    self.arbres.push(stats::exemple_arbre());
                } else {
                    self.arbres.push(Arbre::default());
                }
                self.arbre_sel = Some(self.arbres.len() - 1);
                self.dirty = true;
            }
        });
        let mut to_delete: Option<usize> = None;
        let mut to_toggle: Option<usize> = None;
        for (i, a) in self.arbres.iter().enumerate() {
            let label = if a.titre.is_empty() {
                format!("{} {}", t.arbres, i + 1)
            } else {
                a.titre.lines().next().unwrap_or("").to_string()
            };
            ui.horizontal(|ui| {
                let vis = if a.dessiner { "\u{25c9}" } else { "\u{25cb}" };
                if ui
                    .add(egui::Label::new(RichText::new(vis).size(16.0)).sense(Sense::click()))
                    .clicked()
                {
                    to_toggle = Some(i);
                }
                let caption = format!("{label} ({} b)", a.branches.len());
                if ui
                    .add(egui::Label::new(RichText::new(caption).small()).sense(Sense::click()))
                    .clicked()
                {
                    self.arbre_sel = Some(i);
                }
                if ui.small_button("\u{2715}").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_toggle {
            if let Some(a) = self.arbres.get_mut(i) {
                a.dessiner = !a.dessiner;
                self.dirty = true;
            }
        }
        if let Some(i) = to_delete {
            self.arbres.remove(i);
            self.arbre_sel = None;
            self.dirty = true;
        }
        if self.arbres.is_empty() {
            ui.label(RichText::new(t.aucune).weak().small());
        }
    }

    /// Liste des simulations statistiques (v1.0) dans le panneau latéral.
    fn section_simulations(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.horizontal(|ui| {
            ui.strong(t.simulations);
            if ui.button(t.m_simu).clicked() {
                self.simus.push(Simulation::default());
                self.simu_sel = Some(self.simus.len() - 1);
                self.dirty = true;
            }
        });
        let mut to_delete: Option<usize> = None;
        let mut to_toggle: Option<usize> = None;
        for (i, s) in self.simus.iter().enumerate() {
            let label = if s.titre.is_empty() {
                format!("{} {}", t.simulations, i + 1)
            } else {
                s.titre.lines().next().unwrap_or("").to_string()
            };
            ui.horizontal(|ui| {
                let vis = if s.dessiner { "\u{25c9}" } else { "\u{25cb}" };
                if ui
                    .add(egui::Label::new(RichText::new(vis).size(16.0)).sense(Sense::click()))
                    .clicked()
                {
                    to_toggle = Some(i);
                }
                let caption = format!("{label} ({} × {})", s.taille, s.nombre);
                if ui
                    .add(egui::Label::new(RichText::new(caption).small()).sense(Sense::click()))
                    .clicked()
                {
                    self.simu_sel = Some(i);
                }
                if ui.small_button("\u{2715}").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_toggle {
            if let Some(s) = self.simus.get_mut(i) {
                s.dessiner = !s.dessiner;
                self.dirty = true;
            }
        }
        if let Some(i) = to_delete {
            self.simus.remove(i);
            self.simu_sel = None;
            self.dirty = true;
        }
        if self.simus.is_empty() {
            ui.label(RichText::new(t.aucune).weak().small());
        }
    }

    /// Fenêtre d'édition d'une simulation statistique (v1.0) : modèle et
    /// réglages de la loi, graphique, taille / nombre / classes, courbe
    /// théorique, intervalle de fluctuation (Bernoulli) ; bouton Lancer
    /// (nouvelle graine + échelle automatique du repère, comme l'original)
    /// et paramètres de l'ensemble des tirages.
    fn fenetre_simulation(&mut self, ctx: &egui::Context) {
        let t = i18n::table(self.lang);
        let Some(idx) = self.simu_sel else {
            return;
        };
        if self.simus.get(idx).is_none() {
            self.simu_sel = None;
            return;
        }
        let mut ouverte = true;
        egui::Window::new(format!("{} — {}", t.simulations, idx + 1))
            .open(&mut ouverte)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                let dirty = &mut self.dirty;
                let sim = &mut self.simus[idx];
                // titre + visibilité
                ui.horizontal(|ui| {
                    ui.label(t.stat_titre);
                    let re = ui.add(TextEdit::singleline(&mut sim.titre).desired_width(200.0));
                    ui.checkbox(&mut sim.dessiner, t.stat_dessiner);
                    if re.changed() {
                        *dirty = true;
                    }
                });
                // modèle de tirage
                let idx_mod = match sim.modele {
                    Modele::UniformeDiscrete { .. } => 0,
                    Modele::UniformeContinue { .. } => 1,
                    Modele::Bernoulli { .. } => 2,
                    Modele::Binomiale { .. } => 3,
                    Modele::Geometrique { .. } => 4,
                    Modele::Poisson { .. } => 5,
                    Modele::Normale { .. } => 6,
                };
                let modes = [
                    t.sim_uni_disc,
                    t.sim_uni_cont,
                    t.sim_bernoulli,
                    t.sim_binomiale,
                    t.sim_geometrique,
                    t.sim_poisson,
                    t.sim_normale,
                ];
                ComboBox::from_label(t.sim_modele)
                    .selected_text(modes[idx_mod])
                    .show_ui(ui, |ui| {
                        for (j, l) in modes.iter().enumerate() {
                            if ui.selectable_label(j == idx_mod, *l).clicked() && j != idx_mod {
                                sim.modele = match j {
                                    0 => Modele::UniformeDiscrete { min: 1, max: 6 },
                                    1 => Modele::UniformeContinue { min: 0.0, max: 1.0 },
                                    2 => Modele::Bernoulli { p: 0.5 },
                                    3 => Modele::Binomiale { n: 10, p: 0.5 },
                                    4 => Modele::Geometrique { p: 1.0 / 6.0 },
                                    5 => Modele::Poisson { lambda: 4.0 },
                                    _ => Modele::Normale {
                                        mu: 0.0,
                                        sigma: 1.0,
                                    },
                                };
                                *dirty = true;
                            }
                        }
                    });
                // réglages propres à chaque loi
                match &mut sim.modele {
                    Modele::UniformeDiscrete { min, max } => {
                        ui.horizontal(|ui| {
                            ui.label(t.sim_min);
                            if ui.add(DragValue::new(min).speed(1)).changed() {
                                *dirty = true;
                            }
                            ui.label(t.sim_max);
                            if ui.add(DragValue::new(max).speed(1)).changed() {
                                *dirty = true;
                            }
                        });
                    }
                    Modele::UniformeContinue { min, max } => {
                        ui.horizontal(|ui| {
                            ui.label(t.sim_min);
                            if ui.add(DragValue::new(min).speed(0.1)).changed() {
                                *dirty = true;
                            }
                            ui.label(t.sim_max);
                            if ui.add(DragValue::new(max).speed(0.1)).changed() {
                                *dirty = true;
                            }
                        });
                    }
                    Modele::Bernoulli { p } | Modele::Geometrique { p } => {
                        ui.horizontal(|ui| {
                            ui.label(t.sim_p);
                            if ui
                                .add(DragValue::new(p).speed(0.01).range(0.0..=1.0))
                                .changed()
                            {
                                *dirty = true;
                            }
                        });
                    }
                    Modele::Binomiale { n, p } => {
                        ui.horizontal(|ui| {
                            ui.label(t.sim_n);
                            if ui
                                .add(DragValue::new(n).speed(1).range(1..=10_000))
                                .changed()
                            {
                                *dirty = true;
                            }
                            ui.label(t.sim_p);
                            if ui
                                .add(DragValue::new(p).speed(0.01).range(0.0..=1.0))
                                .changed()
                            {
                                *dirty = true;
                            }
                        });
                    }
                    Modele::Poisson { lambda } => {
                        ui.horizontal(|ui| {
                            ui.label(t.sim_lambda);
                            if ui
                                .add(DragValue::new(lambda).speed(0.1).range(0.01..=1_000.0))
                                .changed()
                            {
                                *dirty = true;
                            }
                        });
                    }
                    Modele::Normale { mu, sigma } => {
                        ui.horizontal(|ui| {
                            ui.label(t.sim_mu);
                            if ui.add(DragValue::new(mu).speed(0.1)).changed() {
                                *dirty = true;
                            }
                            ui.label(t.sim_sigma);
                            if ui
                                .add(DragValue::new(sigma).speed(0.05).range(0.01..=10_000.0))
                                .changed()
                            {
                                *dirty = true;
                            }
                        });
                    }
                }
                // taille, nombre d'échantillons, classes
                ui.horizontal(|ui| {
                    ui.label(t.sim_taille);
                    if ui.add(DragValue::new(&mut sim.taille).speed(1)).changed() {
                        *dirty = true;
                    }
                    ui.label(t.sim_nombre);
                    if ui.add(DragValue::new(&mut sim.nombre).speed(1)).changed() {
                        *dirty = true;
                    }
                    ui.label(t.sim_classes);
                    if ui.add(DragValue::new(&mut sim.classes).speed(1)).changed() {
                        *dirty = true;
                    }
                });
                // graphique
                let idx_g = match sim.graphique {
                    Graphique::Points => 0,
                    Graphique::Histogrammes => 1,
                    Graphique::HistogrammeEnsemble => 2,
                    Graphique::Boites => 3,
                };
                let graphs = [t.g_points, t.g_histogrammes, t.g_ensemble, t.g_boites];
                ComboBox::from_label(t.sim_graphique)
                    .selected_text(graphs[idx_g])
                    .show_ui(ui, |ui| {
                        for (j, l) in graphs.iter().enumerate() {
                            if ui.selectable_label(j == idx_g, *l).clicked() && j != idx_g {
                                sim.graphique = match j {
                                    0 => Graphique::Points,
                                    1 => Graphique::Histogrammes,
                                    2 => Graphique::HistogrammeEnsemble,
                                    _ => Graphique::Boites,
                                };
                                *dirty = true;
                            }
                        }
                    });
                // habillages
                if ui
                    .checkbox(&mut sim.courbe_theorique, t.sim_theorique)
                    .changed()
                {
                    *dirty = true;
                }
                if matches!(sim.modele, Modele::Bernoulli { .. })
                    && ui
                        .checkbox(&mut sim.aff_fluctuation, t.sim_fluctuation)
                        .changed()
                {
                    *dirty = true;
                }
                // Lancer : nouvelle graine (horloge) + échelle automatique
                if ui.button(t.sim_lancer).clicked() {
                    sim.graine = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| (d.as_nanos() as u64).max(1))
                        .unwrap_or(1);
                    let tir = sim.tirages();
                    let ((x0, x1), (y0, y1)) = sim.plages(&tir);
                    let dx = (x1 - x0).abs().max(1e-9);
                    self.page.cx = (x0 + x1) / 2.0;
                    self.page.unit_mm = ((self.page.width_mm - 10.0) / dx).max(1e-6);
                    if y0.is_finite() && y1.is_finite() {
                        let dy = (y1 - y0).abs().max(1e-9);
                        self.page.cy = (y0 + y1) / 2.0;
                        self.page.unit_y_mm = Some(((self.page.height_mm - 10.0) / dy).max(1e-6));
                    }
                    *dirty = true;
                }
                // grille générale : calculs sur l'ensemble des tirages
                if let Some(pa) = sim.params_ensemble(&sim.tirages()) {
                    ui.separator();
                    ui.strong(t.sim_ensemble);
                    ui.label(format!("{} {:.0}", t.st_n, pa.n));
                    ui.label(format!("{} {:.2}", t.st_moyenne, pa.moyenne));
                    ui.label(format!("{} {:.2}", t.st_sigma, pa.sigma));
                    ui.label(format!("{} {:.2}", t.st_min, pa.minimum));
                    ui.label(format!("{} {:.2}", t.st_max, pa.maximum));
                }
            });
    }

    /// Bouton « Calculs » du panneau latéral (v1.1) : solveur
    /// approché du menu Calculs de l'original (man. p. 95).
    fn section_calculs(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.strong(t.calculs);
        if ui.button(t.calc_resoudre).clicked() {
            if !self.calculs_ouvert {
                self.calculs_ouvert = true;
                if self.calc_f1.is_none() {
                    self.calc_f1 = self
                        .curves
                        .iter()
                        .position(|c| matches!(c.kind, CurveKind::Fonction { .. }));
                    self.calc_f2 = None;
                    let (x0, x1) = self.page.x_range();
                    self.calc_a = x0;
                    self.calc_b = x1;
                }
            } else {
                self.calculs_ouvert = false;
            }
        }
    }

    /// Fenêtre « Résoudre une équation » (v1.1) : deux fonctions ou Ox,
    /// intervalle d'étude, recherche approchée des solutions (balayage +
    /// dichotomie) et des extrema locaux, marquage sur la feuille.
    fn fenetre_calculs(&mut self, ctx: &egui::Context) {
        if !self.calculs_ouvert {
            return;
        }
        let t = i18n::table(self.lang);
        // les 10 fonctions de l'original : le solveur n'agit que sur elles
        let fns: Vec<(usize, String)> = self
            .curves
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.kind, CurveKind::Fonction { .. }))
            .map(|(i, c)| {
                let e = match &c.kind {
                    CurveKind::Fonction { expr } => expr.clone(),
                    _ => String::new(),
                };
                (i, e)
            })
            .collect();
        if fns.is_empty() {
            let mut ouverte = true;
            egui::Window::new(t.calculs)
                .open(&mut ouverte)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(RichText::new(t.calc_aucune_fonction).weak());
                });
            if !ouverte {
                self.calculs_ouvert = false;
            }
            return;
        }
        let mut ouverte = true;
        egui::Window::new(t.calc_resoudre)
            .open(&mut ouverte)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                if self.calc_f1.is_none() {
                    self.calc_f1 = Some(fns[0].0);
                }
                if self.calc_f2 == self.calc_f1 && self.calc_f2.is_some() {
                    // la courbe de droite ne peut pas être celle de gauche
                    self.calc_f2 = None;
                }
                let idx1 = self.calc_f1.unwrap_or(0);
                // choix de la fonction f
                let ex1 = fns
                    .iter()
                    .find(|(i, _)| *i == idx1)
                    .map(|(_, e)| e.clone())
                    .unwrap_or_default();
                ComboBox::from_label(t.calc_f)
                    .selected_text(ex1.clone())
                    .show_ui(ui, |ui| {
                        for (i, e) in &fns {
                            if ui.selectable_label(*i == idx1, e).clicked() && *i != idx1 {
                                self.calc_f1 = Some(*i);
                            }
                        }
                    });
                // choix de g (une autre fonction, ou Ox)
                let lb2 = match self.calc_f2 {
                    None => t.calc_contre_ox.to_string(),
                    Some(i) => fns
                        .iter()
                        .find(|(j, _)| *j == i)
                        .map(|(_, e)| e.clone())
                        .unwrap_or_default(),
                };
                ComboBox::from_label(t.calc_contre)
                    .selected_text(lb2)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.calc_f2.is_none(), t.calc_contre_ox)
                            .clicked()
                            && self.calc_f2.is_some()
                        {
                            self.calc_f2 = None;
                        }
                        for (i, e) in &fns {
                            if *i == idx1 {
                                continue;
                            }
                            if ui.selectable_label(self.calc_f2 == Some(*i), e).clicked()
                                && self.calc_f2 != Some(*i)
                            {
                                self.calc_f2 = Some(*i);
                            }
                        }
                    });
                // intervalle d'étude
                ui.horizontal(|ui| {
                    ui.label(t.calc_intervalle);
                    ui.add(DragValue::new(&mut self.calc_a).speed(0.5));
                    ui.label(t.calc_to);
                    ui.add(DragValue::new(&mut self.calc_b).speed(0.5));
                });
                if ui
                    .checkbox(&mut self.calc_marquer, t.calc_marquer)
                    .changed()
                {
                    // (les marques ne changent qu'à la prochaine recherche)
                }
                // recherche des solutions et des extrema
                if ui.button(t.calc_rechercher).clicked() {
                    let ex2 = match self.calc_f2 {
                        None => String::new(),
                        Some(i) => fns
                            .iter()
                            .find(|(j, _)| *j == i)
                            .map(|(_, e)| e.clone())
                            .unwrap_or_default(),
                    };
                    let (a, b) = (self.calc_a.min(self.calc_b), self.calc_a.max(self.calc_b));
                    let n = (((b - a).abs() * 400.0).max(4000.0) as usize).min(200_000);
                    let f1v = |x: f64| eval_fonction(&ex1, x).unwrap_or(f64::NAN);
                    self.calc_solutions = match self.calc_f2 {
                        None => solve::zeros(&f1v, a, b, n),
                        Some(_) => {
                            let f2v = |x: f64| eval_fonction(&ex2, x).unwrap_or(f64::NAN);
                            solve::resolue(&f1v, &f2v, a, b, n)
                        }
                    }
                    .into_iter()
                    .map(|x| (x, f1v(x)))
                    .collect();
                    self.calc_extrema = solve::extrema(&f1v, a, b, n);
                    // marques sur la feuille (l'« interprétation graphique »)
                    let old = std::mem::take(&mut self.solve_marques);
                    self.points.retain(|p| !old.contains(p));
                    if self.calc_marquer {
                        for &(x, _) in &self.calc_solutions {
                            let m = PointAnnot { curve: idx1, x };
                            self.solve_marques.push(m);
                            self.points.push(m);
                        }
                        for &(x, _, _) in &self.calc_extrema {
                            let m = PointAnnot { curve: idx1, x };
                            if !self.solve_marques.contains(&m) {
                                self.solve_marques.push(m);
                                self.points.push(m);
                            }
                        }
                    }
                    self.dirty = true;
                }
                // affichage des valeurs approchées
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        ui.separator();
                        ui.strong(t.calc_solutions);
                        if self.calc_solutions.is_empty() {
                            ui.label(RichText::new(t.calc_sol_aucune).weak().small());
                        }
                        for &(x, y) in &self.calc_solutions {
                            ui.label(format!("x ≈ {x:.4} ; y ≈ {y:.4}"));
                        }
                        ui.strong(t.calc_extrema_t);
                        if self.calc_extrema.is_empty() {
                            ui.label(RichText::new(t.calc_extrema_aucun).weak().small());
                        }
                        for &(x, y, min) in &self.calc_extrema {
                            let lbl = if min { t.calc_min } else { t.calc_max };
                            ui.label(format!("{lbl} : x ≈ {x:.4} ; y ≈ {y:.4}"));
                        }
                    });
            });
        if !ouverte {
            self.calculs_ouvert = false;
        }
    }

    /// Fenêtre d'édition d'un arbre de probabilités (v0.9) : table des
    /// branches (code, nom, probabilité — préfixe « & » = barre), Ω,
    /// « sans grille », couleur, épaisseur ; aperçu du niveau.
    fn fenetre_arbre(&mut self, ctx: &egui::Context) {
        let t = i18n::table(self.lang);
        let Some(idx) = self.arbre_sel else {
            return;
        };
        if self.arbres.get(idx).is_none() {
            self.arbre_sel = None;
            return;
        }
        let mut ouverte = true;
        egui::Window::new(format!("{} — {}", t.arbres, idx + 1))
            .open(&mut ouverte)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                let dirty = &mut self.dirty;
                let a = &mut self.arbres[idx];
                ui.horizontal(|ui| {
                    ui.label(t.stat_titre);
                    let re = ui.add(TextEdit::singleline(&mut a.titre).desired_width(200.0));
                    ui.checkbox(&mut a.dessiner, t.stat_dessiner);
                    if ui.checkbox(&mut a.omega, t.arb_omega).changed()
                        || ui.checkbox(&mut a.sans_grille, t.arb_sans_grille).changed()
                        || re.changed()
                    {
                        *dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t.couleur);
                    if ui.color_edit_button_srgb(&mut a.couleur).changed() {
                        *dirty = true;
                    }
                    ui.label(t.arb_epaisseur);
                    if ui
                        .add(
                            DragValue::new(&mut a.epaisseur)
                                .speed(0.05)
                                .range(0.2..=4.0),
                        )
                        .changed()
                    {
                        *dirty = true;
                    }
                });
                ui.separator();
                egui::Grid::new("arbre-branches")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(t.arb_code);
                        ui.strong(t.arb_nom);
                        ui.strong(t.arb_pro);
                        ui.label("");
                        ui.end_row();
                        let mut to_remove: Option<usize> = None;
                        for (i, b) in a.branches.iter_mut().enumerate() {
                            if ui
                                .add(DragValue::new(&mut b.code).speed(1).range(1..=9999))
                                .changed()
                            {
                                *dirty = true;
                            }
                            if ui
                                .add(TextEdit::singleline(&mut b.nom).desired_width(80.0))
                                .changed()
                            {
                                *dirty = true;
                            }
                            if ui
                                .add(TextEdit::singleline(&mut b.proba).desired_width(50.0))
                                .changed()
                            {
                                *dirty = true;
                            }
                            if ui.small_button("\u{2715}").clicked() {
                                to_remove = Some(i);
                            }
                            ui.end_row();
                        }
                        if let Some(i) = to_remove {
                            a.branches.remove(i);
                            *dirty = true;
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button(t.st_aj_ligne).clicked() {
                        a.branches.push(BrancheArbre {
                            code: a.branches.iter().map(|b| b.code + 10).max().unwrap_or(1),
                            nom: String::new(),
                            proba: String::new(),
                        });
                        *dirty = true;
                    }
                    if ui.button(t.arb_exemple).clicked() {
                        *a = stats::exemple_arbre();
                        *dirty = true;
                    }
                });
                // aperçu : niveau et feuilles reconnus par l'analyseur
                if !a.branches.is_empty() {
                    ui.weak(format!(
                        "{} : {}, {} : {}",
                        t.arb_niveaux,
                        a.niveaux(),
                        t.arb_terminaux,
                        a.terminaux().len()
                    ));
                }
            });
        if !ouverte {
            self.arbre_sel = None;
        }
    }

    fn section_repere(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.strong(t.repere);
        ui.horizontal(|ui| {
            ui.label(t.origine);
            if ui
                .add(DragValue::new(&mut self.page.cx).speed(0.1))
                .changed()
            {
                self.dirty = true;
            }
            ui.label(";");
            if ui
                .add(DragValue::new(&mut self.page.cy).speed(0.1))
                .changed()
            {
                self.dirty = true;
            }
            ui.label(")");
        });
        ui.horizontal(|ui| {
            ui.label(t.largeur);
            if ui
                .add(
                    DragValue::new(&mut self.page.width_mm)
                        .speed(1.0)
                        .suffix(" mm")
                        .range(20.0..=1189.0),
                )
                .changed()
            {
                self.dirty = true;
            }
            ui.label(t.hauteur);
            if ui
                .add(
                    DragValue::new(&mut self.page.height_mm)
                        .speed(1.0)
                        .suffix(" mm")
                        .range(20.0..=1189.0),
                )
                .changed()
            {
                self.dirty = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(t.unite);
            if ui
                .add(
                    DragValue::new(&mut self.page.unit_mm)
                        .speed(0.5)
                        .suffix(" mm")
                        .range(1.0..=100.0),
                )
                .changed()
            {
                self.dirty = true;
            }
            ui.small(t.grille);
        });
        // unité propre à l'axe des ordonnées (v0.4+, unités séparées de
        // l'original) ; bouton Ajouter si l'unité de X est partagée
        ui.horizontal(|ui| match self.page.unit_y_mm {
            None => {
                ui.label(t.unite_oy_vierge);
                if ui.small_button("➕").clicked() {
                    self.page.unit_y_mm = Some(self.page.unit_mm);
                    self.dirty = true;
                }
            }
            Some(ref mut uy) => {
                ui.label(t.unite_oy);
                if ui
                    .add(
                        DragValue::new(uy)
                            .speed(0.5)
                            .suffix(" mm")
                            .range(1.0..=100.0),
                    )
                    .changed()
                {
                    self.dirty = true;
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.page.log_x, "log X").changed() {
                self.dirty = true;
            }
            if ui.checkbox(&mut self.page.log_y, "log Y").changed() {
                self.dirty = true;
            }
            if ui.button(t.recentrer).clicked() {
                self.page.cx = 0.0;
                self.page.cy = 0.0;
                self.zoom = 1.0;
                self.view_offset = Vec2::ZERO;
                self.dirty = true;
            }
        });
    }

    fn section_export(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.strong(t.export);
        ui.horizontal(|ui| {
            if ui.button("SVG (sinequanon.svg)").clicked() {
                let data = self.export_data();
                let path = PathBuf::from("sinequanon.svg");
                match render::write_svg_file(
                    &self.page,
                    &data,
                    &self.annots,
                    &self.tableaux,
                    &self.stats,
                    &self.boites,
                    &self.suites,
                    &self.serie2,
                    &self.arbres,
                    &self.simus,
                    &path,
                ) {
                    Ok(n) => self.set_status(format!("✔ {} écrit ({} octets)", path.display(), n)),
                    Err(e) => self.set_status(format!("✖ Échec SVG : {e}")),
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("PNG (sinequanon.png)").clicked() {
                let data = self.export_data();
                let path = PathBuf::from("sinequanon.png");
                match render::write_png_file(
                    &self.page,
                    &data,
                    &self.annots,
                    &self.tableaux,
                    &self.stats,
                    &self.boites,
                    &self.suites,
                    &self.serie2,
                    &self.arbres,
                    &self.simus,
                    self.px_per_mm as f64,
                    &path,
                ) {
                    Ok(()) => self.set_status(format!("✔ {} écrit", path.display())),
                    Err(e) => self.set_status(format!("✖ Échec PNG : {e}")),
                }
            }
            if ui.button("BMP (sinequanon.bmp)").clicked() {
                let data = self.export_data();
                let path = PathBuf::from("sinequanon.bmp");
                match render::write_bmp_file(
                    &self.page,
                    &data,
                    &self.annots,
                    &self.tableaux,
                    &self.stats,
                    &self.boites,
                    &self.suites,
                    &self.serie2,
                    &self.arbres,
                    &self.simus,
                    self.px_per_mm as f64,
                    &path,
                ) {
                    Ok(()) => self.set_status(format!("✔ {} écrit", path.display())),
                    Err(e) => self.set_status(format!("✖ Échec BMP : {e}")),
                }
            }
            if ui.button("GIF (sinequanon.gif)").clicked() {
                let data = self.export_data();
                let path = PathBuf::from("sinequanon.gif");
                match render::write_gif_file(
                    &self.page,
                    &data,
                    &self.annots,
                    &self.tableaux,
                    &self.stats,
                    &self.boites,
                    &self.suites,
                    &self.serie2,
                    &self.arbres,
                    &self.simus,
                    self.px_per_mm as f64,
                    &path,
                ) {
                    Ok(()) => self.set_status(format!("✔ {} écrit", path.display())),
                    Err(e) => self.set_status(format!("✖ Échec GIF : {e}")),
                }
            }
            if ui.button("EPS (sinequanon.eps)").clicked() {
                let data = self.export_data();
                let path = PathBuf::from("sinequanon.eps");
                match render::write_eps_file(
                    &self.page,
                    &data,
                    &self.annots,
                    &self.tableaux,
                    &self.stats,
                    &self.boites,
                    &self.suites,
                    &self.serie2,
                    &self.arbres,
                    &self.simus,
                    self.px_per_mm as f64,
                    &path,
                ) {
                    Ok(()) => self.set_status(format!("✔ {} écrit", path.display())),
                    Err(e) => self.set_status(format!("✖ Échec EPS : {e}")),
                }
            }
            ui.label(t.resolution);
            ui.add(
                DragValue::new(&mut self.px_per_mm)
                    .suffix(" px/mm")
                    .range(1.0..=12.0),
            );
        });
        if let Some((msg, t)) = &self.status {
            if t.elapsed().as_secs() < 10 {
                ui.add_space(4.0);
                ui.label(RichText::new(msg).small());
            }
        }
    }

    fn zone_dessin(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let fit = (rect.width() / self.page.width_mm as f32)
            .min(rect.height() / self.page.height_mm as f32)
            .max(0.5);
        let ppm = (fit * self.zoom).max(0.2);

        // --- interactions (extraites avant le dessin) ---
        let drag = if response.dragged() {
            response.drag_delta()
        } else {
            Vec2::ZERO
        };
        let wheel = ui.input(|i| i.raw_scroll_delta.y);
        let hover = response.hover_pos();
        if drag != Vec2::ZERO {
            self.view_offset += drag;
        }
        if wheel != 0.0 {
            if let Some(pos) = hover {
                let factor = (-wheel * 0.0015).exp();
                let ppm2 = (ppm * factor).clamp(fit * 0.25, fit * 8.0);
                if (ppm2 - ppm).abs() > f32::EPSILON {
                    let page_w = self.page.width_mm as f32 * ppm;
                    let page_h = self.page.height_mm as f32 * ppm;
                    let page_min =
                        rect.center() - Vec2::new(page_w, page_h) / 2.0 + self.view_offset;
                    let mm = ((pos.x - page_min.x) / ppm, (pos.y - page_min.y) / ppm);
                    let page_w2 = self.page.width_mm as f32 * ppm2;
                    let page_h2 = self.page.height_mm as f32 * ppm2;
                    let want_min = Pos2::new(pos.x - mm.0 * ppm2, pos.y - mm.1 * ppm2);
                    let centered_min = rect.center() - Vec2::new(page_w2, page_h2) / 2.0;
                    self.view_offset = want_min - centered_min;
                    self.zoom = ppm2 / fit;
                }
            }
        }
        // limitation du décalage pour garder la page à portée
        let page_w = self.page.width_mm as f32 * ppm;
        let page_h = self.page.height_mm as f32 * ppm;
        let max_off = Vec2::new(page_w.max(rect.width()), page_h.max(rect.height()));
        self.view_offset = self.view_offset.clamp(-max_off, max_off);

        // --- dessin ---
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(226, 228, 232));
        let page_rect = Rect::from_min_size(
            rect.center() - Vec2::new(page_w, page_h) / 2.0 + self.view_offset,
            Vec2::new(page_w, page_h),
        );
        painter.rect_filled(page_rect, 0.0, Color32::WHITE);
        painter.rect_stroke(
            page_rect,
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgb(150, 150, 155)),
        );

        self.dessine_grille(&painter, page_rect, ppm);
        self.dessine_axes(&painter, page_rect, ppm);
        self.dessine_courbes(&painter, page_rect, ppm);
        self.dessine_annots(&painter, page_rect, ppm);
        self.dessine_tableaux(&painter, page_rect, ppm);
        self.dessine_stats(&painter, page_rect, ppm);

        // lecture des coordonnées sous le curseur
        if let Some(pos) = hover {
            let mm = (
                (pos.x - page_rect.min.x) / ppm,
                (pos.y - page_rect.min.y) / ppm,
            );
            let ux = self.page.x_at_mm(mm.0 as f64);
            let uy = self.page.y_at_mm(mm.1 as f64);
            if ux.is_finite() && uy.is_finite() {
                painter.text(
                    Pos2::new(rect.left() + 8.0, rect.bottom() - 10.0),
                    Align2::LEFT_BOTTOM,
                    format!("x = {ux:.4}    y = {uy:.4}"),
                    FontId::monospace(12.0),
                    Color32::from_rgb(40, 40, 60),
                );
            }
        }
    }

    /// Section « Statistiques à une variable » du panneau latéral (v0.4).
    fn section_statistiques(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        ui.horizontal(|ui| {
            ui.strong(t.stats);
            if ui.button(t.m_stat).clicked() {
                if self.stats.is_empty() {
                    // exemple du manuel (p.45) comme contenu par défaut
                    self.stats.push(stats::exemple_enfants());
                }
                self.stat_sel = Some(0);
                // le volet suit les données
                self.stat_volet = match &self.stats[0].donnees {
                    Donnees::Quali(_) => 0,
                    Donnees::Isolees(_) => 1,
                    Donnees::Classes(_) => 2,
                };
                self.dirty = true;
            }
        });
        let mut to_delete: Option<usize> = None;
        let mut to_toggle: Option<usize> = None;
        for (i, s) in self.stats.iter().enumerate() {
            let label = if s.titre.is_empty() {
                format!("{} {}", t.stats, i + 1)
            } else {
                s.titre.lines().next().unwrap_or("").to_string()
            };
            ui.horizontal(|ui| {
                let vis = if s.dessiner { "◉" } else { "○" };
                if ui
                    .add(egui::Label::new(RichText::new(vis).size(16.0)).sense(Sense::click()))
                    .clicked()
                {
                    to_toggle = Some(i);
                }
                if ui
                    .add(egui::Label::new(RichText::new(label).small()).sense(Sense::click()))
                    .clicked()
                {
                    self.stat_sel = Some(i);
                    // le volet suit les données de la série cliquée
                    self.stat_volet = match &self.stats[i].donnees {
                        Donnees::Quali(_) => 0,
                        Donnees::Isolees(_) => 1,
                        Donnees::Classes(_) => 2,
                    };
                }
                if ui.small_button("✕").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_toggle {
            if let Some(s) = self.stats.get_mut(i) {
                s.dessiner = !s.dessiner;
                self.dirty = true;
            }
        }
        if let Some(i) = to_delete {
            self.stats.remove(i);
            self.stat_sel = None;
            self.dirty = true;
        }
    }

    /// Fenêtre « Statistiques à une variable » : 4 volets, saisie des
    /// données et panneau Calculs comme dans l'original.
    fn fenetre_stat(&mut self, ctx: &egui::Context) {
        let t = i18n::table(self.lang);
        if self.stat_sel.is_none() {
            return;
        }
        if self.stats.is_empty() {
            self.stat_sel = None;
            return;
        }
        let mut ouverte = true;
        egui::Window::new(t.stats)
            .open(&mut ouverte)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                self.vois_stat(ui);
            });
        if !ouverte {
            self.stat_sel = None;
        }
    }

    /// Volets de la fenêtre statistique (0 quali, 1 isolées, 2 classes,
    /// 3 boîtes multiples) ; le volet actif est `self.stat_volet`.
    fn vois_stat(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        // démontrer le volet dans le combo (4 volets comme l'original)
        let volet = self.stat_volet;
        let labels = [t.v_quali, t.v_isolees, t.v_classes, t.v_boites];
        ui.horizontal(|ui| {
            ui.label(t.volet);
            ComboBox::from_id_salt("volet-stat")
                .selected_text(labels[volet.min(labels.len() - 1)])
                .show_ui(ui, |ui| {
                    for (i, l) in labels.iter().enumerate() {
                        if ui.selectable_label(volet == i, *l).clicked() && volet != i {
                            self.stat_volet = i;
                            if i < 3 {
                                // adapter données et style au volet choisi,
                                // comme une boîte de saisie vierge
                                self.reinit_volet(i);
                            }
                            self.dirty = true;
                        }
                    }
                });
        });
        ui.separator();
        // 4e volet : boîtes à moustaches multiples (objet séparé)
        if self.stat_volet == 3 {
            self.vois_boites(ui);
            return;
        }
        let idx = self.stat_sel.unwrap_or(0);
        if self.stats.get(idx).is_none() {
            return;
        }
        let s = &mut self.stats[idx];
        // emprunt disjoint : les closures écrivent ce compteur
        let dirty = &mut self.dirty;
        ui.horizontal(|ui| {
            ui.label(t.stat_titre);
            if ui
                .add(TextEdit::singleline(&mut s.titre).desired_width(240.0))
                .changed()
            {
                *dirty = true;
            }
        });
        ui.checkbox(&mut s.dessiner, t.stat_dessiner);
        // exemples du manuel (p. 43-45), comme les Var1..Var8 des tableaux
        let mut to_load: Option<usize> = None;
        ui.horizontal(|ui| {
            ui.label(t.exemples);
            if ui.small_button("Enfants").clicked() {
                to_load = Some(0);
            }
            if ui.small_button("Salaires").clicked() {
                to_load = Some(1);
            }
            if ui.small_button("Diplômes").clicked() {
                to_load = Some(2);
            }
        });
        if let Some(k) = to_load {
            // le volet suit les données de l'exemple ; le style vient avec
            self.stat_volet = [1, 2, 0][k];
            *s = match k {
                0 => stats::exemple_enfants(),
                1 => stats::exemple_salaires(),
                _ => stats::exemple_quali(),
            };
            *dirty = true;
        }
        // style du graphique, selon le volet
        let styles: &[(Style, &str)] = match self.stat_volet {
            0 => &[
                (Style::Camembert, t.st_camembert),
                (Style::Histogramme, t.st_histogramme),
            ],
            1 => &[
                (Style::Batons, t.st_batons),
                (Style::Boite, t.st_boite),
                (Style::BoiteAvecDeciles, t.st_boite_deciles),
                (Style::LoiProba, t.st_loi),
            ],
            _ => &[
                (Style::Histogramme, t.st_histogramme),
                (Style::Batons, t.st_batons),
                (Style::Boite, t.st_boite),
                (Style::BoiteAvecDeciles, t.st_boite_deciles),
                (Style::FreqCumul, t.st_cumul),
                (Style::DroiteHenry, t.st_henry),
                (Style::LoiProba, t.st_loi),
            ],
        };
        ui.horizontal(|ui| {
            ui.label(t.style);
            let courant = styles
                .iter()
                .find(|(st, _)| *st == s.style)
                .map(|(_, l)| *l)
                .unwrap_or("");
            ComboBox::from_id_salt("style-stat")
                .selected_text(courant)
                .show_ui(ui, |ui| {
                    for (st, l) in styles {
                        let sel = s.style == *st;
                        if ui.selectable_label(sel, *l).clicked() && !sel {
                            s.style = *st;
                            *dirty = true;
                        }
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label(t.stat_visualiser);
            let courant = match s.visualiser {
                Visualiser::MedianeSeulement => t.vis_mediane,
                Visualiser::MedianeEtQuartiles => t.vis_q,
                Visualiser::Tous => t.vis_tous,
            };
            ComboBox::from_id_salt("visualiser")
                .selected_text(courant)
                .show_ui(ui, |ui| {
                    let vals = [
                        (Visualiser::MedianeSeulement, t.vis_mediane),
                        (Visualiser::MedianeEtQuartiles, t.vis_q),
                        (Visualiser::Tous, t.vis_tous),
                    ];
                    for (v, l) in vals {
                        let sel = s.visualiser == v;
                        if ui.selectable_label(sel, l).clicked() && !sel {
                            s.visualiser = v;
                            *dirty = true;
                        }
                    }
                });
        });
        // options propres aux styles « classes »
        if matches!(s.donnees, Donnees::Classes(_)) {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut s.avec_pct, t.st_avec_pct).changed() {
                    *dirty = true;
                }
                ui.label(t.st_canaux);
                if ui
                    .add(DragValue::new(&mut s.canaux).speed(0.1).range(0.01..=1e9))
                    .changed()
                {
                    *dirty = true;
                }
                if ui.checkbox(&mut s.aff_effectifs, t.st_effectifs).changed() {
                    *dirty = true;
                }
            });
            if s.style == Style::FreqCumul {
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut s.croissantes, t.st_croiss).changed() {
                        *dirty = true;
                    }
                    if ui.checkbox(&mut s.decroissantes, t.st_decroiss).changed() {
                        *dirty = true;
                    }
                });
            }
        }
        ui.separator();
        // saisie : grille éditable selon le volet
        match &mut s.donnees {
            Donnees::Quali(mods) => {
                egui::Grid::new("quali").striped(true).show(ui, |ui| {
                    ui.strong(t.stat_nom);
                    ui.strong(t.stat_effectif);
                    ui.end_row();
                    for m in mods.iter_mut() {
                        if ui
                            .add(TextEdit::singleline(&mut m.nom).desired_width(200.0))
                            .changed()
                        {
                            *dirty = true;
                        }
                        if ui.add(DragValue::new(&mut m.n).range(0.0..=1e12)).changed() {
                            *dirty = true;
                        }
                        ui.end_row();
                    }
                });
                if ui.button(t.st_aj_ligne).clicked() {
                    let n = mods.len();
                    mods.push(stats::Modalite {
                        nom: format!("modalité {n}"),
                        n: 1.0,
                        couleur: stats::COULEURS_MODALITES[n % 7],
                    });
                    *dirty = true;
                }
            }
            Donnees::Isolees(vals) => {
                egui::Grid::new("isolees").striped(true).show(ui, |ui| {
                    ui.strong(t.stat_valeur);
                    ui.strong(t.stat_effectif);
                    ui.end_row();
                    for r in vals.iter_mut() {
                        if ui
                            .add(DragValue::new(&mut r.v).speed(0.1).range(-1e12..=1e12))
                            .changed()
                        {
                            *dirty = true;
                        }
                        if ui.add(DragValue::new(&mut r.n).range(0.0..=1e12)).changed() {
                            *dirty = true;
                        }
                        ui.end_row();
                    }
                });
                if ui.button(t.st_aj_ligne).clicked() {
                    vals.push(stats::ValEff { v: 0.0, n: 1.0 });
                    *dirty = true;
                }
            }
            Donnees::Classes(c) => {
                egui::Grid::new("classes").striped(true).show(ui, |ui| {
                    ui.strong(t.stat_borne_inf);
                    ui.strong(t.stat_borne_sup);
                    ui.strong(t.stat_effectif);
                    ui.end_row();
                    for r in c.iter_mut() {
                        for x in [&mut r.a, &mut r.b, &mut r.n] {
                            if ui
                                .add(DragValue::new(x).speed(1.0).range(0.0..=1e12))
                                .changed()
                            {
                                *dirty = true;
                            }
                        }
                        ui.end_row();
                    }
                });
                if ui.button(t.st_aj_ligne).clicked() {
                    let b = c.last().map(|x| x.b).unwrap_or(0.0);
                    c.push(stats::Classe {
                        a: b,
                        b: b + 100.0,
                        n: 1.0,
                    });
                    *dirty = true;
                }
            }
        }
        // module probabilités (v0.5) : loi de probabilité affichée
        if s.style == Style::LoiProba {
            let loi = s.loi.or_else(|| {
                s.parametres().map(|pr| Loi::Normal {
                    mu: pr.moyenne,
                    sigma: pr.sigma.max(0.5),
                })
            });
            if let Some(loi) = loi {
                ui.strong(t.st_loi);
                ui.label(loi.nom());
                ui.label(RichText::new(loi.description()).strong());
                if let Some(pr) = s.parametres() {
                    ui.label(format!(
                        "{} : {}",
                        t.st_proba,
                        stats::num(loi.probabilite(pr.moyenne))
                    ));
                }
            }
        }
        // panneau Calculs, valeurs à la française comme l'original
        if let Some(pr) = s.parametres() {
            ui.strong(t.stat_calculs);
            egui::Grid::new("calculs").num_columns(2).show(ui, |ui| {
                for (lbl, val) in [
                    (t.st_n, pr.n),
                    (t.st_somme, pr.somme),
                    (t.st_moyenne, pr.moyenne),
                    (t.st_sigma, pr.sigma),
                    (t.st_min, pr.minimum),
                    (t.st_max, pr.maximum),
                    (t.st_d1, pr.d1),
                    (t.st_q1, pr.q1),
                    (t.st_med, pr.mediane),
                    (t.st_q3, pr.q3),
                    (t.st_d9, pr.d9),
                ] {
                    ui.label(lbl);
                    ui.label(RichText::new(stats::num(val)).strong());
                    ui.end_row();
                }
            });
        } else {
            // volet qualitatif : l'original n'y autorise aucun calcul
            ui.weak(t.st_aucun_calcul);
        }
    }

    /// Réinitialise les données au volet i (contenu « vierge »), en
    /// conservant titre et couleur ; style compatible avec le volet.
    fn reinit_volet(&mut self, i: usize) {
        let Some(idx) = self.stat_sel else {
            return;
        };
        let Some(s) = self.stats.get_mut(idx) else {
            return;
        };
        if !s.donnees.est_volet(i) {
            s.donnees = match i {
                0 => Donnees::Quali(vec![stats::Modalite {
                    nom: "modalité 1".into(),
                    n: 1.0,
                    couleur: stats::COULEURS_MODALITES[0],
                }]),
                1 => Donnees::Isolees(vec![stats::ValEff { v: 1.0, n: 1.0 }]),
                _ => Donnees::Classes(vec![stats::Classe {
                    a: 0.0,
                    b: 100.0,
                    n: 1.0,
                }]),
            };
        }
        // style compatible avec les données du volet
        match s.donnees {
            Donnees::Quali(_) => {
                if s.style != Style::Camembert && s.style != Style::Histogramme {
                    s.style = Style::Camembert;
                }
            }
            Donnees::Isolees(_) => {
                if matches!(
                    s.style,
                    Style::Histogramme | Style::FreqCumul | Style::DroiteHenry | Style::LoiProba
                ) {
                    s.style = Style::Batons;
                }
            }
            Donnees::Classes(_) => {
                if s.style == Style::Batons {
                    s.style = Style::Histogramme;
                }
            }
        }
    }

    /// Éditeur du 4e volet : boîtes à moustaches multiples.
    fn vois_boites(&mut self, ui: &mut Ui) {
        let t = i18n::table(self.lang);
        if self.boites.is_empty() {
            self.boites.push(BoitesMultiples::default());
            self.dirty = true;
        }
        ui.horizontal(|ui| {
            ui.label(t.stat_titre);
            if ui
                .add(TextEdit::singleline(&mut self.boites[0].titre).desired_width(240.0))
                .changed()
            {
                self.dirty = true;
            }
        });
        ui.checkbox(&mut self.boites[0].dessiner, t.stat_dessiner);
        egui::Grid::new("boites").striped(true).show(ui, |ui| {
            ui.strong(t.st_legende);
            ui.strong(t.st_min);
            ui.strong("D1");
            ui.strong(t.st_q1);
            ui.strong(t.st_med);
            ui.strong(t.st_q3);
            ui.strong("D9");
            ui.strong(t.st_max);
            ui.end_row();
            for b in self.boites[0].boites.iter_mut() {
                if ui
                    .add(TextEdit::singleline(&mut b.legende).desired_width(110.0))
                    .changed()
                {
                    self.dirty = true;
                }
                for x in [
                    &mut b.min,
                    &mut b.d1.get_or_insert(0.0),
                    &mut b.q1,
                    &mut b.med.get_or_insert(0.0),
                    &mut b.q3,
                    &mut b.d9.get_or_insert(0.0),
                    &mut b.max,
                ] {
                    if ui
                        .add(DragValue::new(x).speed(1.0).range(-1e12..=1e12))
                        .changed()
                    {
                        self.dirty = true;
                    }
                }
                ui.end_row();
            }
        });
        if ui.button(t.st_aj_ligne).clicked() {
            let n = self.boites[0].boites.len();
            self.boites[0].boites.push(stats::BoiteMultiple {
                legende: format!("série {}", n + 1),
                min: 0.0,
                q1: 1.0,
                med: Some(2.0),
                q3: 3.0,
                max: 4.0,
                d1: None,
                d9: None,
                couleur: stats::LIME,
            });
            self.dirty = true;
        }
    }

    /// Dessine les séries statistiques (aperçu GUI, miroir de
    /// `render::draw_stats_raster`).
    fn dessine_stats(&self, painter: &egui::Painter, page_rect: Rect, ppm: f32) {
        let px = |x: f64, y: f64| {
            Pos2::new(
                page_rect.left() + x as f32 * ppm,
                page_rect.top() + y as f32 * ppm,
            )
        };
        let dessiner = |pieces: &[PieceStats]| {
            for piece in pieces {
                match piece {
                    PieceStats::Fond { mm, w, h } => {
                        painter.rect_filled(
                            Rect::from_min_size(
                                px(mm.0, mm.1),
                                Vec2::new(*w as f32 * ppm, *h as f32 * ppm),
                            ),
                            0.0,
                            Color32::WHITE,
                        );
                    }
                    PieceStats::Rect {
                        mm,
                        w,
                        h,
                        fill,
                        stroke,
                    } => {
                        let r_ = Rect::from_min_size(
                            px(mm.0, mm.1),
                            Vec2::new(*w as f32 * ppm, *h as f32 * ppm),
                        );
                        if let Some(c) = fill {
                            painter.rect_filled(r_, 0.0, Color32::from_rgb(c[0], c[1], c[2]));
                        }
                        let (stc, stw) = stroke;
                        painter.rect_stroke(
                            r_,
                            0.0,
                            Stroke::new(
                                (*stw as f32 * ppm).clamp(0.7, 4.0),
                                Color32::from_rgb(stc[0], stc[1], stc[2]),
                            ),
                        );
                    }
                    PieceStats::Poly { pts, fill, stroke } => {
                        let pl: Vec<Pos2> = pts.iter().map(|(x, y)| px(*x, *y)).collect();
                        if let Some(c) = fill {
                            painter.add(egui::Shape::convex_polygon(
                                pl.clone(),
                                Color32::from_rgb(c[0], c[1], c[2]),
                                Stroke::NONE,
                            ));
                        }
                        if let Some((c, w)) = stroke {
                            let mut pl2 = pl;
                            if let Some(first) = pl2.first().copied() {
                                pl2.push(first);
                            }
                            painter.add(egui::Shape::line(
                                pl2,
                                Stroke::new(
                                    (*w as f32 * ppm).clamp(0.7, 4.0),
                                    Color32::from_rgb(c[0], c[1], c[2]),
                                ),
                            ));
                        }
                    }
                    PieceStats::Ligne {
                        x0,
                        y0,
                        x1,
                        y1,
                        c,
                        w,
                    } => {
                        painter.line_segment(
                            [px(*x0, *y0), px(*x1, *y1)],
                            Stroke::new(
                                (*w as f32 * ppm).clamp(0.7, 8.0),
                                Color32::from_rgb(c[0], c[1], c[2]),
                            ),
                        );
                    }
                    PieceStats::Pointille {
                        x0,
                        y0,
                        x1,
                        y1,
                        c,
                        w,
                    } => {
                        for sh in egui::Shape::dashed_line(
                            &[px(*x0, *y0), px(*x1, *y1)],
                            Stroke::new(
                                (*w as f32 * ppm).clamp(0.7, 4.0),
                                Color32::from_rgb(c[0], c[1], c[2]),
                            ),
                            4.0,
                            3.0,
                        ) {
                            painter.add(sh);
                        }
                    }
                    PieceStats::Disque {
                        c: centre,
                        r: ray,
                        fill,
                        stroke,
                    } => {
                        painter.circle_filled(
                            px(centre.0, centre.1),
                            (*ray as f32 * ppm).max(1.5),
                            Color32::from_rgb(fill[0], fill[1], fill[2]),
                        );
                        if let Some((c, w)) = stroke {
                            painter.circle_stroke(
                                px(centre.0, centre.1),
                                (*ray as f32 * ppm).max(1.5),
                                Stroke::new(
                                    (*w as f32 * ppm).clamp(0.7, 3.0),
                                    Color32::from_rgb(c[0], c[1], c[2]),
                                ),
                            );
                        }
                    }
                    PieceStats::Texte {
                        mm,
                        txt,
                        c,
                        taille,
                        ancre,
                        vertical,
                        gras,
                        ..
                    } => {
                        let colour = Color32::from_rgb(c[0], c[1], c[2]);
                        let font = FontId::proportional((*taille as f32 * ppm).max(3.0));
                        let align = match (*ancre, *vertical) {
                            ("start", false) => Align2::LEFT_CENTER,
                            ("end", false) => Align2::RIGHT_CENTER,
                            (_, false) => Align2::CENTER_CENTER,
                            // vertical : chars empilés du bas vers le haut
                            (_, true) => Align2::CENTER_CENTER,
                        };
                        if *vertical {
                            let u = (*taille as f32 * ppm).max(3.0) * 1.15;
                            let total = txt.chars().count() as f32 * u;
                            let (ax, ay) = match *ancre {
                                "end" => (mm.0, mm.1),
                                "middle" => (mm.0, mm.1 + total as f64 / (2.0 * ppm as f64)),
                                _ => (mm.0, mm.1 + total as f64 / ppm as f64),
                            };
                            let mut ycur = ay + (total / 2.0) as f64 / ppm as f64;
                            for ch in txt.chars() {
                                painter.text(
                                    px(ax, ycur),
                                    Align2::CENTER_CENTER,
                                    ch.to_string(),
                                    font.clone(),
                                    colour,
                                );
                                if *gras {
                                    painter.text(
                                        px(ax + 0.35, ycur),
                                        Align2::CENTER_CENTER,
                                        ch.to_string(),
                                        font.clone(),
                                        colour,
                                    );
                                }
                                ycur -= (u / ppm) as f64;
                            }
                        } else {
                            painter.text(px(mm.0, mm.1), align, txt, font.clone(), colour);
                            if *gras {
                                painter.text(px(mm.0 + 0.35, mm.1), align, txt, font, colour);
                            }
                        }
                    }
                }
            }
        };
        for s in &self.stats {
            let pieces = s.draw_list(&self.page);
            dessiner(&pieces);
        }
        for b in &self.boites {
            let pieces = b.draw_list(&self.page);
            dessiner(&pieces);
        }
        for s in &self.suites {
            let pieces = s.draw_list(&self.page);
            dessiner(&pieces);
        }
        for s2 in &self.serie2 {
            let pieces = s2.draw_list(&self.page);
            dessiner(&pieces);
        }
        for a in &self.arbres {
            let pieces = a.draw_list(&self.page);
            dessiner(&pieces);
        }
        for sm in &self.simus {
            let pieces = sm.draw_list(&self.page);
            dessiner(&pieces);
        }
    }

    /// Dessine les tableaux (aperçu GUI : segments + textes).
    fn dessine_tableaux(&self, painter: &egui::Painter, page_rect: Rect, ppm: f32) {
        for tabl in &self.tableaux {
            if !tabl.dessiner {
                continue;
            }
            let ct = Color32::from_rgb(
                tabl.couleur_traits[0],
                tabl.couleur_traits[1],
                tabl.couleur_traits[2],
            );
            let cx = Color32::from_rgb(
                tabl.couleur_textes[0],
                tabl.couleur_textes[1],
                tabl.couleur_textes[2],
            );
            let px = |x: f64, y: f64| {
                Pos2::new(
                    page_rect.left() + (tabl.x_mm + x) as f32 * ppm,
                    page_rect.top() + (tabl.y_mm + y) as f32 * ppm,
                )
            };
            for piece in tab::pieces(tabl) {
                match piece {
                    Piece::Ligne { x0, y0, x1, y1, w } => {
                        painter.line_segment(
                            [px(x0, y0), px(x1, y1)],
                            Stroke::new((w as f32 * ppm).clamp(1.0, 4.0), ct),
                        );
                    }
                    Piece::Texte {
                        x,
                        y,
                        size,
                        txt,
                        italique: _,
                    } => {
                        // ancre bas-gauche ≈ ligne de base + jambage
                        painter.text(
                            px(x, y + 0.28 * size),
                            Align2::LEFT_BOTTOM,
                            txt,
                            FontId::proportional(size as f32 * ppm),
                            cx,
                        );
                    }
                }
            }
        }
    }

    fn dessine_grille(&self, painter: &egui::Painter, page_rect: Rect, ppm: f32) {
        let minor = Color32::from_rgb(219, 234, 245);
        let major = Color32::from_rgb(179, 207, 227);
        // mineure : tous les 5 mm
        let pas = (PAS_MM_F32 * ppm).max(3.0);
        let mut x = page_rect.left();
        while x <= page_rect.right() + 0.5 {
            painter.line_segment(
                [
                    Pos2::new(x, page_rect.top()),
                    Pos2::new(x, page_rect.bottom()),
                ],
                Stroke::new(1.0_f32, minor),
            );
            x += pas;
        }
        let mut y = page_rect.top();
        while y <= page_rect.bottom() + 0.5 {
            painter.line_segment(
                [
                    Pos2::new(page_rect.left(), y),
                    Pos2::new(page_rect.right(), y),
                ],
                Stroke::new(1.0_f32, minor),
            );
            y += pas;
        }
        // majeures : une unité (ou une décade)
        let (xlo, xhi) = self.page.x_range();
        let (ylo, yhi) = self.page.y_range();
        for k in self.page.majors(xlo, xhi, self.page.log_x) {
            let mx = self.page.mm_x(k);
            if !mx.is_finite() {
                continue;
            }
            let px = page_rect.left() + (mx as f32) * ppm;
            if px > page_rect.left() + 0.5 && px < page_rect.right() - 0.5 {
                painter.line_segment(
                    [
                        Pos2::new(px, page_rect.top()),
                        Pos2::new(px, page_rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, major),
                );
            }
        }
        for k in self.page.majors(ylo, yhi, self.page.log_y) {
            let my = self.page.mm_y(k);
            if !my.is_finite() {
                continue;
            }
            let py = page_rect.top() + (my as f32) * ppm;
            if py > page_rect.top() + 0.5 && py < page_rect.bottom() - 0.5 {
                painter.line_segment(
                    [
                        Pos2::new(page_rect.left(), py),
                        Pos2::new(page_rect.right(), py),
                    ],
                    Stroke::new(1.0_f32, major),
                );
            }
        }
    }

    fn dessine_axes(&self, painter: &egui::Painter, page_rect: Rect, ppm: f32) {
        let black = Color32::from_rgb(26, 26, 26);
        let gray = Color32::from_rgb(70, 70, 80);
        let yax = self.page.mm_y(0.0);
        let xax = self.page.mm_x(0.0);
        let has_y_axis = xax.is_finite() && xax >= 0.0 && xax <= self.page.width_mm;
        let has_x_axis = yax.is_finite() && yax >= 0.0 && yax <= self.page.height_mm;

        if has_x_axis {
            let y = page_rect.top() + (yax as f32) * ppm;
            painter.line_segment(
                [
                    Pos2::new(page_rect.left(), y),
                    Pos2::new(page_rect.right(), y),
                ],
                Stroke::new(1.5_f32, black),
            );
            let tip = Pos2::new(page_rect.right() - 1.0, y);
            painter.line_segment(
                [tip, Pos2::new(tip.x - 9.0, tip.y - 4.0)],
                Stroke::new(1.5_f32, black),
            );
            painter.line_segment(
                [tip, Pos2::new(tip.x - 9.0, tip.y + 4.0)],
                Stroke::new(1.5_f32, black),
            );
        }
        if has_y_axis {
            let x = page_rect.left() + (xax as f32) * ppm;
            painter.line_segment(
                [
                    Pos2::new(x, page_rect.top()),
                    Pos2::new(x, page_rect.bottom()),
                ],
                Stroke::new(1.5_f32, black),
            );
            let tip = Pos2::new(x, page_rect.top() + 1.0);
            painter.line_segment(
                [tip, Pos2::new(tip.x - 4.0, tip.y + 9.0)],
                Stroke::new(1.5_f32, black),
            );
            painter.line_segment(
                [tip, Pos2::new(tip.x + 4.0, tip.y + 9.0)],
                Stroke::new(1.5_f32, black),
            );
        }

        // graduations et labels
        let (xlo, xhi) = self.page.x_range();
        let (ylo, yhi) = self.page.y_range();
        let majors_x = self.page.majors(xlo, xhi, self.page.log_x);
        let majors_y = self.page.majors(ylo, yhi, self.page.log_y);
        let label_x = majors_x.len() <= 2 || {
            let a = majors_x.first().copied().unwrap_or(0.0);
            let b = majors_x.last().copied().unwrap_or(0.0);
            ((self.page.mm_x(b) - self.page.mm_x(a)).abs() as f32 / (majors_x.len().max(1) as f32)
                * ppm)
                > 26.0
        };
        let label_y = majors_y.len() <= 2 || {
            let a = majors_y.first().copied().unwrap_or(0.0);
            let b = majors_y.last().copied().unwrap_or(0.0);
            ((self.page.mm_y(b) - self.page.mm_y(a)).abs() as f32 / (majors_y.len().max(1) as f32)
                * ppm)
                > 26.0
        };

        if has_x_axis {
            let yax_px = page_rect.top() + (yax as f32) * ppm;
            for &k in &majors_x {
                let mx = self.page.mm_x(k);
                if !mx.is_finite() {
                    continue;
                }
                let px = page_rect.left() + (mx as f32) * ppm;
                if px <= page_rect.left() + 1.0 || px >= page_rect.right() - 1.0 {
                    continue;
                }
                painter.line_segment(
                    [Pos2::new(px, yax_px - 4.0), Pos2::new(px, yax_px + 4.0)],
                    Stroke::new(1.0_f32, black),
                );
                if label_x && k.abs() > 1e-12 {
                    let txt = if self.page.log_x {
                        format!("10^{}", fmt_pow(k))
                    } else {
                        fmt_label(k)
                    };
                    painter.text(
                        Pos2::new(px, yax_px + 8.0),
                        Align2::CENTER_TOP,
                        txt,
                        FontId::proportional(10.0),
                        gray,
                    );
                }
            }
        }
        if has_y_axis {
            let xax_px = page_rect.left() + (xax as f32) * ppm;
            for &k in &majors_y {
                let my = self.page.mm_y(k);
                if !my.is_finite() {
                    continue;
                }
                let py = page_rect.top() + (my as f32) * ppm;
                if py <= page_rect.top() + 1.0 || py >= page_rect.bottom() - 1.0 {
                    continue;
                }
                painter.line_segment(
                    [Pos2::new(xax_px - 4.0, py), Pos2::new(xax_px + 4.0, py)],
                    Stroke::new(1.0_f32, black),
                );
                if label_y && k.abs() > 1e-12 {
                    let txt = if self.page.log_y {
                        format!("10^{}", fmt_pow(k))
                    } else {
                        fmt_label(k)
                    };
                    painter.text(
                        Pos2::new(xax_px - 6.0, py),
                        Align2::RIGHT_CENTER,
                        txt,
                        FontId::proportional(10.0),
                        gray,
                    );
                }
            }
        }
    }

    /// Dessine les annotations : aires hachurées (remplissage translucide +
    /// trame diagonale 45°), tangentes (segment + point de tangence) et
    /// points (marqueur + coordonnées).
    fn dessine_annots(&self, painter: &egui::Painter, page_rect: Rect, ppm: f32) {
        let to_px = |mm: (f64, f64)| {
            Pos2::new(
                page_rect.min.x + mm.0 as f32 * ppm,
                page_rect.min.y + mm.1 as f32 * ppm,
            )
        };

        // aires hachurées
        for a in &self.annots {
            if let AnnotDraw::Integrale {
                path,
                label,
                label_mm,
                color,
                ..
            } = a
            {
                let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 42);
                // remplissage : ruban vertical (quad par échantillon consécutif)
                let samples = if path.len() >= 3 {
                    &path[1..path.len() - 1]
                } else {
                    &[][..]
                };
                if samples.len() >= 2 {
                    let axis_y = path[0].1;
                    let mut mesh = egui::Mesh::default();
                    for &(mx, my) in samples {
                        mesh.colored_vertex(to_px((mx, axis_y)), col);
                        mesh.colored_vertex(to_px((mx, my)), col);
                    }
                    let n = samples.len();
                    for i in 0..n - 1 {
                        let (a0, b0) = (2 * i, 2 * i + 1);
                        let (a1, b1) = (2 * i + 2, 2 * i + 3);
                        mesh.add_triangle(a0 as u32, b0 as u32, b1 as u32);
                        mesh.add_triangle(a0 as u32, b1 as u32, a1 as u32);
                    }
                    painter.add(egui::Shape::Mesh(mesh));
                }
                // trame diagonale (45°), même pas qu'en export (1,8 mm)
                let hatch_col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 170);
                for ((x0, y0), (x1, y1)) in annot::hatch_segments(path, 1.8) {
                    painter.line_segment(
                        [to_px((x0, y0)), to_px((x1, y1))],
                        Stroke::new(1.0_f32, hatch_col),
                    );
                }
                // valeur de l'aire
                painter.text(
                    to_px(*label_mm) - Vec2::new(0.0, 5.0),
                    Align2::CENTER_BOTTOM,
                    format!("∫ ≈ {label}"),
                    FontId::proportional(11.0),
                    Color32::from_rgb(color[0], color[1], color[2]),
                );
            }
        }

        // tangentes puis points
        for a in &self.annots {
            match a {
                AnnotDraw::Tangente {
                    x0,
                    y0,
                    x1,
                    y1,
                    mm,
                    label,
                    color,
                    ..
                } => {
                    let col = Color32::from_rgb(color[0], color[1], color[2]);
                    painter.line_segment(
                        [to_px((*x0, *y0)), to_px((*x1, *y1))],
                        Stroke::new(1.5_f32, col),
                    );
                    let c = to_px(*mm);
                    painter.circle_filled(c, 5.0, Color32::WHITE);
                    painter.circle_stroke(c, 5.0, Stroke::new(2.0_f32, col));
                    painter.text(
                        c + Vec2::new(8.0, -6.0),
                        Align2::LEFT_BOTTOM,
                        label.as_str(),
                        FontId::proportional(11.0),
                        col,
                    );
                }
                AnnotDraw::Point {
                    mm, label, color, ..
                } => {
                    let col = Color32::from_rgb(color[0], color[1], color[2]);
                    let c = to_px(*mm);
                    painter.circle_filled(c, 5.0, Color32::WHITE);
                    painter.circle_stroke(c, 5.0, Stroke::new(2.0_f32, col));
                    painter.text(
                        c + Vec2::new(8.0, -6.0),
                        Align2::LEFT_BOTTOM,
                        label.as_str(),
                        FontId::proportional(11.0),
                        col,
                    );
                }
                AnnotDraw::Integrale { .. } => {}
            }
        }
    }

    fn dessine_courbes(&self, painter: &egui::Painter, page_rect: Rect, ppm: f32) {
        for (i, curve) in self.curves.iter().enumerate() {
            if !curve.visible {
                continue;
            }
            let Ok(polys) = &self.poly[i] else { continue };
            let color = Color32::from_rgb(curve.color[0], curve.color[1], curve.color[2]);
            let stroke = Stroke::new(curve.thickness.max(0.5), color);
            let expanded = page_rect.expand(60.0);
            for poly in polys {
                let mut pts: Vec<Pos2> = Vec::with_capacity(poly.len());
                for &(mx, my) in poly {
                    let p = Pos2::new(
                        page_rect.min.x + (mx as f32) * ppm,
                        page_rect.min.y + (my as f32) * ppm,
                    );
                    if !p.x.is_finite() || !p.y.is_finite() {
                        continue;
                    }
                    if !expanded.contains(p) {
                        if pts.len() >= 2 {
                            painter.add(egui::Shape::line(std::mem::take(&mut pts), stroke));
                        } else {
                            pts.clear();
                        }
                        continue;
                    }
                    pts.push(p);
                }
                if pts.len() >= 2 {
                    painter.add(egui::Shape::line(pts, stroke));
                }
            }
        }
    }
}

const PAS_MM_F32: f32 = 5.0;

fn fmt_label(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
}

fn fmt_pow(v: f64) -> i64 {
    v.log10().round() as i64
}

impl eframe::App for SqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let cols = (ctx.input(|i| i.screen_rect.width()) / 2.0).max(64.0) as usize;
        self.refresh(cols);
        self.panneau_lateral(ctx);
        egui::CentralPanel::default().show(ctx, |ui| self.zone_dessin(ui));
        self.fenetre_tableau(ctx);
        self.fenetre_genere(ctx);
        self.fenetre_stat(ctx);
        self.fenetre_suite(ctx);
        self.fenetre_serie2(ctx);
        self.fenetre_arbre(ctx);
        self.fenetre_simulation(ctx);
        self.fenetre_calculs(ctx);
        // sauvegarde automatique si nécessaire
        if self.dirty {
            let s = Session {
                version: 1,
                page: self.page,
                curves: self.curves.clone(),
                lang: self.lang,
                points: self.points.clone(),
                tangentes: self.tangentes.clone(),
                integrales: self.integrales.clone(),
                tableaux: self.tableaux.clone(),
                stats: self.stats.clone(),
                boites: self.boites.clone(),
                suites: self.suites.clone(),
                serie2: self.serie2.clone(),
                arbres: self.arbres.clone(),
                simus: self.simus.clone(),
            };
            if session::save(&s).is_ok() {
                self.dirty = false;
            }
        }
    }
}
