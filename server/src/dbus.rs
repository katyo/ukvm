use crate::{log, ButtonId, DBusAddr, Error, GracefulShutdown, LedId, Result, Server};
use tokio::spawn;
use zbus::{dbus_interface, Address, ConnectionBuilder, SignalContext};

struct Button {
    id: ButtonId,
    server: Server,
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
        self.server
            .buttons()
            .get(&self.id)
            .unwrap()
            .set_state(state)
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
    pub async fn spawn_dbus(&self, addr: &DBusAddr, gs: &GracefulShutdown) -> Result<()> {
        let gs = gs.clone();

        let builder = match addr {
            DBusAddr::System => ConnectionBuilder::system()?,
            DBusAddr::Session => ConnectionBuilder::session()?,
            DBusAddr::Addr(addr) => ConnectionBuilder::address(
                format!(
                    "tcp:host={},port={},family=ipv{}",
                    addr.ip(),
                    addr.port(),
                    if addr.is_ipv4() { '4' } else { '6' }
                )
                .parse::<Address>()?,
            )?,
            DBusAddr::Path(path) => ConnectionBuilder::address(
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
                while watch.changed().await.is_ok() {
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
                while watch.changed().await.is_ok() {
                    let state = *watch.borrow();
                    let s_ctx = reference.signal_context();
                    if let Err(error) = Led::state_changed(s_ctx, state).await {
                        log::error!("Error notifying LED state change: {}", error);
                    }
                }
            });
        }

        spawn(async move {
            let _ = gs.shutdowned().await;
            drop(connection);
            log::info!("Stopped");
        });

        log::info!("Started");

        Ok(())
    }
}
