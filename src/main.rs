use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use bare_metal_modulo::{MNum, ModNum};
use eframe::egui::{self, Color32, Pos2, Vec2, Visuals};
use midi_fundsp::{
    io::Speaker, note_velocity_from, sound_builders::ProgramTable, sounds::favorites,
};
use midi_melody_gui::{
    melody_renderer::MelodyRenderer,
    recorder::{Recorder, setup_threads},
    render_synth_sounds, setup_font,
};
use midi_msg::MidiMsg;
use music_analyzer_generator::{
    analyzer::{Melody, MelodyDirection},
    scales::RootedScale,
};

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
    current_recording: ModNum<usize>,
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(Visuals::light());
        egui::CentralPanel::default().show(ctx, |ui| {
            let heading = format!("MIDI Melody GUI ({})", self.port_name());
            ui.heading(heading);
            ui.horizontal(|ui| {
                self.render_settings(ui);
                self.render_midi_instructions(ui);
            });
            self.render_melody_choice(ui);
            self.render_melody(ui);
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
            current_recording: ModNum::new(0, 1),
        })
    }

    fn port_name(&self) -> String {
        self.recorder.lock().unwrap().input_port_name().to_string()
    }

    fn render_melody_choice(&mut self, ui: &mut egui::Ui) {
        let recorder = self.recorder.lock().unwrap();
        if recorder.len() > 1 {
            if recorder.len() > self.current_recording.m() {
                self.current_recording = ModNum::new(recorder.len() - 1, recorder.len());
            }
            ui.horizontal(|ui| {
                if ui.button("<").clicked() {
                    self.current_recording -= 1;
                }
                ui.label(format!(
                    "Recording {}/{}",
                    self.current_recording.a() + 1,
                    recorder.len()
                ));
                if ui.button(">").clicked() {
                    self.current_recording += 1;
                }
            });
        }
    }

    fn render_melody(&mut self, ui: &mut egui::Ui) {
        let recorder = self.recorder.lock().unwrap();
        if recorder.len() > 0 {
            let melody = Melody::from(&recorder[self.current_recording.a()]);
            MelodyRenderer::render(ui, &vec![(melody, Color32::BLACK)]);
        }
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

    fn render_midi_instructions(&mut self, ui: &mut egui::Ui) {
        let recorder = self.recorder.lock().unwrap();
        if recorder.len() > 0 {
            let recording = &recorder[self.current_recording.a()];
            let melody = Melody::from(recording);
            let scale = melody.highest_weight_scale();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("MIDI instructions")
                    .num_columns(4)
                    .spacing((10.0, 4.0))
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Timestamp");
                        ui.label("Pitch");
                        ui.label("Velocity");
                        ui.label("Note");
                        ui.end_row();
                        self.render_midi_instruction_rows(ui, &scale, recording.midi_queue());
                    });
            });
        }
    }

    fn render_midi_instruction_rows(
        &self,
        ui: &mut egui::Ui,
        scale: &RootedScale,
        mut msgs: VecDeque<(f64, MidiMsg)>,
    ) {
        let mut last_pitch = None;
        while let Some((time, msg)) = msgs.pop_front() {
            if let Some((note, velocity)) = note_velocity_from(&msg) {
                ui.label(format!("{time:.2}"));
                ui.label(format!("{note}"));
                ui.label(format!("{velocity}"));
                let direction = Self::pick_direction(note, last_pitch);
                let (name, _, accidental) = scale.matching_pitch(note, direction);
                if let Some(accidental) = accidental {
                    let name = name.with_acc(accidental);
                    ui.label(format!("{name}*"));
                } else {
                    ui.label(format!("{name}"));
                }
                ui.end_row();
                last_pitch = Some(note);
            }
        }
    }

    fn pick_direction(pitch: u8, last_pitch: Option<u8>) -> MelodyDirection {
        last_pitch.map_or(MelodyDirection::Ascending, |lp| {
            if lp < pitch {
                MelodyDirection::Ascending
            } else {
                MelodyDirection::Descending
            }
        })
    }
}
