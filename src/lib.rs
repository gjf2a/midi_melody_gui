use std::path::PathBuf;

use eframe::egui::{self, FontDefinitions};

pub mod melody_renderer;
pub mod recorder;

pub fn setup_font(filename: &str, cc: &eframe::CreationContext<'_>) -> anyhow::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let file_path = PathBuf::from(manifest_dir).join(filename);
    let bytes = std::fs::read(&file_path)?;
    let name = filename_sans_suffix(&file_path);
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        name.clone(),
        eframe::egui::FontData::from_owned(bytes).into(),
    );
    fonts
        .families
        .get_mut(&eframe::egui::FontFamily::Proportional)
        .unwrap()
        .push(name);
    cc.egui_ctx.set_fonts(fonts);
    Ok(())
}

pub fn filename_sans_suffix(path: &PathBuf) -> String {
    path.file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .split(".")
        .next()
        .unwrap()
        .to_owned()
}

pub fn render_synth_sounds(
    label: &str,
    target: &mut usize,
    sounds: &Vec<String>,
    ui: &mut egui::Ui,
) -> Option<usize> {
    let start = *target;
    ui.vertical(|ui| {
        ui.label(label);
        for (i, name) in sounds.iter().enumerate() {
            ui.radio_value(target, i, name);
        }
    });
    if start != *target {
        Some(*target)
    } else {
        None
    }
}
