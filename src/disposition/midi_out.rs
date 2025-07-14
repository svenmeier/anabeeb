use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use crossbeam_channel::Sender;
use log::{debug, error, info, warn};
use midir::{MidiOutput, MidiOutputConnection};
use crate::disposition::fluidsynth::fluidsynth_send_messages;
use crate::disposition::general::{Disposition, Element, Id};
use crate::disposition::midi::{MidiRange, MidiAction, MidiSwitchBinding, MidiContinuousBinding, to_regex};
use crate::midi::{get_output_ports, set_midi_channel, set_wildcard};
use crate::{print_error, print_info};
use crate::processor::Event;

pub struct MidiOutHandler {
    events: Sender<Event>,
    outputs: HashMap<String, Rc<RefCell<SharedOutput>>>,
}
impl MidiOutHandler {
    pub fn new(events: Sender<Event>) -> Self {
        Self {
            events,
            outputs: HashMap::new(),
        }
    }

    pub fn init(&mut self, disposition: &mut Disposition) {
        log_midi_ports();
        
        for (id, element) in &mut disposition.elements {
            match element {
                Element::MidiSound(sound) => {
                    if let Some(port) = &sound.port {
                        sound._output = self.midi_output(id, port);
                    }
                },
                Element::MidiConsole(console) => {
                    if let Some(port) = &console.port {
                        console._output = self.midi_output(id, port);
                    }
                },
                _ => {},
            };
        }
    }

    pub fn process(&mut self, disposition: &mut Disposition, event: &Event) {
        match event {
            Event::Activate(id, active) => {
                self.activate(disposition, id.clone(), active.clone());
            },
            Event::Change(id, value) => {
                self.change(disposition, id.clone(), value.clone());
            },
            Event::Modified(id) => {
                let element = disposition.elements.get(&id);

                match element {
                    Some(Element::Coupler(coupler)) => {
                        self.send_switch_binding(id, disposition, coupler.active, &coupler.midi_out_binding.clone());
                    },
                    Some(Element::Captor(captor)) => {
                        self.send_switch_binding(id, disposition, captor.active, &captor.midi_out_binding.clone());
                    },
                    Some(Element::MidiAction(action)) => {
                        self.send_switch_binding(id, disposition, action.active, &action.midi_out_binding.clone());
                    },
                    Some(Element::MidiRange(range)) => {
                        self.send_continuous_binding(id, disposition, range.value, range.min, range.max, &range.midi_out_binding.clone());
                    },
                    Some(Element::Memory(memory)) => {
                        self.send_continuous_binding(id, disposition, memory.value, memory.min, memory.max, &memory.midi_out_binding.clone());
                    },
                    Some(Element::Roller(roller)) => {
                        self.send_continuous_binding(id, disposition, roller.value, roller.min, roller.max, &roller.midi_out_binding.clone());
                    }
                    _ => {},
                }
            },
            _ => {},
        }
    }

    fn send_switch_binding(&self, id: &Id, disposition: &mut Disposition, active: bool, binding: &Option<MidiSwitchBinding>) {
        if let Some(binding) = binding {
            let message = if active { binding.activate.clone() } else { binding.deactivate.clone() };
            midi_console_send(disposition, id.clone(), message);
        }
    }

    fn send_continuous_binding(&self, id: &Id, disposition: &mut Disposition, value: u32, min: u32, max: u32, binding: &Option<MidiContinuousBinding>) {
        if let Some(binding) = binding {
            let delta = max.saturating_sub(min);
            let value = if delta != 0 {
                (value.saturating_sub(min).saturating_mul(127) / delta) as u8
            } else {
                0
            };

            let message = set_wildcard(&*binding.change, value);
            midi_console_send(disposition, id.clone(), message)
        }
    }

