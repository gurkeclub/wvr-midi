use std::convert::TryFrom;
use std::sync::mpsc::{channel, Receiver};
use std::usize;

use anyhow::Result;

use midir::{Ignore, MidiInput};

use wvr_data::types::DataHolder;
use wvr_data::types::InputProvider;

pub struct MidiProvider {
    name: String,

    time: f64,

    _port: midir::MidiInputConnection<()>,
    midi_input_channel: Receiver<Vec<u8>>,

    pressed: [bool; 1024],
    pressed_time: [f64; 1024],

    toggled: [bool; 1024],
    toggled_time: [f64; 1024],

    values: [u8; 1024],
}

impl MidiProvider {
    pub fn new(name: String, port_name: String) -> Result<Self> {
        let mut midi_in = MidiInput::new(&name).unwrap();
        midi_in.ignore(Ignore::None);

        for i in 0..midi_in.port_count() {
            println!("{:?}", midi_in.port_name(i).unwrap());

            if midi_in.port_name(i).unwrap().contains(&port_name) {
                let (port, midi_input_channel) = {
                    let (tx, rx) = channel();

                    let port_name = midi_in.port_name(i).unwrap();

                    let port = midi_in
                        .connect(
                            i,
                            &port_name,
                            move |_timestamp, midi_message, _| {
                                tx.send(midi_message.to_vec())
                                    .expect("Could not send midi message to midi message receiver");
                            },
                            (),
                        )
                        .unwrap();

                    (port, rx)
                };

                return Ok(MidiProvider {
                    name,
                    time: 0.0,

                    _port: port,
                    midi_input_channel,

                    pressed: [false; 1024],
                    pressed_time: [0.0; 1024],
                    toggled: [false; 1024],
                    toggled_time: [0.0; 1024],

                    values: [0; 1024],
                });
            }
        }

        Result::Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                r#"Could not find midi device with matching port name matching "{:}""#,
                port_name
            ),
        ))?
    }
}

impl InputProvider for MidiProvider {
    fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    fn provides(&self) -> Vec<String> {
        vec![
            format!("{:}.pressed", self.name),
            format!("{:}.toggled", self.name),
            format!("{:}.values", self.name),
        ]
    }

    fn set_property(&mut self, _property: &str, _value: &DataHolder) {}

    fn get(&mut self, uniform_name: &str, _invalidate: bool) -> Option<DataHolder> {
        while let Ok(message) = self.midi_input_channel.try_recv() {
            if message.is_empty() {
                continue;
            }

            if let Ok(midi_message) = wmidi::MidiMessage::try_from(message.as_ref()) {
                match midi_message {
                    wmidi::MidiMessage::ControlChange(_channel, control_number, control_value) => {
                        let control_number = u8::from(control_number.0);

                        self.values[control_number as usize] = u8::from(control_value);

                        println!("val {:} ({:})", control_number, self.name);
                    }
                    wmidi::MidiMessage::NoteOn(_channel, note_number, note_value) => {
                        let note_value = u8::from(note_value);
                        let note_number = note_number as usize;

                        let was_pressed = self.pressed[note_number];

                        if note_value > 0 {
                            self.pressed[note_number] = true;
                        } else {
                            self.pressed[note_number] = false
                        }

                        if !was_pressed && self.pressed[note_number] {
                            self.toggled[note_number] = !self.toggled[note_number];
                            if self.pressed[note_number] {
                                self.pressed_time[note_number] = self.time;
                            }
                            self.toggled_time[note_number] = self.time;
                        }

                        println!("on {:} ({:})", note_number, self.name);
                    }
                    wmidi::MidiMessage::NoteOff(_channel, note_number, _note_value) => {
                        let note_number = note_number as usize;
                        let was_pressed = self.pressed[note_number];

                        self.pressed[note_number] = false;

                        if was_pressed != self.pressed[note_number] {
                            self.toggled[note_number] = !self.toggled[note_number];
                            self.toggled_time[note_number] = self.time;
                        }

                        println!("of {:} ({:})", note_number, self.name);
                    }
                    message => println!("{:?}", message),
                }
            }
        }

        if uniform_name.starts_with("pressed_time") {
            if let Ok(index) = uniform_name.split('.').nth(1)?.parse::<usize>() {
                if index < self.pressed_time.len() {
                    return Some(DataHolder::Float(self.pressed_time[index] as f32));
                }
            }
        }
        if uniform_name.starts_with("pressed") {
            if let Ok(index) = uniform_name.split('.').nth(1)?.parse::<usize>() {
                if index < self.pressed.len() {
                    return Some(DataHolder::Bool(self.pressed[index]));
                }
            }
        }
        if uniform_name.starts_with("toggled") {
            if let Ok(index) = uniform_name.split('.').nth(1)?.parse::<usize>() {
                if index < self.pressed.len() {
                    return Some(DataHolder::Bool(self.toggled[index]));
                }
            }
        }
        if uniform_name.starts_with("value") {
            if let Ok(index) = uniform_name.split('.').nth(1)?.parse::<usize>() {
                if index < self.pressed.len() {
                    return Some(DataHolder::Int(self.values[index] as i32));
                }
            }
        }

        if uniform_name == format!("{:}.pressed", self.name) {
            Some(DataHolder::BoolArray(self.pressed.to_vec()))
        } else if uniform_name == format!("{:}.toggled", self.name) {
            Some(DataHolder::BoolArray(self.toggled.to_vec()))
        } else if uniform_name == format!("{:}.values", self.name) {
            Some(DataHolder::ByteArray(self.values.to_vec()))
        } else {
            None
        }
    }

    fn set_time(&mut self, time: f64, _sync: bool) {
        self.time = time;
    }
}
