use std::error::Error;
use crokey::crossterm::event::KeyEvent;
use crossbeam_channel::{unbounded, Receiver, Sender};
use crate::disposition::general::{general_init, Disposition, Id};
use crate::disposition::fluidsynth::fluidsynth_init;
use crate::disposition::midi_in::{midi_in_init, midi_in_process};
use crate::disposition::midi_out::midi_out_init;
use crate::disposition::rest::{rest_init, rest_process, Command};
use crate::disposition::term::{term_init, term_process};
use crate::midi::midi_manager::MidiManager;

pub enum Event {
    MidiIn(Id, Vec<u8>),
    TermKey(Id, KeyEvent),
    Rest(Id, Command, Sender<Option<String>>),
}

pub struct Processor {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
    midi_manager: MidiManager,
}

impl Processor {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded::<Event>();

        Self {
            sender,
            receiver,
            midi_manager: MidiManager::new()
        }
    }

    pub fn init(&self, disposition: &mut Disposition) -> Result<(), Box<dyn Error>> {
        
        self.midi_manager.log_midi()?;

        general_init(disposition, &self.sender)?;
        midi_in_init(disposition, &self.sender, &self.midi_manager)?;
        midi_out_init(disposition, &self.sender, &self.midi_manager)?;
        rest_init(disposition, &self.sender)?;
        term_init(disposition, &self.sender)?;
        fluidsynth_init(disposition, &self.sender)?;
        
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
}