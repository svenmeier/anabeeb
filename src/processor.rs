use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use crokey::crossterm::event::KeyEvent;
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::info;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use regex::{escape, Regex};
use crate::disposition::general::{general_init, Disposition, Id};
use crate::disposition::fluidsynth::fluidsynth_init;
use crate::disposition::midi_in::{midi_in_init, midi_in_process};
use crate::disposition::midi_out::midi_out_init;
use crate::disposition::rest::{rest_init, rest_process, Command};
use crate::disposition::term::{term_init, term_process};
use crate::midi::log_midi;
use crate::processor::Event::MidiIn;

pub enum Event {
    MidiIn(Id, Vec<u8>),
    TermKey(Id, KeyEvent),
    Rest(Id, Command, Sender<Option<String>>),
}

pub struct Processor {
    pub events: Sender<Event>,
    receiver: Receiver<Event>,

    /// Note: sharing of midi input/output is required for WinMM, which allows a single
    /// connection to each port only (MMSYSERR_ALLOCATED).
    inputs: HashMap<String, SharedInput>,
    outputs: HashMap<String, Rc<RefCell<SharedOutput>>>,
}
impl Processor {
    pub fn new() -> Self {
        let (events, receiver) = unbounded::<Event>();
        
        Self {
            events,
            receiver,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub fn init(&mut self, disposition: &mut Disposition) -> Result<(), Box<dyn Error>> {
        
        log_midi()?;

        general_init(disposition, self)?;
        rest_init(disposition, self)?;
        term_init(disposition, self)?;
        midi_in_init(disposition, self)?;
        midi_out_init(disposition, self)?;
        fluidsynth_init(disposition, self)?;
        
        Ok(())
    }

    pub fn process(&self, disposition: &mut Disposition) -> Result<(), Box<dyn Error>> {
        loop {
            let event = self.receiver.recv()?;
            
            midi_in_process(disposition, &event);
            term_process(disposition, &event);
            rest_process(disposition, &event);
        }
    }

    pub fn midi_input(&mut self, id: &Id, name: &str) -> Result<(), Box<dyn Error>> {
        let midi_in = MidiInput::new("anabeeb").expect("no input");

        let regex = to_regex(&name)?;

        for port in midi_in.ports() {
            if let Ok(port_name) = midi_in.port_name(&port) {
                if regex.is_match(&port_name) {
                    match self.inputs.get_mut(&port_name) {
                        Some(input) => {
                            input.ids.lock().unwrap().push(id.clone());
                        },
                        None => {
                            let sender_clone = self.events.clone();
                            let ids = Arc::new(Mutex::new(vec![id.clone()]));

                            let ids_clone = ids.clone();
                            let _connection = midi_in.connect(&port, "anabeeb",
                                                             move |_, message, _| {
                                                                 let ids_lock = ids_clone.lock().unwrap();
                                                                 
                                                                 for id in ids_lock.iter() {
                                                                     sender_clone.send(MidiIn(id.clone(), message.to_vec())).unwrap();
                                                                 }
                                                             },
                                                             (),
                            )?;

                            let input = SharedInput { _connection, ids };
                            self.inputs.insert(port_name.clone(), input);
                        },
                    };

                    info!("connected {} to input port '{}'", id, port_name);
                    return Ok(());
                }
            }
        }

        Err(format!("could not connect {} to input port '{}'", id, name).into())
    }
    
    pub fn midi_output(&mut self, id: &Id, name: &str) -> Result<Output, Box<dyn Error>> {
        let midi_out = MidiOutput::new("anabeeb")?;
        let regex = to_regex(name)?;

        for port in midi_out.ports() {
            let port_name = midi_out.port_name(&port)?;
            if regex.is_match(&port_name) {
                let output = match self.outputs.get(&port_name) {
                    None => {
                        let connection = midi_out.connect(&port, "anabeeb")?;
                        let output = Rc::new(RefCell::new(SharedOutput { connection }));
                        let output_clone = output.clone();
                        self.outputs.insert(port_name.clone(), output);
                        
                        output_clone
                    },
                    Some(output) => output.clone(),
                };

                info!("connected {} to output port '{}'", id, port_name);
                return Ok(Output {output});
            }
        }

        Err(format!("could not connect {} to output port '{}'", id, name).into())
    }
}

fn to_regex(name: &str) -> Result<Regex, Box<dyn Error>> {
    let pattern = if name.starts_with('^') || name.ends_with('$') {
        name.to_string()
    } else {
        format!(".*{}.*", escape(name))
    };
    Ok(Regex::new(&pattern)?)
}

pub struct Output {
    output: Rc<RefCell<SharedOutput>>,
}
impl Output {
    pub fn send(&mut self, message: &[u8]) -> () {
        self.output.borrow_mut().connection.send(message).unwrap();
    }
}

struct SharedOutput {
    connection: MidiOutputConnection,
}

struct SharedInput {
    _connection: MidiInputConnection<()>,
    ids: Arc<Mutex<Vec<Id>>>,
}
