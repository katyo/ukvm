mod args;
mod buttons;
mod leds;
mod server;

pub use args::Args;
pub use buttons::{Buttons, ButtonsConfig};
pub use leds::{Leds, LedsConfig};
pub use server::{Server, ServerConfig};

#[cfg(feature = "dbus")]
mod dbus;

pub use anyhow::{Error, Result};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = clap::Parser::parse();

    println!("Hello, world!: {:?}", args);

    let config = ServerConfig::from_file(&args.config).await?;

    let server = Server::new(&config).await?;

    #[cfg(feature = "dbus")]
    if args.dbus {
        server.dbus().await?;
    }

    Ok(())
}
