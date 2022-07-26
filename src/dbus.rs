use crate::{ButtonId, Error, LedId, Result, Server, ServerEvent};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{spawn, sync::Semaphore};
use tokio_stream::StreamExt;
use zbus::{dbus_interface, Address, ConnectionBuilder, SignalContext};

/// DBus service binding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "bind", rename_all = "lowercase")]
pub enum DBusBind {
    /// System bus
    System,

    /// Session bus
    Session,

    /// TCP socket address
    #[serde(rename = "tcp")]
    Addr(SocketAddr),

    #[serde(rename = "unix")]
    /// Unix socket path
    Path(PathBuf),
}

impl zbus::DBusError for Error {
    fn create_reply(&self, msg: &zbus::MessageHeader<'_>) -> zbus::Result<zbus::Message> {
        zbus::MessageBuilder::error(msg, self.name())?.build(&self.to_string())
    }

    fn name(&self) -> zbus::names::ErrorName<'_> {
        zbus::names::ErrorName::from_str_unchecked(self.as_ref())
    }

    fn description(&self) -> Option<&str> {
        Some(self.as_ref())
    }
}

/*impl From<Error> for zbus::fdo::Error {
    fn from(error: Error) -> Self {}
}*/

macro_rules! dbus {
    (name $($name:ident).+) => {
        concat!("org.", env!("CARGO_PKG_NAME"), ".", stringify!($($name).+))
    };

    (path $($name:ident)/+ ) => {
        concat!("/org/", env!("CARGO_PKG_NAME"), "/", stringify!($($name)/+))
    };
}

struct Buttons(Server);

#[dbus_interface(name = "org.ubc.Buttons")]
impl Buttons {
    /// List present buttons
    #[dbus_interface(property)]
    fn list(&self) -> Vec<ButtonId> {
        self.0.buttons().collect()
    }

    /// Press specific button
    async fn press(&self, button: ButtonId) -> Result<()> {
        Ok(self.0.button_press(button).await?)
    }

    /// Button pressed
    #[dbus_interface(signal)]
    async fn pressed(signal_ctx: &SignalContext<'_>, id: ButtonId) -> zbus::Result<()>;
}

struct Leds(Server);

#[dbus_interface(name = "org.ubc.Leds")]
impl Leds {
    /// List present LEDs
    #[dbus_interface(property)]
    fn list(&self) -> Vec<LedId> {
        self.0.leds().collect()
    }

    /// Get status of specific LED
    fn status(&self, led: LedId) -> Result<bool> {
        Ok(self.0.led_status(led)?)
    }

    /// LEDs status changes
    #[dbus_interface(signal)]
    async fn changed(signal_ctx: &SignalContext<'_>, id: LedId, status: bool) -> zbus::Result<()>;
}

impl Server {
    pub async fn spawn_dbus(&self, bind: &DBusBind, stop: &Arc<Semaphore>) -> Result<()> {
        let stop = stop.clone();

        let builder = match bind {
            DBusBind::System => ConnectionBuilder::system()?,
            DBusBind::Session => ConnectionBuilder::session()?,
            DBusBind::Addr(addr) => ConnectionBuilder::address(
                format!(
                    "tcp:host={},port={},family=ipv{}",
                    addr.ip(),
                    addr.port(),
                    if addr.is_ipv4() { '4' } else { '6' }
                )
                .parse::<Address>()?,
            )?,
            DBusBind::Path(path) => ConnectionBuilder::address(
                format!("unix:path={}", path.display()).parse::<Address>()?,
            )?,
        };

        let connection = builder
            .name(dbus!(name Control))?
            .serve_at(dbus!(path buttons), Buttons(self.clone()))?
            .serve_at(dbus!(path leds), Leds(self.clone()))?
            .build()
            .await?;

        let events = spawn({
            let connection = connection.clone();
            let server = self.clone();

            async move {
                let mut events = server.events().await?;
                let object_server = connection.object_server();

                let leds_ref = object_server.interface::<_, Leds>(dbus!(path leds)).await?;

                let buttons_ref = object_server
                    .interface::<_, Buttons>(dbus!(path buttons))
                    .await?;

                while let Some(event) = events.next().await {
                    match event {
                        ServerEvent::LedStatus { id, status } => {
                            Leds::changed(leds_ref.signal_context(), id, status).await?;
                        }
                        ServerEvent::ButtonPress { id } => {
                            Buttons::pressed(buttons_ref.signal_context(), id).await?;
                        }
                    }
                }

                Ok::<(), Error>(())
            } /*
              .then(|result| {
                  if let Err(error) = result {
                      log::error!("Error while events processing: {}", error);
                  }
              })*/
        });

        spawn(async move {
            log::debug!("Await signal to stop");
            let lock = stop.acquire().await;
            log::debug!("Received stop signal");

            events.abort();

            drop(connection);
            log::info!("Stopped");
            drop(lock);
        });

        log::info!("Started");

        Ok(())
    }
}
