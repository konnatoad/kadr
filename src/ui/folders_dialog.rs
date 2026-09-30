use std::path::PathBuf;

use egui::{Color32, RichText, Stroke};

use crate::ui::widgets::theme;

pub struct FoldersDialog {
    pub open: bool,
    pub paths: Vec<PathBuf>,
}

impl Default for FoldersDialog {
    fn default() -> Self {
        Self {
            open: false,
            paths: Vec::new(),
        }
    }
}

pub enum FoldersAction {
    None,
    AddFolders,
    Open(Vec<PathBuf>),
    Cancel,
}

impl FoldersDialog {
    pub fn show(&mut self, ctx: &egui::Context) -> FoldersAction {
        if !self.open {
            return FoldersAction::None;
        }

        let mut action = FoldersAction::None;

        egui::Window::new("folders_dialog")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .inner_margin(egui::Margin::same(22i8)),
            )
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("open")
                        .font(mono(10.5))
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(2.0);
                ui.label(RichText::new("Folders").font(mono(15.0)).color(theme::TEXT));
                ui.add_space(16.0);

                if self.paths.is_empty() {
                    ui.label(
                        RichText::new("No folders added yet.")
                            .font(mono(12.0))
                            .color(theme::TEXT_MUTED),
                    );
                } else {
                    let mut to_remove: Option<usize> = None;
                    egui::ScrollArea::vertical()
                        .id_salt("folders_list")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for (i, path) in self.paths.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(path.to_string_lossy().into_owned())
                                            .font(mono(12.0))
                                            .color(theme::TEXT_DIM),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if remove_glyph(ui) {
                                                to_remove = Some(i);
                                            }
                                        },
                                    );
                                });
                                ui.add_space(6.0);
                                ui.separator();
                                ui.add_space(6.0);
                            }
                        });
                    if let Some(i) = to_remove {
                        self.paths.remove(i);
                    }
                }

                ui.add_space(4.0);
                if plain_action(ui, "+ Add folder", theme::ACCENT_TEXT) {
                    action = FoldersAction::AddFolders;
                }

                ui.add_space(10.0);
                let meta = match self.paths.len() {
                    0 => "No folders yet".to_owned(),
                    1 => "1 folder selected".to_owned(),
                    n => format!("{n} folders selected"),
                };
                ui.label(RichText::new(meta).font(mono(11.0)).color(theme::TEXT_MUTED));

                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if open_button(ui, !self.paths.is_empty()) {
                            action = FoldersAction::Open(self.paths.clone());
                        }
                        ui.add_space(16.0);
                        if plain_action(ui, "Cancel", theme::TEXT_DIM) {
                            action = FoldersAction::Cancel;
                        }
                    });
                });
            });

        action
    }
}

fn mono(size: f32) -> egui::FontId {
    egui::FontId::monospace(size)
}

fn remove_glyph(ui: &mut egui::Ui) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
    let col = if resp.hovered() {
        theme::ERROR_TEXT
    } else {
        theme::TEXT_MUTED
    };
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter()
        .text(rect.center(), egui::Align2::CENTER_CENTER, "×", mono(13.0), col);
    resp.clicked()
}

fn plain_action(ui: &mut egui::Ui, label: &str, base: Color32) -> bool {
    let font = mono(12.5);
    let size = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE)
        .size();
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let col = if resp.hovered() { theme::TEXT } else { base };
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter()
        .text(rect.left_center(), egui::Align2::LEFT_CENTER, label, font, col);
    resp.clicked()
}

fn open_button(ui: &mut egui::Ui, enabled: bool) -> bool {
    let mut clicked = false;
    ui.add_enabled_ui(enabled, |ui| {
        let label = "Open";
        let font = mono(12.5);
        let text_size = ui
            .painter()
            .layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE)
            .size();
        let pad = egui::vec2(16.0, 8.0);
        let (rect, resp) = ui.allocate_exact_size(text_size + pad * 2.0, egui::Sense::click());

        let fill = theme::accent_fill(if resp.hovered() { 32 } else { 20 });
        ui.painter().rect(
            rect,
            3.0,
            fill,
            Stroke::new(1.0, theme::ACCENT),
            egui::StrokeKind::Outside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            theme::ACCENT_TEXT,
        );

        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        clicked = resp.clicked();
    });
    clicked
}
