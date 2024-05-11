mod args;
mod buttons;
mod leds;
mod result;
mod server;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "dbus")]
mod dbus;

#[cfg(feature = "hid")]
mod hid;

#[cfg(feature = "video")]
mod video;

pub use tracing as log;

pub use args::{Args, Bind};
pub use buttons::{Buttons, ButtonsConfig};
pub use leds::{Leds, LedsConfig};
pub use server::{Server, ServerConfig, ServerRef};
pub use ukvm_core::{ButtonId, LedId};

#[cfg(feature = "http")]
pub use ukvm_core::{SocketInput, SocketOutput};

#[cfg(feature = "http")]
pub use http::{HttpBind, HttpBindWay};

#[cfg(feature = "dbus")]
pub use dbus::DBusBind;

#[cfg(feature = "hid")]
pub use hid::{Hid, HidConfig};

#[cfg(feature = "video")]
pub use video::{Video, VideoConfig};

pub use result::{Error, Result};

use std::sync::Arc;
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    sync::Semaphore,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::new();

    #[cfg(feature = "tracing-subscriber")]
    if let Some(log) = args.log {
        use tracing_subscriber::prelude::*;

        let registry = tracing_subscriber::registry().with(log);

        #[cfg(all(feature = "stderr", feature = "journal"))]
        let registry = registry.with(if !args.journal {
            Some(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
        } else {
            None
        });

        #[cfg(all(feature = "stderr", not(feature = "journal")))]
        let registry =
            registry.with(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr));

        #[cfg(feature = "journal")]
        let registry = registry.with(if args.journal {
            Some(tracing_journald::Layer::new()?)
        } else {
            None
        });

        registry.init();
    }

    let mut run = true;

    let mut intr = signal(SignalKind::interrupt())?;
    let mut term = signal(SignalKind::terminate())?;
    let mut usr1 = signal(SignalKind::user_defined1())?;

    while run {
        let config = ServerConfig::from_file(&args.config).await?;

        log::debug!("Config: {:#?}", config);

        if !args.run {
            break;
        }

        let mut spawns: u32 = 0;
        let stop = Arc::new(Semaphore::new(0));

        // create server instance
        let server = Server::new(&config).await?;

        log::info!("Starting");

        // start server interfaces
        for bind in args.bind.iter().chain(&config.binds) {
            match bind {
                #[cfg(feature = "http")]
                Bind::Http(bind) => {
                    spawns += 1;
                    server.spawn_http(bind, &stop).await?;
                }

                #[cfg(feature = "dbus")]
                Bind::DBus(bind) => {
                    spawns += 1;
                    server.spawn_dbus(bind, &stop).await?;
                }
            }
        }

        log::info!("Started");

        select! {
            // stop server
            _ = intr.recv() => {
                log::info!("Interrupt");
                run = false;
            }
            // stop server
            _ = term.recv() => {
                log::info!("Terminate");
                run = false;
            }
            // reload server
            _ = usr1.recv() => {
                log::info!("Reload");
            }
        }

        // stop server interfaces
        stop.add_permits(spawns as _);

        // await while interfaces stop
        let _ = stop.acquire_many(spawns).await;
    }

    log::info!("Bye");

    Ok(())
}
