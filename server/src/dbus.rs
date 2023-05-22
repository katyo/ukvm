use crate::{ButtonId, Error, LedId, Result, Server};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{spawn, sync::Semaphore};
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

struct Button {
    id: ButtonId,
    server: Server,
}

#[dbus_interface(name = "org.ukvm.Button")]
impl Button {
    /// Button id
    #[dbus_interface(property)]
    fn id(&self) -> ButtonId {
        self.id
    }

    /// Current state
    fn state(&self) -> bool {
        self.server.buttons().get(&self.id).unwrap().state()
    }

    /// Change state
    fn set_state(&self, state: bool) -> Result<()> {
        Ok(self
            .server
            .buttons()
            .get(&self.id)
            .unwrap()
            .set_state(state)?)
    }

    /// State changed
    #[dbus_interface(signal)]
    async fn state_changed(signal_ctx: &SignalContext<'_>, state: bool) -> zbus::Result<()>;
}

struct Led {
    id: LedId,
    server: Server,
}

#[dbus_interface(name = "org.ukvm.Led")]
impl Led {
    /// LED id
    #[dbus_interface(property)]
    fn id(&self) -> LedId {
        self.id
    }

    /// Current state
    fn state(&self) -> bool {
        self.server.leds().get(&self.id).unwrap().state()
    }

    /// State changed
    #[dbus_interface(signal)]
    async fn state_changed(signal_ctx: &SignalContext<'_>, state: bool) -> zbus::Result<()>;
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

        let mut builder = builder.name("org.ukvm.Control")?;

        for id in self.buttons().keys().copied() {
            builder = builder.serve_at(
                format!("/org/ukvm/button/{}", id),
                Button {
                    id,
                    server: self.clone(),
                },
            )?;
        }

        for id in self.leds().keys().copied() {
            builder = builder.serve_at(
                format!("/org/ukvm/led/{}", id),
                Led {
                    id,
                    server: self.clone(),
                },
            )?;
        }

        let connection = builder.build().await?;

        for (id, inst) in self.buttons().iter() {
            let mut watch = inst.watch();
            let reference = connection
                .object_server()
                .interface::<_, Button>(format!("/org/ukvm/button/{}", id))
                .await?;
            spawn(async move {
                while let Ok(_) = watch.changed().await {
                    let state = *watch.borrow();
                    let s_ctx = reference.signal_context();
                    if let Err(error) = Button::state_changed(s_ctx, state).await {
                        log::error!("Error notifying button state change: {}", error);
                    }
                }
            });
        }

        for (id, inst) in self.leds().iter() {
            let mut watch = inst.watch();
            let reference = connection
                .object_server()
                .interface::<_, Led>(format!("/org/ukvm/led/{}", id))
                .await?;
            spawn(async move {
                while let Ok(_) = watch.changed().await {
                    let state = *watch.borrow();
                    let s_ctx = reference.signal_context();
                    if let Err(error) = Led::state_changed(s_ctx, state).await {
                        log::error!("Error notifying LED state change: {}", error);
                    }
                }
            });
        }

        spawn(async move {
            log::debug!("Await signal to stop");
            let lock = stop.acquire().await;
            log::debug!("Received stop signal");

            drop(connection);
            log::info!("Stopped");
            drop(lock);
        });

        log::info!("Started");

        Ok(())
    }
}
