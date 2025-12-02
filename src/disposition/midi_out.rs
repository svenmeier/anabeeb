use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::rc::Rc;
use log::{debug, error, info, warn};
use midir::{MidiOutput, MidiOutputConnection};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::disposition::{Disposition, Element, Id};
use crate::disposition::midi::{MidiSwitchBinding, MidiContinuousBinding, to_regex, MidiMessage};
use crate::midi::{get_output_ports, set_midi_channel, set_wildcard};
use crate::{print_error, print_info};
use crate::disposition::term::{TermContinuousBinding, TermSwitchBinding};
use crate::midi::channel_pool::ChannelPool;
use crate::processor::{midi_out_dispatch, Event, Events};

pub struct SharedOutput {
    connection: MidiOutputConnection,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiRange {
    #[serde(default)]
    pub value: u32,

    pub min: u32,

    pub max: u32,

    pub change: Vec<MidiMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding: Option<MidiContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_out_binding: Option<MidiContinuousBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip, default)]
    pub _channels: HashSet<String>
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiAction {
    #[serde(default)]
    pub active: bool,

    pub engage: Vec<MidiMessage>,

    pub disengage: Vec<MidiMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding: Option<MidiSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_out_binding: Option<MidiSwitchBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip, default)]
    pub _channels: HashSet<String>
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiRank {
    #[serde(default)]
    pub references: Vec<Id>,

    pub acquire: Vec<MidiMessage>,
    pub release: Vec<MidiMessage>,

    #[serde(skip, default)]
    pub _pressed_key_count: u8,
}

/**
 Forwards all MIDI out messages to an arbitrary MIDI device.
*/
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiSound {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,

    #[serde(skip)]
    pub _output: Option<Rc<RefCell<SharedOutput>>>,

    #[serde(skip, default)]
    pub _channels: ChannelPool,
}

pub struct MidiOutHandler {
    events: Events,
    outputs: HashMap<String, Rc<RefCell<SharedOutput>>>,
}
impl MidiOutHandler {
    pub fn new(events: Events) -> Self {
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
            Event::KeyPress(id, key) => {
                self.press_key(disposition, id.clone(), *key);
            },
            Event::KeyRelease(id, key) => {
                self.release_key(disposition, id.clone(), *key);
            },
            Event::MidiOutMessages(id, channel, messages, release) => {
                self.send_messages(disposition, id.clone(), channel, messages, *release);
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
            console_send(disposition, id.clone(), message);
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

            let message = set_wildcard(&binding.change, value);
            console_send(disposition, id.clone(), message)
        }
    }

    fn change(&self, disposition: &mut Disposition, id: Id, mut value: u32) {
        let modified = match disposition.elements.get_mut(&id) {
            Some(Element::MidiRange(range)) => {
                value = value.clamp(range.min, range.max);
                if value != range.value {
                    range.value = value;
                    print_info!("range ${} changed {}", id, value);

                    let messages = range_messages(range);
                    for channel in range._channels.clone().iter() {
                        midi_out_dispatch(&self.events, &range.references, channel, &messages, false);
                    }

                    true
                } else {
                    false
                }
            },
            _ => false,
        };

        if modified {
            self.events.append(Event::Modified(id.clone()));
        }
    }

    fn activate(&self, disposition: &mut Disposition, id: Id, active: bool) {
        let modified = match disposition.elements.get_mut(&id) {
            Some(Element::MidiAction(action)) => {
                if active != action.active {
                    action.active = active;
                    print_info!("action ${} activated {}", id, active);

                    let messages = action_messages(action);
                    for channel in action._channels.clone().iter() {
                        midi_out_dispatch(&self.events, &action.references, channel, &messages, false);
                    }

                    true
                } else {
                    false
                }
            },
            _ => false,
        };

        if modified {
            self.events.append(Event::Modified(id.clone()));
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

    fn send_messages(&self, disposition: &mut Disposition, id: Id, channel: &String, messages: &Vec<MidiMessage>, release: bool) {
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
            Some(Element::MidiRange(range)) => {
                midi_out_dispatch(&self.events, &range.references, channel, &messages, release);
                if release {
                    range._channels.remove(channel);
                } else {
                    if range._channels.insert(channel.clone()) {
                        let range_messages = range_messages(range);
                        midi_out_dispatch(&self.events, &range.references, channel, &range_messages, false);
                    }
                }
            }
            Some(Element::MidiAction(action)) => {
                midi_out_dispatch(&self.events, &action.references, channel, &messages, release);
                if release {
                    action._channels.remove(channel);
                } else {
                    if action._channels.insert(channel.clone()) {
                        let action_messages = action_messages(action);
                        midi_out_dispatch(&self.events, &action.references, channel, &action_messages, false);
                    }
                }

            },
            _ => {},
        }
    }

    fn press_key(&self, disposition: &mut Disposition, id: Id, key: u8) {
        match disposition.elements.get_mut(&id) {
            Some(Element::MidiRank(rank)) => {
                debug!("midi rank ${} press key {}", id, key);
                rank._pressed_key_count += 1;

                let message = vec![144, key, 127];
                let messages = match rank._pressed_key_count {
                    1 => {
                        let mut messages = rank.acquire.clone();
                        messages.push(message);
                        messages
                    },
                    _ => {
                        vec![message]
                    },
                };

                midi_out_dispatch(&self.events, &rank.references, &id.0, &messages, false);
            }
            _ => {},
        }
    }

    fn release_key(&self, disposition: &mut Disposition, id: Id, key: u8) {
        match disposition.elements.get_mut(&id) {
            Some(Element::MidiRank(rank)) => {
                debug!("midi rank ${} release key {}", id, key);
                rank._pressed_key_count -= 1;

                let message = vec![128, key, 0];
                let (messages, release) = match rank._pressed_key_count {
                    0 => {
                        let mut messages = rank.release.clone();
                        messages.push(message);
                        (messages, true)
                    },
                    _ => {
                        (vec![message], false)
                    },
                };

                midi_out_dispatch(&self.events, &rank.references, &id.0, &messages, release);
            }
            _ => {},
        }
    }
}

fn range_messages(filter: &mut MidiRange) -> Vec<MidiMessage> {
    filter.change.iter().map(| message | {
        set_wildcard(message, filter.value as u8)
    }).collect()
}

fn action_messages(action: &mut MidiAction) -> Vec<MidiMessage> {
    if  action.active {
        action.engage.clone()
    } else {
        action.disengage.clone()
    }
}

fn console_send(disposition: &mut Disposition, reference: Id, message: MidiMessage) {
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