use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct CombineDialog {
    pub open: bool,
    pub source_paths: Vec<PathBuf>,
    pub dest_name: String,
    pub dest_parent: Option<PathBuf>,
    pub running: bool,
    pub result_msg: Option<String>,
    pub progress: Arc<AtomicUsize>,
    pub total: usize,
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
            .min_width(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Source folders:");
                    if !self.running {
                        if ui.button("Add folders…").clicked() {
                            action = CombineAction::PickSources;
                        }
                    }
                });

                if self.source_paths.is_empty() {
                    ui.label("No folders selected.");
                } else {
                    let mut to_remove: Option<usize> = None;
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (i, path) in self.source_paths.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if !self.running && ui.small_button("X").clicked() {
                                        to_remove = Some(i);
                                    }
                                    ui.label(path.to_string_lossy().as_ref());
                                });
                            }
                        });
                    if let Some(i) = to_remove {
                        self.source_paths.remove(i);
                    }
                }

                ui.add_space(8.0);
                ui.label("Output folder name:");
                ui.add_enabled_ui(!self.running, |ui| {
                    ui.text_edit_singleline(&mut self.dest_name);
                });

                ui.add_space(4.0);
                ui.label("Output parent directory:");
                ui.horizontal(|ui| {
                    let dest_text = self
                        .dest_parent
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "not selected".to_string());
                    ui.label(&dest_text);
                    if !self.running && ui.button("Browse…").clicked() {
                        action = CombineAction::PickDest;
                    }
                });

                ui.add_space(12.0);

                if self.running {
                    let done = self.progress.load(Ordering::Relaxed);
                    let total = self.total;
                    let fraction = if total > 0 { done as f32 / total as f32 } else { 0.0 };
                    ui.add(egui::ProgressBar::new(fraction)
                        .text(format!("{}/{} copied", done, total))
                        .animate(true));
                    ui.add_space(4.0);
                }

                if let Some(msg) = &self.result_msg {
                    ui.colored_label(egui::Color32::from_rgb(100, 220, 100), msg);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    let can_run = !self.source_paths.is_empty()
                        && self.dest_parent.is_some()
                        && !self.dest_name.trim().is_empty()
                        && !self.running;

                    ui.add_enabled_ui(can_run, |ui| {
                        if ui.button("Combine").clicked() {
                            if let Some(parent) = &self.dest_parent {
                                let dest = parent.join(self.dest_name.trim());
                                action = CombineAction::Run {
                                    sources: self.source_paths.clone(),
                                    dest,
                                };
                            }
                        }
                    });

                    if !self.running && ui.button("Close").clicked() {
                        action = CombineAction::Close;
                    }
                });
            });

        if !open {
            action = CombineAction::Close;
        }

        action
    }
}
