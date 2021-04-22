use std::convert::TryFrom;
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

use midir::{Ignore, MidiInput};

use wvr_data::DataHolder;
use wvr_data::InputProvider;

pub struct DjP8Provider {
    _port: midir::MidiInputConnection<()>,
    midi_input_channel: Receiver<Vec<u8>>,

    last_left_sync_press: Option<Instant>,
    left_bpm: f32,

    last_right_sync_press: Option<Instant>,
    right_bpm: f32,

    left_low: u8,
    left_mid: u8,
    left_high: u8,

    right_low: u8,
    right_mid: u8,
    right_high: u8,

    left_pad_1: bool,
    left_pad_2: bool,
    left_pad_3: bool,
    left_pad_4: bool,

    right_pad_1: bool,
    right_pad_2: bool,
    right_pad_3: bool,
    right_pad_4: bool,

    left_play: bool,
    left_cue: bool,
    left_sync: bool,
    left_shift: bool,

    right_play: bool,
    right_cue: bool,
    right_sync: bool,
    right_shift: bool,
}

impl DjP8Provider {
    pub fn new() -> Option<Self> {
        let mut midi_in = MidiInput::new("midir forwarding input").unwrap();
        midi_in.ignore(Ignore::None);

        for i in 0..midi_in.port_count() {
            if midi_in.port_name(i).unwrap().contains("P8") {
                let (port, midi_input_channel) = {
                    let (tx, rx) = channel();

                    let port_name = midi_in.port_name(i).unwrap();
                    let port = midi_in
                        .connect(
                            i,
                            &port_name,
                            move |_timestamp, midi_message, _| {
                                tx.send(midi_message.to_vec()).unwrap();
                            },
                            (),
                        )
                        .unwrap();

                    (port, rx)
                };

                return Some(DjP8Provider {
                    _port: port,
                    midi_input_channel,

                    last_left_sync_press: None,
                    left_bpm: 0.0,

                    last_right_sync_press: None,
                    right_bpm: 0.0,

                    left_low: 0,
                    left_mid: 0,
                    left_high: 0,

                    right_low: 0,
                    right_mid: 0,
                    right_high: 0,

                    left_pad_1: false,
                    left_pad_2: false,
                    left_pad_3: false,
                    left_pad_4: false,

                    right_pad_1: false,
                    right_pad_2: false,
                    right_pad_3: false,
                    right_pad_4: false,

                    left_play: false,
                    left_cue: false,
                    left_sync: false,
                    left_shift: false,

                    right_play: false,
                    right_cue: false,
                    right_sync: false,
                    right_shift: false,
                });
            }
        }

        None
    }
}

impl InputProvider for DjP8Provider {
    fn set_name(&mut self, name: &str) {}

    fn provides(&self) -> Vec<String> {
        vec![
            "left_bpm".into(),
            "right_bpm".into(),
            "left_low".into(),
            "left_mid".into(),
            "left_high".into(),
            "right_low".into(),
            "right_mid".into(),
            "right_high".into(),
            "left_pad_1".into(),
            "left_pad_2".into(),
            "left_pad_3".into(),
            "left_pad_4".into(),
            "right_pad_1".into(),
            "right_pad_2".into(),
            "right_pad_3".into(),
            "right_pad_4".into(),
            "left_play".into(),
            "left_cue".into(),
            "left_sync".into(),
            "left_shift".into(),
            "right_play".into(),
            "right_cue".into(),
            "right_sync".into(),
            "right_shift".into(),
        ]
    }

