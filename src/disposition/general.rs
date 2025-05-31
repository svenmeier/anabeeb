use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::{Display, Formatter};
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use log::{debug, info, warn};
use schemars::JsonSchema;
use crate::disposition::fluidsynth::FluidsynthSound;
use crate::disposition::midi_out::{midi_activate, midi_element_modified, midi_register_press_key};
use crate::disposition::rest::{rest_element_modified, RestConsole};
use crate::disposition::term::{term_element_modified, TermMomentaryBinding, TermConsole, TermSwitchBinding};
use crate::io::write_disposition;
use crate::{print_error, print_info};
use crate::disposition::general::CombinationCapture::{Active, Value};
use crate::disposition::midi::{MidiMomentaryBinding, MidiConsole, MidiRange, MidiKeyboard, MidiRegister, MidiSound, MidiSwitchBinding, MidiAction};
use crate::disposition::midi_out::{midi_change};
use crate::processor::Event;

#[derive(Serialize, Deserialize,JsonSchema)]
pub struct Disposition {
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
pub struct Combination {

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_binding: Option<MidiMomentaryBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermMomentaryBinding>,

    #[serde(default)]
    pub references: HashMap<Id, CombinationCapture>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CombinationCapture {
    Active(bool),
    Value(u32),
}

pub fn general_init(_: &mut Disposition, _: &Sender<Event>) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn save(disposition: &Disposition) {
    match write_disposition(disposition) {
        Ok(()) => {
            print_info!("saved disposition");
        },
        Err(e) =>  {
            print_error!("failed to save disposition: {}", e);
        },
    }
}

pub fn quit() {
    print_info!("good bye");
    std::process::exit(0);
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
        _ => false
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
        }
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
            info!("combination recall {}", id);
            for (id, capture) in combination.references.clone() {
                match disposition.elements.get(&id) {
                    Some(Element::Coupler(_)) => {
                        match capture {
                            Active(active) => {
                                activate(disposition, id, active);
                            },
                            _ => {}
                        }
                    },
                    Some(Element::MidiAction(_)) => {
                        match capture {
                            Active(active) => {
                                midi_activate(disposition, id, active);
                            },
                            _ => {}
                        }
                    },
                    Some(Element::MidiRange(_)) => {
                        match capture {
                            Value(value) => {
                                midi_change(disposition, id, value);
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        },
        _ => {}
    };
}

fn capture(disposition: &mut Disposition, id: Id) {
    info!("combination capture {}", id);

    let ids: Vec<Id> = if let Some(Element::Combination(combination)) = disposition.elements.get(&id) {
        combination.references.keys().cloned().collect()
    } else {
        warn!("invalid id {}", id);
        return;
    };

    let references: HashMap<Id, CombinationCapture> = ids
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
            combination.references = references;
        },
        _ => {},
    };
}