    fn change(&self, disposition: &mut Disposition, id: Id, mut value: u32) {
        let modified = match disposition.elements.get_mut(&id) {
            Some(Element::MidiRange(range)) => {
                value = value.clamp(range.min, range.max);
                if value != range.value {
                    range.value = value;
                    print_info!("range ${} changed {}", id, value);

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
            self.events.send(Event::Modified(id.clone())).unwrap();
        }
    }

    fn activate(&self, disposition: &mut Disposition, id: Id, active: bool) {
        let modified = match disposition.elements.get_mut(&id) {
            Some(Element::MidiAction(action)) => {
                if active != action.active {
                    action.active = active;
                    print_info!("action ${} activated {}", id, active);

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
            self.events.send(Event::Modified(id.clone())).unwrap();
        }
    }

    fn midi_output(&mut self, id: &Id, name: &str) -> Option<Rc<RefCell<SharedOutput>>> {
        match self.try_midi_output(name) {
            Ok((port_name, shared_output)) => {
                print_info!("connected ${} to port '{}'", id, port_name);
                Some(shared_output)
            },
            Err(e) => {
                print_error!("connection ${} failed: {}", id, e);
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

pub struct SharedOutput {
    connection: MidiOutputConnection,
}

fn send_messages_dispatch(disposition: &mut Disposition, ids: Vec<Id>, channel: String, release: bool, messages: Vec<Vec<u8>>) {
    for id in ids {
        match disposition.elements.get_mut(&id) {
            Some(Element::MidiRange(_) | Element::MidiAction(_) | Element::MidiSound(_)) => {
                send_messages(disposition, id, channel.clone(), release, messages.clone());
            },
            Some(Element::FluidsynthSound(_)) => {
                fluidsynth_send_messages(disposition, id, channel.clone(), release, messages.clone());
            },
            None => {
                warn!("unknown id ${}", id);
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
                        debug!("midi sound ${} send '{}' {} '{:?}'", id, channel, channel_number, message);
                        let channel_message = set_midi_channel(message, channel_number);
                        output.borrow_mut().connection.send(&channel_message).unwrap();
                    }
                }
            } else {
                if new {
                    error!("no channel available in ${} for '{}'", id, channel);
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

fn midi_range_messages(filter: &mut MidiRange) -> Vec<Vec<u8>> {
    filter.change.iter().map(| message | {
        set_wildcard(message, filter.value as u8)
    }).collect()
}

fn midi_action_messages(filter: &mut MidiAction) -> Vec<Vec<u8>> {
    if  filter.active {
        filter.engage.clone()
    } else {
        filter.disengage.clone()
    }
}

pub fn midi_press_key(disposition: &mut Disposition, id: Id, key: u8, down: bool) {
    match disposition.elements.get_mut(&id) {
        Some(Element::MidiRank(rank)) => {
            debug!("midi rank ${} key {} {}", id, key, down);
            if down { rank._pressed_key_count += 1 } else { rank._pressed_key_count -= 1 };

            let pressed_keys = rank._pressed_key_count;

            let message = if down { vec![144, key, 127] } else { vec![128, key, 0] };
            let messages = match (down, pressed_keys) {
                (true, 1) => {
                    let mut messages = rank.acquire.clone();
                    messages.push(message);
                    messages
                },
                (false, 0) => {
                    let mut messages = rank.release.clone();
                    messages.push(message);
                    messages
                },
                _ => {
                    vec![message]
                },
            };

            let references = rank.references.clone();
            send_messages_dispatch(disposition, references, id.0, pressed_keys == 0, messages);
        }
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
                        debug!("midi console ${} send ${} '{:?}'", id, reference, message);
                        output.borrow_mut().connection.send(message.as_slice()).unwrap();
                    }
                }
            },
            _ => {},
        }
    };
}

fn log_midi_ports() {
    match get_output_ports() {
        Ok(ports) => {
            for port in ports {
                info!("Output Port '{}'", port);
            }
        },
        Err(e) => {
            warn!("failed to log midi ports: {}", e);
        }
    }
}