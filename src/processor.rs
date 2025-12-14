use std::collections::BinaryHeap;
use std::process::exit;
use std::time::{Duration, Instant};
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
use crate::disposition::sam::{SamsHandler};
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
    MidiInput(Id, MidiMessage),
    MidiConsoleOutput(Id, MidiMessage),
    MidiSoundOutput(Id, Vec<MidiMessage>, String, bool),
    MagnetRelease(Id, u64, MidiMessage),
    TermKey(KeyCombination),
    RestResponse(Id, Sender<Option<String>>),
}

struct PendingEvent {
    when: Instant,
    event: Event,
}
impl Eq for PendingEvent {}
impl PartialEq for PendingEvent {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}
impl Ord for PendingEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse so earliest (smallest Instant) is "greater"
        other.when.cmp(&self.when)
    }
}
impl PartialOrd for PendingEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct Events{
    sender: Sender<PendingEvent>,
}

impl Events {
    pub fn append(&self, event: Event) {
        self.sender.send(PendingEvent { when: Instant::now(), event}).unwrap();
    }

    pub fn prepend(&self, event: Event) {
        self.sender.send(PendingEvent { when: Instant::now() - Duration::from_secs(60000), event}).unwrap();
    }

    pub fn delay(&self, event: Event, delay: Duration) {
        self.sender.send(PendingEvent { when: Instant::now() + delay, event}).unwrap();
    }
}

pub struct Processor {
    args: Args,
    receiver: Receiver<PendingEvent>,
    pending_events: BinaryHeap<PendingEvent>,
    midi_in_handler: MidiInHandler,
    midi_out_handler: MidiOutHandler,
    sams_handler: SamsHandler,
    rest_handler: RestHandler,
    term_handler: TermHandler,
    fluidsynth_handler: FluidsynthHandler,
    general_handler: GeneralHandler,
    errors: Vec<Event>,
}
impl Processor {
    pub fn new(args: Args) -> Self {
        let (sender, receiver) = unbounded::<PendingEvent>();
        let events = Events{ sender };

        Self {
            args,
            receiver,
            pending_events: BinaryHeap::new(),
            midi_in_handler: MidiInHandler::new(events.clone()),
            midi_out_handler: MidiOutHandler::new(events.clone()),
            sams_handler: SamsHandler::new(events.clone()),
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
        self.sams_handler.init(disposition);
        self.rest_handler.init(disposition);
        self.term_handler.init(disposition);
        self.fluidsynth_handler.init(disposition);
        self.general_handler.init(disposition);
    }

    pub fn process(&mut self, disposition: &mut Disposition) {
        loop {
            // drain all received events
            while let Ok(ev) = self.receiver.try_recv() {
                self.pending_events.push(ev);
            }

            // timeout until next pending
            let timeout = match self.pending_events.peek() {
                Some(next) => {
                    let now = Instant::now();
                    if next.when > now {
                        next.when - now
                    } else {
                        Duration::from_secs(0) // already due
                    }
                }
                None => Duration::MAX, // wait indefinitely for new events
            };
            match self.receiver.recv_timeout(timeout) {
                Ok(ev) => {
                    // new event interrupted wait
                    self.pending_events.push(ev);
                    continue;
                }
                Err(_) => {
                }
            }

            let event = self.pending_events.pop().unwrap().event;
            match event {
                Event::Error(_, _) => self.errors.push(event.clone()),
                Event::Save => self.save(disposition),
                Event::Quit => self.quit(disposition),
                _ => {
                    self.midi_in_handler.process(disposition, &event);
                    self.midi_out_handler.process(disposition, &event);
                    self.sams_handler.process(disposition, &event);
                    self.rest_handler.process(disposition, &event);
                    self.term_handler.process(disposition, &event);
                    self.fluidsynth_handler.process(disposition, &event);
                    self.general_handler.process(disposition, &event);
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

    pub fn quit(&mut self, disposition: &mut Disposition) {
        if self.args.save_on_exit {
            self.save(disposition);
        }
        print_info!("good bye");
        exit(0);
    }
}

pub fn key_press_dispatch(events: &Events, ids: &Vec<Id>, key: u8) {
    for id in ids {
        events.prepend(Event::KeyPress(id.clone(), key));
    }
}

pub fn key_release_dispatch(events: &Events, ids: &Vec<Id>, key: u8) {
    for id in ids {
        events.prepend(Event::KeyRelease(id.clone(), key));
    }
}

pub fn midi_sound_dispatch(events: &Events, ids: &Vec<Id>, messages: &Vec<MidiMessage>, channel: &String, release: bool) {
    for id in ids {
        events.prepend(Event::MidiSoundOutput(id.clone(), messages.clone(), channel.clone(), release));
    }
}