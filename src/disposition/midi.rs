use std::error::Error;
use regex::{escape, Regex};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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
pub struct MidiMomentaryBinding {
    #[serde(default)]
    pub trigger: MidiMessage,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiKeyboardBinding {
    #[serde(default)]
    pub key_down: MidiMessage,
    #[serde(default)]
    pub key_up: MidiMessage,
}

pub fn to_regex(name: &str) -> Result<Regex, Box<dyn Error>> {
    let pattern = if name.starts_with('^') || name.ends_with('$') {
        name.to_string()
    } else {
        format!(".*{}.*", escape(name))
    };
    Ok(Regex::new(&pattern)?)
}