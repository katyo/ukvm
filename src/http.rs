use crate::{
    hid::{Button, Key, KeyboardState, Led, MouseState},
    ButtonId, LedId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "video")]
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$")]
pub enum SocketInput {
    #[serde(rename = "b")]
    Button {
        #[serde(rename = "b")]
        button: ButtonId,
        #[serde(rename = "s")]
        state: bool,
    },
    #[cfg(feature = "hid")]
    #[serde(rename = "k")]
    KeyboardKey {
        #[serde(rename = "k")]
        key: Key,
        #[serde(rename = "s")]
        state: bool,
    },
    #[cfg(feature = "hid")]
    #[serde(rename = "m")]
    MouseButton {
        #[serde(rename = "b")]
        button: Button,
        #[serde(rename = "s")]
        state: bool,
    },
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MousePointer { x: i16, y: i16 },
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MouseWheel {
        #[serde(rename = "w")]
        wheel: i8,
    },
}

/// Outgoing message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$")]
pub enum SocketOutput {
    /// Initial state
    #[serde(rename = "s")]
    State {
        /// Led states
        #[serde(rename = "l")]
        leds: HashMap<LedId, bool>,
        /// Button states
        #[serde(rename = "b")]
        buttons: HashMap<ButtonId, bool>,
        /// Keyboard state
        #[cfg(feature = "hid")]
        #[serde(rename = "k")]
        keyboard: Option<KeyboardState>,
        /// Mouse state
        #[cfg(feature = "hid")]
        #[serde(rename = "m")]
        mouse: Option<MouseState>,
    },
    /// LED state change
    #[serde(rename = "l")]
    Led {
        #[serde(rename = "l")]
        led: LedId,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Button state change
    #[serde(rename = "b")]
    Button {
        #[serde(rename = "b")]
        button: ButtonId,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Keyboard key state change
    #[cfg(feature = "hid")]
    #[serde(rename = "k")]
    KeyboardKey {
        #[serde(rename = "k")]
        key: Key,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Keyboard led state change
    #[cfg(feature = "hid")]
    #[serde(rename = "i")]
    KeyboardLed {
        #[serde(rename = "l")]
        led: Led,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Mouse button state change
    #[cfg(feature = "hid")]
    #[serde(rename = "m")]
    MouseButton {
        #[serde(rename = "b")]
        button: Button,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Mouse pointer change
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MousePointer { x: i16, y: i16 },
    /// Mouse wheel change
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MouseWheel {
        #[serde(rename = "w")]
        wheel: i8,
    },
    /// Video frames
    #[cfg(feature = "video")]
    #[serde(rename = "v")]
    VideoFrame {
        #[serde(rename = "f")]
        frame: Arc<Vec<u8>>,
    },
}
