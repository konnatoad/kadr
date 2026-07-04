use egui::{Color32, Key, RichText, StrokeKind, Ui};

use crate::fs::sorter::SortMode;
use crate::keybinds::{KeyAction, KeyBinding, KeyBindings, SerializableKey};
use crate::ui::widgets::{self, theme};

pub struct SettingsDialog {
    pub open: bool,
    pub tab: SettingsTab,
    pub rebinding: Option<KeyAction>,

    // General / Behavior
    pub show_thumbnails: bool,
    pub scan_subfolders: bool,
    pub filter_images: bool,
    pub filter_videos: bool,
    pub sort_mode: SortMode,
    pub remember_last_folder: bool,

    // Appearance
    pub bg_color: [f32; 3],
    pub thumb_size: f32,

    // Slideshow
    pub slideshow_interval: f64,
    pub slideshow_transition: f32,
    pub slideshow_loop: bool,
    pub slideshow_random: bool,

    // Display
    pub preferred_monitor: usize,

    // Lua — code lives here; editing happens in LuaEditor window
    pub lua_code: String,
    pub lua_error: Option<String>,

    /// Set to true when the user clicks "Edit Lua Script" — app.rs will open the editor
    pub open_lua_editor: bool,

    pub monitors: Vec<crate::monitor::MonitorInfo>,
}

#[derive(PartialEq)]
pub enum SettingsTab {
    General,
    Appearance,
    Keybinds,
    Slideshow,
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self {
            open: false,
            tab: SettingsTab::General,
            rebinding: None,
            show_thumbnails: true,
            scan_subfolders: false,
            filter_images: true,
            filter_videos: true,
            sort_mode: SortMode::Name,
            remember_last_folder: true,
            preferred_monitor: 0,
            bg_color: [0.08, 0.08, 0.08],
            thumb_size: 80.0,
            slideshow_interval: 3.0,
            slideshow_transition: 0.5,
            slideshow_loop: true,
            slideshow_random: false,
            lua_code: String::new(),
            lua_error: None,
            open_lua_editor: false,
            monitors: Vec::new(),
        }
    }
}

pub enum SettingsAction {
    None,
    Save,
    Close,
}

impl SettingsDialog {
    pub fn show(&mut self, ctx: &egui::Context, bindings: &mut KeyBindings) -> SettingsAction {
        if !self.open {
            return SettingsAction::None;
        }

        let mut action = SettingsAction::None;
        let mut open = self.open;

        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .min_size([480.0, 360.0])
            .default_size([540.0, 440.0])
            .show(ctx, |ui| {
                // ── Tab bar ──────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if widgets::tab_button(ui, "General",    self.tab == SettingsTab::General).clicked() {
                        self.tab = SettingsTab::General;
                    }
                    if widgets::tab_button(ui, "Appearance", self.tab == SettingsTab::Appearance).clicked() {
                        self.tab = SettingsTab::Appearance;
                    }
                    if widgets::tab_button(ui, "Keybinds",   self.tab == SettingsTab::Keybinds).clicked() {
                        self.tab = SettingsTab::Keybinds;
                    }
                    if widgets::tab_button(ui, "Slideshow",  self.tab == SettingsTab::Slideshow).clicked() {
                        self.tab = SettingsTab::Slideshow;
                    }
                });
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(6.0);

                // ── Content ───────────────────────────────────────────────
                let footer_h = 44.0;
                egui::ScrollArea::vertical()
                    .id_salt("settings_content")
                    .max_height(ui.available_height() - footer_h)
                    .show(ui, |ui| {
                        match self.tab {
                            SettingsTab::General    => self.show_general(ui),
                            SettingsTab::Appearance => self.show_appearance(ui),
                            SettingsTab::Keybinds   => self.show_keybinds(ui, bindings),
                            SettingsTab::Slideshow  => self.show_slideshow(ui),
                        }
                    });

