use egui::{Color32, CursorIcon, ScrollArea, Sense, TextureHandle, Ui, Vec2};

use crate::ui::widgets::theme;

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
        // Floating card background, matching the toolbar's treatment.
        ui.painter().rect_filled(available_rect, theme::RADIUS, theme::SURFACE);
        ui.painter().rect_stroke(
            available_rect,
            theme::RADIUS,
            egui::Stroke::new(1.0, theme::BORDER),
            egui::StrokeKind::Outside,
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

                                // Slot background — hover contrast bumped up from the
                                // original 10-unit delta so it reads clearly at a glance.
                                let slot_bg = if resp.hovered() {
                                    theme::SURFACE4
                                } else {
                                    theme::SURFACE2
                                };
                                ui.painter().rect_filled(rect, theme::RADIUS_SM, slot_bg);

                                // Selected: stronger accent fill + border than before
                                // (was a near-invisible 25-alpha fill).
                                if is_current {
                                    ui.painter().rect_filled(rect, theme::RADIUS_SM, theme::accent_fill(50));
                                    ui.painter().rect_stroke(
                                        rect,
                                        theme::RADIUS_SM,
                                        egui::Stroke::new(2.0, theme::ACCENT),
                                        egui::StrokeKind::Inside,
                                    );
                                } else if resp.hovered() {
                                    ui.painter().rect_stroke(
                                        rect,
                                        theme::RADIUS_SM,
                                        egui::Stroke::new(1.0, theme::accent_fill(70)),
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
                                } else if entry.is_video {
                                    // Play glyph over a soft backdrop, matching the
                                    // vector play icon used in the video controls bar.
                                    let c = inner.center();
                                    let s = inner.height().min(inner.width()) * 0.16;
                                    ui.painter().circle_filled(
                                        c,
                                        s * 2.1,
                                        Color32::from_rgba_premultiplied(0, 0, 0, 70),
                                    );
                                    ui.painter().add(egui::Shape::convex_polygon(
                                        vec![
                                            egui::pos2(c.x - s * 0.55 + 1.0, c.y - s),
                                            egui::pos2(c.x - s * 0.55 + 1.0, c.y + s),
                                            egui::pos2(c.x + s * 0.9, c.y),
                                        ],
                                        theme::ACCENT,
                                        egui::Stroke::NONE,
                                    ));
                                } else {
                                    // Still loading — a plain ring instead of a stray "." glyph.
                                    let r = inner.height().min(inner.width()) * 0.12;
                                    ui.painter().circle_stroke(
                                        inner.center(),
                                        r,
                                        egui::Stroke::new(1.4, theme::TEXT_MUTED),
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
