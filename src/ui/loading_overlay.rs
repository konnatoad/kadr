use egui::RichText;

use crate::ui::widgets::theme;

pub fn show(ctx: &egui::Context, done: usize, total: usize) -> bool {
    let mut cancel = false;
    egui::Window::new("loading_overlay")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(280.0);
            let label = if total == 0 {
                "Scanning folders…".to_owned()
            } else {
                format!("Loading images… {done} / {total}")
            };
            ui.label(RichText::new(label).color(theme::TEXT).size(13.5));
            ui.add_space(8.0);
            let fraction = if total > 0 { done as f32 / total as f32 } else { 0.0 };
            ui.add(egui::ProgressBar::new(fraction).animate(true));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        });
    cancel
}