                // ── Footer ───────────────────────────────────────────────
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if widgets::accent_button(ui, "Save").clicked() { action = SettingsAction::Save; }
                    if ui.button("Close").clicked()                 { action = SettingsAction::Close; }
                });
            });

        if !open {
            action = SettingsAction::Close;
        }

        // Capture rebind key press, or close the whole dialog via Escape when
        // not in the middle of capturing a keybind.
        if self.rebinding.is_some() {
            ctx.input(|input| {
                // Escape always cancels — must be checked before the capture
                // loop below, since Key::Escape is itself one of the
                // candidate keys `all_known_keys()` offers to bind.
                if input.key_pressed(Key::Escape) {
                    self.rebinding = None;
                    return;
                }
                for key in all_known_keys() {
                    if input.key_pressed(key) {
                        if let Some(sk) = key_to_serializable(key) {
                            if let Some(rebind_action) = self.rebinding.take() {
                                bindings.0.insert(
                                    rebind_action,
                                    KeyBinding {
                                        key: sk,
                                        ctrl:  input.modifiers.ctrl,
                                        shift: input.modifiers.shift,
                                        alt:   input.modifiers.alt,
                                    },
                                );
                            }
                        }
                    }
                }
            });
        } else if ctx.input(|input| input.key_pressed(Key::Escape)) {
            action = SettingsAction::Close;
        }

        action
    }

    fn show_general(&mut self, ui: &mut Ui) {
        // Populate monitor list lazily
        if self.monitors.is_empty() {
            self.monitors = crate::monitor::enumerate();
        }

        widgets::section_label(ui, "Display");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Open on:");
            ui.add_space(8.0);
            let current_label = if self.preferred_monitor == 0 {
                "Auto (OS default)".to_owned()
            } else {
                self.monitors
                    .get(self.preferred_monitor - 1)
                    .map(|m| m.label())
                    .unwrap_or_else(|| format!("Monitor {}", self.preferred_monitor))
            };
            egui::ComboBox::from_id_salt("settings_monitor")
                .selected_text(current_label)
                .width(ui.available_width().min(340.0))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.preferred_monitor == 0, "Auto (OS default)").clicked() {
                        self.preferred_monitor = 0;
                    }
                    for m in &self.monitors {
                        let idx = m.index + 1;
                        if ui.selectable_label(self.preferred_monitor == idx, m.label()).clicked() {
                            self.preferred_monitor = idx;
                        }
                    }
                });
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new("Takes effect on next launch.")
                .color(theme::TEXT_MUTED)
                .size(11.0),
        );

        ui.add_space(12.0);
        widgets::section_label(ui, "View");
        ui.add_space(4.0);
        ui.checkbox(&mut self.show_thumbnails, "Show thumbnail strip");
        ui.add_space(2.0);
        ui.checkbox(&mut self.scan_subfolders, "Scan subfolders");

        ui.add_space(12.0);
        widgets::section_label(ui, "Filter");
        ui.add_space(4.0);
        ui.checkbox(&mut self.filter_images, "Show images");
        ui.add_space(2.0);
        ui.checkbox(&mut self.filter_videos, "Show videos");

        ui.add_space(12.0);
        widgets::section_label(ui, "Sort");
        ui.add_space(6.0);
        egui::ComboBox::from_id_salt("settings_sort")
            .selected_text(self.sort_mode.label())
            .width(ui.available_width().min(280.0))
            .show_ui(ui, |ui| {
                for mode in SortMode::all() {
                    if ui.selectable_label(&self.sort_mode == mode, mode.label()).clicked() {
                        self.sort_mode = mode.clone();
                    }
                }
            });

        ui.add_space(12.0);
        widgets::section_label(ui, "Session");
        ui.add_space(4.0);
        ui.checkbox(&mut self.remember_last_folder,
            "Reopen last folder on startup");
        ui.add_space(2.0);
        ui.label(
            RichText::new("When off, kadr starts with an empty window every time.")
                .color(theme::TEXT_MUTED)
                .size(11.5),
        );
    }

    fn show_appearance(&mut self, ui: &mut Ui) {
        widgets::section_label(ui, "Viewer");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Background color:");
            ui.add_space(8.0);
            ui.color_edit_button_rgb(&mut self.bg_color);
            ui.add_space(8.0);

            // Larger live preview swatch — the color button itself is small,
            // this makes it obvious at a glance what the viewer background
            // will look like.
            let preview = Color32::from_rgb(
                (self.bg_color[0] * 255.0).round() as u8,
                (self.bg_color[1] * 255.0).round() as u8,
                (self.bg_color[2] * 255.0).round() as u8,
            );
            let (rect, _) = ui.allocate_exact_size(egui::vec2(56.0, 22.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, theme::RADIUS_SM, preview);
            ui.painter().rect_stroke(
                rect,
                theme::RADIUS_SM,
                egui::Stroke::new(1.0, theme::BORDER),
                StrokeKind::Outside,
            );
        });

        ui.add_space(12.0);
        widgets::section_label(ui, "Thumbnails");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add_space(8.0);
            ui.add(egui::Slider::new(&mut self.thumb_size, 40.0..=160.0).suffix(" px"));
        });
    }

    fn show_keybinds(&mut self, ui: &mut Ui, bindings: &mut KeyBindings) {
        if self.rebinding.is_some() {
            ui.colored_label(
                theme::WARNING,
                "Press any key combination to assign  (Escape to cancel)",
            );
            ui.add_space(6.0);
        }

        for (i, (group_name, actions)) in keybind_groups().iter().enumerate() {
            if i > 0 {
                ui.add_space(10.0);
            }
            widgets::section_label(ui, group_name);
            ui.add_space(4.0);

            for action in actions {
                let binding_text = bindings
                    .get(action)
                    .map(|b| b.display())
                    .unwrap_or_else(|| "unbound".to_string());

                ui.horizontal(|ui| {
                    ui.label(RichText::new(action.label()).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let is_rebinding = self.rebinding.as_ref() == Some(action);
                        let btn_label = if is_rebinding {
                            RichText::new("listening…").color(theme::WARNING)
                        } else {
                            RichText::new(&binding_text).color(theme::TEXT_DIM)
                        };
                        if ui.button(btn_label).clicked() && !is_rebinding {
                            self.rebinding = Some(action.clone());
                        }
                    });
                });
                ui.add(egui::Separator::default().spacing(2.0));
            }
        }
    }

    fn show_slideshow(&mut self, ui: &mut Ui) {
        widgets::section_label(ui, "Playback");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Interval:");
            ui.add_space(8.0);
            ui.add(egui::Slider::new(&mut self.slideshow_interval, 0.5..=60.0).suffix(" s"));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Crossfade:");
            ui.add_space(8.0);
            ui.add(
                egui::Slider::new(&mut self.slideshow_transition, 0.0..=3.0)
                    .suffix(" s")
                    .custom_formatter(|v, _| {
                        if v < 0.05 { "off".into() } else { format!("{v:.2} s") }
                    }),
            );
        });
        ui.add_space(4.0);
        ui.checkbox(&mut self.slideshow_loop,   "Loop");
        ui.add_space(2.0);
        ui.checkbox(&mut self.slideshow_random, "Random order");

        ui.add_space(14.0);
        widgets::section_label(ui, "Lua Script");
        ui.add_space(6.0);

        // Lua error (from last compilation attempt)
        if let Some(err) = &self.lua_error {
            widgets::error_banner(ui, err);
            ui.add_space(6.0);
        }

        // Script status line
        let script_status = if self.lua_code.trim().is_empty() {
            RichText::new("No script set").color(theme::TEXT_MUTED).size(12.5)
        } else {
            let line_count = self.lua_code.lines().count();
            RichText::new(format!("{line_count} lines")).color(theme::TEXT_DIM).size(12.5)
        };
        ui.horizontal(|ui| {
            ui.label(script_status);
            ui.add_space(8.0);

            if widgets::accent_button_sized(ui, "Edit Lua Script", 12.5, 30, 130).clicked() {
                self.open_lua_editor = true;
            }

            if !self.lua_code.trim().is_empty() {
                ui.add_space(4.0);
                if ui.button(RichText::new("Clear").color(theme::TEXT_DIM).size(12.0)).clicked() {
                    self.lua_code.clear();
                    self.lua_error = None;
                }
            }
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "The script runs callbacks during slideshow playback.\n\
                 on_interval(ctx) can return { zoom_target } to animate zoom."
            )
            .color(theme::TEXT_MUTED)
            .size(11.5),
        );
    }
}

