use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, Pos2, Vec2, Visuals};
use midi_fundsp::{io::Speaker, sound_builders::ProgramTable, sounds::favorites};
use midi_melody_gui::{
    melody_renderer::MelodyRenderer,
    recorder::{Recorder, setup_threads},
    render_synth_sounds, setup_font,
};
use music_analyzer_generator::analyzer::Melody;

const FPS: f32 = 20.0;
const FRAME_INTERVAL: f32 = 1.0 / FPS;

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2 { x: 800.0, y: 600.0 })
            .with_position(Pos2 { x: 50.0, y: 25.0 })
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "MIDI Melody GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(MainApp::new(cc).unwrap()))),
    )
    .unwrap();
}

struct MainApp {
    recorder: Arc<Mutex<Recorder>>,
    synth_sounds: ProgramTable,
    synth_sound: usize,
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(Visuals::light());
        egui::CentralPanel::default().show(ctx, |ui| {
            let heading = format!("MIDI Melody GUI ({})", self.port_name());
            ui.heading(heading);
            self.render_settings(ui);
            let recorder = self.recorder.lock().unwrap();
            if recorder.len() > 0 {
                let melody = Melody::from(&recorder[recorder.len() - 1]);
                MelodyRenderer::render(ui, &vec![(melody, Color32::BLACK)]);
            }
            ctx.request_repaint_after_secs(FRAME_INTERVAL);
        });
    }
}

impl MainApp {
    fn new(cc: &eframe::CreationContext<'_>) -> anyhow::Result<Self> {
        setup_font("bravura/BravuraText.otf", cc)?;
        let synth_sounds = favorites();
        Ok(Self {
            recorder: setup_threads(synth_sounds.clone())?,
            synth_sounds,
            synth_sound: 0,
        })
    }

    fn port_name(&self) -> String {
        self.recorder.lock().unwrap().input_port_name().to_string()
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        let sounds = self
            .synth_sounds
            .iter()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>();
        if let Some(changed) =
            render_synth_sounds("Synth Sounds", &mut self.synth_sound, &sounds, ui)
        {
            self.recorder
                .lock()
                .unwrap()
                .program_change(changed as u8, Speaker::Both);
        }
    }
}
