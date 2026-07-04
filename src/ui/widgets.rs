//! Shared styling helpers used across the toolbar and every dialog window.
//!
//! These mirror the palette set up once in `apply_theme` (app.rs) so that
//! toolbar.rs, settings_dialog.rs, lua_editor.rs, combine_dialog.rs,
//! video_controls.rs and thumbnail_strip.rs all draw their custom
//! (non-default-egui-styled) buttons, cards and banners from the same
//! source instead of each redeclaring their own `Color32` literals.
//!
//! Palette: Tokyo Night (deep navy background, blue/purple accents).

use egui::{Color32, RichText, Stroke, Ui};

/// Palette constants, mirroring the values set in `apply_theme` (app.rs).
pub mod theme {
    use egui::Color32;

    // Surfaces, darkest to lightest.
    pub const BG: Color32 = Color32::from_rgb(0x1a, 0x1b, 0x26);
    pub const SURFACE: Color32 = Color32::from_rgb(0x1f, 0x23, 0x35);
    pub const SURFACE2: Color32 = Color32::from_rgb(0x24, 0x28, 0x3b);
    pub const SURFACE3: Color32 = Color32::from_rgb(0x29, 0x2e, 0x42);
    pub const SURFACE4: Color32 = Color32::from_rgb(0x41, 0x48, 0x68);
    pub const BORDER: Color32 = Color32::from_rgb(0x3b, 0x42, 0x61);

    // Blue is the primary interactive accent; purple is used sparingly as a
    // secondary accent (section headers, a couple of icons) so the palette
    // reads as "Tokyo Night" rather than "the old blue theme recolored."
    pub const ACCENT: Color32 = Color32::from_rgb(0x7a, 0xa2, 0xf7);
    pub const ACCENT_TEXT: Color32 = Color32::from_rgb(0x9a, 0xb8, 0xfb);
    pub const ACCENT2: Color32 = Color32::from_rgb(0xbb, 0x9a, 0xf7);

    pub const TEXT: Color32 = Color32::from_rgb(0xc0, 0xca, 0xf5);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x73, 0x7a, 0xa2);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x56, 0x5f, 0x89);

    pub const SUCCESS: Color32 = Color32::from_rgb(0x9e, 0xce, 0x6a);
    pub const WARNING: Color32 = Color32::from_rgb(0xe0, 0xaf, 0x68);
    pub const ERROR_TEXT: Color32 = Color32::from_rgb(0xf7, 0x76, 0x8e);

    /// Base corner radius for cards/dialogs — bigger and softer than the old 6-8px.
    pub const RADIUS: f32 = 12.0;
    /// Corner radius for smaller inline controls (buttons, thumb slots).
    pub const RADIUS_SM: f32 = 10.0;
    /// Large enough that egui's rect tessellation clamps it to a true capsule/pill.
    pub const RADIUS_PILL: f32 = 999.0;

    /// Distinct from `WARNING` — Tokyo Night's orange, used for numeric literals
    /// in the Lua syntax highlighter.
    pub const ORANGE: Color32 = Color32::from_rgb(0xff, 0x9e, 0x64);

    // NOTE: all "color at alpha" helpers below use `from_rgba_unmultiplied`,
    // NOT `from_rgba_premultiplied`. The latter takes r/g/b that are already
    // scaled down by alpha (e.g. alpha=40 needs dim, small r/g/b) — passing
    // full-brightness r/g/b with a low alpha there is invalid premultiplied
    // data and renders as an oversaturated/opaque blob, not a subtle tint.
    // `from_rgba_unmultiplied` is the one that means "this color, at this
    // opacity" the way a color picker would. (`from_rgba_unmultiplied` isn't
    // `const`, so these can't be `const fn` — negligible cost, called a
    // handful of times per frame.)

    /// The error red background used for banners — not `const` (see note above).
    pub fn error_bg() -> Color32 {
        Color32::from_rgba_unmultiplied(0xf7, 0x76, 0x8e, 28)
    }

    /// Background for transient on-image overlays (filename pill, status
    /// toast, zoom indicator) and floating cards (toolbar, thumbnail strip).
    pub fn overlay_bg() -> Color32 {
        Color32::from_rgba_unmultiplied(0x1a, 0x1b, 0x26, 210)
    }

    /// The accent blue at a given alpha — used for button/pill fills and strokes.
    pub fn accent_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0x7a, 0xa2, 0xf7, alpha)
    }

    /// The secondary accent (purple) at a given alpha.
    pub fn accent2_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0xbb, 0x9a, 0xf7, alpha)
    }

    /// The error red at a given alpha — used for destructive buttons (Remove) and banners.
    pub fn error_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0xf7, 0x76, 0x8e, alpha)
    }

    /// The warning yellow at a given alpha — used for non-destructive confirm banners.
    pub fn warning_fill(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(0xe0, 0xaf, 0x68, alpha)
    }

    /// A translucent white wash at a given alpha — used for subtle hover/inactive fills.
    pub fn white_wash(alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
    }
}

/// Frame for "floating card" surfaces — the toolbar bar, thumbnail strip and
/// viewer canvas all sit in one of these with a gutter around them, instead
/// of being flush with the window edge.
pub fn card_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS)
}

