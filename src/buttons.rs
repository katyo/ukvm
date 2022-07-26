use crate::Result;
use gpiod::{Active, Bias, Chip, Drive, LineId, Lines, Options, Output};
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{
    sync::RwLock,
    time::{sleep, Duration},
};
#[cfg(feature = "zbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

struct ButtonState {
    outputs: RwLock<Lines<Output>>,
    delay: Duration,
}

/// Button interface
pub struct Button {
    state: ButtonState,
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

    /// Press duration (milliseconds)
    #[serde(default = "default_delay")]
    pub delay: u32,
}

const fn default_delay() -> u32 {
    250
}

impl Button {
    /// Instantiate new button
    pub async fn new(id: ButtonId, config: &ButtonConfig) -> Result<Self> {
        let state = ButtonState {
            outputs: RwLock::new(
                Chip::new(&config.chip)
                    .await?
                    .request_lines(
                        Options::output(&[config.line])
                            .active(config.active)
                            .bias(config.bias)
                            .drive(config.drive)
                            .consumer(format!("{}-{}-button", env!("CARGO_PKG_NAME"), id)),
                    )
                    .await?,
            ),
            delay: Duration::from_millis(config.delay as _),
        };

        Ok(Self { state })
    }

    /// Simulate button press
    pub async fn press(&self) -> Result<()> {
        let outputs = self.state.outputs.write().await;
        outputs.set_values([true]).await?;
        sleep(self.state.delay).await;
        outputs.set_values([false]).await?;
        Ok(())
    }
}

/// Button type
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    FromStr,
    Display,
)]
#[cfg_attr(feature = "zbus", derive(Type, Value, OwnedValue))]
#[cfg_attr(feature = "zbus", zvariant(signature = "s"))]
#[serde(rename_all = "kebab-case")]
#[display(style = "kebab-case")]
pub enum ButtonId {
    /// System power button
    Power = 1,

    /// System reset button
    Reset = 2,

    /// Clear CMOS button
    Clear = 3,
}

/// Buttons control service
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

    /// Get present buttons
    pub fn list<'a>(&'a self) -> impl Iterator<Item = ButtonId> + 'a {
        self.buttons.keys().copied()
    }

    /// Simulate button press
    pub async fn press(&self, id: ButtonId) -> Result<bool> {
        Ok(if let Some(button) = self.buttons.get(&id) {
            button.press().await?;
            true
        } else {
            false
        })
    }
}
