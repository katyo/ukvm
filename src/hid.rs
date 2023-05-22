pub use hidg_core::{Button, Key, Led, MouseInput};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KeyboardState {
    #[serde(rename = "k")]
    pub keys: Vec<Key>,
    #[serde(rename = "l")]
    pub leds: Vec<Led>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MouseState {
    #[serde(rename = "b")]
    pub buttons: Vec<Button>,
    #[serde(rename = "p")]
    pub pointer: (i16, i16),
    #[serde(rename = "w")]
    pub wheel: i8,
}

impl From<&MouseInput> for MouseState {
    fn from(input: &MouseInput) -> Self {
        Self {
            buttons: input.pressed().collect(),
            pointer: input.pointer(),
            wheel: input.wheel(),
        }
    }
}
