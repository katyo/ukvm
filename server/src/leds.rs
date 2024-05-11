use crate::{log, LedId, Result};
use gpiod::{Active, Bias, Chip, Edge, EdgeDetect, LineId, Options};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{select, spawn, sync::watch};

/// Single LED
pub struct Led {
    state_receiver: watch::Receiver<bool>,
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

        let (state_sender, state_receiver) = watch::channel(inputs.get_values([false]).await?[0]);

        spawn(async move {
            log::debug!("{id}: Initialize receiving events");

            loop {
                select! {
                    // LED object dropped
                    _ = state_sender.closed() => break,
                    result = inputs.read_event() => match result {
                        // Edge event received
                        Ok(event) => {
                            log::trace!("{id}: Event received: {}", event);
                            let state = matches!(event.edge, Edge::Rising);
                            if let Err(error) = state_sender.send(state) {
                                log::error!("{id}: Error when sending state: {}", error);
                                break;
                            }
                        }
                        // Input error happenned
                        Err(error) => {
                            log::error!("{id}: Error when receiving event: {}", error);
                            break;
                        }
                    },
                }
            }
            log::debug!("{id}: Finalize receiving events");
        });

        Ok(Self { state_receiver })
    }

    /// Get current state
    pub fn state(&self) -> bool {
        *self.state_receiver.borrow()
    }

    /// Watch state changes
    pub fn watch(&self) -> watch::Receiver<bool> {
        self.state_receiver.clone()
    }
}

/// LEDs configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LedsConfig {
    /// LED configurations
    pub leds: HashMap<LedId, LedConfig>,
}

/// LEDs control service
#[derive(educe::Educe)]
#[educe(Deref)]
pub struct Leds {
    /// LEDs
    leds: HashMap<LedId, Led>,
}

impl Leds {
    /// Create LEDs state service using specified config
    pub async fn new(config: &LedsConfig) -> Result<Self> {
        let mut leds = HashMap::default();

        for (id, config) in &config.leds {
            leds.insert(*id, Led::new(*id, config).await?);
        }

        Ok(Self { leds })
    }
}
