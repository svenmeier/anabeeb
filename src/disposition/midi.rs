use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use regex::{escape, Regex};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::disposition::Id;
use crate::disposition::midi_out::SharedOutput;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiConsole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip)]
    pub _output: Option<Rc<RefCell<SharedOutput>>>,
}

pub type MidiMessage = Vec<u8>;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiSwitchBinding {
    #[serde(default)]
    pub activate: MidiMessage,
    #[serde(default)]
    pub deactivate: MidiMessage,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiContinuousBinding {
    #[serde(default)]
    pub change: MidiMessage,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiKeyboardBinding {
    #[serde(default)]
    pub down: MidiMessage,
    #[serde(default)]
    pub up: MidiMessage,
}

pub fn to_regex(name: &str) -> Result<Regex, Box<dyn Error>> {
    let pattern = if name.starts_with('^') || name.ends_with('$') {
        name.to_string()
    } else {
        format!(".*{}.*", escape(name))
    };
    Ok(Regex::new(&pattern)?)
}