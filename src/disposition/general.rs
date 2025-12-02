use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
use log::{debug, error, info, warn};
use schemars::JsonSchema;
use crate::disposition::term::{TermMomentaryBinding, TermSwitchBinding, TermContinuousBinding};
use crate::io::{combine_paths, read_memory};
use crate::disposition::general::CombinationCapture::{Active, Value};
use crate::disposition::{Binding, Disposition, Element, Id};
use crate::disposition::midi::{MidiMomentaryBinding, MidiSwitchBinding, MidiContinuousBinding};
use crate::print_info;
use crate::processor::{key_press_dispatch, key_release_dispatch, Event, Events};

/**
 When activated, forwards all key presses to referenced elements.
*/
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Coupler {
    #[serde(default)]
    pub active: bool,

    #[serde(default)]
    pub transpose: i8,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding: Option<MidiSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_out_binding: Option<MidiSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermSwitchBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip)]
    #[serde(default)]
    _down_keys: HashMap<u8, u8>
}

/**
 When activated, the next triggered combination will record its state rather than recalling it.
*/
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Captor {
    #[serde(default)]
    pub active: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding: Option<MidiSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_out_binding: Option<MidiSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermSwitchBinding>,
}

/**
When triggered, recalls the state of all references elements.
*/
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Combination {

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding: Option<MidiMomentaryBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_out_binding: Option<MidiMomentaryBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermMomentaryBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(default)]
    pub state: CombinationState,
}

/**
 When its value changes, stores the state of referenced combinations in a matching level. 
*/
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Memory {
    #[serde(default)]
    pub value: u32,

    pub min: u32,

    pub max: u32,

    #[serde(default = "default_memory_state")]
    pub state: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding: Option<MidiContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_out_binding: Option<MidiContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip)]
    #[serde(default)]
    pub _state: Option<MemoryState>,
}

/**
 When its value changes, the element matching the previous value is deactivated,
 and the element matching the new value is activated (or triggered, if it is a combination).
*/
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Roller {
    #[serde(default)]
    pub value: u32,

    pub min: u32,

    pub max: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_in_binding: Option<MidiContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_out_binding: Option<MidiContinuousBinding>,

    #[serde(default)]
    pub references: Vec<Id>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct MemoryState {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default)]
    pub levels: Vec<MemoryLevel>,
}
impl MemoryState {
    pub fn new() -> Self {
        Self {
            title: None,
            schema: None,
            levels: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct MemoryLevel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default)]
    pub references: BTreeMap<Id, CombinationState>,
}

pub type CombinationState = BTreeMap<Id, CombinationCapture>;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CombinationCapture {
    Active(bool),
    Value(u32),
}

pub struct GeneralHandler {
    pub events: Events,
}
impl GeneralHandler {
    pub fn new(events: Events) -> Self {
        Self {
            events,
        }
    }

    pub fn init(&self, disposition: &mut Disposition) {
        let path = disposition._path.as_deref().unwrap_or(".");

        for (id, element) in &mut disposition.elements {
            match element {
                Element::Memory(memory) => {
                    let combined_path = combine_paths(&path, &memory.state);
                    match read_memory(combined_path.clone()) {
                        Err(e) => {
                            error!("failed to load memory: {}", e);
                        },
                        Ok(state) => {
                            memory._state = Some(state);
                            info!("loaded memory state ${} from '{}'", id, combined_path);
                        },
                    }
                },
                _ => {},
            };
        }
    }

