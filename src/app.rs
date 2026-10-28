//! Interface graphique (egui/eframe) — feuille quadrillée 5 × 5 mm,
//! liste de courbes, repère paramétrable et export SVG/PNG.

use crate::curve::{apply_style, sample_curve, Curve, CurveKind, KindId, PALETTE};
use crate::grid::Page;
use crate::render::{self, CurvePolys};
use crate::session::{self, Session};
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
}

impl SqApp {
    pub fn new() -> Self {
        let (page, curves) = match session::load() {
            Some(s) => (s.page, s.curves),
            None => (
                Page::default(),
                vec![
                    Curve::fonction("sin(x)", PALETTE[0]),
                    Curve::fonction("x^2/4 - 2", PALETTE[1]),
                ],
            ),
        };
        let selected = (!curves.is_empty()).then_some(0usize);
        Self {
            page,
            curves,
            selected,
            zoom: 1.0,
            view_offset: Vec2::ZERO,
            px_per_mm: 4.0,
            status: None,
            dirty: false,
            poly: Vec::new(),
        }
    }

    fn set_status(&mut self, msg: String) {
        self.status = Some((msg, Instant::now()));
    }

    /// Recalcule toutes les polylignes (appelé à chaque frame).
    fn refresh(&mut self, cols: usize) {
        let mut poly = Vec::with_capacity(self.curves.len());
        for c in &self.curves {
            poly.push(sample_curve(c, &self.page, cols, 4000).map(|ps| apply_style(ps, c.style)));
        }
        self.poly = poly;
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
                        ui.label(
                            RichText::new("Réécriture en Rust — traceur de courbes")
                                .small()
                                .weak(),
                        );
                        ui.separator();
                        self.section_liste(ui);
                        ui.separator();
                        self.section_proprietes(ui);
                        ui.separator();
                        self.section_repere(ui);
                        ui.separator();
                        self.section_export(ui);
                    });
            });
    }

    fn section_liste(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.strong("Courbes");
            ui.menu_button("➕ Ajouter", |ui| {
                let n = self.curves.len();
                let color = PALETTE[n % PALETTE.len()];
                if ui.button("Fonction  y = f(x)").clicked() {
                    self.curves.push(Curve::fonction("sin(x)", color));
                    self.selected = Some(self.curves.len() - 1);
                    self.dirty = true;
                    ui.close_menu();
                }
                if ui.button("Courbe paramétrée").clicked() {
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
                if ui.button("Courbe polaire").clicked() {
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
            if ui.button("Vider").clicked() {
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
        ui.strong("Propriétés de la courbe");
        let Some(sel) = self.selected else {
            ui.label(RichText::new("Aucune courbe sélectionnée.").weak().small());
            return;
        };
        let Some(curve) = self.curves.get_mut(sel) else {
            self.selected = None;
            return;
        };

        // nature
        let kind_id = curve.kind.kind_id();
        ComboBox::from_label("Nature")
            .selected_text(match kind_id {
                KindId::Fonction => "Fonction y = f(x)",
                KindId::Parametree => "Paramétrée",
                KindId::Polaire => "Polaire",
            })
            .show_ui(ui, |ui| {
                for (id, name) in [
                    (KindId::Fonction, "Fonction y = f(x)"),
                    (KindId::Parametree, "Paramétrée"),
                    (KindId::Polaire, "Polaire"),
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
                    ui.label("t de");
                    if ui.add(DragValue::new(tmin).speed(0.1)).changed() {
                        self.dirty = true;
                    }
                    ui.label("à");
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
                    ui.label("θ de");
                    if ui.add(DragValue::new(tmin).speed(0.1)).changed() {
                        self.dirty = true;
                    }
                    ui.label("à");
                    if ui.add(DragValue::new(tmax).speed(0.1)).changed() {
                        self.dirty = true;
                    }
                });
            }
        }

        // style
        ui.horizontal(|ui| {
            ui.label("Couleur :");
            let old = Color32::from_rgb(curve.color[0], curve.color[1], curve.color[2]);
            let mut c = old;
            ui.color_edit_button_srgba(&mut c);
            if c != old {
                curve.color = [c.r(), c.g(), c.b()];
                self.dirty = true;
            }
            ui.label("Épaisseur :");
            if ui
                .add(egui::Slider::new(&mut curve.thickness, 0.5..=6.0))
                .changed()
            {
                self.dirty = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Style :");
            for (st, name) in [
                (crate::curve::LineStyle::Continu, "Continu"),
                (crate::curve::LineStyle::Tirets, "Tirets"),
                (crate::curve::LineStyle::Pointilles, "Pointillés"),
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

    fn section_repere(&mut self, ui: &mut Ui) {
        ui.strong("Repère");
        ui.horizontal(|ui| {
            ui.label("Origine (");
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
            ui.label("Largeur :");
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
            ui.label("Hauteur :");
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
            ui.label("Unité :");
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
            ui.small("grille 5 × 5 mm");
        });
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.page.log_x, "log X").changed() {
                self.dirty = true;
            }
            if ui.checkbox(&mut self.page.log_y, "log Y").changed() {
                self.dirty = true;
            }
            if ui.button("Recentrer").clicked() {
                self.page.cx = 0.0;
                self.page.cy = 0.0;
                self.zoom = 1.0;
                self.view_offset = Vec2::ZERO;
                self.dirty = true;
            }
        });
    }

    fn section_export(&mut self, ui: &mut Ui) {
        ui.strong("Export");
        ui.horizontal(|ui| {
            if ui.button("SVG (sinequanon.svg)").clicked() {
                let data = self.export_data();
                let path = PathBuf::from("sinequanon.svg");
                match render::write_svg_file(&self.page, &data, &path) {
                    Ok(n) => self.set_status(format!("✔ {} écrit ({} octets)", path.display(), n)),
                    Err(e) => self.set_status(format!("✖ Échec SVG : {e}")),
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("PNG (sinequanon.png)").clicked() {
                let data = self.export_data();
                let path = PathBuf::from("sinequanon.png");
                match render::write_png_file(&self.page, &data, self.px_per_mm as f64, &path) {
                    Ok(()) => self.set_status(format!("✔ {} écrit", path.display())),
                    Err(e) => self.set_status(format!("✖ Échec PNG : {e}")),
                }
            }
            ui.label("Résolution :");
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
        // sauvegarde automatique si nécessaire
        if self.dirty {
            let s = Session {
                version: 1,
                page: self.page,
                curves: self.curves.clone(),
            };
            if session::save(&s).is_ok() {
                self.dirty = false;
            }
        }
    }
}
