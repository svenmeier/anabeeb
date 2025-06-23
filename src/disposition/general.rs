use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use log::{debug, error, info, warn};
use schemars::JsonSchema;
use crate::disposition::fluidsynth::FluidsynthSound;
use crate::disposition::midi_out::{midi_activate, midi_element_modified, midi_register_press_key};
use crate::disposition::rest::{rest_element_modified, RestConsole};
use crate::disposition::term::{term_element_modified, TermMomentaryBinding, TermConsole, TermSwitchBinding, TermContinuousBinding};
use crate::io::{combine_paths, read_memory};
use crate::disposition::general::CombinationCapture::{Active, Value};
use crate::disposition::midi::{MidiMomentaryBinding, MidiConsole, MidiRange, MidiKeyboard, MidiRegister, MidiSound, MidiSwitchBinding, MidiAction, MidiContinuousBinding};
use crate::disposition::midi_out::{midi_change};
use crate::processor::{Processor};

#[derive(Serialize, Deserialize,JsonSchema)]
pub struct Disposition {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    pub elements: BTreeMap<Id, Element>,

    #[serde(skip)]
    pub _path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
pub struct Id(
    #[schemars(regex(pattern = r"^\S+$"))]
    pub String
);
impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
impl From<String> for Id {
    fn from(s: String) -> Self {
        Id(s)
    }
}
impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id(s.to_string())
    }
}

#[derive(Serialize, Deserialize,JsonSchema)]
#[serde(tag = "type")]
pub enum Element {
    Coupler(Coupler),
    Captor(Captor),
    Memory(Memory),
    Combination(Combination),
    RestConsole(RestConsole),
    TermConsole(TermConsole),
    MidiConsole(MidiConsole),
    MidiKeyboard(MidiKeyboard),
    MidiRegister(MidiRegister),
    MidiRange(MidiRange),
    MidiAction(MidiAction),
    MidiSound(MidiSound),
    FluidsynthSound(FluidsynthSound),
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Coupler {
    #[serde(default)]
    pub active: bool,

    #[serde(default)]
    pub transpose: i8,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_binding: Option<MidiSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermSwitchBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip)]
    #[serde(default)]
    _down_keys: HashMap<u8, u8>
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Captor {
    #[serde(default)]
    pub active: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_binding: Option<MidiSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermSwitchBinding>,
}

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
    pub midi_binding: Option<MidiContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip)]
    #[serde(default)]
    pub _state: Option<MemoryState>,
}

fn default_memory_state() -> String { "memory.json".to_string() }

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

pub type CombinationState = BTreeMap<Id, CombinationCapture>;

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct MemoryLevel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default)]
    pub references: BTreeMap<Id, CombinationState>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Combination {

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_binding: Option<MidiMomentaryBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermMomentaryBinding>,

    #[serde(default)]
    pub references: Vec<Id>,
    
    #[serde(default)]
    pub state: CombinationState,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CombinationCapture {
    Active(bool),
    Value(u32),
}

pub fn general_init(disposition: &mut Disposition, _: &Processor) {
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
                        info!("loaded memory state {} from '{}'", id, combined_path);
                    },
                }
            },
            _ => {},
        };
    }
}

pub fn press_key_dispatch(disposition: &mut Disposition, ids: Vec<Id>, key: u8, down: bool) {
    for id in ids {
        match disposition.elements.get_mut(&id) {
            Some(Element::Coupler(_)) => {
                press_key(disposition, id, key, down);
            },
            Some(Element::MidiRegister(_)) => {
                midi_register_press_key(disposition, id, key, down);
            },
            None => {
                warn!("unknown id {}", id);
            },
            _ => {},
        };
    }
}

