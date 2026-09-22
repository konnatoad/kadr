use std::path::PathBuf;

use egui::RichText;

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
        let mut open = self.open;

        egui::Window::new("Open Folders")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Folders:");
                    if ui.button("Add folder…").clicked() {
                        action = FoldersAction::AddFolders;
                    }
                });
                ui.add_space(4.0);

                if self.paths.is_empty() {
                    ui.label(RichText::new("No folders added yet.").color(theme::TEXT_MUTED));
                } else {
                    let mut to_remove: Option<usize> = None;
                    egui::Frame::default()
                        .fill(theme::SURFACE)
                        .corner_radius(theme::RADIUS_SM)
                        .inner_margin(egui::Margin::same(6i8))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(160.0)
                                .show(ui, |ui| {
                                    for (i, path) in self.paths.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("Remove").clicked() {
                                                    to_remove = Some(i);
                                                }
                                                ui.add_space(6.0);
                                                ui.label(path.to_string_lossy().as_ref());
                                            });
                                        });
                                    }
                                });
                        });
                    if let Some(i) = to_remove {
                        self.paths.remove(i);
                    }
                }

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.paths.is_empty(), |ui| {
                        if ui.button("Open").clicked() {
                            action = FoldersAction::Open(self.paths.clone());
                        }
                    });
                    if ui.button("Cancel").clicked() {
                        action = FoldersAction::Cancel;
                    }
                });
            });

        if !open {
            action = FoldersAction::Cancel;
        }

        action
    }
}
