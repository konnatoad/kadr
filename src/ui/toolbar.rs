use crate::fs::sorter::SortMode;
use crate::ui::widgets::theme;
use egui::{Color32, Painter, Rect, RichText, Ui};

#[derive(Default)]
pub struct ToolbarResponse {
    pub open_folder: bool,
    pub add_folder: bool,
    pub open_file: bool,
    pub combine: bool,
    pub settings: bool,
    pub sort_changed: Option<SortMode>,
    pub toggle_images: bool,
    pub toggle_videos: bool,
    pub toggle_subfolders: bool,
    pub slideshow: bool,
}

#[allow(clippy::too_many_arguments)]
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
    let right_w = right_cluster_width(ui, current_index, image_count);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;

        if text_action(ui, icon_folder, "Folders") {
            resp.open_folder = true;
        }
        if text_action(ui, icon_add_folder, "Add") {
            resp.add_folder = true;
        }
        if text_action(ui, icon_file, "File") {
            resp.open_file = true;
        }

        dot(ui);

        if let Some(mode) = sort_cluster(ui, current_sort) {
            resp.sort_changed = Some(mode);
        }
        if spec_token(ui, "IMG", filter_images) {
            resp.toggle_images = true;
        }
        if spec_token(ui, "VID", filter_videos) {
            resp.toggle_videos = true;
        }
        if spec_token(ui, "sub", scan_subfolders) {
            resp.toggle_subfolders = true;
        }

        let remaining = (ui.available_width() - right_w - 32.0).max(0.0);
        ui.add_space(remaining);

        if icon_action(ui, icon_gear) {
            resp.settings = true;
        }
        if icon_action(ui, icon_combine) {
            resp.combine = true;
        }
        if slideshow_active {
            if icon_action_base(ui, icon_stop, theme::ERROR_TEXT, theme::ERROR_TEXT) {
                resp.slideshow = true;
            }
        } else if icon_action(ui, icon_slideshow) {
            resp.slideshow = true;
        }

        dot(ui);

        readout(ui, current_index, image_count);
    });

    let rect = ui.min_rect();
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.bottom() + 1.0,
        egui::Stroke::new(1.0, theme::BORDER),
    );

    resp
}

// ── Compact controls ──────────────────────────────────────────────────────────

fn mono(size: f32) -> egui::FontId {
    egui::FontId::monospace(size)
}

fn measure(ui: &Ui, text: &str, font: egui::FontId) -> egui::Vec2 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::WHITE)
        .size()
}

fn readout_label(current_index: Option<usize>, total: usize) -> String {
    match current_index {
        Some(idx) => format!("{}⁄{}", idx + 1, total),
        None => "—".to_owned(),
    }
}

fn right_cluster_width(ui: &Ui, current_index: Option<usize>, total: usize) -> f32 {
    let icon_size = 14.0;
    let dot_w = measure(ui, "·", mono(13.0)).x;
    let readout_w = measure(ui, &readout_label(current_index, total), mono(12.5))
        .x
        .max(72.0);
    let pieces = [readout_w, dot_w, icon_size, icon_size, icon_size];
    pieces.iter().sum::<f32>() + 16.0 * (pieces.len() as f32 - 1.0)
}

fn dot(ui: &mut Ui) {
    ui.label(RichText::new("·").font(mono(13.0)).color(theme::TEXT_MUTED));
}

