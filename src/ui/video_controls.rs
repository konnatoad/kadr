use egui::{CursorIcon, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::ui::widgets::theme;

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
/// Returns an action if the user interacted. Volume (and, if the window gets
/// very narrow, the time label too) is dropped from the layout rather than
/// letting the remaining controls overflow or overlap.
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

    ui.painter().rect_filled(bar_rect, 0.0, theme::overlay_bg());
    ui.painter().line_segment(
        [bar_rect.left_top(), bar_rect.right_top()],
        Stroke::new(1.0, theme::accent_fill(55)),
    );

    let pad = 12.0_f32;
    let inner = bar_rect.shrink2(vec2(pad, 0.0));
    let cy = bar_rect.center().y;

    let play_w = 30.0_f32;
    let time_w = 84.0_f32;
    let vol_icon_w = 20.0_f32;
    let vol_bar_w = 60.0_f32;
    let gap = 10.0_f32;

    let min_for_time = play_w + gap + 40.0 + gap + time_w;
    let min_for_volume = min_for_time + gap + vol_icon_w + gap * 0.5 + vol_bar_w;

    let show_volume = inner.width() >= min_for_volume;
    let show_time = inner.width() >= min_for_time;

    let seek_w = if show_volume {
        inner.width() - play_w - gap - time_w - gap - vol_icon_w - gap * 0.5 - vol_bar_w
    } else if show_time {
        inner.width() - play_w - gap - time_w - gap
    } else {
        inner.width() - play_w - gap
    }
    .max(20.0);

    // ── Play / pause ──────────────────────────────────────────────────────────
    let play_rect = Rect::from_center_size(
        pos2(inner.min.x + play_w / 2.0, cy),
        vec2(play_w, play_w),
    );
    let play_resp = ui
        .allocate_rect(play_rect, Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    draw_play_pause(ui, play_rect, paused, play_resp.hovered());
    if play_resp.clicked() {
        return ControlsAction::PlayPause;
    }

    // ── Seek bar ──────────────────────────────────────────────────────────────
    let seek_x = inner.min.x + play_w + gap;
    let seek_rect = Rect::from_min_size(pos2(seek_x, cy - 3.0), vec2(seek_w, 6.0));
    if let Some(action) = draw_seek_bar(ui, seek_rect, position, duration) {
        return action;
    }

    let mut x = seek_x + seek_w;

    // ── Time label ────────────────────────────────────────────────────────────
    if show_time {
        x += gap;
        let time_str = format!("{} / {}", fmt_time(position), fmt_time(duration));
        ui.painter().text(
            pos2(x, cy),
            egui::Align2::LEFT_CENTER,
            &time_str,
            egui::FontId::monospace(12.0),
            theme::TEXT_DIM,
        );
        x += time_w;
    }

    // ── Volume ────────────────────────────────────────────────────────────────
    if show_volume {
        x += gap;
        let vol_icon_rect = Rect::from_center_size(pos2(x + vol_icon_w / 2.0, cy), vec2(vol_icon_w, vol_icon_w));
        draw_volume_icon(ui, vol_icon_rect, volume);
        x += vol_icon_w + gap * 0.5;

        let vol_bar_rect = Rect::from_min_size(pos2(x, cy - 3.0), vec2(vol_bar_w, 6.0));
        if let Some(new_vol) = draw_vol_bar(ui, vol_bar_rect, volume) {
            return ControlsAction::SetVolume(new_vol);
        }
    }

    ControlsAction::None
}

// ── Widget drawers ────────────────────────────────────────────────────────────

fn draw_play_pause(ui: &mut Ui, rect: Rect, paused: bool, hovered: bool) {
    let c = rect.center();
    let p = ui.painter();

    if hovered {
        p.circle_filled(c, rect.height() * 0.52, theme::white_wash(18));
    }
    let icon_col = if hovered { theme::TEXT } else { theme::TEXT_DIM };

    if paused {
        // Triangle (play) — nudged right of center to look visually balanced
        let h = rect.height() * 0.36;
        p.add(egui::Shape::convex_polygon(
            vec![
                pos2(c.x - h * 0.55 + 1.0, c.y - h),
                pos2(c.x - h * 0.55 + 1.0, c.y + h),
                pos2(c.x + h * 0.9, c.y),
            ],
            icon_col,
            Stroke::NONE,
        ));
    } else {
        // Two rounded bars (pause)
        let bw = rect.width() * 0.16;
        let bh = rect.height() * 0.44;
        let gap = bw * 1.1;
        for dx in [-gap - bw / 2.0, gap + bw / 2.0 - bw] {
            p.rect_filled(
                Rect::from_center_size(pos2(c.x + dx + bw / 2.0, c.y), vec2(bw, bh)),
                1.5,
                icon_col,
            );
        }
    }
}

fn draw_seek_bar(ui: &mut Ui, track: Rect, position: f64, duration: f64) -> Option<ControlsAction> {
    // Hit area is taller than the visual bar for easier clicking
    let hit = track.expand2(vec2(0.0, 8.0));
    let resp = ui
        .allocate_rect(hit, Sense::click_and_drag())
        .on_hover_and_drag_cursor(CursorIcon::PointingHand);

    let bg_color = theme::SURFACE3;
    let fill_color = theme::ACCENT;

    let p = ui.painter();
    p.rect_filled(track, 3.0, bg_color);

    let frac = if duration > 0.0 { (position / duration).clamp(0.0, 1.0) as f32 } else { 0.0 };
    if frac > 0.0 {
        let filled = Rect::from_min_max(track.min, pos2(track.min.x + track.width() * frac, track.max.y));
        p.rect_filled(filled, 3.0, fill_color);
    }

    // Handle dot — always visible so the playhead position reads at a glance,
    // grows slightly on hover/drag for a clearer grab target.
    let handle_x = track.min.x + track.width() * frac;
    let handle_center = pos2(handle_x, track.center().y);
    let active = resp.hovered() || resp.dragged();
    let radius = if active { 7.0 } else { 4.5 };
    p.circle_filled(handle_center, radius, theme::TEXT);
    if active {
        p.circle_stroke(handle_center, radius, Stroke::new(1.0, theme::ACCENT));
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
    let hit = track.expand2(vec2(0.0, 8.0));
    let resp = ui
        .allocate_rect(hit, Sense::click_and_drag())
        .on_hover_and_drag_cursor(CursorIcon::PointingHand);

    let p = ui.painter();
    p.rect_filled(track, 3.0, theme::SURFACE3);

    let frac = volume.clamp(0.0, 1.0) as f32;
    if frac > 0.0 {
        let filled = Rect::from_min_max(track.min, pos2(track.min.x + track.width() * frac, track.max.y));
        p.rect_filled(filled, 3.0, theme::ACCENT);
    }

    let handle_x = track.min.x + track.width() * frac;
    let handle_center = pos2(handle_x, track.center().y);
    let active = resp.hovered() || resp.dragged();
    p.circle_filled(handle_center, if active { 6.0 } else { 4.0 }, theme::TEXT);

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
    let col = theme::TEXT_DIM;
    let s = rect.height() * 0.34;

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

    if volume <= 0.01 {
        // Muted: cross through where the sound waves would be
        let a0 = pos2(c.x + s * 0.15, c.y - s * 0.55);
        let a1 = pos2(c.x + s * 1.15, c.y + s * 0.55);
        let b0 = pos2(c.x + s * 0.15, c.y + s * 0.55);
        let b1 = pos2(c.x + s * 1.15, c.y - s * 0.55);
        p.line_segment([a0, a1], Stroke::new(1.6, col));
        p.line_segment([b0, b1], Stroke::new(1.6, col));
        return;
    }

    // Sound waves based on volume
    if volume > 0.05 {
        p.circle_stroke(pos2(c.x - s * 0.1, c.y), s * 0.9, Stroke::new(1.3, col));
    }
    if volume > 0.5 {
        p.circle_stroke(pos2(c.x - s * 0.1, c.y), s * 1.5, Stroke::new(1.3, col));
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
