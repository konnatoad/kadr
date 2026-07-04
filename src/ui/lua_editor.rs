use egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, RichText, Ui,
};

use crate::ui::widgets::{self, theme};

pub struct LuaEditor {
    pub open: bool,
    pub code: String,
    pub error: Option<String>,
    show_vars: bool,
    /// True while the "this will overwrite your script" prompt is shown in
    /// place of a silent, irreversible "Load Example" click.
    confirm_load_example: bool,
}

pub enum LuaEditorAction {
    None,
    Saved(String),
    Closed,
}

pub const EXAMPLE_SCRIPT: &str = r#"-- Ken Burns: gentle zoom + pan per slide.
-- The crossfade between images is handled automatically by the engine;
-- do NOT set opacity here — it would fight the built-in blend.

function on_interval(ctx)
    local t = ctx.elapsed_secs / ctx.interval_secs
    t = math.max(0, math.min(1, t))
    local e = t * t * (3 - 2 * t)  -- smoothstep easing

    -- zoom from fit size to 120%
    local zoom = 1.0 + e * 0.2

    -- alternate pan direction between odd and even images
    local dir = (ctx.current_index % 2 == 0) and 1 or -1
    local pan_x = dir * 0.03 * e

    return { zoom_target = zoom, pan_x = pan_x }
end
"#;

impl Default for LuaEditor {
    fn default() -> Self {
        Self {
            open: false,
            code: String::new(),
            error: None,
            show_vars: false,
            confirm_load_example: false,
        }
    }
}

impl LuaEditor {
    pub fn open_with(&mut self, code: &str) {
        self.code = code.to_owned();
        self.error = None;
        self.open  = true;
        self.confirm_load_example = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> LuaEditorAction {
        if !self.open {
            return LuaEditorAction::None;
        }

        let mut action = LuaEditorAction::None;
        let mut still_open = self.open;

        egui::Window::new("Lua Script Editor")
            .open(&mut still_open)
            .resizable(true)
            .collapsible(false)
            .min_size([480.0, 160.0])
            .default_size([640.0, 500.0])
            .show(ctx, |ui| {
                // ── Header bar ──────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Slideshow Lua Script")
                            .color(theme::TEXT)
                            .size(13.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::accent_button_sized(ui, "Load Example", 12.0, 28, 120).clicked() {
                            if self.code.trim().is_empty() {
                                self.code = EXAMPLE_SCRIPT.to_owned();
                                self.error = None;
                            } else {
                                self.confirm_load_example = true;
                            }
                        }
                        ui.add_space(6.0);
                        let vars_label = if self.show_vars { "▲ Variables" } else { "▼ Variables" };
                        if ui.add(
                            egui::Button::new(RichText::new(vars_label).size(12.0).color(theme::TEXT))
                                .fill(theme::white_wash(12))
                                .stroke(egui::Stroke::new(1.0, theme::BORDER)),
                        ).clicked() {
                            self.show_vars = !self.show_vars;
                        }
                    });
                });

                // ── Overwrite-confirmation banner ────────────────────────
                if self.confirm_load_example {
                    ui.add_space(6.0);
                    egui::Frame::default()
                        .fill(theme::warning_fill(20))
                        .corner_radius(theme::RADIUS_SM)
                        .inner_margin(egui::Margin::symmetric(8i8, 6i8))
                        .show(ui, |ui| {
                            ui.colored_label(
                                theme::WARNING,
                                "Replace the current script with the example? This overwrites your code.",
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if widgets::accent_button_sized(ui, "Load anyway", 12.0, 34, 140).clicked() {
                                    self.code = EXAMPLE_SCRIPT.to_owned();
                                    self.error = None;
                                    self.confirm_load_example = false;
                                }
                                ui.add_space(4.0);
                                if ui.button("Cancel").clicked() {
                                    self.confirm_load_example = false;
                                }
                            });
                        });
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // ── Variable reference panel ─────────────────────────────
                if self.show_vars {
                    egui::Frame::new()
                        .fill(theme::SURFACE2)
                        .corner_radius(theme::RADIUS_SM)
                        .stroke(egui::Stroke::new(1.0, theme::BORDER))
                        .inner_margin(egui::Margin::symmetric(10i8, 8i8))
                        .show(ui, |ui| {
                            ui.columns(2, |cols| {
                                // Left column: ctx.* inputs
                                let ui = &mut cols[0];
                                ui.label(RichText::new("ctx.*  (read-only inputs)").size(11.0).color(theme::TEXT_MUTED));
                                ui.add_space(4.0);
                                for (i, &(name, typ, desc)) in CTX_FIELDS.iter().enumerate() {
                                    var_row(ui, i, name, typ, desc);
                                }

                                // Right column: return value fields
                                let ui = &mut cols[1];
                                ui.label(RichText::new("return { … }  (output fields)").size(11.0).color(theme::TEXT_MUTED));
                                ui.add_space(4.0);
                                for (i, &(name, typ, desc)) in RETURN_FIELDS.iter().enumerate() {
                                    var_row(ui, i, name, typ, desc);
                                }
                            });
                        });
                    ui.add_space(6.0);
                }

                // ── Error banner ────────────────────────────────────────
                if let Some(err) = &self.error {
                    widgets::error_banner(ui, err);
                    ui.add_space(4.0);
                }

                // ── Code editor ─────────────────────────────────────────
                // Reserve space for footer; clamp to at least 60px so the
                // editor stays usable even when the window is dragged very small.
                let available_h = (ui.available_height() - 44.0).max(60.0);
                let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                    let mut job = lua_highlight(buf.as_str());
                    job.wrap.max_width = wrap_width;
                    ui.ctx().fonts_mut(|f| f.layout_job(job))
                };

