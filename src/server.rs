use crate::{Buttons, ButtonsConfig, Leds, LedsConfig, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Server configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServerConfig {
    /// GPIO buttons
    pub buttons: ButtonsConfig,

    /// GPIO LEDs
    pub leds: LedsConfig,
}

impl ServerConfig {
    /// Read config from file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let raw: Vec<u8> = tokio::fs::read(path).await?;

        let cfg = toml::de::from_slice(&raw)?;

        Ok(cfg)
    }
}

pub struct Server {
    buttons: Buttons,

    leds: Leds,
}

impl Server {
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        let buttons = Buttons::new(&config.buttons).await?;
        let leds = Leds::new(&config.leds).await?;

        Ok(Self { buttons, leds })
    }
}
