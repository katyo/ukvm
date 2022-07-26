use crate::{ButtonType, Buttons, ButtonsConfig, LedType, Leds, LedsConfig, Result};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tokio_stream::{wrappers::WatchStream, Stream, StreamExt};

/// Server configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServerConfig {
    /// GPIO buttons
    #[serde(default)]
    pub buttons: ButtonsConfig,

    /// GPIO LEDs
    #[serde(default)]
    pub leds: LedsConfig,
}

impl ServerConfig {
    /// Read config from file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let raw: Vec<u8> = tokio::fs::read(path).await?;

        let cfg = toml::de::from_slice::<serde_json::Value>(&raw)?;
        let cfg = serde_json::from_value(cfg)?;

        Ok(cfg)
    }
}

struct ServerState {
    //sender: mpsc::Sender<Server>
    /// Buttons
    buttons: Buttons,

    /// LEDs
    leds: Leds,
}

/// Server instance
#[derive(Clone)]
pub struct Server {
    state: Arc<ServerState>,
}

impl Server {
    /// Instantiate server using provided config
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        let buttons = Buttons::new(&config.buttons).await?;
        let leds = Leds::new(&config.leds).await?;

        Ok(Self {
            state: Arc::new(ServerState { buttons, leds }),
        })
    }

    /// Get capabilities
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            buttons: self.state.buttons.list().collect(),
            leds: self.state.leds.list().collect(),
        }
    }

    /// Press specific button
    pub async fn button_press(&self, button: ButtonType) -> Result<()> {
        self.state.buttons.press(button).await?;
        Ok(())
    }

    /// Get status of specific LED
    pub async fn led_status(&self, led: LedType) -> Result<bool> {
        Ok(self.state.leds.status(led).unwrap_or(false))
    }

    /// Server events stream
    pub async fn events(&self) -> Result<impl Stream<Item = ServerEvent>> {
        // Get LED events receivers and convert into server events streams
        let led_listens = self.state.leds.list().map(|led| async move {
            self.state.leds.listen(led).await.map(|receiver| {
                WatchStream::new(receiver.unwrap())
                    .map(move |state| ServerEvent::Led { led, state })
            })
        });

        // Await all of them
        let led_listens = futures::future::try_join_all(led_listens).await?;

        // Merge server events streams together
        Ok(futures::stream::select_all(led_listens))
    }
}

/// Server capabilities
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Capabilities {
    /// Present buttons
    pub buttons: Vec<ButtonType>,

    /// Present LEDs
    pub leds: Vec<LedType>,
}

/// Server events
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "$", rename_all = "snake_case")]
pub enum ServerEvent {
    /// Button pressed
    Button { button: ButtonType },

    /// LED status changed
    Led { led: LedType, state: bool },
}
