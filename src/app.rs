//! Interface graphique (egui/eframe) — feuille quadrillée 5 × 5 mm,
//! liste de courbes, repère paramétrable, annotations (points, tangentes,
//! intégrales hachurées) et export SVG/PNG.

use crate::annot::{self, draw_list, AnnotDraw, IntegraleAnnot, PointAnnot, TangenteAnnot};
use crate::curve::{apply_style, sample_curve, Curve, CurveKind, KindId, PALETTE};
use crate::grid::Page;
use crate::i18n::{self, Lang};
use crate::render::{self, CurvePolys};
use crate::session::{self, Session};
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
        let (page, curves, saved_lang, points, tangentes, integrales, tableaux) =
            match session::load() {
                Some(s) => (
                    s.page,
                    s.curves,
                    Some(s.lang),
                    s.points,
                    s.tangentes,
                    s.integrales,
                    s.tableaux,
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
                match render::write_svg_file(&self.page, &data, &self.annots, &self.tableaux, &path)
                {
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
                    self.px_per_mm as f64,
                    &path,
                ) {
                    Ok(()) => self.set_status(format!("✔ {} écrit", path.display())),
                    Err(e) => self.set_status(format!("✖ Échec PNG : {e}")),
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
            };
            if session::save(&s).is_ok() {
                self.dirty = false;
            }
        }
    }
}
