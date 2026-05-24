use egui::{Color32, CursorIcon, ScrollArea, Sense, TextureHandle, Ui, Vec2};

pub struct ThumbnailStrip {
    pub height: f32,
    pub thumb_size: f32,
}

impl Default for ThumbnailStrip {
    fn default() -> Self {
        Self { height: 90.0, thumb_size: 80.0 }
    }
}

pub struct ThumbEntry<'a> {
    pub texture: Option<&'a TextureHandle>,
    pub label: &'a str,
    pub is_video: bool,
}

pub struct StripResponse {
    pub clicked_index: Option<usize>,
    pub drag_started_index: Option<usize>,
}

impl ThumbnailStrip {
    pub fn show(
        &self,
        ui: &mut Ui,
        entries: &[ThumbEntry<'_>],
        current: usize,
    ) -> StripResponse {
        let mut clicked_index = None;
        let mut drag_started_index = None;

        let thumb_size = Vec2::splat(self.thumb_size);
        let padding = 4.0;
        let slot_size = thumb_size + Vec2::splat(6.0);

        let available_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            Vec2::new(ui.available_width(), self.height),
        );
        // Strip background — slightly darker than panel
        ui.painter().rect_filled(
            available_rect,
            0.0,
            Color32::from_rgb(12, 12, 15),
        );
        // Thin accent top border
        ui.painter().line_segment(
            [available_rect.left_top(), available_rect.right_top()],
            egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 45)),
        );

        ui.allocate_ui(
            Vec2::new(ui.available_width(), self.height),
            |ui| {
                ScrollArea::horizontal()
                    .id_salt("thumb_strip")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(padding);
                            for (i, entry) in entries.iter().enumerate() {
                                let is_current = i == current;

                                let (rect, resp) = ui.allocate_exact_size(
                                    slot_size,
                                    Sense::click_and_drag(),
                                );

                                if !ui.is_rect_visible(rect) {
                                    ui.add_space(padding);
                                    continue;
                                }

                                // Slot background
                                let slot_bg = if resp.hovered() {
                                    Color32::from_rgb(32, 32, 42)
                                } else {
                                    Color32::from_rgb(22, 22, 28)
                                };
                                ui.painter().rect_filled(rect, 6.0, slot_bg);

                                // Selected: accent fill + border
                                if is_current {
                                    ui.painter().rect_filled(
                                        rect,
                                        6.0,
                                        Color32::from_rgba_premultiplied(99, 155, 255, 25),
                                    );
                                    ui.painter().rect_stroke(
                                        rect,
                                        6.0,
                                        egui::Stroke::new(2.0, Color32::from_rgb(99, 155, 255)),
                                        egui::StrokeKind::Inside,
                                    );
                                }

                                let inner = rect.shrink(4.0);
                                if let Some(tex) = entry.texture {
                                    // Letterbox: maintain aspect ratio
                                    let [tw, th] = tex.size();
                                    let tw = tw as f32;
                                    let th = th as f32;
                                    let scale = (inner.width() / tw).min(inner.height() / th);
                                    let dw = tw * scale;
                                    let dh = th * scale;
                                    let draw_rect = egui::Rect::from_center_size(
                                        inner.center(),
                                        egui::vec2(dw, dh),
                                    );
                                    ui.painter().image(
                                        tex.id(),
                                        draw_rect,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );
                                } else {
                                    // Loading / placeholder
                                    let label = if entry.is_video { "▶" } else { "·" };
                                    let label_color = if entry.is_video {
                                        Color32::from_rgb(99, 155, 255)
                                    } else {
                                        Color32::from_gray(60)
                                    };
                                    ui.painter().text(
                                        inner.center(),
                                        egui::Align2::CENTER_CENTER,
                                        label,
                                        egui::FontId::proportional(if entry.is_video { 22.0 } else { 28.0 }),
                                        label_color,
                                    );
                                }

                                let resp = resp.on_hover_text(entry.label);

                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                }
                                if resp.clicked() {
                                    clicked_index = Some(i);
                                }
                                if resp.drag_started_by(egui::PointerButton::Primary) {
                                    drag_started_index = Some(i);
                                }

                                ui.add_space(padding);
                            }
                        });
                    });
            },
        );

        StripResponse { clicked_index, drag_started_index }
    }
}
