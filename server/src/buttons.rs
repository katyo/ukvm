use crate::{ButtonId, Result};
use gpiod::{Active, Bias, Chip, Drive, LineId, Options};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{spawn, sync::watch};

/// Button interface
pub struct Button {
    state_sender: watch::Sender<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ButtonConfig {
    /// GPIO chip name
    pub chip: String,

    /// GPIO line offset
    pub line: LineId,

    /// GPIO line active state
    #[serde(default)]
    pub active: Active,

    /// GPIO line bias
    #[serde(default)]
    pub bias: Bias,

    /// GPIO line drive
    #[serde(default)]
    pub drive: Drive,
}

impl Button {
    /// Instantiate new button
    pub async fn new(id: ButtonId, config: &ButtonConfig) -> Result<Self> {
        let outputs = Chip::new(&config.chip)
            .await?
            .request_lines(
                Options::output(&[config.line])
                    .active(config.active)
                    .bias(config.bias)
                    .drive(config.drive)
                    .consumer(format!("{}-{}-button", env!("CARGO_PKG_NAME"), id)),
            )
            .await?;

        //let delay = Duration::from_millis(config.delay as _);

        let (state_sender, mut state_receiver) = watch::channel(false);

        spawn({
            async move {
                log::debug!("Initialize receiving events");

                while let Ok(_) = state_receiver.changed().await {
                    // Button state changed
                    let state = *state_receiver.borrow();
                    if let Err(error) = outputs.set_values([state]).await {
                        log::error!("Error when changing state: {}", error);
                        break;
                    }
                }

                log::debug!("Finalize receiving events");
            }
        });

        Ok(Self { state_sender })
    }

    /// Get current state
    pub fn state(&self) -> bool {
        *self.state_sender.borrow()
    }

    /// Subscribe to state changes
    pub fn watch(&self) -> watch::Receiver<bool> {
        self.state_sender.subscribe()
    }

    /// Change button state
    pub fn set_state(&self, state: bool) -> Result<()> {
        self.state_sender
            .send(state)
            .map_err(|_| "Button dropped")?;
        Ok(())
    }
}

/// Buttons control service
#[derive(educe::Educe)]
#[educe(Deref)]
pub struct Buttons {
    /// Buttons
    buttons: HashMap<ButtonId, Button>,
}

/// Buttons configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ButtonsConfig {
    /// Button configurations
    pub buttons: HashMap<ButtonId, ButtonConfig>,
}

impl Buttons {
    /// Create buttons control service using specified config
    pub async fn new(config: &ButtonsConfig) -> Result<Self> {
        let mut buttons = HashMap::default();

        for (id, config) in &config.buttons {
            buttons.insert(*id, Button::new(*id, config).await?);
        }

        Ok(Self { buttons })
    }
}
