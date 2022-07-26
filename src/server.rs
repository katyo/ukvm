use crate::{Bind, ButtonId, Buttons, ButtonsConfig, LedId, Leds, LedsConfig, Result};
use serde::{Deserialize, Serialize};
use slab::Slab;
use std::{path::Path, sync::Arc};
use tokio::{spawn, sync::mpsc};
use tokio_stream::{
    wrappers::{ReceiverStream, WatchStream},
    Stream, StreamExt,
};

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

enum ServerAction {
    Subscribe(mpsc::Sender<ServerEvent>),
    EmitEvent(ServerEvent),
}

struct ServerState {
    sender: mpsc::Sender<ServerAction>,

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

        let (sender, mut receiver) = mpsc::channel(8);

        spawn(async move {
            let mut listeners = Slab::with_capacity(8);

            log::debug!("Initialize receiving events");

            loop {
                let action = receiver.recv().await;

                match action {
                    // New listener received
                    Some(ServerAction::Subscribe(listener)) => {
                        log::debug!("Add listener");
                        listeners.insert(listener);
                    }
                    // Server event received
                    Some(ServerAction::EmitEvent(event)) => {
                        log::trace!("Event received: {:?}", event);
                        // Send current status to all listeners
                        for (_, listener) in &listeners {
                            if listener.is_closed() {
                                continue;
                            }
                            let _ = listener.send(event.clone()).await;
                        }
                    }
                    // Server state dropped
                    None => {
                        log::debug!("Finalize receiving events");
                        break;
                    }
                }

                // Remove already closed listeners
                listeners.retain(|_, listener| {
                    let remove = listener.is_closed();
                    if remove {
                        log::debug!("Remove listener");
                    }
                    !remove
                });
            }
        });

        Ok(Self {
            state: Arc::new(ServerState {
                sender,
                buttons,
                leds,
            }),
        })
    }

    /// Emit internal server event
    async fn emit(&self, event: ServerEvent) -> Result<()> {
        self.state
            .sender
            .send(ServerAction::EmitEvent(event))
            .await?;
        Ok(())
    }

    /// Subscribe to internal server events
    async fn subscribe(&self) -> Result<mpsc::Receiver<ServerEvent>> {
        let (sender, receiver) = mpsc::channel(8);

        self.state
            .sender
            .send(ServerAction::Subscribe(sender))
            .await?;

        Ok(receiver)
    }

    /// Get capabilities
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            buttons: self.state.buttons.list().collect(),
            leds: self.state.leds.list().collect(),
        }
    }

    /// Get present buttons
    pub fn buttons<'a>(&'a self) -> impl Iterator<Item = ButtonId> + 'a {
        self.state.buttons.list()
    }

    /// Press specific button
    pub async fn button_press(&self, id: ButtonId) -> Result<()> {
        if self.state.buttons.press(id).await? {
            self.emit(ServerEvent::ButtonPress { id }).await?;
            Ok(())
        } else {
            Err("Button doesn't present".into())
        }
    }

    /// Get present LEDs
    pub fn leds<'a>(&'a self) -> impl Iterator<Item = LedId> + 'a {
        self.state.leds.list()
    }

    /// Get status of specific LED
    pub fn led_status(&self, led: LedId) -> Result<bool> {
        Ok(self.state.leds.status(led).ok_or("LED doesn't present")?)
    }

    /// Get server events stream
    pub async fn events(&self) -> Result<impl Stream<Item = ServerEvent>> {
        // Get LED events receivers and convert into server events streams
        let led_listens = self.state.leds.list().map(|id| async move {
            self.state.leds.listen(id).await.map(|receiver| {
                WatchStream::new(receiver.unwrap())
                    .map(move |status| ServerEvent::LedStatus { id, status })
            })
        });

        // Await all of them
        let led_listens = futures::future::try_join_all(led_listens).await?;

        // Merge server events streams together
        let events = futures::stream::select_all(led_listens);

        // Mixin internal server events
        let events = events.merge(ReceiverStream::new(self.subscribe().await?));

        Ok(events)
    }
}

/// Server capabilities
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "zbus", derive(zbus::zvariant::Type))]
#[serde(rename_all = "snake_case")]
pub struct Capabilities {
    /// Present buttons
    pub buttons: Vec<ButtonId>,

    /// Present LEDs
    pub leds: Vec<LedId>,
}

/// Server events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$", rename_all = "snake_case")]
pub enum ServerEvent {
    /// Button pressed
    ButtonPress { id: ButtonId },

    /// LED status changed
    LedStatus { id: LedId, status: bool },
}
