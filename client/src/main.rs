mod args;

use args::{Args, Action, ButtonArgs};
use ukvmc::{Result, Client, Addr, ButtonId};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::new();

    if args.version {
        println!(
            "{name} {version}",
            name = env!("CARGO_PKG_NAME"),
            version = env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }

    #[cfg(feature = "tracing-subscriber")]
    if let Some(log) = args.log {
        use tracing_subscriber::prelude::*;

        let registry = tracing_subscriber::registry().with(log);

        #[cfg(feature = "stderr")]
        let registry =
            registry.with(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr));

        registry.init();
    }

    let client = Client::open(&args.uri).await?;

    match args.action {
        Action::Status(_) => {
            print!("Buttons:");
            for id in client.buttons() {
                let state = if client.button_state(id)? {
                    "pressed"
                } else {
                    "released"
                };
                print!(" {id}:{state}");
            }
            println!("");
            print!("LEDs:");
            for id in client.leds() {
                let state = if client.led_state(id)? {
                    "on"
                } else {
                    "off"
                };
                print!(" {id}:{state}");
            }
            println!("");
        }
        Action::Button(ButtonArgs { press, release, delay, button }) => {
            let id = button.parse::<ButtonId>().map_err(|_| format!("Unknown button {button}"))?;
            let (press, release) = if press || release {
                (press, release)
            } else {
                (true, true)
            };
            if press {
                println!("Press {id}");
                client.set_button_state(id, true).await?;
            }
            println!("Wait {delay}mS");
            tokio::time::sleep(core::time::Duration::from_millis(delay as _)).await;
            if release {
                println!("Release {id}");
                client.set_button_state(id, false).await?;
            }
        }
    }

    Ok(())
}
