use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use crokey::KeyCombination;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::disposition::fluidsynth::FluidsynthSound;
use crate::disposition::general::{Captor, Combination, Coupler, Memory, Roller};
use crate::disposition::midi::MidiMessage;
use crate::disposition::midi_in::{MidiConsole, MidiKeyboard};
use crate::disposition::midi_out::{MidiAction, MidiRange, MidiRank, MidiSound};
use crate::disposition::rest::RestConsole;
use crate::disposition::term::TermConsole;

pub mod general;
pub mod midi;
pub mod midi_in;
pub mod midi_out;
pub mod rest;
pub mod fluidsynth;
pub mod term;

#[derive(Serialize, Deserialize,JsonSchema)]
pub struct Disposition {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    pub elements: BTreeMap<Id, Element>,

    #[serde(skip)]
    pub _path: Option<String>,

    #[serde(skip)]
    pub _binding: Option<Binding>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
pub struct Id(
    #[schemars(regex(pattern = r"^\S+$"))]
    pub String
);
impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<String> for Id {
    fn from(s: String) -> Self {
        Id(s.trim().into())
    }
}
impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id(s.to_string())
    }
}

pub struct Binding {
    pub id: Id,
    pub messages: Vec<MidiMessage>,
    pub keys: Vec<KeyCombination>,
}
impl Binding {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            messages: Vec::new(),
            keys: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize,JsonSchema)]
#[serde(tag = "type")]
pub enum Element {
    Coupler(Coupler),
    Captor(Captor),
    Combination(Combination),
    Roller(Roller),
    Memory(Memory),
    RestConsole(RestConsole),
    TermConsole(TermConsole),
    MidiConsole(MidiConsole),
    MidiKeyboard(MidiKeyboard),
    MidiRank(MidiRank),
    MidiRange(MidiRange),
    MidiAction(MidiAction),
    MidiSound(MidiSound),
    FluidsynthSound(FluidsynthSound),
}