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
    pub const PILL_ACCENT: Color32 = Color32::from_rgb(0xff, 0x69, 0xb4);
    pub const TEXT: Color32 = Color32::from_gray(225);
    pub const TEXT_DIM: Color32 = Color32::from_gray(145);
    pub const TEXT_MUTED: Color32 = Color32::from_gray(100);
    pub const SUCCESS: Color32 = Color32::from_rgb(0x9e, 0xce, 0x6a);
    pub const WARNING: Color32 = Color32::from_rgb(0xe0, 0xaf, 0x67);
    pub const ERROR_TEXT: Color32 = Color32::from_rgb(0xf7, 0x76, 0x8e);
    pub const RADIUS: f32 = 9.0;
    pub const RADIUS_SM: f32 = 6.0;
    pub const RADIUS_PILL: f32 = 999.0;
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
    pub fn pill_accent_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0xff, 0x69, 0xb4, alpha)
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

pub fn card_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS)
}

pub fn icon_button(
    ui: &mut Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
    label: &str,
) -> egui::Response {
    let icon_size = 15.0_f32;
    let gap = 7.0_f32;
    let pad = egui::vec2(10.0, 6.0);
    let font = egui::FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, theme::TEXT);

    let content_w = icon_size + gap + galley.size().x;
    let desired = egui::vec2(
        content_w + pad.x * 2.0,
        icon_size.max(galley.size().y) + pad.y * 2.0,
    );
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, theme::RADIUS_SM, theme::SURFACE4);
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let icon_col = if resp.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + pad.x, rect.center().y - icon_size / 2.0),
        egui::vec2(icon_size, icon_size),
    );
    icon(ui.painter(), icon_rect, icon_col);

    let text_pos = egui::pos2(
        icon_rect.right() + gap,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, theme::TEXT);

    resp
}

pub fn icon_only_button(
    ui: &mut Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
    size: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    if resp.hovered() {
        ui.painter()
            .circle_filled(rect.center(), size * 0.5, theme::SURFACE4);
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let icon_col = if resp.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    icon(ui.painter(), rect.shrink(size * 0.22), icon_col);
    resp
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

pub fn pill_toggle(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let id = ui.id().with(label);
    let t = ui.ctx().animate_bool_responsive(id, active);

    let bg = theme::white_wash(8).lerp_to_gamma(theme::pill_accent_fill(40), t);
    let text_col = theme::TEXT_DIM.lerp_to_gamma(theme::PILL_ACCENT, t);
    let stroke_col = theme::BORDER.lerp_to_gamma(theme::pill_accent_fill(180), t);

    let btn = egui::Button::new(RichText::new(label).size(12.0).color(text_col))
        .fill(bg)
        .stroke(Stroke::new(1.0, stroke_col))
        .corner_radius(theme::RADIUS_PILL);
    ui.add(btn)
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
    let height = ui.spacing().interact_size.y.max(16.0) * 0.82;
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
        let radius = 0.5 * rect.height();
        let mut off_fill = theme::SURFACE4;
        if response.hovered() {
            off_fill = off_fill.lerp_to_gamma(Color32::WHITE, 0.06);
        }
        let track = off_fill.lerp_to_gamma(theme::PILL_ACCENT, t);
        ui.painter()
            .rect(rect, radius, track, Stroke::NONE, egui::StrokeKind::Inside);
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(
            egui::pos2(knob_x, rect.center().y),
            radius - 2.5,
            Color32::WHITE,
        );
    }
    response
}

pub fn sliding_tab_bar(ui: &mut Ui, labels: &[&str], active: usize) -> Option<usize> {
    let font = egui::FontId::proportional(13.0);
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
    let pill_rect = egui::Rect::from_min_size(
        egui::pos2(anim_x, row_rect.min.y),
        egui::vec2(anim_w, height),
    );

    ui.painter().rect(
        pill_rect,
        theme::RADIUS_PILL,
        theme::accent_fill(40),
        Stroke::new(1.0, theme::accent_fill(170)),
        egui::StrokeKind::Inside,
    );
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
            .size(11.0)
            .color(theme::TEXT_DIM)
            .strong(),
    );
}

pub fn vsep(ui: &mut Ui) {
    ui.add_space(4.0);
    ui.add(egui::Separator::default().vertical().spacing(8.0));
    ui.add_space(4.0);
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
