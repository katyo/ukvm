use crate::{Result, Server};
use dbus_async::{Binder, DBus, DBusResult};
use dbus_message_parser::{message::Message, value::Value};
use std::{convert::TryInto, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{spawn, sync::Semaphore};

/// DBus service binding
#[derive(Debug, Clone)]
pub enum DBusBind {
    /// System bus
    System,

    /// Session bus
    Session,

    /// TCP socket address
    Addr(SocketAddr),

    /// Unix socket path
    Path(PathBuf),
}

#[async_trait::async_trait]
impl dbus_async::Handler for Server {
    async fn handle(&mut self, dbus: &DBus, msg: Message) -> DBusResult<()> {
        println!("Got message {:?}", msg);
        if let Ok(mut msg) = msg.method_return() {
            msg.add_value(Value::String("Hello world".to_string()));
            println!("Response: Hello world");
            dbus.send(msg)?;
        }
        Ok(())
    }
}

impl Server {
    pub async fn spawn_dbus(&self, bind: &DBusBind, stop: &Arc<Semaphore>) -> Result<()> {
        let introspectable = true;
        let peer = true;

        let stop = stop.clone();

        let (dbus, handle) = match bind {
            DBusBind::System => DBus::system(introspectable, peer).await?,
            DBusBind::Session => DBus::session(introspectable, peer).await?,
            DBusBind::Addr(addr) => {
                DBus::new(
                    &format!(
                        "tcp:host={},port={},family=ipv{}",
                        addr.ip(),
                        addr.port(),
                        if addr.is_ipv4() { '4' } else { '6' }
                    ),
                    introspectable,
                    peer,
                )
                .await?
            }
            DBusBind::Path(path) => {
                DBus::new(
                    &format!("unix:path={}", path.display()),
                    introspectable,
                    peer,
                )
                .await?
            }
        };

        spawn({
            let server = self.clone();
            let dbus = dbus.clone();

            async move {
                log::debug!("obj started");
                let object_path = "/org/example/object/path".try_into().unwrap();
                // Bind the same object to the second object path
                if let Err(error) = server.bind(dbus, object_path).await {
                    log::error!("DBus error: {}", error);
                }
                log::debug!("obj stopped");
            }
        });

        spawn(async move {
            log::debug!("Await signal to stop");
            let lock = stop.acquire().await;
            log::debug!("Received stop signal");
            let _ = dbus.close();

            log::debug!("Await stop");
            let _ = handle.await;
            log::info!("Stopped");
            drop(lock);
        });

        log::info!("Started");

        Ok(())
    }
}
