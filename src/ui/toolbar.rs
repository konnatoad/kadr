use egui::{Color32, RichText, Stroke, Ui};

use crate::fs::sorter::SortMode;

pub struct ToolbarResponse {
    pub open_folder: bool,
    pub open_file: bool,
    pub combine: bool,
    pub settings: bool,
    pub sort_changed: Option<SortMode>,
    pub toggle_images: bool,
    pub toggle_videos: bool,
    pub toggle_subfolders: bool,
    pub slideshow: bool,
}

impl Default for ToolbarResponse {
    fn default() -> Self {
        Self {
            open_folder: false,
            open_file: false,
            combine: false,
            settings: false,
            sort_changed: None,
            toggle_images: false,
            toggle_videos: false,
            toggle_subfolders: false,
            slideshow: false,
        }
    }
}

pub fn show_toolbar(
    ui: &mut Ui,
    current_sort: &SortMode,
    filter_images: bool,
    filter_videos: bool,
    scan_subfolders: bool,
    slideshow_active: bool,
    image_count: usize,
    current_index: Option<usize>,
) -> ToolbarResponse {
    let mut resp = ToolbarResponse::default();

    let frame_resp = egui::Frame::default()
        .fill(Color32::from_rgb(16, 16, 20))
        .inner_margin(egui::Margin::symmetric(10i8, 5i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                action_btn(ui, "Open Folder", || resp.open_folder = true);
                action_btn(ui, "Open File",   || resp.open_file   = true);

                vsep(ui);

                egui::ComboBox::from_id_salt("sort_mode")
                    .selected_text(current_sort.label())
                    .width(168.0)
                    .show_ui(ui, |ui| {
                        for mode in SortMode::all() {
                            if ui.selectable_label(current_sort == mode, mode.label()).clicked() {
                                resp.sort_changed = Some(mode.clone());
                            }
                        }
                    });

                vsep(ui);

                pill_toggle(ui, "Images",      filter_images,    || resp.toggle_images    = true);
                pill_toggle(ui, "Videos",      filter_videos,    || resp.toggle_videos    = true);
                pill_toggle(ui, "Subfolders",  scan_subfolders,  || resp.toggle_subfolders = true);

                vsep(ui);

                if slideshow_active {
                    let btn = egui::Button::new(
                        RichText::new("Stop").color(Color32::from_rgb(255, 110, 90)).size(12.5),
                    )
                    .fill(Color32::from_rgba_premultiplied(255, 80, 60, 22))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 80, 60, 100)));
                    if ui.add(btn).clicked() { resp.slideshow = true; }
                } else {
                    action_btn(ui, "Slideshow", || resp.slideshow = true);
                }

                vsep(ui);
                action_btn(ui, "Combine", || resp.combine = true);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    action_btn(ui, "Settings", || resp.settings = true);

                    if let Some(idx) = current_index {
                        vsep(ui);
                        ui.label(
                            RichText::new(format!("{} / {}", idx + 1, image_count))
                                .color(Color32::from_gray(130))
                                .size(11.5)
                                .monospace(),
                        );
                    }
                });
            });
        });

    // Thin accent line along the bottom edge of the toolbar
    let r = frame_resp.response.rect;
    ui.painter().line_segment(
        [r.left_bottom(), r.right_bottom()],
        Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 55)),
    );

    resp
}

fn action_btn(ui: &mut Ui, label: &str, mut on_click: impl FnMut()) {
    if ui.button(RichText::new(label).size(12.5)).clicked() {
        on_click();
    }
}

fn pill_toggle(ui: &mut Ui, label: &str, active: bool, mut on_click: impl FnMut()) {
    let (bg, text_col, stroke) = if active {
        (
            Color32::from_rgba_premultiplied(99, 155, 255, 38),
            Color32::from_rgb(145, 190, 255),
            Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 170)),
        )
    } else {
        (
            Color32::from_rgba_premultiplied(255, 255, 255, 7),
            Color32::from_gray(135),
            Stroke::new(1.0, Color32::from_gray(50)),
        )
    };
    let btn = egui::Button::new(RichText::new(label).size(12.0).color(text_col))
        .fill(bg)
        .stroke(stroke);
    if ui.add(btn).clicked() { on_click(); }
}

fn vsep(ui: &mut Ui) {
    ui.add_space(3.0);
    ui.add(egui::Separator::default().vertical().spacing(6.0));
    ui.add_space(3.0);
}
