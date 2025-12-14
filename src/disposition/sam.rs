use std::collections::HashMap;
use std::time::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::disposition::{Disposition, Element, Id};
use crate::disposition::midi::MidiSwitchBinding;
use crate::processor::{Event, Events};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MagnetReleaser {

    #[serde(default="default_delay")]
    pub delay: u64,
}

pub struct SamsHandler {
    events: Events,
    current_generation: u64,
    latest_element_generation: HashMap<Id, u64>,
}
impl SamsHandler {
    pub fn new(events: Events) -> Self {
        Self { events, current_generation: 0, latest_element_generation: HashMap::new() }
    }

    pub fn init(&mut self, _: &mut Disposition) {
    }

    pub fn process(&mut self, disposition: &mut Disposition, event: &Event) {
        match event {
            Event::Modified(id, ) => {
                let element = disposition.elements.get(&id);

                match element {
                    Some(Element::Coupler(coupler)) => {
                        self.send_console_output(disposition, id.clone(), coupler.magnet_release_binding.clone(), coupler.active);
                    },
                    Some(Element::Captor(captor)) => {
                        self.send_console_output(disposition, id.clone(), captor.magnet_release_binding.clone(), captor.active);
                    },
                    Some(Element::MidiAction(action)) => {
                        self.send_console_output(disposition, id.clone(), action.magnet_release_binding.clone(), action.active);
                    },
                    _ => {}
                };
            },
            Event::MagnetRelease(id, generation, message) => {
                if self.latest_element_generation.get(id)
                    .is_some_and(|element_generation| element_generation == generation)
                {
                    self.events.prepend(
                        Event::MidiConsoleOutput(id.clone(), message.clone())
                    );
                }
            }
            _ => {},
        }
    }

    fn send_console_output(&mut self, disposition: &mut Disposition, id: Id, binding: Option<MidiSwitchBinding>, active: bool) {
        if let Some(binding) = binding {

            for (_, element) in &mut disposition.elements {
                match element {
                    Element::MagnetReleaser(magnet) => {
                        let delay = Duration::from_millis(magnet.delay);

                        self.current_generation += 1;
                        self.latest_element_generation.insert(id.clone(), self.current_generation);

                        if active {
                            self.events.prepend(Event::MidiConsoleOutput(id.clone(), binding.deactivate.clone()));
                            self.events.delay(Event::MagnetRelease(id.clone(), self.current_generation.clone(), binding.activate.clone()), delay);
                        } else {
                            self.events.prepend(Event::MidiConsoleOutput(id.clone(), binding.activate.clone()));
                            self.events.delay(Event::MagnetRelease(id.clone(), self.current_generation.clone(), binding.deactivate.clone()), delay);
                        }
                    },
                    _ => {}
                };
            }
        }
    }
}

fn default_delay() -> u64 { 100 }