use crate::Result;
use hidg::{Class, Device, Keyboard, Mouse, StateChange, ValueChange};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::Path};
use tokio::{
    select, spawn,
    sync::{mpsc, watch},
};

pub use hidg::{Button, Key, Led, MouseInput, MouseInputChange as MouseStateChange};

/// Keyboard key state change event
pub type KeyStateChange = StateChange<Key>;

/// Keyboard LED state change event
pub type LedStateChange = StateChange<Led>;

/// Mouse button state change event
pub type ButtonStateChange = StateChange<Button>;

/// Mouse pointer state change event
pub type PointerValueChange = ValueChange<(i16, i16)>;

/// Mouse wheel state change event
pub type WheelValueChange = ValueChange<i8>;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct HidConfig {
    /// Keyboard device
    pub keyboard: Option<String>,

    /// Mouse device
    pub mouse: Option<String>,
}

pub struct HidIo<C: Class> {
    input_sender: watch::Sender<C::Input>,
    output_receiver: watch::Receiver<C::Output>,
}

impl<C: Class> HidIo<C> {
    async fn new(class: C, path: impl AsRef<Path>) -> Result<Self>
    where
        C: Display + Copy + Send + Sync + 'static,
        C::Input: AsRef<[u8]> + Copy + Send + Sync + 'static,
        C::Output: AsMut<[u8]> + Copy + Send + Sync + core::fmt::Debug + 'static,
    {
        let mut device = Device::<C>::open(path).await?;

        let (input_sender, mut input_receiver) = watch::channel(class.input());
        let (output_sender, output_receiver) = watch::channel(class.output());

        spawn(async move {
            log::debug!("{class}: Initialize receiving events");

            let mut output = class.output();

            loop {
                select! {
                    // device dropped
                    _ = output_sender.closed() => break,
                    result = input_receiver.changed() => match result {
                        // Input report changed
                        Ok(_) => {
                            let report = *input_receiver.borrow();
                            if let Err(error) = device.input(&report).await {
                                log::error!("{class}: Error when sending input: {}", error);
                                break;
                            }
                        }
                        // Disconnected
                        Err(_) => break,
                    },
                    result = device.output(&mut output) => match result {
                        // Output report received
                        Ok(_) => {
                            let report = output;
                            log::trace!("{class}: Report received: {:?}", report);
                            if let Err(error) = output_sender.send(report) {
                                log::error!("{class}: Error when sending state: {}", error);
                                break;
                            }
                        }
                        // Output report receiving error happenned
                        Err(error) => {
                            log::error!("{class}: Error when receiving output: {}", error);
                            break;
                        }
                    },
                }
            }
            log::debug!("{class}: Finalize receiving events");
        });

        Ok(Self {
            input_sender,
            output_receiver,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KeyboardState {
    #[serde(rename = "k")]
    pub keys: Vec<Key>,
    #[serde(rename = "l")]
    pub leds: Vec<Led>,
}

impl HidIo<Keyboard> {
    /// Get pressed keys
    pub fn get_state(&self) -> KeyboardState {
        let keys = self.active_keys();
        let leds = self.active_leds();

        KeyboardState { keys, leds }
    }

    /// Get pressed keys
    pub fn active_keys(&self) -> Vec<Key> {
        self.input_sender.borrow().pressed().collect()
    }

    /// Change key state
    pub async fn change_key(&self, change: KeyStateChange) -> Result<()> {
        let mut report = *self.input_sender.borrow();
        report.change_key(*change, change.state());
        self.input_sender.send_replace(report);
        Ok(())
    }

    /// Watch key state changes
    pub fn watch_keys(&self) -> mpsc::Receiver<KeyStateChange> {
        let mut old_report = *self.input_sender.borrow();
        let mut input_receiver = self.input_sender.subscribe();
        let (event_sender, event_receiver) = mpsc::channel(10);

        spawn(async move {
            loop {
                if let Err(error) = input_receiver.changed().await {
                    log::error!("Error when receiving key state changes: {}", error);
                    break;
                } else {
                    // New report is set
                    let new_report = *input_receiver.borrow();
                    for change in &new_report - &old_report {
                        log::trace!(
                            "Key {:?} is {}",
                            &*change,
                            if change.state() {
                                "pressed"
                            } else {
                                "released"
                            }
                        );
                        if let Err(error) = event_sender.send(change).await {
                            log::error!("Error when sending key state change: {}", error);
                            break;
                        }
                    }
                    old_report = new_report;
                }
            }
        });

        event_receiver
    }

    /// Get lit LEDs
    pub fn active_leds(&self) -> Vec<Led> {
        self.output_receiver.borrow().lit().collect()
    }

    /// Watch LEDs state chsnges
    pub fn watch_leds(&self) -> mpsc::Receiver<LedStateChange> {
        let mut old_report = *self.output_receiver.borrow();
        let mut output_receiver = self.output_receiver.clone();
        let (event_sender, event_receiver) = mpsc::channel(10);

        spawn(async move {
            loop {
                if let Err(error) = output_receiver.changed().await {
                    log::error!("Error when receiving LED state changes: {}", error);
                    break;
                } else {
                    // LED state change received
                    let new_report = *output_receiver.borrow();
                    for change in &new_report - &old_report {
                        log::trace!(
                            "LED {:?} is {}",
                            &*change,
                            if change.is_on() { "on" } else { "off" }
                        );
                        if let Err(error) = event_sender.send(change).await {
                            log::error!("Error when sending LED state change: {}", error);
                            break;
                        }
                    }
                    old_report = new_report;
                }
            }
        });

        event_receiver
    }
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

impl HidIo<Mouse> {
    /// Get pressed buttons
    pub fn get_state(&self) -> MouseState {
        (&*self.input_sender.borrow()).into()
    }

    /// Change mouse state state
    pub async fn change_state(&self, change: MouseStateChange) -> Result<()> {
        let mut report = *self.input_sender.borrow();
        report.change(&change);
        self.input_sender.send_replace(report);
        Ok(())
    }

    /// Watch mouse state changes
    pub fn watch_state(&self) -> mpsc::Receiver<MouseStateChange> {
        let mut old_report = *self.input_sender.borrow();
        let mut input_receiver = self.input_sender.subscribe();
        let (event_sender, event_receiver) = mpsc::channel(10);

        spawn(async move {
            loop {
                if let Err(error) = input_receiver.changed().await {
                    log::error!("Error when receiving mouse state changes: {}", error);
                    break;
                } else {
                    // New report is set
                    let new_report = *input_receiver.borrow();
                    for change in &new_report - &old_report {
                        log::trace!("Mouse state change {:?}", change);
                        if let Err(error) = event_sender.send(change.into()).await {
                            log::error!("Error when sending mouse state change: {}", error);
                            break;
                        }
                    }
                    old_report = new_report;
                }
            }
        });

        event_receiver
    }
}

pub struct Hid {
    keyboard: Option<HidIo<Keyboard>>,
    mouse: Option<HidIo<Mouse>>,
}

impl Hid {
    /// Crate HID devices from config
    pub async fn new(config: &HidConfig) -> Result<Self> {
        let keyboard = if let Some(keyboard) = &config.keyboard {
            Some(HidIo::new(Keyboard, keyboard).await?)
        } else {
            None
        };

        let mouse = if let Some(mouse) = &config.keyboard {
            Some(HidIo::new(Mouse, mouse).await?)
        } else {
            None
        };

        Ok(Self { keyboard, mouse })
    }

    /// Get access to keyboard
    pub fn keyboard(&self) -> Option<&HidIo<Keyboard>> {
        self.keyboard.as_ref()
    }

    /// Get access to mouse
    pub fn mouse(&self) -> Option<&HidIo<Mouse>> {
        self.mouse.as_ref()
    }
}