                egui::ScrollArea::vertical()
                    .id_salt("lua_ed_scroll")
                    .min_scrolled_height(available_h)
                    .max_height(available_h)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.code)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(4)
                                .layouter(&mut layouter),
                        );
                    });

                // ── Footer ───────────────────────────────────────────────
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if widgets::accent_button(ui, "Save").clicked() {
                        action = LuaEditorAction::Saved(self.code.clone());
                    }
                    if ui.button("Cancel").clicked() {
                        action = LuaEditorAction::Closed;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("on_advance(ctx)  ·  on_interval(ctx)  — click ▼ Variables for field list")
                                .color(theme::TEXT_MUTED)
                                .size(11.0),
                        );
                    });
                });
            });

        if !still_open {
            action = LuaEditorAction::Closed;
        }

        if matches!(action, LuaEditorAction::Closed | LuaEditorAction::Saved(_)) {
            self.open = false;
        }

        action
    }
}

// ── Variable reference data ──────────────────────────────────────────────────

static CTX_FIELDS: &[(&str, &str, &str)] = &[
    ("elapsed_secs",  "number",  "seconds elapsed since slide started"),
    ("interval_secs", "number",  "total display time for this slide"),
    ("current_index", "integer", "0-based index of the current image"),
    ("total",         "integer", "total number of images in the folder"),
];

static RETURN_FIELDS: &[(&str, &str, &str)] = &[
    ("zoom_target",  "number",  "zoom multiplier — 1.0 = fit, 1.4 = 40 % larger"),
    ("pan_x",        "number",  "horizontal pan, fraction of viewport width"),
    ("pan_y",        "number",  "vertical pan, fraction of viewport height"),
    ("opacity",      "number",  "image opacity — 0.0 transparent, 1.0 opaque"),
    ("next_index",   "integer", "jump to this image index on next advance"),
    ("new_interval", "number",  "change the slide interval (seconds)"),
];

