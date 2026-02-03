use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::io::{
    Speaker, SynthMsg, get_first_midi_device, start_input_thread, start_output_thread,
};
use midi_fundsp::note_velocity_from;
use midi_fundsp::sound_builders::ProgramTable;
use midi_note_recorder::Recording;
use midir::MidiInput;
use std::ops::Index;
use std::sync::Mutex;
use std::{sync::Arc, time::Instant};

pub const NUM_CHANNELS: usize = 10;
pub const DEFAULT_TIMEOUT: f64 = 2.0;

pub trait SynthMsgReceiver: Send {
    fn receive(&mut self, msg: SynthMsg);
    fn live_speaker(&self) -> Speaker;
    fn new(
        timeout: f64,
        incoming: Arc<SegQueue<SynthMsg>>,
        outgoing: Arc<SegQueue<SynthMsg>>,
        input_port_name: String,
    ) -> Self;
}

fn start_monitor_thread<R: SynthMsgReceiver + 'static>(
    incoming: Arc<SegQueue<SynthMsg>>,
    outgoing: Arc<SegQueue<SynthMsg>>,
    quit: Arc<AtomicCell<bool>>,
    recorder: Arc<Mutex<R>>,
) {
    std::thread::spawn(move || {
        while !quit.load() {
            if let Some(msg) = incoming.pop() {
                let mut recorder = recorder.lock().unwrap();
                let mut outgoing_msg = msg.clone();
                outgoing_msg.speaker = recorder.live_speaker();
                outgoing.push(outgoing_msg);
                recorder.receive(msg);
            }
        }
    });
}

pub fn setup_threads<R: SynthMsgReceiver + 'static>(
    synth_sounds: ProgramTable,
) -> anyhow::Result<Arc<Mutex<R>>> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let input2monitor = Arc::new(SegQueue::new());
    let monitor2output = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));
    let recorder = Arc::new(Mutex::new(R::new(
        DEFAULT_TIMEOUT,
        input2monitor.clone(),
        monitor2output.clone(),
        midi_in.port_name(&in_port)?,
    )));
    start_input_thread(input2monitor.clone(), midi_in, in_port, quit.clone());
    start_monitor_thread(
        input2monitor,
        monitor2output.clone(),
        quit,
        recorder.clone(),
    );
    start_output_thread::<NUM_CHANNELS>(monitor2output, Arc::new(Mutex::new(synth_sounds)));
    Ok(recorder)
}

pub struct Recorder {
    pub timeout: f64,
    recordings: Vec<Recording>,
    solo_duration: Option<f64>,
    outgoing: Arc<SegQueue<SynthMsg>>,
    last_msg: Instant,
    current_start: Instant,
    input_port_name: String,
}

impl SynthMsgReceiver for Recorder {
    fn new(
        timeout: f64,
        _: Arc<SegQueue<SynthMsg>>,
        outgoing: Arc<SegQueue<SynthMsg>>,
        input_port_name: String,
    ) -> Self {
        Self {
            timeout,
            recordings: vec![],
            solo_duration: None,
            outgoing,
            last_msg: Instant::now(),
            current_start: Instant::now(),
            input_port_name,
        }
    }

    fn receive(&mut self, msg: SynthMsg) {
        let now = Instant::now();
        if !self.actively_recording() {
            self.recordings.push(Recording::default());
            self.current_start = now;
        }
        self.recordings.last_mut().unwrap().add_message(
            now.duration_since(self.current_start).as_secs_f64(),
            &msg.msg,
        );
        self.last_msg = now;
    }

    fn live_speaker(&self) -> Speaker {
        Speaker::Both
    }
}

impl Recorder {
    pub fn program_change(&self, program: u8, speaker: Speaker) {
        self.outgoing
            .push(SynthMsg::program_change(program, speaker));
    }

    pub fn len(&self) -> usize {
        self.recordings.len()
    }

    pub fn delete_last_recording(&mut self) {
        if self.recordings.len() > 0 {
            self.recordings.pop();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn input_port_name(&self) -> &str {
        self.input_port_name.as_str()
    }

    pub fn actively_recording(&self) -> bool {
        if let Some(recent) = self.recordings.last() {
            if let Some((_, last_msg)) = recent.last() {
                if let Some((_, velocity)) = note_velocity_from(&last_msg) {
                    return velocity > 0
                        || Instant::now().duration_since(self.last_msg).as_secs_f64()
                            < self.timeout;
                }
            }
        }
        false
    }

    pub fn actively_soloing(&self) -> bool {
        self.solo_duration.is_some()
    }
}

impl Index<usize> for Recorder {
    type Output = Recording;

    fn index(&self, index: usize) -> &Self::Output {
        &self.recordings[index]
    }
}
