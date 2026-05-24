use egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, RichText, Stroke, Ui,
};

pub struct LuaEditor {
    pub open: bool,
    pub code: String,
    pub error: Option<String>,
}

pub enum LuaEditorAction {
    None,
    Saved(String),
    Closed,
}

pub const EXAMPLE_SCRIPT: &str = r#"-- Example: steady zoom-in during each slide.
-- zoom_target is relative to fit-to-window:
--   1.0 = normal fit,  1.4 = 40 % larger than fit.

function on_advance(ctx)
    -- called when the slideshow advances to the next image
    return {}
end

function on_interval(ctx)
    -- called ~10 times per second while a slide is shown
    -- ctx.elapsed_secs  : seconds since this slide started
    -- ctx.interval_secs : total display time for this slide
    -- ctx.current_index : 0-based index of current image
    -- ctx.total         : total number of images

    local t = ctx.elapsed_secs / ctx.interval_secs
    local zoom = 1.0 + t * 0.4   -- zoom from 1.0x to 1.4x
    return { zoom_target = zoom }
end
"#;

impl Default for LuaEditor {
    fn default() -> Self {
        Self { open: false, code: String::new(), error: None }
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
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

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
                            RichText::new("Callbacks: on_advance(ctx)  on_interval(ctx)")
                                .color(Color32::from_gray(100))
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
