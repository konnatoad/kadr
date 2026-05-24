use std::path::PathBuf;

pub struct CombineDialog {
    pub open: bool,
    pub source_path: Option<PathBuf>,
    pub dest_name: String,
    pub dest_parent: Option<PathBuf>,
    pub running: bool,
    pub result_msg: Option<String>,
}

impl Default for CombineDialog {
    fn default() -> Self {
        Self {
            open: false,
            source_path: None,
            dest_name: "combined".to_string(),
            dest_parent: None,
            running: false,
            result_msg: None,
        }
    }
}

pub enum CombineAction {
    None,
    PickSource,
    PickDest,
    Run { source: PathBuf, dest: PathBuf },
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
                ui.label("Source folder (will be scanned recursively):");
                ui.horizontal(|ui| {
                    let src_text = self
                        .source_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "not selected".to_string());
                    ui.label(&src_text);
                    if ui.button("Browse…").clicked() {
                        action = CombineAction::PickSource;
                    }
                });

                ui.add_space(8.0);
                ui.label("Output folder name:");
                ui.text_edit_singleline(&mut self.dest_name);

                ui.add_space(4.0);
                ui.label("Output parent directory:");
                ui.horizontal(|ui| {
                    let dest_text = self
                        .dest_parent
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "not selected".to_string());
                    ui.label(&dest_text);
                    if ui.button("Browse…").clicked() {
                        action = CombineAction::PickDest;
                    }
                });

                ui.add_space(12.0);

                if let Some(msg) = &self.result_msg {
                    ui.colored_label(egui::Color32::from_rgb(100, 220, 100), msg);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    let can_run = self.source_path.is_some()
                        && self.dest_parent.is_some()
                        && !self.dest_name.trim().is_empty()
                        && !self.running;

                    ui.add_enabled_ui(can_run, |ui| {
                        if ui.button("Combine").clicked() {
                            if let (Some(src), Some(parent)) =
                                (&self.source_path, &self.dest_parent)
                            {
                                let dest = parent.join(self.dest_name.trim());
                                action = CombineAction::Run {
                                    source: src.clone(),
                                    dest,
                                };
                            }
                        }
                    });

                    if ui.button("Close").clicked() {
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