/// A button combining a small hand-drawn vector icon with a text label.
/// `icon` draws into the given square icon rect using the given color.
/// Used for the toolbar's primary actions instead of text-only buttons.
pub fn icon_button(
    ui: &mut Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
    label: &str,
) -> egui::Response {
    let icon_size = 15.0_f32;
    let gap = 7.0_f32;
    let pad = egui::vec2(10.0, 6.0);
    let font = egui::FontId::proportional(12.5);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, theme::TEXT);

    let content_w = icon_size + gap + galley.size().x;
    let desired = egui::vec2(content_w + pad.x * 2.0, icon_size.max(galley.size().y) + pad.y * 2.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    if resp.hovered() {
        ui.painter().rect_filled(rect, theme::RADIUS_SM, theme::SURFACE4);
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let icon_col = if resp.hovered() { theme::TEXT } else { theme::TEXT_DIM };
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + pad.x, rect.center().y - icon_size / 2.0),
        egui::vec2(icon_size, icon_size),
    );
    icon(ui.painter(), icon_rect, icon_col);

    let text_pos = egui::pos2(icon_rect.right() + gap, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, theme::TEXT);

    resp
}

/// An icon-only button with a circular hover backdrop — used for Settings.
pub fn icon_only_button(
    ui: &mut Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
    size: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    if resp.hovered() {
        ui.painter().circle_filled(rect.center(), size * 0.5, theme::SURFACE4);
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let icon_col = if resp.hovered() { theme::TEXT } else { theme::TEXT_DIM };
    icon(ui.painter(), rect.shrink(size * 0.22), icon_col);
    resp
}

/// A button styled with the accent (blue) fill — the "primary action" look
/// used for Save / Combine / Edit Lua Script buttons across dialogs.
pub fn accent_button(ui: &mut Ui, label: &str) -> egui::Response {
    accent_button_sized(ui, label, 13.0, 40, 170)
}

/// Same as [`accent_button`] but with caller-controlled font size and fill/stroke alpha,
/// for the slightly-more-subdued accent buttons used in compact rows.
pub fn accent_button_sized(
    ui: &mut Ui,
    label: &str,
    font_size: f32,
    fill_alpha: u8,
    stroke_alpha: u8,
) -> egui::Response {
    // Buttons here carry hardcoded colors rather than the theme's widget
    // visuals, so disabled state (e.g. inside add_enabled_ui(false, ..)) needs
    // to be dimmed explicitly or it would otherwise look identical to enabled.
    let enabled = ui.is_enabled();
    let text_col = if enabled { theme::ACCENT_TEXT } else { theme::TEXT_MUTED };
    let scale = if enabled { 1.0 } else { 0.4 };
    let btn = egui::Button::new(RichText::new(label).size(font_size).color(text_col))
        .fill(theme::accent_fill((fill_alpha as f32 * scale) as u8))
        .stroke(Stroke::new(1.0, theme::accent_fill((stroke_alpha as f32 * scale) as u8)))
        .corner_radius(theme::RADIUS_SM);
    ui.add(btn)
}

/// Plain toolbar-style action button (default egui widget styling, sized text).
pub fn action_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.button(RichText::new(label).size(12.5))
}

/// A capsule-shaped toggle button that highlights in accent color when active
/// — used for the Images/Videos/Subfolders filters.
pub fn pill_toggle(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let (bg, text_col, stroke) = if active {
        (
            theme::accent_fill(40),
            theme::ACCENT_TEXT,
            Stroke::new(1.0, theme::accent_fill(180)),
        )
    } else {
        (
            theme::white_wash(8),
            theme::TEXT_DIM,
            Stroke::new(1.0, theme::BORDER),
        )
    };
    let btn = egui::Button::new(RichText::new(label).size(12.0).color(text_col))
        .fill(bg)
        .stroke(stroke)
        .corner_radius(theme::RADIUS_PILL);
    ui.add(btn)
}

/// A capsule-shaped tab-bar button (used in Settings' top tab row): accent
/// background when active, transparent + muted text when not.
pub fn tab_button(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let (bg, text_col, stroke) = if active {
        (
            theme::accent_fill(40),
            theme::ACCENT_TEXT,
            Stroke::new(1.0, theme::accent_fill(170)),
        )
    } else {
        (Color32::TRANSPARENT, theme::TEXT_DIM, Stroke::NONE)
    };
    let btn = egui::Button::new(RichText::new(label).size(13.0).color(text_col))
        .fill(bg)
        .stroke(stroke)
        .corner_radius(theme::RADIUS_PILL);
    ui.add(btn)
}

/// Small, bold section-header label (secondary/purple accent) used to group
/// related controls.
pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(11.0)
            .color(theme::ACCENT2)
            .strong(),
    );
}

/// Thin vertical separator with breathing room on both sides.
pub fn vsep(ui: &mut Ui) {
    ui.add_space(4.0);
    ui.add(egui::Separator::default().vertical().spacing(8.0));
    ui.add_space(4.0);
}

/// Inline error banner (red-tinted background + red text) — used for Lua
/// compile errors in both the settings dialog and the Lua editor.
pub fn error_banner(ui: &mut Ui, message: &str) {
    egui::Frame::default()
        .fill(theme::error_bg())
        .corner_radius(theme::RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(10i8, 6i8))
        .show(ui, |ui| {
            ui.colored_label(theme::ERROR_TEXT, message);
        });
}