/// Groups `KeyAction::all()` under scannable headings for the Keybinds tab,
/// instead of one long flat list.
fn keybind_groups() -> Vec<(&'static str, Vec<KeyAction>)> {
    vec![
        ("Navigation", vec![
            KeyAction::NextImage,
            KeyAction::PrevImage,
            KeyAction::ToggleZoom,
            KeyAction::ZoomIn,
            KeyAction::ZoomOut,
            KeyAction::ZoomReset,
            KeyAction::PanUp,
            KeyAction::PanDown,
            KeyAction::PanLeft,
            KeyAction::PanRight,
        ]),
        ("View", vec![
            KeyAction::Fullscreen,
            KeyAction::ToggleThumbnails,
            KeyAction::RotateCW,
            KeyAction::RotateCCW,
            KeyAction::FlipHorizontal,
            KeyAction::FlipVertical,
        ]),
        ("File", vec![
            KeyAction::OpenFolder,
            KeyAction::OpenFile,
            KeyAction::CombineFolders,
            KeyAction::DeleteFile,
        ]),
        ("Other", vec![
            KeyAction::ToggleSlideshow,
            KeyAction::OpenSettings,
            KeyAction::Quit,
        ]),
    ]
}

fn all_known_keys() -> Vec<Key> {
    vec![
        Key::ArrowLeft, Key::ArrowRight, Key::ArrowUp, Key::ArrowDown,
        Key::Space, Key::Enter, Key::Escape, Key::Delete,
        Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6,
        Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12,
        Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G,
        Key::H, Key::I, Key::J, Key::K, Key::L, Key::M, Key::N,
        Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U,
        Key::V, Key::W, Key::X, Key::Y, Key::Z,
        Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4,
        Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9,
        Key::Plus, Key::Minus, Key::Comma, Key::Period,
        Key::Home, Key::End, Key::PageUp, Key::PageDown,
    ]
}

