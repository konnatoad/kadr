use egui::{Color32, Rect, Sense, Stroke, Ui, pos2, vec2};

// ── Result type ───────────────────────────────────────────────────────────────

/// Actions the controls bar emits back to the caller.
pub enum ControlsAction {
    None,
    PlayPause,
    SeekTo(f64),   // absolute seconds
    SetVolume(f64), // 0.0–1.0
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Draw the video controls bar at the bottom of `rect`.
///
/// Returns an action if the user interacted.
pub fn show_video_controls(
    ui: &mut Ui,
    available: Rect,
    paused: bool,
    position: f64,
    duration: f64,
    volume: f64,
) -> ControlsAction {
    let bar_h = 52.0_f32;
    let bar_rect = Rect::from_min_max(
        pos2(available.min.x, available.max.y - bar_h),
        available.max,
    );

    // Semi-transparent background
    ui.painter().rect_filled(
        bar_rect,
        0.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 200),
    );

    let pad = 10.0_f32;
    let inner = bar_rect.shrink2(vec2(pad, 0.0));

    // Layout: [play] [seek bar] [time] [vol icon] [vol bar]
    let play_w = 28.0_f32;
    let time_w = 88.0_f32;
    let vol_icon_w = 18.0_f32;
    let vol_bar_w = 60.0_f32;
    let gap = 8.0_f32;

    let seek_w = inner.width()
        - play_w - gap
        - time_w - gap
        - vol_icon_w - gap / 2.0
        - vol_bar_w - gap;

    let cy = bar_rect.center().y;

    // ── Play / pause ──────────────────────────────────────────────────────────
    let play_rect = Rect::from_center_size(
        pos2(inner.min.x + play_w / 2.0, cy),
        vec2(play_w, play_w),
    );
    let play_resp = ui.allocate_rect(play_rect, Sense::click());
    draw_play_pause(ui, play_rect, paused);
    if play_resp.clicked() {
        return ControlsAction::PlayPause;
    }

    // ── Seek bar ──────────────────────────────────────────────────────────────
    let seek_x = inner.min.x + play_w + gap;
    let seek_rect = Rect::from_min_size(pos2(seek_x, cy - 3.0), vec2(seek_w, 6.0));
    if let Some(action) = draw_seek_bar(ui, seek_rect, position, duration) {
        return action;
    }

    // ── Time label ────────────────────────────────────────────────────────────
    let time_x = seek_x + seek_w + gap;
    let time_str = format!("{} / {}", fmt_time(position), fmt_time(duration));
    ui.painter().text(
        pos2(time_x, cy),
        egui::Align2::LEFT_CENTER,
        &time_str,
        egui::FontId::monospace(11.0),
        Color32::from_gray(160),
    );

    // ── Volume ────────────────────────────────────────────────────────────────
    let vol_x = time_x + time_w + gap;
    let vol_icon_rect = Rect::from_center_size(pos2(vol_x + vol_icon_w / 2.0, cy), vec2(vol_icon_w, vol_icon_w));
    draw_volume_icon(ui, vol_icon_rect, volume);

    let vol_bar_rect = Rect::from_min_size(
        pos2(vol_x + vol_icon_w + gap / 2.0, cy - 3.0),
        vec2(vol_bar_w, 6.0),
    );
    if let Some(new_vol) = draw_vol_bar(ui, vol_bar_rect, volume) {
        return ControlsAction::SetVolume(new_vol);
    }

    ControlsAction::None
}

// ── Widget drawers ────────────────────────────────────────────────────────────

