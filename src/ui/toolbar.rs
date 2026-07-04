use egui::{Color32, Painter, Rect, RichText, Stroke, Ui};

use crate::fs::sorter::SortMode;
use crate::ui::widgets::{self, theme};

pub struct ToolbarResponse {
    pub open_folder: bool,
    pub open_file: bool,
    pub combine: bool,
    pub settings: bool,
    pub sort_changed: Option<SortMode>,
    pub toggle_images: bool,
    pub toggle_videos: bool,
    pub toggle_subfolders: bool,
    pub slideshow: bool,
}

impl Default for ToolbarResponse {
    fn default() -> Self {
        Self {
            open_folder: false,
            open_file: false,
            combine: false,
            settings: false,
            sort_changed: None,
            toggle_images: false,
            toggle_videos: false,
            toggle_subfolders: false,
            slideshow: false,
        }
    }
}

pub fn show_toolbar(
    ui: &mut Ui,
    current_sort: &SortMode,
    filter_images: bool,
    filter_videos: bool,
    scan_subfolders: bool,
    slideshow_active: bool,
    image_count: usize,
    current_index: Option<usize>,
) -> ToolbarResponse {
    let mut resp = ToolbarResponse::default();

    widgets::card_frame()
        .inner_margin(egui::Margin::symmetric(14i8, 9i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                if widgets::icon_button(ui, icon_folder, "Open Folder").clicked() { resp.open_folder = true; }
                if widgets::icon_button(ui, icon_file, "Open File").clicked()     { resp.open_file   = true; }

                widgets::vsep(ui);

                egui::ComboBox::from_id_salt("sort_mode")
                    .selected_text(current_sort.label())
                    .width(168.0)
                    .show_ui(ui, |ui| {
                        for mode in SortMode::all() {
                            if ui.selectable_label(current_sort == mode, mode.label()).clicked() {
                                resp.sort_changed = Some(mode.clone());
                            }
                        }
                    });

                widgets::vsep(ui);

                if widgets::pill_toggle(ui, "Images", filter_images).clicked()       { resp.toggle_images = true; }
                if widgets::pill_toggle(ui, "Videos", filter_videos).clicked()       { resp.toggle_videos = true; }
                if widgets::pill_toggle(ui, "Subfolders", scan_subfolders).clicked() { resp.toggle_subfolders = true; }

                widgets::vsep(ui);

                if slideshow_active {
                    let btn = egui::Button::new(
                        RichText::new("Stop").color(theme::ERROR_TEXT).size(12.5),
                    )
                    .fill(theme::error_fill(25))
                    .stroke(Stroke::new(1.0, theme::error_fill(110)))
                    .corner_radius(theme::RADIUS_SM);
                    if ui.add(btn).clicked() { resp.slideshow = true; }
                } else if widgets::icon_button(ui, icon_slideshow, "Slideshow").clicked() {
                    resp.slideshow = true;
                }

                widgets::vsep(ui);
                if widgets::icon_button(ui, icon_combine, "Combine").clicked() { resp.combine = true; }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if widgets::icon_only_button(ui, icon_gear, 28.0).clicked() { resp.settings = true; }

                    if let Some(idx) = current_index {
                        widgets::vsep(ui);
                        ui.label(
                            RichText::new(format!("{} / {}", idx + 1, image_count))
                                .color(theme::TEXT_MUTED)
                                .size(11.5)
                                .monospace(),
                        );
                    }
                });
            });
        });

    resp
}

// ── Hand-drawn toolbar icons ─────────────────────────────────────────────────

fn icon_folder(p: &Painter, r: Rect, col: Color32) {
    let stroke = Stroke::new(1.3, col);
    let tab_h = r.height() * 0.24;
    let tab = Rect::from_min_size(r.min, egui::vec2(r.width() * 0.5, tab_h));
    let body = Rect::from_min_max(egui::pos2(r.min.x, r.min.y + tab_h * 0.7), r.max);
    p.rect_stroke(tab, 1.5, stroke, egui::StrokeKind::Outside);
    p.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Outside);
}

fn icon_file(p: &Painter, r: Rect, col: Color32) {
    let stroke = Stroke::new(1.3, col);
    let body = r.shrink2(egui::vec2(r.width() * 0.16, 0.0));
    p.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Outside);
    let x0 = body.min.x + body.width() * 0.22;
    let x1 = body.max.x - body.width() * 0.22;
    for frac in [0.4_f32, 0.62] {
        let y = body.min.y + body.height() * frac;
        p.line_segment([egui::pos2(x0, y), egui::pos2(x1, y)], stroke);
    }
}

fn icon_combine(p: &Painter, r: Rect, col: Color32) {
    let stroke = Stroke::new(1.3, col);
    let s = r.width() * 0.62;
    let r1 = Rect::from_min_size(r.min, egui::vec2(s, s));
    let r2 = Rect::from_min_size(r.min + egui::vec2(r.width() - s, r.height() - s), egui::vec2(s, s));
    p.rect_stroke(r1, 2.0, stroke, egui::StrokeKind::Outside);
    p.rect_stroke(r2, 2.0, stroke, egui::StrokeKind::Outside);
}

fn icon_slideshow(p: &Painter, r: Rect, col: Color32) {
    let c = r.center();
    let h = r.height() * 0.42;
    p.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(c.x - h * 0.5, c.y - h),
            egui::pos2(c.x - h * 0.5, c.y + h),
            egui::pos2(c.x + h * 0.9, c.y),
        ],
        col,
        Stroke::NONE,
    ));
}

fn icon_gear(p: &Painter, r: Rect, col: Color32) {
    let c = r.center();
    let radius = r.width() * 0.5;
    p.circle_stroke(c, radius * 0.5, Stroke::new(1.3, col));
    let teeth = 8;
    for i in 0..teeth {
        let angle = (i as f32 / teeth as f32) * std::f32::consts::TAU;
        let dir = egui::vec2(angle.cos(), angle.sin());
        let inner = c + dir * (radius * 0.72);
        let outer = c + dir * radius;
        p.line_segment([inner, outer], Stroke::new(1.6, col));
    }
}
