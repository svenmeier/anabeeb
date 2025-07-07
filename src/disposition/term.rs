use serde::{Deserialize, Serialize};
use crossbeam_channel::Sender;
use crokey::crossterm::event::Event::Key;
use crokey::crossterm::event::{read};
use crokey::{key, KeyCombination};
use log::{debug};
use schemars::JsonSchema;
use crate::disposition::general::{Disposition, Element, Id};
use crate::disposition::midi_in::midi_panic;
use crate::{print_error, print_info};
use crate::console::{raw_mode, read_choice};
use crate::processor::Event;

pub struct TermHandler {
    events: Sender<Event>,
}
impl TermHandler {
    pub fn new(events: Sender<Event>) -> Self {
        Self {
            events,
        }
    }

    pub fn init(&mut self, disposition: &mut Disposition) {
        for (_, element) in &disposition.elements {
            match element {
                Element::TermConsole(_) => {
                    print_info!("ctrl-b binds an element");
                    print_info!("ctrl-p releases all keys (MIDI panic)");
                    print_info!("ctrl-s saves the disposition");
                    print_info!("ctrl-q quits");

                    let ids = disposition.elements.keys().cloned().collect();
                    read_and_send(self.events.clone(), ids);
                },
                _ => {},
            };
        }
    }
    
    pub fn process(&mut self, disposition: &mut Disposition, event: &Event) {
        match event {
            Event::TermKey(key) => {
                debug!("processing term event: {:?}", key);

                if let Some(binding) = &mut disposition._binding {
                    binding.keys.push(key.clone());
                } else {
                    self.match_bindings(disposition, key);
                }

                match key {
                    key!(ctrl-p) => {
                        midi_panic(disposition);
                    },
                    key!(ctrl-s) => {
                        self.events.send(Event::Save).unwrap();
                    },
                    key!(ctrl-q) => {
                        raw_mode(false);
                        self.events.send(Event::Quit).unwrap();
                    },
                    _ => {},
                }
            },
            Event::BindingEnd => {
                binding_end(disposition);
            },
            _ => {},
        }
    }

    fn match_bindings(&self, disposition: &mut Disposition, key: &KeyCombination) {
        let ids = disposition.elements.keys().cloned().collect::<Vec<Id>>();
        
        for id in ids {
            match disposition.elements.get_mut(&id) {
                Some(Element::Coupler(coupler)) => {
                    self.match_switch_binding(id, coupler.active, key, &coupler.term_binding);
                },
                Some(Element::Captor(captor)) => {
                    self.match_switch_binding(id, captor.active, key, &captor.term_binding);
                },
                Some(Element::MidiAction(action)) => {
                    self.match_switch_binding(id, action.active, key, &action.term_binding);
                },
                Some(Element::MidiRange(range)) => {
                    self.match_continuous_binding(id, range.value, range.min, range.max, key, &range.term_binding);
                },
                Some(Element::Memory(memory)) => {
                    self.match_continuous_binding(id, memory.value, memory.min, memory.max, key, &memory.term_binding);
                },
                Some(Element::Combination(combination)) => {
                    if let Some(binding) = &mut combination.term_binding {
                        if *key == binding.trigger {
                            self.events.send(Event::Trigger(id.clone())).unwrap();
                        }
                    }
                },
                _ => {},
            }
        }
    }

    fn match_switch_binding(&self, id: Id, active: bool, key: &KeyCombination, binding: &Option<TermSwitchBinding>) {
        if let Some(binding) = binding {
            if !active && *key == binding.activate {
                self.events.send(Event::Activate(id.clone(), true)).unwrap();
            } else if active && *key == binding.deactivate {
                self.events.send(Event::Activate(id.clone(), false)).unwrap();
            }
        }
    }