fn text_action(ui: &mut Ui, icon: impl FnOnce(&Painter, Rect, Color32), label: &str) -> bool {
    let icon_size = 13.0;
    let gap = 7.0;
    let font = mono(12.5);
    let text_size = measure(ui, label, font.clone());
    let desired = egui::vec2(icon_size + gap + text_size.x, icon_size.max(text_size.y));
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    let col = if resp.hovered() {
        theme::ACCENT_TEXT
    } else {
        theme::TEXT_DIM
    };
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let icon_rect = Rect::from_min_size(
        egui::pos2(rect.min.x, rect.center().y - icon_size / 2.0),
        egui::vec2(icon_size, icon_size),
    );
    icon(ui.painter(), icon_rect, col);

    ui.painter().text(
        egui::pos2(icon_rect.right() + gap, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        col,
    );

    resp.clicked()
}

fn icon_action(ui: &mut Ui, icon: impl FnOnce(&Painter, Rect, Color32)) -> bool {
    icon_action_base(ui, icon, theme::TEXT_DIM, theme::TEXT)
}

fn icon_action_base(
    ui: &mut Ui,
    icon: impl FnOnce(&Painter, Rect, Color32),
    base: Color32,
    hover: Color32,
) -> bool {
    let size = 14.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let col = if resp.hovered() { hover } else { base };
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    icon(ui.painter(), rect, col);
    resp.clicked()
}

fn spec_token(ui: &mut Ui, label: &str, active: bool) -> bool {
    let font = mono(12.5);
    let base = if active {
        theme::ACCENT_TEXT
    } else {
        theme::TEXT_MUTED
    };
    let size = measure(ui, label, font.clone());
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let col = if resp.hovered() {
        if active { theme::ACCENT_TEXT } else { theme::TEXT }
    } else {
        base
    };
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter()
        .text(rect.center(), egui::Align2::CENTER_CENTER, label, font, col);
    resp.clicked()
}

fn sort_short_label(mode: &SortMode) -> (&'static str, &'static str) {
    match mode {
        SortMode::Name => ("NAME", "↓"),
        SortMode::NameReverse => ("NAME", "↑"),
        SortMode::Size => ("SIZE", "↓"),
        SortMode::SizeReverse => ("SIZE", "↑"),
        SortMode::Modified => ("DATE", "↓"),
        SortMode::ModifiedReverse => ("DATE", "↑"),
        SortMode::Type => ("TYPE", ""),
        SortMode::Random => ("RANDOM", ""),
    }
}

fn sort_cluster(ui: &mut Ui, current: &SortMode) -> Option<SortMode> {
    let (word, arrow) = sort_short_label(current);
    let font = mono(12.5);
    let word_size = measure(ui, word, font.clone());
    let arrow_w = if arrow.is_empty() {
        0.0
    } else {
        measure(ui, arrow, font.clone()).x
    };
    let desired = egui::vec2(word_size.x + arrow_w, word_size.y);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    ui.painter().text(
        rect.left_center(),
        egui::Align2::LEFT_CENTER,
        word,
        font.clone(),
        theme::TEXT,
    );
    if !arrow.is_empty() {
        ui.painter().text(
            egui::pos2(rect.left() + word_size.x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            arrow,
            font,
            theme::ACCENT_TEXT,
        );
    }

    if resp.clicked() {
        let all = SortMode::all();
        let idx = all.iter().position(|m| m == current).unwrap_or(0);
        Some(all[(idx + 1) % all.len()].clone())
    } else {
        None
    }
}

fn readout(ui: &mut Ui, current_index: Option<usize>, total: usize) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 5.0;
        let label = readout_label(current_index, total);
        ui.label(RichText::new(label).font(mono(12.5)).color(theme::TEXT));

        let track_w = 72.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(track_w, 2.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, theme::BORDER);
        if let Some(idx) = current_index
            && total > 0
        {
            let frac = (idx + 1) as f32 / total as f32;
            let filled = Rect::from_min_size(rect.min, egui::vec2(track_w * frac, 2.0));
            ui.painter().rect_filled(filled, 0.0, theme::ACCENT);
        }
    });
}

// ── Hand-drawn toolbar icons ─────────────────────────────────────────────────

fn icon_folder(p: &Painter, r: Rect, col: Color32) {
    let stroke = egui::Stroke::new(1.3, col);
    let tab_h = r.height() * 0.24;
    let tab = Rect::from_min_size(r.min, egui::vec2(r.width() * 0.5, tab_h));
    let body = Rect::from_min_max(egui::pos2(r.min.x, r.min.y + tab_h * 0.7), r.max);
    p.rect_stroke(tab, 1.5, stroke, egui::StrokeKind::Outside);
    p.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Outside);
}

fn icon_add_folder(p: &Painter, r: Rect, col: Color32) {
    icon_folder(p, r, col);
    let stroke = egui::Stroke::new(1.3, col);
    let c = r.center() + egui::vec2(0.0, r.height() * 0.12);
    let s = r.width() * 0.14;
    p.line_segment([c - egui::vec2(s, 0.0), c + egui::vec2(s, 0.0)], stroke);
    p.line_segment([c - egui::vec2(0.0, s), c + egui::vec2(0.0, s)], stroke);
}

fn icon_file(p: &Painter, r: Rect, col: Color32) {
    let stroke = egui::Stroke::new(1.3, col);
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
    let stroke = egui::Stroke::new(1.3, col);
    let s = r.width() * 0.62;
    let r1 = Rect::from_min_size(r.min, egui::vec2(s, s));
    let r2 = Rect::from_min_size(
        r.min + egui::vec2(r.width() - s, r.height() - s),
        egui::vec2(s, s),
    );
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
        egui::Stroke::NONE,
    ));
}

fn icon_stop(p: &Painter, r: Rect, col: Color32) {
    let s = r.shrink(r.width() * 0.22);
    p.rect_filled(s, 1.0, col);
}

fn icon_gear(p: &Painter, r: Rect, col: Color32) {
    let c = r.center();
    let radius = r.width() * 0.5;
    p.circle_stroke(c, radius * 0.5, egui::Stroke::new(1.3, col));
    let teeth = 8;
    for i in 0..teeth {
        let angle = (i as f32 / teeth as f32) * std::f32::consts::TAU;
        let dir = egui::vec2(angle.cos(), angle.sin());
        let inner = c + dir * (radius * 0.72);
        let outer = c + dir * radius;
        p.line_segment([inner, outer], egui::Stroke::new(1.6, col));
    }
}
