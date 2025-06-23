use serde::{Deserialize, Serialize};
use crossbeam_channel::Sender;
use atty::Stream;
use crokey::crossterm::event::Event::Key;
use crokey::crossterm::event::{read, KeyEvent};
use crokey::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crokey::{key, KeyCombination};
use log::{debug, warn};
use schemars::JsonSchema;
use crate::disposition::general::{activate, change, combination_trigger, Disposition, Element, Id};
use crate::disposition::midi_in::midi_panic;
use crate::disposition::midi_out::{midi_activate, midi_change};
use crate::print_info;
use crate::processor::{Event, Processor};

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

pub fn term_init(disposition: &mut Disposition, processor: &Processor) {
    for (id, element) in &disposition.elements {
        match element {
            Element::TermConsole(_) => {
                read_and_send(id.clone(), processor.events.clone());
            },
            _ => {},
        };
    }
}

fn read_and_send(id: Id, events: Sender<Event>) {
    if atty::is(Stream::Stdin) {
        // might hang if no TTY
        match enable_raw_mode() {
            Ok(()) => {
                debug!("enabled raw mode");
            },
            Err(e) => {
                warn!("could not enable raw mode: {}", e)
            }
        }
    } else {
        warn!("no raw mode since no TTY");
    }

    std::thread::spawn(move || {
        loop {
            if let Ok(Key(key_event)) = read() {
                // in windows cmd each key event is sent twice, i.e. for press and release
                if key_event.is_press() {
                    events.send(Event::TermKey(id.clone(), key_event)).unwrap();
                }
            }
        }
    });
}

pub fn term_process(disposition: &mut Disposition, processor: &Processor, event: &Event) {
    if let Event::TermKey(id, key) = event {
        debug!("processing term event: {:?}", key);

        match disposition.elements.get(id) {
            Some(Element::TermConsole(_)) => {
                term_match_bindings(disposition, key.clone());
            },
            _ => {},
        }

        match key.clone().into() {
            key!(shift-p) => {
                midi_panic(disposition);
            },
            key!(ctrl-s) => {
                processor.save(disposition);
            },
            key!(ctrl-q) => {
                if let Err(e) = disable_raw_mode() {
                    warn!("Failed to disable raw mode: {}", e);
                }
                processor.quit(disposition);
            },
            _ => {},
        }
    }
}

fn term_match_bindings(disposition: &mut Disposition, key: KeyEvent) {
    let ids = disposition.elements.keys().cloned().collect::<Vec<Id>>();

    for id in ids {
        match disposition.elements.get_mut(&id) {
            Some(Element::Coupler(coupler)) => {
                if let Some(binding) = &mut coupler.term_binding {
                    if !coupler.active && is_char(key, binding.activate) {
                        activate(disposition, id.clone(), true);
                    } else if coupler.active && is_char(key, binding.deactivate) {
                        activate(disposition, id.clone(), false);
                    }
                }
            },
            Some(Element::Captor(captor)) => {
                if let Some(binding) = &mut captor.term_binding {
                    if !captor.active && is_char(key, binding.activate) {
                        activate(disposition, id.clone(), true);
                    } else if captor.active && is_char(key, binding.deactivate) {
                        activate(disposition, id.clone(), false);
                    }
                }
            },
            Some(Element::Combination(combination)) => {
                if let Some(binding) = &mut combination.term_binding {
                    if is_char(key, binding.trigger) {
                        combination_trigger(disposition, id.clone());
                    }
                }
            },
            Some(Element::Memory(memory)) => {
                if let Some(binding) = &mut memory.term_binding {
                    if is_char(key, binding.decrease) {
                        let value = memory.value.saturating_sub(binding.delta).clamp(memory.min, memory.max);
                        change(disposition, id.clone(), value);
                    } else if is_char(key, binding.increase) {
                        let value = memory.value.saturating_add(binding.delta).clamp(memory.min, memory.max);
                        change(disposition, id.clone(), value);
                    }
                }
            },
            Some(Element::MidiAction(filter)) => {
                if let Some(binding) = &mut filter.term_binding {
                    if !filter.active && is_char(key, binding.activate) {
                        midi_activate(disposition, id.clone(), true);
                    } else if filter.active && is_char(key, binding.deactivate) {
                        midi_activate(disposition, id.clone(), false);
                    }
                }
            },
            Some(Element::MidiRange(range)) => {
                if let Some(binding) = &mut range.term_binding {
                    if is_char(key, binding.decrease) {
                        let value = range.value.saturating_sub(binding.delta).clamp(range.min, range.max);
                        midi_change(disposition, id.clone(), value);
                    } else if is_char(key, binding.increase) {
                        let value = range.value.saturating_add(binding.delta).clamp(range.min, range.max);
                        midi_change(disposition, id.clone(), value);
                    }
                }
            },
            _ => {},
        }
    }
}

fn is_char(event: KeyEvent, combination: KeyCombination) -> bool {
    <KeyEvent as Into<KeyCombination>>::into(event.into()) == combination
}

pub fn term_element_modified(disposition: &mut Disposition, id: Id) {
    match disposition.elements.get(&id) {
        Some(Element::Coupler(coupler)) => {
            print_info!("coupler {} activated {}", id, coupler.active);
        },
        Some(Element::Captor(captor)) => {
            print_info!("captor {} activated {}", id, captor.active);
        },
        Some(Element::Memory(memory)) => {
            print_info!("memory {} changed {}", id, memory.value);
        },
        Some(Element::MidiAction(action)) => {
            print_info!("action {} activated {}", id, action.active);
        },
        Some(Element::MidiRange(filter)) => {
            print_info!("range {} changed {}", id, filter.value);
        },
        _ => {},
    }
}
