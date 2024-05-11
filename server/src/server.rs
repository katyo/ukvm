use crate::{Bind, Buttons, ButtonsConfig, Leds, LedsConfig, Result, log};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Weak},
};

#[cfg(feature = "hid")]
use crate::{Hid, HidConfig};

#[cfg(feature = "video")]
use crate::{Video, VideoConfig};

/// Server configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Service bindings
    #[serde(default)]
    pub binds: Vec<Bind>,

    /// GPIO buttons
    #[serde(default)]
    pub buttons: ButtonsConfig,

    /// GPIO LEDs
    #[serde(default)]
    pub leds: LedsConfig,

    /// HID devices
    #[cfg(feature = "hid")]
    #[serde(default)]
    pub hid: Option<HidConfig>,

    /// Video device
    #[cfg(feature = "video")]
    #[serde(default)]
    pub video: Option<VideoConfig>,
}

impl ServerConfig {
    /// Read config from file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let raw: Vec<u8> = tokio::fs::read(path).await?;
        let utf = core::str::from_utf8(&raw)?;
        let cfg = toml::from_str::<serde_json::Value>(utf)?;
        let cfg = serde_json::from_value(cfg)?;

        Ok(cfg)
    }
}

struct ServerState {
    /// Buttons
    buttons: Buttons,

    /// LEDs
    leds: Leds,

    /// HID devices
    #[cfg(feature = "hid")]
    hid: Option<Hid>,

    /// Video device
    #[cfg(feature = "video")]
    video: Option<Video>,
}

/// Server instance
#[derive(Clone)]
pub struct Server {
    state: Arc<ServerState>,
}

/// Weak reference to server
#[derive(Clone)]
pub struct ServerRef {
    state: Weak<ServerState>,
}

impl ServerRef {
    /// Try to get server instance by weak reference
    pub fn upgrade(&self) -> Result<Server> {
        Ok(Server {
            state: self
                .state
                .upgrade()
                .ok_or_else(|| "Seems server is out of life")?,
        })
    }
}

impl Server {
    /// Instantiate server using provided config
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        let buttons = Buttons::new(&config.buttons).await?;
        let leds = Leds::new(&config.leds).await?;

        #[cfg(feature = "hid")]
        let hid = if let Some(hid) = &config.hid {
            log::info!("Setup HID input");
            Some(Hid::new(hid).await?)
        } else {
            log::info!("No HID input");
            None
        };

        #[cfg(feature = "video")]
        let video = if let Some(video) = &config.video {
            log::info!("Setup video capturing");
            Some(Video::new(video).await?)
        } else {
            log::info!("No video capturing");
            None
        };

        Ok(Self {
            state: Arc::new(ServerState {
                buttons,
                leds,
                #[cfg(feature = "hid")]
                hid,
                #[cfg(feature = "video")]
                video,
            }),
        })
    }

    /// Get weak ref to server
    pub fn downgrade(&self) -> ServerRef {
        ServerRef {
            state: Arc::downgrade(&self.state),
        }
    }

    /// Get LEDs
    pub fn leds(&self) -> &Leds {
        &self.state.leds
    }

    /// Get buttons
    pub fn buttons(&self) -> &Buttons {
        &self.state.buttons
    }

    /// Get HID devices
    #[cfg(feature = "hid")]
    pub fn hid(&self) -> Option<&Hid> {
        self.state.hid.as_ref()
    }

    /// Get video device
    #[cfg(feature = "video")]
    pub fn video(&self) -> Option<&Video> {
        self.state.video.as_ref()
    }
}
