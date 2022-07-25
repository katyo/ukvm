use crate::Result;
use gpiod::{Active, Bias, Chip, Edge, EdgeDetect, LineId, Options};
use serde::{Deserialize, Serialize};
use slab::Slab;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    select, spawn,
    sync::{mpsc, watch},
};

struct LedState {
    status: AtomicBool,
    subscriber: mpsc::Sender<watch::Sender<bool>>,
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
    pub async fn new(config: &LedConfig) -> Result<Self> {
        let (subscriber, mut subscriptions) = mpsc::channel(8);
        let mut inputs = Chip::new(&config.chip)
            .await?
            .request_lines(
                Options::input(&[config.line])
                    .active(config.active)
                    .bias(config.bias)
                    .edge(EdgeDetect::Both),
            )
            .await?;

        let state = Arc::new(LedState {
            status: AtomicBool::new(inputs.get_values([false]).await?[0]),
            subscriber,
        });

        spawn({
            let state = state.clone();
            async move {
                let mut listeners = Slab::with_capacity(8);

                log::debug!("Initialize receiving events");

                loop {
                    select! {
                        result = inputs.read_event() => match result {
                            // Edge event received
                            Ok(event) => {
                                log::trace!("Event received: {}", event);
                                let status = if matches!(event.edge, Edge::Rising) {
                                    true
                                } else {
                                    false
                                };
                                state.status.store(status, Ordering::Relaxed);
                            }
                            // Input error happenned
                            Err(error) => {
                                log::error!("Error when receiving event: {}", error);
                                break;
                            }
                        },
                        action = subscriptions.recv() => match action {
                            // New listener received
                            Some(listener) => {
                                log::debug!("Add listener");
                                listeners.insert(listener);
                            }
                            // Led object dropped
                            None => {
                                log::debug!("Finalize receiving events");
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
            }
        });

        Ok(Self { state })
    }

    /// Get current status of LED
    pub fn status(&self) -> bool {
        self.state.status.load(Ordering::Relaxed)
    }

    /// Subscribe to status changes
    pub async fn listen(&self) -> Result<watch::Receiver<bool>> {
        let (sender, receiver) = watch::channel(self.status());
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
    pub leds: HashMap<LedType, LedConfig>,
}

/// LED type
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LedType {
    /// Power status LED
    Power,

    /// Disk usage LED
    Disk,

    /// Ethernet usage LED
    Ether,
}

/// LEDs control service
pub struct Leds {
    /// LEDs
    leds: HashMap<LedType, Led>,
}

impl Leds {
    /// Create LEDs status service using specified config
    pub async fn new(config: &LedsConfig) -> Result<Self> {
        let mut leds = HashMap::default();

        for (type_, config) in &config.leds {
            leds.insert(*type_, Led::new(config).await?);
        }

        Ok(Self { leds })
    }

    /// Get current status of specified LED
    pub fn status(&self, type_: LedType) -> Option<bool> {
        self.leds.get(&type_).map(|led| led.status())
    }

    /// Listen LEDs status
    pub async fn listen(&self, type_: LedType) -> Result<Option<watch::Receiver<bool>>> {
        Ok(if let Some(led) = self.leds.get(&type_) {
            Some(led.listen().await?)
        } else {
            None
        })
    }
}
