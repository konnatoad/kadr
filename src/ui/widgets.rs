use egui::{Color32, RichText, Stroke, Ui};

pub mod theme {
    use egui::Color32;

    pub const BG: Color32 = Color32::from_gray(6);
    pub const SURFACE: Color32 = Color32::from_gray(16);
    pub const SURFACE2: Color32 = Color32::from_gray(22);
    pub const SURFACE3: Color32 = Color32::from_gray(30);
    pub const SURFACE4: Color32 = Color32::from_gray(48);
    pub const BORDER: Color32 = Color32::from_gray(40);
    pub const ACCENT: Color32 = Color32::from_rgb(60, 130, 220);
    pub const ACCENT_TEXT: Color32 = Color32::from_rgb(84, 156, 240);
    pub const ACCENT2: Color32 = Color32::from_rgb(0xbb, 0x9a, 0xf7);
    pub const TEXT: Color32 = Color32::from_gray(225);
    pub const TEXT_DIM: Color32 = Color32::from_gray(145);
    pub const TEXT_MUTED: Color32 = Color32::from_gray(100);
    pub const SUCCESS: Color32 = Color32::from_rgb(0x9e, 0xce, 0x6a);
    pub const WARNING: Color32 = Color32::from_rgb(0xe0, 0xaf, 0x67);
    pub const ERROR_TEXT: Color32 = Color32::from_rgb(0xf7, 0x76, 0x8e);
    pub const RADIUS: f32 = 5.0;
    pub const RADIUS_SM: f32 = 3.0;
    pub const ORANGE: Color32 = Color32::from_rgb(0xff, 0x9e, 0x64);

    pub fn error_bg() -> Color32 {
        Color32::from_rgba_unmultiplied(0xf7, 0x76, 0x8e, 28)
    }
    pub fn overlay_bg() -> Color32 {
        Color32::from_rgba_unmultiplied(6, 6, 6, 210)
    }
    pub fn accent_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(60, 130, 220, alpha)
    }
    #[allow(dead_code)]
    pub fn accent2_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0xbb, 0x9a, 0xf7, alpha)
    }
    pub fn error_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0xf7, 0x76, 0x8e, alpha)
    }
    pub fn warning_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0xe0, 0xaf, 0x68, alpha)
    }
    pub fn white_wash(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
    }
}

pub fn accent_button(ui: &mut Ui, label: &str) -> egui::Response {
    accent_button_sized(ui, label, 13.0, 40, 170)
}

pub fn accent_button_sized(
    ui: &mut Ui,
    label: &str,
    font_size: f32,
    fill_alpha: u8,
    stroke_alpha: u8,
) -> egui::Response {
    let enabled = ui.is_enabled();
    let text_col = if enabled {
        theme::ACCENT_TEXT
    } else {
        theme::TEXT_MUTED
    };
    let scale = if enabled { 1.0 } else { 0.4 };
    let btn = egui::Button::new(RichText::new(label).size(font_size).color(text_col))
        .fill(theme::accent_fill((fill_alpha as f32 * scale) as u8))
        .stroke(Stroke::new(
            1.0,
            theme::accent_fill((stroke_alpha as f32 * scale) as u8),
        ))
        .corner_radius(theme::RADIUS_SM);
    ui.add(btn)
}

#[allow(dead_code)]
pub fn action_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.button(RichText::new(label).size(12.5))
}

pub fn pill_checkbox(ui: &mut Ui, value: &mut bool, label: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let mut response = toggle_switch(ui, value);
        let label_resp = ui.add(egui::Label::new(label).sense(egui::Sense::click()));
        if label_resp.clicked() {
            *value = !*value;
            response.mark_changed();
        }
        response
    })
    .inner
}

fn toggle_switch(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let height = ui.spacing().interact_size.y.max(16.0) * 0.78;
    let size = egui::vec2(height * 1.85, height);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let mut track_stroke = theme::BORDER;
        if response.hovered() {
            track_stroke = theme::TEXT_MUTED;
        }
        let track_stroke = track_stroke.lerp_to_gamma(theme::ACCENT, t);
        let track_fill = theme::SURFACE.lerp_to_gamma(theme::accent_fill(28), t);
        ui.painter().rect(
            rect,
            theme::RADIUS_SM,
            track_fill,
            Stroke::new(1.0, track_stroke),
            egui::StrokeKind::Inside,
        );
        let inset = 3.0;
        let knob_size = rect.height() - inset * 2.0;
        let knob_x = egui::lerp((rect.left() + inset)..=(rect.right() - inset - knob_size), t);
        let knob_rect = egui::Rect::from_min_size(
            egui::pos2(knob_x, rect.top() + inset),
            egui::vec2(knob_size, knob_size),
        );
        let knob_col = theme::TEXT_MUTED.lerp_to_gamma(theme::ACCENT_TEXT, t);
        ui.painter().rect_filled(knob_rect, 1.0, knob_col);
    }
    response
}

pub fn sliding_tab_bar(ui: &mut Ui, labels: &[&str], active: usize) -> Option<usize> {
    let font = egui::FontId::monospace(12.5);
    let pad_x = 14.0_f32;
    let height = 28.0_f32;
    let gap = 2.0_f32;
    let mut rel_rects = Vec::with_capacity(labels.len());
    let mut x = 0.0_f32;
    for label in labels {
        let galley = ui
            .painter()
            .layout_no_wrap((*label).to_string(), font.clone(), theme::TEXT);
        let w = galley.size().x + pad_x * 2.0;
        rel_rects.push(egui::Rect::from_min_size(
            egui::pos2(x, 0.0),
            egui::vec2(w, height),
        ));
        x += w + gap;
    }
    let total_w = (x - gap).max(0.0);

    let (row_rect, _) = ui.allocate_exact_size(egui::vec2(total_w, height), egui::Sense::hover());
    let rects: Vec<egui::Rect> = rel_rects
        .iter()
        .map(|r| r.translate(row_rect.min.to_vec2()))
        .collect();
    let active = active.min(rects.len().saturating_sub(1));
    let target = rects[active];
    let base_id = ui.id().with("sliding_tab_bar");
    let anim_x = ui
        .ctx()
        .animate_value_with_time(base_id.with("x"), target.min.x, 0.2);
    let anim_w = ui
        .ctx()
        .animate_value_with_time(base_id.with("w"), target.width(), 0.2);
    let underline = egui::Rect::from_min_size(
        egui::pos2(anim_x, row_rect.bottom() - 2.0),
        egui::vec2(anim_w, 2.0),
    );
    ui.painter().rect_filled(underline, 0.0, theme::ACCENT);

    let mut clicked = None;
    for (i, (label, rect)) in labels.iter().zip(rects.iter()).enumerate() {
        let resp = ui.interact(*rect, base_id.with(("tab", i)), egui::Sense::click());
        let text_col = if i == active {
            theme::ACCENT_TEXT
        } else if resp.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            text_col,
        );
        if resp.clicked() {
            clicked = Some(i);
        }
    }

    clicked
}

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(egui::FontId::monospace(11.0))
            .color(theme::TEXT_DIM)
            .strong(),
    );
}

pub fn error_banner(ui: &mut Ui, message: &str) {
    egui::Frame::default()
        .fill(theme::error_bg())
        .corner_radius(theme::RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(10i8, 6i8))
        .show(ui, |ui| {
            ui.colored_label(theme::ERROR_TEXT, message);
        });
}