    pub fn process(&self, disposition: &mut Disposition, event: &Event) {
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
            Event::Trigger(id) => {
                self.trigger(disposition, id.clone());
            },
            Event::BindingStart(id) => {
                print_info!("binding start ${}", id);
                disposition._binding = Some(Binding::new(id.clone()));
            },
            Event::BindingEnd => {
                if let Some(binding) = &disposition._binding {
                    print_info!("binding end ${}", binding.id.clone());
                    disposition._binding = None;
                }
            },
            _ => {}
        }
    }

    fn change(&self, disposition: &mut Disposition, id: Id, mut value: u32) {
        let modified = match disposition.elements.get_mut(&id) {
            Some(Element::Memory(memory)) => {
                value = value.clamp(memory.min, memory.max);
                if value != memory.value {
                    let old_value = memory.value;
                    memory.value = value;
                    print_info!("memory ${} changed {}", id, memory.value);

                    self.memory_level(disposition, id.clone(), old_value as usize, value as usize);
                    true
                } else {
                    false
                }
            },
            Some(Element::Roller(roller)) => {
                value = value.clamp(roller.min, roller.max);
                if value != roller.value {
                    let old_value = roller.value;
                    roller.value = value;
                    print_info!("roller ${} changed {}", id, roller.value);

                    self.roller_roll(disposition, id.clone(), old_value as usize, value as usize);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        if modified {
            self.events.append(Event::Modified(id.clone()));
        }
    }

    fn activate(&self, disposition: &mut Disposition, id: Id, active: bool) {
        let modified = match disposition.elements.get_mut(&id) {
            Some(Element::Captor(captor)) => {
                if captor.active != active {
                    captor.active = active;
                    print_info!("captor ${} activated {}", id, captor.active);

                    true
                } else {
                    false
                }
            },
            Some(Element::Coupler(coupler)) => {
                if coupler.active != active {
                    coupler.active = active;
                    print_info!("coupler ${} activated {}", id, coupler.active);

                    let keys: Vec<u8>  = coupler._down_keys.keys().cloned().collect();
                    let references = coupler.references.clone();
                    let transpose = coupler.transpose.clone();
                    for key in keys {
                        if active {
                            key_press_dispatch(&self.events, &references, coupler_transpose(key, transpose));
                        } else {
                            key_release_dispatch(&self.events, &references, coupler_transpose(key, transpose));
                        }
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

    fn trigger(&self, disposition: &mut Disposition, id: Id) {
        let captor_id = disposition.elements.iter_mut().find_map(|(id, element)| {
            if let Element::Captor(captor) = element {
                if captor.active {
                    return Some(id.clone());
                }
            }
            None
        });
        if let Some(captor_id) = captor_id {
            self.events.prepend(Event::Activate(captor_id.clone(), false));
            self.combination_capture(disposition, id.clone());
            return;
        }

        self.combination_recall(disposition, id.clone());
    }

    fn combination_recall(&self, disposition: &mut Disposition, id: Id) {
        match disposition.elements.get_mut(&id) {
            Some(Element::Combination(combination)) => {
                let state = combination.state.clone();

                info!("combination recall ${}", id);
                for id in combination.references.clone() {
                    match disposition.elements.get(&id) {
                        Some(Element::Coupler(_) | Element::MidiAction(_)) => {
                            if let Some(Active(active)) = state.get(&id) {
                                self.events.prepend(Event::Activate(id.clone(), active.clone()));
                            }
                        },
                        Some(Element::Roller(_) | Element::MidiRange(_)) => {
                            if let Some(Value(value)) = state.get(&id) {
                                self.events.prepend(Event::Change(id.clone(), value.clone()));
                            }
                        },
                        _ => {},
                    }
                }
            },
            _ => {},
        };
    }

    fn combination_capture(&self, disposition: &mut Disposition, id: Id) {
        let ids = if let Some(Element::Combination(combination)) = disposition.elements.get(&id) {
            info!("combination capture ${}", id);
            combination.references.clone()
        } else {
            warn!("invalid id ${}", id);
            return;
        };

        let state: CombinationState = ids
            .into_iter()
            .filter_map(|id| {
                match disposition.elements.get_mut(&id) {
                    Some(Element::Coupler(coupler)) => {
                        Some((id, Active(coupler.active)))
                    },
                    Some(Element::MidiAction(action)) => {
                        Some((id, Active(action.active)))
                    },
                    Some(Element::MidiRange(range)) => {
                        Some((id, Value(range.value)))
                    },
                    Some(Element::Roller(roller)) => {
                        Some((id, Value(roller.value)))
                    }
                    _ => None,
                }
            })
            .collect();

        match disposition.elements.get_mut(&id) {
            Some(Element::Combination(combination)) => {
                combination.state = state;
            },
            _ => {},
        };
    }
    
    fn roller_roll(&self, disposition: &mut Disposition, id: Id, previous_index: usize, new_index: usize) {
        if let Some(Element::Roller(roller)) = disposition.elements.get_mut(&id) {
            if let Some(id) = roller.references.get(previous_index).cloned() {
                let event = match disposition.elements.get(&id) {
                    Some(Element::Combination(_)) => None,
                    _ => Some(Event::Activate(id, false))
                };

                if let Some(event) = event {
                    self.events.prepend(event);
                }
            }
        }

        if let Some(Element::Roller(roller)) = disposition.elements.get_mut(&id) {
            if let Some(id) = roller.references.get(new_index).cloned() {
                let event = match disposition.elements.get(&id) {
                    Some(Element::Combination(_)) => Some(Event::Trigger(id)),
                    _ => Some(Event::Activate(id, true)),
                };

                if let Some(event) = event {
                    self.events.prepend(event);
                }
            }
        }
    }
    
    fn memory_level(&self, disposition: &mut Disposition, id: Id, previous_index: usize, new_index: usize) {

        let mut previous_level = MemoryLevel{ title: None, references: BTreeMap::new() };
        for id in disposition.elements.keys().cloned().collect::<Vec<Id>>() {
            match disposition.elements.get(&id) {
                Some(Element::Combination(combination)) => {
                    previous_level.references.insert(id.clone(), combination.state.clone());
                }
                _ => {},
            }
        }

        if let Some(Element::Memory(memory)) = disposition.elements.get_mut(&id) {
            let state = memory._state.get_or_insert_with(MemoryState::new);

            if previous_index < state.levels.len() {
                previous_level.title = state.levels.get(previous_index).unwrap().title.clone();
            } else {
                state.levels.resize(previous_index + 1, MemoryLevel{ title: None, references: BTreeMap::new() });
            }
            state.levels[previous_index] = previous_level;

            if let Some(mut new_level) = state.levels.get(new_index).cloned() {
                memory.title = new_level.title.clone();

                for id in disposition.elements.keys().cloned().collect::<Vec<Id>>() {
                    match disposition.elements.get_mut(&id) {
                        Some(Element::Combination(combination)) => {
                            if let Some(state) = new_level.references.remove(&id) {
                                combination.state = state;
                            }
                        }
                        _ => {},
                    }
                }
            }
        }
    }

    fn press_key(&self, disposition: &mut Disposition, id: Id, key: u8) {
        match disposition.elements.get_mut(&id) {
            Some(Element::Coupler(coupler)) => {
                let transpose = coupler.transpose;
                if let Some(value) = coupler._down_keys.get_mut(&key) {
                    *value += 1;
                } else {
                    coupler._down_keys.insert(key, 1);
                    debug!("coupler ${} key {} true", id, key);
                    if coupler.active {
                        key_press_dispatch(&self.events, &coupler.references, coupler_transpose(key, transpose));
                    }
                }
            },
            _ => {},
        };
    }

    fn release_key(&self, disposition: &mut Disposition, id: Id, key: u8) {
        match disposition.elements.get_mut(&id) {
            Some(Element::Coupler(coupler)) => {
                let transpose = coupler.transpose;
                if let Some(value) = coupler._down_keys.get_mut(&key) {
                    if *value == 0 {
                        // ignore
                        return
                    }
                    *value -= 1;

                    if *value == 0 {
                        coupler._down_keys.remove(&key);
                        debug!("coupler ${} key {} false", id, key);

                        if coupler.active {
                            key_release_dispatch(&self.events, &coupler.references, coupler_transpose(key, transpose));
                        }
                    }
                }
            },
            _ => {},
        };
    }
}

fn coupler_transpose(key: u8, delta: i8) -> u8 {
    (key as i8 + delta).clamp(0, 127) as u8
}

fn default_memory_state() -> String { "memory.json".to_string() }