fn key_to_serializable(key: Key) -> Option<SerializableKey> {
    Some(match key {
        Key::ArrowLeft  => SerializableKey::ArrowLeft,
        Key::ArrowRight => SerializableKey::ArrowRight,
        Key::ArrowUp    => SerializableKey::ArrowUp,
        Key::ArrowDown  => SerializableKey::ArrowDown,
        Key::Space      => SerializableKey::Space,
        Key::Enter      => SerializableKey::Enter,
        Key::Escape     => SerializableKey::Escape,
        Key::Delete     => SerializableKey::Delete,
        Key::F1  => SerializableKey::F1,  Key::F2  => SerializableKey::F2,
        Key::F3  => SerializableKey::F3,  Key::F4  => SerializableKey::F4,
        Key::F5  => SerializableKey::F5,  Key::F6  => SerializableKey::F6,
        Key::F7  => SerializableKey::F7,  Key::F8  => SerializableKey::F8,
        Key::F9  => SerializableKey::F9,  Key::F10 => SerializableKey::F10,
        Key::F11 => SerializableKey::F11, Key::F12 => SerializableKey::F12,
        Key::A => SerializableKey::A, Key::B => SerializableKey::B,
        Key::C => SerializableKey::C, Key::D => SerializableKey::D,
        Key::E => SerializableKey::E, Key::F => SerializableKey::F,
        Key::G => SerializableKey::G, Key::H => SerializableKey::H,
        Key::I => SerializableKey::I, Key::J => SerializableKey::J,
        Key::K => SerializableKey::K, Key::L => SerializableKey::L,
        Key::M => SerializableKey::M, Key::N => SerializableKey::N,
        Key::O => SerializableKey::O, Key::P => SerializableKey::P,
        Key::Q => SerializableKey::Q, Key::R => SerializableKey::R,
        Key::S => SerializableKey::S, Key::T => SerializableKey::T,
        Key::U => SerializableKey::U, Key::V => SerializableKey::V,
        Key::W => SerializableKey::W, Key::X => SerializableKey::X,
        Key::Y => SerializableKey::Y, Key::Z => SerializableKey::Z,
        Key::Num0 => SerializableKey::Num0, Key::Num1 => SerializableKey::Num1,
        Key::Num2 => SerializableKey::Num2, Key::Num3 => SerializableKey::Num3,
        Key::Num4 => SerializableKey::Num4, Key::Num5 => SerializableKey::Num5,
        Key::Num6 => SerializableKey::Num6, Key::Num7 => SerializableKey::Num7,
        Key::Num8 => SerializableKey::Num8, Key::Num9 => SerializableKey::Num9,
        Key::Plus  => SerializableKey::Plus,  Key::Minus  => SerializableKey::Minus,
        Key::Comma => SerializableKey::Comma, Key::Period => SerializableKey::Period,
        Key::Home   => SerializableKey::Home,   Key::End     => SerializableKey::End,
        Key::PageUp => SerializableKey::PageUp, Key::PageDown => SerializableKey::PageDown,
        _ => return None,
    })
}
