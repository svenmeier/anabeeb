use std::process::exit;
use crokey::KeyCombination;
use crossbeam_channel::{unbounded, Receiver, Sender};
use crate::disposition::general::{GeneralHandler};
use crate::disposition::rest::RestHandler;
use crate::{print_error, print_info, Args};
use crate::disposition::{Disposition, Element, Id};
use crate::disposition::fluidsynth::FluidsynthHandler;
use crate::disposition::midi::MidiMessage;
use crate::disposition::midi_in::MidiInHandler;
use crate::disposition::midi_out::MidiOutHandler;
use crate::disposition::term::TermHandler;
use crate::io::{combine_paths, write_disposition, write_disposition_override, write_memory};
use crate::setup::setup;

#[derive(Clone)]
pub enum Event {
    KeyPress(Id, u8),
    KeyRelease(Id, u8),
    Activate(Id, bool),
    Change(Id, u32),
    Trigger(Id),
    Modified(Id),
    Error(Id, String),
    BindingStart(Id),
    BindingEnd,
    Save,
    Quit,
    MidiPanic,
    MidiInMessage(Id, MidiMessage),
    MidiOutMessages(Id, String, Vec<MidiMessage>, bool),
    TermKey(KeyCombination),
    RestResponse(Id, Sender<Option<String>>),
}

#[derive(Clone)]
pub struct Events{
    sender: Sender<Event>,
    priority_sender: Sender<Event>,
}
impl Events {
    pub fn send(&self, event: Event) {
        self.sender.send(event).unwrap();
    }

    pub fn send_priority(&self, event: Event) {
        self.priority_sender.send(event).unwrap();
    }
}


pub struct Processor {
    args: Args,
    receiver: Receiver<Event>,
    priority_receiver: Receiver<Event>,
    midi_in_handler: MidiInHandler,
    midi_out_handler: MidiOutHandler,
    rest_handler: RestHandler,
    term_handler: TermHandler,
    fluidsynth_handler: FluidsynthHandler,
    general_handler: GeneralHandler,
    errors: Vec<Event>,
}
impl Processor {
    pub fn new(args: Args) -> Self {
        let (sender, receiver) = unbounded::<Event>();
        let (priority_sender, priority_receiver) = unbounded::<Event>();
        let events = Events{ sender, priority_sender };

        Self {
            args,
            receiver,
            priority_receiver,
            midi_in_handler: MidiInHandler::new(events.clone()),
            midi_out_handler: MidiOutHandler::new(events.clone()),
            rest_handler: RestHandler::new(events.clone()),
            term_handler: TermHandler::new(events.clone()),
            fluidsynth_handler: FluidsynthHandler::new(events.clone()),
            general_handler: GeneralHandler::new(events.clone()),
            errors: Vec::new(),
        }
    }

    pub fn init(&mut self, disposition: &mut Disposition) {

        if self.args.setup {
            match setup(disposition, &self) {
                Ok(()) => print_info!("setup completed"),
                Err(e) => {
                    print_error!("setup failed: {}", e);
                    exit(1);
                },
            }
        }

        self.midi_in_handler.init(disposition);
        self.midi_out_handler.init(disposition);
        self.rest_handler.init(disposition);
        self.term_handler.init(disposition);
        self.fluidsynth_handler.init(disposition);
        self.general_handler.init(disposition);
    }

    pub fn process(&mut self, disposition: &mut Disposition) {
        loop {
            let mut event = self.receiver.recv().unwrap();

            loop {
                match event {
                    Event::Error(_, _) => self.errors.push(event.clone()),
                    Event::Save => {
                        self.save(disposition);
                    },
                    Event::Quit => {
                        self.quit(disposition);
                    },
                    _ => {
                        self.midi_in_handler.process(disposition, &event);
                        self.midi_out_handler.process(disposition, &event);
                        self.rest_handler.process(disposition, &event);
                        self.term_handler.process(disposition, &event);
                        self.fluidsynth_handler.process(disposition, &event);
                        self.general_handler.process(disposition, &event);
                    },
                };

                let priority_event = self.priority_receiver.try_recv();
                if let Ok(priority_event) = priority_event {
                    event = priority_event;
                } else {
                    break;
                }
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
        exit(0);
    }
}

pub fn key_press_dispatch(events: &Events, ids: &Vec<Id>, key: u8) {
    for id in ids {
        events.send_priority(Event::KeyPress(id.clone(), key));
    }
}

pub fn key_release_dispatch(events: &Events, ids: &Vec<Id>, key: u8) {
    for id in ids {
        events.send_priority(Event::KeyRelease(id.clone(), key));
    }
}

pub fn midi_out_dispatch(events: &Events, ids: &Vec<Id>, channel: &String, messages: &Vec<MidiMessage>, release: bool) {
    for id in ids {
        events.send_priority(Event::MidiOutMessages(id.clone(), channel.clone(), messages.clone(), release));
    }
}