    fn match_continuous_binding(&self, id: Id, value: u32, min: u32, max: u32, key: &KeyCombination, binding: &Option<TermContinuousBinding>) {
        if let Some(binding) = binding {
            if *key == binding.decrease {
                let value = value.saturating_sub(binding.delta).clamp(min, max);
                self.events.send(Event::Change(id.clone(), value)).unwrap();
            } else if *key == binding.increase {
                let value = value.saturating_add(binding.delta).clamp(min, max);
                self.events.send(Event::Change(id.clone(), value)).unwrap();
            }
        }
    }
}

fn binding_end(disposition: &mut Disposition) {
    if let Some(binding) = &disposition._binding {
        if let Some(element) = disposition.elements.get_mut(&binding.id) {
            match element {
                Element::Coupler(e) => {
                    e.term_binding = switch_binding(&binding.keys).or(e.term_binding.clone());
                },
                Element::Combination(e) => {
                    e.term_binding = momentary_binding(&binding.keys).or(e.term_binding.clone());
                },
                Element::Captor(e) => {
                    e.term_binding = switch_binding(&binding.keys).or(e.term_binding.clone());
                },
                Element::Memory(e) => {
                    e.term_binding = continuous_binding(&binding.keys).or(e.term_binding.clone());
                },
                Element::MidiAction(e) => {
                    e.term_binding = switch_binding(&binding.keys).or(e.term_binding.clone());
                },
                Element::MidiRange(e) => {
                    e.term_binding = continuous_binding(&binding.keys).or(e.term_binding.clone());
                },
                _ => {},
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct TermSwitchBinding {
    #[schemars(with="String")]
    pub activate: KeyCombination,
    #[schemars(with="String")]
    pub deactivate: KeyCombination,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct TermContinuousBinding {
    #[schemars(with="String")]
    pub increase: KeyCombination,
    #[schemars(with="String")]
    pub decrease: KeyCombination,
    pub delta: u32,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct TermMomentaryBinding {
    #[schemars(with="String")]
    pub trigger: KeyCombination,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TermConsole {
}


fn read_and_send(events: Sender<Event>, ids: Vec<Id>) {
    std::thread::spawn(move || {
        raw_mode(true);

        loop {
            if let Ok(Key(key_event)) = read() {
                // in windows cmd each key event is sent twice, i.e. for press and release
                if key_event.is_press() {
                    let key_combination = key_event.clone().into();

                    if key_combination == key!(ctrl-b) {
                        raw_mode(false);
                        bind_element(events.clone(), ids.clone()).unwrap();
                        raw_mode(true);
                    } else {
                        events.send(Event::TermKey(key_combination)).unwrap();
                    }
                }
            }
        }
    });
}

fn bind_element(events: Sender<Event>, ids: Vec<Id>) -> Result<(), Box<dyn std::error::Error>> {
    events.send(Event::BindingEnd)?;

    print_info!("Choose an element (use TAB for completion)");

    let chosen = read_choice("Enter an Id", ids.iter().map(|id| id.to_string()).collect());
    if !chosen.is_empty() {
        let id: Id = chosen.into();
        if ids.contains(&id) {
            events.send(Event::BindingStart(id.clone()))?;
        } else {
            print_error!("Unknown element ${}", id)
        }
    }
    
    Ok(())
}

fn switch_binding(combinations: &Vec<KeyCombination>) -> Option<TermSwitchBinding> {
    let len = combinations.len();
    if len >= 2 {
        return Some(TermSwitchBinding{
            activate: combinations[len - 2].clone(),
            deactivate: combinations[len - 1].clone(),
        });
    }
    None
}

fn continuous_binding(combinations: &Vec<KeyCombination>) -> Option<TermContinuousBinding> {
    let len = combinations.len();
    if len >= 2 {
        return Some(TermContinuousBinding{
            increase: combinations[len - 2].clone(),
            decrease: combinations[len - 1].clone(),
            delta: 1,
        });
    }
   None
}

fn momentary_binding(combinations: &Vec<KeyCombination>) -> Option<TermMomentaryBinding> {
    let len = combinations.len();
    if len >= 1 {
        return Some(TermMomentaryBinding{
            trigger: combinations[len - 1].clone(),
        });
    }
    None
}
