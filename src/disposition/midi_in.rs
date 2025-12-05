use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex};
use log::{debug, info, warn};
use midir::{MidiInput, MidiInputConnection};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::disposition::{Disposition, Element, Id};
use crate::midi::{get_input_ports, get_wildcard};
use crate::{print_error, print_info};
use crate::disposition::midi::{to_regex, MidiContinuousBinding, MidiKeyboardBinding, MidiMessage, MidiMomentaryBinding, MidiSwitchBinding};
use crate::processor::{key_press_dispatch, key_release_dispatch, Event, Events};

struct SharedInput {
    _connection: MidiInputConnection<()>,
    ids: Arc<Mutex<Vec<Id>>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiKeyboard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding : Option<MidiKeyboardBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip, default)]
    pub _pressed_keys: HashSet<u8>,
}

pub struct MidiInHandler {
    inputs: HashMap<String, SharedInput>,
    events: Events,
}
impl MidiInHandler {
    pub fn new(events: Events) -> Self {
        Self {
            inputs: HashMap::new(),
            events,
        }
    }

    pub fn init(&mut self, disposition: &mut Disposition) {
        log_midi_ports();
        
        for (id, element) in &mut disposition.elements {
            match element {
                Element::MidiKeyboard(keyboard) => {
                    if let Some(port) = &keyboard.port {
                        self.midi_input(id, port);
                    }
                },
                Element::MidiConsole(console) => {
                    if let Some(port) = &console.port {
                        self.midi_input(id, port);
                    }
                },
                _ => {},
            };
        }
    }

    pub fn process(&mut self, disposition: &mut Disposition, event: &Event) {
        match event {
            Event::MidiInMessage(id, message) => {
                match disposition.elements.get_mut(&id) {
                    Some(Element::MidiKeyboard(keyboard)) => {
                        if let Some(binding) = &mut disposition._binding {
                            if binding.id == *id {
                                binding.messages.push(message.clone());
                            }
                            return;
                        }

                        if let Some(binding) = &keyboard.midi_in_binding {
                            let key_down = binding.key_down.clone();
                            let key_up = binding.key_up.clone();

                            if let Some((_, key)) = get_wildcard(&message, &key_down) {
                                if keyboard._pressed_keys.insert(key) {
                                    debug!("midi keyboard ${} key {} pressed", id, key);
                                    key_press_dispatch(&self.events, &keyboard.references, key);
                                } else {
                                    warn!("midi keyboard ${} key {} already pressed", id, key)
                                }
                            } else if let Some((_, key)) = get_wildcard(&message, &key_up) {
                                if keyboard._pressed_keys.remove(&key) {
                                    debug!("midi keyboard ${} key {} released", id, key);
                                    key_release_dispatch(&self.events, &keyboard.references, key);
                                } else {
                                    warn!("midi keyboard ${} key {} was not pressed", id, key)
                                }
                            }
                        }
                    },
                    Some(Element::MidiConsole(console)) => {
                        if let Some(binding) = &mut disposition._binding {
                            if console.references.contains(&binding.id) {
                                binding.messages.push(message.clone());
                            }
                            return;
                        }

                        let references = console.references.clone();
                        self.match_bindings(disposition, references, &message);
                    },
                    _ => {},
                }
            },
            Event::MidiPanic => {
                self.midi_panic(disposition);
            },
            Event::BindingEnd => {
                binding_end(disposition);
            },
            _ => {},
        }
    }

    fn match_bindings(&self, disposition: &mut Disposition, ids: Vec<Id>, message: &MidiMessage) {
        for id in ids {
            match disposition.elements.get_mut(&id) {
                Some(Element::Coupler(coupler)) => {
                    self.match_switch_binding(id, &message, &coupler.midi_in_binding);
                },
                Some(Element::Captor(captor)) => {
                    self.match_switch_binding(id, &message, &captor.midi_in_binding);
                },
                Some(Element::MidiAction(action)) => {
                    self.match_switch_binding(id, &message, &action.midi_in_binding);
                }
                Some(Element::MidiRange(range)) => {
                    self.match_continuous_binding(id, range.min, range.max, &message, &range.midi_in_binding);
                },
                Some(Element::Memory(memory)) => {
                    self.match_continuous_binding(id, memory.min, memory.max, &message, &memory.midi_in_binding);
                },
                Some(Element::Combination(combination)) => {
                    if let Some(binding) = &mut combination.midi_in_binding {
                        if binding.trigger == *message {
                            self.events.append(Event::Trigger(id));
                        }
                    }
                },
                None => {
                    warn!("invalid id ${}", id);
                },
                _ => {},
            }
        }
    }

    fn match_switch_binding(&self, id: Id, message: &MidiMessage, binding: &Option<MidiSwitchBinding>) {
        if let Some(binding) = binding {
            if binding.activate == *message {
                self.events.append(Event::Activate(id, true));
            } else if binding.deactivate == *message {
                self.events.append(Event::Activate(id, false));
            }
        }
    }

    fn match_continuous_binding(&self, id: Id, min: u32, max: u32, message: &MidiMessage, binding: &Option<MidiContinuousBinding>) {
        if let Some(binding) = binding {
            if let Some((_, value)) = get_wildcard(&message, &binding.change) {
                let delta = max.saturating_sub(min);
                let value = min.saturating_add((value as u32).saturating_mul(delta) / 127);

                self.events.append(Event::Change(id, value));
            }
        }
    }

