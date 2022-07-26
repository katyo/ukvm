mod args;
mod buttons;
mod leds;
mod server;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "dbus")]
mod dbus;

pub use args::{Args, Bind};
pub use buttons::{ButtonType, Buttons, ButtonsConfig};
pub use leds::{LedType, Leds, LedsConfig};
pub use server::{Server, ServerConfig};

#[cfg(feature = "http")]
pub use http::HttpBind;

#[cfg(feature = "dbus")]
pub use dbus::DBusBind;

pub use anyhow::{Error, Result};

use std::sync::Arc;
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    sync::Semaphore,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "lovely_env_logger")]
    lovely_env_logger::init_default();

    #[cfg(feature = "systemd-journal-logger")]
    systemd_journal_logger::init()?;

    //log::set_max_level(log::LevelFilter::Info);

    let args: Args = clap::Parser::parse();

    log::debug!("Args: {:#?}", args);

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

        // create server instance
        let server = Server::new(&config).await?;

        let stop = Arc::new(Semaphore::new(0));
        let mut interfaces: u32 = 0;

        // start server interfaces
        for bind in &args.bind {
            match bind {
                #[cfg(feature = "http")]
                Bind::Http(bind) => {
                    interfaces += 1;
                    server.spawn_http(bind, &stop).await?;
                }

                #[cfg(feature = "dbus")]
                Bind::DBus(bind) => {
                    interfaces += 1;
                    server.spawn_dbus(bind, &stop).await?;
                }
            }
        }

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
        stop.add_permits(interfaces as _);

        // await while interfaces stop
        let _ = stop.acquire_many(interfaces).await;
    }

    log::info!("Bye");

    Ok(())
}