    fn get(&mut self, uniform_name: &str, _invalidate: bool) -> Option<DataHolder> {
        if let Some(last_left_sync_press) = self.last_left_sync_press {
            if last_left_sync_press.elapsed().as_secs() > 2 {
                self.last_left_sync_press = None;
            }
        }

        if let Some(last_right_sync_press) = self.last_right_sync_press {
            if last_right_sync_press.elapsed().as_secs() > 2 {
                self.last_right_sync_press = None;
            }
        }

        let left_sync_pressed = self.left_sync;
        let right_sync_pressed = self.right_sync;

        while let Ok(message) = self.midi_input_channel.try_recv() {
            if message.is_empty() {
                continue;
            }

            if let Ok(midi_message) = wmidi::MidiMessage::try_from(message.as_ref()) {
                match midi_message {
                    wmidi::MidiMessage::ControlChange(_channel, control_number, control_value) => {
                        let control_value = u8::from(control_value);
                        match u8::from(control_number) {
                            68 => self.left_low = control_value,
                            70 => self.left_mid = control_value,
                            72 => self.left_high = control_value,

                            80 => self.right_low = control_value,
                            82 => self.right_mid = control_value,
                            84 => self.right_high = control_value,
                            _ => (),
                        }
                    }
                    wmidi::MidiMessage::NoteOn(_channel, note_number, note_value) => {
                        let note_value = u8::from(note_value);
                        match u8::from(note_number) {
                            25 => self.left_pad_1 = note_value != 0,
                            26 => self.left_pad_2 = note_value != 0,
                            27 => self.left_pad_3 = note_value != 0,
                            28 => self.left_pad_4 = note_value != 0,

                            73 => self.right_pad_1 = note_value != 0,
                            74 => self.right_pad_2 = note_value != 0,
                            75 => self.right_pad_3 = note_value != 0,
                            76 => self.right_pad_4 = note_value != 0,

                            33 => self.left_play = note_value != 0,
                            34 => self.left_cue = note_value != 0,
                            35 => self.left_sync = note_value != 0,
                            99 => self.left_shift = note_value != 0,

                            81 => self.right_play = note_value != 0,
                            82 => self.right_cue = note_value != 0,
                            83 => self.right_sync = note_value != 0,
                            47 => self.right_shift = note_value != 0,

                            x => println!("{:}", x),
                        }
                    }
                    message => println!("{:?}", message),
                }
            }
        }

        if self.left_sync && !left_sync_pressed {
            if let Some(last_left_sync_press) = self.last_left_sync_press {
                let elapsed = last_left_sync_press.elapsed();
                self.left_bpm = 60.0
                    / (elapsed.as_secs() as f32 + elapsed.subsec_micros() as f32 / 1_000_000.0);
            }
            self.last_left_sync_press = Some(Instant::now());
        }

        if self.right_sync && !right_sync_pressed {
            if let Some(last_right_sync_press) = self.last_right_sync_press {
                let elapsed = last_right_sync_press.elapsed();
                self.right_bpm = 60.0
                    / (elapsed.as_secs() as f32 + elapsed.subsec_micros() as f32 / 1_000_000.0);
            }
            self.last_right_sync_press = Some(Instant::now());
        }

        match uniform_name {
            "left_bpm" => Some(DataHolder::Float(self.left_bpm)),
            "right_bpm" => Some(DataHolder::Float(self.right_bpm)),

            "left_low" => Some(DataHolder::Float(self.left_low as f32 / 127.0)),
            "left_mid" => Some(DataHolder::Float(self.left_mid as f32 / 127.0)),
            "left_high" => Some(DataHolder::Float(self.left_high as f32 / 127.0)),

            "right_low" => Some(DataHolder::Float(self.right_low as f32 / 127.0)),
            "right_mid" => Some(DataHolder::Float(self.right_mid as f32 / 127.0)),
            "right_high" => Some(DataHolder::Float(self.right_high as f32 / 127.0)),

            "left_pad_1" => Some(DataHolder::Bool(self.left_pad_1)),
            "left_pad_2" => Some(DataHolder::Bool(self.left_pad_2)),
            "left_pad_3" => Some(DataHolder::Bool(self.left_pad_3)),
            "left_pad_4" => Some(DataHolder::Bool(self.left_pad_4)),

            "right_pad_1" => Some(DataHolder::Bool(self.right_pad_1)),
            "right_pad_2" => Some(DataHolder::Bool(self.right_pad_2)),
            "right_pad_3" => Some(DataHolder::Bool(self.right_pad_3)),
            "right_pad_4" => Some(DataHolder::Bool(self.right_pad_4)),

            "left_play" => Some(DataHolder::Bool(self.left_play)),
            "left_cue" => Some(DataHolder::Bool(self.left_cue)),
            "left_sync" => Some(DataHolder::Bool(self.left_sync)),
            "left_shift" => Some(DataHolder::Bool(self.left_shift)),

            "right_play" => Some(DataHolder::Bool(self.right_play)),
            "right_cue" => Some(DataHolder::Bool(self.right_cue)),
            "right_sync" => Some(DataHolder::Bool(self.right_sync)),
            "right_shift" => Some(DataHolder::Bool(self.right_shift)),

            _ => None,
        }
    }
}
