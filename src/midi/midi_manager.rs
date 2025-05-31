use std::error::Error;
use log::info;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use regex::{escape, Regex};

pub struct MidiManager {

}

impl MidiManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn connect_input<F>(&self,
                            name: &String,
                            handler: F,
    ) -> Result<(String, MidiInputConnection<()>), Box<dyn Error>>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        let midi_in = MidiInput::new("anabeeb").expect("no input");

        let regex = get_regex(&name);

        for port in midi_in.ports() {
            if let Ok(port_name) = midi_in.port_name(&port) {
                if regex.is_match(&port_name) {
                    let conn = midi_in.connect(&port, "anabeeb",
                                               move |_, message, _| {
                                                   handler(message.to_vec());
                                               },
                                               (),
                    )?;
                    return Ok((port_name, conn));
                }
            }
        }

        Err(format!("could not connect input to port '{}'", name).into())
    }

    pub fn connect_output(&self, name: &String) -> Result<(String, MidiOutputConnection), Box<dyn Error>> {
        let midi_out = MidiOutput::new("anabeeb").expect("no output");

        let regex = get_regex(&name);

        for port in midi_out.ports() {
            if let Ok(port_name) = midi_out.port_name(&port) {
                if regex.is_match(&port_name) {
                    let conn = midi_out.connect(&port, "anabeeb")?;
                    return Ok((port_name, conn));
                }
            }
        }

        Err(format!("could not connect output to port '{}'", name).into())
    }


    pub fn log_midi(&self) -> Result<(), Box<dyn Error>> {
        let midi_in = MidiInput::new("anabeeb")?;
        for (_i, port) in midi_in.ports().iter().enumerate() {
            info!("Input Port '{}'", midi_in.port_name(port)?);
        }

        let midi_out = MidiOutput::new("anabeeb")?;
        for (_, port) in midi_out.ports().iter().enumerate() {
            info!("Output Port '{}'", midi_out.port_name(port)?);
        }
        Ok(())
    }
}

fn get_regex(name: &String) -> Regex {
    let pattern = if name.starts_with('^') || name.ends_with('$') {
        name.clone()
    } else {
        escape(name)
    };

    Regex::new(&pattern).unwrap()
}