fn draw_play_pause(ui: &mut Ui, rect: Rect, paused: bool) {
    let c = rect.center();
    let p = ui.painter();
    if paused {
        // Triangle (play)
        let h = rect.height() * 0.40;
        p.add(egui::Shape::convex_polygon(
            vec![
                pos2(c.x - h * 0.5, c.y - h),
                pos2(c.x - h * 0.5, c.y + h),
                pos2(c.x + h, c.y),
            ],
            Color32::from_gray(220),
            Stroke::NONE,
        ));
    } else {
        // Two bars (pause)
        let bw = rect.width() * 0.18;
        let bh = rect.height() * 0.50;
        let gap = bw * 0.8;
        for dx in [-gap - bw / 2.0, gap + bw / 2.0 - bw] {
            p.rect_filled(
                Rect::from_center_size(pos2(c.x + dx + bw / 2.0, c.y), vec2(bw, bh)),
                1.0,
                Color32::from_gray(220),
            );
        }
    }
}

fn draw_seek_bar(ui: &mut Ui, track: Rect, position: f64, duration: f64) -> Option<ControlsAction> {
    // Hit area is taller than the visual bar for easier clicking
    let hit = track.expand2(vec2(0.0, 6.0));
    let resp = ui.allocate_rect(hit, Sense::click_and_drag());

    let bg_color = Color32::from_gray(55);
    let fill_color = Color32::from_gray(200);
    let handle_color = Color32::WHITE;

    let p = ui.painter();
    p.rect_filled(track, 3.0, bg_color);

    let frac = if duration > 0.0 { (position / duration).clamp(0.0, 1.0) as f32 } else { 0.0 };
    if frac > 0.0 {
        let filled = Rect::from_min_max(track.min, pos2(track.min.x + track.width() * frac, track.max.y));
        p.rect_filled(filled, 3.0, fill_color);
    }

    // Handle dot
    let handle_x = track.min.x + track.width() * frac;
    let handle_center = pos2(handle_x, track.center().y);
    if resp.hovered() || resp.dragged() {
        p.circle_filled(handle_center, 7.0, handle_color);
    }

    if resp.clicked() || resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let t = ((pos.x - track.min.x) / track.width()).clamp(0.0, 1.0) as f64;
            return Some(ControlsAction::SeekTo(t * duration));
        }
    }

    None
}

fn draw_vol_bar(ui: &mut Ui, track: Rect, volume: f64) -> Option<f64> {
    let hit = track.expand2(vec2(0.0, 6.0));
    let resp = ui.allocate_rect(hit, Sense::click_and_drag());

    let p = ui.painter();
    p.rect_filled(track, 3.0, Color32::from_gray(55));

    let frac = volume.clamp(0.0, 1.0) as f32;
    if frac > 0.0 {
        let filled = Rect::from_min_max(track.min, pos2(track.min.x + track.width() * frac, track.max.y));
        p.rect_filled(filled, 3.0, Color32::from_gray(160));
    }

    if resp.clicked() || resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let t = ((pos.x - track.min.x) / track.width()).clamp(0.0, 1.0);
            return Some(t as f64);
        }
    }

    None
}

fn draw_volume_icon(ui: &mut Ui, rect: Rect, volume: f64) {
    let c = rect.center();
    let p = ui.painter();
    let col = Color32::from_gray(140);
    let s = rect.height() * 0.35;

    // Speaker cone (simple trapezoid)
    p.add(egui::Shape::convex_polygon(
        vec![
            pos2(c.x - s * 1.1, c.y - s * 0.45),
            pos2(c.x - s * 1.1, c.y + s * 0.45),
            pos2(c.x - s * 0.3, c.y + s * 0.8),
            pos2(c.x - s * 0.3, c.y - s * 0.8),
        ],
        col,
        Stroke::NONE,
    ));

    // Sound waves based on volume
    if volume > 0.05 {
        p.circle_stroke(pos2(c.x - s * 0.1, c.y), s * 0.9, Stroke::new(1.2, col));
    }
    if volume > 0.5 {
        p.circle_stroke(pos2(c.x - s * 0.1, c.y), s * 1.5, Stroke::new(1.2, col));
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fmt_time(secs: f64) -> String {
    if secs < 0.0 || secs.is_nan() {
        return "0:00".to_string();
    }
    let s = secs as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let s = s % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}
