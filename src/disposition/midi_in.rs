use crossbeam_channel::Sender;
use std::error::Error;
use log::{error, info, warn};
use midir::MidiInputConnection;
use crate::disposition::general::{activate, combination_trigger, press_key_dispatch, Disposition, Element, Id};
use crate::disposition::midi_out::{midi_activate, midi_change};
use crate::midi::{get_wildcard};
use crate::midi::midi_manager::MidiManager;
use crate::print_info;
use crate::processor::Event;

pub fn midi_in_init(disposition: &mut Disposition, events: &Sender<Event>, midi_manager: &MidiManager) -> Result<(), Box<dyn Error>> {
    for (id, element) in &mut disposition.elements {
        match element {
            Element::MidiKeyboard(keyboard) => {
                keyboard._input_connection = Some(connect_input(midi_manager, id, &keyboard.port, events)?);
            }
            Element::MidiConsole(console) => {
                console._input_connection = Some(connect_input(midi_manager, id, &console.port, events)?);
            }
            _ => {},
        };
    }

    Ok(())
}

fn connect_input(midi_manager: &MidiManager, id: &Id, port: &String, events: &Sender<Event>) -> Result<MidiInputConnection<()>, Box<dyn Error>> {
    let id_clone = id.clone();
    let events_clone = events.clone();

    let (name, connection) = midi_manager.connect_input(port, move |msg| {
        events_clone
            .send(Event::MidiIn(id_clone.clone(), msg))
            .unwrap_or_else(|e| error!("failed to process midi event: {}", e));
    })?;
    
    info!("connected {} to '{}'", id, name);
    Ok(connection)
}

pub fn midi_in_process(disposition: &mut Disposition, event: &Event) {
    if let Event::MidiIn(id, message) = event {
        match disposition.elements.get_mut(&id) {
            Some(Element::MidiKeyboard(keyboard)) => {
                let key_down = keyboard.midi_binding.key_down.clone();
                let key_up = keyboard.midi_binding.key_up.clone();
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
                match &mut coupler.midi_binding {
                    Some(binding) => {
                        if binding.activate == message {
                            activate(disposition, id, true);
                        } else if binding.deactivate == message {
                            activate(disposition, id, false);
                        }
                    }
                    _ => {},
                }
            },
            Some(Element::Captor(captor)) => {
                match &mut captor.midi_binding {
                    Some(binding) => {
                        if binding.activate == message {
                            activate(disposition, id, true);
                        } else if binding.deactivate == message {
                            activate(disposition, id, false);
                        }
                    }
                    _ => {},
                }
            },
            Some(Element::Combination(combination)) => {
                match &mut combination.midi_binding {
                    Some(binding) => {
                        if binding.trigger == message {
                            combination_trigger(disposition, id);
                        }
                    }
                    _ => {},
                }
            },
            Some(Element::MidiAction(filter)) => {
                match &mut filter.midi_binding {
                    Some(binding) => {
                        if binding.activate == message {
                            midi_activate(disposition, id, true);
                        } else if binding.deactivate == message {
                            midi_activate(disposition, id, false);
                        }
                    }
                    _ => {},
                }
            },
            Some(Element::MidiRange(filter)) => {
                match &mut filter.midi_binding {
                    Some(binding) => {
                        match get_wildcard(&message, &binding.change) {
                            Some((_, value)) => {
                                let range = filter.max.saturating_sub(filter.min);
                                let value = filter.min.saturating_add((value as u32).saturating_mul(range) / 127);

                                midi_change(disposition, id, value);
                            },
                            _ => {}
                        }
                    },
                    _ => {},
                }
            },
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
            }
            _ => {}
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
        _ => {}
    }
}
