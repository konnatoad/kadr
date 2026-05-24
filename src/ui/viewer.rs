use egui::{pos2, Color32, Pos2, Rect, Sense, TextureHandle, Ui, Vec2};

pub struct ViewerState {
    pub zoom: f32,
    pub offset: Vec2,
    pub fit_mode: bool,
    drag_start: Option<(Pos2, Vec2)>,
    pub lua_pan: Vec2,
    pub lua_opacity: f32,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: Vec2::ZERO,
            fit_mode: true,
            drag_start: None,
            lua_pan: Vec2::ZERO,
            lua_opacity: 1.0,
        }
    }
}

impl ViewerState {
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.offset = Vec2::ZERO;
        self.fit_mode = true;
        self.lua_pan = Vec2::ZERO;
        self.lua_opacity = 1.0;
    }

    pub fn toggle_zoom(&mut self, image_size: Vec2, viewport: Vec2) {
        if self.fit_mode {
            self.fit_mode = false;
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        } else {
            self.fit_to(image_size, viewport);
        }
    }

    pub fn fit_to(&mut self, image_size: Vec2, viewport: Vec2) {
        self.fit_mode = true;
        let scale_x = viewport.x / image_size.x;
        let scale_y = viewport.y / image_size.y;
        self.zoom = scale_x.min(scale_y).min(1.0);
        self.offset = Vec2::ZERO;
    }

    pub fn zoom_by(&mut self, delta: f32, anchor: Option<Vec2>, image_size: Vec2) {
        self.fit_mode = false;
        let old_zoom = self.zoom;
        self.zoom = (self.zoom * delta).clamp(0.05, 32.0);
        if let Some(anchor) = anchor {
            let scale_change = self.zoom / old_zoom;
            self.offset = self.offset * scale_change + anchor * (1.0 - scale_change);
        }
        let _ = image_size;
    }

    pub fn clamp_offset(&mut self, image_size: Vec2, viewport: Vec2) {
        let scaled = image_size * self.zoom;
        let max_offset_x = ((scaled.x - viewport.x) / 2.0).max(0.0);
        let max_offset_y = ((scaled.y - viewport.y) / 2.0).max(0.0);
        self.offset.x = self.offset.x.clamp(-max_offset_x, max_offset_x);
        self.offset.y = self.offset.y.clamp(-max_offset_y, max_offset_y);
    }

    pub fn is_overflowing(&self, image_size: Vec2, viewport: Vec2) -> bool {
        let scaled = image_size * self.zoom;
        scaled.x > viewport.x + 1.0 || scaled.y > viewport.y + 1.0
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.fit_mode = false;
        self.offset += delta;
    }

    /// Apply a Lua-driven zoom. `target` is a multiplier relative to fit-to-window:
    /// 1.0 = fit size, 1.4 = 40% larger than fit.
    pub fn apply_lua_zoom(&mut self, target: f32, image_size: Vec2, viewport: Vec2) {
        if image_size.x <= 0.0 || image_size.y <= 0.0 { return; }
        let fit_scale = (viewport.x / image_size.x).min(viewport.y / image_size.y).min(1.0);
        let new_zoom = (fit_scale * target).clamp(0.05, 32.0);
        if (self.zoom - new_zoom).abs() > 0.0005 {
            self.fit_mode = false;
            self.zoom = new_zoom;
        }
    }
}

pub fn show_viewer(
    ui: &mut Ui,
    texture: &TextureHandle,
    state: &mut ViewerState,
    image_size: Vec2,
    bg_color: Color32,
) -> ViewerResponse {
    let mut response = ViewerResponse::default();
    let available = ui.available_size();

    let (rect, interact) = ui.allocate_exact_size(available, Sense::click_and_drag());
    ui.painter().rect_filled(rect, 0.0, bg_color);

    if state.fit_mode {
        state.fit_to(image_size, available);
    }

    let scaled = image_size * state.zoom;
    let lua_pixel_offset = Vec2::new(state.lua_pan.x * available.x, state.lua_pan.y * available.y);
    let center = rect.center() + state.offset + lua_pixel_offset;
    let img_rect = Rect::from_center_size(center, scaled);

    let alpha = (state.lua_opacity.clamp(0.0, 1.0) * 255.0) as u8;
    let uv_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    ui.painter().image(texture.id(), img_rect, uv_rect, Color32::from_white_alpha(alpha));

    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if interact.hovered() && scroll_delta != 0.0 {
        let pointer_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());
        let anchor = pointer_pos - rect.center() - state.offset;
        let factor = if scroll_delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
        state.zoom_by(factor, Some(anchor), image_size);
        state.clamp_offset(image_size, available);
    }

    if interact.drag_started_by(egui::PointerButton::Primary) {
        state.drag_start = Some((
            ui.input(|i| i.pointer.press_origin()).unwrap_or(Pos2::ZERO),
            state.offset,
        ));
    }
    if interact.dragged_by(egui::PointerButton::Primary) {
        if state.is_overflowing(image_size, available) {
            state.offset += interact.drag_delta();
            state.clamp_offset(image_size, available);
        }
    }
    if interact.drag_stopped() {
        state.drag_start = None;
    }

    response.overflowing = state.is_overflowing(image_size, available);
    response
}

#[derive(Default)]
pub struct ViewerResponse {
    pub overflowing: bool,
}