fn var_row(ui: &mut egui::Ui, index: usize, name: &str, typ: &str, desc: &str) {
    // Alternating row shading makes the two-column reference easier to scan.
    let bg = if index % 2 == 1 {
        theme::white_wash(6)
    } else {
        Color32::TRANSPARENT
    };
    egui::Frame::default()
        .fill(bg)
        .inner_margin(egui::Margin::symmetric(4i8, 2i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).monospace().size(12.0).color(theme::ACCENT_TEXT));
                ui.label(RichText::new(typ).size(11.0).color(theme::TEXT_MUTED));
                ui.label(RichText::new(desc).size(11.0).color(theme::TEXT_DIM));
            });
        });
}

// ── Syntax highlighting ──────────────────────────────────────────────────────

fn lua_highlight(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();

    // Matches Tokyo Night's own editor syntax-highlighting convention.
    let kw  = theme::ACCENT2;      // keywords — purple
    let bi  = theme::ACCENT;       // builtins — blue
    let str_c = theme::SUCCESS;    // strings — green
    let num = theme::ORANGE;       // numbers — orange
    let cmt = theme::TEXT_MUTED;   // comments
    let def = theme::TEXT;         // default

    let font = FontId::monospace(13.0);

    let src = text.as_bytes();
    let len = src.len();
    let mut i = 0usize;

    while i < len {
        // Comment: --
        if src.get(i..i + 2) == Some(b"--") {
            let end = text[i..].find('\n').map(|n| i + n + 1).unwrap_or(len);
            append(&mut job, &text[i..end], cmt, &font);
            i = end;
            continue;
        }

        // String literal
        if src[i] == b'"' || src[i] == b'\'' {
            let quote = src[i];
            let start = i;
            i += 1;
            while i < len {
                if src[i] == b'\\' { i += 2; continue; }
                if src[i] == quote { i += 1; break; }
                i += 1;
            }
            append(&mut job, &text[start..i], str_c, &font);
            continue;
        }

        // Long string [[ ... ]]
        if src.get(i..i + 2) == Some(b"[[") {
            let start = i;
            i += 2;
            while i + 1 < len {
                if src.get(i..i + 2) == Some(b"]]") { i += 2; break; }
                i += 1;
            }
            append(&mut job, &text[start..i], str_c, &font);
            continue;
        }

        // Number
        if src[i].is_ascii_digit()
            || (src[i] == b'.' && src.get(i + 1).map(|c| c.is_ascii_digit()).unwrap_or(false))
        {
            let start = i;
            while i < len && (src[i].is_ascii_alphanumeric() || src[i] == b'.' || src[i] == b'x') {
                i += 1;
            }
            append(&mut job, &text[start..i], num, &font);
            continue;
        }

        // Identifier / keyword / builtin
        if src[i].is_ascii_alphabetic() || src[i] == b'_' {
            let start = i;
            while i < len && (src[i].is_ascii_alphanumeric() || src[i] == b'_') {
                i += 1;
            }
            let word = &text[start..i];
            let color = if is_keyword(word) { kw } else if is_builtin(word) { bi } else { def };
            append(&mut job, word, color, &font);
            continue;
        }

        // Anything else — advance one char
        let ch_len = text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        append(&mut job, &text[i..i + ch_len], def, &font);
        i += ch_len;
    }

    job
}

fn append(job: &mut LayoutJob, text: &str, color: Color32, font: &FontId) {
    job.append(text, 0.0, TextFormat { font_id: font.clone(), color, ..Default::default() });
}

fn is_keyword(w: &str) -> bool {
    matches!(w,
        "and" | "break" | "do" | "else" | "elseif" | "end" | "false"
        | "for" | "function" | "goto" | "if" | "in" | "local" | "nil"
        | "not" | "or" | "repeat" | "return" | "then" | "true" | "until" | "while"
    )
}

fn is_builtin(w: &str) -> bool {
    matches!(w,
        "print" | "pairs" | "ipairs" | "type" | "tostring" | "tonumber"
        | "math" | "string" | "table" | "io" | "os" | "require"
        | "pcall" | "xpcall" | "error" | "assert" | "select" | "next"
        | "rawget" | "rawset" | "setmetatable" | "getmetatable" | "unpack"
        | "coroutine" | "collectgarbage" | "dofile" | "load" | "loadfile"
    )
}