pub fn change(disposition: &mut Disposition, id: Id, mut value: u32) {
    let modified = match disposition.elements.get_mut(&id) {
        Some(Element::Memory(memory)) => {
            value = value.clamp(memory.min, memory.max);
            if value != memory.value {
                let old_value = memory.value;
                memory.value = value;
                capture_memory(disposition, id.clone(), old_value as usize, value as usize);
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

fn capture_memory(disposition: &mut Disposition, id: Id, previous_index: usize, new_index: usize) {

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

pub fn activate(disposition: &mut Disposition, id: Id, active: bool) {
    let modified = match disposition.elements.get_mut(&id) {
        Some(Element::Captor(captor)) => {
            if captor.active != active {
                captor.active = active;
                true
            } else {
                false
            }
        },
        Some(Element::Coupler(coupler)) => {
            if coupler.active != active {
                coupler.active = active;

                let keys: Vec<u8>  = coupler._down_keys.keys().cloned().collect();
                let references = coupler.references.clone();
                let transpose = coupler.transpose.clone();
                for key in keys {
                    press_key_dispatch(disposition, references.clone(), coupler_transpose(key, transpose), active);
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

pub fn press_key(disposition: &mut Disposition, id: Id, key: u8, down: bool) {
    match disposition.elements.get_mut(&id) {
        Some(Element::Coupler(coupler)) => {
            let transpose = coupler.transpose;
            if down {
                if let Some(value) = coupler._down_keys.get_mut(&key) {
                    *value += 1;
                } else {
                    coupler._down_keys.insert(key, 1);
                    debug!("coupler {} key {} true", id, key);
                    if coupler.active {
                        let ids = coupler.references.clone();

                        press_key_dispatch(disposition, ids, coupler_transpose(key, transpose), true);
                    }
                }
            } else {
                if let Some(value) = coupler._down_keys.get_mut(&key) {
                    if *value == 0 {
                        // ignore
                        return
                    }
                    *value -= 1;

                    if *value == 0 {
                        coupler._down_keys.remove(&key);
                        debug!("coupler {} key {} false", id, key);

                        if coupler.active {
                            let ids = coupler.references.clone();

                            press_key_dispatch(disposition, ids, coupler_transpose(key, transpose), false);
                        }
                    }
                }
            }
        },
        _ => {},
    };
}

fn coupler_transpose(key: u8, delta: i8) -> u8 {
    (key as i8 + delta).clamp(0, 127) as u8
}

pub fn combination_trigger(disposition: &mut Disposition, id: Id) {
    let capturing = disposition.elements.iter_mut().find_map(|(id, element)| {
        if let Element::Captor(captor) = element {
            if captor.active {
                return Some(id.clone());
            }
        }
        None
    });
    if let Some(capturing) = capturing {
        activate(disposition, capturing, false);
        capture(disposition, id.clone());
        return;
    }
    
    recall(disposition, id.clone());
}

fn recall(disposition: &mut Disposition, id: Id) {
    match disposition.elements.get_mut(&id) {
        Some(Element::Combination(combination)) => {
            let state = combination.state.clone();
            
            info!("combination recall {}", id);
            for id in combination.references.clone() {
                match disposition.elements.get(&id) {
                    Some(Element::Coupler(_)) => {
                        if let Some(Active(active)) = state.get(&id) {
                            activate(disposition, id, *active);
                        }
                    },
                    Some(Element::MidiAction(_)) => {
                        if let Some(Active(active)) = state.get(&id) {
                            midi_activate(disposition, id, *active);
                        }
                    },
                    Some(Element::MidiRange(_)) => {
                        if let Some(Value(value)) = state.get(&id) {
                            midi_change(disposition, id, *value);
                        }
                    },
                    _ => {},
                }
            }
        },
        _ => {},
    };
}

fn capture(disposition: &mut Disposition, id: Id) {
    info!("combination capture {}", id);

    let ids: Vec<Id> = if let Some(Element::Combination(combination)) = disposition.elements.get(&id) {
        combination.references.clone()
    } else {
        warn!("invalid id {}", id);
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