    fn midi_input(&mut self, id: &Id, name: &str)  {
        match self.try_midi_input(id, name) {
            Ok(port_name) => {
                print_info!("connected ${} to port '{}'", id, port_name);
            },
            Err(e) => {
                print_error!("connection ${} failed: {}", id, e);
            },
        }
    }

    fn try_midi_input(&mut self, id: &Id, name: &str) -> Result<String, Box<dyn Error>> {
        let midi_in = MidiInput::new("anabeeb").expect("no input");

        let regex = to_regex(&name)?;

        for port in midi_in.ports() {
            if let Ok(port_name) = midi_in.port_name(&port) {
                if regex.is_match(&port_name) {
                    match self.inputs.get_mut(&port_name) {
                        Some(input) => {
                            input.ids.lock().unwrap().push(id.clone());
                        },
                        None => {
                            let sender_clone = self.events.clone();
                            let ids = Arc::new(Mutex::new(vec![id.clone()]));

                            let ids_clone = ids.clone();
                            let _connection = midi_in.connect(&port, "anabeeb",
                                                              move |_, message, _| {
                                                                  let ids_lock = ids_clone.lock().unwrap();

                                                                  debug!("midi in message {:?}", message);

                                                                  for id in ids_lock.iter() {
                                                                      sender_clone.append(Event::MidiInMessage(id.clone(), message.to_vec()));
                                                                  }
                                                              },
                                                              (),
                            )?;

                            let input = SharedInput { _connection, ids };
                            self.inputs.insert(port_name.clone(), input);
                        },
                    };

                    return Ok(port_name);
                }
            }
        }

        Err(format!("no input port '{}'", name).into())
    }

    fn midi_panic(&self, disposition: &Disposition) {
        let mut panicked = false;
        
        for (id, element) in &disposition.elements {
            if let Element::MidiKeyboard(keyboard) = element {
                let keys = keyboard._pressed_keys.clone();
                for key in keys {
                    panicked = true;
                    info!("midi panic ${} stuck key {}", id, key);
                    key_release_dispatch(&self.events, &keyboard.references, key);
                }
            }
        }
        
        if !panicked {
            info!("midi panic no stuck keys");
        }
    }
}

fn binding_end(disposition: &mut Disposition) {
    if let Some(binding) = &disposition._binding {
        if let Some(element) = disposition.elements.get_mut(&binding.id) {
            match element {
                Element::Coupler(e) => {
                    e.midi_in_binding = switch_binding(&binding.messages).or(e.midi_in_binding.clone());
                }
                Element::Combination(e) => {
                    e.midi_in_binding = momentary_binding(&binding.messages).or(e.midi_in_binding.clone());
                }
                Element::Captor(e) => {
                    e.midi_in_binding = switch_binding(&binding.messages).or(e.midi_in_binding.clone());
                }
                Element::Memory(e) => {
                    e.midi_in_binding = continuous_binding(&binding.messages).or(e.midi_in_binding.clone());
                }
                Element::MidiAction(e) => {
                    e.midi_in_binding = switch_binding(&binding.messages).or(e.midi_in_binding.clone());
                }
                Element::MidiRange(e) => {
                    e.midi_in_binding = continuous_binding(&binding.messages).or(e.midi_in_binding.clone());
                }
                Element::MidiKeyboard(e) => {
                    e.midi_in_binding = midi_keyboard_binding(&binding.messages).or(e.midi_in_binding.clone());
                }
                _ => {},
            }
        }
    }
}

fn midi_keyboard_binding(messages: &Vec<MidiMessage>) -> Option<MidiKeyboardBinding> {
    let mut key_down: MidiMessage = Vec::new();
    let mut key_up: MidiMessage = Vec::new();

    for message in messages {
        if message.len() >= 3 {
            let status = message[0] & 0xF0;
            let channel = message[0] & 0x0F;
            let data = message[2];
            if status == 144 && data > 0 {
                key_down = vec![144 | channel, 255, 255];
            } else if status == 144 && data == 0 {
                key_up = vec![144 | channel, 255, 0];
            } else if status == 128 {
                key_up = vec![128 | channel, 255, 255];
            }
        }
    }

    if key_down.is_empty() || key_up.is_empty() {
        return None;
    }
    Some(MidiKeyboardBinding{ key_down, key_up })
}

fn continuous_binding(messages: &Vec<MidiMessage>) -> Option<MidiContinuousBinding> {
    let change = messages.iter().cloned().reduce(|mut a, b| {
        if a[0] != b[0] {
            a[0] = 255;
        }
        if a[1] != b[1] {
            a[1] = 255;
        }
        if a[2] != b[2] {
            a[2] = 255;
        }
        a
    });

    change.map(|m| MidiContinuousBinding { change: m.clone() })
}

fn momentary_binding(messages: &Vec<MidiMessage>) -> Option<MidiMomentaryBinding> {

    let len = messages.len();
    if len >= 1 {
        return Some(MidiMomentaryBinding {
            trigger: messages[len - 1].clone(),
        })
    }
    None
}

fn switch_binding(messages: &Vec<MidiMessage>) -> Option<MidiSwitchBinding> {
    let len = messages.len();
    if len >= 2 {
        return Some(MidiSwitchBinding{
            activate: messages[len - 2].clone(),
            deactivate: messages[len - 1].clone(),
        })
    }
    None
}

fn log_midi_ports() {
    match get_input_ports() {
        Ok(ports) => {
            for port in ports {
                info!("Input Port '{}'", port);
            }
        },
        Err(e) => {
            warn!("failed to log midi ports: {}", e);
        }
    }
}
