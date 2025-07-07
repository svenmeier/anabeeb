use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::process::exit;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use crokey::crossterm::event::KeyEvent;
use crossbeam_channel::{unbounded, Receiver, Sender};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use regex::{escape, Regex};
use crate::disposition::general::{general_init, Disposition, Element, Id};
use crate::disposition::fluidsynth::fluidsynth_init;
use crate::disposition::midi_in::{midi_in_init, midi_in_process};
use crate::disposition::midi_out::midi_out_init;
use crate::disposition::rest::{rest_init, rest_process, Command};
use crate::disposition::term::{term_init, term_process};
use crate::midi::log_midi_ports;
use crate::{print_error, print_info, Args};
use crate::io::{combine_paths, write_disposition, write_disposition_override, write_memory};
use crate::processor::Event::MidiIn;
use crate::setup::setup;

pub enum Event {
    Error(Id, String),
    MidiIn(Id, Vec<u8>),
    TermKey(Id, KeyEvent),
    Rest(Id, Command, Sender<Option<String>>),
}

pub struct ProcessingError {
    pub id: Id,
    pub message: String,
}

pub struct Processor {
    pub events: Sender<Event>,
    receiver: Receiver<Event>,
    errors: Vec<ProcessingError>,
    args: Args,

    /// Note: sharing of midi input/output is required for WinMM, which allows a single
    /// connection to each port only (MMSYSERR_ALLOCATED).
    inputs: HashMap<String, SharedInput>,
    outputs: HashMap<String, Rc<RefCell<SharedOutput>>>,
}

impl Processor {
    pub fn new(args: Args) -> Self {
        let (events, receiver) = unbounded::<Event>();
        
        Self {
            events,
            receiver,
            args,
            errors: Vec::new(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        }
    }
    
    pub fn init(&mut self, disposition: &mut Disposition) {
        
        log_midi_ports();

        if self.args.setup {
            match setup(disposition, &self) {
                Ok(()) => print_info!("setup completed"),
                Err(e) => {
                    print_error!("setup failed: {}", e);
                    exit(1);
                },
            }
        }

        general_init(disposition, self);
        rest_init(disposition, self);
        term_init(disposition, self);
        midi_in_init(disposition, self);
        midi_out_init(disposition, self);
        fluidsynth_init(disposition, self);
    }

    pub fn process(&mut self, disposition: &mut Disposition) {
        loop {
            let event = self.receiver.recv().unwrap();

            if let Event::Error(id, message) = event {
                self.errors.push(ProcessingError { id, message });
            } else {
                midi_in_process(disposition, self, &event);
                term_process(disposition, self, &event);
                rest_process(disposition, self, &event);
            }
        }
    }

    pub fn save(&self, disposition: &Disposition) {

        let path = disposition._path.clone().unwrap();
        for (_, element) in &disposition.elements {
            match element {
                Element::Memory(memory) => {
                    if let Some(state) = &memory._state {
                        let combined_path = combine_paths(&path, &memory.state);
                        match write_memory(combined_path.clone(), state) {
                            Err(e) => print_error!("failed to save memory: {}", e),
                            Ok(()) => print_info!("saved memory to '{}'", &combined_path),
                        }
                    }
                },
                _ => {},
            };
        }

        if self.args.save_no_override {
            match write_disposition(disposition) {
                Err(e) =>  print_error!("failed to save disposition: {}", e),
                Ok(()) => print_info!("saved disposition"),
            }
        } else {
            match write_disposition_override(disposition) {
                Err(e) =>  {
                    print_error!("failed to save disposition override: {}", e);
                },
                Ok(()) => {
                    print_info!("saved disposition override");
                },
            }
        };
    }

    pub fn quit(&self, disposition: &Disposition) {
        if self.args.save_on_exit {
            self.save(disposition);
        }
        print_info!("good bye");
        std::process::exit(0);
    }


    pub fn midi_input(&mut self, id: &Id, name: &str)  {
        match self.try_midi_input(id, name) {
            Ok(port_name) => {
                print_info!("connected {} to port '{}'", id, port_name);
            },
            Err(e) => {
                print_error!("connection {} failed: {}", id, e);
                let message = format!("port '{}' not available", name);
                self.errors.push(ProcessingError{ id: id.clone(), message });
            },
        }
    }

    pub fn try_midi_input(&mut self, id: &Id, name: &str) -> Result<String, Box<dyn Error>> {
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

                    return Ok(port_name);
                }
            }
        }

        Err(format!("no input port '{}'", name).into())
    }

    pub fn midi_output(&mut self, id: &Id, name: &str) -> Option<Output> {
        match self.try_midi_output(name) {
            Ok((port_name, shared_output)) => {
                print_info!("connected {} to port '{}'", id, port_name);
                Some(Output{ shared_output })
            },
            Err(e) => {
                print_error!("connection {} failed: {}", id, e);
                let message = format!("port '{}' not available", name);
                self.errors.push(ProcessingError{ id: id.clone(), message });
                None
            },
        }
    }
    
    fn try_midi_output(&mut self, name: &str) -> Result<(String, Rc<RefCell<SharedOutput>>), Box<dyn Error>> {
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

                return Ok((port_name, output));
            }
        }

        Err(format!("no output port '{}'", name).into())
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
    shared_output: Rc<RefCell<SharedOutput>>,
}
impl Output {
    pub fn send(&mut self, message: &[u8]) -> () {
        self.shared_output.borrow_mut().connection.send(message).unwrap();
    }
}

struct SharedOutput {
    connection: MidiOutputConnection,
}

struct SharedInput {
    _connection: MidiInputConnection<()>,
    ids: Arc<Mutex<Vec<Id>>>,
}
