use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::disposition::general::Id;
use crate::disposition::term::{TermContinuousBinding, TermSwitchBinding};
use crate::midi::channel_pool::ChannelPool;
use crate::processor::Output;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiSwitchBinding {
    #[serde(default)]
    pub activate: Vec<u8>,
    #[serde(default)]
    pub deactivate: Vec<u8>,
    #[serde(default)]
    pub activated: Vec<u8>,
    #[serde(default)]
    pub deactivated: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiContinuousBinding {
    #[serde(default)]
    pub change: Vec<u8>,
    #[serde(default)]
    pub changed: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiMomentaryBinding {
    #[serde(default)]
    pub trigger: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MidiKeyboardBinding {
    #[serde(default)]
    pub key_down: Vec<u8>,
    #[serde(default)]
    pub key_up: Vec<u8>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiConsole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip)]
    pub _output: Option<Output>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiKeyboard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_binding : Option<MidiKeyboardBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip, default)]
    pub _pressed_keys: HashSet<u8>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiRange {
    #[serde(default)]
    pub value: u32,

    pub min: u32,

    pub max: u32,

    pub change: Vec<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermContinuousBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_binding: Option<MidiContinuousBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip, default)]
    pub _channels: HashSet<String>
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiAction {
    #[serde(default)]
    pub active: bool,
    
    pub engage: Vec<Vec<u8>>,

    pub disengage: Vec<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_binding: Option<TermSwitchBinding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_binding: Option<MidiSwitchBinding>,

    #[serde(default)]
    pub references: Vec<Id>,

    #[serde(skip, default)]
    pub _channels: HashSet<String>
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiRegister {
    #[serde(default)]
    pub references: Vec<Id>,

    pub acquire: Vec<Vec<u8>>,
    pub release: Vec<Vec<u8>>,

    #[serde(skip, default)]
    pub _pressed_key_count: u8,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MidiSound {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,

    #[serde(skip)]
    pub _output: Option<Output>,

    #[serde(skip, default)]
    pub _channels: ChannelPool,
}
