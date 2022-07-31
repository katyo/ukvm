use crate::Result;
use gpiod::{Active, Bias, Chip, Edge, EdgeDetect, LineId, Options};
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use slab::Slab;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{select, spawn, sync::mpsc};
#[cfg(feature = "zbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

struct LedState {
    status: AtomicBool,
    subscriber: mpsc::Sender<mpsc::Sender<bool>>,
}

/// Single LED
pub struct Led {
    state: Arc<LedState>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LedConfig {
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
}

impl Led {
    pub async fn new(id: LedId, config: &LedConfig) -> Result<Self> {
        let (subscriber, mut subscriptions) = mpsc::channel(8);
        let mut inputs = Chip::new(&config.chip)
            .await?
            .request_lines(
                Options::input(&[config.line])
                    .active(config.active)
                    .bias(config.bias)
                    .edge(EdgeDetect::Both)
                    .consumer(format!("{}-{}-led", env!("CARGO_PKG_NAME"), id)),
            )
            .await?;

        let state = Arc::new(LedState {
            status: AtomicBool::new(inputs.get_values([false]).await?[0]),
            subscriber,
        });

        spawn({
            let state_ref = Arc::downgrade(&state);
            async move {
                let mut listeners = Slab::with_capacity(8);

                log::debug!("Initialize receiving events");

                loop {
                    select! {
                        action = subscriptions.recv() => match action {
                            // New listener received
                            Some(listener) => {
                                log::debug!("Add listener");
                                if let Some(state) = state_ref.upgrade() {
                                    let _ = listener.try_send(state.status.load(Ordering::SeqCst));
                                    listeners.insert(listener);
                                } else {
                                    // Seems LED object dropped
                                    break;
                                }
                            }
                            // LED object dropped
                            None => {
                                break;
                            }
                        },
                        result = inputs.read_event() => match result {
                            // Edge event received
                            Ok(event) => {
                                log::trace!("Event received: {}", event);
                                let status = if matches!(event.edge, Edge::Rising) {
                                    true
                                } else {
                                    false
                                };
                                if let Some(state) = state_ref.upgrade() {
                                    state.status.store(status, Ordering::SeqCst);
                                    // Send current status to all listeners
                                    for (_, listener) in &listeners {
                                        if listener.is_closed() {
                                            continue;
                                        }
                                        let _ = listener.try_send(status);
                                    }
                                } else {
                                    // Seems LED object dropped
                                    break;
                                }
                            }
                            // Input error happenned
                            Err(error) => {
                                log::error!("Error when receiving event: {}", error);
                                break;
                            }
                        },
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
                log::debug!("Finalize receiving events");
            }
        });

        Ok(Self { state })
    }

    /// Get current status of LED
    pub fn status(&self) -> bool {
        self.state.status.load(Ordering::SeqCst)
    }

    /// Subscribe to status changes
    pub async fn listen(&self) -> Result<mpsc::Receiver<bool>> {
        let (sender, receiver) = mpsc::channel(4);
        let result = self.state.subscriber.send(sender).await;
        if let Err(error) = &result {
            log::error!("Error when subscribing to status: {}", error);
        }
        result?;
        Ok(receiver)
    }
}

/// LEDs configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LedsConfig {
    /// LED configurations
    pub leds: HashMap<LedId, LedConfig>,
}

/// LED type
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    FromStr,
    Display,
)]
#[cfg_attr(feature = "zbus", derive(Type, Value, OwnedValue))]
#[cfg_attr(feature = "zbus", zvariant(signature = "s"))]
#[serde(rename_all = "kebab-case")]
#[display(style = "kebab-case")]
pub enum LedId {
    /// Power status LED
    Power = 1,

    /// Disk usage LED
    Disk = 2,

    /// Ethernet usage LED
    Ether = 3,
}

/// LEDs control service
pub struct Leds {
    /// LEDs
    leds: HashMap<LedId, Led>,
}

impl Leds {
    /// Create LEDs status service using specified config
    pub async fn new(config: &LedsConfig) -> Result<Self> {
        let mut leds = HashMap::default();

        for (id, config) in &config.leds {
            leds.insert(*id, Led::new(*id, config).await?);
        }

        Ok(Self { leds })
    }

    /// Get present LEDs
    pub fn list<'a>(&'a self) -> impl Iterator<Item = LedId> + 'a {
        self.leds.keys().copied()
    }

    /// Get current status of specified LED
    pub fn status(&self, id: LedId) -> Option<bool> {
        self.leds.get(&id).map(|led| led.status())
    }

    /// Listen LEDs status
    pub async fn listen(&self, id: LedId) -> Result<Option<mpsc::Receiver<bool>>> {
        Ok(if let Some(led) = self.leds.get(&id) {
            Some(led.listen().await?)
        } else {
            None
        })
    }
}
