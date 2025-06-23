use log::{debug, error, warn};
use crate::disposition::fluidsynth::fluidsynth_send_messages;
use crate::disposition::general::{Disposition, Element, Id};
use crate::disposition::midi::{MidiRange, MidiAction};
use crate::disposition::rest::rest_element_modified;
use crate::disposition::term::{term_element_modified};
use crate::midi::{set_midi_channel, set_wildcard};
use crate::processor::{Processor};

pub fn midi_out_init(disposition: &mut Disposition, processor: &mut Processor) {
    for (id, element) in &mut disposition.elements {
        match element {
            Element::MidiSound(sound) => {
                if let Some(port) = &sound.port {
                    sound._output = processor.midi_output(id, port);
                }
            },
            Element::MidiConsole(console) => {
                if let Some(port) = &console.port {
                    console._output = processor.midi_output(id, port);
                }
            },
            _ => {},
        };
    }
}

fn send_messages_dispatch(disposition: &mut Disposition, ids: Vec<Id>, channel: String, release: bool, messages: Vec<Vec<u8>>) {
    for id in ids {
        match disposition.elements.get_mut(&id) {
            Some(Element::MidiRange(_)) => {
                send_messages(disposition, id, channel.clone(), release, messages.clone());
            },
            Some(Element::MidiAction(_)) => {
                send_messages(disposition, id, channel.clone(), release, messages.clone());
            },
            Some(Element::MidiSound(_)) => {
                send_messages(disposition, id, channel.clone(), release, messages.clone());
            },
            Some(Element::FluidsynthSound(_)) => {
                fluidsynth_send_messages(disposition, id, channel.clone(), release, messages.clone());
            },
            None => {
                warn!("unknown id {}", id);
            },
            _ => {},
        };
    }
}

fn send_messages(disposition: &mut Disposition, id: Id, channel: String, release: bool, mut messages: Vec<Vec<u8>>) {
    match disposition.elements.get_mut(&id) {
        Some(Element::MidiSound(sound)) => {
            let (channel_number, new) = sound._channels.acquire(channel.as_str());
            if channel_number < 16 {
                if let Some(ref mut output) = sound._output {
                    for message in messages.iter() {
                        debug!("midi sound {} send '{}' {} '{:?}'", id, channel, channel_number, message);
                        let channel_message = set_midi_channel(message, channel_number);
                        output.send(&channel_message);
                    }
                }
            } else {
                if new {
                    error!("no channel available in {} for '{}'", id, channel);
                }
            }

            if release {
                sound._channels.release(channel.as_str());
            }
        },
        Some(Element::MidiRange(filter)) => {
            if release {
                filter._channels.remove(&channel);
            } else {
                if filter._channels.insert(channel.clone()) {
                    for message in midi_range_messages(filter) {
                        messages.push(message);
                    }
                }
            }

            let references = filter.references.clone();
            send_messages_dispatch(disposition, references, channel, release, messages);
        },
        Some(Element::MidiAction(action)) => {
            if release {
                action._channels.remove(&channel);
            } else {
                if action._channels.insert(channel.clone()) {
                    for message in midi_action_messages(action) {
                        messages.push(message);
                    }
                }
            }

            let references = action.references.clone();
            send_messages_dispatch(disposition, references, channel, release, messages);
        },
        _ => {},
    }
}

pub fn midi_change(disposition: &mut Disposition, id: Id, mut value: u32) {
    let modified = match disposition.elements.get_mut(&id) {
        Some(Element::MidiRange(range)) => {
            value = value.clamp(range.min, range.max);
            if value != range.value {
                range.value = value;

                let references = range.references.clone();
                let messages = midi_range_messages(range);
                for channel in range._channels.clone().iter() {
                    send_messages_dispatch(disposition, references.clone(), channel.clone(), false, messages.clone());
                }

                true
            } else {
                false
            }
        },
        _ => false,
    };

    if modified {
        midi_element_modified(disposition, id.clone());
        rest_element_modified(disposition, id.clone());
        term_element_modified(disposition, id.clone());
    }
}

fn midi_range_messages(filter: &mut MidiRange) -> Vec<Vec<u8>> {
    filter.change.iter().map(| message | {
        set_wildcard(message, filter.value as u8)
    }).collect()
}

pub fn midi_activate(disposition: &mut Disposition, id: Id, active: bool) {
    let modified = match disposition.elements.get_mut(&id) {
        Some(Element::MidiAction(action)) => {
            if active != action.active {
                action.active = active;

                let references = action.references.clone();
                let messages = midi_action_messages(action);
                for channel in action._channels.clone().iter() {
                    send_messages_dispatch(disposition, references.clone(), channel.clone(), false, messages.clone());
                }

                true
            } else {
                false
            }
        },
        _ => false,
    };

    if modified {
        midi_element_modified(disposition, id.clone());
        rest_element_modified(disposition, id.clone());
        term_element_modified(disposition, id.clone());
    }
}

fn midi_action_messages(filter: &mut MidiAction) -> Vec<Vec<u8>> {
    if  filter.active {
        filter.engage.clone()
    } else {
        filter.disengage.clone()
    }
}

pub fn midi_register_press_key(disposition: &mut Disposition, id: Id, key: u8, down: bool) {
    match disposition.elements.get_mut(&id) {
        Some(Element::MidiRegister(register)) => {
            debug!("midi register {} key {} {}", id, key, down);
            if down { register._pressed_key_count += 1 } else { register._pressed_key_count -= 1 };

            let pressed_keys = register._pressed_key_count;

            let message = if down { vec![144, key, 127] } else { vec![128, key, 0] };
            let messages = match (down, pressed_keys) {
                (true, 1) => {
                    let mut messages = register.acquire.clone();
                    messages.push(message);
                    messages
                },
                (false, 0) => {
                    let mut messages = register.release.clone();
                    messages.push(message);
                    messages
                },
                _ => {
                    vec![message]
                },
            };

            let references = register.references.clone();
            send_messages_dispatch(disposition, references, id.0, pressed_keys == 0, messages);
        },
        _ => {},
    }
}

pub fn midi_element_modified(disposition: &mut Disposition, id: Id) {
    match disposition.elements.get(&id) {
        Some(Element::Coupler(coupler)) => {
            if let Some(binding) = &coupler.midi_binding {
                let message = if coupler.active { binding.activated.clone() } else { binding.deactivated.clone() };
                midi_console_send(disposition, id.clone(), message);
            }
        },
        Some(Element::Captor(captor)) => {
            if let Some(binding) = &captor.midi_binding {
                let message = if captor.active { binding.activated.clone() } else { binding.deactivated.clone() };
                midi_console_send(disposition, id.clone(), message)
            }
        },
        Some(Element::MidiRange(range)) => {
            if let Some(binding) = &range.midi_binding {
                let delta = range.max.saturating_sub(range.min);
                let value = if delta != 0 {
                    (range.value.saturating_sub(range.min).saturating_mul(127) / delta) as u8
                } else {
                    0
                };

                let message = set_wildcard(&*binding.changed, value);
                midi_console_send(disposition, id.clone(), message)
            }
        },
        _ => {},
    }
}

fn midi_console_send(disposition: &mut Disposition, reference: Id, message: Vec<u8>) {
    if message.is_empty() {
        return;
    }
    
    for (id, element) in &mut disposition.elements {
        match element {
            Element::MidiConsole(console) => {
                if console.references.contains(&reference) {
                    if let Some(ref mut output) = console._output {
                        debug!("midi console {} send {} '{:?}'", id, reference, message);
                        output.send(message.as_slice());
                    }
                }
            },
            _ => {},
        }
    };
}