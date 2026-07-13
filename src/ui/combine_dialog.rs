use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use egui::{RichText, Stroke};

use crate::ui::widgets::{self, theme};

pub struct CombineDialog {
    pub open: bool,
    pub source_paths: Vec<PathBuf>,
    pub dest_name: String,
    pub dest_parent: Option<PathBuf>,
    pub running: bool,
    pub result_msg: Option<String>,
    pub progress: Arc<AtomicUsize>,
    pub total: usize,
    /// True while the "are you sure?" prompt is shown in place of the normal
    /// action row, after clicking Combine but before the operation starts.
    confirming: bool,
}

impl Default for CombineDialog {
    fn default() -> Self {
        Self {
            open: false,
            source_paths: Vec::new(),
            dest_name: "combined".to_string(),
            dest_parent: None,
            running: false,
            result_msg: None,
            progress: Arc::new(AtomicUsize::new(0)),
            total: 0,
            confirming: false,
        }
    }
}

pub enum CombineAction {
    None,
    PickSources,
    PickDest,
    Run { sources: Vec<PathBuf>, dest: PathBuf },
    Close,
}

impl CombineDialog {
    pub fn show(&mut self, ctx: &egui::Context) -> CombineAction {
        if !self.open {
            return CombineAction::None;
        }

        let mut action = CombineAction::None;
        let mut open = self.open;

        egui::Window::new("Combine folders")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .min_width(420.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Source folders:");
                    if !self.running && ui.button("Add folders…").clicked() {
                        action = CombineAction::PickSources;
                    }
                });
                ui.add_space(4.0);

                // Inputs dim (rather than just silently disabling) while a
                // combine is running, so it's obvious why nothing responds.
                ui.scope(|ui| {
                    if self.running {
                        ui.set_opacity(0.4);
                    }

                    if self.source_paths.is_empty() {
                        ui.label(RichText::new("No folders selected.").color(theme::TEXT_MUTED));
                    } else {
                        let mut to_remove: Option<usize> = None;
                        egui::Frame::default()
                            .fill(theme::SURFACE)
                            .corner_radius(theme::RADIUS_SM)
                            .inner_margin(egui::Margin::same(6i8))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(120.0)
                                    .show(ui, |ui| {
                                        for (i, path) in self.source_paths.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if !self.running {
                                                        let remove_btn = egui::Button::new(
                                                            RichText::new("Remove").size(11.5).color(theme::ERROR_TEXT),
                                                        )
                                                        .fill(theme::error_fill(20))
                                                        .stroke(Stroke::new(1.0, theme::error_fill(90)))
                                                        .corner_radius(theme::RADIUS_SM);
                                                        if ui.add(remove_btn).clicked() {
                                                            to_remove = Some(i);
                                                        }
                                                        ui.add_space(6.0);
                                                    }
                                                    ui.label(path.to_string_lossy().as_ref());
                                                });
                                            });
                                        }
                                    });
                            });
                        if let Some(i) = to_remove {
                            self.source_paths.remove(i);
                        }
                    }

                    ui.add_space(10.0);
                    ui.label("Output folder name:");
                    ui.add_enabled_ui(!self.running, |ui| {
                        ui.text_edit_singleline(&mut self.dest_name);
                    });

                    ui.add_space(6.0);
                    ui.label("Output parent directory:");
                    ui.horizontal(|ui| {
                        let dest_text = self
                            .dest_parent
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "not selected".to_string());
                        let color = if self.dest_parent.is_some() {
                            theme::TEXT
                        } else {
                            theme::TEXT_MUTED
                        };
                        ui.label(RichText::new(&dest_text).color(color));
                        if !self.running && ui.button("Browse…").clicked() {
                            action = CombineAction::PickDest;
                        }
                    });
                });

                ui.add_space(12.0);

                if self.running {
                    let done = self.progress.load(Ordering::Relaxed);
                    let total = self.total;
                    let fraction = if total > 0 { done as f32 / total as f32 } else { 0.0 };
                    ui.label(RichText::new("Combining…").color(theme::ACCENT_TEXT).size(12.5));
                    ui.add_space(2.0);
                    ui.add(egui::ProgressBar::new(fraction)
                        .text(format!("{}/{} copied", done, total))
                        .animate(true));
                    ui.add_space(6.0);
                }

                if let Some(msg) = &self.result_msg {
                    ui.colored_label(theme::SUCCESS, msg);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if self.confirming {
                        ui.label(
                            RichText::new("Combine these folders now?").color(theme::TEXT),
                        );
                        ui.add_space(8.0);
                        if widgets::accent_button(ui, "Yes, combine").clicked() {
                            if let Some(parent) = &self.dest_parent {
                                let dest = parent.join(self.dest_name.trim());
                                action = CombineAction::Run {
                                    sources: self.source_paths.clone(),
                                    dest,
                                };
                            }
                            self.confirming = false;
                        }
                        ui.add_space(4.0);
                        if ui.button("Cancel").clicked() {
                            self.confirming = false;
                        }
                    } else {
                        let can_run = !self.source_paths.is_empty()
                            && self.dest_parent.is_some()
                            && !self.dest_name.trim().is_empty()
                            && !self.running;

                        ui.add_enabled_ui(can_run, |ui| {
                            if widgets::accent_button(ui, "Combine").clicked() {
                                self.confirming = true;
                            }
                        });

                        if !self.running && ui.button("Close").clicked() {
                            action = CombineAction::Close;
                        }
                    }
                });
            });

        if !open {
            action = CombineAction::Close;
        }

        action
    }
}
