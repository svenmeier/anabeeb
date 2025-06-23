use log::{info, warn};
use crate::disposition::general::{activate, change, combination_trigger, press_key_dispatch, Disposition, Element, Id};
use crate::disposition::midi_out::{midi_activate, midi_change};
use crate::midi::{get_wildcard};
use crate::print_info;
use crate::processor::{Event, Processor};

pub fn midi_in_init(disposition: &mut Disposition, processor: &mut Processor) {
    for (id, element) in &mut disposition.elements {
        match element {
            Element::MidiKeyboard(keyboard) => {
                if let Some(port) = &keyboard.port {
                    processor.midi_input(id, port);
                }
            },
            Element::MidiConsole(console) => {
                if let Some(port) = &console.port {
                    processor.midi_input(id, port);
                }
            },
            _ => {},
        };
    }
}

pub fn midi_in_process(disposition: &mut Disposition, _: &Processor, event: &Event) {
    if let Event::MidiIn(id, message) = event {
        match disposition.elements.get_mut(&id) {
            Some(Element::MidiKeyboard(keyboard)) => {
                if let Some(binding) = &keyboard.midi_binding {
                    let key_down = binding.key_down.clone();
                    let key_up = binding.key_up.clone();
                    let references = keyboard.references.clone();

                    if let Some((_, key)) = get_wildcard(&message, &key_down) {
                        if keyboard._pressed_keys.insert(key) {
                            press_key_dispatch(disposition, references.clone(), key, true);
                        } else {
                            warn!("midi keyboard key {} already pressed", key)
                        }
                    } else if let Some((_, key)) = get_wildcard(&message, &key_up) {
                        if keyboard._pressed_keys.remove(&key) {
                            press_key_dispatch(disposition, references.clone(), key, false);
                        } else {
                            warn!("midi keyboard key {} was not pressed", key)
                        }
                    }
                }
            },
            Some(Element::MidiConsole(console)) => {
                let references = console.references.clone();
                midi_match_bindings(disposition, references, message.clone());
            },
            _ => {},
        }
    }
}

fn midi_match_bindings(disposition: &mut Disposition, ids: Vec<Id>, message: Vec<u8>) {
    for id in ids {
        match disposition.elements.get_mut(&id) {
            Some(Element::Coupler(coupler)) => {
                if let Some(binding) = &mut coupler.midi_binding {
                    if binding.activate == message {
                        activate(disposition, id, true);
                    } else if binding.deactivate == message {
                        activate(disposition, id, false);
                    }
                }
            },
            Some(Element::Captor(captor)) => {
                if let Some(binding) = &mut captor.midi_binding {
                    if binding.activate == message {
                        activate(disposition, id, true);
                    } else if binding.deactivate == message {
                        activate(disposition, id, false);
                    }
                }
            },
            Some(Element::Combination(combination)) => {
                if let Some(binding) = &mut combination.midi_binding {
                    if binding.trigger == message {
                        combination_trigger(disposition, id);
                    }
                }
            },
            Some(Element::Memory(memory)) => {
                if let Some(binding) = &mut memory.midi_binding {
                    if let Some((_, value)) = get_wildcard(&message, &binding.change) {
                        let delta = memory.max.saturating_sub(memory.min);
                        let value = memory.min.saturating_add((value as u32).saturating_mul(delta) / 127);

                        change(disposition, id, value);
                    }
                }
            },
            Some(Element::MidiAction(action)) => {
                if let Some(binding) = &mut action.midi_binding {
                    if binding.activate == message {
                        midi_activate(disposition, id, true);
                    } else if binding.deactivate == message {
                        midi_activate(disposition, id, false);
                    }
                }
            }
            Some(Element::MidiRange(range)) => {
                if let Some(binding) = &mut range.midi_binding {
                    if let Some((_, value)) = get_wildcard(&message, &binding.change) {
                        let delta = range.max.saturating_sub(range.min);
                        let value = range.min.saturating_add((value as u32).saturating_mul(delta) / 127);

                        midi_change(disposition, id, value);
                    }
                }
            }
            None => {
                warn!("invalid id {}", id);
            },
            _ => {},
        }
    }
}


pub fn midi_panic(disposition: &mut Disposition) {
    print_info!("midi panic");
    for id in disposition.elements.keys().cloned().collect::<Vec<Id>>() {
        match disposition.elements.get_mut(&id) {
            Some(Element::MidiKeyboard(_)) => {
                midi_keyboard_panic(disposition, id);
            },
            _ => {},
        };
    }
}

fn midi_keyboard_panic(disposition: &mut Disposition, id: Id) {
    match disposition.elements.get(&id) {
        Some(Element::MidiKeyboard(keyboard)) => {
            let keys = keyboard._pressed_keys.clone();
            let references = keyboard.references.clone();
            for key in keys {
                info!("midi keyboard panic {} key {}", id, key);
                press_key_dispatch(disposition, references.clone(), key, false);
            }
        },
        _ => {},
    }
}
