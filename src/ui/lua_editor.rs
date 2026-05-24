use egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, RichText, Stroke, Ui,
};

pub struct LuaEditor {
    pub open: bool,
    pub code: String,
    pub error: Option<String>,
    show_vars: bool,
}

pub enum LuaEditorAction {
    None,
    Saved(String),
    Closed,
}

pub const EXAMPLE_SCRIPT: &str = r#"-- Ken Burns effect: slow zoom + pan + fade transition

function on_advance(ctx)
    -- reset opacity to 0 so each image fades in
    return { opacity = 0.0 }
end

function on_interval(ctx)
    local t = ctx.elapsed_secs / ctx.interval_secs
    t = math.max(0, math.min(1, t))
    local e = t * t * (3 - 2 * t)  -- smoothstep

    -- zoom from fit to 130%
    local zoom = 1.0 + e * 0.3

    -- gentle pan (alternates direction each image)
    local dir = (ctx.current_index % 2 == 0) and 1 or -1
    local pan_x = dir * 0.04 * e

    -- fade in over first second, fade out over last second
    local opacity = 1.0
    if ctx.elapsed_secs < 1.0 then
        opacity = ctx.elapsed_secs
    elseif ctx.interval_secs - ctx.elapsed_secs < 1.0 then
        opacity = ctx.interval_secs - ctx.elapsed_secs
    end

    return {
        zoom_target = zoom,
        pan_x       = pan_x,
        opacity     = opacity,
    }
end
"#;

impl Default for LuaEditor {
    fn default() -> Self {
        Self { open: false, code: String::new(), error: None, show_vars: false }
    }
}

impl LuaEditor {
    pub fn open_with(&mut self, code: &str) {
        self.code = code.to_owned();
        self.error = None;
        self.open  = true;
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
            .min_size([500.0, 400.0])
            .default_size([640.0, 500.0])
            .show(ctx, |ui| {
                // ── Header bar ──────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Slideshow Lua Script")
                            .color(Color32::from_gray(180))
                            .size(13.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(
                                RichText::new("Load Example").size(12.0).color(Color32::from_rgb(145, 190, 255))
                            )
                            .fill(Color32::from_rgba_premultiplied(99, 155, 255, 28))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 120))),
                        ).clicked() {
                            self.code = EXAMPLE_SCRIPT.to_owned();
                            self.error = None;
                        }
                        ui.add_space(6.0);
                        let vars_label = if self.show_vars { "▲ Variables" } else { "▼ Variables" };
                        if ui.add(
                            egui::Button::new(RichText::new(vars_label).size(12.0).color(Color32::from_gray(200)))
                                .fill(Color32::from_rgba_premultiplied(255, 255, 255, 12))
                                .stroke(Stroke::new(1.0, Color32::from_gray(60))),
                        ).clicked() {
                            self.show_vars = !self.show_vars;
                        }
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // ── Variable reference panel ─────────────────────────────
                if self.show_vars {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_premultiplied(18, 18, 26, 255))
                        .stroke(Stroke::new(1.0, Color32::from_gray(40)))
                        .inner_margin(egui::Margin::symmetric(10i8, 8i8))
                        .show(ui, |ui| {
                            ui.columns(2, |cols| {
                                // Left column: ctx.* inputs
                                let ui = &mut cols[0];
                                ui.label(RichText::new("ctx.*  (read-only inputs)").size(11.0).color(Color32::from_gray(110)));
                                ui.add_space(4.0);
                                for (name, typ, desc) in CTX_FIELDS {
                                    var_row(ui, name, typ, desc);
                                }

                                // Right column: return value fields
                                let ui = &mut cols[1];
                                ui.label(RichText::new("return { … }  (output fields)").size(11.0).color(Color32::from_gray(110)));
                                ui.add_space(4.0);
                                for (name, typ, desc) in RETURN_FIELDS {
                                    var_row(ui, name, typ, desc);
                                }
                            });
                        });
                    ui.add_space(6.0);
                }

                // ── Error banner ────────────────────────────────────────
                if let Some(err) = &self.error {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_premultiplied(200, 60, 60, 35))
                        .inner_margin(egui::Margin::symmetric(8i8, 6i8))
                        .show(ui, |ui| {
                            ui.colored_label(Color32::from_rgb(255, 120, 110), err);
                        });
                    ui.add_space(4.0);
                }

                // ── Code editor ─────────────────────────────────────────
                let available_h = ui.available_height() - 44.0; // reserve footer
                let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                    let mut job = lua_highlight(buf.as_str());
                    job.wrap.max_width = wrap_width;
                    ui.ctx().fonts_mut(|f| f.layout_job(job))
                };

                egui::ScrollArea::vertical()
                    .id_salt("lua_ed_scroll")
                    .max_height(available_h)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.code)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(24)
                                .layouter(&mut layouter),
                        );
                    });

                // ── Footer ───────────────────────────────────────────────
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(
                        RichText::new("Save").color(Color32::from_rgb(145, 190, 255)),
                    )
                    .fill(Color32::from_rgba_premultiplied(99, 155, 255, 38))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 160)));
                    if ui.add(save_btn).clicked() {
                        action = LuaEditorAction::Saved(self.code.clone());
                    }
                    if ui.button("Cancel").clicked() {
                        action = LuaEditorAction::Closed;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("on_advance(ctx)  ·  on_interval(ctx)  — click ▼ Variables for field list")
                                .color(Color32::from_gray(85))
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

fn var_row(ui: &mut egui::Ui, name: &str, typ: &str, desc: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).monospace().size(12.0).color(Color32::from_rgb(130, 185, 255)));
        ui.label(RichText::new(typ).size(11.0).color(Color32::from_gray(80)));
        ui.label(RichText::new(desc).size(11.0).color(Color32::from_gray(130)));
    });
}

// ── Syntax highlighting ──────────────────────────────────────────────────────

fn lua_highlight(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();

    let kw  = Color32::from_rgb(130, 175, 255); // keywords
    let bi  = Color32::from_rgb(95,  200, 195); // builtins
    let str_c = Color32::from_rgb(195, 160,  90); // strings
    let num = Color32::from_rgb(220, 150,  80); // numbers
    let cmt = Color32::from_rgb(100, 140, 100); // comments
    let def = Color32::from_rgb(210, 210, 215); // default

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
